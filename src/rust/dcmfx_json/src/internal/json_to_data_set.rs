use std::rc::Rc;

use base64::prelude::*;
use byteorder::ByteOrder;

use dcmfx_core::{
  dictionary, DataElementTag, DataElementValue, DataSet, DataSetPath,
  TransferSyntax, ValueRepresentation,
};

use crate::json_error::JsonDeserializeError;

/// Converts DICOM JSON into a data set. This is used to read the root data set
/// and also recursively when reading sequences.
///
pub fn convert_json_to_data_set(
  data_set_json: serde_json::Value,
  path: &mut DataSetPath,
) -> Result<DataSet, JsonDeserializeError> {
  let raw_map = if let serde_json::Value::Object(map) = data_set_json {
    map
  } else {
    return Err(JsonDeserializeError::JsonInvalid {
      details: "Data set is not an object".to_string(),
      path: path.clone(),
    });
  };

  let mut data_set = DataSet::new();
  let mut transfer_syntax: Option<&'static TransferSyntax> = None;

  for (raw_tag, raw_value) in raw_map.into_iter() {
    // Parse the data element tag
    let tag = match DataElementTag::from_hex_string(&raw_tag) {
      Ok(tag) => tag,
      Err(()) => {
        return Err(JsonDeserializeError::JsonInvalid {
          details: format!("Invalid data set tag: {}", raw_tag),
          path: path.clone(),
        })
      }
    };

    path.add_data_element(tag).unwrap();

    // Parse the data element value
    let value =
      convert_json_to_data_element(raw_value, tag, &transfer_syntax, path)?;

    // Add data element to the final data set
    data_set.insert(tag, value);

    // Look up the transfer syntax if this is the relevant tag
    if tag == dictionary::TRANSFER_SYNTAX_UID.tag {
      if let Ok(ts) = data_set.get_transfer_syntax() {
        transfer_syntax = Some(ts);
      }
    }

    path.pop().unwrap();
  }

  Ok(data_set)
}

/// Converts a single DICOM JSON data element value to a native data element
/// value.
///
fn convert_json_to_data_element(
  json: serde_json::Value,
  tag: DataElementTag,
  transfer_syntax: &Option<&'static TransferSyntax>,
  path: &mut DataSetPath,
) -> Result<DataElementValue, JsonDeserializeError> {
  let mut raw_value = if let serde_json::Value::Object(map) = json {
    map
  } else {
    return Err(JsonDeserializeError::JsonInvalid {
      details: "Data element is not an object".to_string(),
      path: path.clone(),
    });
  };

  // Read the VR for this value
  let vr = read_dicom_json_vr(&raw_value, path)?;

  // To read the data element value, first look for a "Value" property, then
  // look for an "InlineBinary" property, then finally look for a "BulkDataURI"
  // property (which is not supported and generates an error)
  if let Some(value) = raw_value.remove("Value") {
    read_dicom_json_primitive_value(tag, vr, value, path)
  } else if let Some(inline_binary) = raw_value.remove("InlineBinary") {
    read_dicom_json_inline_binary_value(
      inline_binary,
      tag,
      vr,
      transfer_syntax,
      path,
    )
  } else if raw_value.contains_key("BulkDataURI") {
    Err(JsonDeserializeError::JsonInvalid {
      details: "BulkDataURI values are not supported".to_string(),
      path: path.clone(),
    })
  } else {
    // No value is present, so fall back to an empty value
    if vr == ValueRepresentation::Sequence {
      Ok(DataElementValue::new_sequence(vec![]))
    } else {
      Ok(DataElementValue::new_binary_unchecked(vr, Rc::new(vec![])))
    }
  }
}

/// Reads a native value representation from a DICOM JSON "vr" property.
///
fn read_dicom_json_vr(
  raw_value: &serde_json::Map<String, serde_json::Value>,
  path: &mut DataSetPath,
) -> Result<ValueRepresentation, JsonDeserializeError> {
  // Read the VR
  let raw_vr = if let Some(raw_vr) = raw_value.get("vr") {
    raw_vr
  } else {
    return Err(JsonDeserializeError::JsonInvalid {
      details: "VR is missing".to_string(),
      path: path.clone(),
    });
  };

  // Get the VR string value
  let vr_string = if let Some(s) = raw_vr.as_str() {
    s
  } else {
    return Err(JsonDeserializeError::JsonInvalid {
      details: "VR is not a string".to_string(),
      path: path.clone(),
    });
  };

  // Convert to a native VR
  if let Ok(vr) = ValueRepresentation::from_bytes(vr_string.as_bytes()) {
    Ok(vr)
  } else {
    Err(JsonDeserializeError::JsonInvalid {
      details: format!("VR is invalid: {}", vr_string),
      path: path.clone(),
    })
  }
}

