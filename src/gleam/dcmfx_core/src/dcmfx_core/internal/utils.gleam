import gleam/bit_array
import gleam/float
import gleam/int
import gleam/result
import gleam/string

/// Returns the length of a string in graphemes (on Erlang), or in UTF-16 code
/// units (on JavaScript).
///
/// This function is much faster than `string.length()` on JavaScript because it
/// uses `String.length()`, but be aware it returns a different result when the
/// string contains characters outside the Basic Multilingual Plane, or
/// graphemes made up of multiple codepoints.
///
/// This function should only be used where the above discrepancy will not occur
/// or does not matter and performance is more important.
///
@external(javascript, "../../dcmfx_core_ffi.mjs", "utils__string_fast_length")
pub fn string_fast_length(s: String) -> Int {
  string.length(s)
}

/// Pads a string to the desired length by prepending a pad string.
///
/// This implementation is faster than `string.pad_left()` on the JavaScript
/// target, but is not grapheme aware.
///
@external(javascript, "../../dcmfx_core_ffi.mjs", "utils__pad_start")
pub fn pad_start(s: String, desired_length: Int, pad_string: String) -> String {
  string.pad_start(s, desired_length, pad_string)
}

/// Helper function that parses a string to a float, handling the case where the
/// input string is actually an integer with no decimal point, or there are no
/// digits following the decimal point.
///
pub fn smart_parse_float(input: String) -> Result(Float, Nil) {
  let input = trim_ascii_end(input, 0x2E)

  input
  |> float.parse
  |> result.lazy_or(fn() { float.parse(input <> ".0") })
}

/// Removes all occurrences of the specified ASCII codepoint from the start and
/// end of a string.
///
pub fn trim_ascii(s: String, ascii_character: Int) -> String {
  s
  |> trim_ascii_start(ascii_character)
  |> trim_ascii_end(ascii_character)
}

/// Removes all occurrences of the specified ASCII character from the start of a
/// string.
///
fn trim_ascii_start(s: String, ascii_character: Int) -> String {
  let s = bit_array.from_string(s)

  do_trim_ascii_start(s, ascii_character)
}

fn do_trim_ascii_start(s: BitArray, ascii_character: Int) -> String {
  case s {
    <<x, rest:bytes>> ->
      case x == ascii_character {
        True -> do_trim_ascii_start(rest, ascii_character)
        False -> {
          let assert Ok(s) = bit_array.to_string(s)
          s
        }
      }

    _ -> ""
  }
}

/// Removes all occurrences of the specified ASCII character from the end of a
/// string.
///
pub fn trim_ascii_end(s: String, ascii_character: Int) -> String {
  let s = bit_array.from_string(s)
  let len = bit_array.byte_size(s)

  do_trim_end_codepoints(s, len, ascii_character)
}

fn do_trim_end_codepoints(
  s: BitArray,
  length: Int,
  ascii_character: Int,
) -> String {
  case length {
    0 -> ""
    _ -> {
      // Get the last byte in the string
      let assert Ok(<<x>>) = bit_array.slice(s, length - 1, 1)

      case x == ascii_character {
        True -> do_trim_end_codepoints(s, length - 1, ascii_character)
        False -> {
          let assert Ok(s) = bit_array.slice(s, 0, length)
          let assert Ok(s) = bit_array.to_string(s)

          s
        }
      }
    }
  }
}

/// Index lookup into a list. This is used when looking up a specific sequence
/// item. This function was removed from the standard library in v0.38 because
/// of its O(N) performance.
///
pub fn list_at(in list: List(a), get index: Int) -> Result(a, Nil) {
  case index >= 0 {
    True ->
      case list_drop(list, index) {
        [] -> Error(Nil)
        [x, ..] -> Ok(x)
      }
    False -> Error(Nil)
  }
}

fn list_drop(from list: List(a), up_to n: Int) -> List(a) {
  case n <= 0 {
    True -> list
    False ->
      case list {
        [] -> []
        [_, ..xs] -> list_drop(xs, n - 1)
      }
  }
}

/// Inspects a bit array in hexadecimal, e.g. `[1A 2B 3C 4D]`. If the number of
/// bytes in the bit array exceeds `max_length` then not all bytes will be
/// shown and a trailing ellipsis will be appended, e.g. `[1A 2B 3C 4D …]`.
///
pub fn inspect_bit_array(bits: BitArray, max_length: Int) -> String {
  let byte_count = int.min(max_length, bit_array.byte_size(bits))

  let assert Ok(bits) = bit_array.slice(bits, 0, byte_count)

  let s = do_inspect_bit_array(bits, "[")

  let suffix = case byte_count == bit_array.byte_size(bits) {
    True -> "]"
    False -> " …]"
  }

  s <> suffix
}

fn do_inspect_bit_array(input: BitArray, acc: String) -> String {
  case input {
    <<x, rest:bytes>> -> {
      let suffix = case rest {
        <<>> -> ""
        _ -> " "
      }

      let acc = acc <> { x |> int.to_base16 |> pad_start(2, "0") } <> suffix

      do_inspect_bit_array(rest, acc)
    }

    _ -> acc
  }
}
