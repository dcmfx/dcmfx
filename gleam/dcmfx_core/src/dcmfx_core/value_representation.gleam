//// DICOM value representations (VRs).
////
//// See [section 6.2](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_6.2.html)
//// of the DICOM specification for VR definitions.

import dcmfx_core/internal/bit_array_utils
import dcmfx_core/internal/endian
import gleam/option.{type Option, None, Some}

/// All DICOM value representations (VRs).
///
pub type ValueRepresentation {
  AgeString
  ApplicationEntity
  AttributeTag
  CodeString
  Date
  DateTime
  DecimalString
  FloatingPointDouble
  FloatingPointSingle
  IntegerString
  LongString
  LongText
  OtherByteString
  OtherDoubleString
  OtherFloatString
  OtherLongString
  OtherVeryLongString
  OtherWordString
  PersonName
  Sequence
  ShortString
  ShortText
  SignedLong
  SignedShort
  SignedVeryLong
  Time
  UniqueIdentifier
  UniversalResourceIdentifier
  Unknown
  UnlimitedCharacters
  UnlimitedText
  UnsignedLong
  UnsignedShort
  UnsignedVeryLong
}

/// Converts a value representation into its equivalent two-character string.
///
pub fn to_string(vr: ValueRepresentation) -> String {
  case vr {
    AgeString -> "AS"
    ApplicationEntity -> "AE"
    AttributeTag -> "AT"
    CodeString -> "CS"
    Date -> "DA"
    DateTime -> "DT"
    DecimalString -> "DS"
    FloatingPointDouble -> "FD"
    FloatingPointSingle -> "FL"
    IntegerString -> "IS"
    LongString -> "LO"
    LongText -> "LT"
    OtherByteString -> "OB"
    OtherDoubleString -> "OD"
    OtherFloatString -> "OF"
    OtherLongString -> "OL"
    OtherVeryLongString -> "OV"
    OtherWordString -> "OW"
    PersonName -> "PN"
    Sequence -> "SQ"
    ShortString -> "SH"
    ShortText -> "ST"
    SignedLong -> "SL"
    SignedShort -> "SS"
    SignedVeryLong -> "SV"
    Time -> "TM"
    UniqueIdentifier -> "UI"
    UniversalResourceIdentifier -> "UR"
    Unknown -> "UN"
    UnlimitedCharacters -> "UC"
    UnlimitedText -> "UT"
    UnsignedLong -> "UL"
    UnsignedShort -> "US"
    UnsignedVeryLong -> "UV"
  }
}

/// Converts a two-character string, e.g. "DA", into a value representation.
///
pub fn from_bytes(bytes: BitArray) -> Result(ValueRepresentation, Nil) {
  case bytes {
    <<0x41, 0x45>> -> Ok(ApplicationEntity)
    <<0x41, 0x53>> -> Ok(AgeString)
    <<0x41, 0x54>> -> Ok(AttributeTag)
    <<0x43, 0x53>> -> Ok(CodeString)
    <<0x44, 0x41>> -> Ok(Date)
    <<0x44, 0x53>> -> Ok(DecimalString)
    <<0x44, 0x54>> -> Ok(DateTime)
    <<0x46, 0x44>> -> Ok(FloatingPointDouble)
    <<0x46, 0x4C>> -> Ok(FloatingPointSingle)
    <<0x49, 0x53>> -> Ok(IntegerString)
    <<0x4C, 0x4F>> -> Ok(LongString)
    <<0x4C, 0x54>> -> Ok(LongText)
    <<0x4F, 0x42>> -> Ok(OtherByteString)
    <<0x4F, 0x44>> -> Ok(OtherDoubleString)
    <<0x4F, 0x46>> -> Ok(OtherFloatString)
    <<0x4F, 0x4C>> -> Ok(OtherLongString)
    <<0x4F, 0x56>> -> Ok(OtherVeryLongString)
    <<0x4F, 0x57>> -> Ok(OtherWordString)
    <<0x50, 0x4E>> -> Ok(PersonName)
    <<0x53, 0x48>> -> Ok(ShortString)
    <<0x53, 0x4C>> -> Ok(SignedLong)
    <<0x53, 0x51>> -> Ok(Sequence)
    <<0x53, 0x53>> -> Ok(SignedShort)
    <<0x53, 0x54>> -> Ok(ShortText)
    <<0x53, 0x56>> -> Ok(SignedVeryLong)
    <<0x54, 0x4D>> -> Ok(Time)
    <<0x55, 0x43>> -> Ok(UnlimitedCharacters)
    <<0x55, 0x49>> -> Ok(UniqueIdentifier)
    <<0x55, 0x4C>> -> Ok(UnsignedLong)
    <<0x55, 0x4E>> -> Ok(Unknown)
    <<0x55, 0x52>> -> Ok(UniversalResourceIdentifier)
    <<0x55, 0x53>> -> Ok(UnsignedShort)
    <<0x55, 0x54>> -> Ok(UnlimitedText)
    <<0x55, 0x56>> -> Ok(UnsignedVeryLong)

    _ -> Error(Nil)
  }
}

