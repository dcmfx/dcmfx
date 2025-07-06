/// Config options used when converting a data set to DICOM JSON.
///
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
}
