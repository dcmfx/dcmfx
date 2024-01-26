/// Config options used when converting a data set to DICOM JSON. The following
/// config is available:
///
/// ### `store_encapsulated_pixel_data: Bool`
///
/// Whether to extend DICOM JSON to support encapsulated pixel data elements.
/// This is done by encoding the encapsulated pixel data fragments to exactly
/// match the DICOM P10 format, then storing it as an `InlineBinary`.
///
/// Enabling this extension also causes the '(0002,0010) Transfer Syntax' data
/// element to be present in the DICOM JSON, as it's needed to interpret the
/// encapsulated pixel data.
///
/// This option is disabled by default as it's not a part of the DICOM JSON
/// standard, which means that data sets with encapsulated pixel data elements
/// won't convert to JSON.
///
pub type DicomJsonConfig {
  DicomJsonConfig(store_encapsulated_pixel_data: Bool)
}
