//! Converts between DICOM data sets and DICOM JSON.
//!
//! Ref: PS3.18 F.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{
  string::{String, ToString},
  vec::Vec,
};

mod internal;
mod json_config;
mod json_error;
mod transforms;

use dcmfx_core::{DataSet, DataSetPath};
use dcmfx_p10::{DataSetP10Extensions, IoError, IoWrite, P10Token};

pub use json_config::DicomJsonConfig;
pub use json_error::{JsonDeserializeError, JsonSerializeError};
pub use transforms::p10_json_transform::P10JsonTransform;

/// Adds functions to [`DataSet`] for converting to and from DICOM JSON.
///
pub trait DataSetJsonExtensions
where
  Self: Sized,
{
  /// Converts a data set to DICOM JSON, returning the JSON data as a string.
  ///
  fn to_json(
    &self,
    config: DicomJsonConfig,
  ) -> Result<String, JsonSerializeError>;

  /// Converts a data set to DICOM JSON, writing the JSON data to a stream.
  ///
  fn to_json_stream(
    &self,
    config: DicomJsonConfig,
    stream: &mut IoWrite,
  ) -> Result<(), JsonSerializeError>;

  /// Constructs a new data set from DICOM JSON data.
  ///
  fn from_json(json: &str) -> Result<Self, JsonDeserializeError>;
}

#[cfg(not(feature = "std"))]
struct Cursor {
  data: Vec<u8>,
}

#[cfg(not(feature = "std"))]
impl Cursor {
  fn into_inner(self) -> Vec<u8> {
    self.data
  }
}

#[cfg(not(feature = "std"))]
impl dcmfx_p10::Write for Cursor {
  fn write_all(&mut self, buf: &[u8]) -> Result<(), IoError> {
    self.data.extend_from_slice(buf);
    Ok(())
  }

  fn flush(&mut self) -> Result<(), IoError> {
    Ok(())
  }
}

impl DataSetJsonExtensions for DataSet {
  fn to_json(
    &self,
    config: DicomJsonConfig,
  ) -> Result<String, JsonSerializeError> {
    let buffer = Vec::with_capacity(64 * 1024);

    #[cfg(feature = "std")]
    let mut cursor = std::io::Cursor::new(buffer);

    #[cfg(not(feature = "std"))]
    let mut cursor = Cursor { data: buffer };

    self.to_json_stream(config, &mut cursor)?;

    Ok(String::from_utf8(cursor.into_inner()).unwrap())
  }

  fn to_json_stream(
    &self,
    config: DicomJsonConfig,
    stream: &mut IoWrite,
  ) -> Result<(), JsonSerializeError> {
    let mut json_transform = P10JsonTransform::new(config);
    let mut token_to_stream =
      |token: &P10Token| json_transform.add_token(token, stream);

    self.to_p10_tokens(&mut token_to_stream)?;

    stream.flush().map_err(JsonSerializeError::IOError)
  }

  fn from_json(json: &str) -> Result<Self, JsonDeserializeError> {
    let json_value = serde_json::from_str(json).map_err(|_| {
      JsonDeserializeError::JsonInvalid {
        details: "Input is not valid JSON".to_string(),
        path: DataSetPath::new(),
      }
    })?;

    internal::json_to_data_set::convert_json_to_data_set(
      json_value,
      &mut DataSetPath::new(),
    )
  }
}

#[cfg(test)]
mod tests {
  use dcmfx_core::{
    DataElementTag, DataElementValue, PersonNameComponents, RcByteSlice,
    StructuredPersonName, ValueRepresentation, dictionary, transfer_syntax,
  };

  use super::*;

  // Tests are run with encapsulated pixel data allowed in the DICOM JSON data
  const JSON_CONFIG: DicomJsonConfig = DicomJsonConfig {
    store_encapsulated_pixel_data: true,
    pretty_print: false,
  };

  #[test]
  fn data_set_to_json_test() {
    for (data_elements, expected_json) in test_data_sets() {
      let ds: DataSet = data_elements.into_iter().collect();

      assert_eq!(
        serde_json::from_str::<serde_json::Value>(
          &ds.to_json(JSON_CONFIG).unwrap()
        )
        .unwrap(),
        expected_json,
      );
    }
  }

  #[test]
  fn json_to_data_set_test() {
    for (data_elements, expected_json) in test_data_sets() {
      let ds: DataSet = data_elements.into_iter().collect();

      assert_eq!(DataSet::from_json(&expected_json.to_string()).unwrap(), ds);
    }
  }

