//// Work with the DICOM `AgeString` value representation.

import dcmfx_core/data_error.{type DataError}
import dcmfx_core/internal/bit_array_utils
import dcmfx_core/internal/utils
import gleam/bit_array
import gleam/bool
import gleam/int
import gleam/option.{Some}
import gleam/regex
import gleam/result
import gleam/string

/// The time units that can be specified by a structured age.
///
pub type AgeUnit {
  Days
  Weeks
  Months
  Years
}

/// A structured age that can be converted to/from an `AgeString` value.
///
pub type StructuredAge {
  StructuredAge(number: Int, unit: AgeUnit)
}

/// Formats a structured age as a human-readable string.
///
pub fn to_string(age: StructuredAge) -> String {
  let unit = case age.unit {
    Days -> "day"
    Weeks -> "week"
    Months -> "month"
    Years -> "year"
  }

  let plural = case age.number {
    1 -> ""
    _ -> "s"
  }

  int.to_string(age.number) <> " " <> unit <> plural
}

/// Converts an `AgeString` value into a structured age.
///
pub fn from_bytes(bytes: BitArray) -> Result(StructuredAge, DataError) {
  let age_string =
    bytes
    |> bit_array.to_string
    |> result.map(utils.trim_right_whitespace)
    |> result.replace_error(data_error.new_value_invalid(
      "AgeString is invalid UTF-8",
    ))
  use age_string <- result.try(age_string)

  let assert Ok(re) = regex.from_string("^(\\d\\d\\d)([DWMY])$")

  case regex.scan(re, age_string) {
    [regex.Match(submatches: [Some(number), Some(unit)], ..)] -> {
      let assert Ok(number) = int.parse(number)

      let unit = case unit {
        "D" -> Days
        "W" -> Weeks
        "M" -> Months
        _ -> Years
      }

      Ok(StructuredAge(number, unit))
    }

    _ ->
      Error(data_error.new_value_invalid(
        "AgeString is invalid: '" <> age_string <> "'",
      ))
  }
}

/// Converts a structured age into an `AgeString` value.
///
pub fn to_bytes(age: StructuredAge) -> Result(BitArray, DataError) {
  use <- bool.guard(
    age.number < 0 || age.number > 999,
    Error(data_error.new_value_invalid(
      "AgeString value "
      <> int.to_string(age.number)
      <> " is outside the valid range of 0-999",
    )),
  )

  let unit = case age.unit {
    Days -> "D"
    Weeks -> "W"
    Months -> "M"
    Years -> "Y"
  }

  age.number
  |> int.to_string
  |> utils.pad_start(3, "0")
  |> string.append(unit)
  |> bit_array.from_string
  |> bit_array_utils.pad_to_even_length(0x20)
  |> Ok
}
