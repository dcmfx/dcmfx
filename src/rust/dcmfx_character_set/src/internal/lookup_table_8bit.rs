use crate::internal::utils;

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
      let codepoint = u32::from(lookup_table[usize::from(*byte_0)]);

      Ok((utils::codepoint_to_char(codepoint), rest))
    }

    _ => Err(()),
  }
}
