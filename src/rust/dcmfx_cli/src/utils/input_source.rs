use std::{
  path::{Path, PathBuf},
  sync::Arc,
};

use object_store::{
  ObjectStore, ObjectStoreExt, path::Path as ObjectStorePath,
};

use dcmfx::p10::P10Error;

use super::object_store::{
  local_path_to_store_and_path, object_url_to_store_and_path,
};

/// Defines an input source for a CLI command that abstracts over the different
/// locations input can come from.
///
#[derive(Clone, Debug)]
pub enum InputSource {
  /// An input source that reads from stdin.
  Stdin,

  /// An input source that reads an object from an object store.
  Object {
    object_store: Arc<dyn ObjectStore>,
    object_path: ObjectStorePath,

    /// The path to display for this input source in output and error messages.
    /// This is the path as specified on the CLI where possible, and is only
    /// for sisplay purposes.
    display_path: PathBuf,
  },
}

impl core::fmt::Display for InputSource {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      InputSource::Stdin => write!(f, "-"),
      InputSource::Object { display_path, .. } => {
        write!(f, "{}", display_path.display())
      }
    }
  }
}

impl InputSource {
  /// Resolves a single, non-glob input path to an input source. Object store
  /// URLs are tried first, with the local file system used as the fallback.
  ///
  /// Exits the process with an error if the path isn't a recognized object
  /// store URL and doesn't reference an existing local file.
  ///
  pub async fn resolve(path: &Path) -> Self {
    let path_str = path.to_string_lossy();

    let (object_store, object_path) =
      match object_url_to_store_and_path(&path_str).await {
        Ok(result) => result,

        Err(_) => {
          if !path.is_file() {
            crate::utils::exit_with_error(
              &format!("Input file '{}' does not exist", path.display()),
              "",
            );
          }

          local_path_to_store_and_path(path_str.to_string()).await
        }
      };

    InputSource::Object {
      object_store,
      object_path,
      display_path: path.to_path_buf(),
    }
  }

  /// Returns the file name of this input source, i.e. the final part of the
  /// path to its object. This is used when deriving output filenames.
  ///
  pub fn file_name(&self) -> &str {
    match self {
      InputSource::Stdin => "-",
      InputSource::Object { object_path, .. } => {
        object_path.filename().unwrap_or_default()
      }
    }
  }

  /// Opens the input source as a read stream.
  ///
  pub async fn open_read_stream(
    &self,
  ) -> Result<Box<dyn dcmfx::p10::IoAsyncRead>, P10Error> {
    match self {
      InputSource::Stdin => Ok(Box::new(tokio::io::stdin())),

      InputSource::Object {
        object_store,
        object_path,
        ..
      } => {
        let get_result =
          object_store.get(&object_path.clone()).await.map_err(|e| {
            P10Error::FileError {
              when: "Opening read stream".to_string(),
              details: e.to_string(),
            }
          })?;

        // Convert to a Tokio async read stream
        let stream =
          tokio_util::io::StreamReader::new(get_result.into_stream());

        Ok(Box::new(stream))
      }
    }
  }
}
