use dcmfx_core::{DataElementTag, DataElementValue, DataSet};

use crate::{p10_part, P10FilterTransform, P10Part};

/// Transform that inserts data elements into a stream of DICOM P10 parts.
///
pub struct P10InsertTransform {
  data_elements_to_insert: Vec<(DataElementTag, DataElementValue)>,
  filter_transform: P10FilterTransform,
}

impl P10InsertTransform {
  /// Creates a new context for inserting data elements into the root data set
  /// of a stream of DICOM P10 parts.
  ///
  pub fn new(data_elements_to_insert: DataSet) -> Self {
    let tags_to_insert = data_elements_to_insert.tags();

    // Create a filter transform that filters out the data elements that are
    // going to be inserted. This ensures there are no duplicate data elements
    // in the resulting part stream.
    let filter_transform = P10FilterTransform::new(
      Box::new(move |tag, _vr, location| {
        !location.is_empty() || !tags_to_insert.contains(&tag)
      }),
      false,
    );

    Self {
      data_elements_to_insert: data_elements_to_insert
        .into_iter()
        .rev()
        .collect(),
      filter_transform,
    }
  }

  /// Adds the next available part to the P10 insert transform and returns the
  /// resulting parts.
  ///
  pub fn add_part(&mut self, part: &P10Part) -> Vec<P10Part> {
    // If there are no more data elements to be inserted then pass the part
    // straight through
    if self.data_elements_to_insert.is_empty() {
      return vec![part.clone()];
    }

    let is_at_root = self.filter_transform.is_at_root();

    // Pass the part through the filter transform
    if !self.filter_transform.add_part(part) {
      return vec![];
    }

    // Data element insertion is only supported in the root data set, so if the
    // stream is not at the root data set then there's nothing to do
    if !is_at_root {
      return vec![part.clone()];
    }

    let mut output_parts = vec![];

    match &part {
      // If this part is the start of a new data element, and there are data
      // elements still to be inserted, then insert any that should appear prior
      // to this next data element
      P10Part::SequenceStart { tag, .. }
      | P10Part::DataElementHeader { tag, .. } => {
        while let Some(data_element) = self.data_elements_to_insert.pop() {
          if data_element.0.to_int() >= tag.to_int() {
            self.data_elements_to_insert.push(data_element);
            break;
          }

          self.append_data_element_parts(data_element, &mut output_parts);
        }

        output_parts.push(part.clone());
      }

      // If this part is the end of the P10 parts and there are still data
      // elements to be inserted then insert them now prior to the end
      P10Part::End => {
        while let Some(data_element) = self.data_elements_to_insert.pop() {
          self.append_data_element_parts(data_element, &mut output_parts);
        }

        output_parts.push(P10Part::End);
      }

      _ => output_parts.push(part.clone()),
    };

    output_parts
  }

  fn append_data_element_parts(
    &self,
    data_element: (DataElementTag, DataElementValue),
    output_parts: &mut Vec<P10Part>,
  ) {
    p10_part::data_element_to_parts::<()>(
      data_element.0,
      &data_element.1,
      &mut |part: &P10Part| {
        output_parts.push(part.clone());
        Ok(())
      },
    )
    .unwrap();
  }
}

#[cfg(test)]
mod tests {
  use std::rc::Rc;

  use dcmfx_core::value_representation::ValueRepresentation;

  use super::*;

  #[test]
  fn add_parts_test() {
    let data_elements_to_insert: DataSet = vec![
      (
        DataElementTag::new(0, 0),
        DataElementValue::new_long_text("0".to_owned()).unwrap(),
      ),
      (
        DataElementTag::new(1, 0),
        DataElementValue::new_long_text("1".to_owned()).unwrap(),
      ),
      (
        DataElementTag::new(3, 0),
        DataElementValue::new_long_text("3".to_owned()).unwrap(),
      ),
      (
        DataElementTag::new(4, 0),
        DataElementValue::new_long_text("4".to_owned()).unwrap(),
      ),
      (
        DataElementTag::new(6, 0),
        DataElementValue::new_long_text("6".to_owned()).unwrap(),
      ),
    ]
    .into_iter()
    .collect();

    let mut insert_transform = P10InsertTransform::new(data_elements_to_insert);

    let input_parts: Vec<P10Part> = vec![
      parts_for_tag(DataElementTag::new(2, 0)),
      parts_for_tag(DataElementTag::new(5, 0)),
      vec![P10Part::End],
    ]
    .into_iter()
    .flatten()
    .collect();

    let mut output_parts = vec![];
    for part in input_parts {
      output_parts
        .extend_from_slice(insert_transform.add_part(&part).as_slice());
    }

    assert_eq!(
      output_parts,
      vec![
        parts_for_tag(DataElementTag::new(0, 0)),
        parts_for_tag(DataElementTag::new(1, 0)),
        parts_for_tag(DataElementTag::new(2, 0)),
        parts_for_tag(DataElementTag::new(3, 0)),
        parts_for_tag(DataElementTag::new(4, 0)),
        parts_for_tag(DataElementTag::new(5, 0)),
        parts_for_tag(DataElementTag::new(6, 0)),
        vec![P10Part::End]
      ]
      .into_iter()
      .flatten()
      .collect::<Vec<P10Part>>()
    );
  }

  fn parts_for_tag(tag: DataElementTag) -> Vec<P10Part> {
    let value_bytes = format!("{} ", tag.group).into_bytes();

    vec![
      P10Part::DataElementHeader {
        tag,
        vr: ValueRepresentation::LongText,
        length: value_bytes.len() as u32,
      },
      P10Part::DataElementValueBytes {
        vr: ValueRepresentation::LongText,
        data: Rc::new(value_bytes),
        bytes_remaining: 0,
      },
    ]
  }
}