/// Returns the human-readable name of a value representation, e.g.
/// `CodeString`, `AttributeTag`.
///
pub fn name(vr: ValueRepresentation) -> String {
  case vr {
    AgeString -> "AgeString"
    ApplicationEntity -> "ApplicationEntity"
    AttributeTag -> "AttributeTag"
    CodeString -> "CodeString"
    Date -> "Date"
    DateTime -> "DateTime"
    DecimalString -> "DecimalString"
    FloatingPointDouble -> "FloatingPointDouble"
    FloatingPointSingle -> "FloatingPointSingle"
    IntegerString -> "IntegerString"
    LongString -> "LongString"
    LongText -> "LongText"
    OtherByteString -> "OtherByteString"
    OtherDoubleString -> "OtherDoubleString"
    OtherFloatString -> "OtherFloatString"
    OtherLongString -> "OtherLongString"
    OtherVeryLongString -> "OtherVeryLongString"
    OtherWordString -> "OtherWordString"
    PersonName -> "PersonName"
    Sequence -> "Sequence"
    ShortString -> "ShortString"
    ShortText -> "ShortText"
    SignedLong -> "SignedLong"
    SignedShort -> "SignedShort"
    SignedVeryLong -> "SignedVeryLong"
    Time -> "Time"
    UniqueIdentifier -> "UniqueIdentifier"
    UniversalResourceIdentifier -> "UniversalResourceIdentifier"
    Unknown -> "Unknown"
    UnlimitedCharacters -> "UnlimitedCharacters"
    UnlimitedText -> "UnlimitedText"
    UnsignedLong -> "UnsignedLong"
    UnsignedShort -> "UnsignedShort"
    UnsignedVeryLong -> "UnsignedVeryLong"
  }
}

/// Returns whether a value representation stores string data.
///
pub fn is_string(vr: ValueRepresentation) -> Bool {
  vr == AgeString
  || vr == ApplicationEntity
  || vr == CodeString
  || vr == Date
  || vr == DateTime
  || vr == DecimalString
  || vr == IntegerString
  || vr == LongString
  || vr == LongText
  || vr == PersonName
  || vr == ShortString
  || vr == ShortText
  || vr == Time
  || vr == UniqueIdentifier
  || vr == UniversalResourceIdentifier
  || vr == UnlimitedCharacters
  || vr == UnlimitedText
}

/// Returns whether a value representation stores string data that is UTF-8
/// encoded and can therefore store any Unicode codepoint.
///
pub fn is_encoded_string(vr: ValueRepresentation) -> Bool {
  vr == LongString
  || vr == LongText
  || vr == PersonName
  || vr == ShortString
  || vr == ShortText
  || vr == UnlimitedCharacters
  || vr == UnlimitedText
}

/// Appends the correct padding byte for the given value representation if the
/// bytes are not of even length.
///
pub fn pad_bytes_to_even_length(
  vr: ValueRepresentation,
  bytes: BitArray,
) -> BitArray {
  case vr {
    // UI uses a zero byte as padding
    UniqueIdentifier -> bit_array_utils.pad_to_even_length(bytes, 0)

    // String values use a space as padding. The rest do not use any padding.
    _ ->
      case is_string(vr) {
        True -> bit_array_utils.pad_to_even_length(bytes, 0x20)
        False -> bytes
      }
  }
}

