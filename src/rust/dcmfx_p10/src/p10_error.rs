//! Defines the type used to describe errors that can occur when reading and
//! writing DICOM P10 data.

use dcmfx_core::{DataSetPath, DcmfxError};

use crate::P10Part;

/// An error that occurred when reading or writing DICOM P10 data.
///
#[derive(Clone, Debug, PartialEq)]
pub enum P10Error {
  /// This error occurs when a DICOM P10 read or write context is supplied data
  /// that specifies a DICOM transfer syntax that isn't supported.
  TransferSyntaxNotSupported { transfer_syntax_uid: String },

  /// This error occurs when a DICOM P10 read context is supplied data that
  /// contains a *'(0008,0005) SpecificCharacterSet'* data element that is invalid
  /// and unable to be decoded.
  ///
  /// This error will never occur on valid DICOM P10 data because all character
  /// sets defined by the DICOM standard are supported.
  SpecificCharacterSetInvalid {
    specific_character_set: String,
    details: String,
  },

  /// This error occurs when a DICOM P10 read context requires more data to be
  /// added to it before the next part can be read.
  DataRequired { when: String },

  /// This error occurs when a DICOM P10 read context reaches the end of its
  /// data while reading the next part, and no more data is able to be added.
  /// This means the provided data is malformed or truncated.
  DataEndedUnexpectedly {
    when: String,
    path: DataSetPath,
    offset: u64,
  },

  /// This error occurs when a DICOM P10 read context is unable to read the next
  /// DICOM P10 part because the supplied data is invalid, and also when a DICOM
  /// P10 write context is unable to serialize a part written to it.
  DataInvalid {
    when: String,
    details: String,
    path: DataSetPath,
    offset: u64,
  },

  /// This error occurs when one of the configured maximums for a DICOM P10 read
  /// context is exceeded during reading of the supplied data. These maximums
  /// are used to control memory usage when reading.
  MaximumExceeded {
    details: String,
    path: DataSetPath,
    offset: u64,
  },

  /// This error occurs when a stream of [`P10Part`]s is being ingested and a
  /// part is received that is invalid at the current location in the part
  /// stream. E.g. a [`P10Part::DataElementValueBytes`] part that does not
  /// follow a [`P10Part::DataElementHeader`].
  PartStreamInvalid {
    when: String,
    details: String,
    part: P10Part,
  },

  /// This error occurs when bytes are written to a DICOM P10 read context after
  /// its final bytes have already been written.
  WriteAfterCompletion,

  /// This error occurs when there is an error with an underlying file or file
  /// stream.
  FileError { when: String, details: String },

  /// A fallback/general-purpose error for cases not covered by the other error
  /// variants.
  OtherError { error_type: String, details: String },
}

impl std::fmt::Display for P10Error {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "DICOM P10 error: {}", self.name())
  }
}

impl P10Error {
  /// Returns the name of the error as a human-readable string.
  ///
  pub fn name(&self) -> String {
    match self {
      P10Error::TransferSyntaxNotSupported { .. } => {
        "Transfer syntax not supported".to_string()
      }
      P10Error::SpecificCharacterSetInvalid { .. } => {
        "Specific character set invalid".to_string()
      }
      P10Error::DataRequired { .. } => "Data required".to_string(),
      P10Error::DataEndedUnexpectedly { .. } => {
        "Unexpected end of data".to_string()
      }
      P10Error::DataInvalid { .. } => "Invalid data".to_string(),
      P10Error::MaximumExceeded { .. } => "Maximum exceeded".to_string(),
      P10Error::PartStreamInvalid { .. } => {
        "P10 part stream invalid".to_string()
      }
      P10Error::WriteAfterCompletion { .. } => {
        "Write after completion".to_string()
      }
      P10Error::FileError { .. } => "File I/O failure".to_string(),
      P10Error::OtherError { error_type, .. } => error_type.clone(),
    }
  }
}

impl DcmfxError for P10Error {
  /// Returns lines of text that describe a DICOM P10 error in a human-readable
  /// format.
  ///
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    let mut lines = vec![];

    lines.push(format!("DICOM P10 error {}", task_description));
    lines.push("".to_string());

    // Add the name of the error
    lines.push(format!("  Error: {}", self.name()));

    // Add the 'when' if it is present
    match self {
      P10Error::DataRequired { when }
      | P10Error::DataEndedUnexpectedly { when, .. }
      | P10Error::DataInvalid { when, .. }
      | P10Error::PartStreamInvalid { when, .. }
      | P10Error::FileError { when, .. } => {
        lines.push(format!("  When: {}", when));
      }

      _ => (),
    };

    // Add the details if present
    match self {
      P10Error::TransferSyntaxNotSupported {
        transfer_syntax_uid,
      } => {
        lines.push(format!("  Transfer syntax UID: {}", transfer_syntax_uid));
      }

      P10Error::SpecificCharacterSetInvalid {
        specific_character_set,
        details,
      } => {
        lines.push(format!(
          "  Specific character set: {}",
          specific_character_set
        ));
        lines.push(format!("  Details: {}", details));
      }

      P10Error::PartStreamInvalid { details, part, .. } => {
        lines.push(format!("  Details: {}", details));
        lines.push(format!("  Part: {}", part));
      }

      P10Error::DataInvalid { details, .. }
      | P10Error::MaximumExceeded { details, .. }
      | P10Error::FileError { details, .. }
      | P10Error::OtherError { details, .. } => {
        lines.push(format!("  Details: {}", details));
      }

      _ => (),
    };

    // Add the path and offset if present
    match self {
      P10Error::DataEndedUnexpectedly { offset, path, .. }
      | P10Error::DataInvalid { path, offset, .. }
      | P10Error::MaximumExceeded { offset, path, .. } => {
        lines.push(format!("  Path: {}", path.to_detailed_string()));
        lines.push(format!("  Offset: 0x{:X}", offset));
      }

      _ => (),
    };

    lines
  }
}

impl std::error::Error for P10Error {}
