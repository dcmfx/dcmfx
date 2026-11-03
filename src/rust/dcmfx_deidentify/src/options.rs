#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

/// The options to the Basic Application Level Confidentiality Profile that an
/// [`Deidentifier`](crate::Deidentifier) applies. Each option is defined in Annex
/// E.3 of DICOM PS3.15 and modifies the Basic Profile by retaining or cleaning
/// data elements that the Basic Profile would otherwise remove or replace.
///
/// With no options active, the Basic Profile is applied unmodified, which is
/// the most aggressive de-identification. Each active option is recorded in
/// the de-identified data set in De-identification Method (0012,0063) and
/// De-identification Method Code Sequence (0012,0064).
///
/// Where the standard specifies that a data element is retained under an
/// option only if it is cleaned, e.g. Application Entity Titles under the
/// Retain Device Identity Option and free text such as Allergies (0010,2110)
/// under the Retain Patient Characteristics Option, the data element is not
/// retained and the Basic Profile's action is applied instead, because
/// cleaning of free text is not performed.
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfileOptions {
  /// The Retain Patient Characteristics Option, defined in PS3.15 E.3.7.
  /// Information about the patient's age, sex, size, weight, and other
  /// physical characteristics is retained. This is needed by applications
  /// that compute metabolic measures such as PET Standard Uptake Values.
  ///
  pub retain_patient_characteristics: bool,

  /// The Retain Device Identity Option, defined in PS3.15 E.3.8. Information
  /// about the identity of the device used to perform the acquisition, such as
  /// serial numbers, station names, and calibration dates, is retained.
  /// Application Entity Titles are not retained because the standard requires
  /// them to be cleaned rather than kept.
  ///
  pub retain_device_identity: bool,

  /// The Retain Institution Identity Option, defined in PS3.15 E.3.11.
  /// Information about the identity of the institution where the acquisition
  /// was performed, such as its name, address, and department, is retained.
  ///
  pub retain_institution_identity: bool,

  /// The Retain UIDs Option, defined in PS3.15 E.3.9. UIDs are retained rather
  /// than replaced, which maintains an audit trail back to the original
  /// instances.
  ///
  pub retain_uids: bool,

  /// One of the two Retain Longitudinal Temporal Information Options defined
  /// in PS3.15 E.3.6, which retain dates and times either unmodified or shifted
  /// by a consistent number of days. When `None`, dates and times are removed
  /// or replaced with dummy values as the Basic Profile specifies.
  ///
  /// Patient's Birth Date (0010,0030) is not covered by these options and is
  /// always replaced with a zero length value.
  ///
  /// Timezone Offset From UTC (0008,0201) is kept under both options. With
  /// modified dates the shift is a whole number of days and times aren't
  /// changed, so the offset remains correct.
  ///
  pub retain_longitudinal_temporal_information:
    Option<LongitudinalTemporalInformation>,

  /// The Clean Structured Content Option, defined in PS3.15 E.3.4. Content
  /// Sequence (0040,A730), Acquisition Context Sequence (0040,0555), Specimen
  /// Preparation Sequence (0040,0610), and Waveform Annotation Sequence
  /// (0040,B020) are retained and their content items are cleaned: content
  /// items whose concept name is listed in Table E.3.4-1 of PS3.15 are
  /// removed or have their value replaced with a dummy value as that table
  /// specifies, taking into account the other active options, and the data
  /// elements of all other content items are subject to the profile in the
  /// usual way. See [`ContentItemAction`](crate::ContentItemAction). This
  /// allows Structured Reports to be de-identified while retaining their
  /// findings and measurements.
  ///
  /// Text content items whose concept name isn't listed in the table are
  /// retained verbatim, as the standard specifies. Free text such as findings
  /// isn't scanned for names or other identifying information. Removing a
  /// content item moves up the content items that follow it in the same
  /// sequence, so a by-reference relationship that identifies one of them by
  /// its position no longer resolves.
  ///
  /// Without this option, the Basic Profile replaces these sequences with
  /// empty sequences, which discards the entire content of a Structured
  /// Report.
  ///
  pub clean_structured_content: bool,
}