/// Reads a data element value from a DICOM JSON "Value" property.
///
fn read_dicom_json_primitive_value(
  tag: DataElementTag,
  vr: ValueRepresentation,
  value: serde_json::Value,
  path: &mut DataSetPath,
) -> Result<DataElementValue, JsonDeserializeError> {
  match vr {
    ValueRepresentation::AgeString
    | ValueRepresentation::ApplicationEntity
    | ValueRepresentation::CodeString
    | ValueRepresentation::Date
    | ValueRepresentation::DateTime
    | ValueRepresentation::LongString
    | ValueRepresentation::LongText
    | ValueRepresentation::ShortString
    | ValueRepresentation::ShortText
    | ValueRepresentation::Time
    | ValueRepresentation::UnlimitedCharacters
    | ValueRepresentation::UnlimitedText
    | ValueRepresentation::UniqueIdentifier
    | ValueRepresentation::UniversalResourceIdentifier => {
      let strings = if let Ok(strings) =
        serde_json::from_value::<Vec<Option<String>>>(value)
      {
        strings
      } else {
        return Err(JsonDeserializeError::JsonInvalid {
          details: "String value is invalid".to_string(),
          path: path.clone(),
        });
      };

      let mut bytes = Vec::with_capacity(
        strings
          .iter()
          .map(|s| s.as_ref().map(|s| s.len()).unwrap_or(0) + 1)
          .sum(),
      );

      for (i, s) in strings.iter().enumerate() {
        if let Some(s) = s {
          bytes.extend_from_slice(s.as_bytes());
        }

        if i + 1 != strings.len() {
          bytes.push(b'\\');
        }
      }

      vr.pad_bytes_to_even_length(&mut bytes);

      Ok(DataElementValue::new_binary_unchecked(vr, Rc::new(bytes)))
    }

    ValueRepresentation::DecimalString => {
      if let Ok(floats) = serde_json::from_value::<Vec<f64>>(value) {
        let bytes =
          dcmfx_core::data_element_value::decimal_string::to_bytes(&floats);

        Ok(DataElementValue::new_binary_unchecked(vr, Rc::new(bytes)))
      } else {
        Err(JsonDeserializeError::JsonInvalid {
          details: "DecimalString value is invalid".to_string(),
          path: path.clone(),
        })
      }
    }

    ValueRepresentation::IntegerString => {
      if let Ok(ints) = serde_json::from_value::<Vec<i32>>(value) {
        let bytes =
          dcmfx_core::data_element_value::integer_string::to_bytes(&ints);

        Ok(DataElementValue::new_binary_unchecked(vr, Rc::new(bytes)))
      } else {
        Err(JsonDeserializeError::JsonInvalid {
          details: "IntegerString value is invalid".to_string(),
          path: path.clone(),
        })
      }
    }

    ValueRepresentation::PersonName => {
      read_dicom_json_person_name_value(value, path)
    }

    ValueRepresentation::SignedLong => {
      if let Ok(ints) = serde_json::from_value::<Vec<i32>>(value) {
        let mut bytes = vec![0u8; ints.len() * 4];
        byteorder::LittleEndian::write_i32_into(&ints, &mut bytes);

        Ok(DataElementValue::new_binary_unchecked(vr, Rc::new(bytes)))
      } else {
        Err(JsonDeserializeError::JsonInvalid {
          details: "SignedLong value is invalid".to_string(),
          path: path.clone(),
        })
      }
    }

    ValueRepresentation::SignedShort | ValueRepresentation::UnsignedShort => {
      let ints = if let Ok(value) = serde_json::from_value::<Vec<i64>>(value) {
        value
      } else {
        return Err(JsonDeserializeError::JsonInvalid {
          details: "Short value is invalid".to_string(),
          path: path.clone(),
        });
      };

      if dictionary::is_lut_descriptor_tag(tag) && ints.len() == 3 {
        let entry_count = ints[0];
        let first_input_value = ints[1];
        let bits_per_entry = ints[2];

        let mut bytes = Vec::with_capacity(6);
        bytes.extend_from_slice(&(entry_count as u16).to_le_bytes());
        if vr == ValueRepresentation::SignedShort {
          bytes.extend_from_slice(&(first_input_value as i16).to_le_bytes());
        } else {
          bytes.extend_from_slice(&(first_input_value as u16).to_le_bytes());
        }
        bytes.extend_from_slice(&(bits_per_entry as u16).to_le_bytes());

        Ok(DataElementValue::new_lookup_table_descriptor_unchecked(
          vr,
          Rc::new(bytes),
        ))
      } else {
        let mut bytes = Vec::with_capacity(ints.len() * 2);

        if vr == ValueRepresentation::SignedShort {
          for i in ints {
            if i >= i16::MIN as i64 && i <= i16::MAX as i64 {
              bytes.extend_from_slice(&(i as i16).to_le_bytes());
            } else {
              return Err(JsonDeserializeError::JsonInvalid {
                details: "SignedShort value is out of range".to_string(),
                path: path.clone(),
              });
            }
          }
        } else {
          for i in ints {
            if i >= u16::MIN as i64 && i <= u16::MAX as i64 {
              bytes.extend_from_slice(&(i as u16).to_le_bytes());
            } else {
              return Err(JsonDeserializeError::JsonInvalid {
                details: "UnsignedShort value is out of range".to_string(),
                path: path.clone(),
              });
            }
          }
        };

        Ok(DataElementValue::new_binary_unchecked(vr, Rc::new(bytes)))
      }
    }

    ValueRepresentation::SignedVeryLong
    | ValueRepresentation::UnsignedVeryLong => {
      let ints: Vec<_> = if let Ok(value) = serde_json::from_value(value) {
        value
      } else {
        return Err(JsonDeserializeError::JsonInvalid {
          details: "Very long value is invalid".to_string(),
          path: path.clone(),
        });
      };

      let mut bytes = Vec::with_capacity(ints.len() * 8);

      let append_int = if vr == ValueRepresentation::SignedVeryLong {
        |i: i128,
         bytes: &mut Vec<u8>,
         path: &DataSetPath|
         -> Result<(), JsonDeserializeError> {
          if i >= i64::MIN as i128 && i <= i64::MAX as i128 {
            bytes.extend_from_slice(&(i as i64).to_le_bytes());
            Ok(())
          } else {
            Err(JsonDeserializeError::JsonInvalid {
              details: "SignedVeryLong value is out of range".to_string(),
              path: path.clone(),
            })
          }
        }
      } else {
        |i: i128,
         bytes: &mut Vec<u8>,
         path: &DataSetPath|
         -> Result<(), JsonDeserializeError> {
          if i >= u64::MIN as i128 && i <= u64::MAX as i128 {
            bytes.extend_from_slice(&(i as u64).to_le_bytes());
            Ok(())
          } else {
            Err(JsonDeserializeError::JsonInvalid {
              details: "UnsignedVeryLong value is out of range".to_string(),
              path: path.clone(),
            })
          }
        }
      };

      // Allow both int and string values. The latter is used when the integer
      // is too large to be represented by a JavaScript number.
      for int in ints {
        if let serde_json::Value::Number(n) = int {
          if let Some(i) = n.as_i64() {
            append_int(i as i128, &mut bytes, path)?;
          } else if let Some(i) = n.as_u64() {
            append_int(i as i128, &mut bytes, path)?;
          } else {
            return Err(JsonDeserializeError::JsonInvalid {
              details: "Very long value is invalid".to_string(),
              path: path.clone(),
            });
          }
        } else if let serde_json::Value::String(s) = int {
          if let Ok(i) = s.parse::<i64>() {
            append_int(i as i128, &mut bytes, path)?;
          } else if let Ok(i) = s.parse::<u64>() {
            append_int(i as i128, &mut bytes, path)?;
          } else {
            return Err(JsonDeserializeError::JsonInvalid {
              details: "Very long value is invalid".to_string(),
              path: path.clone(),
            });
          }
        }
      }

      Ok(DataElementValue::new_binary_unchecked(vr, Rc::new(bytes)))
    }

    ValueRepresentation::UnsignedLong => {
      if let Ok(ints) = serde_json::from_value::<Vec<u32>>(value) {
        let mut bytes = vec![0u8; ints.len() * 4];
        byteorder::LittleEndian::write_u32_into(&ints, &mut bytes);

        Ok(DataElementValue::new_binary_unchecked(vr, Rc::new(bytes)))
      } else {
        Err(JsonDeserializeError::JsonInvalid {
          details: "UnsignedLong value is invalid".to_string(),
          path: path.clone(),
        })
      }
    }

    ValueRepresentation::FloatingPointDouble => {
      let floats =
        read_dicom_json_float_array::<f64>(&value).map_err(|_| {
          JsonDeserializeError::JsonInvalid {
            details: "FloatingPointDouble value is invalid".to_string(),
            path: path.clone(),
          }
        })?;

      let mut bytes = vec![0u8; floats.len() * 8];
      byteorder::LittleEndian::write_f64_into(&floats, &mut bytes);

      Ok(DataElementValue::new_binary_unchecked(vr, Rc::new(bytes)))
    }

    ValueRepresentation::FloatingPointSingle => {
      let floats =
        read_dicom_json_float_array::<f32>(&value).map_err(|_| {
          JsonDeserializeError::JsonInvalid {
            details: "FloatingPointSingle value is invalid".to_string(),
            path: path.clone(),
          }
        })?;

      let mut bytes = vec![0u8; floats.len() * 4];
      byteorder::LittleEndian::write_f32_into(&floats, &mut bytes);

      Ok(DataElementValue::new_binary_unchecked(vr, Rc::new(bytes)))
    }

    ValueRepresentation::AttributeTag => {
      let tags = if let Ok(tags) = serde_json::from_value::<Vec<String>>(value)
      {
        tags
      } else {
        return Err(JsonDeserializeError::JsonInvalid {
          details: "AttributeTag value is invalid".to_string(),
          path: path.clone(),
        });
      };

      let mut bytes = Vec::with_capacity(tags.len() * 4);

      for tag in tags {
        if let Ok(tag) = DataElementTag::from_hex_string(&tag) {
          bytes.extend_from_slice(&tag.group.to_le_bytes());
          bytes.extend_from_slice(&tag.element.to_le_bytes());
        } else {
          return Err(JsonDeserializeError::JsonInvalid {
            details: "AttributeTag value is invalid".to_string(),
            path: path.clone(),
          });
        }
      }

      Ok(DataElementValue::new_binary_unchecked(vr, Rc::new(bytes)))
    }

    ValueRepresentation::Sequence => {
      let raw_items = if let Ok(items) = serde_json::from_value::<Vec<_>>(value)
      {
        items
      } else {
        return Err(JsonDeserializeError::JsonInvalid {
          details: "Sequence value is invalid".to_string(),
          path: path.clone(),
        });
      };

      let mut final_items = vec![];

      for (i, item) in raw_items.into_iter().enumerate() {
        path.add_sequence_item(i).unwrap();
        final_items.push(convert_json_to_data_set(item, path)?);
        path.pop().unwrap();
      }

      Ok(DataElementValue::new_sequence(final_items))
    }

    _ => Err(JsonDeserializeError::JsonInvalid {
      details: format!("Invalid 'Value' data element with VR '{}'", vr),
      path: path.clone(),
    }),
  }
}

