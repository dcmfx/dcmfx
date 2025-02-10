use std::fs::File;
use std::io::{Read, Write};

use clap::Args;

use dcmfx::core::*;
use dcmfx::json::*;
use dcmfx::p10::*;

pub const ABOUT: &str = "Converts a DICOM P10 file to a DICOM JSON file";

#[derive(Args)]
pub struct ToJsonArgs {
  #[clap(
    help = "The name of the file to read DICOM P10 content from. Specify '-' \
      to read from stdin."
  )]
  input_filename: String,

  #[clap(
    help = "The name of the file to write DICOM JSON content to. Specify '-' \
      to write to stdout."
  )]
  output_filename: String,

  #[arg(
    long = "pretty",
    help = "Whether to format the DICOM JSON for readability with newlines and \
      indentation",
    default_value_t = false
  )]
  pretty_print: bool,

  #[arg(
    long,
    short = 'p',
    help = "Whether to extend DICOM JSON to store encapsulated pixel data as \
      inline binaries",
    default_value_t = false
  )]
  store_encapsulated_pixel_data: bool,
}

pub fn run(args: &ToJsonArgs) -> Result<(), ()> {
  let config = DicomJsonConfig {
    pretty_print: args.pretty_print,
    store_encapsulated_pixel_data: args.store_encapsulated_pixel_data,
  };

  match perform_to_json(&args.input_filename, &args.output_filename, &config) {
    Ok(()) => Ok(()),
    Err(e) => {
      let task_description =
        &format!("converting \"{}\" to JSON", args.input_filename);

      match e {
        ToJsonError::SerializeError(e) => e.print(task_description),
        ToJsonError::P10Error(e) => e.print(task_description),
      }

      Err(())
    }
  }
}

enum ToJsonError {
  SerializeError(JsonSerializeError),
  P10Error(P10Error),
}

fn perform_to_json(
  input_filename: &str,
  output_filename: &str,
  config: &DicomJsonConfig,
) -> Result<(), ToJsonError> {
  // Open input stream
  let mut input_stream: Box<dyn Read> = match input_filename {
    "-" => Box::new(std::io::stdin()),
    _ => match File::open(input_filename) {
      Ok(file) => Box::new(file),
      Err(e) => {
        return Err(ToJsonError::P10Error(P10Error::FileError {
          when: "Opening input file".to_string(),
          details: e.to_string(),
        }));
      }
    },
  };

  // Open output stream
  let mut output_stream: Box<dyn Write> = match output_filename {
    "-" => Box::new(std::io::stdout()),
    _ => match File::create(output_filename) {
      Ok(file) => Box::new(file),
      Err(e) => {
        return Err(ToJsonError::P10Error(P10Error::FileError {
          when: "Opening output file".to_string(),
          details: e.to_string(),
        }));
      }
    },
  };

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
        Err(e) => return Err(ToJsonError::SerializeError(e)),
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
