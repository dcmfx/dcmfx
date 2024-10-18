/// Removes all whitespace from the end of the passed string. Whitespace is
/// defined as the following Unicode codepoints: U+0000, U+0009, U+000A, U+000D,
/// U+0020.
///
pub fn trim_right_whitespace(s: &str) -> &str {
  s.trim_end_matches([
    '\u{0000}', '\u{0009}', '\u{000A}', '\u{000D}', '\u{0020}',
  ])
}
