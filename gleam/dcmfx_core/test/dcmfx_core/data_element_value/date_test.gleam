import dcmfx_core/data_element_value/date.{StructuredDate}
import dcmfx_core/data_error
import gleeunit/should

pub fn to_string_test() {
  StructuredDate(2024, 7, 2)
  |> date.to_iso8601
  |> should.equal("2024-07-02")
}

pub fn from_bytes_test() {
  <<"20000102">>
  |> date.from_bytes
  |> should.equal(Ok(date.StructuredDate(2000, 1, 2)))

  <<0xD0>>
  |> date.from_bytes
  |> should.equal(Error(data_error.new_value_invalid("Date is invalid UTF-8")))

  <<>>
  |> date.from_bytes
  |> should.equal(Error(data_error.new_value_invalid("Date is invalid: ''")))

  <<"2024">>
  |> date.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("Date is invalid: '2024'")),
  )
}

pub fn to_bytes_test() {
  date.StructuredDate(2000, 1, 2)
  |> date.to_bytes
  |> should.equal(Ok(<<"20000102">>))

  date.StructuredDate(-1, 1, 2)
  |> date.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("Date's year is invalid: -1")),
  )

  date.StructuredDate(0, 13, 2)
  |> date.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("Date's month is invalid: 13")),
  )

  date.StructuredDate(100, 1, 32)
  |> date.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("Date's day is invalid: 32")),
  )
}
