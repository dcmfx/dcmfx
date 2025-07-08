use std::{io::Read, path::PathBuf};

use clap::Args;
use rayon::prelude::*;

use dcmfx::core::*;
use dcmfx::json::*;
use dcmfx::p10::*;

use crate::{InputSource, args::file_list_arg, utils};

pub const ABOUT: &str = "Converts DICOM JSON files to DICOM P10 files";

#[derive(Args)]
pub struct ToDcmArgs {
  #[arg(
    help = "DICOM JSON files to convert to DICOM P10 files. Specify '-' to \
      read from stdin."
  )]
  input_filenames: Vec<PathBuf>,

  #[arg(long, help = file_list_arg::HELP)]
  file_list: Option<PathBuf>,

  #[arg(
    long,
    short,
    help = "The name of the DICOM P10 output file. By default the output \
      DICOM P10 file is the name of the input file with '.dcm' appended. \
      Specify '-' to write to stdout."
  )]
  output_filename: Option<PathBuf>,

  #[arg(
    long,
    short = 'd',
    help = "The directory to write output files into. The names of the output \
      DICOM P10 files will be the name of the input file with '.dcm' \
      appended."
  )]
  output_directory: Option<PathBuf>,

  #[arg(
    long,
    help = "The number of threads to use to perform work. Each thread operates \
      on one input file at a time, so using more threads may improve \
      performance when processing many input files.\n\
      \n\
      When outputting to stdout only one thread can be used.\n\
      \n\
      The default thread count is the number of logical CPUs available.",
    default_value_t = rayon::current_num_threads()
  )]
  threads: usize,

  #[arg(
    long,
    help = "Overwrite files without prompting",
    default_value_t = false
  )]
  overwrite: bool,

  #[arg(
    long,
    help = "The value of the Implementation Version Name data element in \
      output DICOM P10 files. The value must conform to the specification of \
      the SS (Short String) value representation.",
    default_value_t = uids::DCMFX_IMPLEMENTATION_VERSION_NAME.to_string(),
  )]
  implementation_version_name: String,
}

enum ToDcmError {
  P10Error(P10Error),
  JsonDeserializeError(JsonDeserializeError),
}

pub fn run(args: &mut ToDcmArgs) -> Result<(), ()> {
  crate::validate_output_args(&args.output_filename, &args.output_directory);

  let input_sources = crate::input_source::create_iterator(
    &mut args.input_filenames,
    &args.file_list,
  );

  let result = utils::create_thread_pool(args.threads).install(move || {
    input_sources.par_bridge().try_for_each(|input_source| {
      let output_filename = if let Some(output_filename) = &args.output_filename
      {
        output_filename.clone()
      } else {
        input_source.output_path(".dcm", &args.output_directory)
      };

      match input_source_to_dcm(&input_source, output_filename, args) {
        Ok(()) => Ok(()),

        Err(e) => {
          let task_description = format!("converting \"{}\"", input_source);

          Err(match e {
            ToDcmError::P10Error(e) => e.to_lines(&task_description),
            ToDcmError::JsonDeserializeError(e) => {
              e.to_lines(&task_description)
            }
          })
        }
      }
    })
  });

  match result {
    Ok(()) => Ok(()),

    Err(lines) => {
      error::print_error_lines(&lines);
      Err(())
    }
  }
}

fn input_source_to_dcm(
  input_source: &InputSource,
  output_filename: PathBuf,
  args: &ToDcmArgs,
) -> Result<(), ToDcmError> {
  let mut stream = input_source
    .open_read_stream()
    .map_err(ToDcmError::P10Error)?;

  // Open output stream
  let output_stream = utils::open_output_stream(
    &output_filename,
    Some(&output_filename),
    args.overwrite,
  )
  .map_err(ToDcmError::P10Error)?;

  let mut buffer = vec![];

  // Read the DICOM JSON from the input stream
  if let Err(e) = stream.read_to_end(&mut buffer) {
    return Err(ToDcmError::P10Error(P10Error::FileError {
      when: "Reading file".to_string(),
      details: e.to_string(),
    }));
  }

  // Validate the data is UTF-8
  let json = match std::str::from_utf8(&buffer) {
    Ok(s) => s,
    Err(e) => {
      return Err(ToDcmError::P10Error(P10Error::FileError {
        when: "Reading file".to_string(),
        details: format!("Invalid UTF-8 at byte {}", e.valid_up_to()),
      }));
    }
  };

  // Read DICOM JSON into a data set
  let data_set =
    DataSet::from_json(json).map_err(ToDcmError::JsonDeserializeError)?;

  let write_config = P10WriteConfig::default()
    .implementation_version_name(args.implementation_version_name.clone());

  // Get exclusive access to the output stream
  let mut output_stream = output_stream.lock().unwrap();

  // Write P10 data to output stream
  data_set
    .write_p10_stream(&mut *output_stream, Some(write_config))
    .map_err(ToDcmError::P10Error)
}
