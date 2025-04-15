import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/dictionary
import dcmfx_core/value_representation.{type ValueRepresentation}
import dcmfx_p10/internal/value_length.{type ValueLength}
import gleam/option.{type Option, None, Some}

/// Describes the header for a single DICOM data element, specifically its tag,
/// VR, and length in bytes. The VR is optional because some data elements, e.g.
/// sequence delimiters and sequence item delimiters, don't have a VR.
///
pub type DataElementHeader {
  DataElementHeader(
    tag: DataElementTag,
    vr: Option(ValueRepresentation),
    length: ValueLength,
  )
}

/// Converts a data element header to a human-readable string in the format
/// "(GROUP,ELEMENT) VR NAME", e.g. `"(0008,0020) DA StudyDate"`.
///
pub fn to_string(header: DataElementHeader) -> String {
  let vr = case header.vr {
    Some(vr) -> value_representation.to_string(vr)
    _ -> "  "
  }

  data_element_tag.to_string(header.tag)
  <> " "
  <> vr
  <> " "
  <> dictionary.tag_name(header.tag, None)
}

/// The two possibilities for the size of the value length for a VR stored in
/// the DICOM P10 format.
///
pub type ValueLengthSize {
  ValueLengthU16
  ValueLengthU32
}

/// Returns the maximum supported value length in bytes.
///
pub fn value_length_size_max_length(size: ValueLengthSize) -> Int {
  case size {
    ValueLengthU16 -> 0xFFFF
    ValueLengthU32 -> 0xFFFFFFFE
  }
}

/// Returns the size of the value length for a VR stored in the DICOM P10
/// format.
///
pub fn value_length_size(vr: ValueRepresentation) -> ValueLengthSize {
  case vr {
    value_representation.AgeString
    | value_representation.ApplicationEntity
    | value_representation.AttributeTag
    | value_representation.CodeString
    | value_representation.Date
    | value_representation.DateTime
    | value_representation.DecimalString
    | value_representation.FloatingPointDouble
    | value_representation.FloatingPointSingle
    | value_representation.IntegerString
    | value_representation.LongString
    | value_representation.LongText
    | value_representation.PersonName
    | value_representation.ShortString
    | value_representation.ShortText
    | value_representation.SignedLong
    | value_representation.SignedShort
    | value_representation.Time
    | value_representation.UniqueIdentifier
    | value_representation.UnsignedLong
    | value_representation.UnsignedShort -> ValueLengthU16

    value_representation.OtherByteString
    | value_representation.OtherDoubleString
    | value_representation.OtherFloatString
    | value_representation.OtherLongString
    | value_representation.OtherVeryLongString
    | value_representation.OtherWordString
    | value_representation.Sequence
    | value_representation.SignedVeryLong
    | value_representation.UniversalResourceIdentifier
    | value_representation.Unknown
    | value_representation.UnlimitedCharacters
    | value_representation.UnlimitedText
    | value_representation.UnsignedVeryLong -> ValueLengthU32
  }
}
