import dcmfx_core/data_element_value/time.{StructuredTime}
import dcmfx_core/data_error
import gleam/option.{None, Some}
import gleeunit/should

pub fn to_string_test() {
  StructuredTime(1, Some(2), Some(3.289))
  |> time.to_iso8601
  |> should.equal("01:02:03.289")

  StructuredTime(1, Some(2), Some(3.0))
  |> time.to_iso8601
  |> should.equal("01:02:03")

  StructuredTime(1, Some(2), None)
  |> time.to_iso8601
  |> should.equal("01:02")

  StructuredTime(1, None, None)
  |> time.to_iso8601
  |> should.equal("01")
}

pub fn from_bytes_test() {
  <<"010203.289">>
  |> time.from_bytes
  |> should.equal(Ok(time.StructuredTime(1, Some(2), Some(3.289))))

  <<"1115">>
  |> time.from_bytes
  |> should.equal(Ok(time.StructuredTime(11, Some(15), None)))

  <<"14">>
  |> time.from_bytes
  |> should.equal(Ok(time.StructuredTime(14, None, None)))

  <<0xD0>>
  |> time.from_bytes
  |> should.equal(Error(data_error.new_value_invalid("Time is invalid UTF-8")))

  <<"10pm">>
  |> time.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("Time is invalid: '10pm'")),
  )
}

pub fn to_bytes_test() {
  time.StructuredTime(1, Some(2), Some(3.289))
  |> time.to_bytes
  |> should.equal(Ok(<<"010203.289">>))

  time.StructuredTime(1, Some(2), Some(3.0))
  |> time.to_bytes
  |> should.equal(Ok(<<"010203">>))

  time.StructuredTime(23, None, None)
  |> time.to_bytes
  |> should.equal(Ok(<<"23">>))

  time.StructuredTime(23, Some(14), None)
  |> time.to_bytes
  |> should.equal(Ok(<<"2314">>))

  time.StructuredTime(23, None, Some(1.0))
  |> time.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid(
      "Time minute value must be present when there is a second value",
    )),
  )

  time.StructuredTime(-1, None, None)
  |> time.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("Time hour value is invalid: -1")),
  )

  time.StructuredTime(0, Some(-1), None)
  |> time.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("Time minute value is invalid: -1")),
  )

  time.StructuredTime(0, Some(0), Some(-1.0))
  |> time.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("Time second value is invalid: -1.0")),
  )
}
