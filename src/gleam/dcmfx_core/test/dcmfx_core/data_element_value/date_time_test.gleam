import dcmfx_core/data_element_value/date_time.{StructuredDateTime}
import dcmfx_core/data_error
import gleam/option.{None, Some}

pub fn to_string_test() {
  assert date_time.to_iso8601(StructuredDateTime(
      year: 2024,
      month: Some(7),
      day: Some(2),
      hour: Some(9),
      minute: Some(40),
      second: Some(2.5),
      time_zone_offset: Some(-400),
    ))
    == "2024-07-02T09:40:02.5-0400"

  assert date_time.to_iso8601(StructuredDateTime(
      year: 2024,
      month: Some(7),
      day: Some(2),
      hour: Some(9),
      minute: None,
      second: None,
      time_zone_offset: Some(200),
    ))
    == "2024-07-02T09+0200"
}

pub fn from_bytes_test() {
  assert date_time.from_bytes(<<"1997">>)
    == Ok(date_time.StructuredDateTime(1997, None, None, None, None, None, None))

  assert date_time.from_bytes(<<"1997070421-0500">>)
    == Ok(date_time.StructuredDateTime(
      1997,
      Some(7),
      Some(4),
      Some(21),
      None,
      None,
      Some(-500),
    ))

  assert date_time.from_bytes(<<"19970704213000-0500">>)
    == Ok(date_time.StructuredDateTime(
      1997,
      Some(7),
      Some(4),
      Some(21),
      Some(30),
      Some(0.0),
      Some(-500),
    ))

  assert date_time.from_bytes(<<"10pm">>)
    == Error(data_error.new_value_invalid("DateTime is invalid: '10pm'"))

  assert date_time.from_bytes(<<0xD0>>)
    == Error(data_error.new_value_invalid("DateTime is invalid UTF-8"))
}

pub fn to_bytes_test() {
  assert date_time.to_bytes(date_time.StructuredDateTime(
      1997,
      Some(7),
      Some(4),
      Some(21),
      Some(30),
      Some(0.0),
      Some(-500),
    ))
    == Ok(<<"19970704213000-0500 ">>)

  assert date_time.to_bytes(date_time.StructuredDateTime(
      1997,
      Some(7),
      Some(4),
      None,
      None,
      None,
      None,
    ))
    == Ok(<<"19970704">>)

  assert date_time.to_bytes(date_time.StructuredDateTime(
      1997,
      None,
      None,
      None,
      None,
      None,
      Some(100),
    ))
    == Ok(<<"1997+0100 ">>)

  assert date_time.to_bytes(date_time.StructuredDateTime(
      1997,
      Some(1),
      None,
      Some(1),
      None,
      None,
      None,
    ))
    == Error(data_error.new_value_invalid(
      "DateTime day value must be present when there is an hour value",
    ))

  assert date_time.to_bytes(date_time.StructuredDateTime(
      1997,
      None,
      Some(1),
      None,
      None,
      None,
      None,
    ))
    == Error(data_error.new_value_invalid(
      "Date's month must be present when there is a day value",
    ))

  assert date_time.to_bytes(date_time.StructuredDateTime(
      1997,
      Some(1),
      Some(1),
      Some(30),
      None,
      None,
      None,
    ))
    == Error(data_error.new_value_invalid("Time hour value is invalid: 30"))

  assert date_time.to_bytes(date_time.StructuredDateTime(
      1997,
      None,
      None,
      None,
      None,
      None,
      Some(2000),
    ))
    == Error(data_error.new_value_invalid(
      "DateTime time zone offset is invalid: 2000",
    ))
}
