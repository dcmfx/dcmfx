//! Defines constants for all character sets in the DICOM standard and exposes
//! functions for converting string data stored in a character set into Unicode
//! codepoints.

#[cfg(not(feature = "std"))]
use alloc::{format, string::String};

use crate::internal::gb_18030;
use crate::internal::iso_8859_1;
use crate::internal::iso_8859_2;
use crate::internal::iso_8859_3;
use crate::internal::iso_8859_4;
use crate::internal::iso_8859_5;
use crate::internal::iso_8859_6;
use crate::internal::iso_8859_7;
use crate::internal::iso_8859_8;
use crate::internal::iso_8859_9;
use crate::internal::iso_8859_11;
use crate::internal::iso_8859_15;
use crate::internal::iso_ir_6;
use crate::internal::jis_x_0201;
use crate::internal::jis_x_0208;
use crate::internal::jis_x_0212;
use crate::internal::ks_x_1001;
use crate::internal::utf8;

/// Describes a single character set as defined by the DICOM standard. This
/// holds metadata about the structure of the character set that can be used to
/// decode data that uses it.
///
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum CharacterSet {
  SingleByteWithoutExtensions {
    defined_term: &'static str,
    description: &'static str,
    decoder: DecodeNextCodepointFn,
  },

  SingleByteWithExtensions {
    defined_term: &'static str,
    description: &'static str,
    code_element_g0: CodeElement,
    code_element_g1: Option<CodeElement>,
  },

  MultiByteWithExtensions {
    defined_term: &'static str,
    description: &'static str,
    code_element_g0: Option<CodeElement>,
    code_element_g1: Option<CodeElement>,
  },

  MultiByteWithoutExtensions {
    defined_term: &'static str,
    description: &'static str,
    decoder: DecodeNextCodepointFn,
  },
}

impl CharacterSet {
  pub fn defined_term(&self) -> &'static str {
    match *self {
      CharacterSet::SingleByteWithoutExtensions { defined_term, .. }
      | CharacterSet::SingleByteWithExtensions { defined_term, .. }
      | CharacterSet::MultiByteWithoutExtensions { defined_term, .. }
      | CharacterSet::MultiByteWithExtensions { defined_term, .. } => {
        defined_term
      }
    }
  }
}

/// Describes the G0 or G1 code element for a character set, including its
/// unique escape sequence bytes (either 2 or 3 bytes), and its decoder
/// function.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CodeElement {
  pub escape_sequence: [u8; 3],
  pub decoder: DecodeNextCodepointFn,
}

/// A function that decodes the next codepoint from the given bytes and returns
/// its integer value along with the remaining bytes.
///
/// Returns an error if called with no bytes.
///
pub type DecodeNextCodepointFn = fn(&[u8]) -> Result<(char, &[u8]), ()>;

//
// Single-byte character sets without code extensions.
//

/// ISO IR 6 character set, also known as ISO 646 and US-ASCII.
///
pub const ISO_IR_6: CharacterSet = CharacterSet::SingleByteWithoutExtensions {
  defined_term: "ISO_IR 6",
  description: "Default repertoire",
  decoder: iso_ir_6::decode_next_codepoint,
};

/// ISO IR 100 character set, also known as ISO 8859-1 and Latin-1. Used by many
/// Western European languages.
///
pub const ISO_IR_100: CharacterSet =
  CharacterSet::SingleByteWithoutExtensions {
    defined_term: "ISO_IR 100",
    description: "Latin alphabet No. 1",
    decoder: iso_8859_1::decode_next_codepoint,
  };

/// ISO IR 101 character set, also known as ISO 8859-2 and Latin-2. Used by many
/// Central European languages.
///
pub const ISO_IR_101: CharacterSet =
  CharacterSet::SingleByteWithoutExtensions {
    defined_term: "ISO_IR 101",
    description: "Latin alphabet No. 2",
    decoder: iso_8859_2::decode_next_codepoint,
  };

