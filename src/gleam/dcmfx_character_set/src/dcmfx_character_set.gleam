//// Decodes DICOM string data that uses a Specific Character Set into a native
//// UTF-8 string.

import dcmfx_character_set/internal/character_set.{
  type CharacterSet, type CodeElement, type CodeElementPair,
  MultiByteWithExtensions, MultiByteWithoutExtensions, SingleByteWithExtensions,
  SingleByteWithoutExtensions,
}
import dcmfx_character_set/internal/iso_ir_6
import dcmfx_character_set/internal/jis_x_0201
import dcmfx_character_set/string_type.{type StringType}
import gleam/bit_array
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result
import gleam/string

/// A specific character set as defined by the *'(0008,0005) Specific Character
/// Set)'* data element. This is a list of one or more individual character
/// sets.
///
/// When there are multiple character sets in a specific character set it means
/// that Code Extension techniques are being used and that escape sequences may
/// be encountered when decoding.
///
pub opaque type SpecificCharacterSet {
  SpecificCharacterSet(charsets: List(CharacterSet))
}

/// Converts a raw value from a "SpecificCharacterSet" data element into a
/// `SpecificCharacterSet` instance that can be used to decode bytes into a
/// native string.
///
pub fn from_string(
  specific_character_set: String,
) -> Result(SpecificCharacterSet, String) {
  let charsets =
    specific_character_set
    |> string.split("\\")
    |> list.map(string.trim)
    |> list.map(string.uppercase)

  // If the first character set is empty then default it to IR 6, i.e the DICOM
  // default character set
  let charsets = case charsets {
    [""] -> ["ISO_IR 6"]
    ["", c, ..rest] -> ["ISO 2022 IR 6", c, ..rest]
    _ -> charsets
  }

  // Convert to recognized character sets
  let charsets =
    charsets
    |> list.map(character_set.from_string)
    |> result.all
  use charsets <- result.try(charsets)

  // If the first character set does not use extensions then it must be the only
  // one. Conversely, if extensions are in use then all character sets must
  // support them.
  let charsets = case charsets {
    // A single value is always fine
    [_] -> Ok(charsets)

    // If there are multiple values they must all support Code Extensions
    _ -> {
      let has_non_iso_2022_charset =
        charsets
        |> list.find(fn(charset) {
          case charset {
            SingleByteWithoutExtensions(..) | MultiByteWithoutExtensions(..) ->
              True
            _ -> False
          }
        })

      case has_non_iso_2022_charset {
        Ok(_) -> Error("SpecificCharacterSet has multiple non-ISO 2022 values")
        _ ->
          // If ISO 2022 IR 6 isn't specified in the character sets then
          // append it so it can still be used. This isn't mandated by the spec
          // but it improves compatibility.
          case list.contains(charsets, character_set.iso_2022_ir_6) {
            True -> charsets

            False -> list.append(charsets, [character_set.iso_2022_ir_6])
          }
          |> Ok
      }
    }
  }
  use charsets <- result.try(charsets)

  Ok(SpecificCharacterSet(charsets))
}

/// Returns whether a specific character set is byte compatible with UTF-8. This
/// is only the case for the DICOM default character set (ISO_IR 6) and the
/// UTF-8 character set itself (ISO_IR 192).
///
pub fn is_utf8_compatible(specific_character_set: SpecificCharacterSet) -> Bool {
  specific_character_set.charsets == [character_set.iso_ir_6]
  || specific_character_set.charsets == [character_set.iso_ir_192]
}

/// Decodes bytes using a specific character set to a native string.
///
/// Trailing whitespace is automatically removed, and invalid bytes are replaced
/// with the U+FFFD character: �.
///
pub fn decode_bytes(
  specific_character_set: SpecificCharacterSet,
  bytes: BitArray,
  string_type: StringType,
) -> String {
  case specific_character_set {
    SpecificCharacterSet([
      SingleByteWithoutExtensions(
        defined_term: term,
        decoder: decoder,
        ..,
      ),
    ]) -> {
      // When using the ISO_IR 13 character set to decode bytes that support
      // multiplicity, use a variant of JIS X 0201 that allows the backslash
      // character
      let decoder = case term, string_type {
        "ISO_IR 13", string_type.MultiValue
        | "ISO_IR 13", string_type.PersonName
        -> jis_x_0201.decode_next_codepoint_allowing_backslash
        _, _ -> decoder
      }

      character_set.decode_bytes(bytes, decoder, [])
    }

    SpecificCharacterSet([MultiByteWithoutExtensions(decoder: decoder, ..)]) ->
      character_set.decode_bytes(bytes, decoder, [])

    _ ->
      decode_iso_2022_bytes(
        specific_character_set,
        bytes,
        string_type,
        default_code_elements(specific_character_set),
        [],
      )
  }
  |> list.reverse
  |> string.from_utf_codepoints
}