/// The restrictions that apply to the length of a value representation's data.
/// These restrictions are defined by the DICOM specification, and are only
/// enforced when creating new values.
///
/// The restrictions are:
///
/// 1. The maximum number of bytes a value can have.
///
/// 2. Optionally, a number that the number of bytes must be an exact multiple
///    of.
///
/// 3. Optionally, for string-valued VRs, a limit on the number of characters
///    (not bytes) in the string. In multi-valued string VRs this limit applies
///    to each value individually.
///
pub type LengthRequirements {
  LengthRequirements(
    bytes_max: Int,
    bytes_multiple_of: Option(Int),
    string_characters_max: Option(Int),
  )
}

/// Returns the length requirements for a value representation. See the
/// `LengthRequirements` type for details.
///
pub fn length_requirements(vr: ValueRepresentation) -> LengthRequirements {
  case vr {
    AgeString -> LengthRequirements(4, None, None)
    ApplicationEntity -> LengthRequirements(16, None, None)
    AttributeTag -> LengthRequirements(0xFFFC, Some(4), None)
    CodeString -> LengthRequirements(0xFFFE, None, Some(16))
    Date -> LengthRequirements(8, None, None)
    DateTime -> LengthRequirements(26, None, None)
    DecimalString -> LengthRequirements(0xFFFE, None, Some(16))
    FloatingPointDouble -> LengthRequirements(0xFFF8, Some(8), None)
    FloatingPointSingle -> LengthRequirements(0xFFFC, Some(4), None)
    IntegerString -> LengthRequirements(0xFFFE, None, Some(12))
    LongString -> LengthRequirements(0xFFFE, None, Some(64))
    LongText -> LengthRequirements(0xFFFE, None, Some(10_240))
    OtherByteString -> LengthRequirements(0xFFFFFFFE, Some(2), None)
    OtherDoubleString -> LengthRequirements(0xFFFFFFF8, Some(8), None)
    OtherFloatString -> LengthRequirements(0xFFFFFFFC, Some(4), None)
    OtherLongString -> LengthRequirements(0xFFFFFFFC, Some(4), None)
    OtherVeryLongString -> LengthRequirements(0xFFFFFFF8, Some(8), None)
    OtherWordString -> LengthRequirements(0xFFFFFFFE, Some(2), None)
    PersonName -> LengthRequirements(0xFFFE, None, Some(324))
    Sequence -> LengthRequirements(0, None, None)
    ShortString -> LengthRequirements(0xFFFE, None, Some(16))
    ShortText -> LengthRequirements(0xFFFE, None, Some(1024))
    SignedLong -> LengthRequirements(0xFFFC, Some(4), None)
    SignedShort -> LengthRequirements(0xFFFE, Some(2), None)
    SignedVeryLong -> LengthRequirements(0xFFFFFFF8, Some(8), None)
    Time -> LengthRequirements(14, None, None)
    UniqueIdentifier -> LengthRequirements(0xFFFE, None, Some(64))
    UniversalResourceIdentifier -> LengthRequirements(0xFFFFFFFE, None, None)
    Unknown -> LengthRequirements(0xFFFFFFFE, None, None)
    UnlimitedCharacters -> LengthRequirements(0xFFFFFFFE, None, None)
    UnlimitedText -> LengthRequirements(0xFFFFFFFE, None, None)
    UnsignedLong -> LengthRequirements(0xFFFC, Some(4), None)
    UnsignedShort -> LengthRequirements(0xFFFE, Some(2), None)
    UnsignedVeryLong -> LengthRequirements(0xFFF8, Some(8), None)
  }
}

/// Swaps the endianness of data for a value representation.
///
pub fn swap_endianness(vr: ValueRepresentation, bytes: BitArray) -> BitArray {
  case vr {
    AttributeTag | OtherWordString | SignedShort | UnsignedShort ->
      endian.swap_16_bit(bytes, [])

    FloatingPointSingle
    | OtherFloatString
    | OtherLongString
    | SignedLong
    | UnsignedLong -> endian.swap_32_bit(bytes, [])

    FloatingPointDouble
    | OtherDoubleString
    | OtherVeryLongString
    | SignedVeryLong
    | UnsignedVeryLong -> endian.swap_64_bit(bytes, [])

    _ -> bytes
  }
}
