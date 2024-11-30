import dcmfx_core/registry
import dcmfx_core/transfer_syntax
import dcmfx_core/value_representation
import dcmfx_p10/internal/data_element_header.{DataElementHeader}
import dcmfx_p10/internal/value_length
import dcmfx_p10/p10_error
import dcmfx_p10/p10_write
import gleam/option.{None, Some}
import gleeunit/should

pub fn data_element_header_to_bytes_test() {
  DataElementHeader(
    registry.waveform_data.tag,
    None,
    value_length.new(0x12345678),
  )
  |> p10_write.data_element_header_to_bytes(transfer_syntax.LittleEndian)
  |> should.equal(Ok(<<0, 84, 16, 16, 120, 86, 52, 18>>))

  DataElementHeader(
    registry.waveform_data.tag,
    None,
    value_length.new(0x12345678),
  )
  |> p10_write.data_element_header_to_bytes(transfer_syntax.BigEndian)
  |> should.equal(Ok(<<84, 0, 16, 16, 18, 52, 86, 120>>))

  DataElementHeader(
    registry.patient_age.tag,
    Some(value_representation.UnlimitedText),
    value_length.new(0x1234),
  )
  |> p10_write.data_element_header_to_bytes(transfer_syntax.LittleEndian)
  |> should.equal(Ok(<<16, 0, 16, 16, 85, 84, 0, 0, 52, 18, 0, 0>>))

  DataElementHeader(
    registry.pixel_data.tag,
    Some(value_representation.OtherWordString),
    value_length.new(0x12345678),
  )
  |> p10_write.data_element_header_to_bytes(transfer_syntax.LittleEndian)
  |> should.equal(Ok(<<224, 127, 16, 0, 79, 87, 0, 0, 120, 86, 52, 18>>))

  DataElementHeader(
    registry.pixel_data.tag,
    Some(value_representation.OtherWordString),
    value_length.new(0x12345678),
  )
  |> p10_write.data_element_header_to_bytes(transfer_syntax.BigEndian)
  |> should.equal(Ok(<<127, 224, 0, 16, 79, 87, 0, 0, 18, 52, 86, 120>>))

  DataElementHeader(
    registry.patient_age.tag,
    Some(value_representation.AgeString),
    value_length.new(0x12345),
  )
  |> p10_write.data_element_header_to_bytes(transfer_syntax.LittleEndian)
  |> should.equal(
    Error(p10_error.DataInvalid(
      "Serializing data element header",
      "Length 0x12345 exceeds the maximum of 0xFFFF",
      None,
      None,
    )),
  )

  DataElementHeader(
    registry.smallest_image_pixel_value.tag,
    Some(value_representation.SignedShort),
    value_length.new(0x1234),
  )
  |> p10_write.data_element_header_to_bytes(transfer_syntax.LittleEndian)
  |> should.equal(Ok(<<40, 0, 6, 1, 83, 83, 52, 18>>))

  DataElementHeader(
    registry.smallest_image_pixel_value.tag,
    Some(value_representation.SignedShort),
    value_length.new(0x1234),
  )
  |> p10_write.data_element_header_to_bytes(transfer_syntax.BigEndian)
  |> should.equal(Ok(<<0, 40, 1, 6, 83, 83, 18, 52>>))
}
