//! Provides a transform for converting a stream of DICOM [`P10Token`]s into a
//! stream of DICOM JSON data.

use core::str;
use std::{io::Write, rc::Rc};

use base64::prelude::*;

use dcmfx_core::{
  DataElementTag, DataElementValue, DataError, DataSet, DataSetPath,
  ValueRepresentation, dictionary,
};
use dcmfx_p10::{P10Error, P10Token};

use crate::DicomJsonConfig;
use crate::json_error::JsonSerializeError;

/// Transform that converts a stream of DICOM P10 tokens to the DICOM JSON
/// model.
///
pub struct P10JsonTransform {
  /// The DICOM JSON config to use when serializing the token stream to JSON.
  config: DicomJsonConfig,

  /// Whether a comma needs to be inserted before the next JSON value.
  insert_comma: bool,

  /// The data element that value bytes are currently being gathered for.
  current_data_element: (DataElementTag, Vec<Rc<Vec<u8>>>),

  /// Whether to ignore DataElementValueBytes tokens when they're received. This
  /// is used to stop certain data elements being included in the JSON.
  ignore_data_element_value_bytes: bool,

  /// Whether tokens for encapsulated pixel data are currently being received.
  in_encapsulated_pixel_data: bool,

  /// When multiple binary tokens are being directly streamed as an
  /// InlineBinary, there can be 0, 1, or 2 bytes left over from the previous
  /// chunk due to Base64 converting in three byte chunks. These leftover bytes
  /// are prepended to the next chunk of data when it arrives for Base64
  /// conversion.
  pending_base64_input: Vec<u8>,

  /// The data set path to where JSON serialization is currently up to. This is
  /// used to provide precise location information when an error occurs.
  data_set_path: DataSetPath,

  /// The number of items in each active sequence in the data set path. This is
  /// used to provide precise location information when an error occurs.
  sequence_item_counts: Vec<usize>,
}

impl P10JsonTransform {
  /// Constructs a new P10 tokens to DICOM JSON transform.
  ///
  pub fn new(config: &DicomJsonConfig) -> Self {
    P10JsonTransform {
      config: config.clone(),
      insert_comma: false,
      current_data_element: (DataElementTag::new(0, 0), vec![]),
      ignore_data_element_value_bytes: false,
      in_encapsulated_pixel_data: false,
      pending_base64_input: vec![],
      data_set_path: DataSetPath::new(),
      sequence_item_counts: Vec::new(),
    }
  }

  /// Adds the next DICOM P10 token to this JSON transform. Bytes of JSON data
  /// are written to the provided `stream` as they become available.
  ///
  /// If P10 tokens are provided in an invalid order then an error may be
  /// returned, but this is not guaranteed for all invalid token orders, so in
  /// some cases the resulting JSON stream could be invalid when the incoming
  /// stream of P10 tokens is malformed.
  ///
  pub fn add_token(
    &mut self,
    token: &P10Token,
    stream: &mut dyn std::io::Write,
  ) -> Result<(), JsonSerializeError> {
    let token_stream_invalid_error = || {
      JsonSerializeError::P10Error(P10Error::TokenStreamInvalid {
        when: "Adding token to JSON transform".to_string(),
        details: "The transform was not able to write this token".to_string(),
        token: token.clone(),
      })
    };

    match token {
      P10Token::FilePreambleAndDICMPrefix { .. } => Ok(()),
      P10Token::FileMetaInformation { data_set } => self
        .begin(data_set, stream)
        .map_err(JsonSerializeError::IOError),

      P10Token::DataElementHeader { tag, vr, length } => {
        self
          .write_data_element_header(*tag, *vr, *length, stream)
          .map_err(JsonSerializeError::IOError)?;

        self
          .data_set_path
          .add_data_element(*tag)
          .map_err(|_| token_stream_invalid_error())
      }

      P10Token::DataElementValueBytes {
        vr,
        data,
        bytes_remaining,
        ..
      } => {
        self.write_data_element_value_bytes(
          *vr,
          data,
          *bytes_remaining,
          stream,
        )?;

        if *bytes_remaining == 0 {
          self
            .data_set_path
            .pop()
            .map_err(|_| token_stream_invalid_error())?;
        }

        Ok(())
      }

      P10Token::SequenceStart { tag, vr } => {
        self.write_sequence_start(*tag, *vr, stream)?;

        self.sequence_item_counts.push(0);

        self
          .data_set_path
          .add_data_element(*tag)
          .map_err(|_| token_stream_invalid_error())
      }

      P10Token::SequenceDelimiter { .. } => {
        self
          .write_sequence_end(stream)
          .map_err(JsonSerializeError::IOError)?;

        self.sequence_item_counts.pop();

        self
          .data_set_path
          .pop()
          .map_err(|_| token_stream_invalid_error())
      }

      P10Token::SequenceItemStart => {
        if let Some(sequence_item_count) = self.sequence_item_counts.last_mut()
        {
          self
            .data_set_path
            .add_sequence_item(*sequence_item_count)
            .map_err(|_| token_stream_invalid_error())?;

          *sequence_item_count += 1;
        }

        self
          .write_sequence_item_start(stream)
          .map_err(JsonSerializeError::IOError)
      }

      P10Token::SequenceItemDelimiter => {
        self
          .write_sequence_item_end(stream)
          .map_err(JsonSerializeError::IOError)?;

        self
          .data_set_path
          .pop()
          .map_err(|_| token_stream_invalid_error())
      }

      P10Token::PixelDataItem { length } => {
        if let Some(sequence_item_count) = self.sequence_item_counts.last_mut()
        {
          self
            .data_set_path
            .add_sequence_item(*sequence_item_count)
            .map_err(|_| token_stream_invalid_error())?;

          *sequence_item_count += 1;
        }

        self.write_encapsulated_pixel_data_item(*length, stream)
      }

      P10Token::End => self.end(stream).map_err(JsonSerializeError::IOError),
    }
  }

