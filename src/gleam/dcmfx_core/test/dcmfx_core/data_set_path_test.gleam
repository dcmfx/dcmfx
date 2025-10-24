import dcmfx_core/data_element_tag.{DataElementTag}
import dcmfx_core/data_set_path

pub fn to_string_test() {
  let path = data_set_path.new()

  let assert Ok(path) =
    data_set_path.add_data_element(path, DataElementTag(0x1234, 0x5678))

  assert data_set_path.to_string(path) == "12345678"

  assert data_set_path.add_data_element(path, DataElementTag(0x1234, 0x5678))
    == Error("Invalid data set path entry: 12345678")

  let assert Ok(path) = data_set_path.add_sequence_item(path, 2)

  assert data_set_path.to_string(path) == "12345678/[2]"

  assert data_set_path.add_sequence_item(path, 2)
    == Error("Invalid data set path entry: [2]")

  let assert Ok(path) =
    data_set_path.add_data_element(path, DataElementTag(0x1122, 0x3344))

  assert data_set_path.to_string(path) == "12345678/[2]/11223344"
}

pub fn from_string_test() {
  let path = data_set_path.new()

  assert data_set_path.from_string("") == Ok(path)

  let assert Ok(path) =
    data_set_path.add_data_element(path, DataElementTag(0x1234, 0x5678))

  assert data_set_path.from_string("12345678") == Ok(path)

  let assert Ok(path) = data_set_path.add_sequence_item(path, 2)

  assert data_set_path.from_string("12345678/[2]") == Ok(path)

  let assert Ok(path) =
    data_set_path.add_data_element(path, DataElementTag(0x1122, 0x3344))

  assert data_set_path.from_string("12345678/[2]/11223344") == Ok(path)
}
