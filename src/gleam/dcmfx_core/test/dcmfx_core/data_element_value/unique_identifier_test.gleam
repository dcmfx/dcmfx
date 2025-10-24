import dcmfx_core/data_element_value/unique_identifier
import dcmfx_core/data_error
import gleam/list
import gleam/result
import gleam/string

pub fn to_bytes_test() {
  let invalid_uid_error =
    Error(data_error.new_value_invalid("UniqueIdentifier is invalid"))

  assert unique_identifier.to_bytes([]) == Ok(<<>>)

  assert unique_identifier.to_bytes([""]) == invalid_uid_error

  assert unique_identifier.to_bytes(["1.0"]) == Ok(<<"1.0", 0>>)

  assert unique_identifier.to_bytes(["1.2", "3.4"]) == Ok(<<"1.2\\3.4", 0>>)

  assert unique_identifier.to_bytes(["1.00"]) == invalid_uid_error

  assert unique_identifier.to_bytes([string.repeat("1", 65)])
    == invalid_uid_error
}

pub fn new_test() {
  list.range(0, 1000)
  |> list.each(fn(_) {
    assert result.map(unique_identifier.new(""), unique_identifier.is_valid)
      == Ok(True)

    assert result.map(
        unique_identifier.new("1111.2222"),
        unique_identifier.is_valid,
      )
      == Ok(True)
  })

  assert result.map(
      unique_identifier.new(string.repeat("1", 60)),
      unique_identifier.is_valid,
    )
    == Ok(True)

  let assert Ok(uid) = unique_identifier.new("1111.2222")
  assert string.starts_with(uid, "1111.2222")
  assert string.length(uid) == 64

  assert unique_identifier.new(string.repeat("1", 61)) == Error(Nil)

  assert unique_identifier.new("1.") == Error(Nil)
}
