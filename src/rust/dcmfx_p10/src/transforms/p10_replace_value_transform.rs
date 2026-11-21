#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use bytes::Bytes;

use dcmfx_core::{
  DataElementTag, DataElementValue, DataSetPath, ValueRepresentation,
};

use crate::{P10Error, P10Token};

/// Transform that replaces specific string values of data elements in a stream
/// of DICOM P10 tokens.
///
/// Each replacement specifies the exact path of the data element it applies to,
/// a value to match, and the value to replace it with. Data elements with
/// multiple values have each of their values matched and replaced
/// individually. Values that don't match any replacement, and data elements
/// that have no matching values, pass through unchanged.
///
/// Only data elements with a string value representation can have their values
/// replaced. A data element with any other VR, or a sequence, at a path that a
/// replacement applies to causes an error.
///
pub struct P10ReplaceValueTransform {
  replacements: Vec<ValueReplacement>,
  pending: Option<PendingDataElement>,
}

/// A single value replacement for a [`P10ReplaceValueTransform`].
///
#[derive(Clone, Debug, PartialEq)]
pub struct ValueReplacement {
  /// The path of the data element that this replacement applies to. This must
  /// end with a data element, e.g. `"0008103E"` or `"00081111/[0]/00081150"`.
  pub path: DataSetPath,

  /// The value to match. Leading and trailing padding is ignored when matching,
  /// following the padding rules of the data element's value representation.
  pub value: String,

  /// The value that replaces each matching value.
  pub replacement: String,
}

/// The maximum number of bytes preallocated when gathering a data element's
/// value bytes. Larger values grow the buffer as their bytes are received.
///
const MAX_PREALLOCATION: usize = 64 * 1024;

/// A data element whose value bytes are being gathered so that its values can
/// be matched against the replacements once they've all been received.
///
struct PendingDataElement {
  tag: DataElementTag,
  vr: ValueRepresentation,
  path: DataSetPath,
  data: Vec<u8>,
}

impl P10ReplaceValueTransform {
  /// Creates a new transform that applies the given value replacements to a
  /// stream of DICOM P10 tokens. When more than one replacement matches a
  /// value, the first matching replacement is used. Replacements aren't
  /// chained, i.e. a replaced value isn't matched again against the remaining
  /// replacements.
  ///
  pub fn new(replacements: Vec<ValueReplacement>) -> Self {
    Self {
      replacements,
      pending: None,
    }
  }

