//! Decodes DICOM string data that uses a Specific Character Set into a native
//! UTF-8 string.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{
  string::{String, ToString},
  vec::Vec,
};

mod internal;

use internal::character_set::{self, CharacterSet, CodeElementPair};

/// The type of string to be decoded. This determines the characters that act as
/// delimiters and reset the active character set during decoding of encoded
/// strings that use ISO 2022 escape sequences.
///
/// Encountering a delimiter resets the active code elements back to their
/// initial state.
///
#[derive(Clone, Copy, PartialEq)]
pub enum StringType {
  /// A single-valued string that does not have multiplicity. This uses the
  /// control characters as delimiters and is for use with the `ShortText`,
  /// `LongText`, and `UnlimitedText` value representations.
  SingleValue,

  /// A multi-valued string that supports multiplicity. This uses the control
  /// characters and backslash as delimiters and is for use with the
  /// `LongString`, `ShortString` and `UnlimitedCharacters` value
  /// representations.
  MultiValue,

  /// A person name string. This uses the control characters, backslash, caret,
  /// and equals sign as delimiters. This is for use with the `PersonName` value
  /// representation.
  PersonName,
}

/// A specific character set as defined by the *'(0008,0005) Specific Character
/// Set)'* DICOM tag. This is a list of one or more individual character sets.
///
/// When there are multiple character sets in a specific character set it means
/// that Code Extension techniques are being used and that escape sequences may
/// be encountered when decoding.
///
#[derive(Clone, Debug, PartialEq)]
pub struct SpecificCharacterSet(Vec<&'static CharacterSet>);

impl SpecificCharacterSet {
  /// Converts a raw value from a "SpecificCharacterSet" data element into a
  /// `SpecificCharacterSet` instance that can be used to decode bytes into a
  /// native string.
  ///
  pub fn from_string(specific_character_set: &str) -> Result<Self, String> {
    let mut charsets: Vec<String> = specific_character_set
      .split('\\')
      .map(&str::trim)
      .map(&str::to_uppercase)
      .collect();

    // If the first character set is empty then default it to IR 6, i.e the
    // DICOM default character set
    if charsets.first() == Some(&"".to_string()) {
      if charsets.len() == 1 {
        charsets[0] = "ISO_IR 6".to_string();
      } else {
        charsets[0] = "ISO 2022 IR 6".to_string();
      }
    }

    // Convert to recognized character sets
    let mut charsets = charsets
      .into_iter()
      .map(character_set::from_string)
      .collect::<Result<Vec<&'static CharacterSet>, String>>(
    )?;

    // If the first character set does not use extensions then it must be the
    // only one. Conversely, if extensions are in use then all character sets
    // must support them.
    let charsets = match charsets.as_slice() {
      // A single value is always fine
      [_] => Ok(charsets),

      // If there are multiple values they must all support Code Extensions
      _ => {
        let has_non_iso_2022_charset =
          charsets.as_slice().iter().any(|charset| {
            matches!(
              charset,
              CharacterSet::SingleByteWithoutExtensions { .. }
                | CharacterSet::MultiByteWithoutExtensions { .. }
            )
          });

        if has_non_iso_2022_charset {
          Err("SpecificCharacterSet has multiple non-ISO 2022 values")
        } else {
          // If ISO 2022 IR 6 isn't specified in the character sets then
          // append it so it can still be used. This isn't mandated by the spec
          // but it improves compatibility.
          if !charsets.contains(&&character_set::ISO_2022_IR_6) {
            charsets.push(&character_set::ISO_2022_IR_6);
          }

          Ok(charsets)
        }
      }
    }?;

    Ok(Self(charsets))
  }

  /// Returns whether a specific character set is byte compatible with UTF-8.
  /// This is only the case for the DICOM default character set (ISO_IR 6) and
  /// the UTF-8 character set itself (ISO_IR 192).
  ///
  pub fn is_utf8_compatible(&self) -> bool {
    self.0.len() == 1
      && (self.0[0] == &character_set::ISO_IR_6
        || self.0[0] == &character_set::ISO_IR_192)
  }

  /// Decodes bytes using a specific character set to a native string.
  ///
  /// Trailing whitespace is automatically removed, and invalid bytes are
  /// replaced with the U+FFFD character: �.
  ///
  pub fn decode_bytes(&self, bytes: &[u8], string_type: StringType) -> String {
    match self.0.as_slice() {
      [
        CharacterSet::SingleByteWithoutExtensions {
          defined_term,
          decoder,
          ..
        },
      ] => {
        // When using the ISO_IR 13 character set to decode bytes that support
        // multiplicity, use a variant of JIS X 0201 that allows the backslash
        // character
        let decoder = if *defined_term == "ISO_IR 13"
          && (string_type == StringType::MultiValue
            || string_type == StringType::PersonName)
        {
          internal::jis_x_0201::decode_next_codepoint_allowing_backslash
        } else {
          *decoder
        };

        character_set::decode_bytes(bytes, decoder)
      }

      [CharacterSet::MultiByteWithoutExtensions { decoder, .. }] => {
        character_set::decode_bytes(bytes, *decoder)
      }

      _ => self.decode_iso_2022_bytes(
        bytes,
        string_type,
        self.default_code_elements(),
      ),
    }
  }

  fn decode_iso_2022_bytes(
    &self,
    mut bytes: &[u8],
    string_type: StringType,
    mut active_code_elements: CodeElementPair,
  ) -> String {
    let mut s = String::with_capacity(bytes.len());

    loop {
      match bytes {
        [] => return s,

        // Detect escape sequences and use them to update the active code
        // elements
        [0x1B, rest @ ..] => {
          bytes = self.apply_escape_sequence(rest, &mut active_code_elements);
        }

        _ => {
          // Determine the decoder to use
          let decoder = match (bytes, &active_code_elements) {
            // If the byte has its high bit set and there is a G1 code element
            // active then use it
            ([byte, ..], (_, Some(g1))) if *byte >= 0x80 => g1.decoder,

            // Otherwise if there is a G0 code element active then use it
            (_, (Some(g0), _)) => g0.decoder,

            // Fall back to the default character set
            _ => internal::iso_ir_6::decode_next_codepoint,
          };

          // This unwrap is safe because decoders only error when fed no bytes
          let (char, next_bytes) = decoder(bytes).unwrap();

          // Detect delimiters and reset code elements to default when they
          // occur
          match (char, &string_type) {
            ('\u{9}', _)
            | ('\u{A}', _)
            | ('\u{C}', _)
            | ('\u{D}', _)
            | ('\\', StringType::MultiValue)
            | ('\\', StringType::PersonName)
            | ('=', StringType::PersonName)
            | ('^', StringType::PersonName) => {
              active_code_elements = self.default_code_elements()
            }

            _ => (),
          };

          let mut char_utf8: [u8; 4] = [0, 0, 0, 0];
          s.push_str(char.encode_utf8(&mut char_utf8));

          bytes = next_bytes;
        }
      }
    }
  }

  /// Returns the default G0 and G1 code elements which are the ones specified
  /// by the first character set. These are the initially active code elements
  /// and they are also reactivated after any delimiter is encountered.
  ///
  fn default_code_elements(&self) -> CodeElementPair {
    match self.0.as_slice() {
      [
        CharacterSet::SingleByteWithExtensions {
          code_element_g0,
          code_element_g1,
          ..
        },
        ..,
      ] => (Some(*code_element_g0), *code_element_g1),

      [
        CharacterSet::MultiByteWithExtensions {
          code_element_g0,
          code_element_g1,
          ..
        },
        ..,
      ] => (*code_element_g0, *code_element_g1),

      _ => (None, None), // grcov-excl-line
    }
  }

  /// Attempts to update the active code elements based on the escape sequence
  /// at the start of the given bytes. If the escape sequence isn't for any of
  /// the available character sets then nothing happens, i.e. unrecognized
  /// escape sequences are ignored.
  ///
  fn apply_escape_sequence<'a>(
    &self,
    bytes: &'a [u8],
    active_code_elements: &mut CodeElementPair,
  ) -> &'a [u8] {
    for charset in self.0.iter() {
      let code_elements = charset.code_elements();

      // See if the escape sequence applies to the G0 code element of this
      // character set
      match update_code_element(&code_elements.0, bytes) {
        Ok(bytes) => {
          active_code_elements.0 = code_elements.0;
          return bytes;
        }

        // See if the escape sequence applies to the G1 code element of this
        // character set
        _ => match update_code_element(&code_elements.1, bytes) {
          Ok(bytes) => {
            active_code_elements.1 = code_elements.1;
            return bytes;
          }

          _ => continue,
        },
      }
    }

    bytes
  }
}

