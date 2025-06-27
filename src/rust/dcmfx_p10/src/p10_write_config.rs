#[cfg(not(feature = "std"))]
use alloc::string::{String, ToString};

pub use crate::uids;

/// Configuration used when writing DICOM P10 data.
///
#[derive(Clone, Debug, PartialEq)]
pub struct P10WriteConfig {
  pub(crate) implementation_class_uid: String,
  pub(crate) implementation_version_name: String,
  pub(crate) zlib_compression_level: u32,
}

impl Default for P10WriteConfig {
  fn default() -> Self {
    Self {
      implementation_class_uid: uids::DCMFX_IMPLEMENTATION_CLASS_UID
        .to_string(),
      implementation_version_name: uids::DCMFX_IMPLEMENTATION_VERSION_NAME
        .to_string(),
      zlib_compression_level: 6,
    }
  }
}

impl P10WriteConfig {
  /// The implementation class UID that will be included in the File Meta
  /// Information header of serialized DICOM P10 data.
  ///
  /// Defaults to the value of [`uids::DCMFX_IMPLEMENTATION_CLASS_UID`].
  ///
  pub fn implementation_class_uid(mut self, value: String) -> Self {
    self.implementation_class_uid = value;
    self
  }

  /// The implementation version name that will be included in the File Meta
  /// Information header of serialized DICOM P10 data.
  ///
  /// Defaults to the value of [`uids::DCMFX_IMPLEMENTATION_VERSION_NAME`].
  ///
  pub fn implementation_version_name(mut self, value: String) -> Self {
    self.implementation_version_name = value;
    self
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
  pub fn zlib_compression_level(mut self, value: u32) -> Self {
    self.zlib_compression_level = value.clamp(0, 9);
    self
  }
}
