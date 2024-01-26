import dcmfx_core/data_element_value/date_time.{StructuredDateTime}
import dcmfx_core/data_error
import gleam/option.{None, Some}
import gleeunit/should

pub fn to_string_test() {
  StructuredDateTime(
    year: 2024,
    month: Some(7),
    day: Some(2),
    hour: Some(9),
    minute: Some(40),
    second: Some(2.5),
    time_zone_offset: Some(-400),
  )
  |> date_time.to_iso8601
  |> should.equal("2024-07-02T09:40:02.5-0400")

  StructuredDateTime(
    year: 2024,
    month: Some(7),
    day: Some(2),
    hour: Some(9),
    minute: None,
    second: None,
    time_zone_offset: Some(200),
  )
  |> date_time.to_iso8601
  |> should.equal("2024-07-02T09+0200")
}

pub fn from_bytes_test() {
  <<"1997">>
  |> date_time.from_bytes
  |> should.equal(
    Ok(date_time.StructuredDateTime(1997, None, None, None, None, None, None)),
  )

  <<"1997070421-0500">>
  |> date_time.from_bytes
  |> should.equal(
    Ok(date_time.StructuredDateTime(
      1997,
      Some(7),
      Some(4),
      Some(21),
      None,
      None,
      Some(-500),
    )),
  )

  <<"19970704213000-0500">>
  |> date_time.from_bytes
  |> should.equal(
    Ok(date_time.StructuredDateTime(
      1997,
      Some(7),
      Some(4),
      Some(21),
      Some(30),
      Some(0.0),
      Some(-500),
    )),
  )

  <<"10pm">>
  |> date_time.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("DateTime is invalid: '10pm'")),
  )

  <<0xD0>>
  |> date_time.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("DateTime is invalid UTF-8")),
  )
}

pub fn to_bytes_test() {
  date_time.StructuredDateTime(
    1997,
    Some(7),
    Some(4),
    Some(21),
    Some(30),
    Some(0.0),
    Some(-500),
  )
  |> date_time.to_bytes
  |> should.equal(Ok(<<"19970704213000-0500 ">>))

  date_time.StructuredDateTime(1997, Some(7), Some(4), None, None, None, None)
  |> date_time.to_bytes
  |> should.equal(Ok(<<"19970704">>))

  date_time.StructuredDateTime(1997, None, None, None, None, None, Some(100))
  |> date_time.to_bytes
  |> should.equal(Ok(<<"1997+0100 ">>))

  date_time.StructuredDateTime(1997, Some(1), None, Some(1), None, None, None)
  |> date_time.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid(
      "DateTime day value must be present when there is an hour value",
    )),
  )

  date_time.StructuredDateTime(1997, None, Some(1), None, None, None, None)
  |> date_time.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid(
      "Date's month must be present when there is a day value",
    )),
  )

  date_time.StructuredDateTime(
    1997,
    Some(1),
    Some(1),
    Some(30),
    None,
    None,
    None,
  )
  |> date_time.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("Time hour value is invalid: 30")),
  )

  date_time.StructuredDateTime(1997, None, None, None, None, None, Some(2000))
  |> date_time.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid(
      "DateTime time zone offset is invalid: 2000",
    )),
  )
}
