#[cfg(not(feature = "std"))]
use alloc::{format, string::String, string::ToString, vec, vec::Vec};

use byteorder::ByteOrder;

use dcmfx_core::{DataElementTag, DataError, DataSet, DataSetPath};

use crate::utils::udiv_round;

/// A lookup table containing unsigned 16-bit values that is created from LUT
/// Descriptor, LUT Data (or Segmented LUT Data), and optionally LUT Explanation
/// data elements.
///
/// Used as part of Modality LUTs and VOI LUTs.
///
#[derive(Clone, Debug, PartialEq)]
pub struct LookupTable {
  /// When looking up an input value in this LUT, this is the input value that
  /// maps to the LUT's first entry. Lower input values than this also return
  /// the first entry in the LUT data.
  ///
  /// Taken from the *'(0028,3002) LUT Descriptor'* data element.
  first_input_value: i64,

  /// Free form text explanation of the meaning of the LUT.
  explanation: Option<String>,

  /// The raw data for the LUT.
  data: Vec<u16>,

  /// The largest number that can be stored in the LUT. This is calculated using
  /// the `bits_per_entry` value.
  int_max: u32,

  /// Scale factor that converts a lookup table value into the range 0-1.
  normalization_scale: f32,
}

impl LookupTable {
  /// Creates a [`LookupTable`] from the relevant data elements in a data set.
  ///
  pub fn from_data_set(
    data_set: &DataSet,
    lut_descriptor_tag: DataElementTag,
    lut_data_tag: DataElementTag,
    segmented_lut_data_tag: Option<DataElementTag>,
    lut_explanation_tag: Option<DataElementTag>,
  ) -> Result<LookupTable, DataError> {
    let (entry_count, first_input_value, bits_per_entry) =
      data_set.get_lookup_table_descriptor(lut_descriptor_tag)?;

    // An entry count of zero means that there are 2^16 entries
    let entry_count = if entry_count == 0 {
      65536usize
    } else {
      usize::from(entry_count)
    };

    // Validate the bits per entry value
    if !(8..=16).contains(&bits_per_entry) {
      return Err(
        DataError::new_value_invalid(format!(
          "LUT descriptor bits per entry '{bits_per_entry}' is invalid"
        ))
        .with_path(&DataSetPath::new_with_data_element(lut_descriptor_tag)),
      );
    }

    // Read the LUT data or segmented LUT data into a u16 buffer
    let mut data = match data_set.get_value_bytes(lut_data_tag) {
      Ok(data) => {
        if data.len() == entry_count * 2 {
          let mut buffer = vec![0u16; entry_count];
          byteorder::LittleEndian::read_u16_into(data, &mut buffer);
          buffer
        } else if data.len() == entry_count && bits_per_entry == 8 {
          data.iter().map(|i| u16::from(*i)).collect()
        } else {
          return Err(
            DataError::new_value_invalid(
              "LUT data length is invalid".to_string(),
            )
            .with_path(&DataSetPath::new_with_data_element(lut_data_tag)),
          );
        }
      }

      Err(e) => match segmented_lut_data_tag {
        Some(tag) => {
          Self::evaluate_segmented_lut(data_set, tag, bits_per_entry)?
        }

        None => return Err(e),
      },
    };

    // Zero any unused high bits in the LUT data
    if bits_per_entry < 16 {
      let mask = (1 << bits_per_entry) - 1;
      for entry in data.iter_mut() {
        *entry &= mask;
      }
    }

    // Read the LUT explanation if specified. It's allowed to be absent.
    let explanation = if let Some(lut_explanation_tag) = lut_explanation_tag {
      if data_set.has(lut_explanation_tag) {
        Some(data_set.get_string(lut_explanation_tag)?.to_string())
      } else {
        None
      }
    } else {
      None
    };

    let int_max = (1u32 << bits_per_entry) - 1;

    // Scale factor that converts a lookup table value into the range 0-1
    let normalization_scale = 1.0 / (int_max as f32);

    Ok(Self {
      first_input_value: first_input_value.into(),
      explanation,
      data,
      int_max,
      normalization_scale,
    })
  }

