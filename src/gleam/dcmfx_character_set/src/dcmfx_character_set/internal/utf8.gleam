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
    <<b0, rest:bytes>> if b0 <= 0x7F -> {
      let codepoint_value = b0

      Ok(#(utils.int_to_codepoint(codepoint_value), rest))
    }

    // 2-byte UTF-8 character
    <<b0, b1, rest:bytes>>
      if b0 >= 0xC0 && b0 <= 0xDF && b1 >= 0x80 && b1 <= 0xBF
    -> {
      let codepoint_value =
        int.bitwise_and(b0, 0x1F) * 64 + int.bitwise_and(b1, 0x3F)

      Ok(#(utils.int_to_codepoint(codepoint_value), rest))
    }

    // 3-byte UTF-8 character
    <<b0, b1, b2, rest:bytes>>
      if b0 >= 0xE0
      && b0 <= 0xEF
      && b1 >= 0x80
      && b1 <= 0xBF
      && b2 >= 0x80
      && b2 <= 0xBF
    -> {
      let codepoint_value =
        int.bitwise_and(b0, 0x0F)
        * 4096
        + int.bitwise_and(b1, 0x3F)
        * 64
        + int.bitwise_and(b2, 0x3F)

      Ok(#(utils.int_to_codepoint(codepoint_value), rest))
    }

    // 4-byte UTF-8 character
    <<b0, b1, b2, b3, rest:bytes>>
      if b0 >= 0xF0
      && b0 <= 0xF7
      && b1 >= 0x80
      && b1 <= 0xBF
      && b2 >= 0x80
      && b2 <= 0xBF
      && b3 >= 0x80
      && b3 <= 0xBF
    -> {
      let codepoint_value =
        int.bitwise_and(b0, 0x07)
        * 262_144
        + int.bitwise_and(b1, 0x3F)
        * 4096
        + int.bitwise_and(b2, 0x3F)
        * 64
        + int.bitwise_and(b3, 0x3F)

      Ok(#(utils.int_to_codepoint(codepoint_value), rest))
    }

    // Any other byte is invalid data, so return the replacement character and
    // continue with the next byte
    <<_, rest:bytes>> -> Ok(#(utils.replacement_character(), rest))

    _ -> Error(Nil)
  }
}
