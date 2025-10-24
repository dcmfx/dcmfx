import dcmfx_core/value_representation.{LengthRequirements}
import gleam/list
import gleam/option.{None, Some}

const all_vrs = [
  #(value_representation.AgeString, "AS", "AgeString"),
  #(value_representation.ApplicationEntity, "AE", "ApplicationEntity"),
  #(value_representation.AttributeTag, "AT", "AttributeTag"),
  #(value_representation.CodeString, "CS", "CodeString"),
  #(value_representation.Date, "DA", "Date"),
  #(value_representation.DateTime, "DT", "DateTime"),
  #(value_representation.DecimalString, "DS", "DecimalString"),
  #(value_representation.FloatingPointDouble, "FD", "FloatingPointDouble"),
  #(value_representation.FloatingPointSingle, "FL", "FloatingPointSingle"),
  #(value_representation.IntegerString, "IS", "IntegerString"),
  #(value_representation.LongString, "LO", "LongString"),
  #(value_representation.LongText, "LT", "LongText"),
  #(value_representation.OtherByteString, "OB", "OtherByteString"),
  #(value_representation.OtherDoubleString, "OD", "OtherDoubleString"),
  #(value_representation.OtherFloatString, "OF", "OtherFloatString"),
  #(value_representation.OtherLongString, "OL", "OtherLongString"),
  #(value_representation.OtherVeryLongString, "OV", "OtherVeryLongString"),
  #(value_representation.OtherWordString, "OW", "OtherWordString"),
  #(value_representation.PersonName, "PN", "PersonName"),
  #(value_representation.Sequence, "SQ", "Sequence"),
  #(value_representation.ShortString, "SH", "ShortString"),
  #(value_representation.ShortText, "ST", "ShortText"),
  #(value_representation.SignedLong, "SL", "SignedLong"),
  #(value_representation.SignedShort, "SS", "SignedShort"),
  #(value_representation.SignedVeryLong, "SV", "SignedVeryLong"),
  #(value_representation.Time, "TM", "Time"),
  #(value_representation.UniqueIdentifier, "UI", "UniqueIdentifier"),
  #(
    value_representation.UniversalResourceIdentifier,
    "UR",
    "UniversalResourceIdentifier",
  ),
  #(value_representation.Unknown, "UN", "Unknown"),
  #(value_representation.UnlimitedCharacters, "UC", "UnlimitedCharacters"),
  #(value_representation.UnlimitedText, "UT", "UnlimitedText"),
  #(value_representation.UnsignedLong, "UL", "UnsignedLong"),
  #(value_representation.UnsignedShort, "US", "UnsignedShort"),
  #(value_representation.UnsignedVeryLong, "UV", "UnsignedVeryLong"),
]

pub fn from_bytes_test() {
  all_vrs
  |> list.each(fn(x) {
    let #(vr, s, _) = x

    assert value_representation.from_bytes(<<s:utf8>>) == Ok(vr)
  })

  assert value_representation.from_bytes(<<"XY">>) == Error(Nil)
}

pub fn to_string_test() {
  all_vrs
  |> list.each(fn(x) {
    let #(vr, s, _) = x

    assert value_representation.to_string(vr) == s
  })
}

pub fn name_test() {
  all_vrs
  |> list.each(fn(x) {
    let #(vr, _, name) = x

    assert value_representation.name(vr) == name
  })
}

pub fn is_string_test() {
  all_vrs
  |> list.each(fn(x) {
    let #(vr, _, _) = x

    assert value_representation.is_string(vr)
      == {
        vr == value_representation.AgeString
        || vr == value_representation.ApplicationEntity
        || vr == value_representation.CodeString
        || vr == value_representation.Date
        || vr == value_representation.DateTime
        || vr == value_representation.DecimalString
        || vr == value_representation.IntegerString
        || vr == value_representation.LongString
        || vr == value_representation.LongText
        || vr == value_representation.PersonName
        || vr == value_representation.ShortString
        || vr == value_representation.ShortText
        || vr == value_representation.Time
        || vr == value_representation.UniqueIdentifier
        || vr == value_representation.UniversalResourceIdentifier
        || vr == value_representation.UnlimitedCharacters
        || vr == value_representation.UnlimitedText
      }
  })
}

pub fn is_encoded_string_test() {
  all_vrs
  |> list.each(fn(x) {
    let #(vr, _, _) = x

    assert value_representation.is_encoded_string(vr)
      == {
        vr == value_representation.LongString
        || vr == value_representation.LongText
        || vr == value_representation.PersonName
        || vr == value_representation.ShortString
        || vr == value_representation.ShortText
        || vr == value_representation.UnlimitedCharacters
        || vr == value_representation.UnlimitedText
      }
  })
}

pub fn pad_bytes_to_even_length_test() {
  assert value_representation.pad_bytes_to_even_length(
      value_representation.LongText,
      <<>>,
    )
    == <<>>

  assert value_representation.pad_bytes_to_even_length(
      value_representation.LongText,
      <<0x41>>,
    )
    == <<0x41, 0x20>>

  assert value_representation.pad_bytes_to_even_length(
      value_representation.UniqueIdentifier,
      <<0x41>>,
    )
    == <<0x41, 0x00>>

  assert value_representation.pad_bytes_to_even_length(
      value_representation.LongText,
      <<0x41, 0x42>>,
    )
    == <<0x41, 0x42>>
}

pub fn length_requirements_test() {
  assert value_representation.length_requirements(
      value_representation.AgeString,
    )
    == LengthRequirements(4, None, None)

  assert value_representation.length_requirements(
      value_representation.AttributeTag,
    )
    == LengthRequirements(0xFFFC, Some(4), None)

  assert value_representation.length_requirements(
      value_representation.PersonName,
    )
    == LengthRequirements(0xFFFE, None, Some(324))

  assert value_representation.length_requirements(value_representation.Sequence)
    == LengthRequirements(0, None, None)
}

pub fn swap_endianness_test() {
  assert value_representation.swap_endianness(value_representation.SignedShort, <<
      0,
      1,
      2,
      3,
    >>)
    == <<1, 0, 3, 2>>

  assert value_representation.swap_endianness(value_representation.SignedLong, <<
      0,
      1,
      2,
      3,
      4,
      5,
      6,
      7,
    >>)
    == <<3, 2, 1, 0, 7, 6, 5, 4>>

  assert value_representation.swap_endianness(
      value_representation.SignedVeryLong,
      <<0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15>>,
    )
    == <<7, 6, 5, 4, 3, 2, 1, 0, 15, 14, 13, 12, 11, 10, 9, 8>>

  assert value_representation.swap_endianness(
      value_representation.OtherByteString,
      <<0, 1, 2, 3>>,
    )
    == <<0, 1, 2, 3>>
}
