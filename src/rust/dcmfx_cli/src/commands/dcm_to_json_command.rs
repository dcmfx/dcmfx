use std::{io::Write, path::PathBuf};

use clap::Args;
use rayon::prelude::*;

use dcmfx::core::*;
use dcmfx::json::*;
use dcmfx::p10::*;

use crate::{InputSource, utils};

pub const ABOUT: &str = "Converts DICOM P10 files to DICOM JSON files";

#[derive(Args)]
pub struct ToJsonArgs {
  #[arg(
    required = true,
    help = "The names of the DICOM P10 files to convert to DICOM JSON files. \
      Specify '-' to read from stdin."
  )]
  input_filenames: Vec<PathBuf>,

  #[arg(
    long,
    help = "Whether to ignore input files that don't contain DICOM P10 data.",
    default_value_t = false
  )]
  ignore_invalid: bool,

  #[arg(
    long,
    short,
    help = "The name of the DICOM JSON output file. By default the output \
      DICOM JSON file is the name of the input file with '.json' appended. \
      Specify '-' to write to stdout.\n\
      \n\
      This argument is not permitted when multiple input files are specified."
  )]
  output_filename: Option<PathBuf>,

  #[arg(
    long,
    short = 'd',
    help = "The directory to write output files into. The names of the output \
      DICOM JSON files will be the name of the input file with '.json' \
      appended."
  )]
  output_directory: Option<PathBuf>,

  #[arg(
    long,
    help = "The number of threads to use to perform work. Each thread operates \
      on one input file at a time, so using more threads may improve \
      performance when processing many input files.\n\
      \n\
      The default thread count is the number of logical CPUs available.",
    default_value_t = rayon::current_num_threads()
  )]
  threads: usize,

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
      inline binaries. This is a common extension to the DICOM JSON standard.",
    default_value_t = true
  )]
  store_encapsulated_pixel_data: bool,

  #[arg(
    long,
    help = "Overwrite files without prompting",
    default_value_t = false
  )]
  overwrite: bool,
}

enum ToJsonError {
  P10Error(P10Error),
  JsonSerializeError(JsonSerializeError),
}

pub fn run(args: &ToJsonArgs) -> Result<(), ()> {
  let input_sources = crate::get_input_sources(&args.input_filenames);

  crate::validate_output_args(
    &input_sources,
    &args.output_filename,
    &args.output_directory,
  );

  let config = DicomJsonConfig {
    pretty_print: args.pretty_print,
    store_encapsulated_pixel_data: args.store_encapsulated_pixel_data,
  };

  let result = utils::create_thread_pool(args.threads).install(move || {
    input_sources.into_par_iter().try_for_each(|input_source| {
      if args.ignore_invalid && !input_source.is_dicom_p10() {
        return Ok(());
      }

      let output_filename = if let Some(output_filename) = &args.output_filename
      {
        output_filename.clone()
      } else {
        input_source.output_path(".json", &args.output_directory)
      };

      match input_source_to_json(&input_source, output_filename, args, &config)
      {
        Ok(()) => Ok(()),

        Err(e) => {
          let task_description = format!("converting \"{}\"", input_source);

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
  config: &DicomJsonConfig,
) -> Result<(), ToJsonError> {
  let mut input_stream = input_source
    .open_read_stream()
    .map_err(ToJsonError::P10Error)?;

  // Open output stream
  let mut output_stream: Box<dyn Write> = utils::open_output_stream(
    &output_filename,
    Some(&output_filename),
    args.overwrite,
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
