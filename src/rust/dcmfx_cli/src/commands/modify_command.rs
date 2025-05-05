use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::PathBuf;

use clap::Args;
use rand::Rng;

use dcmfx::core::*;
use dcmfx::p10::*;
use dcmfx::pixel_data::*;

use crate::utils::prompt_to_overwrite_if_exists;
use crate::{InputSource, transfer_syntax_arg::TransferSyntaxArg, utils};

pub const ABOUT: &str = "Modifies the content of DICOM P10 files";

#[derive(Args)]
pub struct ModifyArgs {
  #[clap(
    required = true,
    help = "The names of the DICOM P10 files to modify. Specify '-' to read \
      from stdin."
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

  #[arg(
    long,
    help = "Whether to modify the input files in place, i.e. overwrite them \
      with the newly modified version rather than write it to a new file. \
      WARNING: this is a potentially irreversible operation.",
    default_value_t = false
  )]
  in_place: bool,

  #[arg(
    long,
    short,
    help = "The transfer syntax for the output DICOM P10 file. Pixel data will \
      be automatically transcoded. For some transfer syntaxes additional \
      arguments are available to control image compression."
  )]
  transfer_syntax: Option<TransferSyntaxArg>,

  #[arg(
    long,
    short,
    help = "When transcoding to the 'jpeg-baseline-8bit' transfer syntax, \
      specifies the JPEG quality level in the range 1-100.",
    default_value_t = 85,
    value_parser = clap::value_parser!(u8).range(1..=100),
  )]
  quality: u8,

  #[arg(
    long,
    help = "\
      The zlib compression level to use when outputting to the 'Deflated \
      Explicit VR Little Endian' and 'Deflated Image Frame Compression' \
      transfer syntaxes. The level ranges from 0, meaning no compression, \
      through to 9, which gives the best compression at the cost of speed.",
    default_value_t = 6,
    value_parser = clap::value_parser!(u32).range(0..=9),
  )]
  zlib_compression_level: u32,

  #[arg(
    long,
    help = "Whether to anonymize the output DICOM P10 file by removing all \
      patient data elements, other identifying data elements, as well as \
      private data elements. Note that this option does not remove any \
      identifying information that may be baked into the pixel data.",
    default_value_t = false
  )]
  anonymize: bool,

  #[arg(
    long,
    help = "A data element tag to delete and not include in the output DICOM \
      P10 file. This argument can be specified multiple times to delete \
      multiple tags.",
    value_parser = validate_data_element_tag,
  )]
  delete_tag: Vec<DataElementTag>,

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

fn validate_data_element_tag(s: &str) -> Result<DataElementTag, String> {
  DataElementTag::from_hex_string(s)
    .map_err(|_| "Invalid data element tag".to_string())
}

enum ModifyCommandError {
  P10Error(P10Error),
  P10PixelDataTranscodeTransformError(P10PixelDataTranscodeTransformError),
}

pub fn run(args: &ModifyArgs) -> Result<(), ()> {
  let input_sources = crate::get_input_sources(&args.input_filenames);

  if !(args.in_place ^ args.output_filename.is_some()) {
    eprintln!(
      "Exactly one of --output-filename or --in-place must be specified"
    );
    return Err(());
  }

  if input_sources.len() > 1 && args.output_filename.is_some() {
    eprintln!(
      "When there are multiple input files --output-filename must not be specified"
    );
    return Err(());
  }

  if args.in_place && input_sources.contains(&InputSource::Stdin) {
    eprintln!("When reading from stdin --in-place must not be specified");
    return Err(());
  }

  for input_source in input_sources {
    match modify_input_source(&input_source, args) {
      Ok(()) => (),

      Err(e) => {
        let task_description = format!("modifying \"{}\"", input_source);

        match e {
          ModifyCommandError::P10Error(e) => e.print(&task_description),
          ModifyCommandError::P10PixelDataTranscodeTransformError(e) => {
            e.print(&task_description);
          }
        }

        return Err(());
      }
    }
  }

  Ok(())
}