/// The two mutually exclusive Retain Longitudinal Temporal Information Options
/// defined in PS3.15 E.3.6. These retain dates and times that the Basic
/// Profile would otherwise remove or replace, so that the temporal
/// relationships between instances are preserved.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LongitudinalTemporalInformation {
  /// The Retain Longitudinal Temporal Information With Full Dates Option.
  /// Dates and times are retained unmodified. Longitudinal Temporal
  /// Information Modified (0028,0303) is set to `UNMODIFIED`.
  ///
  FullDates,

  /// The Retain Longitudinal Temporal Information With Modified Dates Option.
  /// Dates, and the date part of date/times, are shifted by a consistent
  /// number of days, which preserves the temporal relationships between
  /// instances while making the dates themselves meaningless. Times are not
  /// modified, so the fine temporal relationships within a day are preserved.
  /// Longitudinal Temporal Information Modified (0028,0303) is set to
  /// `MODIFIED`.
  ///
  /// Dates that are invalid and can't be shifted are replaced with a dummy
  /// value.
  ///
  ModifiedDates {
    /// The number of days to shift dates by, which is usually negative so
    /// that dates are moved into the past. When `None`, the shift is derived
    /// from the de-identifier's UID secret and lies between 1 and 3650 days into
    /// the past, so de-identifiers created with the same secret shift dates by
    /// the same amount. See [`Deidentifier::date_shift_days()`]. A shift of
    /// zero is rejected when de-identifying, see
    /// [`DeidentifyError::ZeroDateShift`], as it would leave dates unmodified
    /// while recording them as modified.
    ///
    /// [`Deidentifier::date_shift_days()`]: crate::Deidentifier::date_shift_days
    /// [`DeidentifyError::ZeroDateShift`]: crate::DeidentifyError::ZeroDateShift
    ///
    date_shift_days: Option<i32>,
  },
}

/// A code from CID 7050 "De-identification Method" in DICOM PS3.16 that
/// identifies the Basic Profile or one of its options in De-identification
/// Method Code Sequence (0012,0064).
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeidentificationMethodCode {
  /// The code value in the DCM coding scheme.
  ///
  pub code_value: &'static str,

  /// The code meaning, which is also used as the value added to
  /// De-identification Method (0012,0063).
  ///
  pub code_meaning: &'static str,
}

/// The code for the Basic Application Confidentiality Profile.
///
pub const BASIC_PROFILE_CODE: DeidentificationMethodCode =
  DeidentificationMethodCode {
    code_value: "113100",
    code_meaning: "Basic Application Confidentiality Profile",
  };

const CLEAN_STRUCTURED_CONTENT_CODE: DeidentificationMethodCode =
  DeidentificationMethodCode {
    code_value: "113104",
    code_meaning: "Clean Structured Content Option",
  };

const RETAIN_FULL_DATES_CODE: DeidentificationMethodCode =
  DeidentificationMethodCode {
    code_value: "113106",
    code_meaning: "Retain Longitudinal Temporal Information Full Dates Option",
  };

const RETAIN_MODIFIED_DATES_CODE: DeidentificationMethodCode =
  DeidentificationMethodCode {
    code_value: "113107",
    code_meaning: "Retain Longitudinal Temporal Information Modified Dates Option",
  };

const RETAIN_PATIENT_CHARACTERISTICS_CODE: DeidentificationMethodCode =
  DeidentificationMethodCode {
    code_value: "113108",
    code_meaning: "Retain Patient Characteristics Option",
  };

const RETAIN_DEVICE_IDENTITY_CODE: DeidentificationMethodCode =
  DeidentificationMethodCode {
    code_value: "113109",
    code_meaning: "Retain Device Identity Option",
  };

const RETAIN_UIDS_CODE: DeidentificationMethodCode =
  DeidentificationMethodCode {
    code_value: "113110",
    code_meaning: "Retain UIDs Option",
  };

