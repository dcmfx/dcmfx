//! Anonymization of data sets by removing data elements that identify the
//! patient, or potentially contribute to identification of the patient.

use dcmfx_core::{DataElementTag, DataSet, ValueRepresentation, dictionary};

/// A list of data elements that identify the patient, or potentially contribute
/// to identification of the patient, and that should be removed during
/// anonymization.
///
/// Note that this list does not include the *'(0008,0018) SOP Instance UID'*,
/// *'(0020,000E) Series Instance UID'*, or *'(0020,000D) Study Instance UID'*
/// data elements.
///
pub const IDENTIFYING_DATA_ELEMENTS: [&dictionary::Item; 80] = [
  &dictionary::ACCESSION_NUMBER,
  &dictionary::ADMITTING_DIAGNOSES_CODE_SEQUENCE,
  &dictionary::ADMITTING_DIAGNOSES_DESCRIPTION,
  &dictionary::ALLERGIES,
  &dictionary::BRANCH_OF_SERVICE,
  &dictionary::CONTENT_SEQUENCE,
  &dictionary::COUNTRY_OF_RESIDENCE,
  &dictionary::ETHNIC_GROUP,
  &dictionary::INSTANCE_CREATOR_UID,
  &dictionary::INSTITUTION_ADDRESS,
  &dictionary::INSTITUTION_CODE_SEQUENCE,
  &dictionary::INSTITUTION_NAME,
  &dictionary::INSTITUTIONAL_DEPARTMENT_NAME,
  &dictionary::INSTITUTIONAL_DEPARTMENT_NAME,
  &dictionary::INSTITUTIONAL_DEPARTMENT_TYPE_CODE_SEQUENCE,
  &dictionary::INVENTORY_ACCESS_END_POINTS_SEQUENCE,
  &dictionary::MEDICAL_RECORD_LOCATOR,
  &dictionary::MILITARY_RANK,
  &dictionary::NAME_OF_PHYSICIANS_READING_STUDY,
  &dictionary::NETWORK_ID,
  &dictionary::OCCUPATION,
  &dictionary::OPERATOR_IDENTIFICATION_SEQUENCE,
  &dictionary::OPERATORS_NAME,
  &dictionary::OTHER_PATIENT_IDS_SEQUENCE,
  &dictionary::OTHER_PATIENT_IDS,
  &dictionary::OTHER_PATIENT_NAMES,
  &dictionary::PATIENT_AGE,
  &dictionary::PATIENT_ALTERNATIVE_CALENDAR,
  &dictionary::PATIENT_BIRTH_DATE_IN_ALTERNATIVE_CALENDAR,
  &dictionary::PATIENT_BIRTH_DATE,
  &dictionary::PATIENT_BIRTH_NAME,
  &dictionary::PATIENT_BIRTH_TIME,
  &dictionary::PATIENT_BREED_DESCRIPTION,
  &dictionary::PATIENT_COMMENTS,
  &dictionary::PATIENT_DEATH_DATE_IN_ALTERNATIVE_CALENDAR,
  &dictionary::PATIENT_ID,
  &dictionary::PATIENT_MOTHER_BIRTH_NAME,
  &dictionary::PATIENT_NAME,
  &dictionary::PATIENT_RELIGIOUS_PREFERENCE,
  &dictionary::PATIENT_SEX,
  &dictionary::PATIENT_SIZE,
  &dictionary::PATIENT_SPECIES_DESCRIPTION,
  &dictionary::PATIENT_STATE,
  &dictionary::PATIENT_TELEPHONE_NUMBERS,
  &dictionary::PATIENT_WEIGHT,
  &dictionary::PERFORMING_PHYSICIAN_IDENTIFICATION_SEQUENCE,
  &dictionary::PERFORMING_PHYSICIAN_NAME,
  &dictionary::PERSON_ADDRESS,
  &dictionary::PERSON_TELECOM_INFORMATION,
  &dictionary::PERSON_TELEPHONE_NUMBERS,
  &dictionary::PHYSICIANS_OF_RECORD_IDENTIFICATION_SEQUENCE,
  &dictionary::PHYSICIANS_OF_RECORD,
  &dictionary::PHYSICIANS_OF_RECORD,
  &dictionary::PHYSICIANS_READING_STUDY_IDENTIFICATION_SEQUENCE,
  &dictionary::PREGNANCY_STATUS,
  &dictionary::PROCEDURE_CODE_SEQUENCE,
  &dictionary::PROTOCOL_NAME,
  &dictionary::REFERENCED_FRAME_OF_REFERENCE_UID,
  &dictionary::REFERRING_PHYSICIAN_ADDRESS,
  &dictionary::REFERRING_PHYSICIAN_IDENTIFICATION_SEQUENCE,
  &dictionary::REFERRING_PHYSICIAN_NAME,
  &dictionary::REFERRING_PHYSICIAN_TELEPHONE_NUMBERS,
  &dictionary::REGION_OF_RESIDENCE,
  &dictionary::REQUEST_ATTRIBUTES_SEQUENCE,
  &dictionary::REQUESTING_SERVICE,
  &dictionary::RESPONSIBLE_ORGANIZATION,
  &dictionary::RESPONSIBLE_PERSON_ROLE,
  &dictionary::RESPONSIBLE_PERSON,
  &dictionary::SCHEDULED_PROCEDURE_STEP_ID,
  &dictionary::SERIES_DESCRIPTION_CODE_SEQUENCE,
  &dictionary::SERIES_DESCRIPTION,
  &dictionary::STATION_AE_TITLE,
  &dictionary::STATION_NAME,
  &dictionary::STATION_NAME,
  &dictionary::STORAGE_MEDIA_FILE_SET_UID,
  &dictionary::STUDY_ACCESS_END_POINTS_SEQUENCE,
  &dictionary::STUDY_DESCRIPTION,
  &dictionary::STUDY_ID,
  &dictionary::TIMEZONE_OFFSET_FROM_UTC,
  &dictionary::UID,
];

