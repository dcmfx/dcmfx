//// Anonymization of data sets by removing data elements that identify the
//// patient, or potentially contribute to identification of the patient.

import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_element_value
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/dictionary
import dcmfx_core/value_representation.{type ValueRepresentation}
import gleam/bool
import gleam/list
import gleam/result

const identifying_data_elements = [
  dictionary.accession_number,
  dictionary.admitting_diagnoses_code_sequence,
  dictionary.admitting_diagnoses_description,
  dictionary.instance_creator_uid,
  dictionary.institution_address,
  dictionary.institution_code_sequence,
  dictionary.institution_name,
  dictionary.institutional_department_name,
  dictionary.institutional_department_type_code_sequence,
  dictionary.inventory_access_end_points_sequence,
  dictionary.name_of_physicians_reading_study,
  dictionary.network_id,
  dictionary.operator_identification_sequence,
  dictionary.operators_name,
  dictionary.performing_physician_identification_sequence,
  dictionary.performing_physician_name,
  dictionary.person_address,
  dictionary.person_telecom_information,
  dictionary.person_telephone_numbers,
  dictionary.physicians_of_record_identification_sequence,
  dictionary.physicians_of_record,
  dictionary.physicians_of_record,
  dictionary.physicians_reading_study_identification_sequence,
  dictionary.pregnancy_status,
  dictionary.procedure_code_sequence,
  dictionary.protocol_name,
  dictionary.referenced_frame_of_reference_uid,
  dictionary.referring_physician_address,
  dictionary.referring_physician_identification_sequence,
  dictionary.referring_physician_name,
  dictionary.referring_physician_telephone_numbers,
  dictionary.request_attributes_sequence,
  dictionary.requesting_service,
  dictionary.scheduled_procedure_step_id,
  dictionary.series_description_code_sequence,
  dictionary.series_description,
  dictionary.station_name,
  dictionary.storage_media_file_set_uid,
  dictionary.study_access_end_points_sequence,
  dictionary.study_description,
  dictionary.study_id,
  dictionary.timezone_offset_from_utc,
  dictionary.uid,
]

/// Returns whether the given tag is allowed through the anonymization process.
///
pub fn filter_tag(tag: DataElementTag, vr: ValueRepresentation) -> Bool {
  // Strip all tags that specify an ApplicationEntity which could be identifying
  use <- bool.guard(
    vr == value_representation.ApplicationEntity
      || vr == value_representation.ApplicationEntity,
    False,
  )

  // Strip private tags
  use <- bool.guard(data_element_tag.is_private(tag), False)

  // Strip all patient tags
  use <- bool.guard(tag.group == 0x0010, False)

  // Strip all tags in the above list
  identifying_data_elements
  |> list.find(fn(item) { item.tag == tag })
  |> result.is_error
}

/// Anonymizes a data set by removing data elements that identify the patient,
/// or potentially contribute to identification of the patient.
///
pub fn anonymize_data_set(data_set: DataSet) -> DataSet {
  data_set.filter(data_set, fn(tag, value) {
    let vr = data_element_value.value_representation(value)

    filter_tag(tag, vr)
  })
}
