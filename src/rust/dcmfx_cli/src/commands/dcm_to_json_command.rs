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
    help = "DICOM P10 files to convert to DICOM JSON files. Specify '-' to \
      read from stdin."
  )]
  input_filenames: Vec<PathBuf>,

  #[arg(long, help = crate::args::file_list_arg::ABOUT)]
  file_list: Option<PathBuf>,

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
      Specify '-' to write to stdout."
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
      When outputting to stdout only one thread can be used.\n\
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

  #[arg(
    long = "select",
    help = "The tags of the root data elements to include in the output DICOM \
      JSON. This allows for a subset of data elements to be emitted, rather \
      than the whole data set. Specify this argument multiple times to include \
      more than one data element in the output.",
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

  let input_sources = crate::input_source::create_iterator(
    &mut args.input_filenames,
    &args.file_list,
  );

  let config = DicomJsonConfig {
    pretty_print: args.pretty_print,
    store_encapsulated_pixel_data: args.store_encapsulated_pixel_data,
  };

  let result = utils::create_thread_pool(args.threads).install(move || {
    input_sources.par_bridge().try_for_each(|input_source| {
      if args.ignore_invalid && !input_source.is_dicom_p10() {
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
  config: DicomJsonConfig,
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

  if args.selected_data_elements.is_empty() {
    // Create P10 read context with max token size to 256 KiB
    let mut context = P10ReadContext::new(Some(
      P10ReadConfig::default().max_token_size(256 * 1024),
    ));

    // Create transform for converting P10 tokens into bytes of JSON
    let mut json_transform = P10JsonTransform::new(config);

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
      None,
    )
    .map_err(ToJsonError::P10Error)?;

    // Convert to DICOM JSON
    let mut dicom_json = data_set
      .to_json(config)
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
