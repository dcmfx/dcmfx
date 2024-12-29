import dcmfx_core/value_representation.{LengthRequirements}
import gleam/list
import gleam/option.{None, Some}
import gleeunit/should

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

    value_representation.from_bytes(<<s:utf8>>)
    |> should.equal(Ok(vr))
  })

  value_representation.from_bytes(<<"XY">>)
  |> should.equal(Error(Nil))
}

pub fn to_string_test() {
  all_vrs
  |> list.each(fn(x) {
    let #(vr, s, _) = x

    value_representation.to_string(vr)
    |> should.equal(s)
  })
}

pub fn name_test() {
  all_vrs
  |> list.each(fn(x) {
    let #(vr, _, name) = x

    value_representation.name(vr)
    |> should.equal(name)
  })
}

pub fn is_string_test() {
  all_vrs
  |> list.each(fn(x) {
    let #(vr, _, _) = x

    value_representation.is_string(vr)
    |> should.equal(
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
      || vr == value_representation.UnlimitedText,
    )
  })
}

pub fn is_encoded_string_test() {
  all_vrs
  |> list.each(fn(x) {
    let #(vr, _, _) = x

    value_representation.is_encoded_string(vr)
    |> should.equal(
      vr == value_representation.LongString
      || vr == value_representation.LongText
      || vr == value_representation.PersonName
      || vr == value_representation.ShortString
      || vr == value_representation.ShortText
      || vr == value_representation.UnlimitedCharacters
      || vr == value_representation.UnlimitedText,
    )
  })
}

pub fn pad_bytes_to_even_length_test() {
  value_representation.LongText
  |> value_representation.pad_bytes_to_even_length(<<>>)
  |> should.equal(<<>>)

  value_representation.LongText
  |> value_representation.pad_bytes_to_even_length(<<0x41>>)
  |> should.equal(<<0x41, 0x20>>)

  value_representation.UniqueIdentifier
  |> value_representation.pad_bytes_to_even_length(<<0x41>>)
  |> should.equal(<<0x41, 0x00>>)

  value_representation.LongText
  |> value_representation.pad_bytes_to_even_length(<<0x41, 0x42>>)
  |> should.equal(<<0x41, 0x42>>)
}

pub fn length_requirements_test() {
  value_representation.length_requirements(value_representation.AgeString)
  |> should.equal(LengthRequirements(4, None, None))

  value_representation.length_requirements(value_representation.AttributeTag)
  |> should.equal(LengthRequirements(0xFFFC, Some(4), None))

  value_representation.length_requirements(value_representation.PersonName)
  |> should.equal(LengthRequirements(0xFFFE, None, Some(324)))

  value_representation.length_requirements(value_representation.Sequence)
  |> should.equal(LengthRequirements(0, None, None))
}

pub fn swap_endianness_test() {
  value_representation.SignedShort
  |> value_representation.swap_endianness(<<0, 1, 2, 3>>)
  |> should.equal(<<1, 0, 3, 2>>)

  value_representation.SignedLong
  |> value_representation.swap_endianness(<<0, 1, 2, 3, 4, 5, 6, 7>>)
  |> should.equal(<<3, 2, 1, 0, 7, 6, 5, 4>>)

  value_representation.SignedVeryLong
  |> value_representation.swap_endianness(<<
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
  >>)
  |> should.equal(<<7, 6, 5, 4, 3, 2, 1, 0, 15, 14, 13, 12, 11, 10, 9, 8>>)

  value_representation.OtherByteString
  |> value_representation.swap_endianness(<<0, 1, 2, 3>>)
  |> should.equal(<<0, 1, 2, 3>>)
}
