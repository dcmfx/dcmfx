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
      let char = unsafe { char::from_u32_unchecked(*byte_0 as u32) };
      Ok((char, rest))
    }

    [byte_0, byte_1, rest @ ..]
      if (0x21..=0x7E).contains(byte_0) && (0x21..=0x7E).contains(byte_1) =>
    {
      // Calculate lookup table index
      let index = (*byte_0 as usize - 0x21) * 0x5E + (*byte_1 as usize - 0x21);

      let char =
        unsafe { char::from_u32_unchecked(lookup_table[index] as u32) };

      Ok((char, rest))
    }

    [_, rest @ ..] => {
      let char = unsafe { char::from_u32_unchecked(0xFFFD) };

      Ok((char, rest))
    }

    _ => Err(()),
  }
}
