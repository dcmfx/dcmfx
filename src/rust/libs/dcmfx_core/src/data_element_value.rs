//! A data element value that can hold any of the DICOM value representations.
//! Data element values are usually stored in a [`DataSet`] which maps data
//! element tags to data element values.

use std::rc::Rc;

use byteorder::ByteOrder;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
  code_strings, registry, utils, value_representation, DataElementTag,
  DataError, DataSet, StructuredAge, StructuredDate, StructuredDateTime,
  StructuredTime, ValueRepresentation,
};

pub mod age_string;
pub mod attribute_tag;
pub mod date;
pub mod date_time;
pub mod decimal_string;
pub mod integer_string;
pub mod person_name;
pub mod time;
pub mod unique_identifier;

/// A DICOM data element value that holds one of the following types of data:
///
/// 1. Binary value. A data element value that holds raw bytes for a specific
///    VR. This is the most common case. When the VR is a string type then the
///    bytes should be UTF-8 encoded. The data is always little endian.
///
/// 2. Lookup table descriptor value. A data element value that holds a lookup
///    table descriptor. The VR should be either
///    [`ValueRepresentation::SignedShort`] or
///    [`ValueRepresentation::UnsignedShort`], and there should be exactly six
///    bytes. The bytes contain three 16-bit integer values, the first and last
///    of which are unsigned, and the second of which is interpreted using the
///    specified VR, i.e. it can be either a signed or unsigned 16-bit integer.
///    The data is always little endian.
///
/// 3. Encapsulated pixel data value. A data element value that holds the raw
///    items for an encapsulated pixel data sequence. The VR must be either
///    [`ValueRepresentation::OtherByteString`] or
///    [`ValueRepresentation::OtherWordString`].
///
/// 4. Sequence value. A data element value that holds a sequence, which is a
///    list of nested data sets used to create hierarchies of data elements in a
///    DICOM data set.
///
/// Data element values that hold binary data always store it in an
/// [`Rc<Vec<u8>>`] which is parsed and converted to a more usable type on
/// request. This improves efficiency as parsing only occurs when the value of a
/// data element is requested, and allows any data to be passed through even if
/// it is non-conformant with the DICOM standard, which is a common occurrence.
///
/// Ref: PS3.5 6.2.
///
#[derive(Debug, Clone, PartialEq)]
pub struct DataElementValue(RawDataElementValue);

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::enum_variant_names)]
enum RawDataElementValue {
  BinaryValue {
    vr: ValueRepresentation,
    bytes: Rc<Vec<u8>>,
  },
  LookupTableDescriptorValue {
    vr: ValueRepresentation,
    bytes: Rc<Vec<u8>>,
  },
  EncapsulatedPixelDataValue {
    vr: ValueRepresentation,
    items: Vec<Rc<Vec<u8>>>,
  },
  SequenceValue {
    items: Vec<DataSet>,
  },
}

impl DataElementValue {
  /// Formats a data element value as a human-readable single line of text.
  /// Values longer than the output width are truncated with a trailing
  /// ellipsis.
  ///
  pub fn to_string(&self, tag: DataElementTag, output_width: usize) -> String {
    // Maximum number of items needed in a comma-separated list of values before
    // reaching the output width
    let output_list_max_size = (output_width + 2) / 3;

    let result = match &self.0 {
      RawDataElementValue::BinaryValue { vr, bytes } if vr.is_string() => {
        // If the data isn't valid UTF-8 then try to ensure the data slice ends
        // exactly on a UTF-8 character boundary so that data element values
        // with partial data are still displayable
        let mut utf8 = std::str::from_utf8(bytes);
        if utf8.is_err() {
          if let Some(index) = bytes
            .iter()
            .rposition(|b| (*b & 0b1100_0000) != 0b1000_0000)
          {
            utf8 = std::str::from_utf8(&bytes[0..index]);
          }
        }

        match utf8 {
          Ok(value) => {
            let formatted_value = match vr {
              ValueRepresentation::AgeString => {
                StructuredAge::from_bytes(bytes)
                  .map(|age| age.to_string())
                  .unwrap_or_else(|_| format!("{:?}", value))
              }

              ValueRepresentation::ApplicationEntity => {
                format!("{:?}", value.trim_matches(' '))
              }

              ValueRepresentation::Date => StructuredDate::from_bytes(bytes)
                .map(|date| date.to_iso8601())
                .unwrap_or_else(|_| format!("{:?}", value)),

              ValueRepresentation::DateTime => {
                StructuredDateTime::from_bytes(bytes)
                  .map(|date_time| date_time.to_iso8601())
                  .unwrap_or_else(|_| format!("{:?}", value))
              }

              ValueRepresentation::Time => StructuredTime::from_bytes(bytes)
                .map(|time| time.to_iso8601())
                .unwrap_or_else(|_| format!("{:?}", value)),

              // Handle string VRs that allow multiplicity
              ValueRepresentation::CodeString
              | ValueRepresentation::DecimalString
              | ValueRepresentation::UniqueIdentifier
              | ValueRepresentation::IntegerString
              | ValueRepresentation::LongString
              | ValueRepresentation::ShortString
              | ValueRepresentation::UnlimitedCharacters => value
                .split("\\")
                .map(|s| match vr {
                  ValueRepresentation::UniqueIdentifier => {
                    format!("{:?}", s.trim_end_matches('\0'))
                  }
                  ValueRepresentation::UnlimitedCharacters => {
                    format!("{:?}", s.trim_end_matches(' '))
                  }
                  _ => format!("{:?}", s.trim()),
                })
                .collect::<Vec<String>>()
                .join(", "),

              _ => format!("{:?}", value.trim_end_matches(' ')),
            };

            // Add a descriptive suffix for known UIDs and CodeStrings
            let suffix = match vr {
              ValueRepresentation::UniqueIdentifier => {
                match registry::uid_name(utils::trim_right_whitespace(value)) {
                  Ok(uid_name) => Some(format!(" ({})", uid_name)),
                  Err(()) => None,
                }
              }

              ValueRepresentation::CodeString => {
                match code_strings::describe(value.trim(), tag) {
                  Ok(description) => Some(format!(" ({})", description)),
                  Err(()) => None,
                }
              }

              _ => None,
            };

            Ok((formatted_value, suffix))
          }

          Err(_) => Ok(("!! Invalid UTF-8 data".to_string(), None)),
        }
      }

      RawDataElementValue::LookupTableDescriptorValue { vr, bytes }
      | RawDataElementValue::BinaryValue { vr, bytes } => match vr {
        ValueRepresentation::AttributeTag => {
          match attribute_tag::from_bytes(bytes) {
            Ok(tags) => Ok((
              tags
                .iter()
                .take(output_list_max_size)
                .map(|tag| tag.to_string())
                .collect::<Vec<String>>()
                .join(", "),
              None,
            )),
            Err(_) => Err(()),
          }
        }

        ValueRepresentation::FloatingPointDouble
        | ValueRepresentation::FloatingPointSingle => match self.get_floats() {
          Ok(floats) => Ok((
            floats
              .iter()
              .take(output_list_max_size)
              .map(|f| {
                if *f == f64::INFINITY {
                  "Infinity".to_string()
                } else if *f == -f64::INFINITY {
                  "-Infinity".to_string()
                } else {
                  format!("{:?}", f)
                }
              })
              .collect::<Vec<String>>()
              .join(", "),
            None,
          )),
          Err(_) => Err(()),
        },

        ValueRepresentation::OtherByteString
        | ValueRepresentation::OtherDoubleString
        | ValueRepresentation::OtherFloatString
        | ValueRepresentation::OtherLongString
        | ValueRepresentation::OtherVeryLongString
        | ValueRepresentation::OtherWordString
        | ValueRepresentation::Unknown => Ok((
          format!(
            "[{}]",
            bytes[0..std::cmp::min(bytes.len(), output_list_max_size)]
              .iter()
              .map(|byte| format!("{:02X}", byte))
              .collect::<Vec<_>>()
              .join(" ")
          ),
          None,
        )),

        ValueRepresentation::SignedLong
        | ValueRepresentation::SignedShort
        | ValueRepresentation::UnsignedLong
        | ValueRepresentation::UnsignedShort => match self.get_ints() {
          Ok(ints) => Ok((
            ints
              .iter()
              .take(output_list_max_size)
              .map(|i| i.to_string())
              .collect::<Vec<String>>()
              .join(", "),
            None,
          )),
          Err(_) => Err(()),
        },

        ValueRepresentation::SignedVeryLong
        | ValueRepresentation::UnsignedVeryLong => match self.get_big_ints() {
          Ok(ints) => Ok((
            ints
              .iter()
              .take(output_list_max_size)
              .map(|i| i.to_string())
              .collect::<Vec<String>>()
              .join(", "),
            None,
          )),
          Err(_) => Err(()),
        },

        _ => Err(()),
      },

      RawDataElementValue::EncapsulatedPixelDataValue { items, .. } => {
        let mut total_size = 0;
        for item in items {
          total_size += item.len();
        }

        Ok((
          format!("Items: {}, bytes: {}", items.len(), total_size),
          None,
        ))
      }

      RawDataElementValue::SequenceValue { items } => {
        Ok((format!("Items: {}", items.len()), None))
      }
    };

    match result {
      Ok((s, suffix)) => {
        let suffix = suffix.unwrap_or_default();

        // Calculate width available for the value once the suffix isn't taken
        // into account. Always allow at least 10 characters.
        let output_width =
          std::cmp::max(output_width.saturating_sub(suffix.len()), 10);

        // If there are more codepoints than columns then convert to graphemes
        // and assume one column per grapheme for display
        if s.len() > output_width {
          let graphemes = UnicodeSegmentation::graphemes(s.as_str(), true)
            .collect::<Vec<&str>>();

          if graphemes.len() > output_width {
            format!("{} …{}", graphemes[0..output_width - 2].join(""), suffix)
          } else {
            format!("{}{}", s, suffix)
          }
        } else {
          format!("{}{}", s, suffix)
        }
      }
      Err(()) => "<error converting to string>".to_string(),
    }
  }
}

