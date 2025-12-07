//! Creates DICOMDIR files that index a set of DICOM P10 files.
//!
//! See <https://dicom.nema.org/medical/dicom/current/output/chtml/part10/chapter_8.html>
//! for details on the structure of a DICOMDIR.

#[cfg(feature = "std")]
use std::collections::BTreeMap;

#[cfg(not(feature = "std"))]
use alloc::{
  boxed::Box, collections::BTreeMap, format, string::String, vec, vec::Vec,
};

use bytes::Bytes;

use dcmfx_core::{
  DataElementTag, DataElementValue, DataError, DataSet, DataSetPath,
  DcmfxError, dictionary, transfer_syntax,
};

use crate::{
  P10Error, P10Token, P10WriteConfig, P10WriteContext,
  p10_token::data_elements_to_tokens,
};

/// The SOP Class UID of the Media Storage Directory Storage SOP Class, which is
/// the SOP class of a DICOMDIR.
///
const MEDIA_STORAGE_DIRECTORY_STORAGE_SOP_CLASS_UID: &str =
  "1.2.840.10008.1.3.10";

/// The tags of the data elements that need to be provided in the data sets
/// passed to [`create`] in order to generate a complete DICOMDIR file.
///
pub const DICOMDIR_DATA_ELEMENT_TAGS: [DataElementTag; 18] = [
  dictionary::TRANSFER_SYNTAX_UID.tag,
  dictionary::SOP_CLASS_UID.tag,
  dictionary::SOP_INSTANCE_UID.tag,
  dictionary::STUDY_DATE.tag,
  dictionary::STUDY_TIME.tag,
  dictionary::ACCESSION_NUMBER.tag,
  dictionary::MODALITY.tag,
  dictionary::STUDY_DESCRIPTION.tag,
  dictionary::SERIES_DESCRIPTION.tag,
  dictionary::PATIENT_NAME.tag,
  dictionary::PATIENT_ID.tag,
  dictionary::PATIENT_BIRTH_DATE.tag,
  dictionary::PATIENT_SEX.tag,
  dictionary::STUDY_INSTANCE_UID.tag,
  dictionary::SERIES_INSTANCE_UID.tag,
  dictionary::STUDY_ID.tag,
  dictionary::SERIES_NUMBER.tag,
  dictionary::INSTANCE_NUMBER.tag,
];

/// An error that occurred in the process of creating a DICOMDIR.
///
#[derive(Clone, Debug, PartialEq)]
pub enum DicomDirError {
  /// An error that occurred when serializing the DICOMDIR to DICOM P10 data.
  P10Error(P10Error),

  /// An error that occurred when creating the data elements of the DICOMDIR,
  /// e.g. one of the supplied values is invalid.
  DataError(DataError),
}

impl core::fmt::Display for DicomDirError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::P10Error(e) => e.fmt(f),
      Self::DataError(e) => e.fmt(f),
    }
  }
}

impl DcmfxError for DicomDirError {
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    match self {
      Self::P10Error(e) => e.to_lines(task_description),
      Self::DataError(e) => e.to_lines(task_description),
    }
  }
}

impl From<P10Error> for DicomDirError {
  fn from(error: P10Error) -> Self {
    Self::P10Error(error)
  }
}

impl From<DataError> for DicomDirError {
  fn from(error: DataError) -> Self {
    Self::DataError(error)
  }
}

