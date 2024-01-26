import dcmfx_core/data_element_tag.{DataElementTag}
import dcmfx_core/data_element_value/attribute_tag
import dcmfx_core/data_error
import gleeunit/should

pub fn from_bytes_test() {
  <<>>
  |> attribute_tag.from_bytes
  |> should.equal(Ok([]))

  <<0x4810:16-little, 0x00FE:16-little, 0x3052:16-little, 0x9A41:16-little>>
  |> attribute_tag.from_bytes
  |> should.equal(
    Ok([DataElementTag(0x4810, 0x00FE), DataElementTag(0x3052, 0x9A41)]),
  )

  <<0, 1>>
  |> attribute_tag.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid(
      "AttributeTag data length is not a multiple of 4",
    )),
  )
}

pub fn to_bytes_test() {
  []
  |> attribute_tag.to_bytes
  |> should.equal(Ok(<<>>))

  [DataElementTag(0x4810, 0x00FE)]
  |> attribute_tag.to_bytes
  |> should.equal(Ok(<<0x4810:16-little, 0x00FE:16-little>>))

  [DataElementTag(0x4810, 0x00FE), DataElementTag(0x1234, 0x5678)]
  |> attribute_tag.to_bytes
  |> should.equal(
    Ok(<<
      0x4810:16-little, 0x00FE:16-little, 0x1234:16-little, 0x5678:16-little,
    >>),
  )
}
