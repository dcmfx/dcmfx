#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::ToString, vec, vec::Vec};

use dcmfx_core::{DataElementTag, DataElementValue, DataSet, DataSetPath};

use crate::{P10Error, P10FilterTransform, P10Token, p10_token};

/// Transform that inserts data elements into a stream of DICOM P10 tokens.
///
pub struct P10InsertTransform {
  data_elements_to_insert: Vec<(DataElementTag, DataElementValue)>,
  filter_transform: P10FilterTransform,
}

impl P10InsertTransform {
  /// Creates a new context for inserting data elements into the root data set
  /// of a stream of DICOM P10 tokens.
  ///
  pub fn new(data_elements_to_insert: DataSet) -> Self {
    let tags_to_insert = data_elements_to_insert.tags();

    // Create a filter transform that filters out the data elements that are
    // going to be inserted. This ensures there are no duplicate data elements
    // in the resulting token stream.
    let filter_transform =
      P10FilterTransform::new(Box::new(move |tag, _vr, _length, path| {
        !path.is_root() || !tags_to_insert.contains(&tag)
      }));

    Self {
      data_elements_to_insert: data_elements_to_insert
        .into_iter()
        .rev()
        .collect(),
      filter_transform,
    }
  }

  /// Adds the next available token to the P10 insert transform and returns the
  /// resulting tokens.
  ///
  pub fn add_token(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<P10Token>, P10Error> {
    // If there are no more data elements to be inserted then pass the token
    // straight through
    if self.data_elements_to_insert.is_empty() {
      return Ok(vec![token.clone()]);
    }

    let is_at_root = self.filter_transform.is_at_root();

    // Pass the token through the filter transform
    if !self.filter_transform.add_token(token)? {
      return Ok(vec![]);
    }

    // Data element insertion is only supported in the root data set, so if the
    // stream is not at the root data set then there's nothing to do
    if !is_at_root {
      return Ok(vec![token.clone()]);
    }

    let mut output_tokens = vec![];

    match &token {
      // If this token is the start of a new data element, and there are data
      // elements still to be inserted, then insert any that should appear prior
      // to this next data element
      P10Token::SequenceStart { tag, path, .. }
      | P10Token::DataElementHeader { tag, path, .. } => {
        while let Some(data_element) = self.data_elements_to_insert.pop() {
          if data_element.0.to_int() >= tag.to_int() {
            self.data_elements_to_insert.push(data_element);
            break;
          }

          let mut path = path.clone();
          path
            .pop()
            .and_then(|path| path.add_data_element(data_element.0))
            .map_err(|_| P10Error::TokenStreamInvalid {
              when: "Adding token to insert transform".to_string(),
              details: "Failed altering path for data element to insert"
                .to_string(),
              token: token.clone(),
            })?;

          self.append_data_element_tokens(
            data_element,
            &path,
            &mut output_tokens,
          );
        }

        output_tokens.push(token.clone());
      }

      // If this token is the end of the P10 tokens and there are still data
      // elements to be inserted then insert them now prior to the end
      P10Token::End => {
        while let Some(data_element) = self.data_elements_to_insert.pop() {
          let tag = data_element.0;

          self.append_data_element_tokens(
            data_element,
            &DataSetPath::new_with_data_element(tag),
            &mut output_tokens,
          );
        }

        output_tokens.push(P10Token::End);
      }

      _ => output_tokens.push(token.clone()),
    };

    Ok(output_tokens)
  }

  fn append_data_element_tokens(
    &self,
    data_element: (DataElementTag, DataElementValue),
    path: &DataSetPath,
    output_tokens: &mut Vec<P10Token>,
  ) {
    p10_token::data_element_to_tokens::<()>(
      data_element.0,
      &data_element.1,
      path,
      &mut |token: &P10Token| {
        output_tokens.push(token.clone());
        Ok(())
      },
    )
    .unwrap();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(not(feature = "std"))]
  use alloc::string::ToString;

  use dcmfx_core::ValueRepresentation;

  #[test]
  fn add_tokens_test() {
    let data_elements_to_insert: DataSet = vec![
      (
        DataElementTag::new(0, 0),
        DataElementValue::new_long_text("00").unwrap(),
      ),
      (
        DataElementTag::new(1, 0),
        DataElementValue::new_long_text("01").unwrap(),
      ),
      (
        DataElementTag::new(3, 0),
        DataElementValue::new_long_text("03").unwrap(),
      ),
      (
        DataElementTag::new(4, 0),
        DataElementValue::new_long_text("04").unwrap(),
      ),
      (
        DataElementTag::new(6, 0),
        DataElementValue::new_long_text("06").unwrap(),
      ),
      (
        DataElementTag::new(7, 0),
        DataElementValue::new_long_text("07").unwrap(),
      ),
    ]
    .into_iter()
    .collect();

    let mut insert_transform = P10InsertTransform::new(data_elements_to_insert);

    let input_tokens: Vec<P10Token> = vec![
      tokens_for_tag(DataElementTag::new(2, 0), b"12"),
      tokens_for_tag(DataElementTag::new(5, 0), b"15"),
      tokens_for_tag(DataElementTag::new(6, 0), b"16"),
      vec![P10Token::End],
    ]
    .into_iter()
    .flatten()
    .collect();

    let mut output_tokens = vec![];
    for token in input_tokens {
      output_tokens.extend_from_slice(
        insert_transform.add_token(&token).unwrap().as_slice(),
      );
    }

    assert_eq!(
      output_tokens,
      vec![
        tokens_for_tag(DataElementTag::new(0, 0), b"00"),
        tokens_for_tag(DataElementTag::new(1, 0), b"01"),
        tokens_for_tag(DataElementTag::new(2, 0), b"12"),
        tokens_for_tag(DataElementTag::new(3, 0), b"03"),
        tokens_for_tag(DataElementTag::new(4, 0), b"04"),
        tokens_for_tag(DataElementTag::new(5, 0), b"15"),
        tokens_for_tag(DataElementTag::new(6, 0), b"06"),
        tokens_for_tag(DataElementTag::new(7, 0), b"07"),
        vec![P10Token::End],
      ]
      .into_iter()
      .flatten()
      .collect::<Vec<P10Token>>()
    );
  }

  fn tokens_for_tag(tag: DataElementTag, value_bytes: &[u8]) -> Vec<P10Token> {
    vec![
      P10Token::DataElementHeader {
        tag,
        vr: ValueRepresentation::LongText,
        length: value_bytes.len() as u32,
        path: DataSetPath::new_with_data_element(tag),
      },
      P10Token::DataElementValueBytes {
        tag,
        vr: ValueRepresentation::LongText,
        data: value_bytes.to_vec().into(),
        bytes_remaining: 0,
      },
    ]
  }
}
