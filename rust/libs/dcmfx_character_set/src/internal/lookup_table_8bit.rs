/// Decodes the next codepoint from the given bytes using an 8-bit lookup table.
/// The lookup table must have exactly 256 16-bit codepoint values.
///
/// This is used for all of the ISO 8859 encodings, as well as the JIS X 0201
/// encoding.
///
pub fn decode_next_codepoint<'a>(
  bytes: &'a [u8],
  lookup_table: &[u16; 256],
) -> Result<(char, &'a [u8]), ()> {
  match bytes {
    [byte_0, rest @ ..] => {
      let codepoint = lookup_table[*byte_0 as usize] as u32;
      let char = unsafe { char::from_u32_unchecked(codepoint) };

      Ok((char, rest))
    }

    _ => Err(()),
  }
}
