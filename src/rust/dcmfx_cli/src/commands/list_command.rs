use std::{
  collections::{HashMap, HashSet},
  io::Write,
  path::{Path, PathBuf},
  sync::{Arc, Mutex},
};

use clap::{Args, ValueEnum};
use dcmfx::{core::*, json::*, p10::*};
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::{args::input_args::InputSource, utils};

pub const ABOUT: &str = "Lists DICOM P10 files in one or more directories";

#[derive(Args)]
pub struct ListArgs {
  #[arg(
    long,
    help = "The number of threads to use to perform work.",
    default_value_t = rayon::current_num_threads()
  )]
  threads: usize,

  #[arg(
    required = true,
    help_heading = "Input",
    help = "Directories to recursively search for DICOM P10 files."
  )]
  directories: Vec<PathBuf>,

  #[arg(
    long,
    short,
    help_heading = "Input",
    help = "Extension that a file must have in order to be checked for whether \
      it's a DICOM file. The most commonly used extension for DICOM files is \
      'dcm'. The extension check is not case sensitive."
  )]
  extension: Option<String>,

  #[arg(
    long,
    short,
    help_heading = "Output",
    help = "The format used to print the details of DICOM files.",
    default_value_t = Format::FileList
  )]
  format: Format,

  #[arg(
    long = "select",
    help_heading = "Output",
    help = "The tags of data elements to include in the output list of DICOM \
      files. This allows for a subset of data elements from each DICOM file to \
      be gathered as part of the listing process. Selected data elements are \
      output as DICOM JSON. Specify this argument multiple times to include \
      more than one data element in the output.\n\
      \n\
      Commonly selected data element tags are:\n\
      \n\
      - (0002,0010) Transfer Syntax UID\n\
      - (0020,000D) Study Instance UID\n\
      - (0020,000E) Series Instance UID\n\
      - (0008,0018) SOP Instance UID\n\
      - (0008,0016) SOP Class UID\n\
      - (0008,0020) Study Date\n\
      - (0008,0030) Study Time\n\
      - (0008,0060) Modality\n\
      ",
    value_parser = crate::args::validate_data_element_tag,
  )]
  selected_data_elements: Vec<DataElementTag>,

  #[arg(
    long,
    help_heading = "Output",
    help = "Whether to print a summary of the listed DICOM files. The summary \
      details the distribution of transfer syntaxes and SOP classes, followed \
      by the total number of DICOM files, total number of studies, and size \
      information. The summary is printed to stderr.",
    default_value_t = false
  )]
  summarize: bool,
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
          eprintln!("Error: {e}");
          std::process::exit(1);
        }
      })
  });

  // Track information needed when printing a summary at the end of the list
  // output
  let summary = Arc::new(Mutex::new(Summary::new()));

  let result = {
    let summary = summary.clone();

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
          if let Some(extension) = &extension
            && let Some(dir_entry_extension) = path.extension()
            && dir_entry_extension.to_string_lossy() != *extension
          {
            return Ok(());
          }

          process_file(&path, args, summary.clone())
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
    summary.lock().unwrap().print_tables();
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
  path: &Path,
  args: &ListArgs,
  summary: Arc<Mutex<Summary>>,
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
  let output_line = output_line_for_file(path, args, &summary, &mut file_size)?;

  // If None was returned then it's not a DICOM P10 file
  let Some(mut output_line) = output_line else {
    return Ok(());
  };

  // Add a terminating newline
  output_line.push('\n');

  // Get exclusive access to the shared stdout stream
  let mut stdout = utils::GLOBAL_STDOUT.lock().unwrap();

  // Write line to stdout
  stdout
    .write_all(output_line.as_bytes())
    .map_err(ProcessFileError::IoError)?;

  // Accumulate stats if a summary of the listing was requested
  if args.summarize {
    summary.lock().unwrap().dicoms.add_dicom(file_size()?);
  }

  Ok(())
}

