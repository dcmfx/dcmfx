use std::{
  io::{BufRead, BufReader},
  path::PathBuf,
};

use clap::Args;
use dcmfx::{
  core::{TransferSyntax, transfer_syntax},
  p10::{P10Error, P10ReadConfig},
};

#[derive(Args, Debug)]
pub struct BaseInputArgs {
  #[arg(help = "Input filenames. Specify '-' to read from stdin.")]
  pub input_filenames: Vec<PathBuf>,

  #[arg(
    long,
    help_heading = "Input",
    help = "A UTF-8 text file containing a list of input filenames. Each input \
      filename should be on its own line. White space is trimmed and blank \
      lines are ignored."
  )]
  pub file_list: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct P10InputArgs {
  #[command(flatten)]
  pub base: BaseInputArgs,

  #[arg(
    long,
    help_heading = "Input",
    help = "Whether to ignore input files that don't contain DICOM P10 data.",
    default_value_t = false
  )]
  pub ignore_invalid: bool,

  #[arg(
    long,
    help_heading = "Input",
    help = "The transfer syntax to use for DICOM P10 data that doesn't specify \
      '(0002,0010) Transfer Syntax UID' in its File Meta Information, or that \
      doesn't have any File Meta Information.\n\
      \n\
      Defaults to '1.2.840.10008.1.2' (Implicit VR Little Endian)",
    value_parser = default_transfer_syntax_arg_validate,
  )]
  pub default_transfer_syntax: Option<&'static TransferSyntax>,
}

impl P10InputArgs {
  pub fn p10_read_config(&self) -> P10ReadConfig {
    P10ReadConfig::default().default_transfer_syntax(
      self
        .default_transfer_syntax
        .unwrap_or(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN),
    )
  }
}

fn default_transfer_syntax_arg_validate(
  s: &str,
) -> Result<&'static TransferSyntax, String> {
  TransferSyntax::from_uid(s)
    .map_err(|_| "Unrecognized transfer syntax UID".to_string())
}

/// Defines a single input into a CLI command, which can either be the `stdin`
/// stream or a file on the local file system.
///
#[derive(Clone, Debug, PartialEq)]
pub enum InputSource {
  Stdin,
  LocalFile { path: PathBuf },
}

impl core::fmt::Display for InputSource {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      InputSource::Stdin => write!(f, "<stdin>"),
      InputSource::LocalFile { path } => write!(f, "{}", path.display()),
    }
  }
}

impl InputSource {
  /// Returns the input source as a [`PathBuf`].
  ///
  pub fn path(&self) -> PathBuf {
    match self {
      InputSource::Stdin => PathBuf::from("stdin"),
      InputSource::LocalFile { path } => path.clone(),
    }
  }

  /// Returns whether the input source is valid DICOM P10 data.
  ///
  pub fn is_dicom_p10(&self) -> bool {
    match self {
      InputSource::Stdin => true,
      InputSource::LocalFile { path } => dcmfx::p10::is_valid_file(path),
    }
  }

  /// Returns path to the output file for this input source taking into account
  /// the specified output suffix and output directory.
  ///
  pub fn output_path(
    &self,
    output_suffix: &str,
    output_directory: &Option<PathBuf>,
  ) -> PathBuf {
    let mut path = self.path();

    if let Some(output_directory) = output_directory {
      output_directory.join(format!(
        "{}{}",
        path.file_name().unwrap().to_string_lossy(),
        output_suffix
      ))
    } else {
      if let Some(file_name) = path.file_name() {
        let new_file_name =
          format!("{}{output_suffix}", file_name.to_string_lossy());
        path.set_file_name(new_file_name);
      }

      path
    }
  }

  /// Opens the input source as a read stream.
  ///
  pub fn open_read_stream(&self) -> Result<Box<dyn std::io::Read>, P10Error> {
    match self {
      InputSource::Stdin => Ok(Box::new(std::io::stdin())),

      InputSource::LocalFile { path } => match std::fs::File::open(path) {
        Ok(file) => Ok(Box::new(file)),

        Err(e) => Err(P10Error::FileError {
          when: "Opening file".to_string(),
          details: e.to_string(),
        }),
      },
    }
  }
}

impl BaseInputArgs {
  /// Returns an iterator over all input sources requested by the CLI arguments.
  ///
  /// This handles recognizing "-" as meaning stdin, expands input filenames
  /// containing wildcards as glob patterns, and iterates through the file list
  /// if one was specified.
  ///
  pub fn create_iterator(
    &mut self,
  ) -> Box<dyn Iterator<Item = InputSource> + Send> {
    if self.input_filenames.is_empty() && self.file_list.is_none() {
      eprintln!(
        "Error: No inputs specified. Pass --help for usage instructions."
      );
      std::process::exit(1);
    }

    // Create iterator over the input filenames, expanding them if they are glob
    // patterns
    let iter = std::mem::take(&mut self.input_filenames)
      .into_iter()
      .flat_map(|input_filename| {
        let input_filename_str = input_filename.to_string_lossy();

        // Handle stdin
        if input_filename_str == "-" {
          return Box::new(std::iter::once(InputSource::Stdin))
            as Box<dyn Iterator<Item = InputSource> + Send>;
        }

        // Attempt to expand each input filename as a glob pattern
        match glob::glob(&input_filename_str) {
          Ok(paths) => {
            let expanded = paths.filter_map(move |path| match path {
              Ok(path) => {
                if path.is_file() {
                  Some(InputSource::LocalFile { path })
                } else {
                  None
                }
              }
              Err(e) => {
                eprintln!(
                  "Error: Failed globbing '{}', details: {}",
                  input_filename.display(),
                  e
                );
                std::process::exit(1);
              }
            });

            Box::new(expanded)
          }

          Err(e) => {
            eprintln!(
              "Error: Invalid glob pattern '{}', details: {}",
              input_filename.display(),
              e
            );
            std::process::exit(1);
          }
        }
      });

    // Iterate the contents of any file list as well
    Box::new(iter.chain(self.create_file_list_iterator()))
  }

  /// Creates an iterator over the paths in the file list. Each path in the
  /// file list file must be on its own line. White space is trimmed and blank
  /// lines are ignored.
  ///
  fn create_file_list_iterator(
    &self,
  ) -> Box<dyn Iterator<Item = InputSource> + Send> {
    let Some(file_list) = &self.file_list else {
      return Box::new(std::iter::empty());
    };

    let file = match std::fs::File::open(file_list) {
      Ok(file) => file,
      Err(e) => {
        eprintln!(
          "Error: Failed opening file list '{}', details: {}",
          file_list.display(),
          e
        );
        std::process::exit(1);
      }
    };

    let iter = BufReader::new(file).lines().filter_map(|path| match path {
      Ok(path) => {
        let path = path.trim();
        if path.is_empty() {
          None
        } else {
          Some(InputSource::LocalFile { path: path.into() })
        }
      }
      Err(e) => {
        eprintln!("Error: Failed reading file list, details: {e}");
        std::process::exit(1);
      }
    });

    Box::new(iter)
  }
}
