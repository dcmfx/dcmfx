//! DICOM value representations (VRs).
//!
//! See [section 6.2](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_6.2.html)
//! of the DICOM specification for VR definitions.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// All DICOM value representations (VRs).
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueRepresentation {
  AgeString,
  ApplicationEntity,
  AttributeTag,
  CodeString,
  Date,
  DateTime,
  DecimalString,
  FloatingPointDouble,
  FloatingPointSingle,
  IntegerString,
  LongString,
  LongText,
  OtherByteString,
  OtherDoubleString,
  OtherFloatString,
  OtherLongString,
  OtherVeryLongString,
  OtherWordString,
  PersonName,
  Sequence,
  ShortString,
  ShortText,
  SignedLong,
  SignedShort,
  SignedVeryLong,
  Time,
  UniqueIdentifier,
  UniversalResourceIdentifier,
  Unknown,
  UnlimitedCharacters,
  UnlimitedText,
  UnsignedLong,
  UnsignedShort,
  UnsignedVeryLong,
}

/// The restrictions that apply to the length of a value representation's data.
/// These restrictions are defined by the DICOM specification, and are only
/// enforced when creating new values.
///
/// The restrictions are:
///
/// 1. The maximum number of bytes a value can have.
///
/// 2. Optionally, a number that the number of bytes must be an exact multiple
///    of.
///
/// 3. Optionally, for string-valued VRs, a limit on the number of characters
///    (not bytes) in the string. In multi-valued string VRs this limit applies
///    to each value individually.
///
#[derive(Debug, PartialEq)]
pub struct LengthRequirements {
  pub bytes_max: usize,
  pub bytes_multiple_of: Option<usize>,
  pub string_characters_max: Option<usize>,
}

impl core::fmt::Display for ValueRepresentation {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    f.write_str(core::str::from_utf8(&self.to_bytes()).unwrap())
  }
}