impl DataElementValue {
  /// Constructs a new data element binary value with the specified value
  /// representation. The only VR that's not allowed is
  /// [`ValueRepresentation::Sequence`]. The length of `bytes` must not exceed
  /// the maximum allowed for the VR, and, where applicable, must also be an
  /// exact multiple of the size of the contained data type. E.g. for the
  /// [`ValueRepresentation::UnsignedLong`] VR the length of `bytes` must be a
  /// multiple of 4.
  ///
  /// When the VR is a string type, `bytes` must be UTF-8 encoded in order for
  /// the value to be readable.
  ///
  pub fn new_binary(
    vr: ValueRepresentation,
    bytes: Rc<Vec<u8>>,
  ) -> Result<Self, DataError> {
    if vr == ValueRepresentation::Sequence {
      return Err(DataError::new_value_invalid(format!(
        "Value representation '{}' is not valid for binary data",
        vr
      )));
    }

    if vr.is_encoded_string() {
      if std::str::from_utf8(&bytes).is_err() {
        return Err(DataError::new_value_invalid(format!(
          "Bytes for '{}' are not valid UTF-8",
          vr
        )));
      }
    } else if vr.is_string() {
      let invalid_byte = (*bytes).iter().find(|b| {
        **b != 0x00
          && **b != 0x09
          && **b != 0x0A
          && **b != 0x0C
          && **b != 0x0D
          && **b != 0x1B
          && (**b < 0x20 || **b > 0x7E)
      });

      if let Some(invalid_byte) = invalid_byte {
        return Err(DataError::new_value_invalid(format!(
          "Bytes for '{}' has disallowed byte: 0x{:02X}",
          vr, *invalid_byte
        )));
      }
    }

    let value = Self::new_binary_unchecked(vr, bytes);

    value.validate_length()?;

    Ok(value)
  }

  /// Constructs a new data element binary value similar to
  /// [`Self::new_binary`], but does not validate `vr` or `bytes`.
  ///
  pub fn new_binary_unchecked(
    vr: ValueRepresentation,
    bytes: Rc<Vec<u8>>,
  ) -> Self {
    Self(RawDataElementValue::BinaryValue { vr, bytes })
  }

  /// Constructs a new data element lookup table descriptor value with the
  /// specified `vr`, which must be one of the following:
  ///
  /// - [`ValueRepresentation::SignedShort`]
  /// - [`ValueRepresentation::UnsignedShort`]
  ///
  /// The length of `bytes` must be exactly six.
  ///
  pub fn new_lookup_table_descriptor(
    vr: ValueRepresentation,
    bytes: Rc<Vec<u8>>,
  ) -> Result<Self, DataError> {
    if vr != ValueRepresentation::SignedShort
      && vr != ValueRepresentation::UnsignedShort
    {
      return Err(DataError::new_value_invalid(format!(
        "Value representation '{}' is not valid for lookup table descriptor \
            data",
        vr
      )));
    }

    let value = Self::new_lookup_table_descriptor_unchecked(vr, bytes);

    value.validate_length()?;

    Ok(value)
  }

  /// Constructs a new data element lookup table descriptor value similar to
  /// [`Self::new_lookup_table_descriptor`], but does not validate
  /// `vr` or `bytes`.
  ///
  pub fn new_lookup_table_descriptor_unchecked(
    vr: ValueRepresentation,
    bytes: Rc<Vec<u8>>,
  ) -> Self {
    Self(RawDataElementValue::LookupTableDescriptorValue { vr, bytes })
  }

  /// Constructs a new data element encapsulated pixel data value with the
  /// specified `vr`, which must be one of the following:
  ///
  /// - [`ValueRepresentation::OtherByteString`]
  /// - [`ValueRepresentation::OtherWordString`]
  ///
  /// Although the DICOM standard states that only
  /// [`ValueRepresentation::OtherByteString`] is valid for encapsulated pixel
  /// data, in practice this is not always followed.
  ///
  /// `items` specifies the data of the encapsulated pixel data items, where the
  /// first item is an optional basic offset table, and is followed by
  /// fragments of pixel data. Each item must be of even length. Ref: PS3.5 A.4.
  ///
  pub fn new_encapsulated_pixel_data(
    vr: ValueRepresentation,
    items: Vec<Rc<Vec<u8>>>,
  ) -> Result<Self, DataError> {
    if vr != ValueRepresentation::OtherByteString
      && vr != ValueRepresentation::OtherWordString
    {
      return Err(DataError::new_value_invalid(format!(
        "Value representation '{}' is not valid for encapsulated pixel data",
        vr
      )));
    }

    let value = Self::new_encapsulated_pixel_data_unchecked(vr, items);

    value.validate_length()?;

    Ok(value)
  }

  /// Constructs a new data element string value similar to
  /// [`Self::new_encapsulated_pixel_data`], but does not validate `vr` or
  /// `items`.
  ///
  pub fn new_encapsulated_pixel_data_unchecked(
    vr: ValueRepresentation,
    items: Vec<Rc<Vec<u8>>>,
  ) -> Self {
    Self(RawDataElementValue::EncapsulatedPixelDataValue { vr, items })
  }

  /// Creates a new `AgeString` data element value.
  ///
  pub fn new_age_string(
    value: &age_string::StructuredAge,
  ) -> Result<Self, DataError> {
    let bytes = value.to_bytes()?;

    Ok(Self::new_binary_unchecked(
      ValueRepresentation::AgeString,
      Rc::new(bytes),
    ))
  }

  /// Creates a new `ApplicationEntity` data element value.
  ///
  pub fn new_application_entity(value: &str) -> Result<Self, DataError> {
    new_string_list(ValueRepresentation::ApplicationEntity, &[value.trim()])
  }

  /// Creates a new `AttributeTag` data element value.
  ///
  pub fn new_attribute_tag(
    value: &[DataElementTag],
  ) -> Result<Self, DataError> {
    let bytes = attribute_tag::to_bytes(value);

    Self::new_binary(ValueRepresentation::AttributeTag, Rc::new(bytes))
  }

  /// Creates a new `CodeString` data element value.
  ///
  pub fn new_code_string(value: &[&str]) -> Result<Self, DataError> {
    new_string_list(
      ValueRepresentation::CodeString,
      &value.iter().map(|s| s.trim()).collect::<Vec<&str>>(),
    )
  }

  /// Creates a new `Date` data element value.
  ///
  pub fn new_date(value: &StructuredDate) -> Result<Self, DataError> {
    let bytes = value.to_bytes()?;

    Ok(Self::new_binary_unchecked(
      ValueRepresentation::Date,
      Rc::new(bytes),
    ))
  }

  /// Creates a new `DateTime` data element value.
  ///
  pub fn new_date_time(
    value: &date_time::StructuredDateTime,
  ) -> Result<Self, DataError> {
    let bytes = value.to_bytes()?;

    Ok(Self::new_binary_unchecked(
      ValueRepresentation::DateTime,
      Rc::new(bytes),
    ))
  }

  /// Creates a new `DecimalString` data element value.
  ///
  pub fn new_decimal_string(value: &[f64]) -> Result<Self, DataError> {
    let bytes = decimal_string::to_bytes(value);

    Self::new_binary(ValueRepresentation::DecimalString, Rc::new(bytes))
  }

  /// Creates a new `FloatingPointDouble` data element value.
  ///
  pub fn new_floating_point_double(value: &[f64]) -> Result<Self, DataError> {
    let mut bytes = vec![0; value.len() * 8];
    byteorder::LittleEndian::write_f64_into(value, &mut bytes);

    Self::new_binary(ValueRepresentation::FloatingPointDouble, Rc::new(bytes))
  }

  /// Creates a new `FloatingPointSingle` data element value.
  ///
  pub fn new_floating_point_single(value: &[f32]) -> Result<Self, DataError> {
    let mut bytes = vec![0; value.len() * 4];
    byteorder::LittleEndian::write_f32_into(value, &mut bytes);

    Self::new_binary(ValueRepresentation::FloatingPointSingle, Rc::new(bytes))
  }

  /// Creates a new `IntegerString` data element value.
  ///
  pub fn new_integer_string(value: &[i32]) -> Result<Self, DataError> {
    let bytes = integer_string::to_bytes(value);

    Self::new_binary(ValueRepresentation::IntegerString, Rc::new(bytes))
  }

  /// Creates a new `LongString` data element value.
  ///
  pub fn new_long_string(value: &[&str]) -> Result<Self, DataError> {
    new_string_list(
      ValueRepresentation::LongString,
      &value.iter().map(|s| s.trim()).collect::<Vec<&str>>(),
    )
  }

  /// Creates a new `LongText` data element value.
  ///
  pub fn new_long_text(value: String) -> Result<Self, DataError> {
    let vr = ValueRepresentation::LongText;

    let mut bytes = value.trim_end().to_string().into_bytes();
    vr.pad_bytes_to_even_length(&mut bytes);

    Self::new_binary(vr, Rc::new(bytes))
  }

  /// Creates a new `OtherByteString` data element value.
  ///
  pub fn new_other_byte_string(value: Vec<u8>) -> Result<Self, DataError> {
    Self::new_binary(ValueRepresentation::OtherByteString, Rc::new(value))
  }

