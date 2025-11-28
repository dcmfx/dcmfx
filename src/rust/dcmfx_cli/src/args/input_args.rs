use std::{path::PathBuf, pin::Pin};

use clap::Args;
use futures::{Stream, StreamExt};

use dcmfx::{
  core::{TransferSyntax, transfer_syntax},
  p10::P10ReadConfig,
};
use tokio::io::AsyncBufReadExt;

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
  /// Returns an async stream over all input sources described by the CLI
  /// arguments.
  ///
  /// This handles recognizing "-" as meaning stdin, expands input filenames
  /// containing wildcards as glob patterns, and includes the contents of the
  /// file list if one was specified.
  ///
  pub async fn input_sources(
    &self,
  ) -> Pin<Box<dyn Stream<Item = InputSource> + Send>> {
    if self.input_filenames.is_empty() && self.file_list.is_none() {
      eprintln!(
        "Error: No inputs specified. Pass --help for usage instructions."
      );
      std::process::exit(1);
    }

    futures::stream::iter(self.input_filenames.clone())
      .map(input_sources_for_input_filename)
      .flatten()
      .chain(file_list_input_sources(&self.file_list).await)
      .boxed()
  }
}

/// Creates a stream of input sources for the given input filename.
///
fn input_sources_for_input_filename(
  input_filename: PathBuf,
) -> Pin<Box<dyn Stream<Item = InputSource> + Send>> {
  Box::pin(async_stream::stream! {
    let input_filename_str = input_filename.to_string_lossy();

    // Handle stdin
    if input_filename_str == "-" {
      yield InputSource::Stdin;
      return;
    }

    // Handle object URLs
    if let Ok((object_store, object_path)) =
      object_url_to_store_and_path(&input_filename_str).await
    {
      yield InputSource::Object {
        object_store,
        object_path,
        specified_path: input_filename.clone(),
      };
      return;
    }

    // If it's not a glob then error if it doesn't point to a valid file
    if !is_glob(&input_filename_str) && !input_filename.is_file() {
      if input_filename.is_dir() {
        return;
      }

      eprintln!(
        "Error: Input file '{}' does not exist",
        input_filename.display()
      );
      std::process::exit(1);
    }

    // Expand glob patterns
    let paths = match glob::glob(&input_filename_str) {
      Ok(paths) => paths,
      Err(e) => {
        eprintln!(
          "Error: Invalid glob pattern '{}', details: {}",
          input_filename.display(),
          e
        );
        std::process::exit(1);
      }
    };

    for entry in paths {
      match entry {
        Ok(path) => {
          if !path.is_file() {
            continue;
          }

          let (object_store, object_path) =
            local_path_to_store_and_path(&path).await;

          yield InputSource::Object {
            object_store,
            object_path,
            specified_path: path,
          };
        }

        Err(e) => {
          eprintln!(
            "Error: Failed globbing '{}', details: {}",
            input_filename.display(),
            e
          );
          std::process::exit(1);
        }
      }
    }
  })
}

/// Creates a stream for the paths in the file list. Each path in the file list
/// file must be on its own line. White space is trimmed and blank lines are
/// ignored.
///
async fn file_list_input_sources(
  file_list: &Option<PathBuf>,
) -> Pin<Box<dyn Stream<Item = InputSource> + Send>> {
  let Some(file_list) = file_list else {
    return Box::pin(futures::stream::empty());
  };

  let file = match tokio::fs::File::open(file_list).await {
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

  let mut lines = tokio::io::BufReader::new(file).lines();

  Box::pin(async_stream::stream! {
    while let Some(path) = lines.next_line().await.transpose() {
      match path {
        Ok(path) => {
          let path = path.trim();
          if path.is_empty() {
            continue;
          }

          if let Ok((object_store, object_path)) =
            object_url_to_store_and_path(path).await
          {
            yield InputSource::Object {
              object_store,
              object_path,
              specified_path: PathBuf::from(path),
            }
          } else {
            let (object_store, object_path) =
              local_path_to_store_and_path(path).await;

            yield InputSource::Object {
              object_store,
              object_path,
              specified_path: PathBuf::from(path),
            }
          }
        }

        Err(e) => {
          eprintln!("Error: Failed reading file list, details: {e}");
          std::process::exit(1);
        }
      }
    }
  })
}

/// Checks if the given string is a potential glob pattern.
///
fn is_glob(s: &str) -> bool {
  s.contains('*') || s.contains('?') || s.contains('[') || s.contains('{')
}
