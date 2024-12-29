//// Defines constants for all character sets in the DICOM standard and exposes
//// functions for converting string data stored in a character set into Unicode
//// codepoints.

import dcmfx_character_set/internal/gb_18030
import dcmfx_character_set/internal/iso_8859_1
import dcmfx_character_set/internal/iso_8859_11
import dcmfx_character_set/internal/iso_8859_15
import dcmfx_character_set/internal/iso_8859_2
import dcmfx_character_set/internal/iso_8859_3
import dcmfx_character_set/internal/iso_8859_4
import dcmfx_character_set/internal/iso_8859_5
import dcmfx_character_set/internal/iso_8859_6
import dcmfx_character_set/internal/iso_8859_7
import dcmfx_character_set/internal/iso_8859_8
import dcmfx_character_set/internal/iso_8859_9
import dcmfx_character_set/internal/iso_ir_6
import dcmfx_character_set/internal/jis_x_0201
import dcmfx_character_set/internal/jis_x_0208
import dcmfx_character_set/internal/jis_x_0212
import dcmfx_character_set/internal/ks_x_1001
import dcmfx_character_set/internal/utf8
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result
import gleam/string

/// Describes a single character set as defined by the DICOM standard. This
/// holds metadata about the structure of the character set that can be used to
/// decode data that uses it.
///
pub type CharacterSet {
  SingleByteWithoutExtensions(
    defined_term: String,
    description: String,
    decoder: DecodeNextCodepointFn,
  )

  SingleByteWithExtensions(
    defined_term: String,
    description: String,
    code_element_g0: CodeElement,
    code_element_g1: Option(CodeElement),
  )

  MultiByteWithExtensions(
    defined_term: String,
    description: String,
    code_element_g0: Option(CodeElement),
    code_element_g1: Option(CodeElement),
  )

  MultiByteWithoutExtensions(
    defined_term: String,
    description: String,
    decoder: DecodeNextCodepointFn,
  )
}

/// Describes the G0 or G1 code element for a character set, including its
/// unique escape sequence bytes (either 2 or 3 bytes), and its decoder
/// function.
///
pub type CodeElement {
  CodeElement(escape_sequence: BitArray, decoder: DecodeNextCodepointFn)
}

/// A function that decodes the next codepoint from the given bytes and returns
/// its integer value along with the remaining bytes.
///
/// Returns an error if called with no bytes.
///
pub type DecodeNextCodepointFn =
  fn(BitArray) -> Result(#(UtfCodepoint, BitArray), Nil)

//
// Single-byte character sets without code extensions.
//

/// ISO IR 6 character set, also known as ISO 646 and US-ASCII.
///
pub const iso_ir_6 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 6",
  description: "Default repertoire",
  decoder: iso_ir_6.decode_next_codepoint,
)

/// ISO IR 100 character set, also known as ISO 8859-1 and Latin-1. Used by many
/// Western European languages.
///
pub const iso_ir_100 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 100",
  description: "Latin alphabet No. 1",
  decoder: iso_8859_1.decode_next_codepoint,
)

/// ISO IR 101 character set, also known as ISO 8859-2 and Latin-2. Used by many
/// Central European languages.
///
pub const iso_ir_101 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 101",
  description: "Latin alphabet No. 2",
  decoder: iso_8859_2.decode_next_codepoint,
)

/// ISO IR 109 character set, also known as ISO 8859-3 and Latin-3. Used by many
/// South European languages.
///
pub const iso_ir_109 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 109",
  description: "Latin alphabet No. 3",
  decoder: iso_8859_3.decode_next_codepoint,
)

/// ISO IR 110 character set, also known as ISO 8859-4 and Latin-4. Used by many
/// North European languages.
///
pub const iso_ir_110 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 110",
  description: "Latin alphabet No. 4",
  decoder: iso_8859_4.decode_next_codepoint,
)

/// ISO IR 144 character set, also known as ISO 8859-5 and Latin/Cyrillic. Used
/// by Slavic languages that use a Cyrillic alphabet.
///
pub const iso_ir_144 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 144",
  description: "Cyrillic",
  decoder: iso_8859_5.decode_next_codepoint,
)

/// ISO IR 127 character set, also known as ISO 8859-6 and Latin/Arabic. Used by
/// the Arabic language.
///
pub const iso_ir_127 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 127",
  description: "Arabic",
  decoder: iso_8859_6.decode_next_codepoint,
)