  /// Creates a new `OtherDoubleString` data element value.
  ///
  pub fn new_other_double_string(value: &[f64]) -> Result<Self, DataError> {
    let mut bytes = vec![0; value.len() * 8];
    byteorder::LittleEndian::write_f64_into(value, &mut bytes);

    Self::new_binary(ValueRepresentation::OtherDoubleString, Rc::new(bytes))
  }

  /// Creates a new `OtherFloatString` data element value.
  ///
  pub fn new_other_float_string(value: &[f32]) -> Result<Self, DataError> {
    let mut bytes = vec![0; value.len() * 4];
    byteorder::LittleEndian::write_f32_into(value, &mut bytes);

    Self::new_binary(ValueRepresentation::OtherFloatString, Rc::new(bytes))
  }

  /// Creates a new `OtherLongString` data element value.
  ///
  pub fn new_other_long_string(value: Vec<u8>) -> Result<Self, DataError> {
    Self::new_binary(ValueRepresentation::OtherLongString, Rc::new(value))
  }

  /// Creates a new `OtherVeryLongString` data element value.
  ///
  pub fn new_other_very_long_string(value: Vec<u8>) -> Result<Self, DataError> {
    Self::new_binary(ValueRepresentation::OtherVeryLongString, Rc::new(value))
  }

  /// Creates a new `OtherWordString` data element value.
  ///
  pub fn new_other_word_string(value: Vec<u8>) -> Result<Self, DataError> {
    Self::new_binary(ValueRepresentation::OtherWordString, Rc::new(value))
  }

  /// Creates a new `PersonName` data element value.
  ///
  pub fn new_person_name(
    value: &[person_name::StructuredPersonName],
  ) -> Result<Self, DataError> {
    let bytes = person_name::to_bytes(value)?;

    Ok(Self::new_binary_unchecked(
      ValueRepresentation::PersonName,
      Rc::new(bytes),
    ))
  }

  /// Creates a new `Sequence` data element value.
  ///
  pub fn new_sequence(items: Vec<DataSet>) -> Self {
    Self(RawDataElementValue::SequenceValue { items })
  }

  /// Creates a new `ShortString` data element value.
  ///
  pub fn new_short_string(value: &[&str]) -> Result<Self, DataError> {
    let value = value.iter().map(|s| s.trim()).collect::<Vec<&str>>();

    new_string_list(ValueRepresentation::ShortString, &value)
  }

  /// Creates a new `ShortText` data element value.
  ///
  pub fn new_short_text(value: &str) -> Result<Self, DataError> {
    let vr = ValueRepresentation::ShortText;

    let mut bytes = value.trim_end().to_string().into_bytes();
    vr.pad_bytes_to_even_length(&mut bytes);

    Self::new_binary(vr, Rc::new(bytes))
  }

  /// Creates a new `SignedLong` data element value.
  ///
  pub fn new_signed_long(value: &[i32]) -> Result<Self, DataError> {
    let mut bytes = vec![0; value.len() * 4];
    byteorder::LittleEndian::write_i32_into(value, &mut bytes);

    Self::new_binary(ValueRepresentation::SignedLong, Rc::new(bytes))
  }

  /// Creates a new `SignedShort` data element value.
  ///
  pub fn new_signed_short(value: &[i16]) -> Result<Self, DataError> {
    let mut bytes = vec![0; value.len() * 2];
    byteorder::LittleEndian::write_i16_into(value, &mut bytes);

    Self::new_binary(ValueRepresentation::SignedShort, Rc::new(bytes))
  }

  /// Creates a new `SignedVeryLong` data element value.
  ///
  pub fn new_signed_very_long(value: &[i64]) -> Result<Self, DataError> {
    let mut bytes = vec![0; value.len() * 8];
    byteorder::LittleEndian::write_i64_into(value, &mut bytes);

    Self::new_binary(ValueRepresentation::SignedVeryLong, Rc::new(bytes))
  }

  /// Creates a new `Time` data element value.
  ///
  pub fn new_time(value: &time::StructuredTime) -> Result<Self, DataError> {
    let bytes = value.to_bytes()?;

    Ok(Self::new_binary_unchecked(
      ValueRepresentation::Time,
      Rc::new(bytes),
    ))
  }

  /// Creates a new `UniqueIdentifier` data element value.
  ///
  pub fn new_unique_identifier(value: &[&str]) -> Result<Self, DataError> {
    let bytes = unique_identifier::to_bytes(value)?;

    Self::new_binary(ValueRepresentation::UniqueIdentifier, Rc::new(bytes))
  }

  /// Creates a new `UniversalResourceIdentifier` data element value.
  ///
  pub fn new_universal_resource_identifier(
    value: &str,
  ) -> Result<Self, DataError> {
    let vr = ValueRepresentation::UniversalResourceIdentifier;

    let mut bytes = value.trim().to_owned().into_bytes();
    vr.pad_bytes_to_even_length(&mut bytes);

    Self::new_binary(vr, Rc::new(bytes))
  }

  /// Creates a new `Unknown` data element value.
  ///
  pub fn new_unknown(value: Vec<u8>) -> Result<Self, DataError> {
    Self::new_binary(ValueRepresentation::Unknown, Rc::new(value))
  }

  /// Creates a new `UnlimitedCharacters` data element value.
  ///
  pub fn new_unlimited_characters(value: &[&str]) -> Result<Self, DataError> {
    new_string_list(
      ValueRepresentation::UnlimitedCharacters,
      &value.iter().map(|s| s.trim_end()).collect::<Vec<&str>>(),
    )
  }

  /// Creates a new `UnlimitedText` data element value.
  ///
  pub fn new_unlimited_text(value: &str) -> Result<Self, DataError> {
    let vr = ValueRepresentation::UnlimitedText;

    let mut bytes = value.trim_end().to_owned().into_bytes();
    vr.pad_bytes_to_even_length(&mut bytes);

    Self::new_binary(vr, Rc::new(bytes))
  }

  /// Creates a new `UnsignedLong` data element value.
  ///
  pub fn new_unsigned_long(value: &[u32]) -> Result<Self, DataError> {
    let mut bytes = vec![0; value.len() * 4];
    byteorder::LittleEndian::write_u32_into(value, &mut bytes);

    Self::new_binary(ValueRepresentation::UnsignedLong, Rc::new(bytes))
  }

  /// Creates a new `UnsignedShort` data element value.
  ///
  pub fn new_unsigned_short(value: &[u16]) -> Result<Self, DataError> {
    let mut bytes = vec![0; value.len() * 2];
    byteorder::LittleEndian::write_u16_into(value, &mut bytes);

    Self::new_binary(ValueRepresentation::UnsignedShort, Rc::new(bytes))
  }

  /// Creates a new `UnsignedVeryLong` data element value.
  ///
  pub fn new_unsigned_very_long(value: &[u64]) -> Result<Self, DataError> {
    let mut bytes = vec![0; value.len() * 8];
    byteorder::LittleEndian::write_u64_into(value, &mut bytes);

    Self::new_binary(ValueRepresentation::UnsignedVeryLong, Rc::new(bytes))
  }

  /// Returns the value representation for a data element value.
  ///
  pub fn value_representation(&self) -> ValueRepresentation {
    match &self.0 {
      RawDataElementValue::BinaryValue { vr, .. }
      | RawDataElementValue::LookupTableDescriptorValue { vr, .. }
      | RawDataElementValue::EncapsulatedPixelDataValue { vr, .. } => *vr,
      RawDataElementValue::SequenceValue { .. } => {
        ValueRepresentation::Sequence
      }
    }
  }

