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
  PixelDataFrame(frame_index: Int, fragments: List(BitArray), length: Int)
}

/// Creates a new empty frame of pixel data.
///
pub fn new(frame_index: Int) -> PixelDataFrame {
  PixelDataFrame(frame_index:, fragments: [], length: 0)
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
    length: frame.length + bit_array.bit_size(data),
  )
}

/// The size in bytes of this frame of pixel data.
///
pub fn length(frame: PixelDataFrame) -> Int {
  { frame.length + 7 } / 8
}

/// The size in bits of this frame of pixel data.
///
pub fn length_in_bits(frame: PixelDataFrame) -> Int {
  frame.length
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

/// Returns whether all of this frame's fragments are byte-aligned, i.e. their
/// length is a multiple of eight.
///
pub fn is_byte_aligned(frame: PixelDataFrame) {
  frame.fragments
  |> list.all(fn(fragment) { bit_array.bit_size(fragment) % 8 == 0 })
}

/// Removes `count` bytes from the end of this frame of pixel data.
///
@internal
pub fn drop_end_bytes(frame: PixelDataFrame, count: Int) -> PixelDataFrame {
  let target_length = int.max(0, frame.length - count * 8)

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
          let length = frame.length - bit_array.bit_size(fragment)

          // If this frame is now too short then restore it, but with a sliced
          // final fragment that exactly meets the target length
          case length < target_length {
            True -> {
              let fragment_length = target_length - length

              let assert <<new_fragment:bits-size(fragment_length), _:bits>> =
                fragment

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
  case frame.fragments {
    [fragment] -> fragment
    fragments -> fragments |> list.reverse |> bit_array.concat
  }
}

/// Compares two frames of pixel data.
///
pub fn equals(frame: PixelDataFrame, other: PixelDataFrame) -> Bool {
  to_bytes(frame) == to_bytes(other)
}
