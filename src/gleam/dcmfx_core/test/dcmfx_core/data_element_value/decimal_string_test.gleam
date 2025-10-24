import dcmfx_core/data_element_value/decimal_string
import dcmfx_core/data_error

pub fn from_bytes_test() {
  assert decimal_string.from_bytes(<<>>) == Ok([])

  assert decimal_string.from_bytes(<<"  1.2">>) == Ok([1.2])

  assert decimal_string.from_bytes(<<"127.">>) == Ok([127.0])

  assert decimal_string.from_bytes(<<"-1024">>) == Ok([-1024.0])

  assert decimal_string.from_bytes(<<"  1.2\\4.5">>) == Ok([1.2, 4.5])

  assert decimal_string.from_bytes(<<"1.868344208e-10">>)
    == Ok([1.868344208e-10])

  assert decimal_string.from_bytes(<<"-0">>) == Ok([-0.0])

  assert decimal_string.from_bytes(<<0xD0>>)
    == Error(data_error.new_value_invalid("DecimalString is invalid UTF-8"))

  assert decimal_string.from_bytes(<<"1.A">>)
    == Error(data_error.new_value_invalid("DecimalString is invalid: '1.A'"))
}

pub fn to_bytes_test() {
  assert decimal_string.to_bytes([]) == <<>>

  assert decimal_string.to_bytes([0.0]) == <<"0 ">>

  assert decimal_string.to_bytes([1.2]) == <<"1.2 ">>

  assert decimal_string.to_bytes([1.2, 3.4]) == <<"1.2\\3.4 ">>

  assert decimal_string.to_bytes([1.868344208e-010]) == <<"1.868344208e-10 ">>

  assert decimal_string.to_bytes([1.123456789123456]) == <<"1.12345678912345">>
}
