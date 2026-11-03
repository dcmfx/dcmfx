#[cfg(not(feature = "std"))]
use alloc::{format, string::String};

use dcmfx_core::DataElementTag;

/// An error that occurs when de-identifying a data set.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeidentifyError {
  /// The Retain Longitudinal Temporal Information With Modified Dates Option
  /// is active with a date shift of zero days, which would leave dates
  /// unmodified while recording them as modified. Use the Retain Longitudinal
  /// Temporal Information With Full Dates Option to retain dates unmodified.
  ///
  ZeroDateShift,

  /// The value that replaces the value of the given data element is too long
  /// to be encoded in its VR, e.g. a multi-valued UID whose replacement UIDs
  /// are longer than the originals and exceed the 65534 bytes that a UI value
  /// can hold.
  ///
  ValueTooLong { tag: DataElementTag },

  /// The value of the given data element is longer than the 64 KiB that is
  /// held in memory for a value whose replacement depends on it, i.e. a UID
  /// that is replaced or a date that is shifted. Such values can only be this
  /// long in an implicit VR data set, as explicit VR data sets cap them at
  /// 65534 bytes.
  ///
  OriginalValueTooLong { tag: DataElementTag },
}

impl DeidentifyError {
  /// Returns a description of this error.
  ///
  pub fn details(&self) -> String {
    match self {
      Self::ZeroDateShift => "The date shift of the Retain Longitudinal \
         Temporal Information With Modified Dates Option must not be zero, \
         as that would leave dates unmodified while recording them as \
         modified"
        .into(),

      Self::ValueTooLong { tag } => format!(
        "The de-identified value of data element {tag} is too long to be \
         encoded in its VR"
      ),

      Self::OriginalValueTooLong { tag } => format!(
        "The value of data element {tag} is longer than the 64 KiB that can be \
         held in memory while it is replaced"
      ),
    }
  }
}

impl core::fmt::Display for DeidentifyError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.write_str(&self.details())
  }
}

#[cfg(feature = "std")]
impl std::error::Error for DeidentifyError {}