  /// For data element values that hold binary data, returns that data.
  ///
  pub fn bytes(&self) -> Result<&Rc<Vec<u8>>, DataError> {
    match &self.0 {
      RawDataElementValue::BinaryValue { bytes, .. }
      | RawDataElementValue::LookupTableDescriptorValue { bytes, .. } => {
        Ok(bytes)
      }
      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// For data element values that hold encapsulated pixel data, returns a
  /// reference to the encapsulated items.
  ///
  pub fn encapsulated_pixel_data(
    &self,
  ) -> Result<&Vec<Rc<Vec<u8>>>, DataError> {
    match &self.0 {
      RawDataElementValue::EncapsulatedPixelDataValue { items, .. } => {
        Ok(items)
      }
      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// For data element values that hold a sequence, returns a reference to the
  /// sequence's items.
  ///
  pub fn sequence_items(&self) -> Result<&Vec<DataSet>, DataError> {
    match &self.0 {
      RawDataElementValue::SequenceValue { items } => Ok(items),
      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// For data element values that hold a sequence, returns a mutable reference
  /// to the sequence's items.
  ///
  pub fn sequence_items_mut(&mut self) -> Result<&mut Vec<DataSet>, DataError> {
    match &mut self.0 {
      RawDataElementValue::SequenceValue { items } => Ok(items),
      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// Returns the size in bytes of a data element value. This recurses through
  /// sequences and also includes a fixed per-value overhead, so never returns
  /// zero even for an empty data element value.
  ///
  pub fn total_byte_size(&self) -> u64 {
    let data_size = match &self.0 {
      RawDataElementValue::BinaryValue { bytes, .. }
      | RawDataElementValue::LookupTableDescriptorValue { bytes, .. } => {
        bytes.len() as u64
      }

      RawDataElementValue::EncapsulatedPixelDataValue { items, .. } => {
        items.len() as u64 * 8
          + items.iter().map(|item| item.len() as u64).sum::<u64>()
      }

      RawDataElementValue::SequenceValue { items } => {
        items.iter().map(|item| item.total_byte_size()).sum()
      }
    };

    let fixed_size = std::mem::size_of::<Self>() as u64;

    data_size + fixed_size
  }

  /// Returns the string contained in a data element value. This is only
  /// supported for value representations that either don't allow multiplicity,
  /// or those that do allow multiplicity but only one string is present in the
  /// value.
  ///
  pub fn get_string(&self) -> Result<&str, DataError> {
    match &self.0 {
      RawDataElementValue::BinaryValue { vr, bytes }
        if *vr == ValueRepresentation::ApplicationEntity
          || *vr == ValueRepresentation::LongText
          || *vr == ValueRepresentation::ShortText
          || *vr == ValueRepresentation::UniversalResourceIdentifier
          || *vr == ValueRepresentation::UnlimitedText =>
      {
        let string = std::str::from_utf8(bytes.as_slice()).map_err(|_| {
          DataError::new_value_invalid(
            "String bytes are not valid UTF-8".to_string(),
          )
        })?;

        Ok(string.trim_end_matches(['\u{0000}', ' ']))
      }

      _ => {
        let mut strings = self.get_strings()?;

        match strings.as_slice() {
          [_] => Ok(strings.pop().unwrap()),
          _ => Err(DataError::new_multiplicity_mismatch()),
        }
      }
    }
  }

  /// Returns the strings contained in a data element value. This is only
  /// supported for value representations that allow multiplicity.
  ///
  pub fn get_strings(&self) -> Result<Vec<&str>, DataError> {
    match &self.0 {
      RawDataElementValue::BinaryValue { vr, bytes }
        if *vr == ValueRepresentation::CodeString
          || *vr == ValueRepresentation::UniqueIdentifier
          || *vr == ValueRepresentation::LongString
          || *vr == ValueRepresentation::ShortString
          || *vr == ValueRepresentation::UnlimitedCharacters =>
      {
        let string = std::str::from_utf8(bytes.as_slice()).map_err(|_| {
          DataError::new_value_invalid(
            "String bytes are not valid UTF-8".to_string(),
          )
        })?;

        let strings = string
          .split('\\')
          .map(|s| s.trim_end_matches(['\u{0000}', ' ']))
          .collect::<Vec<&str>>();

        Ok(strings)
      }

      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// Returns the integer contained in a data element value. This is only
  /// supported for value representations that contain integer data and when
  /// exactly one integer is present.
  ///
  pub fn get_int(&self) -> Result<i64, DataError> {
    let ints = self.get_ints()?;

    match ints.as_slice() {
      [i] => Ok(*i),
      _ => Err(DataError::new_multiplicity_mismatch()),
    }
  }

  /// Returns the integers contained in a data element value. This is only
  /// supported for value representations that contain integer data.
  ///
  pub fn get_ints(&self) -> Result<Vec<i64>, DataError> {
    match &self.0 {
      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::IntegerString,
        bytes,
      } => Ok(
        integer_string::from_bytes(bytes)?
          .iter()
          .map(|i| *i as i64)
          .collect::<Vec<i64>>(),
      ),

      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::SignedLong,
        bytes,
      } => {
        if bytes.len() % 4 != 0 {
          return Err(DataError::new_value_invalid(
            "Invalid Int32 list".to_string(),
          ));
        }

        let mut values = Vec::with_capacity(bytes.len() / 4);
        for i32_bytes in bytes.chunks_exact(4) {
          values.push(byteorder::LittleEndian::read_i32(i32_bytes) as i64);
        }

        Ok(values)
      }

      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::SignedShort,
        bytes,
      } => {
        if bytes.len() % 2 != 0 {
          return Err(DataError::new_value_invalid(
            "Invalid Int16 list".to_string(),
          ));
        }

        let mut values = Vec::with_capacity(bytes.len() / 2);
        for i16_bytes in bytes.chunks_exact(2) {
          values.push(byteorder::LittleEndian::read_i16(i16_bytes) as i64);
        }

        Ok(values)
      }

      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::UnsignedLong,
        bytes,
      } => {
        if bytes.len() % 4 != 0 {
          return Err(DataError::new_value_invalid(
            "Invalid Uint32 list".to_string(),
          ));
        }

        let mut values = Vec::with_capacity(bytes.len() / 4);
        for u32_bytes in bytes.chunks_exact(4) {
          values.push(byteorder::LittleEndian::read_u32(u32_bytes) as i64);
        }

        Ok(values)
      }

      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::UnsignedShort,
        bytes,
      } => {
        if bytes.len() % 2 != 0 {
          return Err(DataError::new_value_invalid(
            "Invalid Uint16 list".to_string(),
          ));
        }

        let mut values = Vec::with_capacity(bytes.len() / 2);
        for u16_bytes in bytes.chunks_exact(2) {
          values.push(byteorder::LittleEndian::read_u16(u16_bytes) as i64);
        }

        Ok(values)
      }

      // Use the lookup table descriptor value's VR to determine how to
      // interpret the second 16-bit integer it contains.
      RawDataElementValue::LookupTableDescriptorValue { vr, bytes } => {
        if bytes.len() == 6
          && (*vr == ValueRepresentation::SignedShort
            || *vr == ValueRepresentation::UnsignedShort)
        {
          let entry_count =
            byteorder::LittleEndian::read_u16(&bytes[0..2]) as i64;

          let first_input_value = if *vr == ValueRepresentation::SignedShort {
            byteorder::LittleEndian::read_i16(&bytes[2..4]) as i64
          } else {
            byteorder::LittleEndian::read_u16(&bytes[2..4]) as i64
          };

          let bits_per_entry =
            byteorder::LittleEndian::read_u16(&bytes[4..6]) as i64;

          Ok(vec![entry_count, first_input_value, bits_per_entry])
        } else {
          Err(DataError::new_value_invalid(
            "Invalid lookup table descriptor".to_string(),
          ))
        }
      }

      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// Returns the big integer contained in a data element value. This is only
  /// supported for value representations that contain big integer data and when
  /// exactly one big integer is present.
  ///
  pub fn get_big_int(&self) -> Result<i128, DataError> {
    let ints = self.get_big_ints()?;

    match ints.as_slice() {
      [i] => Ok(*i),
      _ => Err(DataError::new_multiplicity_mismatch()),
    }
  }

  /// Returns the big integers contained in a data element value. This is only
  /// supported for value representations that contain big integer data.
  ///
  pub fn get_big_ints(&self) -> Result<Vec<i128>, DataError> {
    match &self.0 {
      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::SignedVeryLong,
        bytes,
      } => {
        if bytes.len() % 8 != 0 {
          return Err(DataError::new_value_invalid(
            "Invalid Int64 list".to_string(),
          ));
        }

        let mut values = Vec::with_capacity(bytes.len() / 8);
        for i64_bytes in bytes.chunks_exact(8) {
          values.push(byteorder::LittleEndian::read_i64(i64_bytes) as i128);
        }

        Ok(values)
      }

      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::UnsignedVeryLong,
        bytes,
      } => {
        if bytes.len() % 8 != 0 {
          return Err(DataError::new_value_invalid(
            "Invalid Uint64 list".to_string(),
          ));
        }

        let mut values = Vec::with_capacity(bytes.len() / 8);
        for u64_bytes in bytes.chunks_exact(8) {
          values.push(byteorder::LittleEndian::read_u64(u64_bytes) as i128);
        }

        Ok(values)
      }

      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// Returns the float contained in a data element value. This is only
  /// supported for value representations that contain floating point data and
  /// when exactly one float is present.
  ///
  pub fn get_float(&self) -> Result<f64, DataError> {
    let floats = self.get_floats()?;

    match floats.as_slice() {
      [f] => Ok(*f),
      _ => Err(DataError::new_multiplicity_mismatch()),
    }
  }

  /// Returns the floats contained in a data element value. This is only
  /// supported for value representations containing floating point data.
  ///
  pub fn get_floats(&self) -> Result<Vec<f64>, DataError> {
    match &self.0 {
      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::DecimalString,
        bytes,
      } => decimal_string::from_bytes(bytes.as_slice()),

      RawDataElementValue::BinaryValue { vr, bytes }
      | RawDataElementValue::BinaryValue { vr, bytes }
        if *vr == ValueRepresentation::FloatingPointDouble
          || *vr == ValueRepresentation::OtherDoubleString =>
      {
        if bytes.len() % 8 != 0 {
          return Err(DataError::new_value_invalid(
            "Invalid Float64 list".to_string(),
          ));
        }

        let mut values = Vec::with_capacity(bytes.len() / 8);
        for f64_bytes in bytes.chunks_exact(8) {
          values.push(byteorder::LittleEndian::read_f64(f64_bytes));
        }

        Ok(values)
      }

      RawDataElementValue::BinaryValue { vr, bytes }
        if *vr == ValueRepresentation::FloatingPointSingle
          || *vr == ValueRepresentation::OtherFloatString =>
      {
        if bytes.len() % 4 != 0 {
          return Err(DataError::new_value_invalid(
            "Invalid Float32 list".to_string(),
          ));
        }

        let mut values = Vec::with_capacity(bytes.len() / 4);
        for f32_bytes in bytes.chunks_exact(4) {
          values.push(byteorder::LittleEndian::read_f32(f32_bytes) as f64);
        }

        Ok(values)
      }

      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// Returns the structured age contained in a data element value. This is only
  /// supported for the `AgeString` value representation.
  ///
  pub fn get_age(&self) -> Result<age_string::StructuredAge, DataError> {
    match &self.0 {
      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::AgeString,
        bytes,
      } => StructuredAge::from_bytes(bytes),
      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// Returns the data element tags contained in a data element value. This is
  /// only supported for the `AttributeTag` value representation.
  ///
  pub fn get_attribute_tags(&self) -> Result<Vec<DataElementTag>, DataError> {
    match &self.0 {
      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::AttributeTag,
        bytes,
      } => attribute_tag::from_bytes(bytes),
      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// Returns the structured date contained in a data element value. This is
  /// only supported for the `Date` value representation.
  ///
  pub fn get_date(&self) -> Result<StructuredDate, DataError> {
    match &self.0 {
      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::Date,
        bytes,
      } => StructuredDate::from_bytes(bytes),
      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// Returns the structured date/time contained in a data element value. This
  /// is only supported for the `DateTime` value representation.
  ///
  pub fn get_date_time(
    &self,
  ) -> Result<date_time::StructuredDateTime, DataError> {
    match &self.0 {
      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::DateTime,
        bytes,
      } => StructuredDateTime::from_bytes(bytes),
      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// Returns the structured time contained in a data element value. This is
  /// only supported for the `Time` value representation.
  ///
  pub fn get_time(&self) -> Result<time::StructuredTime, DataError> {
    match &self.0 {
      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::Time,
        bytes,
      } => StructuredTime::from_bytes(bytes),
      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// Returns the float contained in a data element value. This is only
  /// supported for the `PersonName` value representation when exactly one
  /// person name is present.
  ///
  pub fn get_person_name(
    &self,
  ) -> Result<person_name::StructuredPersonName, DataError> {
    let mut person_names = self.get_person_names()?;

    match person_names.as_slice() {
      [_] => Ok(person_names.pop().unwrap()),
      _ => Err(DataError::new_multiplicity_mismatch()),
    }
  }

  /// Returns the structured time contained in a data element value. This is
  /// only supported for the `PersonName` value representation.
  ///
  pub fn get_person_names(
    &self,
  ) -> Result<Vec<person_name::StructuredPersonName>, DataError> {
    match &self.0 {
      RawDataElementValue::BinaryValue {
        vr: ValueRepresentation::PersonName,
        bytes,
      } => person_name::from_bytes(bytes),
      _ => Err(DataError::new_value_not_present()),
    }
  }

  /// Checks that the number of bytes stored in a data element value is valid
  /// for its value representation.
  ///
  pub fn validate_length(&self) -> Result<(), DataError> {
    let value_length = self.bytes().map(|bytes| bytes.len()).unwrap_or(0);

    match &self.0 {
      RawDataElementValue::LookupTableDescriptorValue { vr, .. } => {
        if value_length != 6 {
          return Err(DataError::new_value_length_invalid(
            *vr,
            value_length,
            "Lookup table descriptor length must be exactly 6 bytes"
              .to_string(),
          ));
        }
      }

      RawDataElementValue::BinaryValue { vr, .. } => {
        let value_representation::LengthRequirements {
          bytes_max,
          bytes_multiple_of,
          string_characters_max: _,
        } = vr.length_requirements();

        let bytes_multiple_of = bytes_multiple_of.unwrap_or(2);

        // Check against the length requirements for this VR
        if value_length > bytes_max {
          return Err(DataError::new_value_length_invalid(
            *vr,
            value_length,
            format!("Must not exceed {} bytes", bytes_max),
          ));
        }

        if value_length % bytes_multiple_of != 0 {
          return Err(DataError::new_value_length_invalid(
            *vr,
            value_length,
            format!("Must be a multiple of {} bytes", bytes_multiple_of),
          ));
        }
      }

      RawDataElementValue::EncapsulatedPixelDataValue { vr, items } => {
        for item in items {
          let item_length = item.len();

          if item_length > 0xFFFFFFFE {
            return Err(DataError::new_value_length_invalid(
              *vr,
              item_length,
              format!("Must not exceed {} bytes", 0xFFFFFFFEu32),
            ));
          }

          if item_length % 2 != 0 {
            return Err(DataError::new_value_length_invalid(
              *vr,
              item_length,
              "Must be a multiple of 2 bytes".to_string(),
            ));
          }
        }
      }

      RawDataElementValue::SequenceValue { .. } => (),
    };

    Ok(())
  }
}

/// Creates a data element containing a multi-valued string. This checks that
/// the individual values are valid and then combines them into final bytes.
///
fn new_string_list(
  vr: ValueRepresentation,
  value: &[&str],
) -> Result<DataElementValue, DataError> {
  let string_characters_max = vr
    .length_requirements()
    .string_characters_max
    .unwrap_or(0xFFFFFFFE);

  // Check no values exceed the max length or contain backslashes that would
  // affect the multiplicity once joined together
  for s in value.iter() {
    if s.len() > string_characters_max {
      return Err(DataError::new_value_invalid(format!(
        "String list item is longer than the max length of {}",
        string_characters_max
      )));
    }

    if s.contains('\\') {
      return Err(DataError::new_value_invalid(
        "String list item contains backslashes".to_string(),
      ));
    }
  }

  let mut bytes = value.join("\\").into_bytes();
  vr.pad_bytes_to_even_length(&mut bytes);

  DataElementValue::new_binary(vr, Rc::new(bytes))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn value_representation_test() {
    assert_eq!(
      DataElementValue::new_long_string(&["123"])
        .unwrap()
        .value_representation(),
      ValueRepresentation::LongString
    );

    assert_eq!(
      DataElementValue::new_floating_point_single(&[1.0])
        .unwrap()
        .value_representation(),
      ValueRepresentation::FloatingPointSingle
    );

    assert_eq!(
      DataElementValue::new_lookup_table_descriptor_unchecked(
        ValueRepresentation::UnsignedShort,
        Rc::new(vec![0; 6])
      )
      .value_representation(),
      ValueRepresentation::UnsignedShort
    );

    assert_eq!(
      DataElementValue::new_encapsulated_pixel_data_unchecked(
        ValueRepresentation::OtherWordString,
        vec![]
      )
      .value_representation(),
      ValueRepresentation::OtherWordString
    );

    assert_eq!(
      DataElementValue::new_sequence(vec![]).value_representation(),
      ValueRepresentation::Sequence
    );
  }

  #[test]
  fn bytes_test() {
    assert_eq!(
      DataElementValue::new_long_string(&["12"]).unwrap().bytes(),
      Ok(&Rc::new(b"12".to_vec()))
    );

    assert_eq!(
      DataElementValue::new_floating_point_single(&[1.0])
        .unwrap()
        .bytes(),
      Ok(&Rc::new(vec![0, 0, 0x80, 0x3F]))
    );

    assert_eq!(
      DataElementValue::new_lookup_table_descriptor_unchecked(
        ValueRepresentation::UnsignedShort,
        Rc::new(vec![0, 1, 2, 3, 4, 5])
      )
      .bytes(),
      Ok(&Rc::new(vec![0, 1, 2, 3, 4, 5]))
    );

    assert_eq!(
      DataElementValue::new_encapsulated_pixel_data_unchecked(
        ValueRepresentation::OtherWordString,
        vec![]
      )
      .bytes(),
      Err(DataError::new_value_not_present())
    );

    assert_eq!(
      DataElementValue::new_sequence(vec![]).bytes(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_string_test() {
    assert_eq!(
      DataElementValue::new_application_entity("A")
        .unwrap()
        .get_string(),
      Ok("A")
    );

    assert_eq!(
      DataElementValue::new_long_text("A".to_string())
        .unwrap()
        .get_string(),
      Ok("A")
    );

    assert_eq!(
      DataElementValue::new_short_text("A").unwrap().get_string(),
      Ok("A")
    );

    assert_eq!(
      DataElementValue::new_universal_resource_identifier("A")
        .unwrap()
        .get_string(),
      Ok("A")
    );

    assert_eq!(
      DataElementValue::new_unlimited_text("A")
        .unwrap()
        .get_string(),
      Ok("A")
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::ShortText,
        Rc::new(vec![0xD0])
      )
      .get_string(),
      Err(DataError::new_value_invalid(
        "String bytes are not valid UTF-8".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_long_string(&["A"])
        .unwrap()
        .get_string(),
      Ok("A")
    );

    assert_eq!(
      DataElementValue::new_long_string(&["A", "B"])
        .unwrap()
        .get_string(),
      Err(DataError::new_multiplicity_mismatch())
    );

    assert_eq!(
      DataElementValue::new_unsigned_short(&[1])
        .unwrap()
        .get_string(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_strings_test() {
    assert_eq!(
      DataElementValue::new_code_string(&["A", "B"])
        .unwrap()
        .get_strings(),
      Ok(vec!["A", "B"])
    );

    assert_eq!(
      DataElementValue::new_unique_identifier(&["1.2", "3.4"])
        .unwrap()
        .get_strings(),
      Ok(vec!["1.2", "3.4"])
    );

    assert_eq!(
      DataElementValue::new_long_string(&["A", "B"])
        .unwrap()
        .get_strings(),
      Ok(vec!["A", "B"])
    );

    assert_eq!(
      DataElementValue::new_short_string(&["A", "B"])
        .unwrap()
        .get_strings(),
      Ok(vec!["A", "B"])
    );

    assert_eq!(
      DataElementValue::new_unlimited_characters(&["A", "B"])
        .unwrap()
        .get_strings(),
      Ok(vec!["A", "B"])
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::ShortString,
        Rc::new(vec![0xD0])
      )
      .get_strings(),
      Err(DataError::new_value_invalid(
        "String bytes are not valid UTF-8".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_long_text("A".to_string())
        .unwrap()
        .get_strings(),
      Err(DataError::new_value_not_present())
    );

    assert_eq!(
      DataElementValue::new_unsigned_short(&[1])
        .unwrap()
        .get_strings(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_int_test() {
    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::IntegerString,
        Rc::new(b"  123   ".to_vec())
      )
      .get_int(),
      Ok(123)
    );

    assert_eq!(
      DataElementValue::new_unsigned_long(&[1234])
        .unwrap()
        .get_int(),
      Ok(1234)
    );

    assert_eq!(
      DataElementValue::new_unsigned_long(&[123, 456])
        .unwrap()
        .get_int(),
      Err(DataError::new_multiplicity_mismatch())
    );

    assert_eq!(
      DataElementValue::new_long_text("123".to_string())
        .unwrap()
        .get_int(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_ints_test() {
    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::IntegerString,
        Rc::new(b" 123 \\456 ".to_vec())
      )
      .get_ints(),
      Ok(vec![123, 456])
    );

    assert_eq!(
      DataElementValue::new_signed_long(&[i32::MIN, i32::MAX])
        .unwrap()
        .get_ints(),
      Ok(vec![i32::MIN as i64, i32::MAX as i64])
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::SignedLong,
        Rc::new(vec![0])
      )
      .get_ints(),
      Err(DataError::new_value_invalid(
        "Invalid Int32 list".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_signed_short(&[i16::MIN, i16::MAX])
        .unwrap()
        .get_ints(),
      Ok(vec![i16::MIN as i64, i16::MAX as i64])
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::SignedShort,
        Rc::new(vec![0])
      )
      .get_ints(),
      Err(DataError::new_value_invalid(
        "Invalid Int16 list".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_unsigned_long(&[u32::MIN, u32::MAX])
        .unwrap()
        .get_ints(),
      Ok(vec![u32::MIN as i64, u32::MAX as i64])
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::UnsignedLong,
        Rc::new(vec![0])
      )
      .get_ints(),
      Err(DataError::new_value_invalid(
        "Invalid Uint32 list".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_unsigned_short(&[u16::MIN, u16::MAX])
        .unwrap()
        .get_ints(),
      Ok(vec![u16::MIN as i64, u16::MAX as i64])
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::UnsignedShort,
        Rc::new(vec![0]),
      )
      .get_ints(),
      Err(DataError::new_value_invalid(
        "Invalid Uint16 list".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_lookup_table_descriptor_unchecked(
        ValueRepresentation::SignedShort,
        Rc::new(vec![0x34, 0x12, 0x00, 0x80, 0x78, 0x56])
      )
      .get_ints(),
      Ok(vec![0x1234, -0x8000, 0x5678])
    );

    assert_eq!(
      DataElementValue::new_lookup_table_descriptor_unchecked(
        ValueRepresentation::UnsignedShort,
        Rc::new(vec![0x34, 0x12, 0x00, 0x80, 0x78, 0x56])
      )
      .get_ints(),
      Ok(vec![0x1234, 0x8000, 0x5678])
    );

    assert_eq!(
      DataElementValue::new_lookup_table_descriptor_unchecked(
        ValueRepresentation::OtherWordString,
        Rc::new(vec![0, 0, 0, 0, 0, 0])
      )
      .get_ints(),
      Err(DataError::new_value_invalid(
        "Invalid lookup table descriptor".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_lookup_table_descriptor_unchecked(
        ValueRepresentation::UnsignedShort,
        Rc::new(vec![0, 0, 0, 0])
      )
      .get_ints(),
      Err(DataError::new_value_invalid(
        "Invalid lookup table descriptor".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_floating_point_single(&[123.0])
        .unwrap()
        .get_ints(),
      Err(DataError::new_value_not_present())
    );

    assert_eq!(
      DataElementValue::new_long_text("123".to_string())
        .unwrap()
        .get_ints(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_big_int_test() {
    assert_eq!(
      DataElementValue::new_signed_very_long(&[i64::MIN])
        .unwrap()
        .get_big_int(),
      Ok(i64::MIN as i128)
    );

    assert_eq!(
      DataElementValue::new_signed_very_long(&[i64::MAX])
        .unwrap()
        .get_big_int(),
      Ok(i64::MAX as i128)
    );

    assert_eq!(
      DataElementValue::new_unsigned_very_long(&[1234, 1234])
        .unwrap()
        .get_big_int(),
      Err(DataError::new_multiplicity_mismatch())
    );

    assert_eq!(
      DataElementValue::new_long_text("123".to_string())
        .unwrap()
        .get_big_int(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_big_ints_test() {
    assert_eq!(
      DataElementValue::new_signed_very_long(&[i64::MIN, i64::MAX])
        .unwrap()
        .get_big_ints(),
      Ok(vec![i64::MIN as i128, i64::MAX as i128])
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::SignedVeryLong,
        Rc::new(vec![0])
      )
      .get_big_ints(),
      Err(DataError::new_value_invalid(
        "Invalid Int64 list".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_unsigned_very_long(&[u64::MIN, u64::MAX])
        .unwrap()
        .get_big_ints(),
      Ok(vec![u64::MIN as i128, u64::MAX as i128])
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::UnsignedVeryLong,
        Rc::new(vec![0])
      )
      .get_big_ints(),
      Err(DataError::new_value_invalid(
        "Invalid Uint64 list".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_floating_point_single(&[123.0])
        .unwrap()
        .get_big_ints(),
      Err(DataError::new_value_not_present())
    );

    assert_eq!(
      DataElementValue::new_long_text("123".to_string())
        .unwrap()
        .get_big_ints(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_float_test() {
    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::DecimalString,
        Rc::new(b" 1.2   ".to_vec())
      )
      .get_float(),
      Ok(1.2)
    );

    assert_eq!(
      DataElementValue::new_floating_point_single(&[1.0])
        .unwrap()
        .get_float(),
      Ok(1.0)
    );

    assert_eq!(
      DataElementValue::new_floating_point_single(&[f32::INFINITY])
        .unwrap()
        .get_float(),
      Ok(f64::INFINITY)
    );

    assert_eq!(
      DataElementValue::new_floating_point_double(&[1.2, 3.4])
        .unwrap()
        .get_float(),
      Err(DataError::new_multiplicity_mismatch())
    );

    assert_eq!(
      DataElementValue::new_long_text("1.2".to_string())
        .unwrap()
        .get_float(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_floats_test() {
    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::DecimalString,
        Rc::new(b" 1.2  \\3.4".to_vec())
      )
      .get_floats(),
      Ok(vec![1.2, 3.4])
    );

    assert_eq!(
      DataElementValue::new_floating_point_double(&[1.0, 2.0])
        .unwrap()
        .get_floats(),
      Ok(vec![1.0, 2.0])
    );

    assert_eq!(
      DataElementValue::new_other_double_string(&[1.0, 2.0])
        .unwrap()
        .get_floats(),
      Ok(vec![1.0, 2.0])
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::FloatingPointDouble,
        Rc::new(vec![0, 0, 0, 0])
      )
      .get_floats(),
      Err(DataError::new_value_invalid(
        "Invalid Float64 list".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_floating_point_single(&[1.0, 2.0])
        .unwrap()
        .get_floats(),
      Ok(vec![1.0, 2.0])
    );

    assert_eq!(
      DataElementValue::new_other_float_string(&[1.0, 2.0])
        .unwrap()
        .get_floats(),
      Ok(vec![1.0, 2.0])
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::FloatingPointSingle,
        Rc::new(vec![0, 0])
      )
      .get_floats(),
      Err(DataError::new_value_invalid(
        "Invalid Float32 list".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_long_text("1.2".to_string())
        .unwrap()
        .get_floats(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_age_test() {
    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::AgeString,
        Rc::new(b"001D".to_vec())
      )
      .get_age(),
      Ok(age_string::StructuredAge {
        number: 1,
        unit: age_string::AgeUnit::Days
      })
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::Date,
        Rc::new(vec![])
      )
      .get_age(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_date_test() {
    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::Date,
        Rc::new(b"20000101".to_vec())
      )
      .get_date(),
      Ok(StructuredDate {
        year: 2000,
        month: 1,
        day: 1
      })
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::Time,
        Rc::new(vec![])
      )
      .get_date(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_date_time_test() {
    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::DateTime,
        Rc::new(b"20000101123043.5".to_vec())
      )
      .get_date_time(),
      Ok(date_time::StructuredDateTime {
        year: 2000,
        month: Some(1),
        day: Some(1),
        hour: Some(12),
        minute: Some(30),
        second: Some(43.5),
        time_zone_offset: None,
      })
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::Date,
        Rc::new(vec![])
      )
      .get_date_time(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_time_test() {
    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::Time,
        Rc::new(b"235921.2".to_vec())
      )
      .get_time(),
      Ok(time::StructuredTime {
        hour: 23,
        minute: Some(59),
        second: Some(21.2)
      })
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::Date,
        Rc::new(vec![])
      )
      .get_time(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn get_person_name_test() {
    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::PersonName,
        Rc::new(vec![])
      )
      .get_person_name(),
      Ok(person_name::StructuredPersonName {
        alphabetic: None,
        ideographic: None,
        phonetic: None
      })
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::PersonName,
        Rc::new(b"\\".to_vec())
      )
      .get_person_name(),
      Err(DataError::new_multiplicity_mismatch())
    );
  }

  #[test]
  fn get_person_names_test() {
    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::PersonName,
        Rc::new(b"\\ ".to_vec())
      )
      .get_person_names(),
      Ok(vec![
        person_name::StructuredPersonName {
          alphabetic: None,
          ideographic: None,
          phonetic: None
        },
        person_name::StructuredPersonName {
          alphabetic: None,
          ideographic: None,
          phonetic: None
        }
      ])
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::Date,
        Rc::new(vec![])
      )
      .get_person_names(),
      Err(DataError::new_value_not_present())
    );
  }

  #[test]
  fn to_string_test() {
    let tag = DataElementTag::new(0, 0);

    assert_eq!(
      DataElementValue::new_code_string(&["DERIVED", "SECONDARY"])
        .unwrap()
        .to_string(tag, 80),
      "\"DERIVED\", \"SECONDARY\"".to_string()
    );

    assert_eq!(
      DataElementValue::new_code_string(&["CT"])
        .unwrap()
        .to_string(registry::MODALITY.tag, 80),
      "\"CT\" (Computed Tomography)".to_string()
    );

    assert_eq!(
      DataElementValue::new_unique_identifier(&["1.23"])
        .unwrap()
        .to_string(tag, 80),
      "\"1.23\"".to_string()
    );

    assert_eq!(
      DataElementValue::new_unique_identifier(&["1.2.840.10008.1.2"])
        .unwrap()
        .to_string(tag, 80),
      "\"1.2.840.10008.1.2\" (Implicit VR Little Endian)".to_string()
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::PersonName,
        Rc::new(vec![0xFF, 0xFF])
      )
      .to_string(tag, 80),
      "!! Invalid UTF-8 data".to_string()
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::AttributeTag,
        Rc::new(vec![0x34, 0x12, 0x78, 0x56])
      )
      .to_string(tag, 80),
      "(1234,5678)".to_string()
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::AttributeTag,
        Rc::new(vec![0])
      )
      .to_string(tag, 80),
      "<error converting to string>".to_string()
    );

    assert_eq!(
      DataElementValue::new_floating_point_single(&[
        1.0,
        2.5,
        f32::INFINITY,
        -f32::INFINITY,
        f32::NAN
      ])
      .unwrap()
      .to_string(tag, 80),
      "1.0, 2.5, Infinity, -Infinity, NaN".to_string()
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::FloatingPointDouble,
        Rc::new(vec![0, 0, 0, 0])
      )
      .to_string(tag, 80),
      "<error converting to string>".to_string()
    );

    assert_eq!(
      DataElementValue::new_other_byte_string(vec![0, 1, 2, 3])
        .unwrap()
        .to_string(tag, 80),
      "[00 01 02 03]".to_string()
    );

    assert_eq!(
      DataElementValue::new_other_byte_string(vec![0; 128])
        .unwrap()
        .to_string(tag, 80),
      "[00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 \
        00 00 00 …"
        .to_string()
    );

    assert_eq!(
      DataElementValue::new_signed_short(&[4000, -30000])
        .unwrap()
        .to_string(tag, 80),
      "4000, -30000".to_string()
    );

    assert_eq!(
      DataElementValue::new_lookup_table_descriptor_unchecked(
        ValueRepresentation::UnsignedShort,
        Rc::new(vec![0xA0, 0x0F, 0x40, 0x9C, 0x50, 0xC3])
      )
      .to_string(tag, 80),
      "4000, 40000, 50000".to_string()
    );

    assert_eq!(
      DataElementValue::new_lookup_table_descriptor_unchecked(
        ValueRepresentation::SignedShort,
        Rc::new(vec![0xA0, 0x0F, 0xE0, 0xB1, 0x50, 0xC3])
      )
      .to_string(tag, 80),
      "4000, -20000, 50000".to_string()
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::SignedShort,
        Rc::new(vec![0])
      )
      .to_string(tag, 80),
      "<error converting to string>".to_string()
    );

    assert_eq!(
      DataElementValue::new_encapsulated_pixel_data_unchecked(
        ValueRepresentation::OtherByteString,
        vec![Rc::new(vec![1, 2]), Rc::new(vec![3, 4])],
      )
      .to_string(tag, 80),
      "Items: 2, bytes: 4".to_string()
    );

    assert_eq!(
      DataElementValue::new_sequence(vec![DataSet::new()]).to_string(tag, 80),
      "Items: 1".to_string()
    );
  }

  #[test]
  fn validate_length_test() {
    assert_eq!(
      DataElementValue::new_lookup_table_descriptor_unchecked(
        ValueRepresentation::SignedShort,
        Rc::new(vec![0; 6])
      )
      .validate_length(),
      Ok(())
    );

    assert_eq!(
      DataElementValue::new_lookup_table_descriptor_unchecked(
        ValueRepresentation::SignedShort,
        Rc::new(vec![0; 4])
      )
      .validate_length(),
      Err(DataError::new_value_length_invalid(
        ValueRepresentation::SignedShort,
        4,
        "Lookup table descriptor length must be exactly 6 bytes".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::ShortText,
        Rc::new(vec![0; 0x10000])
      )
      .validate_length(),
      Err(DataError::new_value_length_invalid(
        ValueRepresentation::ShortText,
        65536,
        "Must not exceed 65534 bytes".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::UnsignedVeryLong,
        Rc::new(vec![0; 7])
      )
      .validate_length(),
      Err(DataError::new_value_length_invalid(
        ValueRepresentation::UnsignedVeryLong,
        7,
        "Must be a multiple of 8 bytes".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_encapsulated_pixel_data_unchecked(
        ValueRepresentation::OtherWordString,
        vec![Rc::new(vec![0; 2])]
      )
      .validate_length(),
      Ok(())
    );

    assert_eq!(
      DataElementValue::new_encapsulated_pixel_data_unchecked(
        ValueRepresentation::OtherWordString,
        vec![Rc::new(vec![0; 3])]
      )
      .validate_length(),
      Err(DataError::new_value_length_invalid(
        ValueRepresentation::OtherWordString,
        3,
        "Must be a multiple of 2 bytes".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_encapsulated_pixel_data_unchecked(
        ValueRepresentation::OtherWordString,
        vec![Rc::new(vec![0; 0xFFFFFFFF])]
      )
      .validate_length(),
      Err(DataError::new_value_length_invalid(
        ValueRepresentation::OtherWordString,
        4294967295,
        "Must not exceed 4294967294 bytes".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_sequence(vec![]).validate_length(),
      Ok(())
    );
  }

  #[test]
  fn new_age_string_test() {
    assert_eq!(
      DataElementValue::new_age_string(&age_string::StructuredAge {
        number: 99,
        unit: age_string::AgeUnit::Years
      }),
      DataElementValue::new_binary(
        ValueRepresentation::AgeString,
        Rc::new(b"099Y".to_vec())
      )
    );
  }

  #[test]
  fn new_application_entity_test() {
    assert_eq!(
      DataElementValue::new_application_entity("TEST  "),
      DataElementValue::new_binary(
        ValueRepresentation::ApplicationEntity,
        Rc::new(b"TEST".to_vec())
      )
    );

    assert_eq!(
      DataElementValue::new_application_entity("A".repeat(17).as_str()),
      Err(DataError::new_value_length_invalid(
        ValueRepresentation::ApplicationEntity,
        18,
        "Must not exceed 16 bytes".to_string(),
      ))
    );
  }

  #[test]
  fn new_attribute_tag_test() {
    assert_eq!(
      DataElementValue::new_attribute_tag(&[
        DataElementTag::new(0x0123, 0x4567),
        DataElementTag::new(0x89AB, 0xCDEF)
      ]),
      DataElementValue::new_binary(
        ValueRepresentation::AttributeTag,
        Rc::new(vec![0x23, 0x01, 0x67, 0x45, 0xAB, 0x89, 0xEF, 0xCD]),
      )
    );
  }

  #[test]
  fn new_code_string_test() {
    assert_eq!(
      DataElementValue::new_code_string(&["DERIVED ", "SECONDARY"]),
      DataElementValue::new_binary(
        ValueRepresentation::CodeString,
        Rc::new(b"DERIVED\\SECONDARY ".to_vec()),
      )
    );

    assert_eq!(
      DataElementValue::new_code_string(&["\\"]),
      Err(DataError::new_value_invalid(
        "String list item contains backslashes".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_code_string(&["A".repeat(17).as_str()]),
      Err(DataError::new_value_invalid(
        "String list item is longer than the max length of 16".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_code_string(&["é"]),
      Err(DataError::new_value_invalid(
        "Bytes for 'CS' has disallowed byte: 0xC3".to_string(),
      ))
    );
  }

  #[test]
  fn new_date_test() {
    assert_eq!(
      DataElementValue::new_date(&StructuredDate {
        year: 2024,
        month: 2,
        day: 14
      }),
      DataElementValue::new_binary(
        ValueRepresentation::Date,
        Rc::new(b"20240214".to_vec()),
      )
    );
  }

  #[test]
  fn new_date_time_test() {
    assert_eq!(
      DataElementValue::new_date_time(&date_time::StructuredDateTime {
        year: 2024,
        month: Some(2),
        day: Some(14),
        hour: Some(22),
        minute: Some(5),
        second: Some(46.1),
        time_zone_offset: Some(800)
      }),
      DataElementValue::new_binary(
        ValueRepresentation::DateTime,
        Rc::new(b"20240214220546.1+0800 ".to_vec()),
      )
    );
  }

  #[test]
  fn new_decimal_string_test() {
    assert_eq!(
      DataElementValue::new_decimal_string(&[1.2, -3.45]),
      DataElementValue::new_binary(
        ValueRepresentation::DecimalString,
        Rc::new(b"1.2\\-3.45 ".to_vec()),
      )
    );
  }

  #[test]
  fn new_floating_point_double_test() {
    assert_eq!(
      DataElementValue::new_floating_point_double(&[1.2, -3.45]),
      DataElementValue::new_binary(
        ValueRepresentation::FloatingPointDouble,
        Rc::new(vec![
          0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0xF3, 0x3F, 0x9A, 0x99, 0x99,
          0x99, 0x99, 0x99, 0xB, 0xC0,
        ])
      )
    );
  }

  #[test]
  fn new_floating_point_single_test() {
    assert_eq!(
      DataElementValue::new_floating_point_single(&[1.2, -3.45]),
      DataElementValue::new_binary(
        ValueRepresentation::FloatingPointSingle,
        Rc::new(vec![0x9A, 0x99, 0x99, 0x3F, 0xCD, 0xCC, 0x5C, 0xC0]),
      )
    );
  }

  #[test]
  fn new_integer_string_test() {
    assert_eq!(
      DataElementValue::new_integer_string(&[10, 2_147_483_647]),
      DataElementValue::new_binary(
        ValueRepresentation::IntegerString,
        Rc::new(b"10\\2147483647 ".to_vec()),
      )
    );
  }

  #[test]
  fn new_long_string_test() {
    assert_eq!(
      DataElementValue::new_long_string(&["AA", "BB"]),
      DataElementValue::new_binary(
        ValueRepresentation::LongString,
        Rc::new(b"AA\\BB ".to_vec()),
      )
    );
  }

  #[test]
  fn new_long_text_test() {
    assert_eq!(
      DataElementValue::new_long_text("ABC".to_string()),
      DataElementValue::new_binary(
        ValueRepresentation::LongText,
        Rc::new(b"ABC ".to_vec()),
      )
    );
  }

  #[test]
  fn new_other_byte_string_test() {
    assert_eq!(
      DataElementValue::new_other_byte_string(vec![1, 2]),
      DataElementValue::new_binary(
        ValueRepresentation::OtherByteString,
        Rc::new(vec![1, 2]),
      )
    );
  }

  #[test]
  fn new_other_double_string_test() {
    assert_eq!(
      DataElementValue::new_other_double_string(&[1.2, -3.45]),
      DataElementValue::new_binary(
        ValueRepresentation::OtherDoubleString,
        Rc::new(vec![
          0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0xF3, 0x3F, 0x9A, 0x99, 0x99,
          0x99, 0x99, 0x99, 0xB, 0xC0,
        ]),
      )
    );
  }

  #[test]
  fn new_other_float_string_test() {
    assert_eq!(
      DataElementValue::new_other_float_string(&[1.2, -3.45]),
      DataElementValue::new_binary(
        ValueRepresentation::OtherFloatString,
        Rc::new(vec![0x9A, 0x99, 0x99, 0x3F, 0xCD, 0xCC, 0x5C, 0xC0]),
      )
    );
  }

  #[test]
  fn new_other_long_string_test() {
    assert_eq!(
      DataElementValue::new_other_long_string(vec![0, 1, 2]),
      Err(DataError::new_value_length_invalid(
        ValueRepresentation::OtherLongString,
        3,
        "Must be a multiple of 4 bytes".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_other_long_string(vec![0, 1, 2, 3]),
      DataElementValue::new_binary(
        ValueRepresentation::OtherLongString,
        Rc::new(vec![0, 1, 2, 3]),
      )
    );
  }

  #[test]
  fn new_other_very_long_string_test() {
    assert_eq!(
      DataElementValue::new_other_very_long_string(vec![0, 1, 2, 3, 4, 5, 6]),
      Err(DataError::new_value_length_invalid(
        ValueRepresentation::OtherVeryLongString,
        7,
        "Must be a multiple of 8 bytes".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_other_very_long_string(vec![
        0, 1, 2, 3, 4, 5, 6, 7
      ]),
      DataElementValue::new_binary(
        ValueRepresentation::OtherVeryLongString,
        Rc::new(vec![0, 1, 2, 3, 4, 5, 6, 7]),
      )
    );
  }

  #[test]
  fn new_other_word_string_test() {
    assert_eq!(
      DataElementValue::new_other_word_string(vec![0, 1, 2]),
      Err(DataError::new_value_length_invalid(
        ValueRepresentation::OtherWordString,
        3,
        "Must be a multiple of 2 bytes".to_string(),
      ))
    );

    assert_eq!(
      DataElementValue::new_other_word_string(vec![0, 1]),
      DataElementValue::new_binary(
        ValueRepresentation::OtherWordString,
        Rc::new(vec![0, 1]),
      )
    );
  }

  #[test]
  fn new_person_name_test() {
    assert_eq!(
      DataElementValue::new_person_name(&[
        person_name::StructuredPersonName {
          alphabetic: None,
          ideographic: Some(person_name::PersonNameComponents {
            last_name: "1".to_string(),
            first_name: " 2 ".to_string(),
            middle_name: "3".to_string(),
            prefix: "4".to_string(),
            suffix: "5".to_string()
          }),
          phonetic: None,
        },
        person_name::StructuredPersonName {
          alphabetic: None,
          ideographic: None,
          phonetic: Some(person_name::PersonNameComponents {
            last_name: "1".to_string(),
            first_name: "2".to_string(),
            middle_name: "3".to_string(),
            prefix: "4".to_string(),
            suffix: "5".to_string()
          }),
        }
      ]),
      DataElementValue::new_binary(
        ValueRepresentation::PersonName,
        Rc::new(b"=1^2^3^4^5\\==1^2^3^4^5".to_vec()),
      )
    );
  }

  #[test]
  fn new_short_string_test() {
    assert_eq!(
      DataElementValue::new_short_string(&[" AA ", "BB"]),
      DataElementValue::new_binary(
        ValueRepresentation::ShortString,
        Rc::new(b"AA\\BB ".to_vec()),
      )
    );
  }

  #[test]
  fn new_short_text_test() {
    assert_eq!(
      DataElementValue::new_short_text(" ABC "),
      DataElementValue::new_binary(
        ValueRepresentation::ShortText,
        Rc::new(b" ABC".to_vec()),
      )
    );
  }

  #[test]
  fn new_signed_long_test() {
    assert_eq!(
      DataElementValue::new_signed_long(&[2_000_000_000, -2_000_000_000]),
      DataElementValue::new_binary(
        ValueRepresentation::SignedLong,
        Rc::new(vec![0x00, 0x94, 0x35, 0x77, 0x00, 0x6C, 0xCA, 0x88])
      )
    );
  }

  #[test]
  fn new_signed_short_test() {
    assert_eq!(
      DataElementValue::new_signed_short(&[10_000, -10_000]),
      DataElementValue::new_binary(
        ValueRepresentation::SignedShort,
        Rc::new(vec![0x10, 0x27, 0xF0, 0xD8])
      )
    );
  }

  #[test]
  fn new_signed_very_long_test() {
    assert_eq!(
      DataElementValue::new_signed_very_long(&[
        1_000_000_000_000_000_000,
        -1_000_000_000_000_000_000
      ]),
      DataElementValue::new_binary(
        ValueRepresentation::SignedVeryLong,
        Rc::new(vec![
          0x00, 0x00, 0x64, 0xA7, 0xB3, 0xB6, 0xE0, 0x0D, 0x00, 0x00, 0x9C,
          0x58, 0x4C, 0x49, 0x1F, 0xF2,
        ])
      )
    );
  }

  #[test]
  fn new_time_test() {
    assert_eq!(
      DataElementValue::new_time(&time::StructuredTime {
        hour: 22,
        minute: Some(45),
        second: Some(14.0)
      }),
      DataElementValue::new_binary(
        ValueRepresentation::Time,
        Rc::new(b"224514".to_vec()),
      )
    );
  }

  #[test]
  fn new_unique_identifier_test() {
    assert_eq!(
      DataElementValue::new_unique_identifier(&["1.2", "3.4"]),
      DataElementValue::new_binary(
        ValueRepresentation::UniqueIdentifier,
        Rc::new(b"1.2\\3.4\0".to_vec()),
      )
    );
  }

  #[test]
  fn new_universal_resource_identifier_test() {
    assert_eq!(
      DataElementValue::new_universal_resource_identifier("http;//test.com  "),
      DataElementValue::new_binary(
        ValueRepresentation::UniversalResourceIdentifier,
        Rc::new(b"http;//test.com ".to_vec()),
      )
    );
  }

  #[test]
  fn new_unknown_test() {
    assert_eq!(
      DataElementValue::new_unknown(vec![1, 2]),
      DataElementValue::new_binary(
        ValueRepresentation::Unknown,
        Rc::new(vec![1, 2]),
      )
    );
  }

  #[test]
  fn new_unlimited_characters_test() {
    assert_eq!(
      DataElementValue::new_unlimited_characters(&[" ABCD "]),
      DataElementValue::new_binary(
        ValueRepresentation::UnlimitedCharacters,
        Rc::new(b" ABCD ".to_vec()),
      )
    );
  }

  #[test]
  fn new_unlimited_text_test() {
    assert_eq!(
      DataElementValue::new_unlimited_text(" ABC "),
      DataElementValue::new_binary(
        ValueRepresentation::UnlimitedText,
        Rc::new(b" ABC".to_vec()),
      )
    );
  }

  #[test]
  fn new_unsigned_long_test() {
    assert_eq!(
      DataElementValue::new_unsigned_long(&[4_000_000_000]),
      DataElementValue::new_binary(
        ValueRepresentation::UnsignedLong,
        Rc::new(vec![0x00, 0x28, 0x6B, 0xEE])
      )
    );
  }

  #[test]
  fn new_unsigned_short_test() {
    assert_eq!(
      DataElementValue::new_unsigned_short(&[50_000]),
      DataElementValue::new_binary(
        ValueRepresentation::UnsignedShort,
        Rc::new(vec![80, 195])
      )
    );
  }

  #[test]
  fn new_unsigned_very_long_test() {
    assert_eq!(
      DataElementValue::new_unsigned_very_long(&[10_000_000_000_000_000_000]),
      DataElementValue::new_binary(
        ValueRepresentation::UnsignedVeryLong,
        Rc::new(vec![0x00, 0x00, 0xE8, 0x89, 0x04, 0x23, 0xC7, 0x8A])
      )
    );
  }
}
