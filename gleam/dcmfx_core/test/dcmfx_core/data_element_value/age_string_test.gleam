import dcmfx_core/data_element_value/age_string.{StructuredAge}
import dcmfx_core/data_error
import gleeunit/should

pub fn to_string_test() {
  StructuredAge(20, age_string.Days)
  |> age_string.to_string
  |> should.equal("20 days")

  StructuredAge(3, age_string.Weeks)
  |> age_string.to_string
  |> should.equal("3 weeks")

  StructuredAge(13, age_string.Months)
  |> age_string.to_string
  |> should.equal("13 months")

  StructuredAge(1, age_string.Years)
  |> age_string.to_string
  |> should.equal("1 year")
}

pub fn from_bytes_test() {
  <<"101D">>
  |> age_string.from_bytes
  |> should.equal(Ok(StructuredAge(101, age_string.Days)))

  <<"070W">>
  |> age_string.from_bytes
  |> should.equal(Ok(StructuredAge(70, age_string.Weeks)))

  <<"009M">>
  |> age_string.from_bytes
  |> should.equal(Ok(StructuredAge(9, age_string.Months)))

  <<"101Y">>
  |> age_string.from_bytes
  |> should.equal(Ok(StructuredAge(101, age_string.Years)))

  <<>>
  |> age_string.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("AgeString is invalid: ''")),
  )

  <<0xD0>>
  |> age_string.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("AgeString is invalid UTF-8")),
  )

  <<"3 days">>
  |> age_string.from_bytes()
  |> should.equal(
    Error(data_error.new_value_invalid("AgeString is invalid: '3 days'")),
  )
}

pub fn to_bytes_test() {
  StructuredAge(101, age_string.Days)
  |> age_string.to_bytes
  |> should.equal(Ok(<<"101D">>))

  StructuredAge(70, age_string.Weeks)
  |> age_string.to_bytes
  |> should.equal(Ok(<<"070W">>))

  StructuredAge(9, age_string.Months)
  |> age_string.to_bytes
  |> should.equal(Ok(<<"009M">>))

  StructuredAge(101, age_string.Years)
  |> age_string.to_bytes
  |> should.equal(Ok(<<"101Y">>))

  StructuredAge(-1, age_string.Years)
  |> age_string.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid(
      "AgeString value -1 is outside the valid range of 0-999",
    )),
  )
}
