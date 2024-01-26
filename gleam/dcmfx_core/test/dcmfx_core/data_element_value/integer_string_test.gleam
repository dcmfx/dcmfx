import dcmfx_core/data_element_value/integer_string
import dcmfx_core/data_error
import gleeunit/should

pub fn from_bytes_test() {
  <<>>
  |> integer_string.from_bytes
  |> should.equal(Ok([]))

  <<" ">>
  |> integer_string.from_bytes
  |> should.equal(Ok([]))

  <<" 1">>
  |> integer_string.from_bytes
  |> should.equal(Ok([1]))

  <<"  1\\2 ">>
  |> integer_string.from_bytes
  |> should.equal(Ok([1, 2]))

  <<0xD0>>
  |> integer_string.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("IntegerString is invalid UTF-8")),
  )

  <<"A">>
  |> integer_string.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("IntegerString is invalid: 'A'")),
  )
}

pub fn to_bytes_test() {
  []
  |> integer_string.to_bytes
  |> should.equal(Ok(<<>>))

  [1]
  |> integer_string.to_bytes
  |> should.equal(Ok(<<"1 ">>))

  [1, 2]
  |> integer_string.to_bytes
  |> should.equal(Ok(<<"1\\2 ">>))

  [1_234_567_891_234]
  |> integer_string.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("IntegerString value is out of range")),
  )
}