  fn indent(&self, offset: isize) -> String {
    let mut indent = 1isize;
    indent += self.data_set_path.sequence_item_count() as isize * 3;
    indent += offset;
    indent = indent.max(0);

    "  ".repeat(indent as usize)
  }

  fn write_indent(
    &self,
    stream: &mut dyn std::io::Write,
    offset: isize,
  ) -> Result<(), std::io::Error> {
    stream.write_all(self.indent(offset).as_bytes())
  }

  fn begin(
    &mut self,
    file_meta_information: &DataSet,
    stream: &mut dyn std::io::Write,
  ) -> Result<(), std::io::Error> {
    if self.config.pretty_print {
      stream.write_all(b"{\n")?;
    } else {
      stream.write_all(b"{")?;
    }

    // Exclude all File Meta Information data elements except for '(0002,0010)
    // Transfer Syntax UID' when encapsulated pixel data is being included as it
    // is needed to interpret that data
    if self.config.store_encapsulated_pixel_data {
      if let Ok(transfer_syntax_uid) =
        file_meta_information.get_string(dictionary::TRANSFER_SYNTAX_UID.tag)
      {
        if self.config.pretty_print {
          stream.write_all(b"  \"00020010\": {\n    \"vr\": \"UI\",\n    \"Value\": [\n      \"")?;
          stream.write_all(transfer_syntax_uid.as_bytes())?;
          stream.write_all(b"\"\n    ]\n  }")?;
        } else {
          stream.write_all(br#""00020010":{"vr":"UI","Value":[""#)?;
          stream.write_all(transfer_syntax_uid.as_bytes())?;
          stream.write_all(br#""]}"#)?;
        }

        self.insert_comma = true;
      }
    }

    Ok(())
  }

  fn write_data_element_header(
    &mut self,
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: u32,
    stream: &mut dyn std::io::Write,
  ) -> Result<(), std::io::Error> {
    // Exclude group length data elements as these have no use in DICOM JSON.
    // Also exclude the '(0008,0005) Specific Character Set' data element as
    // DICOM JSON always uses UTF-8
    if tag.element == 0 || tag == dictionary::SPECIFIC_CHARACTER_SET.tag {
      self.ignore_data_element_value_bytes = true;
      return Ok(());
    }

    if self.insert_comma {
      if self.config.pretty_print {
        stream.write_all(b",\n")?;
      } else {
        stream.write_all(b",")?;
      }
    }
    self.insert_comma = true;

    self.current_data_element.0 = tag;
    self.current_data_element.1.clear();

    // Write the tag and VR
    if self.config.pretty_print {
      self.write_indent(stream, 0)?;

      let mut json = *b"\"________\": {\n";
      json[1..9].copy_from_slice(&tag.to_hex_digits());
      stream.write_all(json.as_slice())?;

      self.write_indent(stream, 1)?;

      let mut json = *b"\"vr\": \"__\"";
      json[7..9].copy_from_slice(&vr.to_bytes());
      stream.write_all(json.as_slice())?;
    } else {
      let mut json = *br#""________":{"vr":"__""#;
      json[1..9].copy_from_slice(&tag.to_hex_digits());
      json[18..20].copy_from_slice(&vr.to_bytes());
      stream.write_all(json.as_slice())?;
    }

    // If the value's length is zero then no 'Value' or 'InlineBinary' should be
    // added to the output. Ref: PS3.18 F.2.5.
    if length == 0 {
      if self.config.pretty_print {
        stream.write_all(b"\n")?;
        self.write_indent(stream, 0)?;
        stream.write_all(b"}")?;
      } else {
        stream.write_all(b"}")?;
      }

      self.ignore_data_element_value_bytes = true;

      return Ok(());
    }

    // The following VRs use InlineBinary in the output
    if vr == ValueRepresentation::OtherByteString
      || vr == ValueRepresentation::OtherDoubleString
      || vr == ValueRepresentation::OtherFloatString
      || vr == ValueRepresentation::OtherLongString
      || vr == ValueRepresentation::OtherVeryLongString
      || vr == ValueRepresentation::OtherWordString
      || vr == ValueRepresentation::Unknown
    {
      if self.config.pretty_print {
        stream.write_all(b",\n")?;
        self.write_indent(stream, 1)?;
        stream.write_all(b"\"InlineBinary\": \"")?;
      } else {
        stream.write_all(br#","InlineBinary":""#)?;
      }
    } else if self.config.pretty_print {
      stream.write_all(b",\n")?;
      self.write_indent(stream, 1)?;
      stream.write_all(b"\"Value\": [\n")?;
    } else {
      stream.write_all(br#","Value":["#)?;
    }

    Ok(())
  }

  fn write_data_element_value_bytes(
    &mut self,
    vr: ValueRepresentation,
    data: &Rc<Vec<u8>>,
    bytes_remaining: u32,
    stream: &mut dyn std::io::Write,
  ) -> Result<(), JsonSerializeError> {
    // If this data element value is being ignored then do nothing
    if self.ignore_data_element_value_bytes {
      if bytes_remaining == 0 {
        self.ignore_data_element_value_bytes = false;
      }

      return Ok(());
    }

    // The following VRs are streamed out directly as Base64
    if vr == ValueRepresentation::OtherByteString
      || vr == ValueRepresentation::OtherDoubleString
      || vr == ValueRepresentation::OtherFloatString
      || vr == ValueRepresentation::OtherLongString
      || vr == ValueRepresentation::OtherVeryLongString
      || vr == ValueRepresentation::OtherWordString
      || vr == ValueRepresentation::Unknown
    {
      self
        .write_base64(
          data,
          bytes_remaining == 0 && !self.in_encapsulated_pixel_data,
          stream,
        )
        .map_err(JsonSerializeError::IOError)?;

      if bytes_remaining == 0 && !self.in_encapsulated_pixel_data {
        if self.config.pretty_print {
          stream
            .write_all(b"\"\n")
            .and_then(|_| self.write_indent(stream, 0))
            .and_then(|_| stream.write_all(b"}"))
        } else {
          stream.write_all(br#""}"#)
        }
        .map_err(JsonSerializeError::IOError)?
      }

      return Ok(());
    }

    // If this data element value is not an inline binary and has no data then
    // there's nothing to do
    if data.len() == 0 && bytes_remaining == 0 {
      return Ok(());
    }

    // Gather the final data for this data element
    self.current_data_element.1.push(data.clone());

    // Wait until all bytes for the data element have been accumulated
    if bytes_remaining > 0 {
      return Ok(());
    }

    // Create final binary data element value
    let bytes = if self.current_data_element.1.len() == 1 {
      self.current_data_element.1[0].clone()
    } else {
      let mut bytes = Vec::with_capacity(
        self.current_data_element.1.iter().map(|v| v.len()).sum(),
      );

      for chunk in self.current_data_element.1.iter() {
        bytes.extend_from_slice(chunk);
      }

      Rc::new(bytes)
    };

    let value = DataElementValue::new_binary_unchecked(vr, bytes.clone());

    let json_values = self
      .convert_binary_value_to_json(&value, bytes)
      .map_err(|e| {
        JsonSerializeError::DataError(e.with_path(&self.data_set_path))
      })?;

    if self.config.pretty_print {
      self
        .write_indent(stream, 2)
        .map_err(JsonSerializeError::IOError)?;
    }

    for (i, json_value) in json_values.iter().enumerate() {
      stream
        .write_all(json_value.as_bytes())
        .map_err(JsonSerializeError::IOError)?;

      if i != json_values.len() - 1 {
        if self.config.pretty_print {
          stream
            .write_all(b",\n")
            .and_then(|_| self.write_indent(stream, 2))
        } else {
          stream.write_all(b",")
        }
        .map_err(JsonSerializeError::IOError)?;
      }
    }

    if self.config.pretty_print {
      stream
        .write_all(b"\n")
        .and_then(|_| self.write_indent(stream, 1))
        .and_then(|_| stream.write(b"]\n"))
        .and_then(|_| self.write_indent(stream, 0))
        .and_then(|_| stream.write(b"}"))
        .map(|_| ())
    } else {
      stream.write_all(b"]}")
    }
    .map_err(JsonSerializeError::IOError)
  }

  fn write_sequence_start(
    &mut self,
    tag: DataElementTag,
    vr: ValueRepresentation,
    stream: &mut dyn std::io::Write,
  ) -> Result<(), JsonSerializeError> {
    if self.insert_comma {
      if self.config.pretty_print {
        stream.write_all(b",\n")
      } else {
        stream.write_all(b",")
      }
      .map_err(JsonSerializeError::IOError)?;
    }
    self.insert_comma = true;

    if vr == ValueRepresentation::Sequence {
      self.insert_comma = false;

      if self.config.pretty_print {
        (|| {
          self.write_indent(stream, 0)?;

          let mut json = *b"\"________\": {\n";
          json[1..9].copy_from_slice(&tag.to_hex_digits());
          stream.write_all(json.as_slice())?;

          self.write_indent(stream, 0)?;
          stream.write_all(b"\"vr\": \"SQ\",\n")?;
          self.write_indent(stream, 1)?;
          stream.write_all(b"\"Value\": [")
        })()
      } else {
        let mut json = *br#""________":{"vr":"SQ","Value":["#;
        json[1..9].copy_from_slice(&tag.to_hex_digits());
        stream.write_all(json.as_slice())
      }
    } else {
      if !self.config.store_encapsulated_pixel_data {
        return Err(JsonSerializeError::DataError(
          DataError::new_value_invalid(
            "DICOM JSON does not support encapsulated pixel data, \
            consider enabling this extension in the config"
              .to_string(),
          )
          .with_path(&self.data_set_path),
        ));
      }

      self.in_encapsulated_pixel_data = true;

      if self.config.pretty_print {
        (|| {
          self.write_indent(stream, 0)?;

          let mut json = *b"\"________\": {\n";
          json[1..9].copy_from_slice(&tag.to_hex_digits());
          stream.write_all(json.as_slice())?;

          self.write_indent(stream, 1)?;

          let mut json = *b"\"vr\": \"__\",\n";
          json[7..9].copy_from_slice(&vr.to_bytes());
          stream.write_all(json.as_slice())?;

          self.write_indent(stream, 1)?;
          stream.write_all(b"\"InlineBinary\": \"")
        })()
      } else {
        let mut json = *br#""________":{"vr":"__","InlineBinary":""#;
        json[1..9].copy_from_slice(&tag.to_hex_digits());
        json[18..20].copy_from_slice(&vr.to_bytes());

        stream.write_all(json.as_slice())
      }
    }
    .map_err(JsonSerializeError::IOError)
  }

  fn write_sequence_end(
    &mut self,
    stream: &mut dyn std::io::Write,
  ) -> Result<(), std::io::Error> {
    if self.in_encapsulated_pixel_data {
      self.in_encapsulated_pixel_data = false;
      self.write_base64(&[], true, stream)?;

      if self.config.pretty_print {
        stream.write_all(b"\"\n")?;
        self.write_indent(stream, 0)?;
        stream.write_all(b"}")
      } else {
        stream.write_all(b"\"}")
      }
    } else {
      self.insert_comma = true;

      if self.config.pretty_print {
        stream.write_all(b"\n")?;
        self.write_indent(stream, 1)?;
        stream.write_all(b"]\n")?;
        self.write_indent(stream, 0)?;
        stream.write_all(b"}")
      } else {
        stream.write_all(b"]}")
      }
    }
  }

  fn write_sequence_item_start(
    &mut self,
    stream: &mut dyn std::io::Write,
  ) -> Result<(), std::io::Error> {
    if self.insert_comma {
      stream.write_all(b",")?;
    }
    self.insert_comma = false;

    if self.config.pretty_print {
      stream.write_all(b"\n")?;
      self.write_indent(stream, -1)?;
      stream.write_all(b"{\n")
    } else {
      stream.write_all(b"{")
    }
  }

  fn write_sequence_item_end(
    &mut self,
    stream: &mut dyn std::io::Write,
  ) -> Result<(), std::io::Error> {
    self.insert_comma = true;

    if self.config.pretty_print {
      stream.write_all(b"\n")?;
      self.write_indent(stream, -1)?;
      stream.write_all(b"}")
    } else {
      stream.write_all(b"}")
    }
  }

  fn write_encapsulated_pixel_data_item(
    &mut self,
    length: u32,
    stream: &mut dyn std::io::Write,
  ) -> Result<(), JsonSerializeError> {
    if !self.config.store_encapsulated_pixel_data {
      return Err(JsonSerializeError::DataError(
        DataError::new_value_invalid(
          "DICOM JSON does not support encapsulated pixel data, \
          consider enabling this extension in the config"
            .to_string(),
        )
        .with_path(&self.data_set_path),
      ));
    }

    // Construct bytes for the item header
    let mut bytes = [0xFE, 0xFF, 0x00, 0xE0, 0x00, 0x00, 0x00, 0x00];
    bytes[4..8].copy_from_slice(length.to_le_bytes().as_slice());

    self
      .write_base64(bytes.as_slice(), false, stream)
      .map_err(JsonSerializeError::IOError)
  }

  fn end(
    &mut self,
    stream: &mut dyn std::io::Write,
  ) -> Result<(), std::io::Error> {
    if self.config.pretty_print {
      stream.write_all(b"\n}\n")
    } else {
      stream.write_all(b"}")
    }
  }

  fn write_base64(
    &mut self,
    input: &[u8],
    finish: bool,
    stream: &mut dyn std::io::Write,
  ) -> Result<(), std::io::Error> {
    // If there's still insufficient data to encode with this new data then
    // accumulate the bytes and wait till next time
    if self.pending_base64_input.len() + input.len() < 3 && !finish {
      self.pending_base64_input.extend_from_slice(input);
      return Ok(());
    }

    // Calculate how many of the input bytes to consume. Bytes must be fed to
    // the Base64 encoder in lots of 3, and any leftover saved till next time.
    // If these are the final bytes then all remaining bytes are encoded and the
    // encoder will add any required Base64 padding.
    let input_bytes_consumed = if finish {
      input.len()
    } else {
      (self.pending_base64_input.len() + input.len()) / 3 * 3
        - self.pending_base64_input.len()
    };

    // Base64 encode the bytes and output to the stream
    let mut encoder =
      base64::write::EncoderWriter::new(stream, &BASE64_STANDARD);
    encoder.write_all(&self.pending_base64_input)?;
    encoder.write_all(&input[0..input_bytes_consumed])?;
    encoder.finish()?;

    // Save off unencoded bytes for next time
    self.pending_base64_input = input[input_bytes_consumed..].to_vec();

    Ok(())
  }

  /// Converts a data element value containing binary data to DICOM JSON.
  ///
  fn convert_binary_value_to_json(
    &self,
    value: &DataElementValue,
    bytes: Rc<Vec<u8>>,
  ) -> Result<Vec<String>, DataError> {
    match value.value_representation() {
      // AttributeTag value representation
      ValueRepresentation::AttributeTag => Ok(
        value
          .get_attribute_tags()?
          .iter()
          .map(|tag| format!("\"{}\"", tag.to_hex_string()))
          .collect(),
      ),

      // Floating point value representations. Because JSON doesn't allow NaN or
      // Infinity values, but they can be present in a DICOM data element, they
      // are converted to strings in the generated JSON.
      ValueRepresentation::DecimalString
      | ValueRepresentation::FloatingPointDouble
      | ValueRepresentation::FloatingPointSingle => Ok(
        value
          .get_floats()?
          .iter()
          .map(|f| {
            if f.is_nan() {
              "\"NaN\"".to_string()
            } else if *f == f64::INFINITY {
              "\"Infinity\"".to_string()
            } else if *f == f64::NEG_INFINITY {
              "\"-Infinity\"".to_string()
            } else {
              format!("{:?}", f)
            }
          })
          .collect(),
      ),

      // PersonName value representation
      ValueRepresentation::PersonName => {
        let string = str::from_utf8(&bytes).map_err(|_| {
          DataError::new_value_invalid(
            "PersonName is invalid UTF-8".to_string(),
          )
        })?;

        string
          .split("\\")
          .map(|raw_name| {
            let mut component_groups: Vec<_> = raw_name
              .split("=")
              .map(|s| s.trim_end_matches(' '))
              .enumerate()
              .collect();

            if component_groups.len() > 3 {
              return Err(DataError::new_value_invalid(format!(
                "PersonName has too many component groups: {}",
                component_groups.len()
              )));
            }

            component_groups.retain(|(_, s)| !s.is_empty());

            let mut result = if self.config.pretty_print {
              format!("{}{{\n", self.indent(-1))
            } else {
              "{".to_string()
            };

            for (j, (i, component_group)) in component_groups.iter().enumerate()
            {
              let name = ["Alphabetic", "Ideographic", "Phonetic"][*i];

              // Escape the value of the component group appropriately for JSON
              let value = serde_json::Value::from(*component_group).to_string();

              if self.config.pretty_print {
                result.push_str(&self.indent(3));
                result.push('"');
                result.push_str(name);
                result.push_str("\": ");
                result.push_str(&value);
              } else {
                result.push('"');
                result.push_str(name);
                result.push_str("\":");
                result.push_str(&value);
              }

              if j != component_groups.len() - 1 {
                if self.config.pretty_print {
                  result.push_str(",\n");
                } else {
                  result.push(',');
                }
              }
            }

            if self.config.pretty_print {
              result.push('\n');
              result.push_str(&self.indent(2));
            };

            result.push('}');

            Ok(result)
          })
          .collect()
      }

      // Binary signed/unsigned integer value representations
      ValueRepresentation::SignedLong
      | ValueRepresentation::SignedShort
      | ValueRepresentation::UnsignedLong
      | ValueRepresentation::UnsignedShort
      | ValueRepresentation::IntegerString => Ok(
        value
          .get_ints::<i64>()?
          .iter()
          .map(|i| i.to_string())
          .collect(),
      ),

      // Binary signed/unsigned big integer value representations
      ValueRepresentation::SignedVeryLong
      | ValueRepresentation::UnsignedVeryLong => {
        // The range of integers representable by JavaScript's Number type.
        // Values outside this range are converted to strings in the generated
        // JSON.
        let safe_integer_range = -9007199254740991i128..=9007199254740991i128;

        Ok(
          value
            .get_big_ints()?
            .iter()
            .map(|i| {
              if safe_integer_range.contains(i) {
                i.to_string()
              } else {
                format!("\"{}\"", i)
              }
            })
            .collect(),
        )
      }

      // Handle string VRs that have explicit internal structure. Their value is
      // deliberately not parsed or validated beyond conversion to UTF-8, and is
      // just passed straight through.
      ValueRepresentation::AgeString
      | ValueRepresentation::Date
      | ValueRepresentation::DateTime
      | ValueRepresentation::Time => {
        let string = std::str::from_utf8(&bytes)
          .map_err(|_| {
            DataError::new_value_invalid(
              "String bytes are not valid UTF-8".to_string(),
            )
          })?
          .trim_end_matches(' ');

        Ok(vec![prepare_json_string(string)])
      }

      // Handle string VRs that don't support multiplicity
      ValueRepresentation::ApplicationEntity
      | ValueRepresentation::LongText
      | ValueRepresentation::ShortText
      | ValueRepresentation::UniversalResourceIdentifier
      | ValueRepresentation::UnlimitedText => {
        let string = prepare_json_string(value.get_string()?);

        Ok(vec![string])
      }

      // Handle remaining string-based VRs that support multiplicity
      ValueRepresentation::CodeString
      | ValueRepresentation::LongString
      | ValueRepresentation::ShortString
      | ValueRepresentation::UniqueIdentifier
      | ValueRepresentation::UnlimitedCharacters => Ok(
        value
          .get_strings()?
          .into_iter()
          .map(prepare_json_string)
          .collect(),
      ),

      _ => unreachable!(),
    }
  }
}

fn prepare_json_string(value: &str) -> String {
  if value.is_empty() {
    "null".to_string()
  } else {
    serde_json::to_string(&value).unwrap()
  }
}
