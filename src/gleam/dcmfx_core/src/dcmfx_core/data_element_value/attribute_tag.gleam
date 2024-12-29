//// Work with the DICOM `AttributeTag` value representation.

import dcmfx_core/data_element_tag.{type DataElementTag, DataElementTag}
import dcmfx_core/data_error.{type DataError}
import dcmfx_core/internal/bit_array_utils
import gleam/bit_array
import gleam/list
import gleam/result

/// Converts an `AttributeTag` value into data element tags.
///
pub fn from_bytes(bytes: BitArray) -> Result(List(DataElementTag), DataError) {
  bytes
  |> bit_array_utils.to_uint32_list
  |> result.replace_error(data_error.new_value_invalid(
    "AttributeTag data length is not a multiple of 4",
  ))
  |> result.map(
    list.map(_, fn(tag) {
      let assert <<group:16-little-unsigned, element:16-little-unsigned>> = <<
        tag:32-little,
      >>

      DataElementTag(group, element)
    }),
  )
}

/// Converts data element tags into an `AttributeTag` value.
///
pub fn to_bytes(values: List(DataElementTag)) -> Result(BitArray, DataError) {
  values
  |> list.map(fn(tag) {
    let is_valid =
      tag.group >= 0
      && tag.group <= 0xFFFF
      && tag.element >= 0
      && tag.element <= 0xFFFF

    case is_valid {
      True -> Ok(<<tag.group:16-little, tag.element:16-little>>)
      False ->
        Error(data_error.new_value_invalid(
          "AttributeTag group or element are not in the range 0 - 0xFFFF",
        ))
    }
  })
  |> result.all
  |> result.map(bit_array.concat)
}
