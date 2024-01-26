import dcmfx_core/data_element_value/unique_identifier
import dcmfx_core/data_error
import gleam/iterator
import gleam/result
import gleam/string
import gleeunit/should

pub fn to_bytes_test() {
  let invalid_uid_error =
    Error(data_error.new_value_invalid("UniqueIdentifier is invalid"))

  []
  |> unique_identifier.to_bytes
  |> should.equal(Ok(<<>>))

  [""]
  |> unique_identifier.to_bytes
  |> should.equal(invalid_uid_error)

  ["1.0"]
  |> unique_identifier.to_bytes
  |> should.equal(Ok(<<"1.0", 0>>))

  ["1.2", "3.4"]
  |> unique_identifier.to_bytes
  |> should.equal(Ok(<<"1.2\\3.4", 0>>))

  ["1.00"]
  |> unique_identifier.to_bytes
  |> should.equal(invalid_uid_error)

  [string.repeat("1", 65)]
  |> unique_identifier.to_bytes
  |> should.equal(invalid_uid_error)
}

pub fn new_test() {
  iterator.range(0, 1000)
  |> iterator.each(fn(_) {
    unique_identifier.new("")
    |> result.map(unique_identifier.is_valid)
    |> should.equal(Ok(True))

    unique_identifier.new("1111.2222")
    |> result.map(unique_identifier.is_valid)
    |> should.equal(Ok(True))
  })

  unique_identifier.new(string.repeat("1", 60))
  |> result.map(unique_identifier.is_valid)
  |> should.equal(Ok(True))

  let assert Ok(uid) = unique_identifier.new("1111.2222")
  string.starts_with(uid, "1111.2222")
  |> should.be_true
  string.length(uid)
  |> should.equal(64)

  unique_identifier.new(string.repeat("1", 61))
  |> should.equal(Error(Nil))

  unique_identifier.new("1.")
  |> should.equal(Error(Nil))
}
