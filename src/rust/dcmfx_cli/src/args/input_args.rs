use std::{
  io::{BufRead, BufReader},
  path::PathBuf,
};

use clap::Args;

use dcmfx::{
  core::{TransferSyntax, transfer_syntax},
  p10::P10ReadConfig,
};

use crate::utils::{
  input_source::InputSource,
  object_store::{local_path_to_store_and_path, object_url_to_store_and_path},
};

#[derive(Args, Debug)]
pub struct BaseInputArgs {
  #[arg(help = "Input filenames. Specify '-' to read from stdin.")]
  pub input_filenames: Vec<PathBuf>,

  #[arg(
    long,
    help_heading = "Input",
    help = "A UTF-8 text file containing a list of input filenames. This is \
      useful when the number of input filenames is very large. Each input \
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
    help = "Whether to ignore input files that don't contain DICOM P10 data, \
      defined as not having the 'DICM' prefix at byte offset 128.",
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

impl BaseInputArgs {
  /// Returns an iterator over all input sources described by the CLI arguments.
  ///
  /// This handles recognizing "-" as meaning stdin, expands input filenames
  /// containing wildcards as glob patterns, and iterates through the file list
  /// if one was specified.
  ///
  pub fn create_iterator(
    &self,
  ) -> Box<dyn Iterator<Item = InputSource> + Send> {
    if self.input_filenames.is_empty() && self.file_list.is_none() {
      eprintln!(
        "Error: No inputs specified. Pass --help for usage instructions."
      );
      std::process::exit(1);
    }

    // Create iterator over the input filenames, expanding them if they are glob
    // patterns
    let iter =
      self
        .input_filenames
        .clone()
        .into_iter()
        .flat_map(|input_filename| {
          let input_filename_str = input_filename.to_string_lossy();

          // Handle stdin
          if input_filename_str == "-" {
            return Box::new(std::iter::once(InputSource::Stdin))
              as Box<dyn Iterator<Item = InputSource> + Send>;
          }

          // Handle an object URL
          if let Ok((object_store, object_path)) =
            object_url_to_store_and_path(&input_filename_str)
          {
            return Box::new(std::iter::once(InputSource::Object {
              object_store,
              object_path,
              specified_path: input_filename.clone(),
            }));
          }

          // If it's not a glob then error if it doesn't point to a valid file
          if !is_glob(&input_filename_str) && !input_filename.is_file() {
            // Ignore directories
            if input_filename.is_dir() {
              return Box::new(std::iter::empty());
            }

            eprintln!(
              "Error: Input file '{}' does not exist",
              input_filename.display()
            );
            std::process::exit(1);
          }

          // Attempt to expand as a glob pattern
          match glob::glob(&input_filename_str) {
            Ok(paths) => {
              let expanded = paths.filter_map(move |path| match path {
                Ok(path) => {
                  if path.is_file() {
                    let (object_store, object_path) =
                      local_path_to_store_and_path(&path);

                    Some(InputSource::Object {
                      object_store,
                      object_path,
                      specified_path: path,
                    })
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
    let iter = iter.chain(self.create_file_list_iterator());

    Box::new(iter)
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
        } else if let Ok((object_store, object_path)) =
          object_url_to_store_and_path(path)
        {
          Some(InputSource::Object {
            object_store,
            object_path,
            specified_path: PathBuf::from(path),
          })
        } else {
          let (object_store, object_path) = local_path_to_store_and_path(path);

          Some(InputSource::Object {
            object_store,
            object_path,
            specified_path: PathBuf::from(path),
          })
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

/// Checks if the given string is a potential glob pattern.
///
fn is_glob(s: &str) -> bool {
  s.contains('*') || s.contains('?') || s.contains('[') || s.contains('{')
}