/// Creates a DICOMDIR that indexes the specified DICOM P10 files, returning the
/// raw bytes of the resulting DICOMDIR file.
///
/// Each DICOM file is specified as a path in the file-set together with a data
/// set containing the data elements that describe it. Only the data elements
/// used by directory records need to be present in the provided data sets.
///
/// Directory records containing a non-ASCII value, e.g. a patient name or a
/// study description, specify UTF-8 as their character set via
/// *'(0008,0005) Specific Character Set'*. Paths in the file-set must be ASCII
/// because they're stored in a data element that always uses the DICOM default
/// character repertoire, and an error is returned if this isn't the case.
///
/// The specified *'(0002,0003) Media Storage SOP Instance UID'* identifies the
/// DICOMDIR, and a new UID should be generated for each one that's created,
/// e.g. with [`dcmfx_core::data_element_value::unique_identifier::new()`].
///
/// The specified *'(0004,1130) File-set ID'* is a human-readable identifier for
/// the file-set. It's able to be empty, and must otherwise be no more than 16
/// characters from the DICOM default character repertoire.
///
/// The specified write config controls the values written into the DICOMDIR's
/// File Meta Information for *'(0002,0012) Implementation Class UID'* and
/// *'(0002,0013) Implementation Version Name'*.
///
/// Directory records are created for the patients, studies, series, and
/// instances of the specified DICOMs. Patients are identified by their
/// *'(0010,0020) Patient ID'*, studies by their *'(0020,000D) Study Instance
/// UID'*, and series by their *'(0020,000E) Series Instance UID'*. Records at
/// each level are ordered by the value that identifies them, and instance
/// records are ordered by their *'(0008,0018) SOP Instance UID'*.
///
/// The byte offsets that link the directory records together are only valid for
/// the exact bytes that are returned, which is why the DICOMDIR is returned in
/// its serialized form rather than as a [`DataSet`].
///
pub fn create(
  files: &[(String, DataSet)],
  media_storage_sop_instance_uid: &str,
  file_set_id: &str,
  config: Option<P10WriteConfig>,
) -> Result<Bytes, DicomDirError> {
  let (mut records, root_indices) = create_directory_records(files)?;

  let file_meta_information =
    create_file_meta_information(media_storage_sop_instance_uid)?;

  // Serialize the DICOMDIR with all of its byte offsets set to zero in order to
  // determine the byte offset that each directory record is written at
  let data_set = create_data_set(&records, &root_indices, &[], file_set_id)?;
  let (_, record_offsets) = data_set_to_bytes_with_record_offsets(
    &file_meta_information,
    &data_set,
    config.clone(),
  )?;

  // Put the byte offsets into the directory records and serialize again. All of
  // the byte offsets use the `UnsignedLong` VR, so changing their values
  // doesn't change the size of any directory record, meaning that the byte
  // offsets just measured remain correct.
  for record in records.iter_mut() {
    record.data_set.insert_int_value(
      &dictionary::OFFSET_OF_THE_NEXT_DIRECTORY_RECORD,
      &[record_offset(&record_offsets, record.next_sibling)],
    )?;

    record.data_set.insert_int_value(
      &dictionary::OFFSET_OF_REFERENCED_LOWER_LEVEL_DIRECTORY_ENTITY,
      &[record_offset(&record_offsets, record.first_child)],
    )?;
  }

  let data_set =
    create_data_set(&records, &root_indices, &record_offsets, file_set_id)?;

  let (bytes, _) = data_set_to_bytes_with_record_offsets(
    &file_meta_information,
    &data_set,
    config,
  )?;

  Ok(bytes.into())
}

/// A single directory record in a DICOMDIR, along with the indices of the other
/// directory records it references.
///
struct DirectoryRecord {
  data_set: DataSet,

  /// The next directory record at the same level of the hierarchy.
  next_sibling: Option<usize>,

  /// The first directory record on the level of the hierarchy below this one.
  first_child: Option<usize>,
}