/// ISO IR 109 character set, also known as ISO 8859-3 and Latin-3. Used by many
/// South European languages.
///
pub const ISO_IR_109: CharacterSet =
  CharacterSet::SingleByteWithoutExtensions {
    defined_term: "ISO_IR 109",
    description: "Latin alphabet No. 3",
    decoder: iso_8859_3::decode_next_codepoint,
  };

/// ISO IR 110 character set, also known as ISO 8859-4 and Latin-4. Used by many
/// North European languages.
///
pub const ISO_IR_110: CharacterSet =
  CharacterSet::SingleByteWithoutExtensions {
    defined_term: "ISO_IR 110",
    description: "Latin alphabet No. 4",
    decoder: iso_8859_4::decode_next_codepoint,
  };

/// ISO IR 144 character set, also known as ISO 8859-5 and Latin/Cyrillic. Used
/// by Slavic languages that use a Cyrillic alphabet.
///
pub const ISO_IR_144: CharacterSet =
  CharacterSet::SingleByteWithoutExtensions {
    defined_term: "ISO_IR 144",
    description: "Cyrillic",
    decoder: iso_8859_5::decode_next_codepoint,
  };

/// ISO IR 127 character set, also known as ISO 8859-6 and Latin/Arabic. Used by
/// the Arabic language.
///
pub const ISO_IR_127: CharacterSet =
  CharacterSet::SingleByteWithoutExtensions {
    defined_term: "ISO_IR 127",
    description: "Arabic",
    decoder: iso_8859_6::decode_next_codepoint,
  };

/// ISO IR 126 character set, also known as ISO 8859-7 and Latin/Greek. Used by
/// the Greek language.
///
pub const ISO_IR_126: CharacterSet =
  CharacterSet::SingleByteWithoutExtensions {
    defined_term: "ISO_IR 126",
    description: "Greek",
    decoder: iso_8859_7::decode_next_codepoint,
  };

/// ISO IR 138 character set, also known as ISO 8859-8 and Latin/Hebrew. Used by
/// the Hebrew language.
///
pub const ISO_IR_138: CharacterSet =
  CharacterSet::SingleByteWithoutExtensions {
    defined_term: "ISO_IR 138",
    description: "Hebrew",
    decoder: iso_8859_8::decode_next_codepoint,
  };

/// ISO IR 148 character set, also known as ISO 8859-9 and Latin-5. Used by the
/// Turkish language.
///
pub const ISO_IR_148: CharacterSet =
  CharacterSet::SingleByteWithoutExtensions {
    defined_term: "ISO_IR 148",
    description: "Latin alphabet No. 5",
    decoder: iso_8859_9::decode_next_codepoint,
  };

/// ISO IR 203 character set, also known as ISO 8859-15 and Latin-9. Used by
/// many languages.
///
pub const ISO_IR_203: CharacterSet =
  CharacterSet::SingleByteWithoutExtensions {
    defined_term: "ISO_IR 203",
    description: "Latin alphabet No. 9",
    decoder: iso_8859_15::decode_next_codepoint,
  };

/// ISO IR 13 character set, also known as JIS X 0201. Used by the Japanese
/// language.
///
pub const ISO_IR_13: CharacterSet = CharacterSet::SingleByteWithoutExtensions {
  defined_term: "ISO_IR 13",
  description: "Japanese",
  decoder: jis_x_0201::decode_next_codepoint,
};

/// ISO IR 166 character set, also known as ISO 8859-11 and TIS 620-2533. Used
/// by the Thai language.
///
pub const ISO_IR_166: CharacterSet =
  CharacterSet::SingleByteWithoutExtensions {
    defined_term: "ISO_IR 166",
    description: "Thai",
    decoder: iso_8859_11::decode_next_codepoint,
  };

//
// Single-byte character sets with code extensions.
//

