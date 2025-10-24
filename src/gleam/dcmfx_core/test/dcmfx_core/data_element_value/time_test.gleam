import dcmfx_core/data_element_value/time.{StructuredTime}
import dcmfx_core/data_error
import gleam/option.{None, Some}

pub fn to_string_test() {
  assert time.to_iso8601(StructuredTime(1, Some(2), Some(3.289)))
    == "01:02:03.289"

  assert time.to_iso8601(StructuredTime(1, Some(2), Some(3.0))) == "01:02:03"

  assert time.to_iso8601(StructuredTime(1, Some(2), None)) == "01:02"

  assert time.to_iso8601(StructuredTime(1, None, None)) == "01"
}

pub fn from_bytes_test() {
  assert time.from_bytes(<<"010203.289">>)
    == Ok(time.StructuredTime(1, Some(2), Some(3.289)))

  assert time.from_bytes(<<"1115">>)
    == Ok(time.StructuredTime(11, Some(15), None))

  assert time.from_bytes(<<"14">>) == Ok(time.StructuredTime(14, None, None))

  assert time.from_bytes(<<0xD0>>)
    == Error(data_error.new_value_invalid("Time is invalid UTF-8"))

  assert time.from_bytes(<<"10pm">>)
    == Error(data_error.new_value_invalid("Time is invalid: '10pm'"))
}

pub fn to_bytes_test() {
  assert time.to_bytes(time.StructuredTime(1, Some(2), Some(3.289)))
    == Ok(<<"010203.289">>)

  assert time.to_bytes(time.StructuredTime(1, Some(2), Some(3.0)))
    == Ok(<<"010203">>)

  assert time.to_bytes(time.StructuredTime(23, None, None)) == Ok(<<"23">>)

  assert time.to_bytes(time.StructuredTime(23, Some(14), None))
    == Ok(<<"2314">>)

  assert time.to_bytes(time.StructuredTime(23, None, Some(1.0)))
    == Error(data_error.new_value_invalid(
      "Time minute value must be present when there is a second value",
    ))

  assert time.to_bytes(time.StructuredTime(-1, None, None))
    == Error(data_error.new_value_invalid("Time hour value is invalid: -1"))

  assert time.to_bytes(time.StructuredTime(0, Some(-1), None))
    == Error(data_error.new_value_invalid("Time minute value is invalid: -1"))

  assert time.to_bytes(time.StructuredTime(0, Some(0), Some(-1.0)))
    == Error(data_error.new_value_invalid("Time second value is invalid: -1.0"))
}
