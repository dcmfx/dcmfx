use crate::internal::utils;

/// Decodes the next codepoint from the given UTF-8 bytes.
///
pub fn decode_next_codepoint(bytes: &[u8]) -> Result<(char, &[u8]), ()> {
  match bytes {
    // 1-byte UTF-8 character
    [b0, rest @ ..] if *b0 <= 0x7F => {
      let codepoint = *b0 as u32;

      Ok((utils::codepoint_to_char(codepoint), rest))
    }

    // 2-byte UTF-8 character
    [b0, b1, rest @ ..]
      if (0xC0..=0xDF).contains(b0) && (0x80..=0xBF).contains(b1) =>
    {
      let codepoint = ((*b0 as u32 & 0x1F) << 6) | (*b1 as u32 & 0x3F);

      Ok((utils::codepoint_to_char(codepoint), rest))
    }

    // 3-byte UTF-8 character
    [b0, b1, b2, rest @ ..]
      if (0xE0..=0xEF).contains(b0)
        && (0x80..=0xBF).contains(b1)
        && (0x80..=0xBF).contains(b2) =>
    {
      let codepoint = ((*b0 as u32 & 0x0F) << 12)
        | ((*b1 as u32 & 0x3F) << 6)
        | (*b2 as u32 & 0x3F);

      Ok((utils::codepoint_to_char(codepoint), rest))
    }

    // 4-byte UTF-8 character
    [b0, b1, b2, b3, rest @ ..]
      if (0xF0..=0xF7).contains(b0)
        && (0x80..=0xBF).contains(b1)
        && (0x80..=0xBF).contains(b2)
        && (0x80..=0xBF).contains(b3) =>
    {
      let codepoint = ((*b0 as u32 & 0x07) << 18)
        | ((*b1 as u32 & 0x3F) << 12)
        | ((*b2 as u32 & 0x3F) << 6)
        | (*b3 as u32 & 0x3F);

      Ok((utils::codepoint_to_char(codepoint), rest))
    }

    // Any other byte is invalid data, so return the replacement character and
    // continue with the next byte
    [_, rest @ ..] => Ok((utils::REPLACEMENT_CHARACTER, rest)),

    _ => Err(()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(not(feature = "std"))]
  use alloc::vec;

  #[test]
  fn decode_next_codepoint_test() {
    for (bytes, expected_codepoint) in [
      (vec![0x20], '\u{0020}'),
      (vec![0xC2, 0xA3], '\u{00A3}'),
      (vec![0xD0, 0x98], '\u{0418}'),
      (vec![0xE0, 0xA4, 0xB9], '\u{0939}'),
      (vec![0xE2, 0x82, 0xAC], '\u{20AC}'),
      (vec![0xED, 0x95, 0x9C], '\u{D55C}'),
      (vec![0xF0, 0x90, 0x8D, 0x88], '\u{10348}'),
      (vec![0xF0], '\u{FFFD}'),
    ] {
      assert_eq!(
        decode_next_codepoint(bytes.as_slice()).unwrap().0,
        expected_codepoint
      );
    }

    assert_eq!(decode_next_codepoint(&[]), Err(()));
  }
}
