//// A path in a data set that specifies the precise location of a specific data
//// element or sequence item.

import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/dictionary
import gleam/bool
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/regexp
import gleam/string

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
pub opaque type DataSetPath {
  DataSetPath(entries: List(DataSetPathEntry))
}

/// An individual entry in a `DataSetPath`.
///
@internal
pub type DataSetPathEntry {
  DataElement(tag: DataElementTag)
  SequenceItem(index: Int)
}

/// Constructs a new data set path with no entries. An empty path is a path to
/// the root data set.
///
pub fn new() -> DataSetPath {
  DataSetPath([])
}

/// Constructs a new data set path with an initial entry for the specified data
/// element.
///
pub fn new_with_data_element(tag: DataElementTag) -> DataSetPath {
  DataSetPath([DataElement(tag)])
}

/// Returns the entries for a data set path.
///
pub fn entries(path: DataSetPath) -> List(DataSetPathEntry) {
  path.entries
}

/// Returns the number of entries in a data set path.
///
pub fn length(path: DataSetPath) -> Int {
  path.entries |> list.length
}

/// Returns whether a data set path is currently empty or pointing to a
/// root-level data element.
///
pub fn is_root(path: DataSetPath) -> Bool {
  case path.entries {
    [] | [DataElement(..)] -> True
    _ -> False
  }
}

/// Returns the number of sequence items present in a data set path.
///
pub fn sequence_item_count(path: DataSetPath) -> Int {
  path.entries
  |> list.fold(0, fn(acc, entry) {
    case entry {
      DataElement(..) -> acc
      SequenceItem(..) -> acc + 1
    }
  })
}

/// Returns the final data element entry in a data set path. Returns an error if
/// the last entry in the data set path is not a data element.
///
pub fn final_data_element(path: DataSetPath) -> Result(DataElementTag, Nil) {
  case path.entries {
    [DataElement(tag), ..] -> Ok(tag)
    _ -> Error(Nil)
  }
}

/// Adds a new entry onto a data set path that specifies the given data
/// element tag. This is only valid when the current path is empty or a
/// sequence item.
///
pub fn add_data_element(
  path: DataSetPath,
  tag: DataElementTag,
) -> Result(DataSetPath, String) {
  case path.entries {
    [] | [SequenceItem(..), ..] ->
      Ok(DataSetPath([DataElement(tag), ..path.entries]))

    _ ->
      Error(
        "Invalid data set path entry: " <> data_element_tag.to_hex_string(tag),
      )
  }
}

/// Adds a new entry onto a data set path that specifies a sequence item
/// index. This is only valid when the current path is a data element tag.
///
pub fn add_sequence_item(
  path: DataSetPath,
  index: Int,
) -> Result(DataSetPath, String) {
  case path.entries {
    [DataElement(..), ..] ->
      Ok(DataSetPath([SequenceItem(index), ..path.entries]))
    _ -> Error("Invalid data set path entry: [" <> int.to_string(index) <> "]")
  }
}

/// Removes the last entry in a data set path.
///
pub fn pop(path: DataSetPath) -> Result(DataSetPath, String) {
  case path.entries {
    [_, ..rest] -> Ok(DataSetPath(rest))
    _ -> Error("Data set path is empty")
  }
}

/// Parses a data set path from a string.
///
pub fn from_string(s: String) -> Result(DataSetPath, String) {
  let path = new()

  use <- bool.guard(s == "", Ok(path))

  s
  |> string.split("/")
  |> list.try_fold(path, fn(path, entry) {
    case data_element_tag.from_hex_string(entry) {
      Ok(tag) -> add_data_element(path, tag)
      Error(Nil) -> {
        let assert Ok(re) = regexp.from_string("^\\[(\\d+)\\]$")

        case regexp.scan(re, entry) {
          [regexp.Match(submatches: [Some(index)], ..)] -> {
            let assert Ok(index) = int.parse(index)
            add_sequence_item(path, index)
          }

          _ -> Error("Invalid data set path entry: " <> entry)
        }
      }
    }
  })
}

/// Formats a data set path with its entries separated by forward slashes,
/// with full details on each of its data element tags that also includes the
/// tag's name.
///
pub fn to_detailed_string(path: DataSetPath) -> String {
  path.entries
  |> list.map(fn(entry) {
    case entry {
      DataElement(tag) -> dictionary.tag_with_name(tag, None)
      SequenceItem(index) -> "Item " <> int.to_string(index)
    }
  })
  |> list.reverse
  |> string.join(" / ")
}

/// Formats a data set path with its entries separated by forward slashes.
///
pub fn to_string(path: DataSetPath) -> String {
  path.entries
  |> list.map(fn(entry) {
    case entry {
      DataElement(tag) -> data_element_tag.to_hex_string(tag)
      SequenceItem(index) -> "[" <> int.to_string(index) <> "]"
    }
  })
  |> list.reverse
  |> string.join("/")
}