fn update_code_element<'a>(
  candidate: &Option<character_set::CodeElement>,
  bytes: &'a [u8],
) -> Result<&'a [u8], ()> {
  match candidate {
    Some(candidate) => {
      let escape_sequence = candidate.escape_sequence;
      let escape_sequence_length = if escape_sequence[2] == 0 { 2 } else { 3 };

      if bytes.starts_with(&escape_sequence[0..escape_sequence_length]) {
        Ok(&bytes[escape_sequence_length..])
      } else {
        Err(())
      }
    }

    None => Err(()),
  }
}

/// Replaces all bytes greater than 0x7F with the value 0x3F, i.e. the question
/// mark character. This can be used to ensure that only valid ISO 646/US-ASCII
/// bytes are present.
///
pub fn sanitize_default_charset_bytes(bytes: &mut [u8]) -> &[u8] {
  for b in bytes.iter_mut() {
    if *b > 0x7F {
      *b = 0x3F;
    }
  }

  bytes
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(not(feature = "std"))]
  use alloc::format;

  #[test]
  pub fn specific_character_set_test() {
    assert!(SpecificCharacterSet::from_string("").is_ok());
    assert!(SpecificCharacterSet::from_string("ISO_IR 144").is_ok());
    assert!(SpecificCharacterSet::from_string("ISO IR 144").is_ok());
    assert!(SpecificCharacterSet::from_string("iso-ir 144").is_ok());
    assert!(SpecificCharacterSet::from_string("\\ISO_IR 144").is_err());
    assert!(SpecificCharacterSet::from_string("\\ISO 2022 IR 144").is_ok());
    assert!(
      SpecificCharacterSet::from_string("ISO 2022 IR 6\\ISO 2022 IR 87")
        .is_ok()
    );
    assert!(
      SpecificCharacterSet::from_string("ISO_IR 6\\ISO 2022 IR 87").is_err()
    );
    assert!(SpecificCharacterSet::from_string("ISO_IR 192").is_ok());
    assert!(
      SpecificCharacterSet::from_string("ISO_IR 192\\ISO 2022 IR 149").is_err()
    );
    assert!(SpecificCharacterSet::from_string("GB18030").is_ok());
    assert!(SpecificCharacterSet::from_string("GB18030\\ISO_IR 192").is_err());
    assert!(SpecificCharacterSet::from_string("ISO_IR 90210").is_err());
  }

  #[test]
  pub fn decode_bytes_single_byte_without_extensions_test() {
    // Test decoding of ISO IR 100 bytes (ISO 646, US-ASCII)
    assert_eq!(
      decode_bytes(
        "ISO_IR 6",
        &[0x48, 0x65, 0x6C, 0x6C, 0x6F],
        StringType::PersonName,
      ),
      "Hello"
    );

    // Test decoding of ISO IR 100 bytes (ISO 8859-1, Latin-1)
    assert_eq!(
      decode_bytes(
        "ISO_IR 100",
        &[0x42, 0x75, 0x63, 0x5E, 0x4A, 0xE9, 0x72, 0xF4, 0x6D, 0x65],
        StringType::PersonName,
      ),
      "Buc^Jérôme"
    );

    // Test decoding of ISO IR 101 bytes (ISO 8859-2, Latin-2)
    assert_eq!(
      decode_bytes(
        "ISO_IR 101",
        &[0x57, 0x61, 0xB3, 0xEA, 0x73, 0x61],
        StringType::PersonName,
      ),
      "Wałęsa"
    );

    // Test decoding of ISO IR 109 bytes (ISO 8859-3, Latin-3)
    assert_eq!(
      decode_bytes(
        "ISO_IR 109",
        &[0x61, 0x6E, 0x74, 0x61, 0xFD, 0x6E, 0x6F, 0x6D, 0x6F],
        StringType::PersonName,
      ),
      "antaŭnomo"
    );

    // Test decoding of ISO IR 110 bytes (ISO 8859-4, Latin-4)
    assert_eq!(
      decode_bytes(
        "ISO_IR 110",
        &[0x76, 0xE0, 0x72, 0x64, 0x73],
        StringType::PersonName,
      ),
      "vārds"
    );

    // Test decoding of ISO IR 144 bytes (ISO 8859-5, Latin/Cyrillic)
    assert_eq!(
      decode_bytes(
        "ISO_IR 144",
        &[0xBB, 0xEE, 0xDA, 0x63, 0x65, 0xDC, 0xD1, 0x79, 0x70, 0xD3],
        StringType::PersonName,
      ),
      "Люкceмбypг"
    );

    // Test decoding of ISO IR 127 bytes (ISO 8859-6, Latin/Arabic)
    assert_eq!(
      decode_bytes(
        "ISO_IR 127",
        &[
          0xE2, 0xC8, 0xC7, 0xE6, 0xEA, 0x5E, 0xE4, 0xE6, 0xD2, 0xC7, 0xD1,
          0x20
        ],
        StringType::PersonName,
      ),
      "قباني^لنزار "
    );

    // Test decoding of ISO IR 126 bytes (ISO 8859-7, Latin/Greek)
    assert_eq!(
      decode_bytes(
        "ISO_IR 126",
        &[0xC4, 0xE9, 0xEF, 0xED, 0xF5, 0xF3, 0xE9, 0xEF, 0xF2],
        StringType::PersonName,
      ),
      "Διονυσιος"
    );

    // Test decoding of ISO IR 138 bytes (ISO 8859-8, Latin/Hebrew)
    assert_eq!(
      decode_bytes(
        "ISO_IR 138",
        &[0xF9, 0xF8, 0xE5, 0xEF, 0x5E, 0xE3, 0xE1, 0xE5, 0xF8, 0xE4],
        StringType::PersonName,
      ),
      "שרון^דבורה"
    );

    // Test decoding of ISO IR 148 bytes (ISO 8859-9 and Latin-5)
    assert_eq!(
      decode_bytes(
        "ISO_IR 148",
        &[0xC7, 0x61, 0x76, 0x75, 0xFE, 0x6F, 0xF0, 0x6C, 0x75],
        StringType::PersonName,
      ),
      "Çavuşoğlu"
    );

    // Test decoding of ISO IR 203 bytes (ISO 8859-15 and Latin-9)
    assert_eq!(
      decode_bytes(
        "ISO_IR 203",
        &[0xC7, 0x61, 0x76, 0x75, 0xFE, 0x6F, 0xF0, 0x6C, 0x75],
        StringType::PersonName,
      ),
      "Çavuþoðlu"
    );

    // Test decoding of ISO IR 13 bytes (JIS X 0201)
    assert_eq!(
      decode_bytes(
        "ISO_IR 13",
        &[0xD4, 0xCF, 0xC0, 0xDE, 0x5E, 0xC0, 0xDB, 0xB3],
        StringType::PersonName,
      ),
      "ﾔﾏﾀﾞ^ﾀﾛｳ"
    );

    // Test that an 0x5C byte results in the Yen symbol when using JIS X 0201 to
    // decode a string value without multiplicity
    assert_eq!(
      decode_bytes(
        "ISO_IR 13",
        &[0xA6, 0xDD, 0xDF, 0x5C, 0x7E],
        StringType::SingleValue,
      ),
      "ｦﾝﾟ¥‾"
    );

    // Test that an 0x5C byte results in a backslash when using JIS X 0201 to
    // decode a string value with multiplicity
    assert_eq!(
      decode_bytes(
        "ISO_IR 13",
        &[0xA6, 0xDD, 0xDF, 0x5C, 0x7E],
        StringType::MultiValue,
      ),
      "ｦﾝﾟ\\‾"
    );

    // Test decoding of ISO IR 166 bytes (ISO 8859-11, TIS 620-2533)
    assert_eq!(
      decode_bytes(
        "ISO_IR 166",
        &[0xB9, 0xD2, 0xC1, 0xCA, 0xA1, 0xD8, 0xC5],
        StringType::PersonName,
      ),
      "นามสกุล"
    );
  }

  #[test]
  pub fn decode_bytes_single_byte_with_extensions_test() {
    // Test decoding of ISO 2022 IR 127 bytes (ISO 8859-6, Latin/Arabic) with no
    // escape sequence
    assert_eq!(
      decode_bytes(
        "ISO 2022 IR 127",
        &[
          0xE2, 0xC8, 0xC7, 0xE6, 0xEA, 0x5E, 0x1B, 0x2D, 0x47, 0xE4, 0xE6,
          0xD2, 0xC7, 0xD1,
        ],
        StringType::PersonName,
      ),
      "قباني^لنزار"
    );

    // Test decoding of ISO 2022 IR 126 bytes (ISO 8859-7, Latin/Greek) with an
    // escape sequence
    assert_eq!(
      decode_bytes(
        "ISO 2022 IR 126",
        &[
          0x1B, 0x2D, 0x46, 0xC4, 0xE9, 0xEF, 0xED, 0xF5, 0xF3, 0xE9, 0xEF,
          0xF2
        ],
        StringType::PersonName,
      ),
      "Διονυσιος"
    );

    // Test decoding of multiple values in different single-byte encodings
    assert_eq!(
      decode_bytes(
        "ISO 2022 IR 100\\ISO 2022 IR 144\\ISO 2022 IR 126",
        &[
          0x42, 0x75, 0x63, 0x5E, 0x4A, 0xE9, 0x72, 0xF4, 0x6D, 0x65, 0x5C,
          0x1B, 0x2D, 0x46, 0xC4, 0xE9, 0xEF, 0xED, 0xF5, 0xF3, 0xE9, 0xEF,
          0xF2, 0x5C, 0x1B, 0x2D, 0x4C, 0xBB, 0xEE, 0xDA, 0x63, 0x65, 0xDC,
          0xD1, 0x79, 0x70, 0xD3,
        ],
        StringType::PersonName,
      ),
      "Buc^Jérôme\\Διονυσιος\\Люкceмбypг"
    );

    // Test decoding of an invalid escape sequence, it should be ignored
    assert_eq!(
      decode_bytes(
        "ISO 2022 IR 126",
        &[0x1B, 0x2D, 0x46, 0xC4, 0x1B, 0xC4],
        StringType::SingleValue,
      ),
      "ΔΔ"
    );

    // Test decoding falls back to the default character set when trying to use
    // an absent code element
    assert_eq!(
      decode_bytes("ISO 2022 IR 149", &[0x41, 0x42], StringType::SingleValue,),
      "AB"
    );
  }

  /// Tests that the relevant delimiters for each string type reset the
  /// character set correctly.
  ///
  #[test]
  pub fn decode_bytes_iso_2022_delimiters_test() {
    let data: [(StringType, &[u8], &[u8]); 3] = [
      (
        StringType::SingleValue,
        &[0x09, 0x0A, 0x0C, 0x0D],
        &[0x5C, 0x3D, 0x5E],
      ),
      (
        StringType::MultiValue,
        &[0x09, 0x0A, 0x0C, 0x0D, 0x5C],
        &[0x3D, 0x5E],
      ),
      (
        StringType::PersonName,
        &[0x09, 0x0A, 0x0C, 0x0D, 0x5C, 0x3D, 0x5E],
        &[],
      ),
    ];

    for (string_type, delimiters, non_delimiters) in data {
      for delimiter in delimiters {
        assert_eq!(
          decode_bytes(
            "ISO 2022 IR 148\\ISO 2022 IR 126",
            &[0x1B, 0x2D, 0x46, 0xED, *delimiter, 0xED],
            string_type,
          ),
          format!("ν{}í", char::from_u32((*delimiter).into()).unwrap())
        );
      }

      for non_delimiter in non_delimiters {
        assert_eq!(
          decode_bytes(
            "ISO 2022 IR 148\\ISO 2022 IR 126",
            &[0x1B, 0x2D, 0x46, 0xED, *non_delimiter, 0xED],
            string_type,
          ),
          format!("ν{}ν", char::from_u32((*non_delimiter).into()).unwrap())
        );
      }
    }
  }

  #[test]
  pub fn decode_bytes_multi_byte_with_extensions_test() {
    // Test decoding of ISO 2002 IR 87 bytes (JIS X 0208)
    assert_eq!(
      decode_bytes(
        "ISO 2022 IR 87",
        &[0x57, 0x5A, 0x61, 0x4F],
        StringType::SingleValue,
      ),
      "忱疣"
    );

    // Test that a 0x5C lead byte is not treated as a backslash when decoding
    // JIS X 0208 bytes
    assert_eq!(
      decode_bytes("ISO 2022 IR 87", &[0x5C, 0x41], StringType::MultiValue),
      "楞"
    );

    // Test decoding of ISO 2002 IR 159 bytes (JIS X 0212)
    assert_eq!(
      decode_bytes(
        "ISO 2022 IR 159",
        &[0x57, 0x5A, 0x61, 0x4F],
        StringType::SingleValue,
      ),
      "苷逘"
    );

    // Test decoding of ISO 2002 IR 149 bytes (KS X 1001)
    assert_eq!(
      decode_bytes(
        "ISO 2022 IR 149",
        &[0xB1, 0xE8, 0xC8, 0xF1, 0xC1, 0xDF],
        StringType::PersonName,
      ),
      "김희중"
    );

    // Test decoding of ISO 2002 IR 58 bytes (GB 2312)
    assert_eq!(
      decode_bytes(
        "ISO 2022 IR 58",
        &[
          0xB5, 0xDA, 0xD2, 0xBB, 0xD0, 0xD0, 0xCE, 0xC4, 0xD7, 0xD6, 0xA1,
          0xA3
        ],
        StringType::PersonName,
      ),
      "第一行文字。"
    );

    // Test decoding with a multi-byte character set as the second character set
    assert_eq!(
      decode_bytes(
        "\\ISO 2022 IR 149",
        &[
          0x1B, 0x24, 0x29, 0x43, 0xB1, 0xE8, 0xC8, 0xF1, 0xC1, 0xDF, 0x1B,
          0x28, 0x42, 0x5C, 0x1B, 0x24, 0x29, 0x43, 0xB1, 0xE8, 0xC8, 0xF1,
          0xC1, 0xDF, 0x1B, 0x28, 0x42, 0x20,
        ],
        StringType::PersonName,
      ),
      "김희중\\김희중 "
    );

    // Test decoding using two different multi-byte character sets
    assert_eq!(
      decode_bytes(
        "ISO 2022 IR 87\\ISO 2022 IR 149",
        &[
          0x1B, 0x24, 0x29, 0x43, 0xD1, 0xCE, 0xD4, 0xD7, 0x21, 0x38, 0x22,
          0x76, 0x30, 0x21, 0x3B, 0x33, 0x45, 0x44, 0x1B, 0x24, 0x42, 0x57,
          0x5A, 0x61, 0x4F,
        ],
        StringType::PersonName,
      ),
      "吉洞仝♪亜山田忱疣"
    );

    // Test use of ISO 2022 IR 6 even when it isn't explicitly specified
    assert_eq!(
      decode_bytes(
        "ISO 2022 IR 13\\ISO 2022 IR 87",
        &[
          0xD4, 0xCF, 0xC0, 0xDE, 0x5E, 0xC0, 0xDB, 0xB3, 0x3D, 0x1B, 0x24,
          0x42, 0x3B, 0x33, 0x45, 0x44, 0x1B, 0x28, 0x42, 0x5E, 0x1B, 0x24,
          0x42, 0x42, 0x40, 0x4F, 0x3A, 0x1B, 0x28, 0x42, 0x3D, 0x1B, 0x24,
          0x42, 0x24, 0x64, 0x24, 0x5E, 0x24, 0x40, 0x1B, 0x28, 0x42, 0x5E,
          0x1B, 0x24, 0x42, 0x24, 0x3F, 0x24, 0x6D, 0x24, 0x26, 0x1B, 0x28,
          0x42,
        ],
        StringType::PersonName,
      ),
      "ﾔﾏﾀﾞ^ﾀﾛｳ=山田^太郎=やまだ^たろう"
    );
  }

  #[test]
  pub fn decode_bytes_multi_byte_without_extensions_test() {
    // Test decoding of ISO IR 192 bytes (UTF-8)
    assert_eq!(
      decode_bytes(
        "ISO_IR 192",
        &[
          0x57, 0x61, 0x6E, 0x67, 0x5E, 0x58, 0x69, 0x61, 0x6F, 0x44, 0x6F,
          0x6E, 0x67, 0x3D, 0xE7, 0x8E, 0x8B, 0x5E, 0xE5, 0xB0, 0x8F, 0xE6,
          0x9D, 0xB1, 0x3D, 0x20,
        ],
        StringType::PersonName,
      ),
      "Wang^XiaoDong=王^小東= "
    );

    // Test decoding of GB 18030 bytes
    assert_eq!(
      decode_bytes(
        "GB18030",
        &[
          0x57, 0x61, 0x6E, 0x67, 0x5E, 0x58, 0x69, 0x61, 0x6F, 0x44, 0x6F,
          0x6E, 0x67, 0x3D, 0xCD, 0xF5, 0x5E, 0xD0, 0xA1, 0xB6, 0xAB, 0x3D,
        ],
        StringType::PersonName,
      ),
      "Wang^XiaoDong=王^小东="
    );

    // Test decoding of GBK bytes
    assert_eq!(
      decode_bytes("GBK", &[0xD0, 0xA1, 0xB6, 0xAB], StringType::SingleValue),
      "小东"
    );
  }

  /// Tests adapted from the examples in the annexes of the DICOM standard.
  ///
  #[test]
  pub fn decode_bytes_dicom_annex_examples_test() {
    //
    // Test examples from Annex H of the DICOM standard (Japanese)
    //

    assert_eq!(
      decode_bytes(
        "\\ISO 2022 IR 87",
        &[
          0x59, 0x61, 0x6D, 0x61, 0x64, 0x61, 0x5E, 0x54, 0x61, 0x72, 0x6F,
          0x75, 0x3D, 0x1B, 0x24, 0x42, 0x3B, 0x33, 0x45, 0x44, 0x1B, 0x28,
          0x42, 0x5E, 0x1B, 0x24, 0x42, 0x42, 0x40, 0x4F, 0x3A, 0x1B, 0x28,
          0x42, 0x3D, 0x1B, 0x24, 0x42, 0x24, 0x64, 0x24, 0x5E, 0x24, 0x40,
          0x1B, 0x28, 0x42, 0x5E, 0x1B, 0x24, 0x42, 0x24, 0x3F, 0x24, 0x6D,
          0x24, 0x26, 0x1B, 0x28, 0x42,
        ],
        StringType::PersonName,
      ),
      "Yamada^Tarou=山田^太郎=やまだ^たろう"
    );

    assert_eq!(
      decode_bytes(
        "ISO 2022 IR 13\\ISO 2022 IR 87",
        &[
          0xD4, 0xCF, 0xC0, 0xDE, 0x5E, 0xC0, 0xDB, 0xB3, 0x3D, 0x1B, 0x24,
          0x42, 0x3B, 0x33, 0x45, 0x44, 0x1B, 0x28, 0x4A, 0x5E, 0x1B, 0x24,
          0x42, 0x42, 0x40, 0x4F, 0x3A, 0x1B, 0x28, 0x4A, 0x3D, 0x1B, 0x24,
          0x42, 0x24, 0x64, 0x24, 0x5E, 0x24, 0x40, 0x1B, 0x28, 0x4A, 0x5E,
          0x1B, 0x24, 0x42, 0x24, 0x3F, 0x24, 0x6D, 0x24, 0x26, 0x1B, 0x28,
          0x4A,
        ],
        StringType::PersonName,
      ),
      "ﾔﾏﾀﾞ^ﾀﾛｳ=山田^太郎=やまだ^たろう"
    );

    //
    // Test examples from Annex I of the DICOM standard (Korean)
    //

    assert_eq!(
      decode_bytes(
        "\\ISO 2022 IR 149",
        &[
          0x48, 0x6F, 0x6E, 0x67, 0x5E, 0x47, 0x69, 0x6C, 0x64, 0x6F, 0x6E,
          0x67, 0x3D, 0x1B, 0x24, 0x29, 0x43, 0xFB, 0xF3, 0x5E, 0x1B, 0x24,
          0x29, 0x43, 0xD1, 0xCE, 0xD4, 0xD7, 0x3D, 0x1B, 0x24, 0x29, 0x43,
          0xC8, 0xAB, 0x5E, 0x1B, 0x24, 0x29, 0x43, 0xB1, 0xE6, 0xB5, 0xBF,
        ],
        StringType::PersonName,
      ),
      "Hong^Gildong=洪^吉洞=홍^길동"
    );

    //
    // Test examples from Annex J of the DICOM standard (Chinese)
    //

    assert_eq!(
      decode_bytes(
        "ISO_IR 192",
        &[
          0x57, 0x61, 0x6E, 0x67, 0x5E, 0x58, 0x69, 0x61, 0x6F, 0x44, 0x6F,
          0x6E, 0x67, 0x3D, 0xE7, 0x8E, 0x8B, 0x5E, 0xE5, 0xB0, 0x8F, 0xE6,
          0x9D, 0xB1, 0x3D,
        ],
        StringType::PersonName,
      ),
      "Wang^XiaoDong=王^小東="
    );

    assert_eq!(
      decode_bytes(
        "ISO_IR 192",
        &[
          0x54, 0x68, 0x65, 0x20, 0x66, 0x69, 0x72, 0x73, 0x74, 0x20, 0x6C,
          0x69, 0x6E, 0x65, 0x20, 0x69, 0x6E, 0x63, 0x6C, 0x75, 0x64, 0x65,
          0x73, 0xE4, 0xB8, 0xAD, 0xE6, 0x96, 0x87, 0x2E, 0x0D, 0x0A, 0x54,
          0x68, 0x65, 0x20, 0x73, 0x65, 0x63, 0x6F, 0x6E, 0x64, 0x20, 0x6C,
          0x69, 0x6E, 0x65, 0x20, 0x69, 0x6E, 0x63, 0x6C, 0x75, 0x64, 0x65,
          0x73, 0xE4, 0xB8, 0xAD, 0xE6, 0x96, 0x87, 0x2C, 0x20, 0x74, 0x6F,
          0x6F, 0x2E, 0x0D, 0x0A, 0x54, 0x68, 0x65, 0x20, 0x74, 0x68, 0x69,
          0x72, 0x64, 0x20, 0x6C, 0x69, 0x6E, 0x65, 0x2E, 0x0D, 0x0A,
        ],
        StringType::MultiValue,
      ),
      "The first line includes中文.\r\n\
       The second line includes中文, too.\r\n\
       The third line.\r\n"
    );

    assert_eq!(
      decode_bytes(
        "GB18030",
        &[
          0x57, 0x61, 0x6E, 0x67, 0x5E, 0x58, 0x69, 0x61, 0x6F, 0x44, 0x6F,
          0x6E, 0x67, 0x3D, 0xCD, 0xF5, 0x5E, 0xD0, 0xA1, 0xB6, 0xAB, 0x3D,
        ],
        StringType::PersonName,
      ),
      "Wang^XiaoDong=王^小东="
    );

    assert_eq!(
      decode_bytes(
        "GB18030",
        &[
          0x54, 0x68, 0x65, 0x20, 0x66, 0x69, 0x72, 0x73, 0x74, 0x20, 0x6C,
          0x69, 0x6E, 0x65, 0x20, 0x69, 0x6E, 0x63, 0x6C, 0x75, 0x64, 0x65,
          0x73, 0xD6, 0xD0, 0xCE, 0xC4, 0x2E, 0x0D, 0x0A, 0x54, 0x68, 0x65,
          0x20, 0x73, 0x65, 0x63, 0x6F, 0x6E, 0x64, 0x20, 0x6C, 0x69, 0x6E,
          0x65, 0x20, 0x69, 0x6E, 0x63, 0x6C, 0x75, 0x64, 0x65, 0x73, 0xD6,
          0xD0, 0xCE, 0xC4, 0x2C, 0x20, 0x74, 0x6F, 0x6F, 0x2E, 0x0D, 0x0A,
          0x54, 0x68, 0x65, 0x20, 0x74, 0x68, 0x69, 0x72, 0x64, 0x20, 0x6C,
          0x69, 0x6E, 0x65, 0x2E, 0x0D, 0x0A,
        ],
        StringType::MultiValue,
      ),
      "The first line includes中文.\r\n\
       The second line includes中文, too.\r\n\
       The third line.\r\n",
    );

    //
    // Test examples from Annex K of the DICOM standard (Chinese)
    //

    assert_eq!(
      decode_bytes(
        "\\ISO 2022 IR 58",
        &[
          0x5A, 0x68, 0x61, 0x6E, 0x67, 0x5E, 0x58, 0x69, 0x61, 0x6F, 0x44,
          0x6F, 0x6E, 0x67, 0x3D, 0x1B, 0x24, 0x29, 0x41, 0xD5, 0xC5, 0x5E,
          0x1B, 0x24, 0x29, 0x41, 0xD0, 0xA1, 0xB6, 0xAB, 0x3D, 0x20,
        ],
        StringType::PersonName,
      ),
      "Zhang^XiaoDong=张^小东= "
    );

    assert_eq!(
      decode_bytes(
        "\\ISO 2022 IR 58",
        &[
          0x31, 0x2E, 0x1B, 0x24, 0x29, 0x41, 0xB5, 0xDA, 0xD2, 0xBB, 0xD0,
          0xD0, 0xCE, 0xC4, 0xD7, 0xD6, 0xA1, 0xA3, 0x0D, 0x0A,
        ],
        StringType::PersonName,
      ),
      "1.第一行文字。\r\n"
    );

    assert_eq!(
      decode_bytes(
        "\\ISO 2022 IR 58",
        &[
          0x32, 0x2E, 0x1B, 0x24, 0x29, 0x41, 0xB5, 0xDA, 0xB6, 0xFE, 0xD0,
          0xD0, 0xCE, 0xC4, 0xD7, 0xD6, 0xA1, 0xA3, 0x0D, 0x0A,
        ],
        StringType::PersonName,
      ),
      "2.第二行文字。\r\n"
    );

    assert_eq!(
      decode_bytes(
        "\\ISO 2022 IR 58",
        &[
          0x33, 0x2E, 0x1B, 0x24, 0x29, 0x41, 0xB5, 0xDA, 0xC8, 0xFD, 0xD0,
          0xD0, 0xCE, 0xC4, 0xD7, 0xD6, 0xA1, 0xA3, 0x0D, 0x0A,
        ],
        StringType::PersonName,
      ),
      "3.第三行文字。\r\n"
    );
  }

  fn decode_bytes(
    specific_character_set: &str,
    bytes: &[u8],
    string_type: StringType,
  ) -> String {
    let charset =
      SpecificCharacterSet::from_string(specific_character_set).unwrap();

    charset.decode_bytes(bytes, string_type)
  }

  #[test]
  pub fn sanitize_default_charset_bytes_test() {
    assert_eq!(sanitize_default_charset_bytes(&mut []), []);

    assert_eq!(
      sanitize_default_charset_bytes(&mut [0x40, 0x50, 0x60]),
      [0x40, 0x50, 0x60]
    );

    assert_eq!(
      sanitize_default_charset_bytes(&mut [0xDD, 0x50, 0x60]),
      [0x3F, 0x50, 0x60]
    );

    assert_eq!(
      sanitize_default_charset_bytes(&mut [0x40, 0xDD, 0x60]),
      [0x40, 0x3F, 0x60]
    );

    assert_eq!(
      sanitize_default_charset_bytes(&mut [0x40, 0x50, 0xDD]),
      [0x40, 0x50, 0x3F]
    );

    assert_eq!(
      sanitize_default_charset_bytes(&mut [0xDD, 0xDD, 0xDD]),
      [0x3F, 0x3F, 0x3F]
    );
  }
}