const ISO_IR_6_CODE_ELEMENT: CodeElement = CodeElement {
  escape_sequence: [0x28, 0x42, 0x00],
  decoder: iso_ir_6::decode_next_codepoint,
};

/// ISO 2022 IR 6 character set, also known as ISO 646 and US-ASCII.
///
pub const ISO_2022_IR_6: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 6",
    description: "Default repertoire",
    code_element_g0: ISO_IR_6_CODE_ELEMENT,
    code_element_g1: None,
  };

/// ISO 2022 IR 100 character set, also known as ISO 8859-1 and Latin-1. Used by
/// many Western European languages.
///
pub const ISO_2022_IR_100: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 100",
    description: "Latin alphabet No. 1",
    code_element_g0: ISO_IR_6_CODE_ELEMENT,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x2D, 0x41, 0x00],
      decoder: iso_8859_1::decode_next_codepoint,
    }),
  };

/// ISO 2022 IR 101 character set, also known as ISO 8859-2 and Latin-2. Used by
/// many Central European languages.
///
pub const ISO_2022_IR_101: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 101",
    description: "Latin alphabet No. 2",
    code_element_g0: ISO_IR_6_CODE_ELEMENT,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x2D, 0x42, 0x00],
      decoder: iso_8859_2::decode_next_codepoint,
    }),
  };

/// ISO 2022 IR 109 character set, also known as ISO 8859-3 and Latin-3. Used by
/// many South European languages.
///
pub const ISO_2022_IR_109: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 109",
    description: "Latin alphabet No. 3",
    code_element_g0: ISO_IR_6_CODE_ELEMENT,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x2D, 0x43, 0x00],
      decoder: iso_8859_3::decode_next_codepoint,
    }),
  };

/// ISO 2022 IR 110 character set, also known as ISO 8859-4 and Latin-4. Used by
/// many North European languages.
///
pub const ISO_2022_IR_110: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 110",
    description: "Latin alphabet No. 4",
    code_element_g0: ISO_IR_6_CODE_ELEMENT,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x2D, 0x44, 0x00],
      decoder: iso_8859_4::decode_next_codepoint,
    }),
  };

/// ISO 2022 IR 144 character set, also known as ISO 8859-5 and Latin/Cyrillic.
/// Used by Slavic languages that use a Cyrillic alphabet.
///
pub const ISO_2022_IR_144: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 144",
    description: "Cyrillic",
    code_element_g0: ISO_IR_6_CODE_ELEMENT,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x2D, 0x4C, 0x00],
      decoder: iso_8859_5::decode_next_codepoint,
    }),
  };

/// ISO 2022 IR 127 character set, also known as ISO 8859-6 and Latin/Arabic.
/// Used by the Arabic language.
///
pub const ISO_2022_IR_127: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 127",
    description: "Arabic",
    code_element_g0: ISO_IR_6_CODE_ELEMENT,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x2D, 0x47, 0x00],
      decoder: iso_8859_6::decode_next_codepoint,
    }),
  };

/// ISO 2022 IR 126 character set, also known as ISO 8859-7 and Latin/Greek.
/// Used by the Greek language.
///
pub const ISO_2022_IR_126: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 126",
    description: "Greek",
    code_element_g0: ISO_IR_6_CODE_ELEMENT,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x2D, 0x46, 0x00],
      decoder: iso_8859_7::decode_next_codepoint,
    }),
  };

/// ISO 2022 IR 138 character set, also known as ISO 8859-8 and Latin/Hebrew.
/// Used by the Hebrew language.
///
pub const ISO_2022_IR_138: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 138",
    description: "Hebrew",
    code_element_g0: ISO_IR_6_CODE_ELEMENT,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x2D, 0x48, 0x00],
      decoder: iso_8859_8::decode_next_codepoint,
    }),
  };

/// ISO 2022 IR 148 character set, also known as ISO 8859-9 and Latin-5. Used by
/// the Turkish language.
///
pub const ISO_2022_IR_148: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 148",
    description: "Latin alphabet No. 5",
    code_element_g0: ISO_IR_6_CODE_ELEMENT,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x2D, 0x4D, 0x00],
      decoder: iso_8859_9::decode_next_codepoint,
    }),
  };

