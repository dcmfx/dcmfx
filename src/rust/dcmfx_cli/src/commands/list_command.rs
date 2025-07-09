use std::{
  collections::HashSet,
  io::Write,
  path::PathBuf,
  sync::{
    Arc, Mutex,
    atomic::{AtomicU64, AtomicUsize, Ordering},
  },
};

use clap::{Args, ValueEnum};
use dcmfx::{core::*, json::*, p10::*};
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::{
  input_source::InputSource,
  utils::{self, GLOBAL_STDOUT},
};

pub const ABOUT: &str = "Lists DICOM P10 files in one or more directories";

#[derive(Args)]
pub struct ListArgs {
  #[arg(
    required = true,
    help = "Directories to recursively search for DICOM P10 files."
  )]
  directories: Vec<PathBuf>,

  #[arg(
    long,
    short,
    help = "Extension that a file must have in order to be checked for whether \
      it's a DICOM file. The most commonly used extension for DICOM files is \
      'dcm'. The extension check is not case sensitive."
  )]
  extension: Option<String>,

  #[arg(
    long,
    help = "The number of threads to use to perform work.\n\
      \n\
      The default thread count is the number of logical CPUs available.",
    default_value_t = rayon::current_num_threads()
  )]
  threads: usize,

  #[arg(
    long,
    help = "Whether to print a summary of the total number of DICOM files \
      found, the total number of studies, and their total size. The summary is \
      printed to stderr.",
    default_value_t = false
  )]
  summarize: bool,

  #[arg(
    long,
    short,
    help = "The format used to print the details of DICOM files.",
    default_value_t = Format::FileList
  )]
  format: Format,

  #[arg(
    long = "select",
    help = "The tags of data elements to include in the output list of DICOM \
      files. This allows for a subset of data elements from each DICOM file to \
      be gathered as part of the listing process. Selected data elements are \
      output as DICOM JSON. Specify this argument multiple times to include \
      more than one data element in the output.",
    value_parser = crate::args::validate_data_element_tag,
  )]
  selected_data_elements: Vec<DataElementTag>,
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
enum Format {
  /// Output each DICOM file as a single line containing its path.
  FileList,

  /// Output each DICOM file as a single line of JSON.
  JsonLines,
}

impl core::fmt::Display for Format {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::FileList => write!(f, "file-list"),
      Self::JsonLines => write!(f, "json-lines"),
    }
  }
}

pub fn run(args: &ListArgs) -> Result<(), ()> {
  if !args.selected_data_elements.is_empty() && args.format != Format::JsonLines
  {
    eprintln!(
      "Error: `--format json-lines` must be specified when selecting data \
       elements"
    );
    std::process::exit(1);
  }

  // Convert extension to lowercase for comparison
  let extension = args.extension.clone().map(|e| e.to_lowercase());

  // Create iterator for listing all files to be processed
  let file_iterator = args.directories.iter().flat_map(|dir| {
    walkdir::WalkDir::new(dir)
      .into_iter()
      .filter_map(|entry| match entry {
        Ok(entry) => {
          if entry.file_type().is_file() {
            Some(InputSource::LocalFile {
              path: entry.path().to_path_buf(),
            })
          } else {
            None
          }
        }

        Err(e) => {
          eprintln!("Error: {}", e);
          std::process::exit(1);
        }
      })
  });

  // Values used to track information needed when printing a summary at the end
  // of the list output
  let dicom_file_count = Arc::new(AtomicUsize::new(0));
  let dicom_file_total_size = Arc::new(AtomicU64::new(0));
  let dicom_study_instance_uids =
    Arc::new(Mutex::new(HashSet::<String>::new()));

  let result = {
    let dicom_file_count = dicom_file_count.clone();
    let dicom_file_total_size = dicom_file_total_size.clone();
    let dicom_study_instance_uids = dicom_study_instance_uids.clone();

    utils::create_thread_pool(args.threads).install(move || {
      file_iterator.par_bridge().try_for_each(
        |input_source| -> Result<(), ProcessFileError> {
          let InputSource::LocalFile { path } = input_source else {
            eprintln!(
              "Error: reading from stdin is not supported with the list command"
            );
            std::process::exit(1);
          };

          // Check file's extension is allowed, if this check was requested
          if let Some(extension) = &extension {
            if let Some(dir_entry_extension) = path.extension() {
              if dir_entry_extension.to_string_lossy() != *extension {
                return Ok(());
              }
            }
          }

          process_file(
            &path,
            args,
            dicom_file_count.clone(),
            dicom_file_total_size.clone(),
            dicom_study_instance_uids.clone(),
          )
        },
      )
    })
  };

  // Print the error if one occurred
  if let Err(error) = result {
    let task_description = "listing DICOM files".to_string();

    match error {
      ProcessFileError::IoError(e) => {
        error::print_error_lines(&[e.to_string()])
      }
      ProcessFileError::P10Error(e) => e.print(&task_description),
      ProcessFileError::JsonSerializeError(e) => e.print(&task_description),
    }

    return Err(());
  }

  // Print summary if requested
  if args.summarize {
    std::io::stdout().flush().unwrap();
    eprintln!();
    eprintln!(
      "Found {} DICOM files, {} studies, total size: {}",
      dicom_file_count.load(Ordering::SeqCst),
      dicom_study_instance_uids.lock().unwrap().len(),
      bytesize::ByteSize::b(dicom_file_total_size.load(Ordering::SeqCst)),
    );
  }

  Ok(())
}

