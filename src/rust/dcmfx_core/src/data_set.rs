//! A DICOM data set, defined as a map of data element tags to data element
//! values.

pub mod print;

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::data_element_value::{
  age_string, date, date_time, person_name, time,
};
use crate::data_set_path::DataSetPathEntry;
use crate::{
  dictionary, DataElementTag, DataElementValue, DataError, DataSetPath,
  DataSetPrintOptions, TransferSyntax, ValueRepresentation,
};

/// A DICOM data set that is a mapping of data element tags to data element
/// values.
///
#[derive(Clone, Debug, PartialEq)]
pub struct DataSet(BTreeMap<DataElementTag, DataElementValue>);

/// The successful result of looking up a [`DataSetPath`] in a data set.
/// Depending on the path, the result will either be a specific data element
/// value, or a specific sequence item in a sequence (i.e. a nested data set).
///
enum DataSetLookupResult<'a> {
  DataElementValue(&'a DataElementValue),
  DataSet(&'a DataSet),
}

impl DataSet {
  /// Returns a new empty data set.
  ///
  pub fn new() -> Self {
    Self(BTreeMap::new())
  }

  /// Returns the number of data elements in a data set.
  ///
  pub fn size(&self) -> usize {
    self.0.len()
  }

  /// Returns whether a data set is empty and contains no data elements.
  ///
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  /// Returns whether a data element with the specified tag exists in a data
  /// set.
  ///
  pub fn has(&self, tag: DataElementTag) -> bool {
    self.0.contains_key(&tag)
  }

  /// Returns a new data set containing the File Meta Information data elements
  /// in this data set, i.e. those where the data element tag group equals 2.
  ///
  /// This function also sets the *'(0002,0002) Media Storage SOP Class UID'*
  /// and *'(0002,0003) Media Storage SOP Instance UID'* data elements to match
  /// the *'(0008,0016) SOP Class UID'* and *'(0008,0018) SOP Instance UID'*
  /// data elements in this data set.
  ///
  pub fn file_meta_information(&self) -> DataSet {
    let mut file_meta_information: DataSet = self
      .0
      .range((
        std::ops::Bound::Included(DataElementTag::new(2, 0x0000)),
        std::ops::Bound::Included(DataElementTag::new(2, 0xFFFF)),
      ))
      .map(|(tag, value)| (*tag, value.clone()))
      .collect();

    if let Ok(value) = self.get_value(dictionary::SOP_CLASS_UID.tag) {
      if self.get_string(dictionary::SOP_CLASS_UID.tag).is_ok() {
        file_meta_information
          .insert(dictionary::MEDIA_STORAGE_SOP_CLASS_UID.tag, value.clone());
      }
    }

    if let Ok(value) = self.get_value(dictionary::SOP_INSTANCE_UID.tag) {
      if self.get_string(dictionary::SOP_INSTANCE_UID.tag).is_ok() {
        file_meta_information.insert(
          dictionary::MEDIA_STORAGE_SOP_INSTANCE_UID.tag,
          value.clone(),
        );
      }
    }

    // Exclude sequences and encapsulated pixel data, as they aren't allowed in
    // File Meta Information
    file_meta_information
      .0
      .retain(|_tag, value| value.bytes().is_ok());

    file_meta_information
  }

  /// Inserts a data element tag and value into a data set. If there is already
  /// a value for the tag then it is replaced with the new value.
  ///
  pub fn insert(&mut self, tag: DataElementTag, value: DataElementValue) {
    self.0.insert(tag, value);
  }

  /// Inserts a new binary value into a data set. If there is already a value
  /// for the tag it is replaced with the new value.
  ///
  pub fn insert_binary_value(
    &mut self,
    tag: DataElementTag,
    vr: ValueRepresentation,
    bytes: Rc<Vec<u8>>,
  ) -> Result<(), DataError> {
    self.insert(tag, DataElementValue::new_binary(vr, bytes)?);

    Ok(())
  }

  /// Inserts a data element with an age string value into a data set. The data
  /// element being inserted must be referenced through its dictionary entry.
  ///
  pub fn insert_age_string_value(
    &mut self,
    item: &dictionary::Item,
    value: &age_string::StructuredAge,
  ) -> Result<(), DataError> {
    if !item.multiplicity.contains(1) {
      return invalid_insert_error(item);
    }

    let value = match item.vrs {
      [ValueRepresentation::AgeString] => {
        DataElementValue::new_age_string(value)
      }
      _ => invalid_insert_error(item),
    }?;

    self.0.insert(item.tag, value);

    Ok(())
  }

  /// Inserts a data element with an attribute tag value into a data set. The
  /// data element being inserted must be referenced through its dictionary
  /// entry.
  ///
  pub fn insert_attribute_tag_value(
    &mut self,
    item: &dictionary::Item,
    value: &[DataElementTag],
  ) -> Result<(), DataError> {
    if !item.multiplicity.contains(value.len()) {
      return invalid_insert_error(item);
    }

    let value = match item.vrs {
      [ValueRepresentation::AttributeTag] => {
        DataElementValue::new_attribute_tag(value)
      }
      _ => invalid_insert_error(item),
    }?;

    self.0.insert(item.tag, value);

    Ok(())
  }

  /// Inserts a data element with a date value into a data set. The data element
  /// being inserted must be referenced through its dictionary entry.
  ///
  pub fn insert_date_value(
    &mut self,
    item: &dictionary::Item,
    value: &date::StructuredDate,
  ) -> Result<(), DataError> {
    if !item.multiplicity.contains(1) {
      return invalid_insert_error(item);
    }

    let value = match item.vrs {
      [ValueRepresentation::Date] => DataElementValue::new_date(value),
      _ => invalid_insert_error(item),
    }?;

    self.0.insert(item.tag, value);

    Ok(())
  }

  /// Inserts a data element with a date time value into a data set. The data
  /// element being inserted must be referenced through its dictionary entry.
  ///
  pub fn insert_date_time_value(
    &mut self,
    item: &dictionary::Item,
    value: &date_time::StructuredDateTime,
  ) -> Result<(), DataError> {
    if !item.multiplicity.contains(1) {
      return invalid_insert_error(item);
    }

    let value = match item.vrs {
      [ValueRepresentation::Date] => DataElementValue::new_date_time(value),
      _ => invalid_insert_error(item),
    }?;

    self.0.insert(item.tag, value);

    Ok(())
  }

  /// Inserts a data element with float values into a data set. The data element
  /// being inserted must be referenced through its dictionary entry. This
  /// method automatically determines the correct VR to use for the new data
  /// element.
  ///
  pub fn insert_float_value(
    &mut self,
    item: &dictionary::Item,
    value: &[f64],
  ) -> Result<(), DataError> {
    if !item.multiplicity.contains(value.len()) {
      return invalid_insert_error(item);
    }

    let value = match item.vrs {
      [ValueRepresentation::DecimalString] => {
        DataElementValue::new_decimal_string(value)
      }
      [ValueRepresentation::FloatingPointDouble] => {
        DataElementValue::new_floating_point_double(value)
      }
      [ValueRepresentation::FloatingPointSingle] => {
        DataElementValue::new_floating_point_single(
          value
            .iter()
            .map(|f| *f as f32)
            .collect::<Vec<f32>>()
            .as_slice(),
        )
      }
      [ValueRepresentation::OtherDoubleString] => {
        DataElementValue::new_other_double_string(value)
      }
      [ValueRepresentation::OtherFloatString] => {
        DataElementValue::new_other_float_string(
          value
            .iter()
            .map(|f| *f as f32)
            .collect::<Vec<f32>>()
            .as_slice(),
        )
      }

      _ => invalid_insert_error(item),
    }?;

    self.0.insert(item.tag, value);

    Ok(())
  }

  /// Inserts a data element with integer values into a data set. The data
  /// element being inserted must be referenced through its dictionary entry.
  /// This method automatically determines the correct VR to use for the new
  /// data element.
  ///
  pub fn insert_int_value(
    &mut self,
    item: &dictionary::Item,
    value: &[i64],
  ) -> Result<(), DataError> {
    fn convert_and_build<U>(
      value: &[i64],
      converter: fn(i64) -> Result<U, std::num::TryFromIntError>,
      builder: fn(&[U]) -> Result<DataElementValue, DataError>,
      vr: ValueRepresentation,
    ) -> Result<DataElementValue, DataError> {
      let mut converted_values: Vec<U> = Vec::<U>::with_capacity(value.len());

      for i in value {
        let j = converter(*i).map_err(|_| {
          DataError::new_value_invalid(format!(
            "Value {} is out of range for the {} VR",
            i, vr,
          ))
        })?;

        converted_values.push(j);
      }

      builder(&converted_values)
    }

    if !item.multiplicity.contains(value.len()) {
      return invalid_insert_error(item);
    }

    let value = match item.vrs {
      [ValueRepresentation::IntegerString] => convert_and_build(
        value,
        i32::try_from,
        DataElementValue::new_integer_string,
        ValueRepresentation::IntegerString,
      ),

      [ValueRepresentation::SignedLong] => convert_and_build(
        value,
        i32::try_from,
        DataElementValue::new_signed_long,
        ValueRepresentation::SignedLong,
      ),

      [ValueRepresentation::SignedShort] => convert_and_build(
        value,
        i16::try_from,
        DataElementValue::new_signed_short,
        ValueRepresentation::SignedShort,
      ),

      [ValueRepresentation::UnsignedLong] => convert_and_build(
        value,
        u32::try_from,
        DataElementValue::new_unsigned_long,
        ValueRepresentation::UnsignedLong,
      ),

      [ValueRepresentation::UnsignedShort] => convert_and_build(
        value,
        u16::try_from,
        DataElementValue::new_unsigned_short,
        ValueRepresentation::UnsignedShort,
      ),

      _ => invalid_insert_error(item),
    }?;

    self.0.insert(item.tag, value);

    Ok(())
  }

  /// Inserts a data element with big integer values into a data set. The data
  /// element being inserted must be referenced through its dictionary entry.
  /// This method automatically determines the correct VR to use for the new
  /// data element.
  ///
  pub fn insert_big_int_value(
    &mut self,
    item: &dictionary::Item,
    value: &[i128],
  ) -> Result<(), DataError> {
    fn convert_and_build<U>(
      value: &[i128],
      converter: fn(i128) -> Result<U, std::num::TryFromIntError>,
      builder: fn(&[U]) -> Result<DataElementValue, DataError>,
      vr: ValueRepresentation,
      tag: DataElementTag,
    ) -> Result<DataElementValue, DataError> {
      let mut converted_values: Vec<U> = Vec::<U>::with_capacity(value.len());

      for i in value {
        let j = converter(*i).map_err(|_| {
          DataError::new_value_invalid(format!(
            "Value {} is out of range for the {} VR",
            i, vr
          ))
          .with_path(&DataSetPath::new_with_data_element(tag))
        })?;

        converted_values.push(j);
      }

      builder(&converted_values)
    }

    if !item.multiplicity.contains(value.len()) {
      return invalid_insert_error(item);
    }

    let value = match item.vrs {
      [ValueRepresentation::SignedVeryLong] => convert_and_build(
        value,
        i64::try_from,
        DataElementValue::new_signed_very_long,
        ValueRepresentation::SignedVeryLong,
        item.tag,
      ),

      [ValueRepresentation::UnsignedVeryLong] => convert_and_build(
        value,
        u64::try_from,
        DataElementValue::new_unsigned_very_long,
        ValueRepresentation::UnsignedVeryLong,
        item.tag,
      ),

      _ => invalid_insert_error(item),
    }?;

    self.0.insert(item.tag, value);

    Ok(())
  }

  /// Inserts a data element with a person name value into a data set. The data
  /// element being inserted must be referenced through its dictionary entry.
  ///
  pub fn insert_person_name_value(
    &mut self,
    item: &dictionary::Item,
    value: &[person_name::StructuredPersonName],
  ) -> Result<(), DataError> {
    if !item.multiplicity.contains(value.len()) {
      return invalid_insert_error(item);
    }

    let value = match item.vrs {
      [ValueRepresentation::PersonName] => {
        DataElementValue::new_person_name(value)
      }
      _ => invalid_insert_error(item),
    }?;

    self.0.insert(item.tag, value);

    Ok(())
  }

  /// Inserts a data element with a sequence value into a data set. The data
  /// element being inserted must be referenced through its dictionary entry.
  ///
  pub fn insert_sequence_value(
    &mut self,
    item: &dictionary::Item,
    items: Vec<Self>,
  ) -> Result<(), DataError> {
    let value = match item.vrs {
      [ValueRepresentation::Sequence] => {
        Ok(DataElementValue::new_sequence(items))
      }
      _ => invalid_insert_error(item),
    }?;

    self.0.insert(item.tag, value);

    Ok(())
  }

  /// Inserts a data element with a string value into a data set. The data
  /// element being inserted must be referenced through its dictionary entry.
  /// This method automatically determines the correct VR to use for the new
  /// data element.
  ///
  pub fn insert_string_value(
    &mut self,
    item: &dictionary::Item,
    value: &[&str],
  ) -> Result<(), DataError> {
    if !item.multiplicity.contains(value.len()) {
      return invalid_insert_error(item);
    }

    let value = match (item.vrs, value) {
      ([ValueRepresentation::ApplicationEntity], [value]) => {
        DataElementValue::new_application_entity(value)
      }
      ([ValueRepresentation::CodeString], _) => {
        DataElementValue::new_code_string(value)
      }
      ([ValueRepresentation::LongString], _) => {
        DataElementValue::new_long_string(value)
      }
      ([ValueRepresentation::LongText], [value]) => {
        DataElementValue::new_long_text(value.to_string())
      }
      ([ValueRepresentation::ShortString], _) => {
        DataElementValue::new_short_string(value)
      }
      ([ValueRepresentation::ShortText], [value]) => {
        DataElementValue::new_short_text(value)
      }
      ([ValueRepresentation::UniqueIdentifier], _) => {
        DataElementValue::new_unique_identifier(value)
      }
      ([ValueRepresentation::UniversalResourceIdentifier], [value]) => {
        DataElementValue::new_universal_resource_identifier(value)
      }
      ([ValueRepresentation::UnlimitedCharacters], _) => {
        DataElementValue::new_unlimited_characters(value)
      }
      ([ValueRepresentation::UnlimitedText], [value]) => {
        DataElementValue::new_unlimited_text(value)
      }

      _ => invalid_insert_error(item),
    }?;

    self.0.insert(item.tag, value);

    Ok(())
  }

  /// Inserts a data element with a time value into a data set. The data element
  /// being inserted must be referenced through its dictionary entry.
  ///
  pub fn insert_time_value(
    &mut self,
    item: &dictionary::Item,
    value: &time::StructuredTime,
  ) -> Result<(), DataError> {
    if !item.multiplicity.contains(1) {
      return invalid_insert_error(item);
    }

    let value = match item.vrs {
      [ValueRepresentation::Time] => DataElementValue::new_time(value),
      _ => invalid_insert_error(item),
    }?;

    self.0.insert(item.tag, value);

    Ok(())
  }

  /// Merges two data sets together. Data elements from the second data set take
  /// precedence.
  ///
  pub fn merge(&mut self, b: Self) {
    for (key, value) in b.0.into_iter() {
      self.0.insert(key, value);
    }
  }

  /// Deletes a data element from a data set. Returns the deleted data element
  /// value, if any.
  ///
  pub fn delete(&mut self, tag: DataElementTag) -> Option<DataElementValue> {
    self.0.remove(&tag)
  }

  /// Returns the tags in a data set, sorted by group and element.
  ///
  pub fn tags(&self) -> Vec<DataElementTag> {
    self.0.keys().copied().collect()
  }

  /// Returns an iterator over a data set's elements, sorted by tag.
  ///
  pub fn iter(
    &self,
  ) -> std::collections::btree_map::Iter<'_, DataElementTag, DataElementValue>
  {
    self.0.iter()
  }

  /// Returns a mutable iterator over a data set's elements, sorted by tag.
  ///
  pub fn iter_mut(
    &mut self,
  ) -> std::collections::btree_map::IterMut<'_, DataElementTag, DataElementValue>
  {
    self.0.iter_mut()
  }

  /// Prints a data set to stdout formatted for readability.
  ///
  pub fn print(&self) {
    self.print_with_options(&DataSetPrintOptions::default());
  }

  /// Prints a data set to stdout formatted for readability using the given
  /// print options.
  ///
  pub fn print_with_options(&self, print_options: &DataSetPrintOptions) {
    self.to_lines(print_options, &mut |line| {
      println!("{}", line);
    })
  }

  /// Converts a data set to a list of printable lines using the specified print
  /// options. The lines are returned via a callback.
  ///
  pub fn to_lines(
    &self,
    print_options: &DataSetPrintOptions,
    mut callback: &mut impl FnMut(String),
  ) {
    print::data_set_to_lines(self, print_options, &mut callback, 0);
  }

  /// Looks up a data set path in a data set and returns the data element or
  /// data set that it specifies. If the path is invalid for the data set then
  /// an error is returned.
  ///
  fn lookup(
    &self,
    path: &DataSetPath,
  ) -> Result<DataSetLookupResult, DataError> {
    let mut lookup_result = DataSetLookupResult::DataSet(self);

    for entry in path.entries().iter() {
      match lookup_result {
        DataSetLookupResult::DataElementValue(value) => {
          if let DataSetPathEntry::SequenceItem { index } = entry {
            if let Ok(items) = value.sequence_items() {
              if let Some(item) = items.get(*index) {
                lookup_result = DataSetLookupResult::DataSet(item);
                continue;
              }
            }
          }
        }

        DataSetLookupResult::DataSet(data_set) => {
          if let DataSetPathEntry::DataElement { tag } = entry {
            if let Some(value) = data_set.0.get(tag) {
              lookup_result = DataSetLookupResult::DataElementValue(value);
              continue;
            }
          }
        }
      }

      return Err(DataError::new_tag_not_present().with_path(path));
    }

    Ok(lookup_result)
  }

  /// Returns the data element value for the specified tag in a data set.
  ///
  pub fn get_value(
    &self,
    tag: DataElementTag,
  ) -> Result<&DataElementValue, DataError> {
    match self.0.get(&tag) {
      Some(value) => Ok(value),
      _ => Err(
        DataError::new_tag_not_present()
          .with_path(&DataSetPath::new_with_data_element(tag)),
      ),
    }
  }

  /// Returns the data element value at the specified path in a data set. The
  /// path must end with a data element tag.
  ///
  pub fn get_value_at_path(
    &self,
    path: &DataSetPath,
  ) -> Result<&DataElementValue, DataError> {
    match self.lookup(path) {
      Ok(DataSetLookupResult::DataElementValue(value)) => Ok(value),
      _ => Err(DataError::new_tag_not_present().with_path(path)),
    }
  }

  /// Returns the data set at the specified path in a data set. The path must
  /// be empty or end with a sequence item index.
  ///
  pub fn get_data_set_at_path(
    &self,
    path: &DataSetPath,
  ) -> Result<&DataSet, DataError> {
    match self.lookup(path) {
      Ok(DataSetLookupResult::DataSet(data_set)) => Ok(data_set),
      _ => Err(DataError::new_tag_not_present().with_path(path)),
    }
  }

  /// Returns the raw value bytes for the specified tag in a data set.
  ///
  /// See [`DataElementValue::bytes()`].
  ///
  pub fn get_value_bytes(
    &self,
    tag: DataElementTag,
    vr: ValueRepresentation,
  ) -> Result<&Rc<Vec<u8>>, DataError> {
    let value = self.get_value(tag)?;

    if value.value_representation() == vr {
      value
        .bytes()
        .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
    } else {
      Err(
        DataError::new_value_not_present()
          .with_path(&DataSetPath::new_with_data_element(tag)),
      )
    }
  }

  /// Returns the singular string value for a data element in a data set. If the
  /// data element with the specified tag does not hold exactly one string value
  /// then an error is returned.
  ///
  pub fn get_string(&self, tag: DataElementTag) -> Result<&str, DataError> {
    self
      .get_value(tag)?
      .get_string()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns all of the string values for a data element in a data set. If the
  /// data element with the specified tag is not of a type that supports
  /// multiple string values then an error is returned.
  ///
  pub fn get_strings(
    &self,
    tag: DataElementTag,
  ) -> Result<Vec<&str>, DataError> {
    self
      .get_value(tag)?
      .get_strings()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns the singular integer value for a data element in a data set. If
  /// the data element with the specified tag does not hold exactly one integer
  /// value then an error is returned.
  ///
  pub fn get_int<T: num_traits::PrimInt + TryFrom<i64>>(
    &self,
    tag: DataElementTag,
  ) -> Result<T, DataError> {
    self
      .get_value(tag)?
      .get_int()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns all of the integer values for a data element in a data set. If the
  /// data element with the specified tag is not of a type that supports
  /// multiple integer values then an error is returned.
  ///
  pub fn get_ints<T: num_traits::PrimInt + TryFrom<i64>>(
    &self,
    tag: DataElementTag,
  ) -> Result<Vec<T>, DataError> {
    self
      .get_value(tag)?
      .get_ints()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns the lookup table descriptor value for a data element in a data
  /// set. If the data element with the specified tab does not hold a lookup
  /// table descriptor then an error is returned.
  ///
  pub fn get_lookup_table_descriptor(
    &self,
    tag: DataElementTag,
  ) -> Result<(u16, i32, u16), DataError> {
    self
      .get_value(tag)?
      .get_lookup_table_descriptor()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns the singular big integer value for a data element in a data set.
  /// If the data element with the specified tag does not hold exactly one big
  /// integer value then an error is returned.
  ///
  pub fn get_big_int(&self, tag: DataElementTag) -> Result<i128, DataError> {
    self
      .get_value(tag)?
      .get_big_int()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns all of the big integer values for a data element in a data set. If
  /// the data element with the specified tag is not of a type that supports
  /// multiple big integer values then an error is returned.
  ///
  pub fn get_big_ints(
    &self,
    tag: DataElementTag,
  ) -> Result<Vec<i128>, DataError> {
    self
      .get_value(tag)?
      .get_big_ints()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns the singular floating point value for a data element in a data
  /// set. If the data element with the specified tag does not hold exactly one
  /// floating point value then an error is returned.
  ///
  pub fn get_float(&self, tag: DataElementTag) -> Result<f64, DataError> {
    self
      .get_value(tag)?
      .get_float()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns all of the floating point values for a data element in a data set.
  /// If the data element with the specified tag is not of a type that supports
  /// multiple floating point values then an error is returned.
  ///
  pub fn get_floats(&self, tag: DataElementTag) -> Result<Vec<f64>, DataError> {
    self
      .get_value(tag)?
      .get_floats()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns the age value for a data element in a data set. If the data
  /// element does not hold an `AgeString` value then an error is returned.
  ///
  pub fn get_age(
    &self,
    tag: DataElementTag,
  ) -> Result<age_string::StructuredAge, DataError> {
    self
      .get_value(tag)?
      .get_age()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns the date value for a data element in a data set. If the data
  /// element does not hold a `Date` value then an error is returned.
  ///
  pub fn get_date(
    &self,
    tag: DataElementTag,
  ) -> Result<date::StructuredDate, DataError> {
    self
      .get_value(tag)?
      .get_date()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns the structured date/time value for a data element in a data set.
  /// If the data element does not hold a `DateTime` value then an error is
  /// returned.
  ///
  pub fn get_date_time(
    &self,
    tag: DataElementTag,
  ) -> Result<date_time::StructuredDateTime, DataError> {
    self
      .get_value(tag)?
      .get_date_time()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns the time value for a data element in a data set. If the data
  /// element does not hold a `Time` value then an error is returned.
  ///
  pub fn get_time(
    &self,
    tag: DataElementTag,
  ) -> Result<time::StructuredTime, DataError> {
    self
      .get_value(tag)?
      .get_time()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns the singular person name value for a data element in a data set.
  /// If the data element with the specified tag does not hold exactly one
  /// person name value then an error is returned.
  ///
  pub fn get_person_name(
    &self,
    tag: DataElementTag,
  ) -> Result<person_name::StructuredPersonName, DataError> {
    self
      .get_value(tag)?
      .get_person_name()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns all of the person name values for a data element in a data set. If
  /// the data element with the specified tag is not of a type that supports
  /// multiple person name values then an error is returned.
  ///
  pub fn get_person_names(
    &self,
    tag: DataElementTag,
  ) -> Result<Vec<person_name::StructuredPersonName>, DataError> {
    self
      .get_value(tag)?
      .get_person_names()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Returns the sequence items for a data element in a data set. If the data
  /// element with the specified tag is not a sequence then an error is
  /// returned.
  ///
  pub fn get_sequence_items(
    &self,
    tag: DataElementTag,
  ) -> Result<&[DataSet], DataError> {
    self
      .get_value(tag)?
      .sequence_items()
      .map_err(|e| e.with_path(&DataSetPath::new_with_data_element(tag)))
  }

  /// Looks up the *'(0002,0010) Transfer Syntax UID'* data element in this data
  /// set, and if present, attempts to convert it to a known transfer syntax
  /// definition.
  ///
  pub fn get_transfer_syntax(
    &self,
  ) -> Result<&'static TransferSyntax, DataError> {
    let transfer_syntax_uid =
      self.get_string(dictionary::TRANSFER_SYNTAX_UID.tag)?;

    TransferSyntax::from_uid(transfer_syntax_uid).map_err(|_| {
      DataError::new_value_invalid(format!(
        "Unrecognized transfer syntax UID: '{}'",
        transfer_syntax_uid
      ))
    })
  }

  /// Returns the size in bytes of all data elements in a data set.
  ///
  /// See [`DataElementValue::total_byte_size()`].
  ///
  pub fn total_byte_size(&self) -> u64 {
    self
      .iter()
      .fold(0, |acc, (_, value)| acc + value.total_byte_size())
  }

  /// Returns the human-readable name for a data element tag in a data set,
  /// using its data elements to determine the private creator if the tag is
  /// private.
  ///
  pub fn tag_name(&self, tag: DataElementTag) -> &'static str {
    let private_creator = self.private_creator_for_tag(tag).ok();

    dictionary::tag_name(tag, private_creator)
  }

  /// Formats a data element tag in a data set as `"(GROUP,ELEMENT) TAG_NAME"`,
  /// e.g. "(0008,0020) StudyDate"`. The other data elements in the data set
  /// are used to determine the private creator if the tag is private.
  ///
  pub fn tag_with_name(&self, tag: DataElementTag) -> String {
    let private_creator = self.private_creator_for_tag(tag).ok();

    dictionary::tag_with_name(tag, private_creator)
  }

  /// Returns the value of the *'(gggg,00xx) Private Creator'* data element in
  /// this data set for the specified private tag.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn private_creator_for_tag(
    &self,
    tag: DataElementTag,
  ) -> Result<&str, ()> {
    if !tag.is_private() {
      return Err(());
    }

    let private_creator_tag = DataElementTag::new(tag.group, tag.element >> 8);

    if private_creator_tag.element < 0x10 {
      return Err(());
    }

    self.get_string(private_creator_tag).map_err(|_| ())
  }

  /// Removes all private range tags from a data set, including recursively
  /// into any sequences that are present.
  ///
  pub fn delete_private_elements(&mut self) {
    self.0.retain(|tag, value| {
      if tag.is_private() {
        return false;
      }

      if let Ok(items) = value.sequence_items_mut() {
        for item in items.iter_mut() {
          item.delete_private_elements();
        }
      }

      true
    })
  }

  /// Returns a new data set containing just the private tags for the given
  /// group and private creator name in a data set. The group number must always
  /// be odd for private data elements, and the private creator name must match
  /// exactly.
  ///
  /// If the group number is even or there is no *'(gggg,00XX) Private Creator'*
  /// data element with the specified name then an error is returned.
  ///
  pub fn private_block(
    &self,
    group: u16,
    private_creator: &str,
  ) -> Result<Self, String> {
    if group & 2 == 0 {
      return Err("Private group number is even".to_string());
    }

    let private_creator_value =
      DataElementValue::new_long_string(&[private_creator])
        .map_err(|_| "Private creator name is invalid")?;

    // Search for a matching `(gggg,00XX) Private Creator' data element.
    // Ref: PS3.5 7.8.1.
    let mut private_creator_element = None;
    for element in 0x10..=0xFF {
      if self.0.get(&DataElementTag::new(group, element))
        == Some(&private_creator_value)
      {
        private_creator_element = Some(element);
        break;
      }
    }

    let private_creator_element = private_creator_element
      .ok_or(format!("Private creator '{}' not found", private_creator))?;

    // Calculate the range of element values to include in the returned data set
    let element_start = private_creator_element << 8;
    let element_end = element_start | 0xFF;

    // Filter this data set to only include the relevant private data elements
    let mut result = Self::new();
    for (tag, value) in self.0.iter() {
      if tag.group == group
        && tag.element >= element_start
        && tag.element <= element_end
      {
        result.insert(*tag, value.clone());
      }
    }

    Ok(result)
  }
}

impl Default for DataSet {
  fn default() -> Self {
    Self::new()
  }
}

impl FromIterator<(DataElementTag, DataElementValue)> for DataSet {
  fn from_iter<T: IntoIterator<Item = (DataElementTag, DataElementValue)>>(
    iter: T,
  ) -> Self {
    Self(iter.into_iter().collect())
  }
}

impl IntoIterator for DataSet {
  type Item = (DataElementTag, DataElementValue);

  type IntoIter =
    std::collections::btree_map::IntoIter<DataElementTag, DataElementValue>;

  fn into_iter(self) -> Self::IntoIter {
    self.0.into_iter()
  }
}

impl Extend<(DataElementTag, DataElementValue)> for DataSet {
  fn extend<T: IntoIterator<Item = (DataElementTag, DataElementValue)>>(
    &mut self,
    iter: T,
  ) {
    self.0.extend(iter);
  }
}

/// Helper function that returns an error message when one of the
/// `insert_*_element` functions is called with invalid arguments.
///
fn invalid_insert_error<T>(item: &dictionary::Item) -> Result<T, DataError> {
  match item.vrs {
    [vr] => Err(DataError::new_value_invalid(format!(
      "Data element '{}' (VR: '{}', multiplicity: {}) does not support the \
       provided data",
      item.name, vr, item.multiplicity
    ))),

    vrs => Err(DataError::new_value_invalid(format!(
      "Data element '{}' supports multiple VRs: {}",
      item.name,
      vrs
        .iter()
        .map(|vr| vr.to_string())
        .collect::<Vec::<String>>()
        .join(", ")
    ))),
  }
}

#[cfg(test)]
mod tests {}