/// ISO 2022 IR 203 character set, also known as ISO 8859-15 and Latin-9. Used
/// by many languages.
///
pub const ISO_2022_IR_203: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 203",
    description: "Latin alphabet No. 9",
    code_element_g0: ISO_IR_6_CODE_ELEMENT,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x2D, 0x62, 0x00],
      decoder: iso_8859_15::decode_next_codepoint,
    }),
  };

/// ISO 2022 IR 13 character set, also known as JIS X 0201. Used by the Japanese
/// language.
///
pub const ISO_2022_IR_13: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 13",
    description: "Japanese",
    code_element_g0: CodeElement {
      escape_sequence: [0x28, 0x4A, 0x00],
      decoder: jis_x_0201::decode_next_codepoint,
    },
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x29, 0x49, 0x00],
      decoder: jis_x_0201::decode_next_codepoint,
    }),
  };

/// ISO 2022 IR 166 character set, also known as ISO 8859-11 and TIS 620-2533.
/// Used by the Thai language.
///
pub const ISO_2022_IR_166: CharacterSet =
  CharacterSet::SingleByteWithExtensions {
    defined_term: "ISO 2022 IR 166",
    description: "Thai",
    code_element_g0: ISO_IR_6_CODE_ELEMENT,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x2D, 0x54, 0x00],
      decoder: iso_8859_11::decode_next_codepoint,
    }),
  };

//
// Multi-byte character sets with code extensions.
//

/// ISO 2022 IR 87 character set, also known as JIS X 0208. Used by the Japanese
/// language.
///
pub const ISO_2022_IR_87: CharacterSet =
  CharacterSet::MultiByteWithExtensions {
    defined_term: "ISO 2022 IR 87",
    description: "Japanese",
    code_element_g0: Some(CodeElement {
      escape_sequence: [0x24, 0x42, 0x00],
      decoder: jis_x_0208::decode_next_codepoint,
    }),
    code_element_g1: None,
  };

/// ISO 2022 IR 159 character set, also known as JIS X 0212. Used by the
/// Japanese language.
///
pub const ISO_2022_IR_159: CharacterSet =
  CharacterSet::MultiByteWithExtensions {
    defined_term: "ISO 2022 IR 159",
    description: "Japanese",
    code_element_g0: Some(CodeElement {
      escape_sequence: [0x24, 0x28, 0x44],
      decoder: jis_x_0212::decode_next_codepoint,
    }),
    code_element_g1: None,
  };

/// ISO 2022 IR 149 character set, also known as KS X 1001. Used by the Korean
/// language.
///
pub const ISO_2022_IR_149: CharacterSet =
  CharacterSet::MultiByteWithExtensions {
    defined_term: "ISO 2022 IR 149",
    description: "Korean",
    code_element_g0: None,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x24, 0x29, 0x43],
      decoder: ks_x_1001::decode_next_codepoint,
    }),
  };

/// ISO 2022 IR 58 character set, also known as GB 2312. Used by the Chinese
/// language.
///
pub const ISO_2022_IR_58: CharacterSet =
  CharacterSet::MultiByteWithExtensions {
    defined_term: "ISO 2022 IR 58",
    description: "Simplified Chinese",
    code_element_g0: None,
    code_element_g1: Some(CodeElement {
      escape_sequence: [0x24, 0x29, 0x41],
      decoder: gb_18030::decode_next_codepoint,
    }),
  };

//
// Multi-byte character sets without code extensions.
//

/// ISO IR 192 character set, also known as UTF-8. Used by all languages.
///
pub const ISO_IR_192: CharacterSet = CharacterSet::MultiByteWithoutExtensions {
  defined_term: "ISO_IR 192",
  description: "Unicode in UTF-8",
  decoder: utf8::decode_next_codepoint,
};

