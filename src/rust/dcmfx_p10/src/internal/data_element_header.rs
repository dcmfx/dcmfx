use dcmfx_core::{registry, DataElementTag, ValueRepresentation};

/// Describes the header for a single DICOM data element, specifically its tag,
/// VR, and length in bytes. The VR is optional because some data elements, e.g.
/// sequence delimiters and sequence item delimiters, don't have a VR.
///
pub struct DataElementHeader {
  pub tag: DataElementTag,
  pub vr: Option<ValueRepresentation>,
  pub length: u32,
}

impl std::fmt::Display for DataElementHeader {
  /// Converts a data element header to a human-readable string in the format
  /// "(GROUP,ELEMENT) VR NAME", e.g. `"(0008,0020) DA StudyDate"`.
  ///
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    let tag_name = registry::tag_name(self.tag, None);

    match self.vr {
      Some(vr) => write!(f, "{} {} {}", self.tag, vr, tag_name),
      _ => write!(f, "{} {}", self.tag, tag_name),
    }
  }
}

/// The two possibilities for the size of the value length for a VR stored in
/// the DICOM P10 format.
///
pub enum ValueLengthSize {
  U16,
  U32,
}

impl DataElementHeader {
  /// Returns the size of the value length for a VR stored in the DICOM P10
  /// format.
  ///
  pub fn value_length_size(vr: ValueRepresentation) -> ValueLengthSize {
    match vr {
      ValueRepresentation::AgeString
      | ValueRepresentation::ApplicationEntity
      | ValueRepresentation::AttributeTag
      | ValueRepresentation::CodeString
      | ValueRepresentation::Date
      | ValueRepresentation::DateTime
      | ValueRepresentation::DecimalString
      | ValueRepresentation::FloatingPointDouble
      | ValueRepresentation::FloatingPointSingle
      | ValueRepresentation::IntegerString
      | ValueRepresentation::LongString
      | ValueRepresentation::LongText
      | ValueRepresentation::PersonName
      | ValueRepresentation::ShortString
      | ValueRepresentation::ShortText
      | ValueRepresentation::SignedLong
      | ValueRepresentation::SignedShort
      | ValueRepresentation::Time
      | ValueRepresentation::UniqueIdentifier
      | ValueRepresentation::UnsignedLong
      | ValueRepresentation::UnsignedShort => ValueLengthSize::U16,

      ValueRepresentation::OtherByteString
      | ValueRepresentation::OtherDoubleString
      | ValueRepresentation::OtherFloatString
      | ValueRepresentation::OtherLongString
      | ValueRepresentation::OtherVeryLongString
      | ValueRepresentation::OtherWordString
      | ValueRepresentation::Sequence
      | ValueRepresentation::SignedVeryLong
      | ValueRepresentation::UniversalResourceIdentifier
      | ValueRepresentation::Unknown
      | ValueRepresentation::UnlimitedCharacters
      | ValueRepresentation::UnlimitedText
      | ValueRepresentation::UnsignedVeryLong => ValueLengthSize::U32,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_string_test() {
    assert_eq!(
      DataElementHeader {
        tag: registry::PATIENT_AGE.tag,
        vr: Some(ValueRepresentation::AgeString),
        length: 0
      }
      .to_string(),
      "(0010,1010) AS Patient's Age".to_string()
    );

    assert_eq!(
      DataElementHeader {
        tag: registry::ITEM.tag,
        vr: None,
        length: 0
      }
      .to_string(),
      "(FFFE,E000) Item".to_string()
    );
  }
}
