//// Work with the `Time` value representation.

import dcmfx_core/data_error.{type DataError}
import dcmfx_core/internal/utils
import gleam/bit_array
import gleam/bool
import gleam/float
import gleam/int
import gleam/option.{type Option, None, Some}
import gleam/regex
import gleam/result

/// A structured time that can be converted from/to a `Time` data element value.
///
pub type StructuredTime {
  StructuredTime(hour: Int, minute: Option(Int), second: Option(Float))
}

/// Converts a `Time` value into a structured time.
///
pub fn from_bytes(bytes: BitArray) -> Result(StructuredTime, DataError) {
  let time_string =
    bytes
    |> bit_array.to_string
    |> result.map(utils.trim_right_whitespace)
    |> result.replace_error(data_error.new_value_invalid(
      "Time is invalid UTF-8",
    ))
  use time_string <- result.try(time_string)

  let assert Ok(re) =
    regex.from_string("^(\\d\\d)((\\d\\d)((\\d\\d)(\\.\\d{1,6})?)?)?$")

  case regex.scan(re, time_string) {
    [match] -> {
      let #(hour, minute, second) = case match.submatches {
        [Some(hour), _, minute, second, ..] -> #(hour, minute, second)
        [Some(hour), _, Some(minute), ..] -> #(hour, Some(minute), None)
        [Some(hour), ..] -> #(hour, None, None)
        _ -> #("0", None, None)
      }

      let assert Ok(hour) = int.parse(hour)

      let minute = case minute {
        Some(minute) -> option.from_result(int.parse(minute))
        None -> None
      }

      let second = case second {
        Some(second) -> option.from_result(utils.smart_parse_float(second))
        None -> None
      }

      Ok(StructuredTime(hour, minute, second))
    }
    _ ->
      Error(data_error.new_value_invalid(
        "Time is invalid: '" <> time_string <> "'",
      ))
  }
}

/// Converts a structured time to a `Time` value.
///
pub fn to_bytes(time: StructuredTime) -> Result(BitArray, DataError) {
  time
  |> to_string
  |> result.map(bit_array.from_string)
}

/// Returns the string value of a structured time.
///
pub fn to_string(value: StructuredTime) -> Result(String, DataError) {
  let has_second_without_minute =
    option.is_some(value.second) && !option.is_some(value.minute)
  use <- bool.guard(
    has_second_without_minute,
    Error(data_error.new_value_invalid(
      "Time minute value must be present when there is a second value",
    )),
  )

  // Validate and format the hour value
  let hour = case value.hour >= 0 && value.hour <= 23 {
    True ->
      value.hour
      |> int.to_string
      |> utils.pad_start(2, "0")
      |> Ok
    False ->
      Error(data_error.new_value_invalid(
        "Time hour value is invalid: " <> int.to_string(value.hour),
      ))
  }
  use hour <- result.try(hour)

  // Validate and format the minute value if present
  let minute = case value.minute {
    Some(minute) ->
      case minute >= 0 && minute <= 59 {
        True ->
          minute
          |> int.to_string
          |> utils.pad_start(2, "0")
          |> Ok
        False ->
          Error(data_error.new_value_invalid(
            "Time minute value is invalid: " <> int.to_string(minute),
          ))
      }
    None -> Ok("")
  }
  use minute <- result.try(minute)

  // Validate and format the second value if present. A second value of exactly
  // 60 is permitted in order to accommodate leap seconds.
  let second = case value.second {
    Some(second) ->
      case second >=. 0.0 && second <=. 60.0 {
        True -> Ok(format_second(second))
        False ->
          Error(data_error.new_value_invalid(
            "Time second value is invalid: " <> float.to_string(second),
          ))
      }
    None -> Ok("")
  }
  use second <- result.try(second)

  // Concatenate all the pieces of the time together
  Ok(hour <> minute <> second)
}

/// Formats a structured time as an ISO 8601 time. Components that aren't
/// specified are omitted.
///
pub fn to_iso8601(time: StructuredTime) -> String {
  let hour =
    time.hour
    |> int.to_string
    |> utils.pad_start(2, "0")

  case time.minute {
    Some(minute) -> {
      let minute =
        minute
        |> int.to_string
        |> utils.pad_start(2, "0")

      case time.second {
        Some(second) -> hour <> ":" <> minute <> ":" <> format_second(second)
        None -> hour <> ":" <> minute
      }
    }
    None -> hour
  }
}

/// Takes a number of seconds and formats it as `SS[.FFFFFF]` with two digits
/// for the whole number of seconds, and up to 6 digits for the fractional
/// seconds. The fractional seconds are only included if the number of seconds
/// is not an exact whole number.
///
fn format_second(seconds: Float) -> String {
  let whole_seconds =
    seconds
    |> float.floor
    |> float.round
    |> int.to_string
    |> utils.pad_start(2, "0")

  let fractional_seconds =
    { { seconds -. float.floor(seconds) } *. 1_000_000.0 }
    |> float.round

  case fractional_seconds {
    0 -> whole_seconds
    _ -> {
      let fractional_seconds =
        fractional_seconds
        |> int.to_string
        |> utils.trim_right("0")

      whole_seconds <> "." <> fractional_seconds
    }
  }
}