/// GB 18030 character set. Used by the Chinese language.
///
pub const GB_18030: CharacterSet = CharacterSet::MultiByteWithoutExtensions {
  defined_term: "GB18030",
  description: "GB 18030",
  decoder: gb_18030::decode_next_codepoint,
};

/// GBK character set. Used by the Chinese language.
///
pub const GBK: CharacterSet = CharacterSet::MultiByteWithoutExtensions {
  defined_term: "GBK",
  description: "GBK",
  decoder: gb_18030::decode_next_codepoint,
};

/// The list of all DICOM character sets, in the order in which they appear in
/// the DICOM standard: single-byte character sets without extensions,
/// single-byte character sets with extensions, multi-byte character sets with
/// extensions, multi-byte character sets without extensions.
///
pub const ALL_CHARACTER_SETS: [&CharacterSet; 33] = [
  &ISO_IR_6,
  &ISO_IR_100,
  &ISO_IR_101,
  &ISO_IR_109,
  &ISO_IR_110,
  &ISO_IR_144,
  &ISO_IR_127,
  &ISO_IR_126,
  &ISO_IR_138,
  &ISO_IR_148,
  &ISO_IR_203,
  &ISO_IR_13,
  &ISO_IR_166,
  &ISO_2022_IR_6,
  &ISO_2022_IR_100,
  &ISO_2022_IR_101,
  &ISO_2022_IR_109,
  &ISO_2022_IR_110,
  &ISO_2022_IR_144,
  &ISO_2022_IR_127,
  &ISO_2022_IR_126,
  &ISO_2022_IR_138,
  &ISO_2022_IR_148,
  &ISO_2022_IR_203,
  &ISO_2022_IR_13,
  &ISO_2022_IR_166,
  &ISO_2022_IR_87,
  &ISO_2022_IR_159,
  &ISO_2022_IR_149,
  &ISO_2022_IR_58,
  &ISO_IR_192,
  &GB_18030,
  &GBK,
];

/// Converts a string containing the 'Defined Term' for a character set in the
/// DICOM standard into a `CharacterSet` instance.
///
/// If the passed term isn't recognized then an error is returned.
///
pub fn from_string(
  defined_term: String,
) -> Result<&'static CharacterSet, String> {
  fn standardize_defined_term(term: &str) -> String {
    term.replace(&[' ', '-', '_'][..], "")
  }

  let charset = standardize_defined_term(&defined_term);

  for character_set in ALL_CHARACTER_SETS {
    if standardize_defined_term(character_set.defined_term()) == charset {
      return Ok(character_set);
    }
  }

  Err(format!("Invalid character set: {defined_term:?}"))
}

/// Decodes bytes into a string using the specified decoder.
///
pub fn decode_bytes(
  mut bytes: &[u8],
  decoder: DecodeNextCodepointFn,
) -> String {
  let mut s = String::with_capacity(bytes.len());

  loop {
    match decoder(bytes) {
      Ok((char, rest)) => {
        let mut char_utf8: [u8; 4] = [0, 0, 0, 0];
        s.push_str(char.encode_utf8(&mut char_utf8));

        bytes = rest;
      }

      Err(()) => return s,
    }
  }
}

/// A pair of G0/G1 code elements.
///
pub type CodeElementPair = (Option<CodeElement>, Option<CodeElement>);

impl CharacterSet {
  /// Returns the G0 and G1 code elements for a character set.
  ///
  pub fn code_elements(&self) -> CodeElementPair {
    match self {
      CharacterSet::SingleByteWithExtensions {
        code_element_g0,
        code_element_g1,
        ..
      } => (Some(*code_element_g0), *code_element_g1),

      CharacterSet::MultiByteWithExtensions {
        code_element_g0,
        code_element_g1,
        ..
      } => (*code_element_g0, *code_element_g1),

      _ => (None, None), // grcov-excl-line
    }
  }
}
