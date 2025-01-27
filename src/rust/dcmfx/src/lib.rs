//! DCMfx is a collection of libraries and a CLI tool for working with the DICOM
//! standard, the international standard for medical images and related
//! information.

/// Anonymization of data sets by removing data elements that identify the
/// patient, or potentially contribute to identification of the patient.
///
/// This module is a re-export of the `dcmfx_anonymize` crate.
///
pub mod anonymize {
  pub use dcmfx_anonymize::*;
}

/// Provides core DICOM concepts including data sets, data elements, value
/// representations, transfer syntaxes, and a dictionary of the data elements
/// defined in DICOM PS3.6 as well as well-known private data elements.
///
/// This module is a re-export of the `dcmfx_core` crate.
///
pub mod core {
  pub use dcmfx_core::*;
}

/// Converts between DICOM data sets and DICOM JSON.
///
/// This module is a re-export of the `dcmfx_json` crate.
///
pub mod json {
  pub use dcmfx_json::*;
}

/// Reads and writes the DICOM Part 10 (P10) binary format used to store and
/// transmit DICOM-based medical imaging information.
///
/// This module is a re-export of the `dcmfx_p10` crate.
///
pub mod p10 {
  pub use dcmfx_p10::*;
}

/// Extracts frames of pixel data from data sets and streams of DICOM P10 parts.
///
/// This module is a re-export of the `dcmfx_pixel_data` crate.
///
pub mod pixel_data {
  pub use dcmfx_pixel_data::*;
}

mod integration_tests;