fn read_dicom_json_float_array<
  T: num_traits::Float + num_traits::FromPrimitive,
>(
  value: &serde_json::Value,
) -> Result<Vec<T>, ()> {
  let array = if let Some(array) = value.as_array() {
    array
  } else {
    return Err(());
  };

  let mut floats: Vec<T> = Vec::with_capacity(array.len());

  for value in array {
    let float = match value.as_f64() {
      Some(f) => f,
      None => match value {
        serde_json::Value::String(s) if s == "NaN" => f64::NAN,
        serde_json::Value::String(s) if s == "Infinity" => f64::INFINITY,
        serde_json::Value::String(s) if s == "-Infinity" => f64::NEG_INFINITY,
        _ => return Err(()),
      },
    };

    floats.push(T::from_f64(float).unwrap());
  }

  Ok(floats)
}

#[derive(serde::Deserialize)]
struct PersonNameVariants {
  #[serde(rename = "Alphabetic")]
  alphabetic: Option<String>,

  #[serde(rename = "Ideographic")]
  ideographic: Option<String>,

  #[serde(rename = "Phonetic")]
  phonetic: Option<String>,
}

/// Reads a data element value from a DICOM JSON person name.
///
fn read_dicom_json_person_name_value(
  value: serde_json::Value,
  path: &mut DataSetPath,
) -> Result<DataElementValue, JsonDeserializeError> {
  let person_name_variants: Vec<PersonNameVariants> =
    serde_json::from_value(value).map_err(|_| {
      JsonDeserializeError::JsonInvalid {
        details: "PersonName value is invalid".to_string(),
        path: path.clone(),
      }
    })?;

  let mut bytes = person_name_variants
    .into_iter()
    .map(|raw_person_name| {
      [
        raw_person_name.alphabetic.unwrap_or_default(),
        raw_person_name.ideographic.unwrap_or_default(),
        raw_person_name.phonetic.unwrap_or_default(),
      ]
      .join("=")
      .trim_end_matches('=')
      .to_string()
    })
    .collect::<Vec<String>>()
    .join("\\")
    .into_bytes();

  if bytes.len() % 2 == 1 {
    bytes.push(0x20);
  }

  Ok(DataElementValue::new_binary_unchecked(
    ValueRepresentation::PersonName,
    Rc::new(bytes),
  ))
}