const RETAIN_INSTITUTION_IDENTITY_CODE: DeidentificationMethodCode =
  DeidentificationMethodCode {
    code_value: "113112",
    code_meaning: "Retain Institution Identity Option",
  };

impl ProfileOptions {
  /// Returns the codes that identify the Basic Profile and each of the active
  /// options. These are the items added to De-identification Method Code
  /// Sequence (0012,0064), and their meanings are the values added to
  /// De-identification Method (0012,0063).
  ///
  pub fn deidentification_method_codes(
    &self,
  ) -> Vec<DeidentificationMethodCode> {
    let mut codes = vec![BASIC_PROFILE_CODE];

    if self.clean_structured_content {
      codes.push(CLEAN_STRUCTURED_CONTENT_CODE);
    }

    match self.retain_longitudinal_temporal_information {
      Some(LongitudinalTemporalInformation::FullDates) => {
        codes.push(RETAIN_FULL_DATES_CODE)
      }
      Some(LongitudinalTemporalInformation::ModifiedDates { .. }) => {
        codes.push(RETAIN_MODIFIED_DATES_CODE)
      }
      None => (),
    }

    if self.retain_patient_characteristics {
      codes.push(RETAIN_PATIENT_CHARACTERISTICS_CODE);
    }

    if self.retain_device_identity {
      codes.push(RETAIN_DEVICE_IDENTITY_CODE);
    }

    if self.retain_uids {
      codes.push(RETAIN_UIDS_CODE);
    }

    if self.retain_institution_identity {
      codes.push(RETAIN_INSTITUTION_IDENTITY_CODE);
    }

    codes
  }

  /// Returns the value for Longitudinal Temporal Information Modified
  /// (0028,0303) that records how dates and times were handled.
  ///
  pub fn longitudinal_temporal_information_modified(&self) -> &'static str {
    match self.retain_longitudinal_temporal_information {
      None => "REMOVED",
      Some(LongitudinalTemporalInformation::FullDates) => "UNMODIFIED",
      Some(LongitudinalTemporalInformation::ModifiedDates { .. }) => "MODIFIED",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn deidentification_method_codes_test() {
    let codes = |options: ProfileOptions| -> Vec<&'static str> {
      options
        .deidentification_method_codes()
        .iter()
        .map(|code| code.code_value)
        .collect()
    };

    assert_eq!(codes(ProfileOptions::default()), vec!["113100"]);

    assert_eq!(
      codes(ProfileOptions {
        retain_patient_characteristics: true,
        retain_device_identity: true,
        retain_institution_identity: true,
        retain_uids: true,
        retain_longitudinal_temporal_information: Some(
          LongitudinalTemporalInformation::FullDates
        ),
        clean_structured_content: true,
      }),
      vec![
        "113100", "113104", "113106", "113108", "113109", "113110", "113112"
      ]
    );

    assert_eq!(
      codes(ProfileOptions {
        retain_longitudinal_temporal_information: Some(
          LongitudinalTemporalInformation::ModifiedDates {
            date_shift_days: None
          }
        ),
        ..Default::default()
      }),
      vec!["113100", "113107"]
    );
  }

  #[test]
  fn longitudinal_temporal_information_modified_test() {
    assert_eq!(
      ProfileOptions::default().longitudinal_temporal_information_modified(),
      "REMOVED"
    );
    assert_eq!(
      ProfileOptions {
        retain_longitudinal_temporal_information: Some(
          LongitudinalTemporalInformation::FullDates
        ),
        ..Default::default()
      }
      .longitudinal_temporal_information_modified(),
      "UNMODIFIED"
    );
    assert_eq!(
      ProfileOptions {
        retain_longitudinal_temporal_information: Some(
          LongitudinalTemporalInformation::ModifiedDates {
            date_shift_days: Some(-10)
          }
        ),
        ..Default::default()
      }
      .longitudinal_temporal_information_modified(),
      "MODIFIED"
    );
  }
}
