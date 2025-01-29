use std::fs::File;
use std::io::{Read, Write};

use clap::Args;

use dcmfx::core::*;
use dcmfx::p10::*;
use dcmfx::pixel_data::*;

pub const ABOUT: &str = "Extracts the pixel data from a DICOM P10 file and \
  writes each frame to a separate image file";

#[derive(Args)]
pub struct ExtractPixelDataArgs {
  #[clap(
    help = "The name of the file to read DICOM P10 content from. Specify '-' \
      to read from stdin."
  )]
  input_filename: String,

  #[arg(
    long,
    short,
    help = "The prefix for output image files. It is suffixed with a 4-digit \
      frame number. By default, the output prefix is the input filename."
  )]
  output_prefix: Option<String>,
}

pub fn run(args: &ExtractPixelDataArgs) -> Result<(), ()> {
  let output_prefix =
    args.output_prefix.as_ref().unwrap_or(&args.input_filename);

  match perform_extract_pixel_data(&args.input_filename, output_prefix) {
    Ok(_) => Ok(()),

    Err(e) => {
      e.print(&format!("reading file \"{}\"", args.input_filename));
      Err(())
    }
  }
}

fn perform_extract_pixel_data(
  input_filename: &str,
  output_prefix: &str,
) -> Result<(), Box<dyn DcmfxError>> {
  // Open input stream
  let mut input_stream: Box<dyn Read> = match input_filename {
    "-" => Box::new(std::io::stdin()),
    _ => match File::open(input_filename) {
      Ok(file) => Box::new(file),
      Err(e) => {
        return Err(Box::new(P10Error::FileError {
          when: "Opening file".to_string(),
          details: e.to_string(),
        }));
      }
    },
  };

  // Create read context
  let mut read_context = P10ReadContext::new();
  read_context.set_config(&P10ReadConfig {
    max_token_size: 1024 * 1024,
    ..P10ReadConfig::default()
  });

  let mut pixel_data_filter = PixelDataFilter::new();

  let mut output_extension: &'static str = "";
  let mut frame_number = 0;

  loop {
    // Read the next tokens from the input stream
    let tokens =
      dcmfx::p10::read_tokens_from_stream(&mut input_stream, &mut read_context)
        .map_err(|e| Box::new(e) as Box<dyn DcmfxError>)?;

    for token in tokens.iter() {
      // Update output extension when the File Meta Information token is
      // received
      if let P10Token::FileMetaInformation { data_set } = token {
        output_extension = file_extension_for_transfer_syntax(
          data_set
            .get_transfer_syntax()
            .unwrap_or(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN),
        );
      }

      // Pass token through the pixel data filter
      let frames = pixel_data_filter
        .add_token(token)
        .map_err(|e| Box::new(e) as Box<dyn DcmfxError>)?;

      // Write frames
      for frame in frames {
        let filename =
          format!("{}.{:04}{}", output_prefix, frame_number, output_extension);

        write_frame(&filename, &frame).map_err(|e| {
          Box::new(P10Error::FileError {
            when: "Writing pixel data frame".to_string(),
            details: e.to_string(),
          }) as Box<dyn DcmfxError>
        })?;

        frame_number += 1;
      }

      if *token == P10Token::End {
        return Ok(());
      }
    }
  }
}

/// Writes the data for a single frame of pixel data to a file.
///
fn write_frame(
  filename: &str,
  frame: &PixelDataFrame,
) -> Result<(), std::io::Error> {
  print!("Writing \"{}\", size: {} bytes … ", filename, frame.len());

  let _ = std::io::stdout().flush();

  let mut stream = File::create(filename)?;
  for fragment in frame.fragments() {
    stream.write_all(fragment)?;
  }

  stream.flush()?;

  println!("done");

  Ok(())
}
