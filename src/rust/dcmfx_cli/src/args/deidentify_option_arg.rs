use clap::ValueEnum;

use dcmfx::deidentify::{LongitudinalTemporalInformation, ProfileOptions};

/// Enum for specifying an option to the Basic Application Level
/// Confidentiality Profile as a CLI argument.
///
#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum DeidentifyOptionArg {
  /// Retain the patient's age, sex, size, weight, and other physical
  /// characteristics. Free text patient characteristics such as allergies are
  /// not retained.
  RetainPatientCharacteristics,

  /// Retain the identity of the device used for the acquisition, such as its
  /// serial number, station name, and calibration dates. Application Entity
  /// Titles are not retained.
  RetainDeviceIdentity,

  /// Retain the name, address, and department of the institution where the
  /// acquisition was performed.
  RetainInstitutionIdentity,

  /// Retain UIDs rather than replacing them.
  RetainUids,

  /// Retain dates and times unmodified. Patient's Birth Date is not retained.
  RetainFullDates,

  /// Retain dates and times with their dates shifted by a consistent number of
  /// days, as specified by --deidentify-date-shift. Patient's Birth Date is not
  /// retained.
  RetainModifiedDates,

  /// Retain the content items of Structured Reports and similar structured
  /// content, removing or replacing those with identifying concept names,
  /// instead of discarding all content items.
  CleanStructuredContent,
}

/// Converts the de-identify option arguments into profile options. Returns an
/// error if the arguments are inconsistent.
///
pub fn profile_options(
  options: &[DeidentifyOptionArg],
  date_shift_days: Option<i32>,
) -> Result<ProfileOptions, String> {
  let has = |option| options.contains(&option);

  let retain_longitudinal_temporal_information = match (
    has(DeidentifyOptionArg::RetainFullDates),
    has(DeidentifyOptionArg::RetainModifiedDates),
  ) {
    (true, true) => {
      return Err(
        "The retain-full-dates and retain-modified-dates de-identify options \
         can't be used together"
          .to_string(),
      );
    }
    (true, false) => Some(LongitudinalTemporalInformation::FullDates),
    (false, true) => {
      Some(LongitudinalTemporalInformation::ModifiedDates { date_shift_days })
    }
    (false, false) => None,
  };

  if date_shift_days.is_some() && !has(DeidentifyOptionArg::RetainModifiedDates)
  {
    return Err(
      "The --deidentify-date-shift option is only valid when the \
       retain-modified-dates de-identify option is specified"
        .to_string(),
    );
  }

  if date_shift_days == Some(0) {
    return Err(
      "The --deidentify-date-shift option must not be zero, as that would \
       leave dates unmodified while recording them as modified"
        .to_string(),
    );
  }

  Ok(ProfileOptions {
    retain_patient_characteristics: has(
      DeidentifyOptionArg::RetainPatientCharacteristics,
    ),
    retain_device_identity: has(DeidentifyOptionArg::RetainDeviceIdentity),
    retain_institution_identity: has(
      DeidentifyOptionArg::RetainInstitutionIdentity,
    ),
    retain_uids: has(DeidentifyOptionArg::RetainUids),
    retain_longitudinal_temporal_information,
    clean_structured_content: has(DeidentifyOptionArg::CleanStructuredContent),
  })
}
