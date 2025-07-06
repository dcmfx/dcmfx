import bigi.{type BigInt}
import gleam/bit_array
import gleam/int
import gleam/result
import ieee_float.{type IEEEFloat}

/// Reads `bytes` as a 16-bit signed integer.
///
pub fn to_int16(bytes: BitArray) -> Result(Int, Nil) {
  case bytes {
    <<value:16-little-signed>> -> Ok(value)
    _ -> Error(Nil)
  }
}

/// Reads `bytes` as a list of 16-bit signed integers.
///
pub fn to_int16_list(bytes: BitArray) -> Result(List(Int), Nil) {
  to_list(bytes, 2, to_int16)
}

/// Reads `bytes` as a 16-bit unsigned integer.
///
pub fn to_uint16(bytes: BitArray) -> Result(Int, Nil) {
  case bytes {
    <<value:16-little-unsigned>> -> Ok(value)
    _ -> Error(Nil)
  }
}

/// Reads `bytes` as a list of 16-bit unsigned integers.
///
pub fn to_uint16_list(bytes: BitArray) -> Result(List(Int), Nil) {
  to_list(bytes, 2, to_uint16)
}

/// Reads `bytes` as a 32-bit signed integer.
///
pub fn to_int32(bytes: BitArray) -> Result(Int, Nil) {
  case bytes {
    <<value:32-little-signed>> -> Ok(value)
    _ -> Error(Nil)
  }
}

/// Reads `bytes` as a list of 32-bit signed integers.
///
pub fn to_int32_list(bytes: BitArray) -> Result(List(Int), Nil) {
  to_list(bytes, 4, to_int32)
}

/// Reads `bytes` as a 32-bit unsigned integer.
///
pub fn to_uint32(bytes: BitArray) -> Result(Int, Nil) {
  case bytes {
    <<value:32-little-unsigned>> -> Ok(value)
    _ -> Error(Nil)
  }
}

/// Reads `bytes` as a list of 32-bit unsigned integers.
///
pub fn to_uint32_list(bytes: BitArray) -> Result(List(Int), Nil) {
  to_list(bytes, 4, to_uint32)
}

/// Reads `bytes` as a 64-bit signed integer.
///
pub fn to_int64(bytes: BitArray) -> Result(BigInt, Nil) {
  case bit_array.byte_size(bytes) {
    8 -> bigi.from_bytes(bytes, bigi.LittleEndian, bigi.Signed)
    _ -> Error(Nil)
  }
}

/// Reads `bytes` as a list of 64-bit signed integers.
///
pub fn to_int64_list(bytes: BitArray) -> Result(List(BigInt), Nil) {
  to_list(bytes, 8, to_int64)
}

/// Reads `bytes` as a 64-bit unsigned integer.
///
pub fn to_uint64(bytes: BitArray) -> Result(BigInt, Nil) {
  case bit_array.byte_size(bytes) {
    8 -> bigi.from_bytes(bytes, bigi.LittleEndian, bigi.Unsigned)
    _ -> Error(Nil)
  }
}

/// Reads `bytes` as a list of 64-bit unsigned integers.
///
pub fn to_uint64_list(bytes: BitArray) -> Result(List(BigInt), Nil) {
  to_list(bytes, 8, to_uint64)
}

/// Reads `bytes` as a 32-bit single-precision floating point number.
///
pub fn to_float32(bytes: BitArray) -> Result(IEEEFloat, Nil) {
  Ok(ieee_float.from_bytes_32_le(bytes))
}

/// Reads `bytes` as a list of 32-bit single-precision floating point numbers.
///
pub fn to_float32_list(bytes: BitArray) -> Result(List(IEEEFloat), Nil) {
  to_list(bytes, 4, to_float32)
}

/// Reads `bytes` as an 64-bit double-precision floating point number.
///
pub fn to_float64(bytes: BitArray) -> Result(IEEEFloat, Nil) {
  Ok(ieee_float.from_bytes_64_le(bytes))
}

/// Reads `bytes` as a list of 64-bit double-precision floating point numbers.
///
pub fn to_float64_list(bytes: BitArray) -> Result(List(IEEEFloat), Nil) {
  to_list(bytes, 8, to_float64)
}

/// Reads `bytes` as a list of one of the supported primitive types.
///
fn to_list(
  bytes: BitArray,
  item_size: Int,
  read_item: fn(BitArray) -> Result(a, Nil),
) -> Result(List(a), Nil) {
  let byte_count = bit_array.byte_size(bytes)

  case byte_count % item_size {
    0 -> {
      let item_count = byte_count / item_size
      do_to_list(bytes, item_size, read_item, item_count - 1, item_count, [])
    }

    _ -> Error(Nil)
  }
}

fn do_to_list(
  bytes: BitArray,
  item_size: Int,
  read_item: fn(BitArray) -> Result(a, Nil),
  i: Int,
  item_count: Int,
  acc: List(a),
) -> Result(List(a), Nil) {
  case i {
    -1 -> Ok(acc)

    _ -> {
      let item =
        bytes
        |> bit_array.slice(i * item_size, item_size)
        |> result.try(read_item)

      case item {
        Ok(item) ->
          do_to_list(bytes, item_size, read_item, i - 1, item_count, [
            item,
            ..acc
          ])
        Error(Nil) -> Error(Nil)
      }
    }
  }
}

/// Appends the specified padding byte if the bytes are of odd length.
///
pub fn pad_to_even_length(bytes: BitArray, padding_byte: Int) -> BitArray {
  case int.is_odd(bit_array.byte_size(bytes)) {
    True -> bit_array.concat([bytes, <<padding_byte>>])
    False -> bytes
  }
}

/// Returns the index of the last byte in a bit array that satisfies the given
/// predicate.
///
pub fn reverse_index(
  bytes: BitArray,
  predicate: fn(Int) -> Bool,
) -> Result(Int, Nil) {
  let index = bit_array.byte_size(bytes) - 1

  do_reverse_index(bytes, predicate, index)
}

fn do_reverse_index(
  bytes: BitArray,
  predicate: fn(Int) -> Bool,
  index: Int,
) -> Result(Int, Nil) {
  case bit_array.slice(bytes, index, 1) {
    Ok(<<final_byte>>) ->
      case predicate(final_byte) {
        True -> Ok(index)
        False ->
          case index {
            0 -> Error(Nil)
            _ -> do_reverse_index(bytes, predicate, index - 1)
          }
      }

    _ -> Error(Nil)
  }
}
