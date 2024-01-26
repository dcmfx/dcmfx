use std::fs::File;
use std::io::{Read, Write};

use clap::Args;

use dcmfx::core::*;
use dcmfx::json::*;
use dcmfx::p10::*;

pub const ABOUT: &str = "Converts a DICOM JSON file to a DICOM P10 file";

#[derive(Args)]
pub struct ToDcmArgs {
  #[clap(
    help = "The name of the file to read DICOM JSON content from. Specify '-' \
      to read from stdin."
  )]
  input_filename: String,

  #[clap(
    help = "The name of the file to write DICOM P10 content to. Specify '-' \
      to write to stdout."
  )]
  output_filename: String,
}

pub fn run(args: &ToDcmArgs) -> Result<(), ()> {
  let json = match args.input_filename.as_str() {
    "-" => {
      let mut input = String::new();
      std::io::stdin().read_to_string(&mut input).map(|_| input)
    }
    _ => std::fs::read_to_string(&args.input_filename),
  };

  let json = match json {
    Ok(json) => json,
    Err(e) => {
      P10Error::FileError {
        when: format!("reading file \"{}\"", args.input_filename),
        details: e.to_string(),
      }
      .print(&format!("reading file \"{}\"", args.input_filename));

      return Err(());
    }
  };

  let data_set = match DataSet::from_json(&json) {
    Ok(data_set) => data_set,
    Err(e) => {
      e.print(&format!("parsing file \"{}\"", args.input_filename));
      return Err(());
    }
  };

  // Open output stream
  let mut output_stream: Box<dyn Write> = match args.output_filename.as_str() {
    "-" => Box::new(std::io::stdout()),
    _ => match File::create(&args.output_filename) {
      Ok(file) => Box::new(file),
      Err(e) => {
        P10Error::FileError {
          when: "Opening file".to_string(),
          details: e.to_string(),
        }
        .print(&format!("opening file \"{}\"", args.output_filename));

        return Err(());
      }
    },
  };

  match data_set.write_p10_stream(&mut output_stream, None) {
    Ok(_) => Ok(()),
    Err(e) => {
      e.print(&format!("writing file \"{}\"", args.output_filename));
      Err(())
    }
  }
}
