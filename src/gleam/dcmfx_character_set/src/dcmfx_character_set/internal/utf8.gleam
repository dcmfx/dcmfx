import dcmfx_character_set/internal/utils
@target(javascript)
import gleam/int

@target(erlang)
/// Decodes the next codepoint from the given UTF-8 bytes.
///
pub fn decode_next_codepoint(
  bytes: BitArray,
) -> Result(#(UtfCodepoint, BitArray), Nil) {
  case bytes {
    <<codepoint:utf8_codepoint, rest:bytes>> -> Ok(#(codepoint, rest))

    <<_, rest:bytes>> -> Ok(#(utils.replacement_character(), rest))

    _ -> Error(Nil)
  }
}

// The above implementation that uses a `utf8_codepoint` segment isn't supported
// on the JavaScript target as of Gleam 1.6.1, so the equivalent pattern match
// is implemented manually on that platform.

@target(javascript)
pub fn decode_next_codepoint(
  bytes: BitArray,
) -> Result(#(UtfCodepoint, BitArray), Nil) {
  case bytes {
    // 1-byte UTF-8 character
    <<byte_0, rest:bytes>> if byte_0 <= 0x7F -> {
      let codepoint_value = byte_0

      Ok(#(utils.int_to_codepoint(codepoint_value), rest))
    }

    // 2-byte UTF-8 character
    <<byte_0, byte_1, rest:bytes>>
      if byte_0 >= 0xC0 && byte_0 <= 0xDF && byte_1 >= 0x80 && byte_1 <= 0xBF
    -> {
      let codepoint_value =
        int.bitwise_and(byte_0, 0x1F) * 64 + int.bitwise_and(byte_1, 0x3F)

      Ok(#(utils.int_to_codepoint(codepoint_value), rest))
    }

    // 3-byte UTF-8 character
    <<byte_0, byte_1, byte_2, rest:bytes>>
      if byte_0 >= 0xE0
      && byte_0 <= 0xEF
      && byte_1 >= 0x80
      && byte_1 <= 0xBF
      && byte_2 >= 0x80
      && byte_2 <= 0xBF
    -> {
      let codepoint_value =
        int.bitwise_and(byte_0, 0x0F)
        * 4096
        + int.bitwise_and(byte_1, 0x3F)
        * 64
        + int.bitwise_and(byte_2, 0x3F)

      Ok(#(utils.int_to_codepoint(codepoint_value), rest))
    }

    // 4-byte UTF-8 character
    <<byte_0, byte_1, byte_2, byte_3, rest:bytes>>
      if byte_0 >= 0xF0
      && byte_0 <= 0xF7
      && byte_1 >= 0x80
      && byte_1 <= 0xBF
      && byte_2 >= 0x80
      && byte_2 <= 0xBF
      && byte_3 >= 0x80
      && byte_3 <= 0xBF
    -> {
      let codepoint_value =
        int.bitwise_and(byte_0, 0x07)
        * 262_144
        + int.bitwise_and(byte_1, 0x3F)
        * 4096
        + int.bitwise_and(byte_2, 0x3F)
        * 64
        + int.bitwise_and(byte_3, 0x3F)

      Ok(#(utils.int_to_codepoint(codepoint_value), rest))
    }

    // Any other byte is invalid data, so return the replacement character and
    // continue with the next byte
    <<_, rest:bytes>> -> Ok(#(utils.replacement_character(), rest))

    _ -> Error(Nil)
  }
}
