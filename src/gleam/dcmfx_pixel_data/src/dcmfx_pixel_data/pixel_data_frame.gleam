//// Holds a single frame of pixel data in its raw form. Details of how to
//// read or interpret the data are not a concern of `PixelDataFrame`.

import gleam/bit_array
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}

/// A single frame of pixel data. This is made up of a one or more bit arrays,
/// which avoids copying of data.
///
/// If required, use `to_bytes()` to get the frame's data in a single bit array.
///
pub opaque type PixelDataFrame {
  PixelDataFrame(
    frame_index: Option(Int),
    chunks: List(BitArray),
    length_in_bits: Int,
    bit_offset: Int,
  )
}

/// Creates a new empty frame of pixel data.
///
pub fn new() -> PixelDataFrame {
  PixelDataFrame(
    frame_index: None,
    chunks: [],
    length_in_bits: 0,
    bit_offset: 0,
  )
}

/// Returns the index of this frame, i.e. 0 for the first frame in its DICOM
/// data set, 1 for the second frame, etc. Returns `None` if the frame's index
/// hasn't been set.
///
pub fn index(frame: PixelDataFrame) -> Option(Int) {
  frame.frame_index
}

/// Sets the index of this frame.
///
pub fn set_index(frame: PixelDataFrame, index: Int) -> PixelDataFrame {
  PixelDataFrame(..frame, frame_index: Some(index))
}

/// Adds the next chunk of pixel data to this frame.
///
pub fn push_chunk(frame: PixelDataFrame, data: BitArray) -> PixelDataFrame {
  PixelDataFrame(
    ..frame,
    chunks: [data, ..frame.chunks],
    length_in_bits: frame.length_in_bits + bit_array.bit_size(data),
  )
}

/// The size in bytes of this frame of pixel data.
///
pub fn length(frame: PixelDataFrame) -> Int {
  { length_in_bits(frame) + 7 } / 8
}

/// The size in bits of this frame of pixel data.
///
pub fn length_in_bits(frame: PixelDataFrame) -> Int {
  int.max(0, frame.length_in_bits - frame.bit_offset)
}

/// Returns the bit offset for this frame.
///
/// The bit offset is only relevant to native multi-frame pixel data that has
/// a *'(0028,0010) Bits Allocated'* value of 1, where it specifies how many
/// high bits in this frame's first byte should be ignored when reading its
/// data. In all other cases it is zero and is unused.
///
pub fn bit_offset(frame: PixelDataFrame) -> Int {
  frame.bit_offset
}

/// Sets this frame's pixel data bit offset. See `bit_offset()` for details.
///
pub fn set_bit_offset(frame: PixelDataFrame, bit_offset: Int) {
  PixelDataFrame(..frame, bit_offset:)
}

/// Returns whether this frame of pixel data is empty.
///
pub fn is_empty(frame: PixelDataFrame) -> Bool {
  length_in_bits(frame) == 0
}

/// Returns the chunks of binary data that make up this frame of pixel data.
///
pub fn chunks(frame: PixelDataFrame) -> List(BitArray) {
  frame.chunks |> list.reverse
}

/// Removes `count` bytes from the end of this frame of pixel data.
///
@internal
pub fn drop_end_bytes(frame: PixelDataFrame, count: Int) -> PixelDataFrame {
  let target_length = int.max(0, length_in_bits(frame) - count * 8)

  do_drop_end_bytes(frame, target_length)
}

fn do_drop_end_bytes(
  frame: PixelDataFrame,
  target_length: Int,
) -> PixelDataFrame {
  case length_in_bits(frame) > target_length {
    True ->
      case frame.chunks {
        [chunk, ..chunks] -> {
          let length_in_bits = length_in_bits(frame) - bit_array.bit_size(chunk)

          // If this frame is now too short then restore it, but with a sliced
          // final chunk that exactly meets the target length
          case length_in_bits < target_length {
            True -> {
              let chunk_length = target_length - length_in_bits

              let assert <<new_chunk:bits-size(chunk_length), _:bits>> = chunk

              PixelDataFrame(
                ..frame,
                chunks: [new_chunk, ..chunks],
                length_in_bits: target_length,
              )
            }

            False ->
              PixelDataFrame(..frame, chunks:, length_in_bits:)
              |> do_drop_end_bytes(target_length)
          }
        }

        _ -> frame
      }

    False -> frame
  }
}

/// Converts this frame of pixel data to a single contiguous bit array. This may
/// require copying the pixel data into a new contiguous buffer, so accessing
/// the individual chunks is preferred when possible.
///
pub fn to_bytes(frame: PixelDataFrame) -> BitArray {
  let bytes = case frame.chunks {
    [chunk] -> chunk
    chunks -> chunks |> list.reverse |> bit_array.concat
  }

  case frame.bit_offset {
    0 -> bytes
    _ -> shift_low_bits(bytes, frame.bit_offset)
  }
}

/// Shifts the specified number of low bits out of the first byte, and moves
/// everything following it into place such that there are no unused leading
/// bytes.
///
fn shift_low_bits(bytes: BitArray, bit_offset: Int) -> BitArray {
  bytes
  |> shift_low_bits_loop([], bit_offset)
  |> list.reverse
  |> bit_array.concat
}

fn shift_low_bits_loop(
  input: BitArray,
  acc: List(BitArray),
  bit_offset: Int,
) -> List(BitArray) {
  case input {
    <<a, b, _:bits>> -> {
      let byte =
        int.bitwise_shift_right(a, bit_offset)
        |> int.bitwise_or(int.bitwise_shift_left(b, 8 - bit_offset))

      let assert <<_, input:bits>> = input
      shift_low_bits_loop(input, [<<byte>>, ..acc], bit_offset)
    }

    <<a, _:bits>> -> {
      let byte = int.bitwise_shift_right(a, bit_offset)
      [<<byte>>, ..acc]
    }

    _ -> acc
  }
}

/// Compares two frames of pixel data.
///
pub fn equals(frame: PixelDataFrame, other: PixelDataFrame) -> Bool {
  to_bytes(frame) == to_bytes(other)
}
