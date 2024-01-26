/// The type of string to be decoded. This affects the characters that act as
/// delimiters when decoding ISO 2022-encoded strings that contain escape
/// sequences.
///
/// Encountering a delimiter resets the active code elements back to their
/// initial state.
///
pub type StringType {
  /// A single-valued string that does not have multiplicity. This uses the
  /// control characters as delimiters and is for use with the `ShortText`,
  /// `LongText`, and `UnlimitedText` value representations.
  SingleValue

  /// A multi-valued string that supports multiplicity. This uses the control
  /// characters and backslash as delimiters and is for use with the
  /// `LongString`, `ShortString` and `UnlimitedCharacters` value
  /// representations.
  MultiValue

  /// A person name string. This uses the control characters, backslash, caret,
  /// and equals sign as delimiters. This is for use with the `PersonName` value
  /// representation.
  PersonName
}
