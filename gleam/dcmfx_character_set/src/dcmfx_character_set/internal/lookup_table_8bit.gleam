import gleam/bit_array
import gleam/string

/// Decodes the next codepoint from the given bytes using an 8-bit lookup table.
/// The lookup table must have exactly 256 16-bit codepoint values.
///
/// This is used for all of the ISO 8859 encodings, as well as the JIS X 0201
/// encoding.
///
pub fn decode_next_codepoint(
  bytes: BitArray,
  lookup_table: BitArray,
) -> Result(#(UtfCodepoint, BitArray), Nil) {
  case bytes {
    <<byte_0, rest:bytes>> -> {
      let index = byte_0 * 2

      let assert Ok(<<codepoint:16>>) = bit_array.slice(lookup_table, index, 2)
      let assert Ok(codepoint) = string.utf_codepoint(codepoint)

      Ok(#(codepoint, rest))
    }

    _ -> Error(Nil)
  }
}