/// ISO IR 126 character set, also known as ISO 8859-7 and Latin/Greek. Used by
/// the Greek language.
///
pub const iso_ir_126 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 126",
  description: "Greek",
  decoder: iso_8859_7.decode_next_codepoint,
)

/// ISO IR 138 character set, also known as ISO 8859-8 and Latin/Hebrew. Used by
/// the Hebrew language.
///
pub const iso_ir_138 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 138",
  description: "Hebrew",
  decoder: iso_8859_8.decode_next_codepoint,
)

/// ISO IR 148 character set, also known as ISO 8859-9 and Latin-5. Used by the
/// Turkish language.
///
pub const iso_ir_148 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 148",
  description: "Latin alphabet No. 5",
  decoder: iso_8859_9.decode_next_codepoint,
)

/// ISO IR 203 character set, also known as ISO 8859-15 and Latin-9. Used by
/// many languages.
///
pub const iso_ir_203 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 203",
  description: "Latin alphabet No. 9",
  decoder: iso_8859_15.decode_next_codepoint,
)

/// ISO IR 13 character set, also known as JIS X 0201. Used by the Japanese
/// language.
///
pub const iso_ir_13 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 13",
  description: "Japanese",
  decoder: jis_x_0201.decode_next_codepoint,
)

/// ISO IR 166 character set, also known as ISO 8859-11 and TIS 620-2533. Used
/// by the Thai language.
///
pub const iso_ir_166 = SingleByteWithoutExtensions(
  defined_term: "ISO_IR 166",
  description: "Thai",
  decoder: iso_8859_11.decode_next_codepoint,
)

//
// Single-byte character sets with code extensions.
//

const iso_ir_6_code_element = CodeElement(
  escape_sequence: <<0x28, 0x42>>,
  decoder: iso_ir_6.decode_next_codepoint,
)

/// ISO 2022 IR 6 character set, also known as ISO 646 and US-ASCII.
///
pub const iso_2022_ir_6 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 6",
  description: "Default repertoire",
  code_element_g0: iso_ir_6_code_element,
  code_element_g1: None,
)

/// ISO 2022 IR 100 character set, also known as ISO 8859-1 and Latin-1. Used by
/// many Western European languages.
///
pub const iso_2022_ir_100 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 100",
  description: "Latin alphabet No. 1",
  code_element_g0: iso_ir_6_code_element,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x2D, 0x41>>,
      decoder: iso_8859_1.decode_next_codepoint,
    ),
  ),
)

/// ISO 2022 IR 101 character set, also known as ISO 8859-2 and Latin-2. Used by
/// many Central European languages.
///
pub const iso_2022_ir_101 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 101",
  description: "Latin alphabet No. 2",
  code_element_g0: iso_ir_6_code_element,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x2D, 0x42>>,
      decoder: iso_8859_2.decode_next_codepoint,
    ),
  ),
)

/// ISO 2022 IR 109 character set, also known as ISO 8859-3 and Latin-3. Used by
/// many South European languages.
///
pub const iso_2022_ir_109 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 109",
  description: "Latin alphabet No. 3",
  code_element_g0: iso_ir_6_code_element,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x2D, 0x43>>,
      decoder: iso_8859_3.decode_next_codepoint,
    ),
  ),
)

/// ISO 2022 IR 110 character set, also known as ISO 8859-4 and Latin-4. Used by
/// many North European languages.
///
pub const iso_2022_ir_110 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 110",
  description: "Latin alphabet No. 4",
  code_element_g0: iso_ir_6_code_element,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x2D, 0x44>>,
      decoder: iso_8859_4.decode_next_codepoint,
    ),
  ),
)

/// ISO 2022 IR 144 character set, also known as ISO 8859-5 and Latin/Cyrillic.
/// Used by Slavic languages that use a Cyrillic alphabet.
///
pub const iso_2022_ir_144 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 144",
  description: "Cyrillic",
  code_element_g0: iso_ir_6_code_element,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x2D, 0x4C>>,
      decoder: iso_8859_5.decode_next_codepoint,
    ),
  ),
)

