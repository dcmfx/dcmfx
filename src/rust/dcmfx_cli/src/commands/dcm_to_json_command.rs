use std::path::PathBuf;

use clap::Args;
use tokio::io::AsyncWriteExt;

use dcmfx::{core::*, json::*, p10::*};

use crate::utils::{self, InputSource, OutputTarget};

pub const ABOUT: &str = "Converts DICOM P10 files to DICOM JSON files";

#[derive(Args)]
pub struct ToJsonArgs {
  #[arg(
    long,
    help = "The number of concurrent tasks to use. Defaults to the number of CPU
      cores.",
    default_value_t = {num_cpus::get()}
  )]
  concurrency: usize,

  #[command(flatten)]
  input: crate::args::input_args::P10InputArgs,

  #[arg(
    long,
    short,
    help_heading = "Output",
    help = "The name of the DICOM JSON output file. By default the output \
      DICOM JSON file is the name of the input file with '.json' appended. \
      Specify '-' to write to stdout."
  )]
  output_filename: Option<PathBuf>,

  #[arg(
    long,
    short = 'd',
    help_heading = "Output",
    help = "The directory to write output files into. The names of the output \
      DICOM JSON files will be the name of the input file with '.json' \
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
    long = "pretty",
    help_heading = "Output",
    help = "Whether to format the DICOM JSON for readability with newlines and \
      indentation",
    default_value_t = false
  )]
  pretty_print: bool,

  #[arg(
      long = "exclude-binary-values",
      help_heading = "Output",
      help = "Prevents conversion to DICOM JSON of data element values that \
        use one of the binary value representations (OB, OD, OF, OL, OV, OW, \
        UN). The data element and its VR will still be emitted, but its value \
        will have zero length.", 
      action = clap::ArgAction::SetTrue
  )]
  exclude_binary_values: bool,

  #[arg(
      long = "select-binary-value",
      value_name = "DATA_ELEMENT_TAG",
      help_heading = "Output",
      help = "Prevents conversion to DICOM JSON of data element values that \
        use one of the binary value representations (OB, OD, OF, OL, OV, OW,UN) \
        except for data elements with the specified tag. This argument can be \
        specified multiple times to include multiple data elements containing \
        binary data in the DICOM JSON output.",
      conflicts_with = "exclude_binary_values",
      value_parser = crate::args::parse_data_element_tag,
  )]
  selected_binary_values: Vec<DataElementTag>,

  #[arg(
    long,
    help_heading = "Output",
    help = "Whether to extend DICOM JSON to store encapsulated pixel data as \
      inline binaries. This is a common extension to the DICOM JSON standard.",
    default_value_t = true
  )]
  store_encapsulated_pixel_data: bool,

  #[arg(
    long = "select",
    value_name = "DATA_ELEMENT_TAG",
    help_heading = "Output",
    help = "The tags of the root data elements to include in the output DICOM \
      JSON. This allows for a subset of data elements to be emitted, rather \
      than the whole data set. This argument can be specified multiple times \
      to include multiple data elements in the output.",
    value_parser = crate::args::parse_data_element_tag,
  )]
  selected_data_elements: Vec<DataElementTag>,
}

enum ToJsonError {
  P10Error(P10Error),
  JsonSerializeError(JsonSerializeError),
}