/// Returns whether the given tag is allowed through the anonymization process.
///
pub fn filter_tag(tag: DataElementTag, vr: ValueRepresentation) -> bool {
  // Strip all tags that specify an ApplicationEntity which could be identifying
  if vr == ValueRepresentation::ApplicationEntity {
    return false;
  }

  // Strip private tags
  if tag.is_private() {
    return false;
  }

  // Strip all patient tags
  if tag.group == 0x0010 {
    return false;
  }

  // Strip all tags in the above list
  !IDENTIFYING_DATA_ELEMENTS.iter().any(|item| item.tag == tag)
}

/// Adds functions to [`DataSet`] to perform anonymization.
///
pub trait DataSetAnonymizeExtensions {
  /// Anonymizes a data set by removing data elements that identify the patient,
  /// or potentially contribute to identification of the patient.
  ///
  fn anonymize(&mut self);
}

impl DataSetAnonymizeExtensions for DataSet {
  fn anonymize(&mut self) {
    for el in IDENTIFYING_DATA_ELEMENTS {
      self.delete(el.tag);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn filter_tag_test() {
    assert!(filter_tag(
      dictionary::SPECIFIC_CHARACTER_SET.tag,
      ValueRepresentation::CodeString,
    ));

    assert!(!filter_tag(
      dictionary::UID.tag,
      ValueRepresentation::UniqueIdentifier
    ));

    assert!(!filter_tag(
      dictionary::STATION_AE_TITLE.tag,
      ValueRepresentation::ApplicationEntity,
    ));

    assert!(!filter_tag(
      DataElementTag::new(0x0009, 0x0002),
      ValueRepresentation::CodeString,
    ));

    assert!(!filter_tag(
      DataElementTag::new(0x0010, 0xABCD),
      ValueRepresentation::PersonName,
    ));
  }
}
