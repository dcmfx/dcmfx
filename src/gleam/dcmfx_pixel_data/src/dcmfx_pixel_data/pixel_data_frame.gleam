//// Defines a single frame of pixel data in its raw form.
//// 
//// The data will be native, RLE encoded, or using an encapsulated transfer
//// syntax, but the details of how it is encoded are not a concern of
//// `PixelDataFrame`.

import gleam/bit_array
import gleam/int
import gleam/list

/// A single frame of pixel data. This is made up of a one or more bit arrays,
/// which avoids copying of data.
///
/// If required, use `to_bytes()` to get the frame's data in a single bit array.
///
pub opaque type PixelDataFrame {
  PixelDataFrame(
    frame_index: Int,
    fragments: List(BitArray),
    length: Int,
    bit_offset: Int,
  )
}

/// Creates a new empty frame of pixel data.
///
pub fn new(frame_index: Int) -> PixelDataFrame {
  PixelDataFrame(frame_index:, fragments: [], length: 0, bit_offset: 0)
}

/// Returns the index of this frame, i.e. 0 for the first frame in its DICOM
/// data set, 1 for the second frame, etc.
///
pub fn index(frame: PixelDataFrame) -> Int {
  frame.frame_index
}

/// Adds the next fragment of pixel data to this frame.
///
@internal
pub fn push_fragment(frame: PixelDataFrame, data: BitArray) -> PixelDataFrame {
  PixelDataFrame(
    ..frame,
    fragments: [data, ..frame.fragments],
    length: frame.length + bit_array.byte_size(data),
  )
}

/// The size in bytes of this frame of pixel data.
///
pub fn length(frame: PixelDataFrame) -> Int {
  frame.length
}

/// The size in bits of this frame of pixel data. This takes into account the
/// frame's bit offset, i.e. the number of high bits in the first byte that
/// aren't used.
///
pub fn length_in_bits(frame: PixelDataFrame) -> Int {
  int.max(frame.length * 8 - frame.bit_offset, 0)
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
  frame.length == 0
}

/// Returns the fragments of binary data that make up this frame of pixel
/// data.
///
pub fn fragments(frame: PixelDataFrame) -> List(BitArray) {
  frame.fragments |> list.reverse
}

/// Removes `count` bytes from the end of this frame of pixel data.
///
@internal
pub fn drop_end_bytes(frame: PixelDataFrame, count: Int) -> PixelDataFrame {
  let target_length = int.max(0, frame.length - count)

  do_drop_end_bytes(frame, target_length)
}

fn do_drop_end_bytes(
  frame: PixelDataFrame,
  target_length: Int,
) -> PixelDataFrame {
  case frame.length > target_length {
    True ->
      case frame.fragments {
        [fragment, ..fragments] -> {
          let length = frame.length - bit_array.byte_size(fragment)

          // If this frame is now too short then restore it, but with a sliced
          // final fragment that exactly meets the target length
          case length < target_length {
            True -> {
              let fragment_length = target_length - length

              let assert Ok(new_fragment) =
                bit_array.slice(fragment, 0, fragment_length)

              PixelDataFrame(
                ..frame,
                fragments: [new_fragment, ..fragments],
                length: target_length,
              )
            }

            False ->
              PixelDataFrame(..frame, fragments:, length:)
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
/// the individual fragments is preferred when possible.
///
pub fn to_bytes(frame: PixelDataFrame) -> BitArray {
  let bytes = case frame.fragments {
    [fragment] -> fragment
    fragments -> fragments |> list.reverse |> bit_array.concat
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