fn decode_iso_2022_bytes(
  specific_character_set: SpecificCharacterSet,
  bytes: BitArray,
  string_type: StringType,
  active_code_elements: CodeElementPair,
  acc: List(UtfCodepoint),
) -> List(UtfCodepoint) {
  case bytes {
    <<>> -> acc

    // Detect escape sequences and use them to update the active code elements
    <<0x1B, rest:bytes>> -> {
      let #(active_code_elements, bytes) =
        apply_escape_sequence(
          specific_character_set,
          rest,
          active_code_elements,
        )

      decode_iso_2022_bytes(
        specific_character_set,
        bytes,
        string_type,
        active_code_elements,
        acc,
      )
    }

    _ -> {
      // Determine the decoder to use
      let decoder = case bytes, active_code_elements {
        // If the byte has its high bit set and there is a G1 code element
        // active then use it
        <<byte, _:bytes>>, #(_, Some(g1)) if byte >= 0x80 -> g1.decoder

        // Otherwise if there is a G0 code element active then use it
        _, #(Some(g0), _) -> g0.decoder

        // Fall back to the default character set
        _, _ -> iso_ir_6.decode_next_codepoint
      }

      // This assert is safe because decoders only error when fed no bytes
      let assert Ok(#(codepoint, bytes)) = decoder(bytes)

      // Detect delimiters and reset code elements to default when they occur
      let active_code_elements = case
        string.utf_codepoint_to_int(codepoint),
        string_type
      {
        0x09, _
        | 0x0A, _
        | 0x0C, _
        | 0x0D, _
        | 0x5C, string_type.MultiValue
        | 0x5C, string_type.PersonName
        | 0x3D, string_type.PersonName
        | 0x5E, string_type.PersonName
        -> default_code_elements(specific_character_set)

        _, _ -> active_code_elements
      }

      decode_iso_2022_bytes(
        specific_character_set,
        bytes,
        string_type,
        active_code_elements,
        [codepoint, ..acc],
      )
    }
  }
}

/// Returns the default G0 and G1 code elements which are the ones specified by
/// the first character set. These are the initially active code elements and
/// they are also reactivated after any delimiter is encountered.
///
fn default_code_elements(
  specific_character_set: SpecificCharacterSet,
) -> CodeElementPair {
  case specific_character_set {
    SpecificCharacterSet([
      SingleByteWithExtensions(
        code_element_g0: code_element_g0,
        code_element_g1: code_element_g1,
        ..,
      ),
      ..
    ]) -> #(Some(code_element_g0), code_element_g1)

    SpecificCharacterSet([
      MultiByteWithExtensions(
        code_element_g0: code_element_g0,
        code_element_g1: code_element_g1,
        ..,
      ),
      ..
    ]) -> #(code_element_g0, code_element_g1)

    _ -> #(None, None)
  }
}

/// Attempts to update the active code elements based on the escape sequence at
/// the start of the given bytes. If the escape sequence isn't for any of the
/// available character sets then nothing happens, i.e. unrecognized escape
/// sequences are ignored.
///
fn apply_escape_sequence(
  specific_character_set: SpecificCharacterSet,
  bytes: BitArray,
  active_code_elements: CodeElementPair,
) -> #(CodeElementPair, BitArray) {
  specific_character_set.charsets
  |> list.fold_until(#(active_code_elements, bytes), fn(current, charset) {
    let code_elements = character_set.code_elements(charset)

    // See if the escape sequence applies to the G0 code element of this
    // character set
    case update_code_element(code_elements.0, bytes) {
      Ok(bytes) -> list.Stop(#(#(code_elements.0, current.0.1), bytes))

      // See if the escape sequence applies to the G1 code element of this
      // character set
      _ ->
        case update_code_element(code_elements.1, bytes) {
          Ok(bytes) -> list.Stop(#(#(current.0.0, code_elements.1), bytes))

          _ -> list.Continue(#(current.0, bytes))
        }
    }
  })
}

fn update_code_element(
  candidate: Option(CodeElement),
  bytes: BitArray,
) -> Result(BitArray, Nil) {
  case candidate {
    Some(candidate) -> {
      let esc = candidate.escape_sequence

      case bit_array.slice(bytes, 0, bit_array.byte_size(esc)) == Ok(esc) {
        True -> {
          let esc_length = bit_array.byte_size(esc)
          let byte_count = bit_array.byte_size(bytes)

          let assert Ok(rest) =
            bit_array.slice(bytes, esc_length, byte_count - esc_length)

          Ok(rest)
        }

        False -> Error(Nil)
      }
    }

    None -> Error(Nil)
  }
}

/// Replaces all bytes greater than 0x7F with the value 0x3F, i.e. the question
/// mark character. This can be used to ensure that only valid ISO 646/US-ASCII
/// bytes are present.
///
pub fn sanitize_default_charset_bytes(bytes: BitArray) -> BitArray {
  do_sanitize_default_charset_bytes(bytes, 0, <<>>)
}

fn do_sanitize_default_charset_bytes(
  bytes: BitArray,
  i: Int,
  acc: BitArray,
) -> BitArray {
  case bit_array.slice(bytes, i, 1) {
    Ok(<<byte>>) ->
      case byte > 0x7F {
        True -> {
          // Get the slices before and after the unwanted byte
          let assert Ok(before) = bit_array.slice(bytes, 0, i)
          let assert Ok(after) =
            bit_array.slice(bytes, i + 1, bit_array.byte_size(bytes) - i - 1)

          let acc = bit_array.concat([acc, before, <<0x3F>>])

          do_sanitize_default_charset_bytes(after, 0, acc)
        }

        False -> do_sanitize_default_charset_bytes(bytes, i + 1, acc)
      }

    _ -> bit_array.concat([acc, bytes])
  }
}
