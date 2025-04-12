use std::{
  io::{Read, Write},
  path::PathBuf,
};

use clap::Args;

use dcmfx::core::*;
use dcmfx::json::*;
use dcmfx::p10::*;

use crate::{InputSource, utils};

pub const ABOUT: &str = "Converts DICOM JSON files to DICOM P10 files";

#[derive(Args)]
pub struct ToDcmArgs {
  #[clap(
    required = true,
    help = "The names of the DICOM JSON files to convert to DICOM P10 files. \
      Specify '-' to read from stdin."
  )]
  input_filenames: Vec<PathBuf>,

  #[clap(
    long,
    short,
    help = "The name of the output DICOM P10 file. This option is only valid \
      when a single input filename is specified. Specify '-' to write to \
      stdout."
  )]
  output_filename: Option<PathBuf>,

  #[clap(
    long,
    help = "Overwrite files without prompting",
    default_value_t = false
  )]
  overwrite: bool,

  #[clap(
    long,
    help = "Specifies the value of the Implementation Version Name data \
      element in output DICOM P10 files.",
    default_value_t = uids::DCMFX_IMPLEMENTATION_VERSION_NAME.to_string(),
  )]
  implementation_version_name: String,
}

enum ToDcmError {
  P10Error(P10Error),
  JsonDeserializeError(JsonDeserializeError),
}

pub fn run(args: &ToDcmArgs) -> Result<(), ()> {
  let input_sources = crate::get_input_sources(&args.input_filenames);

  if input_sources.len() > 1 && args.output_filename.is_some() {
    eprintln!(
      "When there are multiple input files --output-filename must not be \
       specified"
    );
    return Err(());
  }

  for input_source in input_sources {
    match input_source_to_dcm(&input_source, args) {
      Ok(()) => (),

      Err(e) => {
        let task_description = format!("converting \"{}\"", input_source);

        match e {
          ToDcmError::P10Error(e) => e.print(&task_description),
          ToDcmError::JsonDeserializeError(e) => e.print(&task_description),
        }

        return Err(());
      }
    }
  }

  Ok(())
}

fn input_source_to_dcm(
  input_source: &InputSource,
  args: &ToDcmArgs,
) -> Result<(), ToDcmError> {
  let mut stream = input_source
    .open_read_stream()
    .map_err(ToDcmError::P10Error)?;

  let output_filename = args
    .output_filename
    .clone()
    .unwrap_or_else(|| input_source.clone().append(".dcm"));

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

  // Open output stream
  let mut output_stream: Box<dyn Write> = utils::open_output_stream(
    &output_filename,
    Some(&output_filename),
    args.overwrite,
  )
  .map_err(ToDcmError::P10Error)?;

  let write_config = P10WriteConfig {
    implementation_version_name: args.implementation_version_name.clone(),
    ..P10WriteConfig::default()
  };

  // Write P10 data to output stream
  data_set
    .write_p10_stream(&mut output_stream, Some(write_config))
    .map_err(ToDcmError::P10Error)
}
