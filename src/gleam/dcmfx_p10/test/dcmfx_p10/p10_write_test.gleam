import dcmfx_core/data_set_path
import dcmfx_core/dictionary
import dcmfx_core/transfer_syntax
import dcmfx_core/value_representation
import dcmfx_p10/internal/data_element_header.{DataElementHeader}
import dcmfx_p10/internal/value_length
import dcmfx_p10/p10_error
import dcmfx_p10/p10_write
import gleam/option.{None, Some}

pub fn data_element_header_to_bytes_test() {
  let context = p10_write.new_write_context(None)

  assert p10_write.data_element_header_to_bytes(
      DataElementHeader(
        dictionary.waveform_data.tag,
        None,
        value_length.new(0x12345678),
      ),
      transfer_syntax.LittleEndian,
      context,
    )
    == Ok(<<0, 84, 16, 16, 120, 86, 52, 18>>)

  assert p10_write.data_element_header_to_bytes(
      DataElementHeader(
        dictionary.waveform_data.tag,
        None,
        value_length.new(0x12345678),
      ),
      transfer_syntax.BigEndian,
      context,
    )
    == Ok(<<84, 0, 16, 16, 18, 52, 86, 120>>)

  assert p10_write.data_element_header_to_bytes(
      DataElementHeader(
        dictionary.patient_age.tag,
        Some(value_representation.UnlimitedText),
        value_length.new(0x1234),
      ),
      transfer_syntax.LittleEndian,
      context,
    )
    == Ok(<<16, 0, 16, 16, 85, 84, 0, 0, 52, 18, 0, 0>>)

  assert p10_write.data_element_header_to_bytes(
      DataElementHeader(
        dictionary.pixel_data.tag,
        Some(value_representation.OtherWordString),
        value_length.new(0x12345678),
      ),
      transfer_syntax.LittleEndian,
      context,
    )
    == Ok(<<224, 127, 16, 0, 79, 87, 0, 0, 120, 86, 52, 18>>)

  assert p10_write.data_element_header_to_bytes(
      DataElementHeader(
        dictionary.pixel_data.tag,
        Some(value_representation.OtherWordString),
        value_length.new(0x12345678),
      ),
      transfer_syntax.BigEndian,
      context,
    )
    == Ok(<<127, 224, 0, 16, 79, 87, 0, 0, 18, 52, 86, 120>>)

  assert p10_write.data_element_header_to_bytes(
      DataElementHeader(
        dictionary.patient_age.tag,
        Some(value_representation.AgeString),
        value_length.new(74_565),
      ),
      transfer_syntax.LittleEndian,
      context,
    )
    == Error(p10_error.DataInvalid(
      "Serializing data element header",
      "Length 74565 exceeds the maximum of 2^16 - 1 bytes",
      data_set_path.new(),
      0,
    ))

  assert p10_write.data_element_header_to_bytes(
      DataElementHeader(
        dictionary.smallest_image_pixel_value.tag,
        Some(value_representation.SignedShort),
        value_length.new(0x1234),
      ),
      transfer_syntax.LittleEndian,
      context,
    )
    == Ok(<<40, 0, 6, 1, 83, 83, 52, 18>>)

  assert p10_write.data_element_header_to_bytes(
      DataElementHeader(
        dictionary.smallest_image_pixel_value.tag,
        Some(value_representation.SignedShort),
        value_length.new(0x1234),
      ),
      transfer_syntax.BigEndian,
      context,
    )
    == Ok(<<0, 40, 1, 6, 83, 83, 18, 52>>)
}
