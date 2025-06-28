use std::{
  collections::HashSet,
  io::{BufRead, BufReader, Write},
  path::PathBuf,
  sync::{
    Arc, Mutex,
    atomic::{AtomicU64, AtomicUsize, Ordering},
  },
};

use clap::Args;
use dcmfx::{core::*, p10::*};
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::utils;

pub const ABOUT: &str = "Lists DICOM P10 files in one or more directories";

#[derive(Args)]
pub struct ListArgs {
  #[arg(
    help = "The directories to recursively search for DICOM files."
  )]
  directories: Vec<PathBuf>,

  #[arg(
    long,
    short,
    help = "Extension that a file must have in order to be checked for whether \
      it's a DICOM file. The most common'y used extension for DICOM files is \
      'dcm'. The extension check is not case sensitive."
  )]
  extension: Option<String>,

  #[arg(long, help = "")]
  file_list: Option<String>,

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
      found, their total size, and the total number of studies. The summary is \
      printed to stderr.",
    default_value_t = false
  )]
  summarize: bool,

  #[arg(
    long = "select",
    help = "The tags of data elements to include in the output list of DICOM \
      files. This allows specific data from each DICOM file to be gathered as \
      part of the listing process. Specify this argument multiple times to \
      include more than one data element in the output.",
    value_parser = crate::args::validate_data_element_tag,
  )]
  selected_data_elements: Vec<DataElementTag>,
}

