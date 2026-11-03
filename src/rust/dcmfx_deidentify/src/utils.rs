/// Trims trailing padding, i.e. spaces and NULs, from the bytes of a value.
///
pub(crate) fn trim_trailing_padding(bytes: &[u8]) -> &[u8] {
  let mut end = bytes.len();
  while end > 0 && (bytes[end - 1] == 0x00 || bytes[end - 1] == 0x20) {
    end -= 1;
  }
  &bytes[..end]
}

/// Trims leading and trailing padding, i.e. spaces and NULs, from a string.
///
pub(crate) fn trim_padding(s: &str) -> &str {
  s.trim_matches([' ', '\0'])
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn trim_trailing_padding_test() {
    assert_eq!(trim_trailing_padding(b"abc \0 "), b"abc");
    assert_eq!(trim_trailing_padding(b" abc"), b" abc");
    assert_eq!(trim_trailing_padding(b"  "), b"");
  }

  #[test]
  fn trim_padding_test() {
    assert_eq!(trim_padding(" DCM \0"), "DCM");
  }
}