#[allow(clippy::enum_variant_names)]
enum ProcessFileError {
  IoError(std::io::Error),
  P10Error(P10Error),
  JsonSerializeError(JsonSerializeError),
}

fn process_file(
  path: &PathBuf,
  args: &ListArgs,
  dicom_file_count: Arc<AtomicUsize>,
  dicom_file_total_size: Arc<AtomicU64>,
  dicom_study_instance_uids: Arc<Mutex<HashSet<String>>>,
) -> Result<(), ProcessFileError> {
  // Memoized closure that returns the size of the file in bytes. This allows
  // the metadata() call to be avoided if not needed, and to only be performed
  // at most once.
  let mut file_size_cache: Option<u64> = None;
  let mut file_size = || -> Result<u64, ProcessFileError> {
    if let Some(cached) = file_size_cache {
      return Ok(cached);
    }

    let size = path.metadata().map_err(ProcessFileError::IoError)?.len();
    file_size_cache = Some(size);
    Ok(size)
  };

  // Get the line of output for this file
  let output_line = output_line_for_file(
    path,
    args,
    &dicom_study_instance_uids,
    &mut file_size,
  )?;

  // If None was returned then it's not a DICOM P10 file
  let Some(mut output_line) = output_line else {
    return Ok(());
  };

  // Add a terminating newline
  output_line.push('\n');

  // Get exclusive access to the shared stdout stream
  let mut stdout = GLOBAL_STDOUT.lock().unwrap();

  // Write line to stdout
  stdout
    .write_all(output_line.as_bytes())
    .map_err(ProcessFileError::IoError)?;

  // Accumulate stats if a summary of the listing was requested
  if args.summarize {
    dicom_file_count.fetch_add(1, Ordering::SeqCst);
    dicom_file_total_size.fetch_add(file_size()?, Ordering::SeqCst);
  }

  Ok(())
}

fn output_line_for_file(
  path: &PathBuf,
  args: &ListArgs,
  dicom_study_instance_uids: &Arc<Mutex<HashSet<String>>>,
  mut file_size: impl FnMut() -> Result<u64, ProcessFileError>,
) -> Result<Option<String>, ProcessFileError> {
  let mut tags_to_read = args.selected_data_elements.to_vec();

  // If summarizing, ensure that the Study Instance UID is read from the DICOM
  // file so it can be counted
  if args.summarize {
    tags_to_read.push(dictionary::STUDY_INSTANCE_UID.tag);
  }

  if tags_to_read.is_empty() {
    // If this isn't a DICOM P10 file then there's nothing to do
    if !dcmfx::p10::is_valid_file(path) {
      return Ok(None);
    }

    match args.format {
      Format::FileList => Ok(Some(path.to_string_lossy().to_string())),

      Format::JsonLines => {
        let mut output = serde_json::Map::new();

        output.insert("path".into(), path.to_string_lossy().into());
        output.insert("size".into(), file_size()?.into());

        Ok(Some(serde_json::to_string(&output).unwrap()))
      }
    }
  } else {
    let data_set = dcmfx::p10::read_file_partial(
      path,
      &tags_to_read,
      Some(P10ReadConfig::default().require_dicm_prefix(true)),
    );

    // If this isn't a DICOM P10 file then there's nothing to do
    if data_set == Err(P10Error::DicmPrefixNotPresent) {
      return Ok(None);
    }

    // Any other error reading the file is returned
    let mut data_set = data_set.map_err(ProcessFileError::P10Error)?;

    // If summarizing, track this DICOM file's Study Instance UID
    if args.summarize {
      if let Ok(study_instance_uid) =
        data_set.get_string(dictionary::STUDY_INSTANCE_UID.tag)
      {
        dicom_study_instance_uids
          .lock()
          .unwrap()
          .insert(study_instance_uid.to_string());
      }

      // Don't include the Study Instance UID if it was only read in order to
      // count studies
      tags_to_read.pop();
      if !tags_to_read.contains(&dictionary::STUDY_INSTANCE_UID.tag) {
        data_set.delete(dictionary::STUDY_INSTANCE_UID.tag);
      }
    }

    match args.format {
      Format::FileList => Ok(Some(path.to_string_lossy().to_string())),

      Format::JsonLines => {
        let mut output = serde_json::Map::new();

        output.insert("path".to_string(), path.to_string_lossy().into());
        output.insert("size".to_string(), file_size()?.into());

        // If there are data elements included in the listing then add the read
        // data set to the output
        if !args.selected_data_elements.is_empty() {
          let dicom_json = &data_set
            .to_json(DicomJsonConfig::default())
            .map_err(ProcessFileError::JsonSerializeError)?;

          output.insert(
            "data_set".to_string(),
            serde_json::from_str(dicom_json).unwrap(),
          );
        }

        // Construct final JSON line
        Ok(Some(serde_json::to_string(&output).unwrap()))
      }
    }
  }
}
