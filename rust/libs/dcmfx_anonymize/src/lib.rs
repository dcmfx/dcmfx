use dcmfx_core::{registry, DataElementTag, DataSet, ValueRepresentation};

/// A list of data elements that identify the patient, or potentially contribute
/// to identification of the patient, and that should be removed during
/// anonymization.
///
/// Note that this list does not include the 'SOP Instance UID', 'Series
/// Instance UID', or 'Study Instance UID' data elements.
///
pub const IDENTIFYING_DATA_ELEMENTS: [&registry::Item; 80] = [
  &registry::ACCESSION_NUMBER,
  &registry::ADMITTING_DIAGNOSES_CODE_SEQUENCE,
  &registry::ADMITTING_DIAGNOSES_DESCRIPTION,
  &registry::ALLERGIES,
  &registry::BRANCH_OF_SERVICE,
  &registry::CONTENT_SEQUENCE,
  &registry::COUNTRY_OF_RESIDENCE,
  &registry::ETHNIC_GROUP,
  &registry::INSTANCE_CREATOR_UID,
  &registry::INSTITUTION_ADDRESS,
  &registry::INSTITUTION_CODE_SEQUENCE,
  &registry::INSTITUTION_NAME,
  &registry::INSTITUTIONAL_DEPARTMENT_NAME,
  &registry::INSTITUTIONAL_DEPARTMENT_NAME,
  &registry::INSTITUTIONAL_DEPARTMENT_TYPE_CODE_SEQUENCE,
  &registry::INVENTORY_ACCESS_END_POINTS_SEQUENCE,
  &registry::MEDICAL_RECORD_LOCATOR,
  &registry::MILITARY_RANK,
  &registry::NAME_OF_PHYSICIANS_READING_STUDY,
  &registry::NETWORK_ID,
  &registry::OCCUPATION,
  &registry::OPERATOR_IDENTIFICATION_SEQUENCE,
  &registry::OPERATORS_NAME,
  &registry::OTHER_PATIENT_IDS_SEQUENCE,
  &registry::OTHER_PATIENT_IDS,
  &registry::OTHER_PATIENT_NAMES,
  &registry::PATIENT_AGE,
  &registry::PATIENT_ALTERNATIVE_CALENDAR,
  &registry::PATIENT_BIRTH_DATE_IN_ALTERNATIVE_CALENDAR,
  &registry::PATIENT_BIRTH_DATE,
  &registry::PATIENT_BIRTH_NAME,
  &registry::PATIENT_BIRTH_TIME,
  &registry::PATIENT_BREED_DESCRIPTION,
  &registry::PATIENT_COMMENTS,
  &registry::PATIENT_DEATH_DATE_IN_ALTERNATIVE_CALENDAR,
  &registry::PATIENT_ID,
  &registry::PATIENT_MOTHER_BIRTH_NAME,
  &registry::PATIENT_NAME,
  &registry::PATIENT_RELIGIOUS_PREFERENCE,
  &registry::PATIENT_SEX,
  &registry::PATIENT_SIZE,
  &registry::PATIENT_SPECIES_DESCRIPTION,
  &registry::PATIENT_STATE,
  &registry::PATIENT_TELEPHONE_NUMBERS,
  &registry::PATIENT_WEIGHT,
  &registry::PERFORMING_PHYSICIAN_IDENTIFICATION_SEQUENCE,
  &registry::PERFORMING_PHYSICIAN_NAME,
  &registry::PERSON_ADDRESS,
  &registry::PERSON_TELECOM_INFORMATION,
  &registry::PERSON_TELEPHONE_NUMBERS,
  &registry::PHYSICIANS_OF_RECORD_IDENTIFICATION_SEQUENCE,
  &registry::PHYSICIANS_OF_RECORD,
  &registry::PHYSICIANS_OF_RECORD,
  &registry::PHYSICIANS_READING_STUDY_IDENTIFICATION_SEQUENCE,
  &registry::PREGNANCY_STATUS,
  &registry::PROCEDURE_CODE_SEQUENCE,
  &registry::PROTOCOL_NAME,
  &registry::REFERENCED_FRAME_OF_REFERENCE_UID,
  &registry::REFERRING_PHYSICIAN_ADDRESS,
  &registry::REFERRING_PHYSICIAN_IDENTIFICATION_SEQUENCE,
  &registry::REFERRING_PHYSICIAN_NAME,
  &registry::REFERRING_PHYSICIAN_TELEPHONE_NUMBERS,
  &registry::REGION_OF_RESIDENCE,
  &registry::REQUEST_ATTRIBUTES_SEQUENCE,
  &registry::REQUESTING_SERVICE,
  &registry::RESPONSIBLE_ORGANIZATION,
  &registry::RESPONSIBLE_PERSON_ROLE,
  &registry::RESPONSIBLE_PERSON,
  &registry::SCHEDULED_PROCEDURE_STEP_ID,
  &registry::SERIES_DESCRIPTION_CODE_SEQUENCE,
  &registry::SERIES_DESCRIPTION,
  &registry::STATION_AE_TITLE,
  &registry::STATION_NAME,
  &registry::STATION_NAME,
  &registry::STORAGE_MEDIA_FILE_SET_UID,
  &registry::STUDY_ACCESS_END_POINTS_SEQUENCE,
  &registry::STUDY_DESCRIPTION,
  &registry::STUDY_ID,
  &registry::TIMEZONE_OFFSET_FROM_UTC,
  &registry::UID,
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
  fn anonymize(&mut self);
}

impl DataSetAnonymizeExtensions for DataSet {
  /// Anonymizes a data set by removing data elements that identify the patient,
  /// or potentially contribute to identification of the patient.
  ///
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
    assert_eq!(
      filter_tag(
        registry::SPECIFIC_CHARACTER_SET.tag,
        ValueRepresentation::CodeString,
      ),
      true
    );

    assert_eq!(
      filter_tag(registry::UID.tag, ValueRepresentation::UniqueIdentifier),
      false
    );

    assert_eq!(
      filter_tag(
        registry::STATION_AE_TITLE.tag,
        ValueRepresentation::ApplicationEntity,
      ),
      false
    );

    assert_eq!(
      filter_tag(
        DataElementTag::new(0x0009, 0x0002),
        ValueRepresentation::CodeString,
      ),
      false
    );

    assert_eq!(
      filter_tag(
        DataElementTag::new(0x0010, 0xABCD),
        ValueRepresentation::PersonName,
      ),
      false
    );
  }
}
