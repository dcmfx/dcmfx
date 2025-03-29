use crate::internal::utils;

/// Decodes the next codepoint from the given bytes using a 16-bit lookup table.
/// The lookup table must have exactly 8,836 (94 * 94) 16-bit codepoint values.
///
/// Input bytes <= 0x20 are passed through unchanged. Input bytes 0x21 - 0x7E
/// with a following byte in this same range are mapped to codepoints using the
/// lookup table.
///
/// This is used for the JIS X 0208 and JIS X 0212 encodings.
///
pub fn decode_next_codepoint<'a>(
  bytes: &'a [u8],
  lookup_table: &[u16; 8836],
) -> Result<(char, &'a [u8]), ()> {
  match bytes {
    [byte_0, rest @ ..] if *byte_0 <= 0x20 => {
      let codepoint = u32::from(*byte_0);

      Ok((utils::codepoint_to_char(codepoint), rest))
    }

    [byte_0, byte_1, rest @ ..]
      if (0x21..=0x7E).contains(byte_0) && (0x21..=0x7E).contains(byte_1) =>
    {
      // Calculate lookup table index
      let index =
        (usize::from(*byte_0) - 0x21) * 0x5E + (usize::from(*byte_1) - 0x21);

      let codepoint = u32::from(lookup_table[index]);

      Ok((utils::codepoint_to_char(codepoint), rest))
    }

    [_, rest @ ..] => Ok((utils::REPLACEMENT_CHARACTER, rest)),

    _ => Err(()),
  }
}
