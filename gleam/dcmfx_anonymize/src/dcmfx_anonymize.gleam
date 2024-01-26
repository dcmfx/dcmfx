//// Anonymization of data sets by removing data elements that identify the
//// patient, or potentially contribute to identification of the patient.

import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_element_value
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/registry
import dcmfx_core/value_representation.{type ValueRepresentation}
import gleam/bool
import gleam/list
import gleam/result

/// A list of data elements that identify the patient, or potentially contribute
/// to identification of the patient, and that should be removed during
/// anonymization.
///
/// Note that this list does not include the 'SOP Instance UID', 'Series
/// Instance UID', or 'Study Instance UID' data elements.
///
pub const identifying_data_elements = [
  registry.accession_number, registry.admitting_diagnoses_code_sequence,
  registry.admitting_diagnoses_description, registry.allergies,
  registry.branch_of_service, registry.content_sequence,
  registry.country_of_residence, registry.ethnic_group,
  registry.instance_creator_uid, registry.institution_address,
  registry.institution_code_sequence, registry.institution_name,
  registry.institutional_department_name, registry.institutional_department_name,
  registry.institutional_department_type_code_sequence,
  registry.inventory_access_end_points_sequence, registry.medical_record_locator,
  registry.military_rank, registry.name_of_physicians_reading_study,
  registry.network_id, registry.occupation,
  registry.operator_identification_sequence, registry.operators_name,
  registry.other_patient_ids_sequence, registry.other_patient_ids,
  registry.other_patient_names, registry.patient_age,
  registry.patient_alternative_calendar,
  registry.patient_birth_date_in_alternative_calendar,
  registry.patient_birth_date, registry.patient_birth_name,
  registry.patient_birth_time, registry.patient_breed_description,
  registry.patient_comments, registry.patient_death_date_in_alternative_calendar,
  registry.patient_id, registry.patient_mother_birth_name, registry.patient_name,
  registry.patient_religious_preference, registry.patient_sex,
  registry.patient_size, registry.patient_species_description,
  registry.patient_state, registry.patient_telephone_numbers,
  registry.patient_weight, registry.performing_physician_identification_sequence,
  registry.performing_physician_name, registry.person_address,
  registry.person_telecom_information, registry.person_telephone_numbers,
  registry.physicians_of_record_identification_sequence,
  registry.physicians_of_record, registry.physicians_of_record,
  registry.physicians_reading_study_identification_sequence,
  registry.pregnancy_status, registry.procedure_code_sequence,
  registry.protocol_name, registry.referenced_frame_of_reference_uid,
  registry.referring_physician_address,
  registry.referring_physician_identification_sequence,
  registry.referring_physician_name,
  registry.referring_physician_telephone_numbers, registry.region_of_residence,
  registry.request_attributes_sequence, registry.requesting_service,
  registry.responsible_organization, registry.responsible_person_role,
  registry.responsible_person, registry.scheduled_procedure_step_id,
  registry.series_description_code_sequence, registry.series_description,
  registry.station_ae_title, registry.station_name, registry.station_name,
  registry.storage_media_file_set_uid, registry.study_access_end_points_sequence,
  registry.study_description, registry.study_id,
  registry.timezone_offset_from_utc, registry.uid,
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
