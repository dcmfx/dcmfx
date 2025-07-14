use std::{io::Write, path::PathBuf};

use clap::Args;
use rayon::prelude::*;

use dcmfx::core::*;
use dcmfx::json::*;
use dcmfx::p10::*;

use crate::{args::input_args::InputSource, utils};

pub const ABOUT: &str = "Converts DICOM P10 files to DICOM JSON files";

#[derive(Args)]
pub struct ToJsonArgs {
  #[arg(
    long,
    help = "The number of threads to use to perform work.",
    default_value_t = rayon::current_num_threads()
  )]
  threads: usize,

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
    help = "Overwrite files without prompting",
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
    value_parser = crate::args::validate_data_element_tag,
  )]
  selected_data_elements: Vec<DataElementTag>,
}

enum ToJsonError {
  P10Error(P10Error),
  JsonSerializeError(JsonSerializeError),
}

pub fn run(args: &mut ToJsonArgs) -> Result<(), ()> {
  crate::validate_output_args(&args.output_filename, &args.output_directory);

  let input_sources = args.input.base.create_iterator();

  let config = DicomJsonConfig {
    pretty_print: args.pretty_print,
    store_encapsulated_pixel_data: args.store_encapsulated_pixel_data,
  };

  let result = utils::create_thread_pool(args.threads).install(move || {
    input_sources.par_bridge().try_for_each(|input_source| {
      if args.input.ignore_invalid && !input_source.is_dicom_p10() {
        return Ok(());
      }

      let output_filename = if let Some(output_filename) = &args.output_filename
      {
        output_filename.clone()
      } else {
        input_source.output_path(".json", &args.output_directory)
      };

      match input_source_to_json(&input_source, output_filename, args, config) {
        Ok(()) => Ok(()),

        Err(e) => {
          let task_description = format!("converting \"{input_source}\"");

          Err(match e {
            ToJsonError::P10Error(e) => e.to_lines(&task_description),
            ToJsonError::JsonSerializeError(e) => e.to_lines(&task_description),
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

fn input_source_to_json(
  input_source: &InputSource,
  output_filename: PathBuf,
  args: &ToJsonArgs,
  json_config: DicomJsonConfig,
) -> Result<(), ToJsonError> {
  let mut input_stream = input_source
    .open_read_stream()
    .map_err(ToJsonError::P10Error)?;

  // Open output stream
  let output_stream = utils::open_output_stream(
    &output_filename,
    Some(&output_filename),
    args.overwrite,
  )
  .map_err(ToJsonError::P10Error)?;

  let read_config = args.input.p10_read_config();

  if args.selected_data_elements.is_empty() {
    // Create P10 read context with a max token size of 256 KiB
    let mut context =
      P10ReadContext::new(Some(read_config.max_token_size(256 * 1024)));

    // Create transform for converting P10 tokens into bytes of JSON
    let mut json_transform = P10JsonTransform::new(json_config);

    // Get exclusive access to the output stream
    let mut output_stream = output_stream.lock().unwrap();

    loop {
      // Read the next tokens from the input
      let tokens = match dcmfx::p10::read_tokens_from_stream(
        &mut input_stream,
        &mut context,
      ) {
        Ok(tokens) => tokens,
        Err(e) => return Err(ToJsonError::P10Error(e)),
      };

      // Write the tokens to the JSON transform, directing the resulting JSON to
      // the output stream
      for token in tokens.iter() {
        match json_transform.add_token(token, &mut *output_stream) {
          Ok(()) => (),
          Err(JsonSerializeError::IOError(e)) => {
            return Err(ToJsonError::P10Error(P10Error::FileError {
              when: "Writing output file".to_string(),
              details: e.to_string(),
            }));
          }
          Err(e) => return Err(ToJsonError::JsonSerializeError(e)),
        };

        // When the end token has been written the conversion is complete
        if *token == P10Token::End {
          return match output_stream.flush() {
            Ok(()) => Ok(()),
            Err(e) => Err(ToJsonError::P10Error(P10Error::FileError {
              when: "Writing output file".to_string(),
              details: e.to_string(),
            })),
          };
        }
      }
    }
  } else {
    // Read just the selected tags into a data set
    let data_set = dcmfx::p10::read_stream_partial(
      &mut input_stream,
      &args.selected_data_elements,
      Some(read_config),
    )
    .map_err(ToJsonError::P10Error)?;

    // Convert to DICOM JSON
    let mut dicom_json = data_set
      .to_json(json_config)
      .map_err(ToJsonError::JsonSerializeError)?;

    if !args.pretty_print {
      dicom_json.push('\n');
    }

    // Write to output stream
    output_stream
      .lock()
      .unwrap()
      .write_all(dicom_json.as_bytes())
      .map_err(|e| {
        ToJsonError::P10Error(P10Error::FileError {
          when: "Writing DICOM JSON to output stream".to_string(),
          details: e.to_string(),
        })
      })?;

    Ok(())
  }
}
