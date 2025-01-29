use std::fs::File;
use std::io::{Read, Write};

use clap::Args;

use dcmfx::core::*;
use dcmfx::p10::*;

pub const ABOUT: &str = "Reads a DICOM P10 file, applies requested \
  modifications, and writes out a new DICOM P10 file";

#[derive(Args)]
pub struct ModifyArgs {
  #[clap(
    help = "The name of the file to read DICOM P10 content from. Specify '-' \
      to read from stdin."
  )]
  input_filename: String,

  #[clap(
    help = "The name of the file to write DICOM P10 content to. Specify '-' to \
      write to stdout."
  )]
  output_filename: String,

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
    short,
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
    short,
    help = "Whether to anonymize the output DICOM P10 file by removing all \
      patient data elements, other identifying data elements, as well as \
      private data elements. Note that this option does not remove any \
      identifying information that may be baked into the pixel data.",
    default_value_t = false
  )]
  anonymize: bool,

  #[arg(
    long,
    short,
    help = "The data element tags to delete and not include in the output \
      DICOM P10 file. Separate each tag to be removed with a comma. E.g. \
      --delete-tags 00100010,00100030",
    value_parser = validate_data_element_tag_list,
    default_values_t = Vec::<DataElementTag>::new()
  )]
  delete_tags: Vec<DataElementTag>,
}

fn validate_data_element_tag_list(
  s: &str,
) -> Result<Vec<DataElementTag>, String> {
  s.split(",")
    .map(|tag| match DataElementTag::from_hex_string(tag) {
      Ok(tag) => Ok(tag),
      Err(_) => Err("".to_string()),
    })
    .collect()
}

pub fn run(args: &ModifyArgs) -> Result<(), ()> {
  // Set the zlib compression level in the write config
  let write_config = P10WriteConfig {
    zlib_compression_level: args.zlib_compression_level,
  };

  let anonymize = args.anonymize;
  let tags_to_delete = args.delete_tags.clone();

  // Create a filter transform for anonymization and tag deletion if needed
  let filter_context = if anonymize || !tags_to_delete.is_empty() {
    Some(P10FilterTransform::new(
      Box::new(move |tag, vr, _| {
        (!anonymize || dcmfx::anonymize::filter_tag(tag, vr))
          && !tags_to_delete.contains(&tag)
      }),
      false,
    ))
  } else {
    None
  };

  let modify_result = match parse_transfer_syntax_flag(&args.transfer_syntax) {
    Ok(output_transfer_syntax) => streaming_rewrite(
      &args.input_filename,
      &args.output_filename,
      write_config,
      output_transfer_syntax,
      filter_context,
    ),

    Err(e) => Err(e),
  };

  match modify_result {
    Ok(_) => Ok(()),
    Err(e) => {
      // Delete any partially written file
      if args.output_filename != "-" {
        let _ = std::fs::remove_file(&args.output_filename);
      }

      e.print(&format!("modifying file \"{}\"", args.input_filename));
      Err(())
    }
  }
}

/// Detects and validates the value passed to --transfer-syntax, if present.
///
fn parse_transfer_syntax_flag(
  transfer_syntax_flag: &Option<String>,
) -> Result<Option<&TransferSyntax>, P10Error> {
  if let Some(transfer_syntax_value) = transfer_syntax_flag {
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
  } else {
    Ok(None)
  }
}

/// Rewrites by streaming the tokens of the DICOM P10 straight to the output
/// file.
///
fn streaming_rewrite(
  input_filename: &str,
  output_filename: &str,
  write_config: P10WriteConfig,
  output_transfer_syntax: Option<&TransferSyntax>,
  mut filter_context: Option<P10FilterTransform>,
) -> Result<(), P10Error> {
  // Check that the input and output filenames don't point to the same
  // underlying file. In-place modification isn't supported because of the
  // stream-based implementation.
  if input_filename != "-" && output_filename != "-" {
    if let Ok(true) = same_file::is_same_file(input_filename, output_filename) {
      return Err(P10Error::OtherError {
        error_type: "Filename error".to_string(),
        details: "Input and output files must be different".to_string(),
      });
    }
  }

  // Open input stream
  let mut input_stream: Box<dyn Read> = match input_filename {
    "-" => Box::new(std::io::stdin()),
    _ => match File::open(input_filename) {
      Ok(file) => Box::new(file),
      Err(e) => {
        return Err(P10Error::FileError {
          when: "Opening input file".to_string(),
          details: e.to_string(),
        });
      }
    },
  };

  // Open output stream
  let mut output_stream: Box<dyn Write> = match output_filename {
    "-" => Box::new(std::io::stdout()),
    _ => match File::create(output_filename) {
      Ok(file) => Box::new(file),
      Err(e) => {
        return Err(P10Error::FileError {
          when: format!("Opening output file \"{}\"", output_filename),
          details: e.to_string(),
        });
      }
    },
  };

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

    // If converting the transfer syntax then update the transfer syntax in the
    // File Meta Information token
    if let Some(ts) = output_transfer_syntax {
      for token in tokens.iter_mut() {
        if let P10Token::FileMetaInformation {
          data_set: ref mut fmi,
        } = token
        {
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