pub fn run(args: &ListArgs) -> Result<(), ()> {
  // Convert extension to lowercase for comparison
  let extension = args.extension.clone().map(|e| e.to_lowercase());

  // Create iterator for listing all files to be processed
  let file_iterator: Box<dyn Iterator<Item = PathBuf> + Send> =
    if let Some(file_list) = &args.file_list {
      let file = std::fs::File::open(file_list).unwrap();

      let iter = BufReader::new(file)
        .lines()
        .filter_map(Result::ok)
        .map(PathBuf::from);

      Box::new(iter)
    } else {
      let iter = args.directories.iter().flat_map(|dir| {
        walkdir::WalkDir::new(dir)
          .into_iter()
          .filter_map(Result::ok)
          .filter(|entry| entry.file_type().is_file())
          .map(|entry| entry.path().to_path_buf())
      });

      Box::new(iter)
    };

  // Counters for the DICOM count and total size
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
        |path| -> Result<(), ProcessFileError> {
          // Check file's extension is allowed, if specified
          if let Some(extension) = &extension {
            if let Some(dir_entry_extension) = path.extension() {
              if dir_entry_extension.to_string_lossy() != *extension {
                return Ok(());
              }
            }
          }

          process_file(
            &path,
            &args.selected_data_elements,
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
      ProcessFileError::DataError(e) => e.print(&task_description),
      ProcessFileError::P10Error(e) => e.print(&task_description),
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

enum ProcessFileError {
  IoError(std::io::Error),
  DataError(DataError),
  P10Error(P10Error),
}

fn process_file(
  path: &PathBuf,
  selected_data_elements: &[DataElementTag],
  dicom_file_count: Arc<AtomicUsize>,
  dicom_file_total_size: Arc<AtomicU64>,
  dicom_study_instance_uids: Arc<Mutex<HashSet<String>>>,
) -> Result<(), ProcessFileError> {
  let mut tags_to_read = selected_data_elements.to_vec();
  tags_to_read.push(dictionary::STUDY_INSTANCE_UID.tag);

  let data_set = dcmfx::p10::read_file_partial(
    path,
    &tags_to_read,
    Some(P10ReadConfig::default().require_dicm_prefix(true)),
  );

  if data_set == Err(P10Error::DicmPrefixNotPresent) {
    return Ok(());
  }

  let data_set = data_set.map_err(ProcessFileError::P10Error)?;

  // Count study
  if let Ok(study_instance_uid) =
    data_set.get_string(dictionary::STUDY_INSTANCE_UID.tag)
  {
    dicom_study_instance_uids
      .lock()
      .unwrap()
      .insert(study_instance_uid.to_string());
  }

  let mut properties = serde_json::Map::new();

  properties.insert(
    "path".to_string(),
    serde_json::Value::String(path.to_string_lossy().into()),
  );

  // Get file size
  let file_size = path.metadata()
    .map_err(ProcessFileError::IoError)?
    .len();

  properties.insert(
    "size".to_string(),
    serde_json::Value::Number(file_size.into()),
  );

  for (key, value) in data_set.iter() {
    let mut value =
      format_data_element_value(value).map_err(ProcessFileError::DataError)?;
    if let serde_json::Value::Array(items) = &mut value {
      if items.len() == 1 {
        if dictionary::find(*key, None).unwrap().multiplicity
          == (ValueMultiplicity {
            min: 1,
            max: Some(1),
          })
        {
          value = items.pop().unwrap();
        }
      }
    }

    properties.insert(key.to_hex_string(), value);
  }

  println!("{}", serde_json::to_string(&properties).unwrap());

  // Accumulate DICOM file statistics
  dicom_file_count.fetch_add(1, Ordering::SeqCst);
  dicom_file_total_size.fetch_add(file_size, Ordering::SeqCst);

  Ok(())
}

fn format_data_element_value(
  value: &DataElementValue,
) -> Result<serde_json::Value, DataError> {
  if value.encapsulated_pixel_data().is_ok() {
    panic!("Encapsulated pixel data can't be emitted in list output");
  }

  match value.value_representation() {
    // AttributeTag value representation
    ValueRepresentation::AttributeTag => {
      let values: Vec<_> = value
        .get_attribute_tags()?
        .iter()
        .map(|tag| serde_json::Value::String(tag.to_hex_string()))
        .collect();

      Ok(serde_json::Value::Array(values))
    }

    // Floating point value representations. Because JSON doesn't allow NaN or
    // Infinity values, but they can be present in a DICOM data element, they
    // are converted to strings in the generated JSON.
    ValueRepresentation::DecimalString
    | ValueRepresentation::FloatingPointDouble
    | ValueRepresentation::FloatingPointSingle => {
      let values = value
        .get_floats()?
        .iter()
        .map(|f| {
          if f.is_nan() {
            serde_json::Value::String("NaN".to_string())
          } else if *f == f64::INFINITY {
            serde_json::Value::String("Infinity".to_string())
          } else if *f == f64::NEG_INFINITY {
            serde_json::Value::String("-Infinity".to_string())
          } else {
            serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap())
          }
        })
        .collect();

      Ok(serde_json::Value::Array(values))
    }

    // String VRs that don't support multiplicity
    ValueRepresentation::AgeString
    | ValueRepresentation::ApplicationEntity
    | ValueRepresentation::LongText
    | ValueRepresentation::ShortText
    | ValueRepresentation::UniversalResourceIdentifier
    | ValueRepresentation::UnlimitedText => {
      Ok(serde_json::Value::String(value.get_string()?.to_string()))
    }

    // String VRs that support multiplicity
    ValueRepresentation::CodeString
    | ValueRepresentation::Date
    | ValueRepresentation::DateTime
    | ValueRepresentation::LongString
    | ValueRepresentation::PersonName
    | ValueRepresentation::ShortString
    | ValueRepresentation::Time
    | ValueRepresentation::UniqueIdentifier
    | ValueRepresentation::UnlimitedCharacters => {
      let values: Vec<_> = value
        .get_strings()?
        .iter()
        .map(|s| serde_json::Value::String(s.to_string()))
        .collect();

      Ok(serde_json::Value::Array(values))
    }

    // Binary signed/unsigned integer value representations
    ValueRepresentation::SignedLong
    | ValueRepresentation::SignedShort
    | ValueRepresentation::UnsignedLong
    | ValueRepresentation::UnsignedShort
    | ValueRepresentation::IntegerString => {
      let values: Vec<_> = value
        .get_ints::<i64>()?
        .iter()
        .map(|i| serde_json::Value::Number(serde_json::Number::from(*i)))
        .collect();

      Ok(serde_json::Value::Array(values))
    }

    // Binary signed/unsigned big integer value representations
    ValueRepresentation::SignedVeryLong
    | ValueRepresentation::UnsignedVeryLong => {
      // The range of integers representable by JavaScript's Number type.
      // Values outside this range are converted to strings in the generated
      // JSON.
      let safe_integer_range = -9007199254740991i128..=9007199254740991i128;

      let values: Vec<_> = value
        .get_big_ints()?
        .iter()
        .map(|i| {
          if safe_integer_range.contains(i) {
            serde_json::Value::Number(serde_json::Number::from(*i as i64))
          } else {
            serde_json::Value::String(i.to_string())
          }
        })
        .collect();

      Ok(serde_json::Value::Array(values))
    }

    ValueRepresentation::OtherByteString
    | ValueRepresentation::OtherDoubleString
    | ValueRepresentation::OtherFloatString
    | ValueRepresentation::OtherLongString
    | ValueRepresentation::OtherVeryLongString
    | ValueRepresentation::OtherWordString
    | ValueRepresentation::Unknown => {
      let bytes = value.bytes().unwrap();

      use base64::{Engine, engine::general_purpose};
      let encoded = general_purpose::STANDARD.encode(bytes);

      Ok(serde_json::Value::String(encoded))
    }

    ValueRepresentation::Sequence => {
      panic!("Sequences can't be emitted in list output")
    }
  }
}
