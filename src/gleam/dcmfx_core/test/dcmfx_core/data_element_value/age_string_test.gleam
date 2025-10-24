import dcmfx_core/data_element_value/age_string.{StructuredAge}
import dcmfx_core/data_error

pub fn to_string_test() {
  assert age_string.to_string(StructuredAge(20, age_string.Days)) == "20 days"

  assert age_string.to_string(StructuredAge(3, age_string.Weeks)) == "3 weeks"

  assert age_string.to_string(StructuredAge(13, age_string.Months))
    == "13 months"

  assert age_string.to_string(StructuredAge(1, age_string.Years)) == "1 year"
}

pub fn from_bytes_test() {
  assert age_string.from_bytes(<<"101D">>)
    == Ok(StructuredAge(101, age_string.Days))

  assert age_string.from_bytes(<<"070W">>)
    == Ok(StructuredAge(70, age_string.Weeks))

  assert age_string.from_bytes(<<"009M">>)
    == Ok(StructuredAge(9, age_string.Months))

  assert age_string.from_bytes(<<"101Y">>)
    == Ok(StructuredAge(101, age_string.Years))

  assert age_string.from_bytes(<<>>)
    == Error(data_error.new_value_invalid("AgeString is invalid: ''"))

  assert age_string.from_bytes(<<0xD0>>)
    == Error(data_error.new_value_invalid("AgeString is invalid UTF-8"))

  assert age_string.from_bytes(<<"3 days">>)
    == Error(data_error.new_value_invalid("AgeString is invalid: '3 days'"))
}

pub fn to_bytes_test() {
  assert age_string.to_bytes(StructuredAge(101, age_string.Days))
    == Ok(<<"101D">>)

  assert age_string.to_bytes(StructuredAge(70, age_string.Weeks))
    == Ok(<<"070W">>)

  assert age_string.to_bytes(StructuredAge(9, age_string.Months))
    == Ok(<<"009M">>)

  assert age_string.to_bytes(StructuredAge(101, age_string.Years))
    == Ok(<<"101Y">>)

  assert age_string.to_bytes(StructuredAge(-1, age_string.Years))
    == Error(data_error.new_value_invalid(
      "AgeString value -1 is outside the valid range of 0-999",
    ))
}
