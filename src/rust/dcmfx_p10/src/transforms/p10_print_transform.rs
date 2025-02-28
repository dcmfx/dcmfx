use dcmfx_core::{
  DataElementTag, DataElementValue, DataSet, DataSetPrintOptions,
  ValueRepresentation, data_set, dictionary,
};

use crate::P10Token;

/// Transform that converts a stream of DICOM P10 tokens into printable text
/// that describes the structure and content of the contained DICOM data.
///
/// This is used for printing data sets on the command line, and the output can
/// be styled via [`DataSetPrintOptions`].
///
pub struct P10PrintTransform {
  print_options: DataSetPrintOptions,

  indent: usize,
  current_data_element: DataElementTag,
  ignore_data_element_value_bytes: bool,
  value_max_width: usize,

  // Track private creator data elements so that private tags can be printed
  // with the correct names where possible
  private_creators: Vec<DataSet>,
  last_data_element_private_creator_tag: Option<DataElementTag>,
}

impl P10PrintTransform {
  /// Constructs a new DICOM P10 print transform with the specified print
  /// options.
  ///
  pub fn new(print_options: &DataSetPrintOptions) -> Self {
    Self {
      print_options: print_options.clone(),
      indent: 0,
      current_data_element: DataElementTag::new(0, 0),
      ignore_data_element_value_bytes: false,
      value_max_width: 0,
      private_creators: vec![DataSet::new()],
      last_data_element_private_creator_tag: None,
    }
  }

  /// Adds the next DICOM P10 token to be printed and returns the next piece of
  /// text output to be displayed.
  ///
  pub fn add_token(&mut self, token: &P10Token) -> String {
    match token {
      P10Token::FileMetaInformation { data_set } => {
        let mut s = "".to_string();

        data_set.to_lines(&self.print_options, &mut |line| {
          s.push_str(&line);
          s.push('\n');
        });

        s
      }

      P10Token::DataElementHeader { tag, vr, length } => {
        let (s, width) = data_set::print::format_data_element_prefix(
          *tag,
          self.private_creators.last().unwrap().tag_name(*tag),
          Some(*vr),
          Some(*length as usize),
          self.indent,
          &self.print_options,
        );

        self.current_data_element = *tag;

        // Calculate the width remaining for previewing the value
        self.value_max_width =
          std::cmp::max(self.print_options.max_width.saturating_sub(width), 10);

        // Use the next value bytes token to print a preview of the data
        // element's value
        self.ignore_data_element_value_bytes = false;

        // If this is a private creator tag then its value will be stored so
        // that well-known private tag names can be printed
        if *vr == ValueRepresentation::LongString && tag.is_private_creator() {
          self.last_data_element_private_creator_tag = Some(*tag);
        } else {
          self.last_data_element_private_creator_tag = None;
        }

        s
      }

      P10Token::DataElementValueBytes { vr, data, .. }
        if !self.ignore_data_element_value_bytes =>
      {
        let value = DataElementValue::new_binary_unchecked(*vr, data.clone());

        // Ignore any further value bytes tokens now that the value has been
        // printed
        self.ignore_data_element_value_bytes = true;

        // Store private creator name data elements
        if let Some(tag) = self.last_data_element_private_creator_tag {
          self.private_creators.last_mut().unwrap().insert(
            tag,
            DataElementValue::new_binary_unchecked(
              ValueRepresentation::LongString,
              data.clone(),
            ),
          )
        }

        format!(
          "{}\n",
          value.to_string(self.current_data_element, self.value_max_width)
        )
      }

      P10Token::SequenceStart { tag, vr } => {
        let mut s = data_set::print::format_data_element_prefix(
          *tag,
          self.private_creators.last().unwrap().tag_name(*tag),
          Some(*vr),
          None,
          self.indent,
          &self.print_options,
        )
        .0;

        s.push('\n');

        self.indent += 1;

        s
      }

      P10Token::SequenceDelimiter { .. } => {
        self.indent -= 1;

        let mut s = data_set::print::format_data_element_prefix(
          dictionary::SEQUENCE_DELIMITATION_ITEM.tag,
          dictionary::SEQUENCE_DELIMITATION_ITEM.name,
          None,
          None,
          self.indent,
          &self.print_options,
        )
        .0;

        s.push('\n');

        s
      }

      P10Token::SequenceItemStart => {
        let mut s = data_set::print::format_data_element_prefix(
          dictionary::ITEM.tag,
          dictionary::ITEM.name,
          None,
          None,
          self.indent,
          &self.print_options,
        )
        .0;

        s.push('\n');

        self.indent += 1;
        self.private_creators.push(DataSet::new());

        s
      }

      P10Token::SequenceItemDelimiter => {
        self.indent -= 1;
        self.private_creators.pop();

        let mut s = data_set::print::format_data_element_prefix(
          dictionary::ITEM_DELIMITATION_ITEM.tag,
          dictionary::ITEM_DELIMITATION_ITEM.name,
          None,
          None,
          self.indent,
          &self.print_options,
        )
        .0;
        s.push('\n');

        s
      }

      P10Token::PixelDataItem { length } => {
        let (s, width) = data_set::print::format_data_element_prefix(
          dictionary::ITEM.tag,
          dictionary::ITEM.name,
          None,
          Some(*length as usize),
          self.indent,
          &self.print_options,
        );

        // Calculate the width remaining for previewing the value
        self.value_max_width =
          std::cmp::max(self.print_options.max_width.saturating_sub(width), 10);

        // Use the next value bytes token to print a preview of the pixel data
        // item's value
        self.ignore_data_element_value_bytes = false;

        s
      }

      _ => "".to_string(),
    }
  }
}
