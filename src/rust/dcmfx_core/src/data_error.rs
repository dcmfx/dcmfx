//! Provides the [`DataError`] type that describes the errors that can occur
//! when working with data sets and elements.

use crate::{DataSetPath, DcmfxError, ValueRepresentation, dictionary};

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

/// An error that occurred when retrieving or creating data elements in data
/// sets. An error can be one of the following types:
///
/// 1. **Tag not present**.
///
///    When retrieving a value, the requested tag was not present in the data
///    set.
///
/// 2. **Value not present**.
///
///    When retrieving a value, the requested type is not present. E.g. tried to
///    retrieve an integer value when the data element value contains a string.
///
/// 3. **Multiplicity mismatch**.
///
///    When retrieving a value, it did not have the required multiplicity. E.g.
///    tried to retrieve a single string value when the data element contained
///    multiple string values.
///
/// 4. **Value invalid**.
///
///    When retrieving a value, there was an error decoding its bytes. E.g. a
///    string value that had bytes that are not valid UTF-8, or a `PersonName`
///    value that had an invalid structure.
///
///    When creating a value, the supplied input was not valid for the type of
///    data element being created.
///
/// 5. **Value length invalid**.
///
///    When creating a value, the supplied data did not meet a required length
///    constraint, e.g. the minimum or maximum length for the value
///    representation wasn't respected.
///
/// 6. **Value unsupported**.
///
///    When creating, reading, or parsing a value, the value itself is valid but
///    is not supported by this library.
///
#[derive(Clone, Debug, PartialEq)]
pub enum DataError {
  TagNotPresent {
    path: DataSetPath,
  },
  ValueNotPresent {
    path: Option<DataSetPath>,
  },
  MultiplicityMismatch {
    path: Option<DataSetPath>,
  },
  ValueInvalid {
    details: String,
    path: Option<DataSetPath>,
  },
  ValueLengthInvalid {
    vr: ValueRepresentation,
    length: u64,
    details: String,
    path: Option<DataSetPath>,
  },
}

impl core::fmt::Display for DataError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    fn optional_path_to_string(path: &Option<DataSetPath>) -> String {
      path
        .as_ref()
        .map(|path| path.to_detailed_string())
        .unwrap_or("<unknown>".to_string())
    }

    let error = match &self {
      Self::TagNotPresent { path } => {
        format!("Tag not present at {}", path.to_detailed_string())
      }
      Self::ValueNotPresent { path } => {
        format!("Value not present at {}", optional_path_to_string(path))
      }
      Self::MultiplicityMismatch { path } => {
        format!("Multiplicity mismatch at {}", optional_path_to_string(path))
      }
      Self::ValueInvalid { details, path } => {
        format!(
          "Invalid value at {}, details: {}",
          optional_path_to_string(path),
          details
        )
      }
      Self::ValueLengthInvalid { details, path, .. } => {
        format!(
          "Invalid value length at {}, details: {}",
          optional_path_to_string(path),
          details
        )
      }
    };

    write!(f, "DICOM Data Error: {error}")
  }
}

impl DataError {
  /// Constructs a new 'Tag not present' data error.
  ///
  pub fn new_tag_not_present() -> Self {
    Self::TagNotPresent {
      path: DataSetPath::new(),
    }
  }

  /// Constructs a new 'Value not present' data error.
  ///
  pub fn new_value_not_present() -> Self {
    Self::ValueNotPresent { path: None }
  }

  /// Constructs a new 'Multiplicity mismatch' data error.
  ///
  pub fn new_multiplicity_mismatch() -> Self {
    Self::MultiplicityMismatch { path: None }
  }

  /// Constructs a new 'Value invalid' data error.
  ///
  pub fn new_value_invalid(details: String) -> Self {
    Self::ValueInvalid {
      details,
      path: None,
    }
  }

  /// Constructs a new 'Value length invalid' data error.
  ///
  pub fn new_value_length_invalid(
    vr: ValueRepresentation,
    length: u64,
    details: String,
  ) -> Self {
    Self::ValueLengthInvalid {
      vr,
      length,
      details,
      path: None,
    }
  }

  /// Returns the data set path for a data error.
  ///
  pub fn path(&self) -> Option<&DataSetPath> {
    match &self {
      Self::TagNotPresent { path } => Some(path),
      Self::ValueNotPresent { path }
      | Self::MultiplicityMismatch { path }
      | Self::ValueInvalid { path, .. }
      | Self::ValueLengthInvalid { path, .. } => path.as_ref(),
    }
  }

