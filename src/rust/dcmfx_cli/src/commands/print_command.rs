use std::fs::File;
use std::io::Write;

use clap::Args;

use dcmfx::core::*;
use dcmfx::p10::*;

pub const ABOUT: &str = "Prints the content of a DICOM P10 file";

#[derive(Args)]
pub struct PrintArgs {
  input_filename: String,

  #[arg(
    long,
    short,
    help = "\
      The maximum width in characters of the printed output. By default this \
      is set to the width of the active terminal, or 80 characters if the \
      terminal width can't be detected.",
    value_parser = clap::value_parser!(u32).range(0..10000),
  )]
  max_width: Option<u32>,

  #[arg(
    long,
    short,
    help = "\
      Whether to print output using color and bold text. By default this is \
      set based on whether there is an active output terminal that supports \
      colored output."
  )]
  styled: Option<bool>,
}

pub fn run(args: &PrintArgs) -> Result<(), ()> {
  let mut context = P10ReadContext::new();

  // Set a small max part size to keep memory usage low. 256 KiB is also plenty
  // of data to preview the content of data element values, even if the max
  // output width is very large.
  context.set_config(&P10ReadConfig {
    max_part_size: 256 * 1024,
    ..P10ReadConfig::default()
  });

  // Apply any print option arguments
  let mut print_options = DataSetPrintOptions::default();
  if let Some(max_width) = args.max_width {
    print_options = print_options.max_width(max_width as usize);
  }
  if let Some(styled) = args.styled {
    print_options = print_options.styled(styled);
  }

  match perform_print(&args.input_filename, context, &print_options) {
    Ok(()) => Ok(()),
    Err(e) => {
      e.print(&format!("printing file \"{}\"", args.input_filename));
      Err(())
    }
  }
}

fn perform_print(
  input_filename: &str,
  mut context: P10ReadContext,
  print_options: &DataSetPrintOptions,
) -> Result<(), P10Error> {
  let mut file = match File::open(input_filename) {
    Ok(file) => file,
    Err(e) => {
      return Err(P10Error::FileError {
        when: "Opening file".to_string(),
        details: e.to_string(),
      })
    }
  };

  let mut p10_print_transform = P10PrintTransform::new(print_options);

  loop {
    let parts = dcmfx::p10::read_parts_from_stream(&mut file, &mut context)?;

    for part in parts.iter() {
      match part {
        P10Part::FilePreambleAndDICMPrefix { .. } => (),

        P10Part::End => return Ok(()),

        _ => {
          let s = p10_print_transform.add_part(part);

          std::io::stdout().write(s.as_bytes()).map_err(|e| {
            P10Error::FileError {
              when: "Writing to stdout".to_string(),
              details: e.to_string(),
            }
          })?;
        }
      };
    }
  }
}
