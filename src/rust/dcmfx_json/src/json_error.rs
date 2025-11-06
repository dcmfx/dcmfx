#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use dcmfx_core::{DataError, DataSetPath, DcmfxError, dictionary};
use dcmfx_p10::P10Error;

/// Occurs when an error is encountered converting to the DICOM JSON model.
///
#[derive(Debug)]
pub enum JsonSerializeError {
  /// The data to be serialized to the DICOM JSON model is invalid. Details of
  /// the issue are contained in the contained [`DataError`].
  DataError(DataError),

  /// A P10 error that occurred during JSON serialization. The most common error
  /// is [`P10Error::TokenStreamInvalid`], indicating that the stream of tokens
  /// was not well-formed.
  ///
  P10Error(P10Error),

  /// An error occurred when trying to read or write DICOM JSON data on the
  /// provided stream. Details of the issue are contained in the enclosed
  /// [`dcmfx_p10::IoError`].
  ///
  IOError(dcmfx_p10::IoError),
}

/// Occurs when an error is encountered converting from the DICOM JSON model.
///
#[derive(Debug, PartialEq)]
pub enum JsonDeserializeError {
  /// The DICOM JSON data to be deserialized is invalid.
  JsonInvalid { details: String, path: DataSetPath },
}

impl PartialEq for JsonSerializeError {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (JsonSerializeError::DataError(a), JsonSerializeError::DataError(b)) => {
        a == b
      }
      (JsonSerializeError::P10Error(a), JsonSerializeError::P10Error(b)) => {
        a == b
      }
      (JsonSerializeError::IOError(a), JsonSerializeError::IOError(b)) => {
        a.to_string() == b.to_string()
      }
      _ => false,
    }
  }
}

impl core::fmt::Display for JsonSerializeError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      JsonSerializeError::DataError(e) => e.fmt(f),
      JsonSerializeError::P10Error(e) => e.fmt(f),
      JsonSerializeError::IOError(e) => e.fmt(f),
    }
  }
}

impl core::fmt::Display for JsonDeserializeError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      JsonDeserializeError::JsonInvalid { details, path } => {
        write!(
          f,
          "DICOM JSON deserialize error, details: {}, path: {}",
          details,
          path.to_detailed_string(),
        )
      }
    }
  }
}

impl DcmfxError for JsonSerializeError {
  /// Returns lines of text that describe a DICOM JSON serialize error in a
  /// human-readable format.
  ///
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    match self {
      JsonSerializeError::DataError(e) => e.to_lines(task_description),
      JsonSerializeError::P10Error(e) => e.to_lines(task_description),
      JsonSerializeError::IOError(e) => vec![
        format!("DICOM JSON IO error {}", task_description),
        "".to_string(),
        format!("  Error: {}", e),
      ],
    }
  }
}

impl DcmfxError for JsonDeserializeError {
  /// Returns lines of text that describe a DICOM JSON deserialize error in a
  /// human-readable format.
  ///
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    match self {
      JsonDeserializeError::JsonInvalid { details, path } => {
        let mut lines = vec![];

        lines.push(format!("DICOM JSON deserialize error {task_description}"));
        lines.push("".to_string());
        lines.push(format!("  Details: {details}"));

        if let Ok(tag) = path.final_data_element() {
          lines.push(format!("  Tag: {tag}"));
          lines.push(format!("  Name: {}", dictionary::tag_name(tag, None)));
        }

        if !path.is_root() {
          lines.push(format!("  Path: {path}"));
        }

        lines
      }
    }
  }
}
