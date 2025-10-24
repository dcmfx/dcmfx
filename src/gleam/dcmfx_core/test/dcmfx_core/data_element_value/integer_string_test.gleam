import dcmfx_core/data_element_value/integer_string
import dcmfx_core/data_error

pub fn from_bytes_test() {
  assert integer_string.from_bytes(<<>>) == Ok([])

  assert integer_string.from_bytes(<<" ">>) == Ok([])

  assert integer_string.from_bytes(<<" 1">>) == Ok([1])

  assert integer_string.from_bytes(<<"  1\\2 ">>) == Ok([1, 2])

  assert integer_string.from_bytes(<<0xD0>>)
    == Error(data_error.new_value_invalid("IntegerString is invalid UTF-8"))

  assert integer_string.from_bytes(<<"A">>)
    == Error(data_error.new_value_invalid("IntegerString is invalid: 'A'"))
}

pub fn to_bytes_test() {
  assert integer_string.to_bytes([]) == Ok(<<>>)

  assert integer_string.to_bytes([1]) == Ok(<<"1 ">>)

  assert integer_string.to_bytes([1, 2]) == Ok(<<"1\\2 ">>)

  assert integer_string.to_bytes([1_234_567_891_234])
    == Error(data_error.new_value_invalid("IntegerString value is out of range"))
}
