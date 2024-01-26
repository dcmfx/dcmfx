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
    default_value_t = String::new()
  )]
  delete_tags: String,
}

fn validate_data_element_tag_list(s: &str) -> Result<String, String> {
  if !s.is_empty() {
    for tag in s.split(",") {
      if DataElementTag::from_hex_string(tag).is_err() {
        return Err("".to_string());
      }
    }
  }

  Ok(s.to_string())
}

pub fn run(args: &ModifyArgs) -> Result<(), ()> {
  // Set the zlib compression level in the write config
  let write_config = P10WriteConfig {
    zlib_compression_level: args.zlib_compression_level,
  };

  // Get the list of tags to be deleted
  let tags_to_delete = if args.delete_tags.is_empty() {
    vec![]
  } else {
    args
      .delete_tags
      .split(",")
      .map(DataElementTag::from_hex_string)
      .collect::<Result<Vec<DataElementTag>, _>>()
      .unwrap()
  };

  let has_tags_to_delete = !tags_to_delete.is_empty();
  let anonymize = args.anonymize;

  // Create a filter transform for anonymization and tag deletion if needed
  let filter_context = if anonymize || has_tags_to_delete {
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
      let _ = std::fs::remove_file(&args.output_filename);

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

/// Rewrites by streaming the parts of the DICOM P10 straight to the output
/// file.
///
fn streaming_rewrite(
  input_filename: &str,
  output_filename: &str,
  write_config: P10WriteConfig,
  output_transfer_syntax: Option<&TransferSyntax>,
  mut filter_context: Option<P10FilterTransform>,
) -> Result<(), P10Error> {
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
    max_part_size: 256 * 1024,
    ..P10ReadConfig::default()
  });
  let mut p10_write_context = P10WriteContext::new();
  p10_write_context.set_config(&write_config);

  // Stream P10 parts from the input stream to the output stream
  loop {
    // Read the next P10 parts from the input stream
    let parts = dcmfx::p10::read_parts_from_stream(
      &mut input_stream,
      &mut p10_read_context,
    )?;

    // Pass parts through the filter if one is specified
    let parts = if let Some(filter_context) = filter_context.as_mut() {
      parts
        .into_iter()
        .filter(|part| filter_context.add_part(part))
        .collect()
    } else {
      parts
    };

    let received_end_part = parts.last() == Some(&P10Part::End);

    // Write all parts to the write context
    for mut part in parts {
      // If converting the transfer syntax then update the transfer syntax in
      // the File Meta Information part
      if let Some(ts) = output_transfer_syntax {
        if let P10Part::FileMetaInformation {
          data_set: ref mut fmi,
        } = part
        {
          change_transfer_syntax(fmi, ts)?;
        }
      }

      p10_write_context.write_part(&part)?;
    }

    // Write bytes from the write context to the output stream
    let p10_bytes = p10_write_context.read_bytes();
    for bytes in p10_bytes {
      match output_stream.write_all(&bytes) {
        Ok(_) => Ok(()),
        Err(e) => Err(P10Error::FileError {
          when: "Writing to output file".to_string(),
          details: e.to_string(),
        }),
      }?;
    }

    // Stop when the end part is received
    if received_end_part {
      break;
    }
  }

  if let Err(e) = output_stream.flush() {
    return Err(P10Error::FileError {
      when: format!("Closing output file \"{}\"", output_filename),
      details: e.to_string(),
    });
  }

  Ok(())
}

/// Adds/updates the '(0002,0010) TransferSyntaxUID' data element in the data
/// set. If the current '(0002,0010) TransferSyntaxUID' is not able to be
/// converted from then an error is returned.
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
        &registry::TRANSFER_SYNTAX_UID,
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
