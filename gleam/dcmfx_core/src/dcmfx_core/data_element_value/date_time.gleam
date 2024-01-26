//// Work with the DICOM `DateTime` value representation.

import dcmfx_core/data_element_value/date
import dcmfx_core/data_element_value/time.{StructuredTime}
import dcmfx_core/data_error.{type DataError}
import dcmfx_core/internal/bit_array_utils
import dcmfx_core/internal/utils
import gleam/bit_array
import gleam/bool
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/regex
import gleam/result
import gleam/string

/// A structured date/time that can be converted to/from a `DateTime` value.
///
pub type StructuredDateTime {
  StructuredDateTime(
    year: Int,
    month: Option(Int),
    day: Option(Int),
    hour: Option(Int),
    minute: Option(Int),
    second: Option(Float),
    time_zone_offset: Option(Int),
  )
}

/// Converts a `DateTime` value into a structured date/time.
///
pub fn from_bytes(bytes: BitArray) -> Result(StructuredDateTime, DataError) {
  let date_time_string =
    bytes
    |> bit_array.to_string
    |> result.map(utils.trim_right_whitespace)
    |> result.replace_error(data_error.new_value_invalid(
      "DateTime is invalid UTF-8",
    ))
  use date_time_string <- result.try(date_time_string)

  let assert Ok(re) =
    regex.from_string(
      "^(\\d{4})((\\d{2})((\\d{2})((\\d{2})((\\d{2})((\\d{2})(\\.\\d{1,6})?)?)?)?)?)?([\\+\\-]\\d{4})?$",
    )

  case regex.scan(re, date_time_string) {
    [match, ..] -> {
      // Pad the submatches list to a length of 13 so that all the relevant
      // ones can be more easily matched against and extracted
      let submatches =
        list.append(
          match.submatches,
          list.repeat(None, 13 - list.length(match.submatches)),
        )

      let #(year, month, day, hour, minute, second, offset) = case submatches {
        [Some(year), _, month, _, day, _, hour, _, minute, second, _, _, offset] -> #(
          year,
          month,
          day,
          hour,
          minute,
          second,
          offset,
        )

        _ -> #("0", None, None, None, None, None, None)
      }

      // Parse all the values to the final types
      let assert Ok(year) = int.parse(year)

      let parse_int = fn(value: Option(String)) -> Option(Int) {
        case value {
          Some(value) -> option.from_result(int.parse(value))
          None -> None
        }
      }

      let month = parse_int(month)
      let day = parse_int(day)
      let hour = parse_int(hour)
      let minute = parse_int(minute)
      let offset = parse_int(offset)

      let second = case second {
        Some(second) -> option.from_result(utils.smart_parse_float(second))
        None -> None
      }

      Ok(StructuredDateTime(year, month, day, hour, minute, second, offset))
    }

    _ ->
      Error(data_error.new_value_invalid(
        "DateTime is invalid: '" <> date_time_string <> "'",
      ))
  }
}

/// Converts a structured date/time to a `DateTime` value.
///
pub fn to_bytes(value: StructuredDateTime) -> Result(BitArray, DataError) {
  let has_hour_without_day =
    option.is_some(value.hour) && !option.is_some(value.day)
  use <- bool.guard(
    has_hour_without_day,
    Error(data_error.new_value_invalid(
      "DateTime day value must be present when there is an hour value",
    )),
  )

  // Validate and format the date
  let date = date.components_to_string(value.year, value.month, value.day)
  use date <- result.try(date)

  // Validate and format the time if present
  let time = case value.hour {
    Some(hour) ->
      time.to_string(StructuredTime(hour, value.minute, value.second))
    _ -> Ok("")
  }
  use time <- result.try(time)

  // Validate and format the time zone offset if present
  let time_zone_offset = case value.time_zone_offset {
    Some(offset) -> {
      let is_offset_valid =
        offset >= -1200 && offset <= 1400 && { offset % 100 < 60 }

      case is_offset_valid {
        True -> {
          let sign = case offset < 0 {
            True -> "-"
            False -> "+"
          }

          offset
          |> int.absolute_value
          |> int.to_string
          |> utils.pad_start(4, "0")
          |> string.append(sign, _)
          |> Ok
        }
        False ->
          Error(data_error.new_value_invalid(
            "DateTime time zone offset is invalid: " <> int.to_string(offset),
          ))
      }
    }

    None -> Ok("")
  }
  use time_zone_offset <- result.try(time_zone_offset)

  { date <> time <> time_zone_offset }
  |> bit_array.from_string
  |> bit_array_utils.pad_to_even_length(0x20)
  |> Ok
}

/// Formats a structured date/time as an ISO 8601 string. Components that
/// aren't specified are omitted.
///
pub fn to_iso8601(date_time: StructuredDateTime) -> String {
  let s = date_time.year |> int.to_string |> utils.pad_start(4, "0")

  let s = case date_time.month {
    Some(month) -> {
      let s = s <> "-" <> { month |> int.to_string |> utils.pad_start(2, "0") }

      case date_time.day {
        Some(day) -> {
          let s =
            s <> "-" <> { day |> int.to_string |> utils.pad_start(2, "0") }

          case date_time.hour {
            Some(hour) ->
              s
              <> "T"
              <> time.to_iso8601(StructuredTime(
                hour,
                date_time.minute,
                date_time.second,
              ))

            None -> s
          }
        }
        None -> s
      }
    }

    None -> s
  }

  case date_time.time_zone_offset {
    Some(offset) -> {
      let sign = case offset < 0 {
        True -> "-"
        False -> "+"
      }

      let value =
        offset |> int.absolute_value |> int.to_string |> utils.pad_start(4, "0")

      s <> sign <> value
    }
    None -> s
  }
}
