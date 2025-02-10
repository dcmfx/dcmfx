//// A DICOM data element tag, defined as 16-bit `group` and `element` values.

import dcmfx_core/internal/utils
import gleam/bool
import gleam/int
import gleam/order
import gleam/result
import gleam/string

/// A data element tag that is defined by `group` and `element` values, each of
/// which is a 16-bit unsigned integer.
///
pub type DataElementTag {
  DataElementTag(group: Int, element: Int)
}

/// Formats a data element tag as `"($GROUP,$ELEMENT)"`, e.g.`"(0008,002D)"`.
///
pub fn to_string(tag: DataElementTag) -> String {
  "("
  <> uint16_to_string(tag.group)
  <> ","
  <> uint16_to_string(tag.element)
  <> ")"
}

/// A data element tag with a group and element set to zero.
///
pub const zero = DataElementTag(0, 0)

/// Returns whether the tag is private, which is determined by its group number
/// being odd.
///
pub fn is_private(tag: DataElementTag) -> Bool {
  int.is_odd(tag.group)
}

/// Returns whether the tag is for a private creator, which is determined by its
/// group number being odd and its element being between 0x10 and 0xFF.
///
/// Ref: PS3.5 7.8.1.
///
pub fn is_private_creator(tag: DataElementTag) -> Bool {
  int.is_odd(tag.group) && tag.element >= 0x10 && tag.element <= 0xFF
}

/// Converts a tag to a single 32-bit integer where the group is in the high 16
/// bits and the element is in the low 16 bits.
///
pub fn to_int(tag: DataElementTag) -> Int {
  tag.group * 65_536 + tag.element
}

/// Compares two data element tags.
///
pub fn compare(lhs: DataElementTag, rhs: DataElementTag) -> order.Order {
  int.compare(to_int(lhs), to_int(rhs))
}

/// Formats a data element tag as `"$GROUP$ELEMENT"`, e.g.`"00080020"`.
///
pub fn to_hex_string(tag: DataElementTag) -> String {
  uint16_to_string(tag.group) <> uint16_to_string(tag.element)
}

/// Creates a data element tag from a hex string formatted as
/// `"$GROUP$ELEMENT"`, e.g.`"00080020"`.
///
pub fn from_hex_string(tag: String) -> Result(DataElementTag, Nil) {
  use <- bool.guard(utils.string_fast_length(tag) != 8, Error(Nil))

  let group = string.slice(tag, 0, 4)
  let element = string.slice(tag, 4, 4)

  use group <- result.try(int.base_parse(group, 16))
  use element <- result.map(int.base_parse(element, 16))

  DataElementTag(group, element)
}

/// Formats a 16-bit unsigned integer as a 4-digit hexadecimal string.
///
fn uint16_to_string(value: Int) -> String {
  value
  |> int.to_base16
  |> utils.pad_start(4, "0")
}
