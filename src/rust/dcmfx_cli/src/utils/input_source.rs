use std::{path::PathBuf, sync::Arc};

use object_store::{
  ObjectStore, ObjectStoreExt, path::Path as ObjectStorePath,
};

use dcmfx::p10::P10Error;

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