/// Creates the directory records for the specified files. The records are
/// returned in the order they're written into the DICOMDIR, i.e. each record is
/// followed by the records on the levels below it. The indices of the records
/// making up the root directory entity are also returned.
///
fn create_directory_records(
  files: &[(String, DataSet)],
) -> Result<(Vec<DirectoryRecord>, Vec<usize>), DataError> {
  // Group the files by patient, then study, then series
  type SeriesGroup<'a> = BTreeMap<&'a str, Vec<&'a (String, DataSet)>>;
  type StudyGroup<'a> = BTreeMap<&'a str, SeriesGroup<'a>>;

  let mut patients: BTreeMap<&str, StudyGroup> = BTreeMap::new();

  for file in files.iter() {
    patients
      .entry(get_string(&file.1, dictionary::PATIENT_ID.tag)?)
      .or_default()
      .entry(get_string(&file.1, dictionary::STUDY_INSTANCE_UID.tag)?)
      .or_default()
      .entry(get_string(&file.1, dictionary::SERIES_INSTANCE_UID.tag)?)
      .or_default()
      .push(file);
  }

  let mut records = vec![];
  let mut patient_indices = vec![];

  for studies in patients.values() {
    let patient_index = records.len();
    patient_indices.push(patient_index);
    let mut study_indices = vec![];

    // The first file at each level of the hierarchy provides the values for
    // that level's directory record
    let patient_file =
      studies.values().next().unwrap().values().next().unwrap()[0];
    records.push(create_patient_record(&patient_file.1)?);

    for series in studies.values() {
      let study_index = records.len();
      study_indices.push(study_index);
      let mut series_indices = vec![];

      let study_file = series.values().next().unwrap()[0];
      records.push(create_study_record(&study_file.1)?);

      for files in series.values() {
        let series_index = records.len();
        series_indices.push(series_index);

        records.push(create_series_record(&files[0].1)?);

        // Instance records for a series are ordered by SOP Instance UID
        let mut files = files
          .iter()
          .map(|file| {
            Ok((
              get_string(&file.1, dictionary::SOP_INSTANCE_UID.tag)?,
              *file,
            ))
          })
          .collect::<Result<Vec<_>, DataError>>()?;
        files.sort_by_key(|(sop_instance_uid, _)| *sop_instance_uid);

        let instance_indices = (records.len()..(records.len() + files.len()))
          .collect::<Vec<usize>>();

        for (_, file) in files.iter() {
          records.push(create_instance_record(&file.0, &file.1)?);
        }

        link_child_records(&mut records, series_index, &instance_indices);
      }

      link_child_records(&mut records, study_index, &series_indices);
    }

    link_child_records(&mut records, patient_index, &study_indices);
  }

  link_sibling_records(&mut records, &patient_indices);

  for record in records.iter_mut() {
    add_specific_character_set(&mut record.data_set)?;
  }

  Ok((records, patient_indices))
}

/// Adds a *'(0008,0005) Specific Character Set'* data element specifying UTF-8
/// (ISO_IR 192) to a directory record when one of its values uses an extended
/// character set. Values in a [`DataSet`] are always UTF-8 encoded, so without
/// this such values would be read using the DICOM default character repertoire.
///
fn add_specific_character_set(record: &mut DataSet) -> Result<(), DataError> {
  let uses_extended_character_set = record.iter().any(|(_tag, value)| {
    value.value_representation().is_string()
      && value
        .bytes()
        .is_ok_and(|bytes| bytes.iter().any(|byte| *byte >= 0x80))
  });

  if uses_extended_character_set {
    record
      .insert_string_value(&dictionary::SPECIFIC_CHARACTER_SET, &["ISO_IR 192"])
  } else {
    Ok(())
  }
}

/// Sets the specified directory records as the children of a parent directory
/// record, and links them together as siblings.
///
fn link_child_records(
  records: &mut [DirectoryRecord],
  parent_index: usize,
  child_indices: &[usize],
) {
  records[parent_index].first_child = child_indices.first().cloned();

  link_sibling_records(records, child_indices);
}

/// Links the specified directory records together as siblings, i.e. each record
/// references the one that follows it.
///
fn link_sibling_records(records: &mut [DirectoryRecord], indices: &[usize]) {
  for indices in indices.windows(2) {
    records[indices[0]].next_sibling = Some(indices[1]);
  }
}

/// Returns the byte offset of the directory record at the specified index, or
/// zero when there is no such directory record.
///
fn record_offset(record_offsets: &[u64], index: Option<usize>) -> i64 {
  match index {
    Some(index) => *record_offsets.get(index).unwrap_or(&0) as i64,
    None => 0,
  }
}