  /// Evaluates segmented lookup table data into a final lookup table.
  ///
  /// Note: indirect segments aren't currently supported.
  ///
  /// Ref: PS3.3 C.7.9.2
  ///
  fn evaluate_segmented_lut(
    data_set: &DataSet,
    segmented_lut_data_tag: DataElementTag,
    bits_per_entry: u16,
  ) -> Result<Vec<u16>, DataError> {
    let segmented_lut_data_bytes =
      data_set.get_value_bytes(segmented_lut_data_tag)?;

    let segment_data = if bits_per_entry == 8 {
      segmented_lut_data_bytes
        .iter()
        .map(|i| u16::from(*i))
        .collect()
    } else if segmented_lut_data_bytes.len() % 2 == 0 {
      let mut buffer = vec![0u16; segmented_lut_data_bytes.len() / 2];
      byteorder::LittleEndian::read_u16_into(
        segmented_lut_data_bytes,
        &mut buffer,
      );
      buffer
    } else {
      return Err(DataError::new_value_invalid("".to_string()));
    };

    let mut lut = vec![];
    let mut offset = 0;

    while offset + 1 < segment_data.len() {
      let opcode = segment_data[offset];
      let length = segment_data[offset + 1];

      offset += 2;

      match opcode {
        // Discrete segment. Ref: PS3.3 C.7.9.2.1.
        0 => {
          if let Some(segment) =
            segment_data.get(offset..(offset + usize::from(length)))
          {
            lut.extend_from_slice(segment);
            offset += segment.len();
          } else {
            return Err(DataError::new_value_invalid(format!(
              "Discrete segment in segmented palette lookup table has invalid \
              length '{length}')",
            )));
          }
        }

        // Linear segment. Ref: PS3.3 C.7.9.2.2.
        1 => {
          let y0 = if let Some(y0) = lut.last() {
            *y0
          } else {
            return Err(DataError::new_value_invalid(
              "Linear segment in segmented palette lookup is invalid when \
               there have been no previous segments with positive length"
                .to_string(),
            ));
          };

          let y1 = match segment_data.get(offset) {
            Some(y1) => *y1,
            None => {
              return Err(DataError::new_value_invalid(
                "Linear segment Y1 value is missing".to_string(),
              ));
            }
          };

          offset += 1;

          // Evaluate the linear segment
          let step = f32::from(y1 - y0) / f32::from(length);
          for i in 0..length {
            let f = f32::from(y0) + f32::from(i + 1) * step;

            lut.push(f.round().clamp(u16::MIN.into(), u16::MAX.into()) as u16);
          }
        }

        // Indirect segment. Ref: PS3.3 C.7.9.2.3.
        2 => {
          // Indirect segments aren't supported due to a lack of data to test
          // against. Segmented lookup tables are somewhat rare, and ones
          // containing indirect segments appear to be even rarer. TODO.
          return Err(DataError::new_value_invalid(
            "Indirect segments in segmented palette lookup tables are \
             not supported"
              .to_string(),
          ));
        }

        opcode => {
          return Err(DataError::new_value_invalid(format!(
            "Invalid segment opcode '{opcode}'in segmented palette lookup table"
          )));
        }
      }
    }

    Ok(lut)
  }

  /// Looks up a value in this lookup table.
  ///
  pub fn lookup(&self, stored_value: i64) -> u16 {
    let index = stored_value - self.first_input_value;
    let clamped_index = index.clamp(0, self.data.len() as i64 - 1);

    self.data[clamped_index as usize]
  }

  /// Looks up a value in this lookup table and normalizes the result into the
  /// 0-1 range.
  ///
  pub fn lookup_normalized(&self, stored_value: i64) -> f32 {
    f32::from(self.lookup(stored_value)) * self.normalization_scale
  }

