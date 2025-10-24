import dcmfx_core/data_element_tag.{DataElementTag}

pub fn is_private_test() {
  assert data_element_tag.is_private(DataElementTag(0x0001, 0))
  assert !data_element_tag.is_private(DataElementTag(0x0002, 1))
}

pub fn is_private_creator_test() {
  assert data_element_tag.is_private_creator(DataElementTag(0x0001, 0x0010))
  assert data_element_tag.is_private_creator(DataElementTag(0x0001, 0x00FF))
  assert !data_element_tag.is_private_creator(DataElementTag(0x0001, 0x000F))
}

pub fn to_int_test() {
  assert data_element_tag.to_int(DataElementTag(0x1122, 0x3344)) == 0x11223344
}

pub fn to_string_test() {
  assert data_element_tag.to_string(DataElementTag(0x1122, 0xAABB))
    == "(1122,AABB)"
}

pub fn to_hex_string_test() {
  assert data_element_tag.to_hex_string(DataElementTag(0x1122, 0xAABB))
    == "1122AABB"
}

pub fn from_hex_string_test() {
  assert data_element_tag.from_hex_string("1122AABB")
    == Ok(DataElementTag(0x1122, 0xAABB))

  assert data_element_tag.from_hex_string("1122334") == Error(Nil)
}
