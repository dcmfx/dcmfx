use dcmfx_core::{
  DataElementTag, DataError, DataSet, DataSetPath, IodModule,
  ValueRepresentation, dictionary,
};

use crate::LookupTable;

/// The attributes of the Palette Color Lookup Table Module, which is a set of
/// three [`LookupTable`]s, one each for red, green and blue values. Used by
/// the `PALETTE_COLOR` photometric interpretation.
///
/// Ref: PS3.3 C.7.9.
///
#[derive(Clone, Debug, PartialEq)]
pub struct PaletteColorLookupTableModule {
  red_lut: LookupTable,
  green_lut: LookupTable,
  blue_lut: LookupTable,
}

impl IodModule for PaletteColorLookupTableModule {
  fn is_iod_module_data_element(
    tag: DataElementTag,
    _vr: ValueRepresentation,
    _length: Option<u32>,
    path: &DataSetPath,
  ) -> bool {
    if !path.is_root() {
      return false;
    }

    tag == dictionary::RED_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag
      || tag == dictionary::RED_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag
      || tag == dictionary::SEGMENTED_RED_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag
      || tag == dictionary::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag
      || tag == dictionary::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag
      || tag == dictionary::SEGMENTED_GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag
      || tag == dictionary::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag
      || tag == dictionary::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag
      || tag == dictionary::SEGMENTED_BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag
  }

  fn iod_module_highest_tag() -> DataElementTag {
    dictionary::SEGMENTED_BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag
  }

  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
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

    Ok(Self::new(red_lut, green_lut, blue_lut))
  }
}

impl PaletteColorLookupTableModule {
  /// Creates a new set of RGB lookup tables.
  ///
  pub fn new(
    red_lut: LookupTable,
    green_lut: LookupTable,
    blue_lut: LookupTable,
  ) -> Self {
    Self {
      red_lut,
      green_lut,
      blue_lut,
    }
  }

  /// Looks up a value in the RGB lookup tables.
  ///
  pub fn lookup(&self, stored_value: i64) -> [u16; 3] {
    [
      self.red_lut.lookup(stored_value),
      self.green_lut.lookup(stored_value),
      self.blue_lut.lookup(stored_value),
    ]
  }

  /// Looks up a value in the RGB lookup tables and normalizes the result into
  /// the 0-1 range.
  ///
  pub fn lookup_normalized(&self, stored_value: i64) -> [f32; 3] {
    [
      self.red_lut.lookup_normalized(stored_value),
      self.green_lut.lookup_normalized(stored_value),
      self.blue_lut.lookup_normalized(stored_value),
    ]
  }

  /// Looks up a value in the RGB lookup table and normalizes the result into
  /// the 0-255 range.
  ///
  pub fn lookup_normalized_u8(&self, stored_value: i64) -> [u8; 3] {
    [
      self.red_lut.lookup_normalized_u8(stored_value),
      self.green_lut.lookup_normalized_u8(stored_value),
      self.blue_lut.lookup_normalized_u8(stored_value),
    ]
  }

  /// Returns the maximum value that can be stored by any of the red, green or
  /// blue LUTs.
  ///
  pub fn int_max(&self) -> u16 {
    let red_int_max = self.red_lut.int_max();
    let green_int_max = self.green_lut.int_max();
    let blue_int_max = self.blue_lut.int_max();

    red_int_max.max(green_int_max).max(blue_int_max)
  }

  /// Converts this Palette Color Lookup Table Module to a data set.
  ///
  pub fn to_data_set(&self) -> DataSet {
    let mut data_set = DataSet::new();

    data_set.merge(self.red_lut.input_data_set().clone());
    data_set.merge(self.green_lut.input_data_set().clone());
    data_set.merge(self.blue_lut.input_data_set().clone());

    data_set
  }
}