impl ValueRepresentation {
  /// Converts a two-character string, e.g. "DA", into a value representation.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn from_bytes(bytes: &[u8]) -> Result<Self, ()> {
    match bytes {
      [0x41, 0x45] => Ok(ValueRepresentation::ApplicationEntity),
      [0x41, 0x53] => Ok(ValueRepresentation::AgeString),
      [0x41, 0x54] => Ok(ValueRepresentation::AttributeTag),
      [0x43, 0x53] => Ok(ValueRepresentation::CodeString),
      [0x44, 0x41] => Ok(ValueRepresentation::Date),
      [0x44, 0x53] => Ok(ValueRepresentation::DecimalString),
      [0x44, 0x54] => Ok(ValueRepresentation::DateTime),
      [0x46, 0x44] => Ok(ValueRepresentation::FloatingPointDouble),
      [0x46, 0x4C] => Ok(ValueRepresentation::FloatingPointSingle),
      [0x49, 0x53] => Ok(ValueRepresentation::IntegerString),
      [0x4C, 0x4F] => Ok(ValueRepresentation::LongString),
      [0x4C, 0x54] => Ok(ValueRepresentation::LongText),
      [0x4F, 0x42] => Ok(ValueRepresentation::OtherByteString),
      [0x4F, 0x44] => Ok(ValueRepresentation::OtherDoubleString),
      [0x4F, 0x46] => Ok(ValueRepresentation::OtherFloatString),
      [0x4F, 0x4C] => Ok(ValueRepresentation::OtherLongString),
      [0x4F, 0x56] => Ok(ValueRepresentation::OtherVeryLongString),
      [0x4F, 0x57] => Ok(ValueRepresentation::OtherWordString),
      [0x50, 0x4E] => Ok(ValueRepresentation::PersonName),
      [0x53, 0x48] => Ok(ValueRepresentation::ShortString),
      [0x53, 0x4C] => Ok(ValueRepresentation::SignedLong),
      [0x53, 0x51] => Ok(ValueRepresentation::Sequence),
      [0x53, 0x53] => Ok(ValueRepresentation::SignedShort),
      [0x53, 0x54] => Ok(ValueRepresentation::ShortText),
      [0x53, 0x56] => Ok(ValueRepresentation::SignedVeryLong),
      [0x54, 0x4D] => Ok(ValueRepresentation::Time),
      [0x55, 0x43] => Ok(ValueRepresentation::UnlimitedCharacters),
      [0x55, 0x49] => Ok(ValueRepresentation::UniqueIdentifier),
      [0x55, 0x4C] => Ok(ValueRepresentation::UnsignedLong),
      [0x55, 0x4E] => Ok(ValueRepresentation::Unknown),
      [0x55, 0x52] => Ok(ValueRepresentation::UniversalResourceIdentifier),
      [0x55, 0x53] => Ok(ValueRepresentation::UnsignedShort),
      [0x55, 0x54] => Ok(ValueRepresentation::UnlimitedText),
      [0x55, 0x56] => Ok(ValueRepresentation::UnsignedVeryLong),

      _ => Err(()),
    }
  }

  /// Converts a value representation to its two-byte character representation.
  ///
  pub fn to_bytes(&self) -> [u8; 2] {
    *match self {
      ValueRepresentation::AgeString => b"AS",
      ValueRepresentation::ApplicationEntity => b"AE",
      ValueRepresentation::AttributeTag => b"AT",
      ValueRepresentation::CodeString => b"CS",
      ValueRepresentation::Date => b"DA",
      ValueRepresentation::DateTime => b"DT",
      ValueRepresentation::DecimalString => b"DS",
      ValueRepresentation::FloatingPointDouble => b"FD",
      ValueRepresentation::FloatingPointSingle => b"FL",
      ValueRepresentation::IntegerString => b"IS",
      ValueRepresentation::LongString => b"LO",
      ValueRepresentation::LongText => b"LT",
      ValueRepresentation::OtherByteString => b"OB",
      ValueRepresentation::OtherDoubleString => b"OD",
      ValueRepresentation::OtherFloatString => b"OF",
      ValueRepresentation::OtherLongString => b"OL",
      ValueRepresentation::OtherVeryLongString => b"OV",
      ValueRepresentation::OtherWordString => b"OW",
      ValueRepresentation::PersonName => b"PN",
      ValueRepresentation::Sequence => b"SQ",
      ValueRepresentation::ShortString => b"SH",
      ValueRepresentation::ShortText => b"ST",
      ValueRepresentation::SignedLong => b"SL",
      ValueRepresentation::SignedShort => b"SS",
      ValueRepresentation::SignedVeryLong => b"SV",
      ValueRepresentation::Time => b"TM",
      ValueRepresentation::UniqueIdentifier => b"UI",
      ValueRepresentation::UniversalResourceIdentifier => b"UR",
      ValueRepresentation::Unknown => b"UN",
      ValueRepresentation::UnlimitedCharacters => b"UC",
      ValueRepresentation::UnlimitedText => b"UT",
      ValueRepresentation::UnsignedLong => b"UL",
      ValueRepresentation::UnsignedShort => b"US",
      ValueRepresentation::UnsignedVeryLong => b"UV",
    }
  }

  /// Returns the human-readable name of a value representation, e.g.
  /// `CodeString`, `AttributeTag`.
  ///
  pub fn name(&self) -> &str {
    match self {
      ValueRepresentation::AgeString => "AgeString",
      ValueRepresentation::ApplicationEntity => "ApplicationEntity",
      ValueRepresentation::AttributeTag => "AttributeTag",
      ValueRepresentation::CodeString => "CodeString",
      ValueRepresentation::Date => "Date",
      ValueRepresentation::DateTime => "DateTime",
      ValueRepresentation::DecimalString => "DecimalString",
      ValueRepresentation::FloatingPointDouble => "FloatingPointDouble",
      ValueRepresentation::FloatingPointSingle => "FloatingPointSingle",
      ValueRepresentation::IntegerString => "IntegerString",
      ValueRepresentation::LongString => "LongString",
      ValueRepresentation::LongText => "LongText",
      ValueRepresentation::OtherByteString => "OtherByteString",
      ValueRepresentation::OtherDoubleString => "OtherDoubleString",
      ValueRepresentation::OtherFloatString => "OtherFloatString",
      ValueRepresentation::OtherLongString => "OtherLongString",
      ValueRepresentation::OtherVeryLongString => "OtherVeryLongString",
      ValueRepresentation::OtherWordString => "OtherWordString",
      ValueRepresentation::PersonName => "PersonName",
      ValueRepresentation::Sequence => "Sequence",
      ValueRepresentation::ShortString => "ShortString",
      ValueRepresentation::ShortText => "ShortText",
      ValueRepresentation::SignedLong => "SignedLong",
      ValueRepresentation::SignedShort => "SignedShort",
      ValueRepresentation::SignedVeryLong => "SignedVeryLong",
      ValueRepresentation::Time => "Time",
      ValueRepresentation::UniqueIdentifier => "UniqueIdentifier",
      ValueRepresentation::UniversalResourceIdentifier => {
        "UniversalResourceIdentifier"
      }
      ValueRepresentation::Unknown => "Unknown",
      ValueRepresentation::UnlimitedCharacters => "UnlimitedCharacters",
      ValueRepresentation::UnlimitedText => "UnlimitedText",
      ValueRepresentation::UnsignedLong => "UnsignedLong",
      ValueRepresentation::UnsignedShort => "UnsignedShort",
      ValueRepresentation::UnsignedVeryLong => "UnsignedVeryLong",
    }
  }

  /// Returns whether a value representation stores string data.
  ///
  pub fn is_string(self) -> bool {
    self == ValueRepresentation::AgeString
      || self == ValueRepresentation::ApplicationEntity
      || self == ValueRepresentation::CodeString
      || self == ValueRepresentation::Date
      || self == ValueRepresentation::DateTime
      || self == ValueRepresentation::DecimalString
      || self == ValueRepresentation::IntegerString
      || self == ValueRepresentation::LongString
      || self == ValueRepresentation::LongText
      || self == ValueRepresentation::PersonName
      || self == ValueRepresentation::ShortString
      || self == ValueRepresentation::ShortText
      || self == ValueRepresentation::Time
      || self == ValueRepresentation::UniqueIdentifier
      || self == ValueRepresentation::UniversalResourceIdentifier
      || self == ValueRepresentation::UnlimitedCharacters
      || self == ValueRepresentation::UnlimitedText
  }

  /// Returns whether a value representation stores string data that is UTF-8
  /// encoded and can therefore store any Unicode codepoint.
  ///
  pub fn is_encoded_string(self) -> bool {
    self == ValueRepresentation::LongString
      || self == ValueRepresentation::LongText
      || self == ValueRepresentation::PersonName
      || self == ValueRepresentation::ShortString
      || self == ValueRepresentation::ShortText
      || self == ValueRepresentation::UnlimitedCharacters
      || self == ValueRepresentation::UnlimitedText
  }

  /// Appends the correct padding byte for the given value representation if the
  /// bytes are not of even length.
  ///
  pub fn pad_bytes_to_even_length(self, bytes: &mut Vec<u8>) {
    if bytes.len() % 2 == 0 {
      return;
    }

    // UI uses a zero byte as padding
    if self == ValueRepresentation::UniqueIdentifier {
      bytes.push(0);
    }
    // String values use a space as padding. The rest do not use any padding.
    else if self.is_string() {
      bytes.push(0x20);
    }
  }

  /// Returns the length requirements for a value representation. See the
  /// `LengthRequirements` type for details.
  ///
  pub fn length_requirements(self) -> LengthRequirements {
    match self {
      ValueRepresentation::AgeString => LengthRequirements {
        bytes_max: 4,
        bytes_multiple_of: None,
        string_characters_max: None,
      },
      ValueRepresentation::ApplicationEntity => LengthRequirements {
        bytes_max: 16,
        bytes_multiple_of: None,
        string_characters_max: None,
      },
      ValueRepresentation::AttributeTag => LengthRequirements {
        bytes_max: 0xFFFC,
        bytes_multiple_of: Some(4),
        string_characters_max: None,
      },
      ValueRepresentation::CodeString => LengthRequirements {
        bytes_max: 0xFFFE,
        bytes_multiple_of: None,
        string_characters_max: Some(16),
      },
      ValueRepresentation::Date => LengthRequirements {
        bytes_max: 8,
        bytes_multiple_of: None,
        string_characters_max: None,
      },
      ValueRepresentation::DateTime => LengthRequirements {
        bytes_max: 26,
        bytes_multiple_of: None,
        string_characters_max: None,
      },
      ValueRepresentation::DecimalString => LengthRequirements {
        bytes_max: 0xFFFE,
        bytes_multiple_of: None,
        string_characters_max: Some(16),
      },
      ValueRepresentation::FloatingPointDouble => LengthRequirements {
        bytes_max: 0xFFF8,
        bytes_multiple_of: Some(8),
        string_characters_max: None,
      },
      ValueRepresentation::FloatingPointSingle => LengthRequirements {
        bytes_max: 0xFFFC,
        bytes_multiple_of: Some(4),
        string_characters_max: None,
      },
      ValueRepresentation::IntegerString => LengthRequirements {
        bytes_max: 0xFFFE,
        bytes_multiple_of: None,
        string_characters_max: Some(12),
      },
      ValueRepresentation::LongString => LengthRequirements {
        bytes_max: 0xFFFE,
        bytes_multiple_of: None,
        string_characters_max: Some(64),
      },
      ValueRepresentation::LongText => LengthRequirements {
        bytes_max: 0xFFFE,
        bytes_multiple_of: None,
        string_characters_max: Some(10_240),
      },
      ValueRepresentation::OtherByteString => LengthRequirements {
        bytes_max: 0xFFFFFFFE,
        bytes_multiple_of: Some(2),
        string_characters_max: None,
      },
      ValueRepresentation::OtherDoubleString => LengthRequirements {
        bytes_max: 0xFFFFFFF8,
        bytes_multiple_of: Some(8),
        string_characters_max: None,
      },
      ValueRepresentation::OtherFloatString => LengthRequirements {
        bytes_max: 0xFFFFFFFC,
        bytes_multiple_of: Some(4),
        string_characters_max: None,
      },
      ValueRepresentation::OtherLongString => LengthRequirements {
        bytes_max: 0xFFFFFFFC,
        bytes_multiple_of: Some(4),
        string_characters_max: None,
      },
      ValueRepresentation::OtherVeryLongString => LengthRequirements {
        bytes_max: 0xFFFFFFF8,
        bytes_multiple_of: Some(8),
        string_characters_max: None,
      },
      ValueRepresentation::OtherWordString => LengthRequirements {
        bytes_max: 0xFFFFFFFE,
        bytes_multiple_of: Some(2),
        string_characters_max: None,
      },
      ValueRepresentation::PersonName => LengthRequirements {
        bytes_max: 0xFFFE,
        bytes_multiple_of: None,
        string_characters_max: Some(324),
      },
      ValueRepresentation::Sequence => LengthRequirements {
        bytes_max: 0,
        bytes_multiple_of: None,
        string_characters_max: None,
      },
      ValueRepresentation::ShortString => LengthRequirements {
        bytes_max: 0xFFFE,
        bytes_multiple_of: None,
        string_characters_max: Some(16),
      },
      ValueRepresentation::ShortText => LengthRequirements {
        bytes_max: 0xFFFE,
        bytes_multiple_of: None,
        string_characters_max: Some(1024),
      },
      ValueRepresentation::SignedLong => LengthRequirements {
        bytes_max: 0xFFFC,
        bytes_multiple_of: Some(4),
        string_characters_max: None,
      },
      ValueRepresentation::SignedShort => LengthRequirements {
        bytes_max: 0xFFFE,
        bytes_multiple_of: Some(2),
        string_characters_max: None,
      },
      ValueRepresentation::SignedVeryLong => LengthRequirements {
        bytes_max: 0xFFFFFFF8,
        bytes_multiple_of: Some(8),
        string_characters_max: None,
      },
      ValueRepresentation::Time => LengthRequirements {
        bytes_max: 14,
        bytes_multiple_of: None,
        string_characters_max: None,
      },
      ValueRepresentation::UniqueIdentifier => LengthRequirements {
        bytes_max: 0xFFFE,
        bytes_multiple_of: None,
        string_characters_max: Some(64),
      },
      ValueRepresentation::UniversalResourceIdentifier => LengthRequirements {
        bytes_max: 0xFFFFFFFE,
        bytes_multiple_of: None,
        string_characters_max: None,
      },
      ValueRepresentation::Unknown => LengthRequirements {
        bytes_max: 0xFFFFFFFE,
        bytes_multiple_of: None,
        string_characters_max: None,
      },
      ValueRepresentation::UnlimitedCharacters => LengthRequirements {
        bytes_max: 0xFFFFFFFE,
        bytes_multiple_of: None,
        string_characters_max: None,
      },
      ValueRepresentation::UnlimitedText => LengthRequirements {
        bytes_max: 0xFFFFFFFE,
        bytes_multiple_of: None,
        string_characters_max: None,
      },
      ValueRepresentation::UnsignedLong => LengthRequirements {
        bytes_max: 0xFFFC,
        bytes_multiple_of: Some(4),
        string_characters_max: None,
      },
      ValueRepresentation::UnsignedShort => LengthRequirements {
        bytes_max: 0xFFFE,
        bytes_multiple_of: Some(2),
        string_characters_max: None,
      },
      ValueRepresentation::UnsignedVeryLong => LengthRequirements {
        bytes_max: 0xFFF8,
        bytes_multiple_of: Some(8),
        string_characters_max: None,
      },
    }
  }

  /// Swaps the endianness of data for a value representation.
  ///
  pub fn swap_endianness(self, bytes: &mut [u8]) {
    match self {
      ValueRepresentation::AttributeTag
      | ValueRepresentation::OtherWordString
      | ValueRepresentation::SignedShort
      | ValueRepresentation::UnsignedShort => {
        for i in 0..(bytes.len() / 2) {
          bytes.swap(i * 2, i * 2 + 1);
        }
      }

      ValueRepresentation::FloatingPointSingle
      | ValueRepresentation::OtherFloatString
      | ValueRepresentation::OtherLongString
      | ValueRepresentation::SignedLong
      | ValueRepresentation::UnsignedLong => {
        for i in 0..(bytes.len() / 4) {
          bytes.swap(i * 4, i * 4 + 3);
          bytes.swap(i * 4 + 1, i * 4 + 2);
        }
      }

      ValueRepresentation::FloatingPointDouble
      | ValueRepresentation::OtherDoubleString
      | ValueRepresentation::OtherVeryLongString
      | ValueRepresentation::SignedVeryLong
      | ValueRepresentation::UnsignedVeryLong => {
        for i in 0..(bytes.len() / 8) {
          bytes.swap(i * 8, i * 8 + 7);
          bytes.swap(i * 8 + 1, i * 8 + 6);
          bytes.swap(i * 8 + 2, i * 8 + 5);
          bytes.swap(i * 8 + 3, i * 8 + 4);
        }
      }

      _ => (),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(not(feature = "std"))]
  use alloc::{string::ToString, vec};

  const ALL_VRS: [(ValueRepresentation, &str, &str); 34] = [
    (ValueRepresentation::AgeString, "AS", "AgeString"),
    (
      ValueRepresentation::ApplicationEntity,
      "AE",
      "ApplicationEntity",
    ),
    (ValueRepresentation::AttributeTag, "AT", "AttributeTag"),
    (ValueRepresentation::CodeString, "CS", "CodeString"),
    (ValueRepresentation::Date, "DA", "Date"),
    (ValueRepresentation::DateTime, "DT", "DateTime"),
    (ValueRepresentation::DecimalString, "DS", "DecimalString"),
    (
      ValueRepresentation::FloatingPointDouble,
      "FD",
      "FloatingPointDouble",
    ),
    (
      ValueRepresentation::FloatingPointSingle,
      "FL",
      "FloatingPointSingle",
    ),
    (ValueRepresentation::IntegerString, "IS", "IntegerString"),
    (ValueRepresentation::LongString, "LO", "LongString"),
    (ValueRepresentation::LongText, "LT", "LongText"),
    (
      ValueRepresentation::OtherByteString,
      "OB",
      "OtherByteString",
    ),
    (
      ValueRepresentation::OtherDoubleString,
      "OD",
      "OtherDoubleString",
    ),
    (
      ValueRepresentation::OtherFloatString,
      "OF",
      "OtherFloatString",
    ),
    (
      ValueRepresentation::OtherLongString,
      "OL",
      "OtherLongString",
    ),
    (
      ValueRepresentation::OtherVeryLongString,
      "OV",
      "OtherVeryLongString",
    ),
    (
      ValueRepresentation::OtherWordString,
      "OW",
      "OtherWordString",
    ),
    (ValueRepresentation::PersonName, "PN", "PersonName"),
    (ValueRepresentation::Sequence, "SQ", "Sequence"),
    (ValueRepresentation::ShortString, "SH", "ShortString"),
    (ValueRepresentation::ShortText, "ST", "ShortText"),
    (ValueRepresentation::SignedLong, "SL", "SignedLong"),
    (ValueRepresentation::SignedShort, "SS", "SignedShort"),
    (ValueRepresentation::SignedVeryLong, "SV", "SignedVeryLong"),
    (ValueRepresentation::Time, "TM", "Time"),
    (
      ValueRepresentation::UniqueIdentifier,
      "UI",
      "UniqueIdentifier",
    ),
    (
      ValueRepresentation::UniversalResourceIdentifier,
      "UR",
      "UniversalResourceIdentifier",
    ),
    (ValueRepresentation::Unknown, "UN", "Unknown"),
    (
      ValueRepresentation::UnlimitedCharacters,
      "UC",
      "UnlimitedCharacters",
    ),
    (ValueRepresentation::UnlimitedText, "UT", "UnlimitedText"),
    (ValueRepresentation::UnsignedLong, "UL", "UnsignedLong"),
    (ValueRepresentation::UnsignedShort, "US", "UnsignedShort"),
    (
      ValueRepresentation::UnsignedVeryLong,
      "UV",
      "UnsignedVeryLong",
    ),
  ];

  #[test]
  fn from_bytes_test() {
    for (vr, s, _) in ALL_VRS {
      assert_eq!(ValueRepresentation::from_bytes(s.as_bytes()), Ok(vr));
    }

    assert_eq!(ValueRepresentation::from_bytes(b"XY"), Err(()));
  }

  #[test]
  fn to_string_test() {
    for (vr, s, _) in ALL_VRS {
      assert_eq!(vr.to_string(), s);
    }
  }

  #[test]
  fn name_test() {
    for (vr, _, name) in ALL_VRS {
      assert_eq!(vr.name(), name);
    }
  }

  #[test]
  fn is_string_test() {
    for (vr, _, _) in ALL_VRS {
      assert_eq!(
        vr.is_string(),
        vr == ValueRepresentation::AgeString
          || vr == ValueRepresentation::ApplicationEntity
          || vr == ValueRepresentation::CodeString
          || vr == ValueRepresentation::Date
          || vr == ValueRepresentation::DateTime
          || vr == ValueRepresentation::DecimalString
          || vr == ValueRepresentation::IntegerString
          || vr == ValueRepresentation::LongString
          || vr == ValueRepresentation::LongText
          || vr == ValueRepresentation::PersonName
          || vr == ValueRepresentation::ShortString
          || vr == ValueRepresentation::ShortText
          || vr == ValueRepresentation::Time
          || vr == ValueRepresentation::UniqueIdentifier
          || vr == ValueRepresentation::UniversalResourceIdentifier
          || vr == ValueRepresentation::UnlimitedCharacters
          || vr == ValueRepresentation::UnlimitedText,
      );
    }
  }

  #[test]
  fn is_encoded_string_test() {
    for (vr, _, _) in ALL_VRS {
      assert_eq!(
        vr.is_encoded_string(),
        vr == ValueRepresentation::LongString
          || vr == ValueRepresentation::LongText
          || vr == ValueRepresentation::PersonName
          || vr == ValueRepresentation::ShortString
          || vr == ValueRepresentation::ShortText
          || vr == ValueRepresentation::UnlimitedCharacters
          || vr == ValueRepresentation::UnlimitedText,
      );
    }
  }

  #[test]
  fn pad_bytes_to_even_length_test() {
    let mut bytes = vec![];
    ValueRepresentation::LongText.pad_bytes_to_even_length(&mut bytes);
    assert_eq!(bytes, vec![]);

    let mut bytes = vec![0x41];
    ValueRepresentation::LongText.pad_bytes_to_even_length(&mut bytes);
    assert_eq!(bytes, vec![0x41, 0x20]);

    let mut bytes = vec![0x41];
    ValueRepresentation::UniqueIdentifier.pad_bytes_to_even_length(&mut bytes);
    assert_eq!(bytes, vec![0x41, 0x00]);

    let mut bytes = vec![0x41, 0x42];
    ValueRepresentation::LongText.pad_bytes_to_even_length(&mut bytes);
    assert_eq!(bytes, vec![0x41, 0x42]);
  }

  #[test]
  fn length_requirements_test() {
    assert_eq!(
      ValueRepresentation::AgeString.length_requirements(),
      LengthRequirements {
        bytes_max: 4,
        bytes_multiple_of: None,
        string_characters_max: None,
      }
    );

    assert_eq!(
      ValueRepresentation::AttributeTag.length_requirements(),
      LengthRequirements {
        bytes_max: 0xFFFC,
        bytes_multiple_of: Some(4),
        string_characters_max: None,
      }
    );

    assert_eq!(
      ValueRepresentation::PersonName.length_requirements(),
      LengthRequirements {
        bytes_max: 0xFFFE,
        bytes_multiple_of: None,
        string_characters_max: Some(324),
      }
    );

    assert_eq!(
      ValueRepresentation::Sequence.length_requirements(),
      LengthRequirements {
        bytes_max: 0,
        bytes_multiple_of: None,
        string_characters_max: None,
      }
    );
  }

  #[test]
  fn swap_endianness_test() {
    let mut bytes = [0, 1, 2, 3];
    ValueRepresentation::SignedShort.swap_endianness(&mut bytes);
    assert_eq!(bytes, [1, 0, 3, 2]);

    let mut bytes = [0, 1, 2, 3, 4, 5, 6, 7];
    ValueRepresentation::SignedLong.swap_endianness(&mut bytes);
    assert_eq!(bytes, [3, 2, 1, 0, 7, 6, 5, 4]);

    let mut bytes = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    ValueRepresentation::SignedVeryLong.swap_endianness(&mut bytes);
    assert_eq!(
      bytes,
      [7, 6, 5, 4, 3, 2, 1, 0, 15, 14, 13, 12, 11, 10, 9, 8]
    );

    let mut bytes = [0, 1, 2, 3];
    ValueRepresentation::OtherByteString.swap_endianness(&mut bytes);
    assert_eq!(bytes, [0, 1, 2, 3]);
  }
}
