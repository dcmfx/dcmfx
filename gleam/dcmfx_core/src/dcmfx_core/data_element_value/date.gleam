//// Work with the DICOM `Date` value representation.

import dcmfx_core/data_error.{type DataError}
import dcmfx_core/internal/utils
import gleam/bit_array
import gleam/bool
import gleam/int
import gleam/option.{type Option, None, Some}
import gleam/regex
import gleam/result

/// A structured date that can be converted to/from a `Date` value.
///
pub type StructuredDate {
  StructuredDate(year: Int, month: Int, day: Int)
}

/// Converts a `Date` value into a structured date.
///
pub fn from_bytes(bytes: BitArray) -> Result(StructuredDate, DataError) {
  let date_string =
    bytes
    |> bit_array.to_string
    |> result.map(utils.trim_right_whitespace)
    |> result.replace_error(data_error.new_value_invalid(
      "Date is invalid UTF-8",
    ))
  use date_string <- result.try(date_string)

  let assert Ok(re) = regex.from_string("^(\\d{4})(\\d\\d)(\\d\\d)$")

  case regex.scan(re, date_string) {
    [regex.Match(submatches: [Some(year), Some(month), Some(day)], ..)] -> {
      let assert Ok(year) = int.parse(year)
      let assert Ok(month) = int.parse(month)
      let assert Ok(day) = int.parse(day)

      Ok(StructuredDate(year, month, day))
    }
    _ ->
      Error(data_error.new_value_invalid(
        "Date is invalid: '" <> date_string <> "'",
      ))
  }
}

/// Converts a structured date to a `Date` value.
///
pub fn to_bytes(value: StructuredDate) -> Result(BitArray, DataError) {
  components_to_string(value.year, Some(value.month), Some(value.day))
  |> result.map(bit_array.from_string)
}

/// Builds the content of a `Date` data element value where both the month and
/// day are optional. The month value is required if there is a day specified.
///
@internal
pub fn components_to_string(
  year: Int,
  month: Option(Int),
  day: Option(Int),
) -> Result(String, DataError) {
  let has_day_without_month = option.is_some(day) && !option.is_some(month)
  use <- bool.guard(
    has_day_without_month,
    Error(data_error.new_value_invalid(
      "Date's month must be present when there is a day value",
    )),
  )

  // Validate and format the year value
  let year = case year >= 0 && year <= 9999 {
    True ->
      year
      |> int.to_string
      |> utils.pad_start(4, "0")
      |> Ok

    False ->
      Error(data_error.new_value_invalid(
        "Date's year is invalid: " <> int.to_string(year),
      ))
  }
  use year <- result.try(year)

  // Validate and format the month value if present
  let month = case month {
    Some(month) ->
      case month >= 1 && month <= 12 {
        True ->
          month
          |> int.to_string
          |> utils.pad_start(2, "0")
          |> Ok
        False ->
          Error(data_error.new_value_invalid(
            "Date's month is invalid: " <> int.to_string(month),
          ))
      }
    None -> Ok("")
  }
  use month <- result.try(month)

  // Validate and format the day value if present
  let day = case day {
    Some(day) ->
      case day >= 1 && day <= 31 {
        True ->
          day
          |> int.to_string
          |> utils.pad_start(2, "0")
          |> Ok
        False ->
          Error(data_error.new_value_invalid(
            "Date's day is invalid: " <> int.to_string(day),
          ))
      }
    None -> Ok("")
  }
  use day <- result.try(day)

  Ok(year <> month <> day)
}

/// Formats a structured date as an ISO 8601 date.
///
pub fn to_iso8601(date: StructuredDate) -> String {
  let year =
    date.year
    |> int.to_string
    |> utils.pad_start(4, "0")

  let month =
    date.month
    |> int.to_string
    |> utils.pad_start(2, "0")

  let day =
    date.day
    |> int.to_string
    |> utils.pad_start(2, "0")

  year <> "-" <> month <> "-" <> day
}
