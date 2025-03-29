use dcmfx_core::{DataElementTag, DataError, DataSet, dictionary};

use crate::LookupTable;

/// A set of three [`LookupTable`]s, one each for red, green and blue values.
/// This type can be created from a data set and is used to support the
/// `"PALETTE_COLOR"` photometric interpretation.
///
#[derive(Clone, Debug, PartialEq)]
pub struct RgbLut {
  red: LookupTable,
  green: LookupTable,
  blue: LookupTable,
}

impl RgbLut {
  /// The tags of the data elements relevant to construction of [`RgbLut`].
  ///
  pub const DATA_ELEMENT_TAGS: [DataElementTag; 9] = [
    dictionary::RED_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag,
    dictionary::RED_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::SEGMENTED_RED_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag,
    dictionary::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::SEGMENTED_GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag,
    dictionary::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::SEGMENTED_BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
  ];

  /// Creates a new [`RgbLut`] from the given data set, using the data elements
  /// specified in [`Self::DATA_ELEMENT_TAGS`].
  ///
  pub fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let red_lut = LookupTable::from_data_set(
      data_set,
      dictionary::RED_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag,
      dictionary::RED_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
      Some(dictionary::SEGMENTED_RED_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag),
      None,
    )?;

    let green_lut = LookupTable::from_data_set(
      data_set,
      dictionary::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag,
      dictionary::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
      Some(dictionary::SEGMENTED_GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag),
      None,
    )?;

    let blue_lut = LookupTable::from_data_set(
      data_set,
      dictionary::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag,
      dictionary::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
      Some(dictionary::SEGMENTED_BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag),
      None,
    )?;

    Ok(Self {
      red: red_lut,
      green: green_lut,
      blue: blue_lut,
    })
  }

  /// Looks up a value in the RGB lookup tables.
  ///
  pub fn lookup(&self, stored_value: i64) -> [u16; 3] {
    [
      self.red.lookup(stored_value),
      self.green.lookup(stored_value),
      self.blue.lookup(stored_value),
    ]
  }

  /// Looks up a value in the RGB lookup tables and normalizes the result into
  /// the 0-1 range.
  ///
  pub fn lookup_normalized(&self, stored_value: i64) -> [f32; 3] {
    [
      self.red.lookup_normalized(stored_value),
      self.green.lookup_normalized(stored_value),
      self.blue.lookup_normalized(stored_value),
    ]
  }

  /// Looks up a value in the RGB lookup table and normalizes the result into
  /// the 0-255 range.
  ///
  pub fn lookup_normalized_u8(&self, stored_value: i64) -> [u8; 3] {
    [
      self.red.lookup_normalized_u8(stored_value),
      self.green.lookup_normalized_u8(stored_value),
      self.blue.lookup_normalized_u8(stored_value),
    ]
  }
}
