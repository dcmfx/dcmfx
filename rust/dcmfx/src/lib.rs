//! DCMfx is a collection of libraries and a CLI tool for working with the DICOM
//! standard, the international standard for medical images and related
//! information.

/// Anonymization of data sets by removing data elements that identify the
/// patient, or potentially contribute to identification of the patient.
///
pub mod anonymize {
  pub use dcmfx_anonymize::*;
}

/// Decodes DICOM string data that uses a Specific Character Set into a native
/// UTF-8 string.
///
pub mod character_set {
  pub use dcmfx_character_set::*;
}

/// Provides core DICOM concepts including data sets, data elements, value
/// representations, transfer syntaxes, and a registry of the data elements
/// defined in DICOM Part 6.
///
pub mod core {
  pub use dcmfx_core::*;
}

/// Converts between DICOM data sets and DICOM JSON.
///
pub mod json {
  pub use dcmfx_json::*;
}

/// Reads and writes the DICOM Part 10 (P10) binary format used to store and
/// transmit DICOM-based medical imaging information.
///
pub mod p10 {
  pub use dcmfx_p10::*;
}

/// Extracts frames of pixel data present in a data set.
///
pub mod pixel_data {
  pub use dcmfx_pixel_data::*;
}

mod integration_tests;
