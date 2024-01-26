import gleam/bit_array
import gleam/string

/// Decodes the next codepoint from the given bytes using a 16-bit lookup table.
/// The lookup table must have exactly 8,836 (94 * 94) 16-bit codepoint values.
///
/// Input bytes <= 0x20 are passed through unchanged. Input bytes 0x21 - 0x7E
/// with a following byte in this same range are mapped to codepoints using the
/// lookup table.
///
/// This is used for the JIS X 0208 and JIS X 0212 encodings.
///
pub fn decode_next_codepoint(
  bytes: BitArray,
  lookup_table: BitArray,
) -> Result(#(UtfCodepoint, BitArray), Nil) {
  case bytes {
    <<byte_0, rest:bytes>> if byte_0 <= 0x20 -> {
      let assert Ok(codepoint) = string.utf_codepoint(byte_0)
      Ok(#(codepoint, rest))
    }

    <<byte_0, byte_1, rest:bytes>>
      if byte_0 >= 0x21 && byte_0 <= 0x7E && byte_1 >= 0x21 && byte_1 <= 0x7E
    -> {
      // Calculate lookup table index
      let index = { byte_0 - 0x21 } * 0x5E + { byte_1 - 0x21 }

      let assert Ok(<<codepoint:16>>) =
        bit_array.slice(lookup_table, index * 2, 2)
      let assert Ok(codepoint) = string.utf_codepoint(codepoint)

      Ok(#(codepoint, rest))
    }

    <<_, rest:bytes>> -> {
      let assert Ok(codepoint) = string.utf_codepoint(0xFFFD)
      Ok(#(codepoint, rest))
    }

    _ -> Error(Nil)
  }
}