/// Creates a new directory record of the specified type. Its byte offsets are
/// filled in once the size of the DICOMDIR's directory records is known.
///
fn create_directory_record(
  record_type: &str,
) -> Result<DirectoryRecord, DataError> {
  let mut data_set = DataSet::new();

  data_set
    .insert_int_value(&dictionary::OFFSET_OF_THE_NEXT_DIRECTORY_RECORD, &[0])?;
  data_set.insert_int_value(&dictionary::RECORD_IN_USE_FLAG, &[0xFFFF])?;
  data_set.insert_int_value(
    &dictionary::OFFSET_OF_REFERENCED_LOWER_LEVEL_DIRECTORY_ENTITY,
    &[0],
  )?;
  data_set
    .insert_string_value(&dictionary::DIRECTORY_RECORD_TYPE, &[record_type])?;

  Ok(DirectoryRecord {
    data_set,
    next_sibling: None,
    first_child: None,
  })
}

/// Creates a `PATIENT` directory record from a file's data set.
///
fn create_patient_record(file: &DataSet) -> Result<DirectoryRecord, DataError> {
  let mut record = create_directory_record("PATIENT")?;

  for item in [&dictionary::PATIENT_NAME, &dictionary::PATIENT_ID] {
    copy_required_data_element(file, item.tag, &mut record.data_set, item);
  }

  for item in [&dictionary::PATIENT_BIRTH_DATE, &dictionary::PATIENT_SEX] {
    copy_data_element(file, item.tag, &mut record.data_set, item);
  }

  Ok(record)
}

/// Creates a `STUDY` directory record from a file's data set.
///
fn create_study_record(file: &DataSet) -> Result<DirectoryRecord, DataError> {
  let mut record = create_directory_record("STUDY")?;

  for item in [
    &dictionary::STUDY_DATE,
    &dictionary::STUDY_TIME,
    &dictionary::ACCESSION_NUMBER,
    &dictionary::STUDY_DESCRIPTION,
    &dictionary::STUDY_INSTANCE_UID,
    &dictionary::STUDY_ID,
  ] {
    copy_required_data_element(file, item.tag, &mut record.data_set, item);
  }

  Ok(record)
}

/// Creates a `SERIES` directory record from a file's data set.
///
fn create_series_record(file: &DataSet) -> Result<DirectoryRecord, DataError> {
  let mut record = create_directory_record("SERIES")?;

  for item in [
    &dictionary::MODALITY,
    &dictionary::SERIES_INSTANCE_UID,
    &dictionary::SERIES_NUMBER,
  ] {
    copy_required_data_element(file, item.tag, &mut record.data_set, item);
  }

  copy_data_element(
    file,
    dictionary::SERIES_DESCRIPTION.tag,
    &mut record.data_set,
    &dictionary::SERIES_DESCRIPTION,
  );

  Ok(record)
}

/// Creates an `IMAGE` directory record from a file's path in the file-set and
/// its data set.
///
fn create_instance_record(
  path: &str,
  file: &DataSet,
) -> Result<DirectoryRecord, DataError> {
  let mut record = create_directory_record("IMAGE")?;

  // The referenced file ID has the `CodeString` VR, which always uses the DICOM
  // default character repertoire, i.e. it isn't affected by the record's
  // Specific Character Set, so paths are required to be ASCII
  if !path.is_ascii() {
    return Err(DataError::new_value_invalid(format!(
      "DICOM file path '{path}' contains non-ASCII characters"
    )));
  }

  // The referenced file ID holds the individual components of the path
  let referenced_file_id = path
    .split(['/', '\\'])
    .filter(|component| !component.is_empty())
    .collect::<Vec<&str>>();

  record.data_set.insert_string_value(
    &dictionary::REFERENCED_FILE_ID,
    &referenced_file_id,
  )?;

  for (tag, item) in [
    (
      dictionary::SOP_CLASS_UID.tag,
      &dictionary::REFERENCED_SOP_CLASS_UID_IN_FILE,
    ),
    (
      dictionary::SOP_INSTANCE_UID.tag,
      &dictionary::REFERENCED_SOP_INSTANCE_UID_IN_FILE,
    ),
    (
      dictionary::TRANSFER_SYNTAX_UID.tag,
      &dictionary::REFERENCED_TRANSFER_SYNTAX_UID_IN_FILE,
    ),
    (
      dictionary::INSTANCE_NUMBER.tag,
      &dictionary::INSTANCE_NUMBER,
    ),
  ] {
    copy_required_data_element(file, tag, &mut record.data_set, item);
  }

  Ok(record)
}

