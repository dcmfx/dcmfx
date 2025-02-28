//! A path in a data set that specifies the precise location of a specific data
//! element or sequence item.

use crate::{DataElementTag, dictionary};

/// A path in a data set that specifies the precise location of a specific data
/// element or sequence item. Entries in a data set path are separated by a
/// forward slash when a path is represented as a string.
///
/// Examples:
///
/// - `""`: Path to the root data set.
/// - `"00100010"`: Path to the *'(0010,0010) Patient Name'* data element.
/// - `"00186011/[0]"`: Path to the first sequence item in the *'(0018,6011)
///   Sequence of Ultrasound Regions'* data element.
/// - `"00186011/[1]/00186014"`: Path to the *'(0018,6014) Region Data Type'*
///   data element in the second item of the *'(0018,6011) Sequence of
///   Ultrasound Regions'* sequence.
///
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DataSetPath(Vec<DataSetPathEntry>);

/// An individual entry in a [`DataSetPath`].
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DataSetPathEntry {
  DataElement { tag: DataElementTag },
  SequenceItem { index: usize },
}

impl DataSetPath {
  /// Constructs a new data set path with no entries. An empty path is a path to
  /// the root data set.
  ///
  pub fn new() -> Self {
    Self(vec![])
  }

  /// Constructs a new data set path with an initial entry for the specified
  /// data element.
  ///
  pub fn new_with_data_element(tag: DataElementTag) -> Self {
    Self(vec![DataSetPathEntry::DataElement { tag }])
  }

  /// Returns the entries for a data set path.
  ///
  pub(crate) fn entries(&self) -> &Vec<DataSetPathEntry> {
    &self.0
  }

  /// Returns the number of entries in a data set path.
  ///
  pub fn len(&self) -> usize {
    self.0.len()
  }

  /// Returns whether a data set path has no entries.
  ///
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  /// Returns the number of sequence items present in a data set path.
  ///
  pub fn sequence_item_count(&self) -> usize {
    self
      .0
      .iter()
      .filter(|entry| matches!(entry, DataSetPathEntry::SequenceItem { .. }))
      .count()
  }

  /// Returns the final data element entry in a data set path. Returns an error
  /// if the last entry in the data set path is not a data element.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn final_data_element(&self) -> Result<DataElementTag, ()> {
    match self.0.last() {
      Some(DataSetPathEntry::DataElement { tag }) => Ok(*tag),
      _ => Err(()),
    }
  }

  /// Adds a new entry onto a data set path that specifies the given data
  /// element tag. This is only valid when the current path is empty or a
  /// sequence item.
  ///
  pub fn add_data_element(
    &mut self,
    tag: DataElementTag,
  ) -> Result<(), String> {
    match self.0.last() {
      None | Some(DataSetPathEntry::SequenceItem { .. }) => {
        self.0.push(DataSetPathEntry::DataElement { tag });
        Ok(())
      }
      _ => Err(format!(
        "Invalid data set path entry: {}",
        tag.to_hex_string()
      )),
    }
  }

  /// Adds a new entry onto a data set path that specifies a sequence item
  /// index. This is only valid when the current path is a data element tag.
  ///
  pub fn add_sequence_item(&mut self, index: usize) -> Result<(), String> {
    match self.0.last() {
      Some(DataSetPathEntry::DataElement { .. }) => {
        self.0.push(DataSetPathEntry::SequenceItem { index });
        Ok(())
      }
      _ => Err(format!("Invalid data set path entry: [{}]", index)),
    }
  }

  /// Removes the last entry in a data set path.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn pop(&mut self) -> Result<(), String> {
    if self.0.is_empty() {
      Err("Data set path is empty".to_string())
    } else {
      self.0.pop();
      Ok(())
    }
  }

  /// Parses a data set path from a string.
  ///
  pub fn from_string(s: &str) -> Result<Self, String> {
    let mut result = Self::new();

    if s.is_empty() {
      return Ok(result);
    }

    for entry in s.split("/") {
      if let Ok(tag) = DataElementTag::from_hex_string(entry) {
        result.add_data_element(tag)?;
        continue;
      }

      if entry.starts_with('[') && entry.ends_with(']') {
        if let Ok(index) = entry[1..entry.len() - 1].parse::<usize>() {
          result.add_sequence_item(index)?;
          continue;
        }
      }

      return Err(format!("Invalid data set path entry: {}", entry));
    }

    Ok(result)
  }

  /// Formats a data set path with its entries separated by forward slashes,
  /// with full details on each of its data element tags that also includes the
  /// tag's name.
  ///
  pub fn to_detailed_string(&self) -> String {
    self
      .0
      .iter()
      .map(|entry| match entry {
        DataSetPathEntry::DataElement { tag } => {
          dictionary::tag_with_name(*tag, None)
        }
        DataSetPathEntry::SequenceItem { index } => format!("Item {}", index),
      })
      .collect::<Vec<_>>()
      .join(" / ")
  }
}

impl std::fmt::Display for DataSetPath {
  /// Formats a data set path with its entries separated by forward slashes.
  ///
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    let path = self
      .0
      .iter()
      .map(|entry| match entry {
        DataSetPathEntry::DataElement { tag } => tag.to_hex_string(),
        DataSetPathEntry::SequenceItem { index } => format!("[{}]", index),
      })
      .collect::<Vec<_>>()
      .join("/");

    f.write_str(&path)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_string_test() {
    let mut path = DataSetPath::new();

    assert_eq!(
      path.add_data_element(DataElementTag::new(0x1234, 0x5678)),
      Ok(())
    );

    assert_eq!(&path.to_string(), "12345678");

    assert_eq!(
      path.add_data_element(DataElementTag::new(0x1234, 0x5678)),
      Err("Invalid data set path entry: 12345678".to_string())
    );

    assert_eq!(path.add_sequence_item(2), Ok(()));

    assert_eq!(&path.to_string(), "12345678/[2]");

    assert_eq!(
      path.add_sequence_item(2),
      Err("Invalid data set path entry: [2]".to_string())
    );

    assert_eq!(
      path.add_data_element(DataElementTag::new(0x1122, 0x3344)),
      Ok(())
    );

    assert_eq!(&path.to_string(), "12345678/[2]/11223344");
  }

  #[test]
  fn from_string_test() {
    let mut path = DataSetPath::new();

    assert_eq!(DataSetPath::from_string(""), Ok(DataSetPath::new()));

    path
      .add_data_element(DataElementTag::new(0x1234, 0x5678))
      .unwrap();
    assert_eq!(DataSetPath::from_string("12345678"), Ok(path.clone()));

    path.add_sequence_item(2).unwrap();
    assert_eq!(DataSetPath::from_string("12345678/[2]"), Ok(path.clone()));

    path
      .add_data_element(DataElementTag::new(0x1122, 0x3344))
      .unwrap();
    assert_eq!(
      DataSetPath::from_string("12345678/[2]/11223344"),
      Ok(path.clone())
    );
  }
}