  /// Looks up a value in this lookup table and normalizes the result into the
  /// 0-255 range.
  ///
  pub fn lookup_normalized_u8(&self, stored_value: i64) -> u8 {
    let x = u32::from(self.lookup(stored_value));

    udiv_round(x * 255, self.int_max).min(0xFF) as u8
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use dcmfx_core::{DataElementValue, ValueRepresentation, dictionary};

  #[test]
  fn create_from_data_set() {
    let mut ds = DataSet::new();
    ds.insert(
      dictionary::LUT_DESCRIPTOR.tag,
      DataElementValue::new_lookup_table_descriptor_unchecked(
        ValueRepresentation::SignedShort,
        vec![4, 0, 255, 255, 16, 0].into(),
      ),
    );
    ds.insert(
      dictionary::LUT_DATA.tag,
      DataElementValue::new_unsigned_short(&[1, 2, 3, 4]).unwrap(),
    );
    ds.insert_string_value(&dictionary::LUT_EXPLANATION, &["test"])
      .unwrap();

    let lut = LookupTable::from_data_set(
      &ds,
      dictionary::LUT_DESCRIPTOR.tag,
      dictionary::LUT_DATA.tag,
      None,
      Some(dictionary::LUT_EXPLANATION.tag),
    )
    .unwrap();

    assert_eq!(lut.first_input_value, -1);
    assert_eq!(lut.data, vec![1, 2, 3, 4]);
    assert_eq!(lut.explanation, Some("test".to_string()));
  }

  #[test]
  fn create_from_data_set_with_segmented_lut() {
    let mut ds = DataSet::new();
    ds.insert(
      dictionary::LUT_DESCRIPTOR.tag,
      DataElementValue::new_lookup_table_descriptor_unchecked(
        ValueRepresentation::SignedShort,
        vec![0, 1, 0, 0, 16, 0].into(),
      ),
    );
    ds.insert(
      dictionary::SEGMENTED_RED_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
      DataElementValue::new_unsigned_short(&[0, 1, 0, 1, 127, 0, 1, 128, 254])
        .unwrap(),
    );

    let lut = LookupTable::from_data_set(
      &ds,
      dictionary::LUT_DESCRIPTOR.tag,
      dictionary::LUT_DATA.tag,
      Some(dictionary::SEGMENTED_RED_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag),
      None,
    )
    .unwrap();

    assert_eq!(
      lut.data,
      vec![
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26,
        28, 30, 32, 34, 36, 38, 40, 42, 44, 46, 48, 50, 52, 54, 56, 58, 60, 62,
        64, 65, 67, 69, 71, 73, 75, 77, 79, 81, 83, 85, 87, 89, 91, 93, 95, 97,
        99, 101, 103, 105, 107, 109, 111, 113, 115, 117, 119, 121, 123, 125,
        127, 129, 131, 133, 135, 137, 139, 141, 143, 145, 147, 149, 151, 153,
        155, 157, 159, 161, 163, 165, 167, 169, 171, 173, 175, 177, 179, 181,
        183, 185, 187, 189, 191, 192, 194, 196, 198, 200, 202, 204, 206, 208,
        210, 212, 214, 216, 218, 220, 222, 224, 226, 228, 230, 232, 234, 236,
        238, 240, 242, 244, 246, 248, 250, 252, 254,
      ]
    );
  }

  #[test]
  fn lookup_value() {
    let lut = LookupTable {
      first_input_value: 50,
      explanation: None,
      data: vec![1, 4, 9, 16, 64],
      int_max: 255,
      normalization_scale: 1.0 / 255.0,
    };

    assert_eq!(lut.lookup(48), 1);
    assert_eq!(lut.lookup(52), 9);
    assert_eq!(lut.lookup(56), 64);
    assert_eq!(lut.lookup_normalized(51), 4.0 / 255.0);
  }
}
