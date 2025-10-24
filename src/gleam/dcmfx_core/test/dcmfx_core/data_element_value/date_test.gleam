import dcmfx_core/data_element_value/date.{StructuredDate}
import dcmfx_core/data_error

pub fn to_string_test() {
  assert date.to_iso8601(StructuredDate(2024, 7, 2)) == "2024-07-02"
}

pub fn from_bytes_test() {
  assert date.from_bytes(<<"20000102">>) == Ok(date.StructuredDate(2000, 1, 2))

  assert date.from_bytes(<<0xD0>>)
    == Error(data_error.new_value_invalid("Date is invalid UTF-8"))

  assert date.from_bytes(<<>>)
    == Error(data_error.new_value_invalid("Date is invalid: ''"))

  assert date.from_bytes(<<"2024">>)
    == Error(data_error.new_value_invalid("Date is invalid: '2024'"))
}

pub fn to_bytes_test() {
  assert date.to_bytes(date.StructuredDate(2000, 1, 2)) == Ok(<<"20000102">>)

  assert date.to_bytes(date.StructuredDate(-1, 1, 2))
    == Error(data_error.new_value_invalid("Date's year is invalid: -1"))

  assert date.to_bytes(date.StructuredDate(0, 13, 2))
    == Error(data_error.new_value_invalid("Date's month is invalid: 13"))

  assert date.to_bytes(date.StructuredDate(100, 1, 32))
    == Error(data_error.new_value_invalid("Date's day is invalid: 32"))
}
