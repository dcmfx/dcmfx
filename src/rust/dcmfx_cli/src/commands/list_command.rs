use std::{
  io::Write,
  path::PathBuf,
  sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
  },
};

use clap::{Args, ValueEnum};
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::Serialize;

use crate::utils;

pub const ABOUT: &str = "Lists DICOM P10 files in one or more directories";

#[derive(Args)]
pub struct ListArgs {
  #[arg(
    required = true,
    help = "The directories to recursively search for DICOM files."
  )]
  directories: Vec<PathBuf>,

  #[arg(
    long,
    short,
    help = "Extension that files must have in order to be checked for whether \
      it's a DICOM file. The extension check is not case sensitive."
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
      found and their total size. The summary is printed to stderr.",
    default_value_t = false
  )]
  summarize: bool,

  #[arg(
    long,
    short,
    help = "The format used to print the details of DICOM files.",
    default_value_t = Format::Path
  )]
  format: Format,
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
enum Format {
  /// Output each DICOM file as a single line.
  Path,

  /// Output each DICOM file as a single line of JSON.
  JsonLines,
}

impl core::fmt::Display for Format {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Path => write!(f, "path"),
      Self::JsonLines => write!(f, "json-lines"),
    }
  }
}

pub fn run(args: &ListArgs) -> Result<(), ()> {
  // Convert extension to lowercase for comparison
  let extension = args.extension.clone().map(|e| e.to_lowercase());

  // Create iterator for listing all files in the input directories
  let file_iterator = args
    .directories
    .iter()
    .flat_map(|directory| walkdir::WalkDir::new(directory).into_iter());

  // Counters for the DICOM count and total size
  let dicom_file_count = Arc::new(AtomicUsize::new(0));
  let dicom_file_total_size = Arc::new(AtomicU64::new(0));

  let result = {
    let dicom_file_count = dicom_file_count.clone();
    let dicom_file_total_size = dicom_file_total_size.clone();

    utils::create_thread_pool(args.threads).install(move || {
      file_iterator.par_bridge().try_for_each(
        |dir_entry| -> Result<(), walkdir::Error> {
          let dir_entry = dir_entry?;

          // Only process files
          if !dir_entry.file_type().is_file() {
            return Ok(());
          }

          // Check file's extension is allowed, if specified
          if let Some(extension) = &extension {
            if let Some(dir_entry_extension) = dir_entry.path().extension() {
              if dir_entry_extension.to_string_lossy() != *extension {
                return Ok(());
              }
            }
          }

          process_file(
            dir_entry,
            args.format,
            dicom_file_count.clone(),
            dicom_file_total_size.clone(),
          )
        },
      )
    })
  };

  // Print the error if one occurred
  if let Err(error) = result {
    eprintln!("{}", error);
    return Err(());
  }

  // Print summary if requested
  if args.summarize {
    std::io::stdout().flush().unwrap();
    eprintln!();
    eprintln!(
      "Found {} DICOM files, size: {}",
      dicom_file_count.load(Ordering::SeqCst),
      bytesize::ByteSize::b(dicom_file_total_size.load(Ordering::SeqCst)),
    );
  }

  Ok(())
}

fn process_file(
  dir_entry: walkdir::DirEntry,
  output_format: Format,
  dicom_file_count: Arc<AtomicUsize>,
  dicom_file_total_size: Arc<AtomicU64>,
) -> Result<(), walkdir::Error> {
  if !dcmfx::p10::is_valid_file(dir_entry.path()) {
    return Ok(());
  }

  // Get file size
  let file_size = dir_entry.metadata()?.len();

  // Print in the desired output format
  match output_format {
    Format::Path => println!("{}", dir_entry.path().display()),

    Format::JsonLines => {
      let properties = DicomFileProperties {
        path: dir_entry.path().into(),
        size: file_size,
      };

      println!("{}", serde_json::to_string(&properties).unwrap());
    }
  }

  // Accumulate DICOM file statistics
  dicom_file_count.fetch_add(1, Ordering::SeqCst);
  dicom_file_total_size.fetch_add(file_size, Ordering::SeqCst);

  Ok(())
}

#[derive(Serialize, Clone)]
struct DicomFileProperties {
  path: PathBuf,
  size: u64,
}
