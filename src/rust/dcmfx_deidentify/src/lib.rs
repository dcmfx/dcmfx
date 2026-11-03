//! De-identification of DICOM data sets and streams of DICOM P10 tokens following
//! the Basic Application Level Confidentiality Profile defined in Annex E of
//! DICOM PS3.15, with support for several of the profile's options.
//!
//! The Basic Profile is a conservative de-identification that removes
//! information about the identity and demographics of the patient, the
//! identities of other people and organizations involved in the procedure,
//! private data elements, and information such as UIDs, dates, and times that
//! could be used to match a de-identified instance back to its original.
//!
//! Every data element listed in Table E.1-1 of PS3.15 has an [`Action`] applied
//! to it, whether it appears in the root data set or inside a sequence item:
//!
//! - **Remove (X).** The data element is removed.
//! - **Replace with zero length (Z).** The value is replaced with a zero length
//!   value of the same VR.
//! - **Replace with dummy (D).** The value is replaced with a non-zero length
//!   dummy value of the same VR, e.g. `"DEIDENTIFIED"` for text, `19000101` for
//!   dates, and `0` for numbers. A sequence is replaced with an empty
//!   sequence, so a sequence that its module requires to have an item, such
//!   as the Content Sequence of a Structured Report, leaves the instance
//!   invalid.
//! - **Replace UID (U).** The UID is replaced with a new UID derived from the
//!   original using a secret, so the same original UID always maps to the same
//!   replacement UID within the scope of an [`Deidentifier`] and references
//!   between instances stay intact. Zero length UIDs are left empty.
//!
//! A data element whose value has the UN VR is removed, as its value can't be
//! inspected. The P10 reader gives every data element in the data dictionary
//! its VR from the dictionary, so this affects data elements that aren't in
//! the dictionary and the values of DICOM JSON or in-memory data sets that
//! are encoded as UN.
//!
//! Encapsulated Document (0042,0011), which the table replaces with a dummy
//! value, is removed instead, together with the MIME type and length that
//! describe it, as no dummy document carries anything of use. An Encapsulated
//! PDF, CDA, STL, OBJ, or MTL instance therefore comes out without the
//! document that its SOP class requires, and is no longer a conformant
//! instance of that class.
//!
//! Where the table makes the action conditional on the type of the data element
//! in the IOD, e.g. X/Z/D, the action that maintains conformance for every
//! type is used. A few data elements that the table removes but that are
//! required by their modules are replaced with dummy or zero-length values
//! instead, so that the result stays valid. In addition to the table, all
//! private data elements, group lengths, curve data, overlays, group 0004
//! data elements, and any data elements in the patient group or with a PN, AE,
//! DA, DT, or TM VR that aren't listed in the table are removed, and a few
//! unlisted UIDs that reference other objects, and unlisted labels and
//! identifiers that modules require, are handled like the listed data
//! elements they resemble. Other unlisted free text, such as descriptions and
//! comments, is kept. The 128-byte file preamble is zeroed and the File Meta
//! Information is reduced to its structural data elements.
//!
//! Some instances come out invalid: a DICOMDIR loses its directory records,
//! which the profile removes, and should instead be regenerated from the
//! de-identified files it references; an image whose pixel data is held by a
//! JPIP provider loses its Pixel Data Provider URL (0028,7FE0) and with it
//! its pixel data; and the retired Standalone Overlay and Standalone Curve
//! Storage instances lose all of their content. Data sets that aren't
//! composite instances, such as those of the Modality Worklist and Modality
//! Performed Procedure Step services, take the same actions, which remove some
//! of the data elements that their own definitions require.
//!
//! The following options defined in Annex E.3 of PS3.15 are supported and are
//! configured with [`ProfileOptions`]. Each one retains or cleans data
//! elements that the Basic Profile would otherwise remove or replace:
//!
//! - **Retain Patient Characteristics.** Age, sex, size, weight, and similar,
//!   including the species and breed of an animal patient.
//! - **Retain Device Identity.** Device serial numbers, station names,
//!   calibration dates, and similar.
//! - **Retain Institution Identity.** Institution names, addresses, and
//!   departments.
//! - **Retain UIDs.** UIDs are kept rather than replaced.
//! - **Retain Longitudinal Temporal Information.** Dates and times are either
//!   kept unmodified or have their dates shifted by a consistent number of
//!   days.
//! - **Clean Structured Content.** The content items of Structured Reports and
//!   similar structured content are retained and cleaned according to their
//!   concept names, rather than the whole content being discarded: content
//!   items whose concept names Table E.3.4-1 lists are removed or have their
//!   values replaced with dummy values, with the SNOMED CT code for Unknown
//!   replacing a code, and every other content item is kept, with text
//!   content items retained verbatim.
//!
//! Patient Identity Removed (0012,0062), De-identification Method (0012,0063),
//! De-identification Method Code Sequence (0012,0064) and Longitudinal
//! Temporal Information Modified (0028,0303) are set in the de-identified
//! data set, as the profile requires, and record which options were applied.
//! Any existing values of them, from an earlier de-identification, are
//! replaced.
//!
//! The Clean Pixel Data, Clean Recognizable Visual Features, Clean Graphics,
//! Clean Descriptors, and Retain Safe Private options are not implemented.
//!
//! # Pixel data is not inspected
//!
//! The profile acts on data elements, not on what pixel data depicts.
//! Identifying information that is burned into pixel data, such as the
//! patient's name and the date in the header of an ultrasound image, a
//! screenshot, or a scanned document, is not detected or removed, and nor are
//! recognizable features such as the face in a head CT or a photograph, or in
//! other representations of the patient such as surface meshes and point
//! clouds. Burned In Annotation (0028,0301) and Recognizable Visual Features
//! (0028,0302) are kept as they are so that they can be checked, but they are
//! optional and often absent, so their absence doesn't mean the pixel data is
//! free of identifying information. Metadata that a frame of encapsulated
//! pixel data carries inside its codestream, such as the EXIF, XMP, and ICC
//! profile segments of a JPEG frame, isn't stripped either, and nor is an
//! overlay plane that the retired encoding stores in the unused high bits of
//! the pixel samples. Images that may have identifying information in their
//! pixel data need it removed by other means, such as cropping or masking,
//! before or after de-identification.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod action;
mod basic_profile;
mod date_shift;
mod deidentifier;
mod error;
mod options;
mod p10_deidentify_transform;
mod utils;