/// Copies a data element out of a file's data set into a directory record. Data
/// elements that aren't present in the file are omitted from the directory
/// record.
///
fn copy_data_element(
  file: &DataSet,
  tag: DataElementTag,
  record: &mut DataSet,
  item: &dictionary::Item,
) {
  if let Ok(value) = file.get_value(tag) {
    record.insert(item.tag, value.clone());
  }
}

/// Copies a data element that's a required directory record key out of a file's
/// data set into a directory record. Because such data elements are required to
/// be present, an empty value is inserted when the file doesn't contain one.
///
fn copy_required_data_element(
  file: &DataSet,
  tag: DataElementTag,
  record: &mut DataSet,
  item: &dictionary::Item,
) {
  let value = match file.get_value(tag) {
    Ok(value) => value.clone(),
    Err(_) => {
      DataElementValue::new_binary_unchecked(item.vrs[0], Bytes::default())
    }
  };

  record.insert(item.tag, value);
}

/// Returns a string value out of a data set, or an empty string when it isn't
/// present.
///
fn get_string(
  data_set: &DataSet,
  tag: DataElementTag,
) -> Result<&str, DataError> {
  match data_set.get_string(tag) {
    Ok(s) => Ok(s),
    Err(DataError::TagNotPresent { .. }) => Ok(""),
    Err(e) => Err(e),
  }
}

/// Creates the File Meta Information for a DICOMDIR.
///
fn create_file_meta_information(
  media_storage_sop_instance_uid: &str,
) -> Result<DataSet, DataError> {
  let mut data_set = DataSet::new();

  data_set.insert_string_value(
    &dictionary::MEDIA_STORAGE_SOP_CLASS_UID,
    &[MEDIA_STORAGE_DIRECTORY_STORAGE_SOP_CLASS_UID],
  )?;
  data_set.insert_string_value(
    &dictionary::MEDIA_STORAGE_SOP_INSTANCE_UID,
    &[media_storage_sop_instance_uid],
  )?;
  data_set.insert_string_value(
    &dictionary::TRANSFER_SYNTAX_UID,
    &[transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN.uid],
  )?;

  Ok(data_set)
}

/// Creates the main data set for a DICOMDIR containing the specified directory
/// records.
///
fn create_data_set(
  records: &[DirectoryRecord],
  root_indices: &[usize],
  record_offsets: &[u64],
  file_set_id: &str,
) -> Result<DataSet, DataError> {
  let mut data_set = DataSet::new();

  data_set.insert_string_value(&dictionary::FILE_SET_ID, &[file_set_id])?;
  data_set.insert_int_value(&dictionary::FILE_SET_CONSISTENCY_FLAG, &[0])?;

  data_set.insert_int_value(
    &dictionary::OFFSET_OF_THE_FIRST_DIRECTORY_RECORD_OF_THE_ROOT_DIRECTORY_ENTITY,
    &[record_offset(record_offsets, root_indices.first().cloned())],
  )?;
  data_set.insert_int_value(
    &dictionary::OFFSET_OF_THE_LAST_DIRECTORY_RECORD_OF_THE_ROOT_DIRECTORY_ENTITY,
    &[record_offset(record_offsets, root_indices.last().cloned())],
  )?;

  data_set.insert_sequence_value(
    &dictionary::DIRECTORY_RECORD_SEQUENCE,
    records
      .iter()
      .map(|record| record.data_set.clone())
      .collect(),
  )?;

  Ok(data_set)
}

