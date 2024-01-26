import dcmfx_core/data_element_tag.{DataElementTag}
import dcmfx_core/data_set_path
import gleeunit/should

pub fn to_string_test() {
  let path = data_set_path.new()

  let assert Ok(path) =
    data_set_path.add_data_element(path, DataElementTag(0x1234, 0x5678))

  path
  |> data_set_path.to_string
  |> should.equal("12345678")

  data_set_path.add_data_element(path, DataElementTag(0x1234, 0x5678))
  |> should.equal(Error("Invalid data set path entry: 12345678"))

  let assert Ok(path) = data_set_path.add_sequence_item(path, 2)

  path
  |> data_set_path.to_string
  |> should.equal("12345678/[2]")

  data_set_path.add_sequence_item(path, 2)
  |> should.equal(Error("Invalid data set path entry: [2]"))

  let assert Ok(path) =
    data_set_path.add_data_element(path, DataElementTag(0x1122, 0x3344))

  path
  |> data_set_path.to_string
  |> should.equal("12345678/[2]/11223344")
}

pub fn from_string_test() {
  let path = data_set_path.new()

  ""
  |> data_set_path.from_string
  |> should.equal(Ok(path))

  let assert Ok(path) =
    data_set_path.add_data_element(path, DataElementTag(0x1234, 0x5678))

  "12345678"
  |> data_set_path.from_string
  |> should.equal(Ok(path))

  let assert Ok(path) = data_set_path.add_sequence_item(path, 2)

  "12345678/[2]"
  |> data_set_path.from_string
  |> should.equal(Ok(path))

  let assert Ok(path) =
    data_set_path.add_data_element(path, DataElementTag(0x1122, 0x3344))

  "12345678/[2]/11223344"
  |> data_set_path.from_string
  |> should.equal(Ok(path))
}