pub use action::Action;
pub use basic_profile::{
  ContentItemAction, action_for_content_item, action_for_data_element,
};
pub use deidentifier::{DUMMY_CODE, DUMMY_TEXT, Deidentifier};
pub use error::DeidentifyError;
pub use options::{
  BASIC_PROFILE_CODE, DeidentificationMethodCode,
  LongitudinalTemporalInformation, ProfileOptions,
};
pub use p10_deidentify_transform::P10DeidentifyTransform;

use dcmfx_core::DataSet;

/// Adds functions to [`DataSet`] to perform de-identification.
///
pub trait DataSetDeidentifyExtensions {
  /// De-identifies this data set by applying the Basic Application Level
  /// Confidentiality Profile and the de-identifier's options to it. Every
  /// data element in the data set, including those inside sequence items and
  /// in the File Meta Information if present, has the action returned by
  /// [`action_for_data_element()`] applied to it, and then the data elements
  /// that record the de-identification are set, replacing any existing
  /// values:
  ///
  /// - Patient Identity Removed (0012,0062) is set to `YES`.
  /// - De-identification Method (0012,0063) holds the meaning of the code for
  ///   the Basic Profile and each active option.
  /// - De-identification Method Code Sequence (0012,0064) holds the code for
  ///   the Basic Profile and each active option.
  /// - Longitudinal Temporal Information Modified (0028,0303) is set to
  ///   `REMOVED`, `UNMODIFIED`, or `MODIFIED` depending on the active Retain
  ///   Longitudinal Temporal Information Option.
  ///
  /// Returns an error if the date shift is zero, see
  /// [`DeidentifyError::ZeroDateShift`], if a replacement value is too long
  /// to encode, see [`DeidentifyError::ValueTooLong`], or if a UID or date
  /// that is replaced is longer than 64 KiB, see
  /// [`DeidentifyError::OriginalValueTooLong`].
  ///
  /// This runs the data set's tokens through a [`P10DeidentifyTransform`] and
  /// rebuilds it from the result, so the result is exactly what
  /// de-identifying a stream of the data set's tokens produces.
  ///
  fn deidentify(
    &mut self,
    deidentifier: &Deidentifier,
  ) -> Result<(), DeidentifyError>;
}

impl DataSetDeidentifyExtensions for DataSet {
  fn deidentify(
    &mut self,
    deidentifier: &Deidentifier,
  ) -> Result<(), DeidentifyError> {
    p10_deidentify_transform::deidentify_data_set(deidentifier, self, true)
  }
}