pub async fn run(args: ToJsonArgs) -> Result<(), ()> {
  crate::validate_output_args(
    args.output_filename.as_ref(),
    args.output_directory.as_ref(),
  )
  .await;

  OutputTarget::set_overwrite(args.overwrite);

  let input_sources = args.input.base.input_sources().await;

  let selected_binary_data_values = if !args.selected_binary_values.is_empty() {
    Some(args.selected_binary_values.clone())
  } else if args.exclude_binary_values {
    Some(vec![])
  } else {
    None
  };

  let config = DicomJsonConfig {
    pretty_print: args.pretty_print,
    store_encapsulated_pixel_data: args.store_encapsulated_pixel_data,
    selected_binary_data_values,
  };

  let result = utils::run_tasks(
    args.concurrency,
    input_sources,
    async |input_source: InputSource| {
      let output_target = if let Some(output_filename) = &args.output_filename {
        OutputTarget::new(output_filename).await
      } else {
        OutputTarget::from_input_source(
          &input_source,
          ".json",
          &args.output_directory,
        )
        .await
      };

      match input_source_to_json(&input_source, output_target, &args, &config)
        .await
      {
        Ok(()) => Ok(()),

        Err(ToJsonError::P10Error(P10Error::DicmPrefixNotPresent))
          if args.input.ignore_invalid =>
        {
          Ok(())
        }

        Err(e) => {
          let task_description = format!("converting \"{input_source}\"");

          Err(match e {
            ToJsonError::P10Error(e) => e.to_lines(&task_description),
            ToJsonError::JsonSerializeError(e) => e.to_lines(&task_description),
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

async fn input_source_to_json(
  input_source: &InputSource,
  output_target: OutputTarget,
  args: &ToJsonArgs,
  json_config: &DicomJsonConfig,
) -> Result<(), ToJsonError> {
  let mut input_stream = input_source
    .open_read_stream()
    .await
    .map_err(ToJsonError::P10Error)?;

  // Open output stream
  let output_stream_handle = output_target
    .open_write_stream(true)
    .await
    .map_err(ToJsonError::P10Error)?;

  let read_config = args
    .input
    .p10_read_config()
    .require_dicm_prefix(args.input.ignore_invalid);

  if args.selected_data_elements.is_empty() {
    // Create P10 read context with a max token size of 256 KiB
    let mut context =
      P10ReadContext::new(Some(read_config.max_token_size(256 * 1024)));

    // Create transform for converting P10 tokens into bytes of JSON
    let mut json_transform = P10JsonTransform::new(json_config.clone());

    // Get exclusive access to the output stream
    let mut output_stream = output_stream_handle.lock().await;

    let mut is_ended = false;

    while !is_ended {
      // Read the next tokens from the input
      let tokens = match dcmfx::p10::read_tokens_from_stream_async(
        &mut input_stream,
        &mut context,
        None,
      )
      .await
      {
        Ok(tokens) => tokens,
        Err(e) => return Err(ToJsonError::P10Error(e)),
      };

      // Write the tokens to the JSON transform, directing the resulting JSON to
      // the output stream
      for token in tokens.iter() {
        let mut cursor = std::io::Cursor::new(vec![]);

        match json_transform.add_token(token, &mut cursor) {
          Ok(()) => output_stream
            .write_all(&cursor.into_inner())
            .await
            .map_err(|e| {
              ToJsonError::P10Error(P10Error::FileError {
                when: "Writing output file".to_string(),
                details: e.to_string(),
              })
            })?,

          Err(e) => return Err(ToJsonError::JsonSerializeError(e)),
        };

        // When the end token has been written the conversion is complete
        if *token == P10Token::End {
          match output_stream.write_all(b"\n").await {
            Ok(()) => (),
            Err(e) => {
              return Err(ToJsonError::P10Error(P10Error::FileError {
                when: "Writing output file".to_string(),
                details: e.to_string(),
              }));
            }
          };

          match output_stream.flush().await {
            Ok(()) => (),
            Err(e) => {
              return Err(ToJsonError::P10Error(P10Error::FileError {
                when: "Writing output file".to_string(),
                details: e.to_string(),
              }));
            }
          };

          is_ended = true;
        }
      }
    }

    output_target
      .commit(&mut output_stream)
      .await
      .map_err(ToJsonError::P10Error)
  } else {
    // Read just the selected tags into a data set
    let data_set = dcmfx::p10::read_stream_partial_async(
      &mut input_stream,
      &args.selected_data_elements,
      Some(read_config),
    )
    .await
    .map_err(ToJsonError::P10Error)?;

    // Convert to DICOM JSON
    let mut dicom_json = data_set
      .to_json(json_config.clone())
      .map_err(ToJsonError::JsonSerializeError)?;

    if !args.pretty_print {
      dicom_json.push('\n');
    }

    // Get exclusive access to the output stream
    let mut output_stream = output_stream_handle.lock().await;

    // Write to output stream
    output_stream
      .write_all(dicom_json.as_bytes())
      .await
      .map_err(|e| {
        ToJsonError::P10Error(P10Error::FileError {
          when: "Writing DICOM JSON to output stream".to_string(),
          details: e.to_string(),
        })
      })?;

    output_target
      .commit(&mut output_stream)
      .await
      .map_err(ToJsonError::P10Error)
  }
}
