use std::{path::PathBuf, pin::Pin, sync::Arc};

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
      crate::utils::exit_with_error(
        "No inputs specified. Pass --help for usage instructions.",
        "",
      );
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
    }
    // Handle object URLs
    else if let Ok((object_store, object_path)) =
      object_url_to_store_and_path(&input_filename_str).await
    {
      if is_glob(object_path.as_ref()) {
        let mut stream = input_sources_for_object_url_glob(
          object_store,
          &object_path,
          input_filename_str.to_string(),
        );

        while let Some(input_source) = stream.next().await {
          yield input_source;
        }
      } else {
        yield InputSource::Object {
          object_store,
          object_path: object_path.into(),
          specified_path: input_filename.clone(),
        };
      }
    }
    // Local file system path
    else {
      let (object_store, object_path) =
        local_path_to_store_and_path(input_filename_str.to_string()).await;

      if is_glob(&object_path) {
        let mut stream = input_sources_for_object_url_glob(
          object_store,
          &object_path,
          input_filename_str.to_string(),
        );

        while let Some(input_source) = stream.next().await {
          yield input_source;
        }
      } else {
        if input_filename.is_dir() {
          return;
        }

        if !input_filename.is_file() {
          crate::utils::exit_with_error(
            &format!(
              "Input file '{}' does not exist",
              input_filename.display()
            ),
            "",
          );
        }

        yield InputSource::Object {
          object_store,
          object_path: object_path.into(),
          specified_path: input_filename.clone(),
        }
      }
    }
  })
}

/// Creates a stream for the input sources referenced by an object URL that
/// contains a glob pattern.
///
/// The listing of objects is scoped to a prefix to the extent that's possible,
/// with further filtering done by matching against the glob pattern.
///
fn input_sources_for_object_url_glob(
  object_store: Arc<dyn object_store::ObjectStore>,
  object_path: &str,
  input_filename_str: String,
) -> Pin<Box<dyn Stream<Item = InputSource> + Send>> {
  let Ok(pattern) = glob::Pattern::new(object_path) else {
    crate::utils::exit_with_error(
      &format!("Invalid glob pattern '{}'", object_path),
      "",
    );
  };

  let prefix = object_url_list_prefix(object_path);
  let mut list_stream = object_store.list(prefix.as_ref());

  Box::pin(async_stream::stream! {
    loop {
      match list_stream.next().await {
        Some(Ok(meta)) => {
          if pattern.matches(meta.location.as_ref()) {
            yield InputSource::Object {
              object_store: object_store.clone(),
              object_path: meta.location.clone(),
              specified_path: PathBuf::from(meta.location.to_string()),
            };
          }
        }

        Some(Err(e)) => crate::utils::exit_with_error(
          &format!("Failed listing '{}'", input_filename_str),
          e,
        ),

        None => break,
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
      crate::utils::exit_with_error(
        &format!("Failed opening file list '{}'", file_list.display()),
        e,
      );
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

          let (object_store, object_path) =
            if let Ok((object_store, object_path)) =
              object_url_to_store_and_path(path).await
            {
              (object_store, object_path)
            } else {
              local_path_to_store_and_path(path).await
            };

          yield InputSource::Object {
            object_store,
            object_path: object_path.into(),
            specified_path: PathBuf::from(path),
          };
        }

        Err(e) => crate::utils::exit_with_error("Failed reading file list", e),
      }
    }
  })
}

/// Checks if the given string is a potential glob pattern.
///
fn is_glob(s: &str) -> bool {
  s.contains('*') || s.contains('?') || s.contains('[') || s.contains('{')
}

/// Returns the prefix to use when listing objects in an object store that are
/// going to be filtered by the given glob pattern.
///
fn object_url_list_prefix(
  glob_pattern: &str,
) -> Option<object_store::path::Path> {
  let star_idx = glob_pattern.find('*')?;
  let separator_idx = &glob_pattern[..star_idx].rfind('/')?;

  match separator_idx {
    0 => None,
    i => Some(object_store::path::Path::from(&glob_pattern[..*i])),
  }
}