  /// Adds the next token to this transform and returns the resulting tokens.
  ///
  pub fn add_token(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<P10Token>, P10Error> {
    if self.replacements.is_empty() {
      return Ok(vec![token.clone()]);
    }

    // If the value bytes of a data element are currently being gathered then
    // add these bytes to it, and process its value once all bytes are present
    if let Some(pending) = self.pending.as_mut() {
      let P10Token::DataElementValueBytes {
        data,
        bytes_remaining,
        ..
      } = token
      else {
        return Err(P10Error::TokenStreamInvalid {
          when: "Adding token to replace value transform".to_string(),
          details: "Expected data element value bytes".to_string(),
          token: token.clone(),
        });
      };

      pending.data.extend_from_slice(data);

      if *bytes_remaining > 0 {
        return Ok(vec![]);
      }

      let pending = self.pending.take().unwrap();
      return self.process_data_element(pending);
    }

    match token {
      P10Token::DataElementHeader {
        tag,
        vr,
        length,
        path,
      } if self.has_replacement_for_path(path) => {
        if !vr.is_string() {
          return Err(Self::vr_not_supported_error(*vr, path));
        }

        // The declared length isn't trusted for preallocation because it comes
        // from the input and may be far larger than the data that's actually
        // present, so the preallocation is capped
        let capacity = (*length as usize).min(MAX_PREALLOCATION);

        self.pending = Some(PendingDataElement {
          tag: *tag,
          vr: *vr,
          path: path.clone(),
          data: Vec::with_capacity(capacity),
        });

        Ok(vec![])
      }

      P10Token::SequenceStart { vr, path, .. }
        if self.has_replacement_for_path(path) =>
      {
        Err(Self::vr_not_supported_error(*vr, path))
      }

      _ => Ok(vec![token.clone()]),
    }
  }

  /// Returns whether any replacement applies to the given data set path.
  ///
  fn has_replacement_for_path(&self, path: &DataSetPath) -> bool {
    self.replacements.iter().any(|r| r.path == *path)
  }

  /// Applies the replacements to a data element whose value bytes have all been
  /// received, and returns the tokens for the resulting data element.
  ///
  fn process_data_element(
    &self,
    pending: PendingDataElement,
  ) -> Result<Vec<P10Token>, P10Error> {
    let PendingDataElement {
      tag,
      vr,
      path,
      data,
    } = pending;

    let original = core::str::from_utf8(&data).map_err(|_| {
      Self::error(&path, "The existing value is not valid UTF-8".to_string())
    })?;

    // Text VRs hold a single value in which a backslash is an ordinary
    // character. All other string VRs use a backslash as the delimiter between
    // multiple values.
    let is_multi_valued = !matches!(
      vr,
      ValueRepresentation::LongText
        | ValueRepresentation::ShortText
        | ValueRepresentation::UnlimitedText
        | ValueRepresentation::UniversalResourceIdentifier
    );

    // Trailing padding isn't significant for any string VR, and is removed so
    // that it isn't retained on the final value if the value is rebuilt
    let unpadded = original.trim_end_matches(['\0', ' ']);

    let values: Vec<&str> = if is_multi_valued {
      unpadded.split('\\').collect()
    } else {
      vec![unpadded]
    };

    let replacements: Vec<_> = self
      .replacements
      .iter()
      .filter(|r| r.path == path)
      .collect();

    // Find the replacement for each value, if one matches
    let mut new_values = Vec::with_capacity(values.len());
    let mut is_modified = false;

    for value in values {
      let trimmed_value = trim_padding(vr, value);

      let replacement = replacements
        .iter()
        .find(|r| trim_padding(vr, &r.value) == trimmed_value);

      if let Some(replacement) = replacement {
        let replacement = replacement.replacement.as_str();

        if is_multi_valued && replacement.contains('\\') {
          return Err(Self::error(
            &path,
            format!(
              "The replacement value \"{replacement}\" contains a backslash, \
               which isn't allowed because it would change the number of \
               values in the data element"
            ),
          ));
        }

        if let Some(max) = vr.length_requirements().string_characters_max
          && replacement.chars().count() > max
        {
          return Err(Self::error(
            &path,
            format!(
              "The replacement value \"{replacement}\" exceeds the maximum \
               length of {max} characters for the {vr} value representation"
            ),
          ));
        }

        new_values.push(replacement);
        is_modified = true;
      } else {
        new_values.push(value);
      }
    }

    // If no values were replaced then pass the original data element through
    // unchanged
    let bytes = if is_modified {
      let mut bytes = new_values.join("\\").into_bytes();
      vr.pad_bytes_to_even_length(&mut bytes);

      // Validate the new value's bytes and length against its VR
      DataElementValue::new_binary(vr, bytes.into())
        .map_err(|e| Self::error(&path, e.to_string()))?
        .bytes()
        .unwrap()
        .clone()
    } else {
      Bytes::from(data)
    };

    Ok(vec![
      P10Token::DataElementHeader {
        tag,
        vr,
        length: bytes.len() as u32,
        path,
      },
      P10Token::DataElementValueBytes {
        tag,
        vr,
        data: bytes,
        bytes_remaining: 0,
      },
    ])
  }

  fn vr_not_supported_error(
    vr: ValueRepresentation,
    path: &DataSetPath,
  ) -> P10Error {
    Self::error(
      path,
      format!(
        "Value replacement isn't supported for data elements with the {vr} \
         value representation, only for those with a string value \
         representation"
      ),
    )
  }

  fn error(path: &DataSetPath, details: String) -> P10Error {
    P10Error::OtherError {
      error_type: "Value replacement failed".to_string(),
      details: format!("{}: {details}", path.to_detailed_string()),
    }
  }
}

/// Removes the padding from a string value that isn't significant for its VR.
/// Text VRs have significant leading spaces, and all other string VRs ignore
/// both leading and trailing spaces. Trailing NUL bytes are always ignored.
///
fn trim_padding(vr: ValueRepresentation, value: &str) -> &str {
  let value = value.trim_end_matches(['\0', ' ']);

  match vr {
    ValueRepresentation::LongText
    | ValueRepresentation::ShortText
    | ValueRepresentation::UnlimitedText
    | ValueRepresentation::UnlimitedCharacters => value,

    _ => value.trim_start_matches(' '),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(not(feature = "std"))]
  use alloc::vec;

  use dcmfx_core::dictionary;

  const SERIES_DESCRIPTION: DataElementTag = dictionary::SERIES_DESCRIPTION.tag;
  const REFERENCED_SOP_SEQUENCE: DataElementTag =
    dictionary::REFERENCED_SOP_SEQUENCE.tag;

  fn replacement(path: &str, value: &str, new_value: &str) -> ValueReplacement {
    ValueReplacement {
      path: DataSetPath::from_string(path).unwrap(),
      value: value.to_string(),
      replacement: new_value.to_string(),
    }
  }

  fn data_element_tokens(
    tag: DataElementTag,
    vr: ValueRepresentation,
    path: DataSetPath,
    chunks: &[&[u8]],
  ) -> Vec<P10Token> {
    let length: usize = chunks.iter().map(|c| c.len()).sum();
    let mut bytes_remaining = length;

    let mut tokens = vec![P10Token::DataElementHeader {
      tag,
      vr,
      length: length as u32,
      path,
    }];

    for chunk in chunks {
      bytes_remaining -= chunk.len();

      tokens.push(P10Token::DataElementValueBytes {
        tag,
        vr,
        data: Bytes::copy_from_slice(chunk),
        bytes_remaining: bytes_remaining as u32,
      });
    }

    tokens
  }

  fn root_tokens(
    tag: DataElementTag,
    vr: ValueRepresentation,
    value: &[u8],
  ) -> Vec<P10Token> {
    data_element_tokens(
      tag,
      vr,
      DataSetPath::new_with_data_element(tag),
      &[value],
    )
  }

  fn run(
    replacements: Vec<ValueReplacement>,
    tokens: Vec<P10Token>,
  ) -> Result<Vec<P10Token>, P10Error> {
    let mut transform = P10ReplaceValueTransform::new(replacements);

    let mut output = vec![];
    for token in tokens {
      output.extend(transform.add_token(&token)?);
    }

    Ok(output)
  }

  #[test]
  fn replaces_matching_value() {
    assert_eq!(
      run(
        vec![replacement("0008103E", "ABC", "1234")],
        root_tokens(
          SERIES_DESCRIPTION,
          ValueRepresentation::LongString,
          b"ABC "
        )
      ),
      Ok(root_tokens(
        SERIES_DESCRIPTION,
        ValueRepresentation::LongString,
        b"1234"
      ))
    );
  }

  #[test]
  fn pads_replacement_to_even_length() {
    assert_eq!(
      run(
        vec![replacement("0008103E", "ABC", "123")],
        root_tokens(
          SERIES_DESCRIPTION,
          ValueRepresentation::LongString,
          b"ABC "
        )
      ),
      Ok(root_tokens(
        SERIES_DESCRIPTION,
        ValueRepresentation::LongString,
        b"123 "
      ))
    );
  }

  #[test]
  fn passes_through_non_matching_value_unchanged() {
    let tokens = data_element_tokens(
      SERIES_DESCRIPTION,
      ValueRepresentation::LongString,
      DataSetPath::new_with_data_element(SERIES_DESCRIPTION),
      &[b" ABCD", b"EF "],
    );

    // The value is reassembled into a single token, but its bytes are
    // unchanged
    assert_eq!(
      run(vec![replacement("0008103E", "ABC", "123")], tokens),
      Ok(root_tokens(
        SERIES_DESCRIPTION,
        ValueRepresentation::LongString,
        b" ABCDEF "
      ))
    );
  }

  #[test]
  fn replaces_value_split_across_tokens() {
    let tokens = data_element_tokens(
      SERIES_DESCRIPTION,
      ValueRepresentation::LongString,
      DataSetPath::new_with_data_element(SERIES_DESCRIPTION),
      &[b"AB", b"C "],
    );

    assert_eq!(
      run(vec![replacement("0008103E", "ABC", "12")], tokens),
      Ok(root_tokens(
        SERIES_DESCRIPTION,
        ValueRepresentation::LongString,
        b"12"
      ))
    );
  }

  #[test]
  fn replaces_individual_values_of_multi_valued_element() {
    let image_type = DataElementTag::new(0x0008, 0x0008);

    assert_eq!(
      run(
        vec![
          replacement("00080008", "DERIVED", "ORIGINAL"),
          replacement("00080008", "SECONDARY", "PRIMARY"),
        ],
        root_tokens(
          image_type,
          ValueRepresentation::CodeString,
          b"DERIVED\\SECONDARY\\AXIAL "
        )
      ),
      Ok(root_tokens(
        image_type,
        ValueRepresentation::CodeString,
        b"ORIGINAL\\PRIMARY\\AXIAL"
      ))
    );
  }

  #[test]
  fn does_not_chain_replacements() {
    assert_eq!(
      run(
        vec![
          replacement("0008103E", "A", "B"),
          replacement("0008103E", "B", "C"),
        ],
        root_tokens(
          SERIES_DESCRIPTION,
          ValueRepresentation::LongString,
          b"A\\B "
        )
      ),
      Ok(root_tokens(
        SERIES_DESCRIPTION,
        ValueRepresentation::LongString,
        b"B\\C "
      ))
    );
  }

  #[test]
  fn treats_backslash_in_text_value_as_ordinary_character() {
    let tag = DataElementTag::new(0x0008, 0x2111);

    assert_eq!(
      run(
        vec![replacement("00082111", "A\\B", "C\\D")],
        root_tokens(tag, ValueRepresentation::ShortText, b"A\\B ")
      ),
      Ok(root_tokens(tag, ValueRepresentation::ShortText, b"C\\D "))
    );
  }

  #[test]
  fn replaces_empty_value() {
    assert_eq!(
      run(
        vec![replacement("0008103E", "", "NONE")],
        root_tokens(SERIES_DESCRIPTION, ValueRepresentation::LongString, b"")
      ),
      Ok(root_tokens(
        SERIES_DESCRIPTION,
        ValueRepresentation::LongString,
        b"NONE"
      ))
    );
  }

  #[test]
  fn only_applies_to_exact_path() {
    let nested_path =
      DataSetPath::from_string("00081199/[0]/0008103E").unwrap();

    let nested_tokens: Vec<P10Token> = vec![
      vec![
        P10Token::SequenceStart {
          tag: REFERENCED_SOP_SEQUENCE,
          vr: ValueRepresentation::Sequence,
          path: DataSetPath::new_with_data_element(REFERENCED_SOP_SEQUENCE),
        },
        P10Token::SequenceItemStart { index: 0 },
      ],
      data_element_tokens(
        SERIES_DESCRIPTION,
        ValueRepresentation::LongString,
        nested_path.clone(),
        &[b"ABC "],
      ),
      vec![
        P10Token::SequenceItemDelimiter,
        P10Token::SequenceDelimiter {
          tag: REFERENCED_SOP_SEQUENCE,
        },
      ],
    ]
    .into_iter()
    .flatten()
    .collect();

    // A root-level path doesn't affect the nested data element
    assert_eq!(
      run(
        vec![replacement("0008103E", "ABC", "12")],
        nested_tokens.clone()
      ),
      Ok(nested_tokens.clone())
    );

    // A nested path affects only the nested data element
    let mut expected = nested_tokens.clone();
    expected[2] = P10Token::DataElementHeader {
      tag: SERIES_DESCRIPTION,
      vr: ValueRepresentation::LongString,
      length: 2,
      path: nested_path,
    };
    expected[3] = P10Token::DataElementValueBytes {
      tag: SERIES_DESCRIPTION,
      vr: ValueRepresentation::LongString,
      data: Bytes::from_static(b"12"),
      bytes_remaining: 0,
    };

    let mut input =
      root_tokens(SERIES_DESCRIPTION, ValueRepresentation::LongString, b"ABC ");
    input.splice(0..0, nested_tokens);

    let mut expected_with_root = expected;
    expected_with_root.extend(root_tokens(
      SERIES_DESCRIPTION,
      ValueRepresentation::LongString,
      b"ABC ",
    ));

    assert_eq!(
      run(
        vec![replacement("00081199/[0]/0008103E", "ABC", "12")],
        input
      ),
      Ok(expected_with_root)
    );
  }

  #[test]
  fn errors_on_non_string_vr() {
    let rows = DataElementTag::new(0x0028, 0x0010);

    assert!(matches!(
      run(
        vec![replacement("00280010", "1", "2")],
        root_tokens(rows, ValueRepresentation::UnsignedShort, &[1, 0])
      ),
      Err(P10Error::OtherError { .. })
    ));
  }

  #[test]
  fn errors_on_sequence() {
    assert!(matches!(
      run(
        vec![replacement("00081199", "A", "B")],
        vec![P10Token::SequenceStart {
          tag: REFERENCED_SOP_SEQUENCE,
          vr: ValueRepresentation::Sequence,
          path: DataSetPath::new_with_data_element(REFERENCED_SOP_SEQUENCE),
        }]
      ),
      Err(P10Error::OtherError { .. })
    ));
  }

  #[test]
  fn errors_on_replacement_that_is_too_long() {
    let station_name = DataElementTag::new(0x0008, 0x1010);

    assert!(matches!(
      run(
        vec![replacement("00081010", "A", "12345678901234567")],
        root_tokens(station_name, ValueRepresentation::ShortString, b"A ")
      ),
      Err(P10Error::OtherError { .. })
    ));
  }

  #[test]
  fn errors_on_replacement_with_backslash_in_multi_valued_vr() {
    assert!(matches!(
      run(
        vec![replacement("0008103E", "A", "B\\C")],
        root_tokens(SERIES_DESCRIPTION, ValueRepresentation::LongString, b"A ")
      ),
      Err(P10Error::OtherError { .. })
    ));
  }

  #[test]
  fn errors_on_replacement_with_invalid_characters() {
    let modality = DataElementTag::new(0x0008, 0x0060);

    assert!(matches!(
      run(
        vec![replacement("00080060", "CT", "Ω")],
        root_tokens(modality, ValueRepresentation::CodeString, b"CT")
      ),
      Err(P10Error::OtherError { .. })
    ));
  }
}
