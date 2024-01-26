//// Work with the DICOM `IntegerString` value representation.

import dcmfx_core/data_error.{type DataError}
import dcmfx_core/internal/bit_array_utils
import dcmfx_core/internal/utils
import gleam/bit_array
import gleam/bool
import gleam/int
import gleam/list
import gleam/result
import gleam/string

/// Converts a `IntegerString` value to a list of ints.
///
pub fn from_bytes(bytes: BitArray) -> Result(List(Int), DataError) {
  let integer_string =
    bytes
    |> bit_array.to_string
    |> result.map(utils.trim_right_whitespace)
    |> result.replace_error(data_error.new_value_invalid(
      "IntegerString is invalid UTF-8",
    ))
  use integer_string <- result.try(integer_string)

  integer_string
  |> string.split("\\")
  |> list.map(string.trim)
  |> list.filter(fn(s) { !string.is_empty(s) })
  |> list.map(int.parse)
  |> result.all
  |> result.map_error(fn(_) {
    data_error.new_value_invalid(
      "IntegerString is invalid: '" <> integer_string <> "'",
    )
  })
}

/// Converts a list of ints to an `IntegerString` value.
///
pub fn to_bytes(values: List(Int)) -> Result(BitArray, DataError) {
  let is_valid =
    list.all(values, fn(i) { i >= -2_147_483_648 && i <= 2_147_483_647 })

  use <- bool.guard(
    !is_valid,
    Error(data_error.new_value_invalid("IntegerString value is out of range")),
  )

  values
  |> list.map(int.to_string)
  |> string.join("\\")
  |> bit_array.from_string
  |> bit_array_utils.pad_to_even_length(0x20)
  |> Ok
}
