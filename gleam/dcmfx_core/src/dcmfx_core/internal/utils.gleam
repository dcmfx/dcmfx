import gleam/bit_array
import gleam/float
import gleam/int
import gleam/list
import gleam/result
import gleam/string

/// Returns the length of a string in graphemes (on Erlang), or in UTF-16 code
/// units (on JavaScript).
///
/// This function is much faster than stdlib's `string.length()` on JavaScript
/// because it uses `String.prototype.length`, but be aware it returns a
/// different result when the string contains characters outside the Basic
/// Multilingual Plane, or graphemes made up of multiple codepoints.
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
/// This implementation is much faster than Gleam's native string.pad_left() on
/// the JavaScript target, but is not grapheme aware.
///
@external(javascript, "../../dcmfx_core_ffi.mjs", "utils__pad_start")
pub fn pad_start(s: String, desired_length: Int, pad_string: String) -> String {
  string.pad_left(s, desired_length, pad_string)
}

/// Returns a string containing the specified number of spaces.
///
/// This implementation is much faster than Gleam's native string.repeat() on
/// the JavaScript target. This function can be removed once gleam_stdlib 0.41
/// is released.
///
@external(javascript, "../../dcmfx_core_ffi.mjs", "utils__spaces")
pub fn spaces(n: Int) -> String {
  string.repeat(" ", n)
}

/// Helper function that parses a string to a float, handling the case where the
/// input string is actually an integer with no decimal point, or there are no
/// digits following the decimal point.
///
pub fn smart_parse_float(input: String) -> Result(Float, Nil) {
  let input = trim_right_codepoints(input, [0x2E])

  input
  |> float.parse
  |> result.lazy_or(fn() { float.parse(input <> ".0") })
}

/// Removes all occurrences of the specified characters from the end of a
/// string.
///
pub fn trim_right(s: String, chars: String) -> String {
  let codepoints =
    chars
    |> string.to_utf_codepoints
    |> list.map(string.utf_codepoint_to_int)

  trim_right_codepoints(s, codepoints)
}

/// Removes all whitespace from the end of the passed string. Whitespace is
/// defined as the following Unicode codepoints: U+0000, U+0009, U+000A, U+000D,
/// U+0020.
///
pub fn trim_right_whitespace(s: String) -> String {
  trim_right_codepoints(s, [0x00, 0x09, 0x0A, 0x0D, 0x20])
}

/// Removes all occurrences of the specified codepoints from the end of a
/// string. This function can only remove ASCII codepoints, i.e. those with
/// values <= `0x7F`.
///
pub fn trim_right_codepoints(s: String, codepoints: List(Int)) -> String {
  let s = bit_array.from_string(s)
  let len = bit_array.byte_size(s)

  do_trim_right_codepoints(s, len, codepoints)
}

fn do_trim_right_codepoints(
  s: BitArray,
  length: Int,
  codepoints: List(Int),
) -> String {
  case length {
    0 -> ""
    _ -> {
      // Get the last byte in the string
      let assert Ok(<<x>>) = bit_array.slice(s, length - 1, 1)

      case list.contains(codepoints, x) {
        True -> do_trim_right_codepoints(s, length - 1, codepoints)
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

/// Inspects a bit array in hexadecimal, e.g. `[1A 2B 3C 4D]`.
///
pub fn inspect_bit_array(bits: BitArray) -> String {
  do_inspect_bit_array(bits, "[") <> "]"
}

fn do_inspect_bit_array(input: BitArray, accumulator: String) -> String {
  case input {
    <<x, rest:bytes>> -> {
      let suffix = case rest {
        <<>> -> ""
        _ -> " "
      }

      let accumulator =
        accumulator
        <> { x |> int.to_base16 |> string.pad_left(2, "0") }
        <> suffix

      do_inspect_bit_array(rest, accumulator)
    }

    _ -> accumulator
  }
}