  /// Returns pairs of data sets and their corresponding DICOM JSON string.
  /// These are used to test conversion both to and from DICOM JSON.
  ///
  fn test_data_sets()
  -> Vec<(Vec<(DataElementTag, DataElementValue)>, serde_json::Value)> {
    vec![
      (
        vec![
          (
            dictionary::MANUFACTURER.tag,
            DataElementValue::new_long_string(&["123"]).unwrap(),
          ),
          (
            dictionary::PATIENT_NAME.tag,
            DataElementValue::new_person_name(&[StructuredPersonName {
              alphabetic: Some(PersonNameComponents {
                last_name: "Jedi".to_string(),
                first_name: "Yoda".to_string(),
                middle_name: "".to_string(),
                prefix: "".to_string(),
                suffix: "".to_string(),
              }),
              ideographic: None,
              phonetic: None,
            }])
            .unwrap(),
          ),
          (
            dictionary::PATIENT_SEX.tag,
            DataElementValue::new_code_string(&["O"]).unwrap(),
          ),
        ],
        serde_json::json!({
          "00080070": { "vr": "LO", "Value": ["123"] },
          "00100010": { "vr": "PN", "Value": [{ "Alphabetic": "Jedi^Yoda" }] },
          "00100040": { "vr": "CS", "Value": ["O"] }
        }),
      ),
      (
        vec![(
          dictionary::MANUFACTURER.tag,
          DataElementValue::new_long_string(&[""]).unwrap(),
        )],
        serde_json::json!({ "00080070": { "vr": "LO" } }),
      ),
      (
        vec![(
          dictionary::MANUFACTURER.tag,
          DataElementValue::new_long_string(&["", ""]).unwrap(),
        )],
        serde_json::json!({ "00080070": { "vr": "LO", "Value": [null, null] } }),
      ),
      (
        vec![(
          dictionary::STAGE_NUMBER.tag,
          DataElementValue::new_integer_string(&[1]).unwrap(),
        )],
        serde_json::json!({ "00082122": { "vr": "IS", "Value": [1] } }),
      ),
      (
        vec![(
          dictionary::PATIENT_SIZE.tag,
          DataElementValue::new_decimal_string(&[1.2]).unwrap(),
        )],
        serde_json::json!({ "00101020": { "vr": "DS", "Value": [1.2] } }),
      ),
      (
        vec![(
          dictionary::PIXEL_DATA.tag,
          DataElementValue::new_other_byte_string(vec![1, 2]).unwrap(),
        )],
        serde_json::json!({ "7FE00010": { "vr": "OB", "InlineBinary": "AQI=" } }),
      ),
      (
        vec![(
          dictionary::PIXEL_DATA.tag,
          DataElementValue::new_other_word_string(vec![0x03, 0x04]).unwrap(),
        )],
        serde_json::json!({ "7FE00010": { "vr": "OW", "InlineBinary": "AwQ=" } }),
      ),
      (
        vec![
          (
            dictionary::TRANSFER_SYNTAX_UID.tag,
            DataElementValue::new_unique_identifier(&[
              transfer_syntax::ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN.uid
            ])
            .unwrap(),
          ),
          (
            dictionary::PIXEL_DATA.tag,
            DataElementValue::new_encapsulated_pixel_data(
              ValueRepresentation::OtherByteString,
              vec![RcByteSlice::empty(), vec![1, 2].into()],
            )
            .unwrap(),
          ),
        ],
        serde_json::json!({
          "00020010": { "vr": "UI", "Value": ["1.2.840.10008.1.2.1.98"] },
          "7FE00010": { "vr": "OB", "InlineBinary": "/v8A4AAAAAD+/wDgAgAAAAEC" }
        }),
      ),
      (
        vec![
          (
            dictionary::ENERGY_WEIGHTING_FACTOR.tag,
            DataElementValue::new_floating_point_single(&[f32::INFINITY])
              .unwrap(),
          ),
          (
            dictionary::DISTANCE_SOURCE_TO_ISOCENTER.tag,
            DataElementValue::new_floating_point_single(&[-f32::INFINITY])
              .unwrap(),
          ),
          (
            dictionary::DISTANCE_OBJECT_TO_TABLE_TOP.tag,
            DataElementValue::new_floating_point_single(&[f32::NAN]).unwrap(),
          ),
        ],
        serde_json::json!({
          "00189353": { "vr": "FL", "Value": ["Infinity"] },
          "00189402": { "vr": "FL", "Value": ["-Infinity"] },
          "00189403": { "vr": "FL", "Value": ["NaN"] }
        }),
      ),
      (
        vec![(
          dictionary::METADATA_SEQUENCE.tag,
          DataElementValue::new_sequence(vec![]),
        )],
        serde_json::json!({ "0008041D": { "vr": "SQ", "Value": [] } }),
      ),
    ]
  }
}