fn modify_input_source(
  input_source: &InputSource,
  args: &ModifyArgs,
) -> Result<(), ModifyCommandError> {
  let output_filename = args
    .output_filename
    .clone()
    .unwrap_or_else(|| input_source.path().unwrap().clone());

  if output_filename != PathBuf::from("-") {
    if args.in_place {
      println!("Modifying \"{}\" in place …", input_source,);
    } else {
      println!(
        "Modifying \"{}\" => \"{}\" …",
        input_source,
        output_filename.display()
      );
    }

    if !args.in_place && !args.overwrite {
      prompt_to_overwrite_if_exists(&output_filename);
    }
  }

  // Append a random suffix to get a unique name for a temporary output file.
  // This isn't needed when outputting to stdout.
  let tmp_output_filename = if output_filename == PathBuf::from("-") {
    None
  } else {
    let mut rng = rand::rng();
    let random_suffix: String = (0..16)
      .map(|_| char::from(rng.sample(rand::distr::Alphanumeric)))
      .collect();

    let file_name = output_filename.file_name().unwrap_or(OsStr::new(""));
    let file_name =
      format!("{}.{}.tmp", file_name.to_string_lossy(), random_suffix);

    let mut new_path = output_filename.clone();
    new_path.set_file_name(file_name);

    Some(new_path)
  };

  // Create a filter transform for anonymization and tag deletion if needed
  let tags_to_delete = args.delete_tag.clone();
  let anonymize = args.anonymize;
  let filter_context = if anonymize || !tags_to_delete.is_empty() {
    Some(P10FilterTransform::new(Box::new(
      move |tag, vr, _length, _location| {
        (!anonymize || dcmfx::anonymize::filter_tag(tag, vr))
          && !tags_to_delete.contains(&tag)
      },
    )))
  } else {
    None
  };

  // Setup write config
  let write_config = P10WriteConfig {
    implementation_version_name: args.implementation_version_name.clone(),
    zlib_compression_level: args.zlib_compression_level,
    ..P10WriteConfig::default()
  };

  // Setup a pixel data transcode transform if an output transfer syntax is
  // specified
  let pixel_data_transcode_transform =
    if let Some(output_transfer_syntax) = args.transfer_syntax {
      let mut pixel_data_encode_config = PixelDataEncodeConfig::new();
      pixel_data_encode_config.set_quality(args.quality);
      pixel_data_encode_config
        .set_zlib_compression_level(args.zlib_compression_level);

      Some(P10PixelDataTranscodeTransform::new(
        output_transfer_syntax.as_transfer_syntax(),
        pixel_data_encode_config,
      ))
    } else {
      None
    };

  let input_stream = input_source
    .open_read_stream()
    .map_err(ModifyCommandError::P10Error)?;

  streaming_rewrite(
    input_stream,
    tmp_output_filename.as_ref().unwrap_or(&output_filename),
    write_config,
    filter_context,
    pixel_data_transcode_transform,
  )?;

  // Rename the temporary file to the desired output filename
  if output_filename != PathBuf::from("-") {
    if let Some(tmp_output_filename) = tmp_output_filename {
      std::fs::rename(&tmp_output_filename, &output_filename).map_err(|e| {
        ModifyCommandError::P10Error(P10Error::FileError {
          when: format!(
            "Renaming '{}' to '{}'",
            tmp_output_filename.display(),
            output_filename.display()
          ),
          details: e.to_string(),
        })
      })?;
    }
  }

  Ok(())
}

/// Rewrites by streaming the tokens of the DICOM P10 straight to the output
/// file.
///
fn streaming_rewrite(
  mut input_stream: Box<dyn Read>,
  output_filename: &PathBuf,
  write_config: P10WriteConfig,
  mut filter_context: Option<P10FilterTransform>,
  mut pixel_data_transcode_transform: Option<P10PixelDataTranscodeTransform>,
) -> Result<(), ModifyCommandError> {
  // Open output stream
  let mut output_stream: Box<dyn Write> =
    utils::open_output_stream(output_filename, None, false)
      .map_err(ModifyCommandError::P10Error)?;

  // Create read and write contexts
  let mut p10_read_context = P10ReadContext::new();
  p10_read_context.set_config(&P10ReadConfig {
    max_token_size: 256 * 1024,
    ..P10ReadConfig::default()
  });
  let mut p10_write_context = P10WriteContext::new();
  p10_write_context.set_config(&write_config);

  // Stream P10 tokens from the input stream to the output stream
  loop {
    // Read the next P10 tokens from the input stream
    let mut tokens = dcmfx::p10::read_tokens_from_stream(
      &mut input_stream,
      &mut p10_read_context,
    )
    .map_err(ModifyCommandError::P10Error)?;

    // Pass tokens through the pixel data transcode transform if one is active
    if let Some(pixel_data_transcode_transform) =
      pixel_data_transcode_transform.as_mut()
    {
      let mut new_tokens = vec![];

      for token in tokens.iter() {
        new_tokens.extend(
          pixel_data_transcode_transform
            .add_token(token)
            .map_err(ModifyCommandError::P10PixelDataTranscodeTransformError)?,
        );
      }

      tokens = new_tokens
    }

    // Pass tokens through the filter if one is specified
    let tokens = if let Some(filter_context) = filter_context.as_mut() {
      tokens.into_iter().try_fold(vec![], |mut acc, token| {
        if filter_context
          .add_token(&token)
          .map_err(ModifyCommandError::P10Error)?
        {
          acc.push(token);
        }

        Ok(acc)
      })
    } else {
      Ok(tokens)
    }?;

    // Write tokens to the output stream
    let ended = dcmfx::p10::write_tokens_to_stream(
      &tokens,
      &mut output_stream,
      &mut p10_write_context,
    )
    .map_err(ModifyCommandError::P10Error)?;

    // Stop when the end token is received
    if ended {
      break;
    }
  }

  Ok(())
}
