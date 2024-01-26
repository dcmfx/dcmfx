import dcmfx_core/data_element_value
import dcmfx_core/data_error
import dcmfx_core/data_set
import dcmfx_core/registry
import dcmfx_core/value_representation
import dcmfx_pixel_data
import gleam/bit_array
import gleam/result
import gleam/string
import gleeunit
import gleeunit/should

pub fn main() {
  gleeunit.main()
}

pub fn get_pixel_data_test() {
  let assert Ok(pixel_data) =
    data_element_value.new_encapsulated_pixel_data(
      value_representation.OtherByteString,
      [
        <<>>,
        string.repeat("1", 0x4C6) |> bit_array.from_string,
        string.repeat("2", 0x24A) |> bit_array.from_string,
        string.repeat("3", 0x628) |> bit_array.from_string,
      ],
    )

  let data_set_with_three_fragments =
    data_set.new()
    |> data_set.insert(registry.pixel_data.tag, pixel_data)

  // Read a single frame of non-encapsulated OB data
  let pixel_data =
    data_element_value.new_binary_unchecked(
      value_representation.OtherByteString,
      <<1, 2, 3, 4>>,
    )

  data_set.new()
  |> data_set.insert(registry.pixel_data.tag, pixel_data)
  |> dcmfx_pixel_data.get_pixel_data
  |> should.equal(
    Ok(#(value_representation.OtherByteString, [[<<1, 2, 3, 4>>]])),
  )

  // Read two frames of non-encapsulated OB data
  data_set.new()
  |> data_set.insert(registry.pixel_data.tag, pixel_data)
  |> data_set.insert(
    registry.number_of_frames.tag,
    data_element_value.new_binary_unchecked(value_representation.IntegerString, <<
      "2",
    >>),
  )
  |> dcmfx_pixel_data.get_pixel_data
  |> should.equal(
    Ok(#(value_representation.OtherByteString, [[<<1, 2>>], [<<3, 4>>]])),
  )

  // Read malformed multi-frame non-encapsulated OB data
  data_set.new()
  |> data_set.insert(registry.pixel_data.tag, pixel_data)
  |> data_set.insert(
    registry.number_of_frames.tag,
    data_element_value.new_binary_unchecked(value_representation.IntegerString, <<
      "3",
    >>),
  )
  |> dcmfx_pixel_data.get_pixel_data
  |> should.equal(
    Error(data_error.new_value_invalid(
      "Multi-frame pixel data of length 4 does not divide evenly into 3 frames",
    )),
  )

  // Read frames specified by an extended offset table
  let assert Ok(extended_offset_table) =
    data_element_value.new_binary(value_representation.OtherVeryLongString, <<
      0:64-little, 0x4CE:64-little, 0x720:64-little,
    >>)
  let assert Ok(extended_offset_table_lengths) =
    data_element_value.new_binary(value_representation.OtherVeryLongString, <<
      0x4C6:64-little, 0x24A:64-little, 0x627:64-little,
    >>)
  data_set_with_three_fragments
  |> data_set.insert(registry.extended_offset_table.tag, extended_offset_table)
  |> data_set.insert(
    registry.extended_offset_table_lengths.tag,
    extended_offset_table_lengths,
  )
  |> dcmfx_pixel_data.get_pixel_data
  |> should.equal(
    Ok(
      #(value_representation.OtherByteString, [
        [string.repeat("1", 0x4C6) |> bit_array.from_string],
        [string.repeat("2", 0x24A) |> bit_array.from_string],
        [string.repeat("3", 0x627) |> bit_array.from_string],
      ]),
    ),
  )

  // Read three fragments into a single frame
  // Taken from the DICOM standard. Ref: PS3.5 Table A.4-1.
  data_set_with_three_fragments
  |> dcmfx_pixel_data.get_pixel_data
  |> should.equal(
    Ok(
      #(value_representation.OtherByteString, [
        [
          string.repeat("1", 0x4C6) |> bit_array.from_string,
          string.repeat("2", 0x24A) |> bit_array.from_string,
          string.repeat("3", 0x628) |> bit_array.from_string,
        ],
      ]),
    ),
  )

  // Reads three fragments as frames when number of frames is three
  // Similar to the previous test but with a number of frames value present
  // that causes each fragment to be its own frame
  data_set_with_three_fragments
  |> data_set.insert_int_value(registry.number_of_frames, [3])
  |> result.try(dcmfx_pixel_data.get_pixel_data)
  |> should.equal(
    Ok(
      #(value_representation.OtherByteString, [
        [string.repeat("1", 0x4C6) |> bit_array.from_string],
        [string.repeat("2", 0x24A) |> bit_array.from_string],
        [string.repeat("3", 0x628) |> bit_array.from_string],
      ]),
    ),
  )

  // Read frames specified by a basic offset table
  // Taken from the DICOM standard. Ref: PS3.5 Table A.4-2.
  let assert Ok(pixel_data) =
    data_element_value.new_encapsulated_pixel_data(
      value_representation.OtherByteString,
      [
        <<0:32-little, 0x646:32-little>>,
        string.repeat("A", 0x2C8) |> bit_array.from_string,
        string.repeat("a", 0x36E) |> bit_array.from_string,
        string.repeat("B", 0xBC8) |> bit_array.from_string,
      ],
    )
  data_set.new()
  |> data_set.insert(registry.pixel_data.tag, pixel_data)
  |> dcmfx_pixel_data.get_pixel_data
  |> should.equal(
    Ok(
      #(value_representation.OtherByteString, [
        [
          string.repeat("A", 0x2C8) |> bit_array.from_string,
          string.repeat("a", 0x36E) |> bit_array.from_string,
        ],
        [string.repeat("B", 0xBC8) |> bit_array.from_string],
      ]),
    ),
  )
}
