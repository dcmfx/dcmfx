//// Defines DCMfx's root UID prefix and implementation details that are stored
//// into File Meta Information of DICOM P10 it serializes.

/// DCMfx's unique root UID prefix. This was allocated via Medical Connections'
/// FreeUID service: https://www.medicalconnections.co.uk/FreeUID.html.
///
pub const dcmfx_root_uid_prefix = "1.2.826.0.1.3680043.10.1462.1."

/// DCMfx's implementation class UID that is included in the File Meta
/// Information header of DICOM P10 data it serializes.
///
pub const dcmfx_implementation_class_uid = dcmfx_root_uid_prefix <> "0"

/// DCMfx's implementation version name that is included in the File Meta
/// Information header of DICOM P10 data it serializes.
///
pub const dcmfx_implementation_version_name = "DCMfx " <> "0.31.0"
