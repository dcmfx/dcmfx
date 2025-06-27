import dcmfx_p10/uids
import gleam/int

/// Configuration used when writing DICOM P10 data.
///
pub type P10WriteConfig {
  P10WriteConfig(
    implementation_class_uid: String,
    implementation_version_name: String,
    zlib_compression_level: Int,
  )
}

/// Returns the default write config.
///
pub fn new() -> P10WriteConfig {
  P10WriteConfig(
    implementation_class_uid: uids.dcmfx_implementation_class_uid,
    implementation_version_name: uids.dcmfx_implementation_version_name,
    zlib_compression_level: 6,
  )
}

/// The implementation class UID that will be included in the File Meta
/// Information header of serialized DICOM P10 data.
///
/// Defaults to the value of `dcmfx_p10/uids.dcmfx_implementation_class_uid`.
///
pub fn implementation_class_uid(
  config: P10WriteConfig,
  value: String,
) -> P10WriteConfig {
  P10WriteConfig(..config, implementation_class_uid: value)
}

/// The implementation version name that will be included in the File Meta
/// Information header of serialized DICOM P10 data.
///
/// Defaults to the value of `dcmfx_p10/uids.dcmfx_implementation_version_name`.
///
pub fn implementation_version_name(
  config: P10WriteConfig,
  value: String,
) -> P10WriteConfig {
  P10WriteConfig(..config, implementation_version_name: value)
}

/// The zlib compression level to use when the transfer syntax being used is
/// deflated. There are only three deflated transfer syntaxes: 'Deflated
/// Explicit VR Little Endian', 'JPIP Referenced Deflate', and 'JPIP HTJ2K
/// Referenced Deflate'.
///
/// The level ranges from 0, meaning no compression, through to 9, which gives
/// the best compression at the cost of speed.
///
/// Default: 6.
///
pub fn zlib_compression_level(
  config: P10WriteConfig,
  value: Int,
) -> P10WriteConfig {
  P10WriteConfig(..config, zlib_compression_level: int.clamp(value, 0, 9))
}
