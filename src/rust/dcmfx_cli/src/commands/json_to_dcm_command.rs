use std::path::PathBuf;

use clap::Args;
use tokio::io::AsyncReadExt;

use dcmfx::{core::*, json::*, p10::*};

use crate::utils::{self, InputSource, OutputTarget};

pub const ABOUT: &str = "Converts DICOM JSON files to DICOM P10 files";

#[derive(Args)]
pub struct ToDcmArgs {
  #[arg(
    long,
    help = "The number of concurrent tasks to use. Defaults to the number of CPU
      cores.",
    default_value_t = {num_cpus::get()}
  )]
  concurrency: usize,

  #[command(flatten)]
  input: crate::args::input_args::BaseInputArgs,

  #[arg(
    long,
    short,
    help_heading = "Output",
    help = "The name of the DICOM P10 output file. By default the output \
      DICOM P10 file is the name of the input file with '.dcm' appended. \
      Specify '-' to write to stdout."
  )]
  output_filename: Option<PathBuf>,

  #[arg(
    long,
    short = 'd',
    help_heading = "Output",
    help = "The directory to write output files into. The names of the output \
      DICOM P10 files will be the name of the input file with '.dcm' \
      appended."
  )]
  output_directory: Option<PathBuf>,

  #[arg(
    long,
    help_heading = "Output",
    help = "Overwrite any output files that already exist",
    default_value_t = false
  )]
  overwrite: bool,

  #[arg(
    long,
    help_heading = "Output",
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

pub async fn run(args: ToDcmArgs) -> Result<(), ()> {
  crate::validate_output_args(
    args.output_filename.as_ref(),
    args.output_directory.as_ref(),
  )
  .await;

  OutputTarget::set_overwrite(args.overwrite);

  let input_sources = args.input.input_sources().await;

  let result = utils::run_tasks(
    args.concurrency,
    input_sources,
    async |input_source: InputSource| {
      let output_target = if let Some(output_filename) = &args.output_filename {
        OutputTarget::new(output_filename).await
      } else {
        OutputTarget::from_input_source(
          &input_source,
          ".dcm",
          &args.output_directory,
        )
        .await
      };

      match input_source_to_dcm(&input_source, output_target, &args).await {
        Ok(()) => Ok(()),

        Err(e) => {
          let task_description = format!("converting \"{input_source}\"");

          Err(match e {
            ToDcmError::P10Error(e) => e.to_lines(&task_description),
            ToDcmError::JsonDeserializeError(e) => {
              e.to_lines(&task_description)
            }
          })
        }
      }
    },
  )
  .await;

  match result {
    Ok(()) => Ok(()),

    Err(lines) => {
      error::print_error_lines(&lines);
      Err(())
    }
  }
}

async fn input_source_to_dcm(
  input_source: &InputSource,
  output_target: OutputTarget,
  args: &ToDcmArgs,
) -> Result<(), ToDcmError> {
  let mut stream = input_source
    .open_read_stream()
    .await
    .map_err(ToDcmError::P10Error)?;

  // Open output stream
  let output_stream = output_target
    .open_write_stream(true)
    .await
    .map_err(ToDcmError::P10Error)?;

  let mut buffer = vec![];

  // Read the DICOM JSON from the input stream
  if let Err(e) = stream.read_to_end(&mut buffer).await {
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
  let mut output_stream = output_stream.lock().await;

  // Write P10 data to output stream
  data_set
    .write_p10_stream_async(&mut *output_stream, Some(write_config))
    .await
    .map_err(ToDcmError::P10Error)?;

  output_target
    .commit(&mut output_stream)
    .await
    .map_err(ToDcmError::P10Error)
}