  /// Adds a data set path to a data error. This indicates the exact location
  /// that a data error occurred in a data set, and should be included wherever
  /// possible to make troubleshooting easier.
  ///
  pub fn with_path(self, path: &DataSetPath) -> Self {
    match self {
      Self::TagNotPresent { .. } => Self::TagNotPresent { path: path.clone() },
      Self::ValueNotPresent { .. } => Self::ValueNotPresent {
        path: Some(path.clone()),
      },
      Self::MultiplicityMismatch { .. } => Self::MultiplicityMismatch {
        path: Some(path.clone()),
      },
      Self::ValueInvalid { details, .. } => Self::ValueInvalid {
        details,
        path: Some(path.clone()),
      },
      Self::ValueLengthInvalid {
        vr,
        length,
        details,
        ..
      } => Self::ValueLengthInvalid {
        vr,
        length,
        details,
        path: Some(path.clone()),
      },
    }
  }

  /// Returns the name of a data error as a human-readable string.
  ///
  pub fn name(&self) -> &'static str {
    match &self {
      Self::TagNotPresent { .. } => "Tag not present",
      Self::ValueNotPresent { .. } => "Value not present",
      Self::MultiplicityMismatch { .. } => "Multiplicity mismatch",
      Self::ValueInvalid { .. } => "Invalid value",
      Self::ValueLengthInvalid { .. } => "Invalid value length",
    }
  }

  /// Returns the `details` field of the error, if one exists.
  ///
  pub fn details(&self) -> &str {
    match self {
      Self::TagNotPresent { .. } => "",
      Self::ValueNotPresent { .. } => "",
      Self::MultiplicityMismatch { .. } => "",
      Self::ValueInvalid { details, .. } => details,
      Self::ValueLengthInvalid { details, .. } => details,
    }
  }
}

impl DcmfxError for DataError {
  /// Returns lines of text that describe a DICOM data error in a human-readable
  /// format.
  ///
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    let mut lines = vec![
      format!("DICOM data error {}", task_description),
      "".to_string(),
      format!("  Error: {}", self.name()),
    ];

    match &self {
      Self::TagNotPresent { path, .. }
      | Self::ValueNotPresent {
        path: Some(path), ..
      }
      | Self::MultiplicityMismatch {
        path: Some(path), ..
      }
      | Self::ValueInvalid {
        path: Some(path), ..
      }
      | Self::ValueLengthInvalid {
        path: Some(path), ..
      } => {
        if let Ok(tag) = path.final_data_element() {
          lines.push(format!("  Tag: {tag}"));
          lines.push(format!("  Name: {}", dictionary::tag_name(tag, None)));
        }

        lines.push(format!("  Path: {}", path.to_detailed_string()));
      }
      _ => (),
    };

    match &self {
      Self::ValueInvalid { details, .. } => {
        lines.push(format!("  Details: {details}"))
      }
      Self::ValueLengthInvalid {
        vr,
        length,
        details,
        ..
      } => {
        lines.push(format!("  VR: {vr}"));
        lines.push(format!("  Length: {length} bytes"));
        lines.push(format!("  Details: {details}"));
      }
      _ => (),
    };

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::DcmfxError;

  #[test]
  fn to_lines_test() {
    assert_eq!(
      DataError::new_tag_not_present()
        .with_path(&DataSetPath::from_string("12345678/[1]/11223344").unwrap())
        .to_lines("testing")
        .join("\n"),
      r#"DICOM data error testing

  Error: Tag not present
  Tag: (1122,3344)
  Name: unknown_tag
  Path: (1234,5678) unknown_tag / Item 1 / (1122,3344) unknown_tag"#
    );

    assert_eq!(
      DataError::new_value_not_present()
        .to_lines("testing")
        .join("\n"),
      r#"DICOM data error testing

  Error: Value not present"#
    );

    assert_eq!(
      DataError::new_multiplicity_mismatch()
        .to_lines("testing")
        .join("\n"),
      r#"DICOM data error testing

  Error: Multiplicity mismatch"#
    );

    assert_eq!(
      DataError::new_value_invalid("123".to_string())
        .to_lines("testing")
        .join("\n"),
      r#"DICOM data error testing

  Error: Invalid value
  Details: 123"#
    );

    assert_eq!(
      DataError::new_value_length_invalid(
        ValueRepresentation::AgeString,
        5,
        "Test 123".to_string(),
      )
      .to_lines("testing")
      .join("\n"),
      r#"DICOM data error testing

  Error: Invalid value length
  VR: AS
  Length: 5 bytes
  Details: Test 123"#
    );
  }
}