/// Serializes a DICOMDIR's File Meta Information and main data set to DICOM P10
/// bytes, also returning the byte offset that each of its directory records was
/// written at.
///
fn data_set_to_bytes_with_record_offsets(
  file_meta_information: &DataSet,
  data_set: &DataSet,
  config: Option<P10WriteConfig>,
) -> Result<(Vec<u8>, Vec<u64>), P10Error> {
  let mut write_context = P10WriteContext::new(config);
  let mut bytes: Vec<u8> = vec![];
  let mut record_offsets = vec![];
  let mut sequence_depth = 0u32;

  let mut token_callback = |token: P10Token| -> Result<(), P10Error> {
    match token {
      P10Token::SequenceStart { .. } => sequence_depth += 1,
      P10Token::SequenceDelimiter { .. } => sequence_depth -= 1,

      // Directory records are the items of the Directory Record Sequence, and
      // the byte offset of a directory record is that of the start of its item
      // tag
      P10Token::SequenceItemStart { .. } if sequence_depth == 1 => {
        record_offsets.push(bytes.len() as u64)
      }

      _ => (),
    }

    write_context.write_token(&token)?;

    for chunk in write_context.read_bytes() {
      bytes.extend_from_slice(&chunk);
    }

    Ok(())
  };

  let token = P10Token::FilePreambleAndDICMPrefix {
    preamble: Box::new([0; 128]),
  };
  token_callback(token)?;

  let token = P10Token::FileMetaInformation {
    data_set: file_meta_information.clone(),
  };
  token_callback(token)?;

  data_elements_to_tokens(data_set, &DataSetPath::new(), &mut token_callback)?;
  token_callback(P10Token::End)?;

  Ok((bytes, record_offsets))
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Creates a file for inclusion in a DICOMDIR. The UIDs are constructed so
  /// that the resulting hierarchy is known.
  ///
  fn create_file(
    patient: usize,
    study: usize,
    series: usize,
    instance: usize,
  ) -> (String, DataSet) {
    let mut data_set = DataSet::new();

    data_set
      .insert_string_value(&dictionary::PATIENT_ID, &[&format!("P{patient}")])
      .unwrap();
    data_set
      .insert_string_value(
        &dictionary::STUDY_INSTANCE_UID,
        &[&format!("1.{patient}.{study}")],
      )
      .unwrap();
    data_set
      .insert_string_value(
        &dictionary::SERIES_INSTANCE_UID,
        &[&format!("1.{patient}.{study}.{series}")],
      )
      .unwrap();
    data_set
      .insert_string_value(
        &dictionary::SOP_INSTANCE_UID,
        &[&format!("1.{patient}.{study}.{series}.{instance}")],
      )
      .unwrap();

    let path = format!("IMAGES/{patient}{study}{series}{instance}");

    (path, data_set)
  }

  /// Returns the byte offsets of all sequence items in the specified DICOMDIR.
  /// A DICOMDIR contains only one sequence, and its items don't contain any
  /// nested sequences, so all item tags in the data are directory records.
  ///
  fn item_offsets(dicom_dir: &[u8]) -> Vec<u64> {
    dicom_dir
      .windows(4)
      .enumerate()
      .filter(|(_, window)| *window == [0xFE, 0xFF, 0x00, 0xE0])
      .map(|(offset, _)| offset as u64)
      .collect()
  }

  /// Returns the number of Specific Character Set data elements in the
  /// specified DICOMDIR, which is explicit VR little endian encoded.
  ///
  fn specific_character_set_count(dicom_dir: &[u8]) -> usize {
    dicom_dir
      .windows(6)
      .filter(|window| *window == [0x08, 0x00, 0x05, 0x00, b'C', b'S'])
      .count()
  }

  #[test]
  fn specific_character_set_test() {
    // A DICOMDIR containing only ASCII values doesn't specify a character set
    let file = create_file(0, 0, 0, 0);
    let dicom_dir = create(&[file], "1.2.3.4", "", None).unwrap();
    assert_eq!(specific_character_set_count(&dicom_dir), 0);

    // Only the records of the patient with a non-ASCII value specify a
    // character set. The second patient's four records don't have one, and nor
    // does the first patient's study, series, or instance record.
    let mut utf8_file = create_file(0, 0, 0, 0);
    utf8_file
      .1
      .insert_string_value(&dictionary::PATIENT_ID, &["Röntgen"])
      .unwrap();

    let dicom_dir =
      create(&[utf8_file, create_file(1, 0, 0, 0)], "1.2.3.4", "", None)
        .unwrap();
    assert_eq!(specific_character_set_count(&dicom_dir), 1);

    // Add a non-ASCII value to the patient, study, and series levels
    let mut file = create_file(0, 0, 0, 0);
    file
      .1
      .insert_string_value(&dictionary::PATIENT_ID, &["Röntgen"])
      .unwrap();
    file
      .1
      .insert_string_value(&dictionary::STUDY_DESCRIPTION, &["Röntgen"])
      .unwrap();
    file
      .1
      .insert_string_value(&dictionary::SERIES_DESCRIPTION, &["Röntgen"])
      .unwrap();

    let dicom_dir = create(&[file], "1.2.3.4", "", None).unwrap();

    // The PATIENT, STUDY, and SERIES records specify UTF-8 as their character
    // set, and the IMAGE record doesn't need to because none of its values are
    // able to use an extended character set
    assert_eq!(specific_character_set_count(&dicom_dir), 3);

    let data_set = crate::read_bytes(dicom_dir, None)
      .map_err(|(e, _)| e)
      .unwrap();
    let records = data_set
      .get_sequence_items(dictionary::DIRECTORY_RECORD_SEQUENCE.tag)
      .unwrap();

    for (index, record_type) in
      ["PATIENT", "STUDY", "SERIES", "IMAGE"].iter().enumerate()
    {
      assert_eq!(
        records[index].get_string(dictionary::DIRECTORY_RECORD_TYPE.tag),
        Ok(*record_type)
      );

      assert_eq!(
        records[index]
          .get_string(dictionary::SPECIFIC_CHARACTER_SET.tag)
          .ok(),
        if *record_type == "IMAGE" {
          None
        } else {
          Some("ISO_IR 192")
        }
      );
    }

    assert_eq!(
      records[0].get_string(dictionary::PATIENT_ID.tag),
      Ok("Röntgen")
    );
  }

  #[test]
  fn non_ascii_path_test() {
    let mut file = create_file(0, 0, 0, 0);
    file.0 = "IMAGES/Röntgen".to_string();

    assert_eq!(
      create(&[file], "1.2.3.4", "", None),
      Err(DicomDirError::DataError(DataError::new_value_invalid(
        "DICOM file path 'IMAGES/Röntgen' contains non-ASCII characters"
          .to_string()
      )))
    );
  }

  #[test]
  fn create_dicom_dir_test() {
    // Create two patients, each with two studies, each with two series, each
    // containing two instances
    let mut files = vec![];
    for patient in 0..2 {
      for study in 0..2 {
        for series in 0..2 {
          for instance in 0..2 {
            files.push(create_file(patient, study, series, instance));
          }
        }
      }
    }

    let dicom_dir = create(&files, "1.2.3.4", "DCMFX_TEST", None).unwrap();

    // The Specific Character Set data element isn't part of the Basic Directory
    // IOD, and none of these values need one
    assert_eq!(specific_character_set_count(&dicom_dir), 0);

    // Check the DICOMDIR is valid DICOM P10 data with the expected File Meta
    // Information
    let data_set = crate::read_bytes(dicom_dir.clone(), None)
      .map_err(|(e, _)| e)
      .unwrap();
    assert_eq!(
      data_set.get_string(dictionary::MEDIA_STORAGE_SOP_CLASS_UID.tag),
      Ok(MEDIA_STORAGE_DIRECTORY_STORAGE_SOP_CLASS_UID)
    );
    assert_eq!(
      data_set.get_string(dictionary::MEDIA_STORAGE_SOP_INSTANCE_UID.tag),
      Ok("1.2.3.4")
    );
    assert_eq!(
      data_set.get_string(dictionary::TRANSFER_SYNTAX_UID.tag),
      Ok(transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN.uid)
    );
    assert_eq!(
      data_set.get_string(dictionary::FILE_SET_ID.tag),
      Ok("DCMFX_TEST")
    );

    // Check the directory records are in the expected order. Each patient has
    // two studies, each study has two series, and each series has two
    // instances.
    let records = data_set
      .get_sequence_items(dictionary::DIRECTORY_RECORD_SEQUENCE.tag)
      .unwrap();

    let record_types = records
      .iter()
      .map(|record| {
        record
          .get_string(dictionary::DIRECTORY_RECORD_TYPE.tag)
          .unwrap()
      })
      .collect::<Vec<&str>>();

    let series_records =
      ["SERIES", "IMAGE", "IMAGE", "SERIES", "IMAGE", "IMAGE"];
    let study_records = [&["STUDY"][..], &series_records[..]].concat();
    let patient_records =
      [&["PATIENT"][..], &study_records[..], &study_records[..]].concat();

    assert_eq!(
      record_types,
      [&patient_records[..], &patient_records[..]].concat()
    );

    // Check that every directory record was found by the raw scan for item tags
    let item_offsets = item_offsets(&dicom_dir);
    assert_eq!(item_offsets.len(), records.len());

    // Maps a byte offset to the index of the directory record at that offset
    let record_index = |offset: u64| -> Option<usize> {
      if offset == 0 {
        None
      } else {
        Some(item_offsets.iter().position(|o| *o == offset).unwrap())
      }
    };

    let next_sibling = |index: usize| -> Option<usize> {
      record_index(
        records[index]
          .get_int(dictionary::OFFSET_OF_THE_NEXT_DIRECTORY_RECORD.tag)
          .unwrap(),
      )
    };

    let first_child = |index: usize| -> Option<usize> {
      record_index(
        records[index]
          .get_int(
            dictionary::OFFSET_OF_REFERENCED_LOWER_LEVEL_DIRECTORY_ENTITY.tag,
          )
          .unwrap(),
      )
    };

    // The root directory entity is the two patient records
    assert_eq!(
      record_index(
        data_set
          .get_int(
            dictionary::OFFSET_OF_THE_FIRST_DIRECTORY_RECORD_OF_THE_ROOT_DIRECTORY_ENTITY.tag
          )
          .unwrap()
      ),
      Some(0)
    );
    assert_eq!(
      record_index(
        data_set
          .get_int(
            dictionary::OFFSET_OF_THE_LAST_DIRECTORY_RECORD_OF_THE_ROOT_DIRECTORY_ENTITY.tag
          )
          .unwrap()
      ),
      Some(15)
    );

    // The first patient's record is followed by the second patient's record,
    // and its lower level directory entity is its first study's record
    assert_eq!(next_sibling(0), Some(15));
    assert_eq!(first_child(0), Some(1));
    assert_eq!(next_sibling(15), None);

    // The first study's record is followed by the second study's record, and
    // its lower level directory entity is its first series' record
    assert_eq!(next_sibling(1), Some(8));
    assert_eq!(first_child(1), Some(2));
    assert_eq!(next_sibling(8), None);

    // The first series' record is followed by the second series' record, and
    // its lower level directory entity is its first instance's record
    assert_eq!(next_sibling(2), Some(5));
    assert_eq!(first_child(2), Some(3));
    assert_eq!(next_sibling(5), None);

    // The instance records have no lower level directory entity
    assert_eq!(next_sibling(3), Some(4));
    assert_eq!(first_child(3), None);
    assert_eq!(next_sibling(4), None);

    // Check the details of an instance record
    assert_eq!(
      records[4].get_strings(dictionary::REFERENCED_FILE_ID.tag),
      Ok(vec!["IMAGES", "0001"])
    );
    assert_eq!(
      records[4]
        .get_string(dictionary::REFERENCED_SOP_INSTANCE_UID_IN_FILE.tag),
      Ok("1.0.0.0.1")
    );
  }
}