/// ISO 2022 IR 127 character set, also known as ISO 8859-6 and Latin/Arabic.
/// Used by the Arabic language.
///
pub const iso_2022_ir_127 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 127",
  description: "Arabic",
  code_element_g0: iso_ir_6_code_element,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x2D, 0x47>>,
      decoder: iso_8859_6.decode_next_codepoint,
    ),
  ),
)

/// ISO 2022 IR 126 character set, also known as ISO 8859-7 and Latin/Greek.
/// Used by the Greek language.
///
pub const iso_2022_ir_126 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 126",
  description: "Greek",
  code_element_g0: iso_ir_6_code_element,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x2D, 0x46>>,
      decoder: iso_8859_7.decode_next_codepoint,
    ),
  ),
)

/// ISO 2022 IR 138 character set, also known as ISO 8859-8 and Latin/Hebrew.
/// Used by the Hebrew language.
///
pub const iso_2022_ir_138 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 138",
  description: "Hebrew",
  code_element_g0: iso_ir_6_code_element,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x2D, 0x48>>,
      decoder: iso_8859_8.decode_next_codepoint,
    ),
  ),
)

/// ISO 2022 IR 148 character set, also known as ISO 8859-9 and Latin-5. Used by
/// the Turkish language.
///
pub const iso_2022_ir_148 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 148",
  description: "Latin alphabet No. 5",
  code_element_g0: iso_ir_6_code_element,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x2D, 0x4D>>,
      decoder: iso_8859_9.decode_next_codepoint,
    ),
  ),
)

/// ISO 2022 IR 203 character set, also known as ISO 8859-15 and Latin-9. Used
/// by many languages.
///
pub const iso_2022_ir_203 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 203",
  description: "Latin alphabet No. 9",
  code_element_g0: iso_ir_6_code_element,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x2D, 0x62>>,
      decoder: iso_8859_15.decode_next_codepoint,
    ),
  ),
)

/// ISO 2022 IR 13 character set, also known as JIS X 0201. Used by the Japanese
/// language.
///
pub const iso_2022_ir_13 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 13",
  description: "Japanese",
  code_element_g0: CodeElement(
    escape_sequence: <<0x28, 0x4A>>,
    decoder: jis_x_0201.decode_next_codepoint,
  ),
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x29, 0x49>>,
      decoder: jis_x_0201.decode_next_codepoint,
    ),
  ),
)

/// ISO 2022 IR 166 character set, also known as ISO 8859-11 and TIS 620-2533.
/// Used by the Thai language.
///
pub const iso_2022_ir_166 = SingleByteWithExtensions(
  defined_term: "ISO 2022 IR 166",
  description: "Thai",
  code_element_g0: iso_ir_6_code_element,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x2D, 0x54>>,
      decoder: iso_8859_11.decode_next_codepoint,
    ),
  ),
)

//
// Multi-byte character sets with code extensions.
//

/// ISO 2022 IR 87 character set, also known as JIS X 0208. Used by the Japanese
/// language.
///
pub const iso_2022_ir_87 = MultiByteWithExtensions(
  defined_term: "ISO 2022 IR 87",
  description: "Japanese",
  code_element_g0: Some(
    CodeElement(
      escape_sequence: <<0x24, 0x42>>,
      decoder: jis_x_0208.decode_next_codepoint,
    ),
  ),
  code_element_g1: None,
)

/// ISO 2022 IR 159 character set, also known as JIS X 0212. Used by the
/// Japanese language.
///
pub const iso_2022_ir_159 = MultiByteWithExtensions(
  defined_term: "ISO 2022 IR 159",
  description: "Japanese",
  code_element_g0: Some(
    CodeElement(
      escape_sequence: <<0x24, 0x28, 0x44>>,
      decoder: jis_x_0212.decode_next_codepoint,
    ),
  ),
  code_element_g1: None,
)

/// ISO 2022 IR 149 character set, also known as KS X 1001. Used by the Korean
/// language.
///
pub const iso_2022_ir_149 = MultiByteWithExtensions(
  defined_term: "ISO 2022 IR 149",
  description: "Korean",
  code_element_g0: None,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x24, 0x29, 0x43>>,
      decoder: ks_x_1001.decode_next_codepoint,
    ),
  ),
)

