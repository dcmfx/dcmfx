import dcmfx_core/data_element_tag.{DataElementTag}
import dcmfx_core/data_element_value/attribute_tag
import dcmfx_core/data_error

pub fn from_bytes_test() {
  assert attribute_tag.from_bytes(<<>>) == Ok([])

  assert attribute_tag.from_bytes(<<
      0x4810:16-little,
      0x00FE:16-little,
      0x3052:16-little,
      0x9A41:16-little,
    >>)
    == Ok([DataElementTag(0x4810, 0x00FE), DataElementTag(0x3052, 0x9A41)])

  assert attribute_tag.from_bytes(<<0, 1>>)
    == Error(data_error.new_value_invalid(
      "AttributeTag data length is not a multiple of 4",
    ))
}

pub fn to_bytes_test() {
  assert attribute_tag.to_bytes([]) == Ok(<<>>)

  assert attribute_tag.to_bytes([DataElementTag(0x4810, 0x00FE)])
    == Ok(<<0x4810:16-little, 0x00FE:16-little>>)

  assert attribute_tag.to_bytes([
      DataElementTag(0x4810, 0x00FE),
      DataElementTag(0x1234, 0x5678),
    ])
    == Ok(<<
      0x4810:16-little, 0x00FE:16-little, 0x1234:16-little, 0x5678:16-little,
    >>)
}