fn output_line_for_file(
  path: &Path,
  args: &ListArgs,
  summary: &Arc<Mutex<Summary>>,
  mut file_size: impl FnMut() -> Result<u64, ProcessFileError>,
) -> Result<Option<String>, ProcessFileError> {
  let mut tags_to_read = args.selected_data_elements.to_vec();

  // If summarizing, read extra tags from the DICOM file
  if args.summarize {
    tags_to_read.extend_from_slice(&Summary::SUMMARY_DATA_ELEMENT_TAGS);
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

    // Propagate any other error reading the file
    let mut data_set = data_set.map_err(ProcessFileError::P10Error)?;

    // If summarizing, add details of this DICOM to the summary
    if args.summarize {
      summary
        .lock()
        .unwrap()
        .update(path, &data_set, file_size()?);

      // Remove data elements that were only added for use in the summary
      for tag in Summary::SUMMARY_DATA_ELEMENT_TAGS {
        tags_to_read.pop();
        if !tags_to_read.contains(&tag) {
          data_set.delete(tag);
        }
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
          let json_config = DicomJsonConfig {
            store_encapsulated_pixel_data: true,
            ..Default::default()
          };

          let dicom_json = &data_set
            .to_json(json_config)
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

/// A summary of the DICOM files found during the listing process.
///
struct Summary {
  dicoms: DicomCountAndSize,
  transfer_syntaxes: HashMap<&'static TransferSyntax, DicomCountAndSize>,
  sop_class_uids: HashMap<String, DicomCountAndSize>,
  study_instance_uids: HashSet<String>,
  largest_dicom: (PathBuf, u64),
}

#[derive(Clone, Copy, Default)]
struct DicomCountAndSize {
  count: usize,
  size: u64,
}

impl DicomCountAndSize {
  fn add_dicom(&mut self, dicom_size: u64) {
    self.count += 1;
    self.size += dicom_size;
  }
}

impl Summary {
  /// The tags of DICOM data elements that need to be read from DICOM files in
  /// order to generate the summary.
  ///
  const SUMMARY_DATA_ELEMENT_TAGS: [DataElementTag; 3] = [
    dictionary::TRANSFER_SYNTAX_UID.tag,
    dictionary::SOP_CLASS_UID.tag,
    dictionary::STUDY_INSTANCE_UID.tag,
  ];

  /// Creates a new summary with default values.
  ///
  fn new() -> Self {
    Self {
      dicoms: DicomCountAndSize::default(),
      transfer_syntaxes: HashMap::new(),
      sop_class_uids: HashMap::new(),
      study_instance_uids: HashSet::new(),
      largest_dicom: (PathBuf::new(), 0),
    }
  }

  /// Updates the summary with information from a DICOM file.
  ///
  fn update(&mut self, path: &Path, data_set: &DataSet, file_size: u64) {
    // Add transfer syntax to summary
    let transfer_syntax = data_set
      .get_transfer_syntax()
      .unwrap_or(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN);

    self
      .transfer_syntaxes
      .entry(transfer_syntax)
      .or_default()
      .add_dicom(file_size);

    // Add SOP Class UID to summary
    if let Ok(sop_class_uid) =
      data_set.get_string(dictionary::SOP_CLASS_UID.tag)
    {
      self
        .sop_class_uids
        .entry(sop_class_uid.to_string())
        .or_default()
        .add_dicom(file_size);
    }

    // Add Study Instance UID to summary
    if let Ok(study_instance_uid) =
      data_set.get_string(dictionary::STUDY_INSTANCE_UID.tag)
    {
      self
        .study_instance_uids
        .insert(study_instance_uid.to_string());
    }

    // Update largest DICOM file if this one is now the largest
    if file_size > self.largest_dicom.1 {
      self.largest_dicom = (path.to_path_buf(), file_size);
    }
  }

  /// Prints the summary tables to stderr.
  ///
  fn print_tables(&self) {
    self.print_transfer_syntaxes_table();
    self.print_sop_class_uids_table();
    self.print_summary_table();
  }

  fn print_transfer_syntaxes_table(&self) {
    let header = ["Transfer Syntax", "Count", "Size"];

    let rows: Vec<_> = self
      .transfer_syntaxes
      .iter()
      .map(|(transfer_syntax, summary)| {
        (transfer_syntax.name.to_string(), *summary)
      })
      .collect();

    self.print_sorted_table(&header, rows);
  }

  fn print_sop_class_uids_table(&self) {
    let header = ["SOP Class", "Count", "Size"];

    let rows: Vec<_> = self
      .sop_class_uids
      .iter()
      .map(|(sop_class_uid, summary)| {
        (
          dictionary::uid_name(sop_class_uid)
            .unwrap_or(sop_class_uid)
            .to_string(),
          *summary,
        )
      })
      .collect();

    self.print_sorted_table(&header, rows);
  }

  fn print_summary_table(&self) {
    let mut table = Self::create_table(&["Summary", "Value"]);
    table.add_row(["DICOM count".to_string(), self.dicoms.count.to_string()]);
    table.add_row([
      "DICOM total size".to_string(),
      bytesize::ByteSize::b(self.dicoms.size).to_string(),
    ]);

    table.add_row([
      "DICOM mean size".to_string(),
      bytesize::ByteSize::b(self.dicoms.size / self.dicoms.count as u64)
        .to_string(),
    ]);
    table.add_row([
      "DICOM largest size".to_string(),
      format!(
        "{} ({})",
        self.largest_dicom.0.display(),
        bytesize::ByteSize::b(self.largest_dicom.1),
      ),
    ]);
    table.add_row([
      "Study count".to_string(),
      self.study_instance_uids.len().to_string(),
    ]);
    eprintln!("{table}");
  }

  fn create_table(header: &[&str]) -> comfy_table::Table {
    use comfy_table::{
      Attribute, Cell, CellAlignment, Table, presets::UTF8_FULL,
    };

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    table.set_header(
      header
        .iter()
        .map(|text| Cell::new(text).add_attribute(Attribute::Bold))
        .collect::<Vec<_>>(),
    );

    if let Some(column) = table.column_mut(1) {
      column.set_cell_alignment(CellAlignment::Right);
    }
    if let Some(column) = table.column_mut(2) {
      column.set_cell_alignment(CellAlignment::Right);
    }

    table
  }

  fn print_sorted_table(
    &self,
    header: &[&str],
    mut rows: Vec<(String, DicomCountAndSize)>,
  ) {
    let mut table = Self::create_table(header);

    // Sort by count (descending) and then by name (ascending)
    rows.sort_by(|a, b| b.1.count.cmp(&a.1.count).then(a.0.cmp(&b.0)));

    for (name, stats) in rows {
      let count_percent =
        (stats.count as f64 / self.dicoms.count as f64) * 100.0;
      let size_percent = (stats.size as f64 / self.dicoms.size as f64) * 100.0;

      table.add_row([
        name,
        format!("{} ({count_percent:.1}%)", stats.count),
        format!("{} ({size_percent:.1}%)", bytesize::ByteSize::b(stats.size)),
      ]);
    }

    eprintln!("{table}");
  }
}
