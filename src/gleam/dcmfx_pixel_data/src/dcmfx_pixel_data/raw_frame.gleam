import gleam/bit_array
import gleam/int
import gleam/list

/// A single frame of raw pixel data. This is made up of a one or more bit
/// arrays, which avoids copying of data.
///
/// If required, use `to_bytes` to get the raw frame data in a single bit array.
///
pub opaque type PixelDataRawFrame {
  PixelDataRawFrame(fragments: List(BitArray), length: Int)
}

/// Creates a new empty frame of raw pixel data.
///
@internal
pub fn new() -> PixelDataRawFrame {
  PixelDataRawFrame(fragments: [], length: 0)
}

/// Adds the next fragment of raw pixel data to this frame.
///
@internal
pub fn push_fragment(
  frame: PixelDataRawFrame,
  data: BitArray,
) -> PixelDataRawFrame {
  PixelDataRawFrame(
    fragments: [data, ..frame.fragments],
    length: frame.length + bit_array.byte_size(data),
  )
}

/// The size in bytes of this frame of raw pixel data.
///
pub fn length(frame: PixelDataRawFrame) -> Int {
  frame.length
}

/// Returns whether this frame of raw pixel data is empty.
///
pub fn is_empty(frame: PixelDataRawFrame) -> Bool {
  frame.length == 0
}

/// Returns the fragments of binary data that make up this frame of raw pixel
/// data.
///
pub fn fragments(frame: PixelDataRawFrame) -> List(BitArray) {
  frame.fragments |> list.reverse
}

/// Removes `count` bytes from the end of this frame of raw pixel data.
///
@internal
pub fn drop_end_bytes(frame: PixelDataRawFrame, count: Int) -> PixelDataRawFrame {
  let target_length = int.max(0, frame.length - count)

  do_drop_end_bytes(frame, target_length)
}

fn do_drop_end_bytes(
  frame: PixelDataRawFrame,
  target_length: Int,
) -> PixelDataRawFrame {
  case frame.length > target_length {
    True ->
      case frame.fragments {
        [fragment, ..fragments] -> {
          let length = frame.length - bit_array.byte_size(fragment)

          // If this raw frame is now too short then restore it, but with a
          // sliced final fragment that exactly meets the target length
          case length < target_length {
            True -> {
              let fragment_length = target_length - length

              let assert Ok(new_fragment) =
                bit_array.slice(fragment, 0, fragment_length)

              PixelDataRawFrame([new_fragment, ..fragments], target_length)
            }

            False ->
              PixelDataRawFrame(fragments, length)
              |> do_drop_end_bytes(target_length)
          }
        }

        _ -> frame
      }

    False -> frame
  }
}

/// Converts this frame of raw pixel data to a single contiguous bit array.
/// This may require copying the raw pixel data into a new contiguous buffer,
/// so accessing the individual fragments is preferred when possible.
///
pub fn to_bytes(frame: PixelDataRawFrame) -> BitArray {
  case frame.fragments {
    [fragment] -> fragment
    fragments -> fragments |> list.reverse |> bit_array.concat
  }
}

/// Compares two frames of raw pixel data.
///
pub fn equals(frame: PixelDataRawFrame, other: PixelDataRawFrame) -> Bool {
  to_bytes(frame) == to_bytes(other)
}
