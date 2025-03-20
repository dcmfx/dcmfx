use std::{io::Write, path::PathBuf};

use clap::Args;

use dcmfx::core::*;
use dcmfx::json::*;
use dcmfx::p10::*;

use crate::{InputSource, utils};

pub const ABOUT: &str = "Converts DICOM P10 files to DICOM JSON files";

#[derive(Args)]
pub struct ToJsonArgs {
  #[clap(
    required = true,
    help = "The names of the DICOM P10 files to convert to DICOM JSON files. \
      Specify '-' to read from stdin."
  )]
  input_filenames: Vec<PathBuf>,

  #[clap(
    long,
    short,
    help = "The name of the output DICOM JSON file. This option is only valid \
      when a single input filename is specified. Specify '-' to write to \
      stdout."
  )]
  output_filename: Option<PathBuf>,

  #[arg(
    long = "pretty",
    help = "Whether to format the DICOM JSON for readability with newlines and \
      indentation",
    default_value_t = false
  )]
  pretty_print: bool,

  #[arg(
    long,
    help = "Whether to extend DICOM JSON to store encapsulated pixel data as \
      inline binaries",
    default_value_t = true
  )]
  store_encapsulated_pixel_data: bool,

  #[clap(
    long = "force",
    short = 'f',
    help = "Overwrite files without prompting",
    default_value_t = false
  )]
  force_overwrite: bool,
}

enum ToJsonError {
  P10Error(P10Error),
  JsonSerializeError(JsonSerializeError),
}

pub fn run(args: &ToJsonArgs) -> Result<(), ()> {
  let input_sources = crate::get_input_sources(&args.input_filenames);

  if input_sources.len() > 1 && args.output_filename.is_some() {
    eprintln!(
      "When there are multiple input files --output-filename must not be \
       specified"
    );
    return Err(());
  }

  let config = DicomJsonConfig {
    pretty_print: args.pretty_print,
    store_encapsulated_pixel_data: args.store_encapsulated_pixel_data,
  };

  for input_source in input_sources {
    match input_source_to_json(&input_source, args, &config) {
      Ok(()) => (),

      Err(e) => {
        let task_description = format!("converting \"{}\"", input_source);

        match e {
          ToJsonError::P10Error(e) => e.print(&task_description),
          ToJsonError::JsonSerializeError(e) => e.print(&task_description),
        }

        return Err(());
      }
    }
  }

  Ok(())
}

fn input_source_to_json(
  input_source: &InputSource,
  args: &ToJsonArgs,
  config: &DicomJsonConfig,
) -> Result<(), ToJsonError> {
  let mut input_stream = input_source
    .open_read_stream()
    .map_err(ToJsonError::P10Error)?;

  let output_filename = args
    .output_filename
    .clone()
    .unwrap_or_else(|| input_source.clone().append(".json"));

  // Open output stream
  let mut output_stream: Box<dyn Write> = utils::open_output_stream(
    &output_filename,
    Some(&output_filename),
    args.force_overwrite,
  )
  .map_err(ToJsonError::P10Error)?;

  // Create P10 read context and set max token size to 256 KiB
  let mut context = P10ReadContext::new();
  context.set_config(&P10ReadConfig {
    max_token_size: 256 * 1024,
    ..P10ReadConfig::default()
  });

  // Create transform for converting P10 tokens into bytes of JSON
  let mut json_transform = P10JsonTransform::new(config);

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
      match json_transform.add_token(token, &mut output_stream) {
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
}
