import dcmfx_core/data_element_value
import dcmfx_core/data_error
import dcmfx_core/data_set
import dcmfx_core/dictionary
import dcmfx_core/value_representation
import dcmfx_pixel_data
import dcmfx_pixel_data/p10_pixel_data_frame_filter
import dcmfx_pixel_data/pixel_data_frame.{type PixelDataFrame}
import gleam/bit_array
import gleam/list
import gleam/string
import gleeunit
import gleeunit/should

pub fn main() {
  gleeunit.main()
}

pub fn read_native_empty_frame_test() {
  let pixel_data =
    data_element_value.new_binary_unchecked(
      value_representation.OtherByteString,
      <<>>,
    )

  data_set.new()
  |> data_set.insert(dictionary.pixel_data.tag, pixel_data)
  |> dcmfx_pixel_data.get_pixel_data_frames
  |> should.equal(Ok([]))
}

pub fn read_native_single_frame_test() {
  let pixel_data =
    data_element_value.new_binary_unchecked(
      value_representation.OtherByteString,
      <<1, 2, 3, 4>>,
    )

  data_set.new()
  |> data_set.insert(dictionary.pixel_data.tag, pixel_data)
  |> dcmfx_pixel_data.get_pixel_data_frames
  |> should.equal(Ok([frame_with_fragments(0, [<<1, 2, 3, 4>>])]))
}

pub fn read_native_multi_frame_test() {
  data_set.new()
  |> data_set.insert(
    dictionary.pixel_data.tag,
    data_element_value.new_binary_unchecked(
      value_representation.OtherByteString,
      <<1, 2, 3, 4>>,
    ),
  )
  |> data_set.insert(
    dictionary.number_of_frames.tag,
    data_element_value.new_binary_unchecked(value_representation.IntegerString, <<
      "2",
    >>),
  )
  |> dcmfx_pixel_data.get_pixel_data_frames
  |> should.equal(
    Ok([
      frame_with_fragments(0, [<<1, 2>>]),
      frame_with_fragments(1, [<<3, 4>>]),
    ]),
  )
}

pub fn read_native_multi_frame_malformed() {
  data_set.new()
  |> data_set.insert(
    dictionary.pixel_data.tag,
    data_element_value.new_binary_unchecked(
      value_representation.OtherByteString,
      <<1, 2, 3, 4>>,
    ),
  )
  |> data_set.insert(
    dictionary.number_of_frames.tag,
    data_element_value.new_binary_unchecked(value_representation.IntegerString, <<
      "3",
    >>),
  )
  |> dcmfx_pixel_data.get_pixel_data_frames
  |> should.equal(
    Error(
      p10_pixel_data_frame_filter.DataError(data_error.new_value_invalid(
        "Multi-frame pixel data of length 4 does not divide evenly into 3 frames",
      )),
    ),
  )
}

// This test is taken from the DICOM standard. Ref: PS3.5 Table A.4-1.
pub fn read_encapsulated_multiple_fragments_into_single_frame_test() {
  data_set_with_three_fragments()
  |> dcmfx_pixel_data.get_pixel_data_frames
  |> should.equal(
    Ok([
      frame_with_fragments(0, [
        string.repeat("1", 0x4C6) |> bit_array.from_string,
        string.repeat("2", 0x24A) |> bit_array.from_string,
        string.repeat("3", 0x628) |> bit_array.from_string,
      ]),
    ]),
  )
}

pub fn read_encapsulated_multiple_fragments_into_multiple_frames_test() {
  let assert Ok(data_set) =
    data_set_with_three_fragments()
    |> data_set.insert_int_value(dictionary.number_of_frames, [3])

  data_set
  |> dcmfx_pixel_data.get_pixel_data_frames
  |> should.equal(
    Ok([
      frame_with_fragments(0, [
        string.repeat("1", 0x4C6) |> bit_array.from_string,
      ]),
      frame_with_fragments(1, [
        string.repeat("2", 0x24A) |> bit_array.from_string,
      ]),
      frame_with_fragments(2, [
        string.repeat("3", 0x628) |> bit_array.from_string,
      ]),
    ]),
  )
}

// This test is taken from the DICOM standard. Ref: PS3.5 Table A.4-2.
pub fn read_encapsulated_using_basic_offset_table_test() {
  let assert Ok(pixel_data) =
    data_element_value.new_encapsulated_pixel_data(
      value_representation.OtherByteString,
      [
        <<0:32-little, 0x646:32-little>>,
        string.repeat("1", 0x2C8) |> bit_array.from_string,
        string.repeat("2", 0x36E) |> bit_array.from_string,
        string.repeat("3", 0xBC8) |> bit_array.from_string,
      ],
    )

  data_set.new()
  |> data_set.insert(dictionary.pixel_data.tag, pixel_data)
  |> dcmfx_pixel_data.get_pixel_data_frames
  |> should.equal(
    Ok([
      frame_with_fragments(0, [
        string.repeat("1", 0x2C8) |> bit_array.from_string,
        string.repeat("2", 0x36E) |> bit_array.from_string,
      ]),
      frame_with_fragments(1, [
        string.repeat("3", 0xBC8) |> bit_array.from_string,
      ]),
    ]),
  )
}

pub fn read_encapsulated_using_extended_offset_table_test() {
  let assert Ok(extended_offset_table) =
    data_element_value.new_binary(value_representation.OtherVeryLongString, <<
      0:64-little, 0x4CE:64-little, 0x720:64-little,
    >>)
  let assert Ok(extended_offset_table_lengths) =
    data_element_value.new_binary(value_representation.OtherVeryLongString, <<
      0x4C6:64-little, 0x24A:64-little, 0x627:64-little,
    >>)

  data_set_with_three_fragments()
  |> data_set.insert(
    dictionary.extended_offset_table.tag,
    extended_offset_table,
  )
  |> data_set.insert(
    dictionary.extended_offset_table_lengths.tag,
    extended_offset_table_lengths,
  )
  |> dcmfx_pixel_data.get_pixel_data_frames
  |> should.equal(
    Ok([
      frame_with_fragments(0, [
        string.repeat("1", 0x4C6) |> bit_array.from_string,
      ]),
      frame_with_fragments(1, [
        string.repeat("2", 0x24A) |> bit_array.from_string,
      ]),
      frame_with_fragments(2, [
        string.repeat("3", 0x627) |> bit_array.from_string,
      ]),
    ]),
  )
}

fn frame_with_fragments(index: Int, fragments: List(BitArray)) -> PixelDataFrame {
  list.fold(
    fragments,
    pixel_data_frame.new(index),
    pixel_data_frame.push_fragment,
  )
}

fn data_set_with_three_fragments() {
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

  data_set.new()
  |> data_set.insert(dictionary.pixel_data.tag, pixel_data)
}
