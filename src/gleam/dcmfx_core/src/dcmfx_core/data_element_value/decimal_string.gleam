//// Work with the DICOM `DecimalString` value representation.

import dcmfx_core/data_error.{type DataError}
import dcmfx_core/internal/bit_array_utils
import dcmfx_core/internal/utils
import gleam/bit_array
import gleam/float
import gleam/list
import gleam/result
import gleam/string

/// Converts a `DecimalString` value to a list of floats.
///
pub fn from_bytes(bytes: BitArray) -> Result(List(Float), DataError) {
  let decimal_string =
    bytes
    |> bit_array.to_string
    |> result.replace_error(data_error.new_value_invalid(
      "DecimalString is invalid UTF-8",
    ))
  use decimal_string <- result.try(decimal_string)

  let decimal_string = utils.trim_ascii(decimal_string, 0x00)

  decimal_string
  |> string.split("\\")
  |> list.map(string.trim)
  |> list.filter(fn(s) { !string.is_empty(s) })
  |> list.map(utils.smart_parse_float)
  |> result.all
  |> result.map_error(fn(_) {
    data_error.new_value_invalid(
      "DecimalString is invalid: '" <> decimal_string <> "'",
    )
  })
}

/// Converts a list of floats to a `DecimalString` value.
///
pub fn to_bytes(values: List(Float)) -> BitArray {
  let values =
    list.map(values, fn(f) {
      let value = float_to_shortest_string(f)

      // When exponential notation isn't in use, trim unnecessary zeros
      // and decimal point characters from the end of the string
      case string.contains(value, "e") {
        True -> value
        False ->
          value
          |> utils.trim_ascii_end(0x30)
          |> utils.trim_ascii_end(0x2E)
          |> string.slice(0, 16)
      }
    })

  values
  |> string.join("\\")
  |> bit_array.from_string
  |> bit_array_utils.pad_to_even_length(0x20)
}

@external(javascript, "./decimal_string_ffi.mjs", "float_to_shortest_string")
fn float_to_shortest_string(f: Float) -> String {
  float.to_string(f)
}