/// Reads a data element value from a DICOM JSON "InlineBinary" property.
///
fn read_dicom_json_inline_binary_value(
  inline_binary: serde_json::Value,
  tag: DataElementTag,
  vr: ValueRepresentation,
  transfer_syntax: &Option<&'static TransferSyntax>,
  path: &mut DataSetPath,
) -> Result<DataElementValue, JsonDeserializeError> {
  let inline_binary = if let serde_json::Value::String(s) = inline_binary {
    s
  } else {
    return Err(JsonDeserializeError::JsonInvalid {
      details: "InlineBinary is not a string".to_string(),
      path: path.clone(),
    });
  };

  let bytes = if let Ok(data) = BASE64_STANDARD.decode(inline_binary) {
    data
  } else {
    return Err(JsonDeserializeError::JsonInvalid {
      details: "InlineBinary is not valid Base64".to_string(),
      path: path.clone(),
    });
  };

  // Look at the tag and the transfer syntax to see if this inline binary holds
  // encapsulated pixel data.
  if tag == dictionary::PIXEL_DATA.tag
    && transfer_syntax.as_ref().map(|ts| ts.is_encapsulated) == Some(true)
  {
    read_encapsulated_pixel_data_items(&bytes, vr).map_err(|_| {
      JsonDeserializeError::JsonInvalid {
        details: "InlineBinary is not valid encapsulated pixel data"
          .to_string(),
        path: path.clone(),
      }
    })
  } else {
    // This value is native pixel data, so construct a binary value directly
    // from the bytes
    match vr {
      ValueRepresentation::OtherByteString
      | ValueRepresentation::OtherDoubleString
      | ValueRepresentation::OtherFloatString
      | ValueRepresentation::OtherLongString
      | ValueRepresentation::OtherVeryLongString
      | ValueRepresentation::OtherWordString
      | ValueRepresentation::Unknown => {
        Ok(DataElementValue::new_binary_unchecked(vr, Rc::new(bytes)))
      }

      _ => Err(JsonDeserializeError::JsonInvalid {
        details: "InlineBinary for a VR that doesn't support it".to_string(),
        path: path.clone(),
      }),
    }
  }
}

/// Reads an encapsulated pixel data value from raw bytes.
///
fn read_encapsulated_pixel_data_items(
  mut bytes: &[u8],
  vr: ValueRepresentation,
) -> Result<DataElementValue, ()> {
  let mut items = vec![];

  loop {
    if bytes.is_empty() {
      break;
    }

    if bytes.len() < 8 {
      return Err(());
    }

    let group = byteorder::LittleEndian::read_u16(&bytes[0..2]);
    let element = byteorder::LittleEndian::read_u16(&bytes[2..4]);
    let length = byteorder::LittleEndian::read_u32(&bytes[4..8]) as usize;

    if group != dictionary::ITEM.tag.group
      || element != dictionary::ITEM.tag.element
    {
      return Err(());
    }

    if let Some(item) = &bytes.get(8..(8 + length)) {
      items.push(Rc::new(item.to_vec()));
    } else {
      return Err(());
    }

    bytes = &bytes[(8 + length)..];
  }

  DataElementValue::new_encapsulated_pixel_data(vr, items).map_err(|_| ())
}
