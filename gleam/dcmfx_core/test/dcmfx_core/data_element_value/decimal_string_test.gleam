import dcmfx_core/data_element_value/decimal_string
import dcmfx_core/data_error
import gleeunit/should

pub fn from_bytes_test() {
  <<>>
  |> decimal_string.from_bytes
  |> should.equal(Ok([]))

  <<"  1.2">>
  |> decimal_string.from_bytes
  |> should.equal(Ok([1.2]))

  <<"127.">>
  |> decimal_string.from_bytes
  |> should.equal(Ok([127.0]))

  <<"-1024">>
  |> decimal_string.from_bytes
  |> should.equal(Ok([-1024.0]))

  <<"  1.2\\4.5">>
  |> decimal_string.from_bytes
  |> should.equal(Ok([1.2, 4.5]))

  <<"1.868344208e-10">>
  |> decimal_string.from_bytes
  |> should.equal(Ok([1.868344208e-10]))

  <<"-0">>
  |> decimal_string.from_bytes
  |> should.equal(Ok([-0.0]))

  <<0xD0>>
  |> decimal_string.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("DecimalString is invalid UTF-8")),
  )

  <<"1.A">>
  |> decimal_string.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("DecimalString is invalid: '1.A'")),
  )
}

pub fn to_bytes_test() {
  []
  |> decimal_string.to_bytes
  |> should.equal(<<>>)

  [0.0]
  |> decimal_string.to_bytes
  |> should.equal(<<"0 ">>)

  [1.2]
  |> decimal_string.to_bytes
  |> should.equal(<<"1.2 ">>)

  [1.2, 3.4]
  |> decimal_string.to_bytes
  |> should.equal(<<"1.2\\3.4 ">>)

  [1.868344208e-010]
  |> decimal_string.to_bytes
  |> should.equal(<<"1.868344208e-10 ">>)

  [1.123456789123456]
  |> decimal_string.to_bytes
  |> should.equal(<<"1.12345678912345">>)
}