/// ISO 2022 IR 58 character set, also known as GB 2312. Used by the Chinese
/// language.
///
pub const iso_2022_ir_58 = MultiByteWithExtensions(
  defined_term: "ISO 2022 IR 58",
  description: "Simplified Chinese",
  code_element_g0: None,
  code_element_g1: Some(
    CodeElement(
      escape_sequence: <<0x24, 0x29, 0x41>>,
      decoder: gb_18030.decode_next_codepoint,
    ),
  ),
)

//
// Multi-byte character sets without code extensions.
//

/// ISO IR 192 character set, also known as UTF-8. Used by all languages.
///
pub const iso_ir_192 = MultiByteWithoutExtensions(
  defined_term: "ISO_IR 192",
  description: "Unicode in UTF-8",
  decoder: utf8.decode_next_codepoint,
)

/// GB 18030 character set. Used by the Chinese language.
///
pub const gb_18030 = MultiByteWithoutExtensions(
  defined_term: "GB18030",
  description: "GB 18030",
  decoder: gb_18030.decode_next_codepoint,
)

/// GBK character set. Used by the Chinese language.
///
pub const gbk = MultiByteWithoutExtensions(
  defined_term: "GBK",
  description: "GBK",
  decoder: gb_18030.decode_next_codepoint,
)

/// The list of all DICOM character sets, in the order in which they appear in
/// the DICOM standard: single-byte character sets without extensions,
/// single-byte character sets with extensions, multi-byte character sets with
/// extensions, multi-byte character sets without extensions.
///
pub const all_character_sets = [
  iso_ir_6,
  iso_ir_100,
  iso_ir_101,
  iso_ir_109,
  iso_ir_110,
  iso_ir_144,
  iso_ir_127,
  iso_ir_126,
  iso_ir_138,
  iso_ir_148,
  iso_ir_203,
  iso_ir_13,
  iso_ir_166,
  iso_2022_ir_6,
  iso_2022_ir_100,
  iso_2022_ir_101,
  iso_2022_ir_109,
  iso_2022_ir_110,
  iso_2022_ir_144,
  iso_2022_ir_127,
  iso_2022_ir_126,
  iso_2022_ir_138,
  iso_2022_ir_148,
  iso_2022_ir_203,
  iso_2022_ir_13,
  iso_2022_ir_166,
  iso_2022_ir_87,
  iso_2022_ir_159,
  iso_2022_ir_149,
  iso_2022_ir_58,
  iso_ir_192,
  gb_18030,
  gbk,
]

/// Converts a string containing the 'Defined Term' for a character set in the
/// DICOM standard into a `CharacterSet` instance.
///
/// If the passed term isn't recognized then an error is returned.
///
pub fn from_string(defined_term: String) -> Result(CharacterSet, String) {
  let standardize_defined_term = fn(term: String) {
    term
    |> string.replace(" ", "")
    |> string.replace("-", "")
    |> string.replace("_", "")
  }

  let charset = standardize_defined_term(defined_term)

  all_character_sets
  |> list.find(fn(character_set) {
    standardize_defined_term(character_set.defined_term) == charset
  })
  |> result.map_error(fn(_) { "Invalid character set: " <> defined_term })
}

/// Decodes bytes into Unicode codepoints using the specified decoder. The list
/// is returned in reverse order, i.e. with the last codepoint at the list's
/// head.
///
pub fn decode_bytes(
  bytes: BitArray,
  decoder: DecodeNextCodepointFn,
  acc: List(UtfCodepoint),
) -> List(UtfCodepoint) {
  case decoder(bytes) {
    Ok(#(codepoint, rest)) -> decode_bytes(rest, decoder, [codepoint, ..acc])
    Error(Nil) -> acc
  }
}

/// A pair of G0/G1 code elements.
///
pub type CodeElementPair =
  #(Option(CodeElement), Option(CodeElement))

/// Returns the G0 and G1 code elements for a character set.
///
pub fn code_elements(character_set: CharacterSet) -> CodeElementPair {
  case character_set {
    SingleByteWithExtensions(
      code_element_g0: code_element_g0,
      code_element_g1: code_element_g1,
      ..,
    ) -> #(Some(code_element_g0), code_element_g1)

    MultiByteWithExtensions(
      code_element_g0: code_element_g0,
      code_element_g1: code_element_g1,
      ..,
    ) -> #(code_element_g0, code_element_g1)

    _ -> #(None, None)
  }
}
