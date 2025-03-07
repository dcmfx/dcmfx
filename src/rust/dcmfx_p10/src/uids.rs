//! Defines DCMfx's root UID prefix and implementation details that are stored
//! into File Meta Information of DICOM P10 it serializes.

#[cfg(not(feature = "std"))]
use alloc::{format, string::String};

/// DCMfx's unique root UID prefix. This was allocated via Medical Connections'
/// FreeUID service: <https://www.medicalconnections.co.uk/FreeUID.html>.
///
pub const DCMFX_ROOT_UID_PREFIX: &str = "1.2.826.0.1.3680043.10.1462.2.";

/// DCMfx's implementation class UID that is included in the File Meta
/// Information header of DICOM P10 data it serializes.
///
pub const DCMFX_IMPLEMENTATION_CLASS_UID: &str =
  "1.2.826.0.1.3680043.10.1462.2.0";

/// DCMfx's implementation version name that is included in the File Meta
/// Information header of DICOM P10 data it serializes.
///
pub fn dcmfx_implementation_version_name() -> String {
  format!("DCMfx {}", env!("CARGO_PKG_VERSION"))
}
