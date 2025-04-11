use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::PathBuf;

use clap::Args;
use rand::Rng;

use dcmfx::core::*;
use dcmfx::p10::*;

use crate::utils::prompt_to_overwrite_if_exists;
use crate::{InputSource, utils};

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
    help = "The transfer syntax for the output DICOM P10 file. This can only \
      convert between the following transfer syntaxes: \
      'implicit-vr-little-endian', 'explicit-vr-little-endian', \
      'deflated-explicit-vr-little-endian', and 'explicit-vr-big-endian'."
  )]
  transfer_syntax: Option<String>,

  #[arg(
    long,
    help = "\
      The zlib compression level to use when outputting to the 'Deflated \
      Explicit VR Little Endian' transfer syntax. The level ranges from 0, \
      meaning no compression, through to 9, which gives the best compression \
      at the cost of speed.",
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
}

fn validate_data_element_tag(s: &str) -> Result<DataElementTag, String> {
  DataElementTag::from_hex_string(s)
    .map_err(|_| "Invalid data element tag".to_string())
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
        e.print(&format!("modifying \"{}\"", input_source));
        return Err(());
      }
    }
  }

  Ok(())
}

fn modify_input_source(
  input_source: &InputSource,
  args: &ModifyArgs,
) -> Result<(), P10Error> {
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
    zlib_compression_level: args.zlib_compression_level,
  };

  let input_stream = input_source.open_read_stream()?;

  streaming_rewrite(
    input_stream,
    tmp_output_filename.as_ref().unwrap_or(&output_filename),
    write_config,
    filter_context,
    args,
  )?;

  // Rename the temporary file to the desired output filename
  if output_filename != PathBuf::from("-") {
    if let Some(tmp_output_filename) = tmp_output_filename {
      std::fs::rename(&tmp_output_filename, &output_filename).map_err(|e| {
        P10Error::FileError {
          when: format!(
            "Renaming '{}' to '{}'",
            tmp_output_filename.display(),
            output_filename.display()
          ),
          details: e.to_string(),
        }
      })?;
    }
  }

  Ok(())
}

/// Detects and validates the value passed to --transfer-syntax, if present.
///
fn parse_transfer_syntax_flag(
  transfer_syntax_flag: &Option<String>,
) -> Result<Option<&TransferSyntax>, P10Error> {
  let Some(transfer_syntax_value) = transfer_syntax_flag else {
    return Ok(None);
  };

  match transfer_syntax_value.as_str() {
    "implicit-vr-little-endian" => {
      Ok(Some(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN))
    }
    "explicit-vr-little-endian" => {
      Ok(Some(&transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN))
    }
    "deflated-explicit-vr-little-endian" => {
      Ok(Some(&transfer_syntax::DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN))
    }
    "explicit-vr-big-endian" => {
      Ok(Some(&transfer_syntax::EXPLICIT_VR_BIG_ENDIAN))
    }

    _ => Err(P10Error::OtherError {
      error_type: "Unsupported transfer syntax conversion".to_string(),
      details: format!(
        "The transfer syntax '{}' is not recognized",
        transfer_syntax_value
      ),
    }),
  }
}

/// Rewrites by streaming the tokens of the DICOM P10 straight to the output
/// file.
///
fn streaming_rewrite(
  mut input_stream: Box<dyn Read>,
  output_filename: &PathBuf,
  write_config: P10WriteConfig,
  mut filter_context: Option<P10FilterTransform>,
  args: &ModifyArgs,
) -> Result<(), P10Error> {
  // Open output stream
  let mut output_stream: Box<dyn Write> =
    utils::open_output_stream(output_filename, None, false)?;

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
    let tokens = dcmfx::p10::read_tokens_from_stream(
      &mut input_stream,
      &mut p10_read_context,
    )?;

    // Pass tokens through the filter if one is specified
    let mut tokens = if let Some(filter_context) = filter_context.as_mut() {
      tokens
        .into_iter()
        .filter(|token| filter_context.add_token(token))
        .collect()
    } else {
      tokens
    };

    let output_transfer_syntax =
      parse_transfer_syntax_flag(&args.transfer_syntax)?;

    // If converting the transfer syntax then update the transfer syntax in the
    // File Meta Information token
    if let Some(ts) = output_transfer_syntax {
      for token in tokens.iter_mut() {
        if let P10Token::FileMetaInformation { data_set: fmi } = token {
          change_transfer_syntax(fmi, ts)?;
        }
      }
    }

    // Write tokens to the output stream
    let ended = dcmfx::p10::write_tokens_to_stream(
      &tokens,
      &mut output_stream,
      &mut p10_write_context,
    )?;

    // Stop when the end token is received
    if ended {
      break;
    }
  }

  Ok(())
}

/// Adds/updates the *'(0002,0010) TransferSyntaxUID'* data element in the data
/// set. If the current transfer syntax is not able to be converted from then an
/// error is returned.
///
fn change_transfer_syntax(
  data_set: &mut DataSet,
  output_transfer_syntax: &TransferSyntax,
) -> Result<(), P10Error> {
  // Read the current transfer syntax, defaulting to 'Implicit VR Little Endian'
  let transfer_syntax = data_set
    .get_transfer_syntax()
    .unwrap_or(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN);

  // The list of transfer syntaxes that can be converted from
  let valid_source_ts = [
    transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN,
    transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN,
    transfer_syntax::DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN,
    transfer_syntax::EXPLICIT_VR_BIG_ENDIAN,
  ];

  if valid_source_ts.contains(transfer_syntax) {
    data_set
      .insert_string_value(
        &dictionary::TRANSFER_SYNTAX_UID,
        &[output_transfer_syntax.uid],
      )
      .unwrap();

    Ok(())
  } else {
    Err(P10Error::OtherError {
      error_type: "Unsupported transfer syntax conversion".to_string(),
      details: format!(
        "The transfer syntax '{}' is not able to be converted from",
        transfer_syntax.name
      ),
    })
  }
}
