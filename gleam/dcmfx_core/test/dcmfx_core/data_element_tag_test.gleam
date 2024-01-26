import dcmfx_core/data_element_tag.{DataElementTag}
import gleeunit/should

pub fn is_private_test() {
  data_element_tag.is_private(DataElementTag(0x0001, 0))
  |> should.equal(True)

  data_element_tag.is_private(DataElementTag(0x0002, 1))
  |> should.equal(False)
}

pub fn is_private_creator_test() {
  data_element_tag.is_private_creator(DataElementTag(0x0001, 0x0010))
  |> should.equal(True)

  data_element_tag.is_private_creator(DataElementTag(0x0001, 0x00FF))
  |> should.equal(True)

  data_element_tag.is_private_creator(DataElementTag(0x0001, 0x000F))
  |> should.equal(False)
}

pub fn to_int_test() {
  data_element_tag.to_int(DataElementTag(0x1122, 0x3344))
  |> should.equal(0x11223344)
}

pub fn to_string_test() {
  data_element_tag.to_string(DataElementTag(0x1122, 0xAABB))
  |> should.equal("(1122,AABB)")
}

pub fn to_hex_string_test() {
  data_element_tag.to_hex_string(DataElementTag(0x1122, 0xAABB))
  |> should.equal("1122AABB")
}

pub fn from_hex_string_test() {
  data_element_tag.from_hex_string("1122AABB")
  |> should.equal(Ok(DataElementTag(0x1122, 0xAABB)))

  data_element_tag.from_hex_string("1122334")
  |> should.equal(Error(Nil))
}
