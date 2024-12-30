use std::fs::File;
use std::io::Write;

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
) -> Result<(), P10Error> {
  let data_set = match input_filename {
    "-" => DataSet::read_p10_stream(&mut std::io::stdin()),
    _ => DataSet::read_p10_file(input_filename),
  }?;

  let transfer_syntax = data_set
    .get_transfer_syntax()
    .unwrap_or(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN);

  let (_vr, frames) =
    data_set
      .get_pixel_data()
      .map_err(|e| P10Error::OtherError {
        error_type: "Failed getting pixel data".to_string(),
        details: format!("{:?}", e),
      })?;

  write_frame_data_files(&frames, output_prefix, transfer_syntax).map_err(|e| {
    P10Error::FileError {
      when: "Failed writing pixel data".to_string(),
      details: e.to_string(),
    }
  })
}

fn write_frame_data_files(
  frames: &[Vec<&[u8]>],
  output_prefix: &str,
  transfer_syntax: &TransferSyntax,
) -> Result<(), std::io::Error> {
  for (index, frame) in frames.iter().enumerate() {
    let filename = format!(
      "{}.{:04}{}",
      output_prefix,
      index,
      file_extension_for_transfer_syntax(transfer_syntax)
    );

    print!("Writing file \"{}\" … ", filename);
    let _ = std::io::stdout().flush();

    let mut stream = File::create(filename)?;
    for fragment in frame {
      stream.write_all(fragment)?;
    }
    stream.flush()?;

    println!("done");
  }

  Ok(())
}
