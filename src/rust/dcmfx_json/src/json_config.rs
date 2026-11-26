#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use dcmfx_core::{DataElementTag, ValueRepresentation};

/// Config options used when converting a data set to DICOM JSON.
///
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DicomJsonConfig {
  /// Whether to extend DICOM JSON to support encapsulated pixel data elements.
  /// This is done by encoding the encapsulated pixel data fragments to exactly
  /// match the DICOM P10 format, then storing it as an `InlineBinary`.
  ///
  /// Enabling this extension also causes the *'(0002,0010) Transfer Syntax'*
  /// data element to be present in the DICOM JSON, as it's needed to interpret
  /// the encapsulated pixel data.
  ///
  /// This option is disabled by default as it's not a part of the DICOM JSON
  /// standard, which means that data sets with encapsulated pixel data elements
  /// will error on conversion to DICOM JSON.
  ///
  pub store_encapsulated_pixel_data: bool,

  /// Whether to format the DICOM JSON for readability with newlines and
  /// indentation. This increases the size of the output but is easier to
  /// directly inspect.
  ///
  pub pretty_print: bool,

  /// The data element tags to emit binary data values for in the DICOM JSON,
  /// or `None` if binary data values should always be emitted.
  ///
  pub selected_binary_data_values: Option<Vec<DataElementTag>>,

  /// The data elements for which invalid data should be ignored rather than
  /// causing the JSON conversion to error. Such data elements are emitted
  /// without a value, i.e. the same as if their value was empty.
  ///
  pub ignore_invalid_data: IgnoreInvalidData,
}

/// Specifies the data elements for which invalid data is ignored when
/// converting to DICOM JSON. The default value doesn't ignore invalid data in
/// any data elements.
///
/// A data element's invalid data is ignored if it matches any of the fields,
/// i.e. the fields are combined as a union.
///
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IgnoreInvalidData {
  /// The tags of specific data elements for which invalid data is ignored.
  ///
  pub tags: Vec<DataElementTag>,

  /// The value representations for which invalid data is ignored in all data
  /// elements.
  ///
  pub vrs: Vec<ValueRepresentation>,

  /// The value representations for which invalid data is ignored in private
  /// data elements.
  ///
  pub private_vrs: Vec<ValueRepresentation>,

  /// Whether invalid data is ignored in all private data elements, regardless
  /// of their value representation.
  ///
  pub private: bool,
}

impl IgnoreInvalidData {
  /// Returns whether invalid data in the specified data element should be
  /// ignored.
  ///
  pub fn matches(&self, tag: DataElementTag, vr: ValueRepresentation) -> bool {
    self.tags.contains(&tag)
      || self.vrs.contains(&vr)
      || (tag.is_private() && (self.private || self.private_vrs.contains(&vr)))
  }
}
