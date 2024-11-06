use dcmfx_core::{registry, DataError, DataSetPath};

/// Occurs when an error is encountered converting to the DICOM JSON model.
///
#[derive(Debug)]
pub enum JsonSerializeError {
  /// The data to be serialized to the DICOM JSON model is invalid. Details of
  /// the issue are contained in the enclosed [`DataError`].
  DataError(DataError),

  /// An error occurred when trying to read or write DICOM JSON data on the
  /// provided stream. Details of the issue are contained in the enclosed
  /// [`std::io::Error`].
  ///
  IOError(std::io::Error),
}

/// Occurs when an error is encountered converting from the DICOM JSON model.
///
#[derive(Debug)]
pub enum JsonDeserializeError {
  /// The DICOM JSON data to be deserialized is invalid.
  JsonInvalid { details: String, path: DataSetPath },
}

impl std::fmt::Display for JsonSerializeError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      JsonSerializeError::DataError(e) => e.fmt(f),
      JsonSerializeError::IOError(e) => e.fmt(f),
    }
  }
}

impl std::fmt::Display for JsonDeserializeError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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

impl dcmfx_core::DcmfxError for JsonSerializeError {
  /// Returns lines of text that describe a DICOM JSON serialize error in a
  /// human-readable format.
  ///
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    match self {
      JsonSerializeError::DataError(e) => e.to_lines(task_description),
      JsonSerializeError::IOError(e) => vec![
        format!("DICOM JSON I/O error {}", task_description),
        "".to_string(),
        format!("  Error: {}", e),
      ],
    }
  }
}

impl dcmfx_core::DcmfxError for JsonDeserializeError {
  /// Returns lines of text that describe a DICOM JSON deserialize error in a
  /// human-readable format.
  ///
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    match self {
      JsonDeserializeError::JsonInvalid { details, path } => {
        let mut lines = vec![];

        lines
          .push(format!("DICOM JSON deserialize error {}", task_description));
        lines.push("".to_string());
        lines.push(format!("  Details: {}", details));

        if let Ok(tag) = path.final_data_element() {
          lines.push(format!("  Tag: {}", tag));
          lines.push(format!("  Name: {}", registry::tag_name(tag, None)));
        }

        if !path.is_empty() {
          lines.push(format!("  Path: {}", path));
        }

        lines
      }
    }
  }
}
