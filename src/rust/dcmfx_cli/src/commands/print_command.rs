use std::{io::Write, path::PathBuf};

use clap::Args;

use dcmfx::core::*;
use dcmfx::p10::*;

use crate::InputSource;

pub const ABOUT: &str = "Prints the content of DICOM P10 files";

#[derive(Args)]
pub struct PrintArgs {
  #[clap(
    required = true,
    help = "The names of the DICOM P10 files to print the content of. Specify \
      '-' to read from stdin."
  )]
  input_filenames: Vec<PathBuf>,

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
  let input_sources = crate::get_input_sources(&args.input_filenames);

  let mut print_options = DataSetPrintOptions::default();
  if let Some(max_width) = args.max_width {
    print_options = print_options.max_width(max_width as usize);
  }
  if let Some(styled) = args.styled {
    print_options = print_options.styled(styled);
  }

  for input_source in input_sources {
    match print_input_source(&input_source, &print_options) {
      Ok(()) => (),

      Err(e) => {
        e.print(&format!("printing \"{}\"", input_source));

        return Err(());
      }
    }
  }

  Ok(())
}

fn print_input_source(
  input_source: &InputSource,
  print_options: &DataSetPrintOptions,
) -> Result<(), P10Error> {
  let mut stream = input_source.open_read_stream()?;

  // Create read context
  let mut context = P10ReadContext::new();

  // Set a small max token size to keep memory usage low. 256 KiB is also plenty
  // of data to preview the content of data element values, even if the max
  // output width is very large.
  context.set_config(&P10ReadConfig {
    max_token_size: 256 * 1024,
    ..P10ReadConfig::default()
  });

  let mut p10_print_transform = P10PrintTransform::new(print_options);

  loop {
    let tokens =
      dcmfx::p10::read_tokens_from_stream(&mut stream, &mut context)?;

    for token in tokens.iter() {
      match token {
        P10Token::FilePreambleAndDICMPrefix { .. } => (),

        P10Token::End => return Ok(()),

        _ => {
          let s = p10_print_transform.add_token(token);

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
