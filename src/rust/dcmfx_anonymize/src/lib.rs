//! Anonymization of data sets by removing data elements that identify the
//! patient, or potentially contribute to identification of the patient.

use dcmfx_core::{DataElementTag, DataSet, ValueRepresentation, dictionary};

const IDENTIFYING_DATA_ELEMENTS: [&dictionary::Item; 42] = [
  &dictionary::ACCESSION_NUMBER,
  &dictionary::ADMITTING_DIAGNOSES_CODE_SEQUENCE,
  &dictionary::ADMITTING_DIAGNOSES_DESCRIPTION,
  &dictionary::INSTANCE_CREATOR_UID,
  &dictionary::INSTITUTION_ADDRESS,
  &dictionary::INSTITUTION_CODE_SEQUENCE,
  &dictionary::INSTITUTION_NAME,
  &dictionary::INSTITUTIONAL_DEPARTMENT_NAME,
  &dictionary::INSTITUTIONAL_DEPARTMENT_TYPE_CODE_SEQUENCE,
  &dictionary::INVENTORY_ACCESS_END_POINTS_SEQUENCE,
  &dictionary::NAME_OF_PHYSICIANS_READING_STUDY,
  &dictionary::NETWORK_ID,
  &dictionary::OPERATOR_IDENTIFICATION_SEQUENCE,
  &dictionary::OPERATORS_NAME,
  &dictionary::PERFORMING_PHYSICIAN_IDENTIFICATION_SEQUENCE,
  &dictionary::PERFORMING_PHYSICIAN_NAME,
  &dictionary::PERSON_ADDRESS,
  &dictionary::PERSON_TELECOM_INFORMATION,
  &dictionary::PERSON_TELEPHONE_NUMBERS,
  &dictionary::PHYSICIANS_OF_RECORD_IDENTIFICATION_SEQUENCE,
  &dictionary::PHYSICIANS_OF_RECORD,
  &dictionary::PHYSICIANS_OF_RECORD,
  &dictionary::PHYSICIANS_READING_STUDY_IDENTIFICATION_SEQUENCE,
  &dictionary::PROCEDURE_CODE_SEQUENCE,
  &dictionary::PROTOCOL_NAME,
  &dictionary::REFERENCED_FRAME_OF_REFERENCE_UID,
  &dictionary::REFERRING_PHYSICIAN_ADDRESS,
  &dictionary::REFERRING_PHYSICIAN_IDENTIFICATION_SEQUENCE,
  &dictionary::REFERRING_PHYSICIAN_NAME,
  &dictionary::REFERRING_PHYSICIAN_TELEPHONE_NUMBERS,
  &dictionary::REQUEST_ATTRIBUTES_SEQUENCE,
  &dictionary::REQUESTING_SERVICE,
  &dictionary::SCHEDULED_PROCEDURE_STEP_ID,
  &dictionary::SERIES_DESCRIPTION_CODE_SEQUENCE,
  &dictionary::SERIES_DESCRIPTION,
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
    self.retain(|tag, value| filter_tag(tag, value.value_representation()));
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
