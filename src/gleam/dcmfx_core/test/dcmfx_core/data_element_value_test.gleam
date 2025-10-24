import bigi
import dcmfx_core/data_element_tag.{DataElementTag}
import dcmfx_core/data_element_value
import dcmfx_core/data_element_value/age_string
import dcmfx_core/data_element_value/date
import dcmfx_core/data_element_value/date_time
import dcmfx_core/data_element_value/person_name
import dcmfx_core/data_element_value/time
import dcmfx_core/data_error
import dcmfx_core/dictionary
import dcmfx_core/value_representation
import gleam/bit_array
import gleam/dict
import gleam/list
import gleam/option.{None, Some}
import gleam/result
import gleam/string
import ieee_float

pub fn value_representation_test() {
  assert ["123"]
    |> data_element_value.new_long_string
    |> result.map(data_element_value.value_representation)
    == Ok(value_representation.LongString)

  assert [ieee_float.finite(1.0)]
    |> data_element_value.new_floating_point_single
    |> result.map(data_element_value.value_representation)
    == Ok(value_representation.FloatingPointSingle)

  assert value_representation.UnsignedShort
    |> data_element_value.new_lookup_table_descriptor_unchecked(<<>>)
    |> data_element_value.value_representation
    == value_representation.UnsignedShort

  assert value_representation.OtherWordString
    |> data_element_value.new_encapsulated_pixel_data_unchecked([])
    |> data_element_value.value_representation
    == value_representation.OtherWordString

  assert []
    |> data_element_value.new_sequence
    |> data_element_value.value_representation
    == value_representation.Sequence
}

pub fn bytes_test() {
  assert result.try(
      data_element_value.new_long_string(["12"]),
      data_element_value.bytes,
    )
    == Ok(<<"12">>)

  assert result.try(
      data_element_value.new_floating_point_single([ieee_float.finite(1.0)]),
      data_element_value.bytes,
    )
    == Ok(<<0x00, 0x00, 0x80, 0x3F>>)

  assert value_representation.UnsignedShort
    |> data_element_value.new_lookup_table_descriptor_unchecked(<<
      0, 1, 2, 3, 4, 5,
    >>)
    |> data_element_value.bytes
    == Ok(<<0, 1, 2, 3, 4, 5>>)

  let assert Error(_) =
    value_representation.OtherWordString
    |> data_element_value.new_encapsulated_pixel_data_unchecked([])
    |> data_element_value.bytes

  let assert Error(_) =
    data_element_value.bytes(data_element_value.new_sequence([]))
}

pub fn get_string_test() {
  assert "A"
    |> data_element_value.new_application_entity
    |> result.try(data_element_value.get_string)
    == Ok("A")

  assert ["AA \u{0}"]
    |> data_element_value.new_code_string
    |> result.try(data_element_value.get_string)
    == Ok("AA")

  assert "A"
    |> data_element_value.new_long_text
    |> result.try(data_element_value.get_string)
    == Ok("A")

  assert "A"
    |> data_element_value.new_short_text
    |> result.try(data_element_value.get_string)
    == Ok("A")

  assert "A"
    |> data_element_value.new_universal_resource_identifier
    |> result.try(data_element_value.get_string)
    == Ok("A")

  assert "A"
    |> data_element_value.new_unlimited_text
    |> result.try(data_element_value.get_string)
    == Ok("A")

  assert data_element_value.get_string(
      data_element_value.new_binary_unchecked(value_representation.ShortText, <<
        0xD0,
      >>),
    )
    == Error(data_error.new_value_invalid("String bytes are not valid UTF-8"))

  assert ["A"]
    |> data_element_value.new_long_string
    |> result.try(data_element_value.get_string)
    == Ok("A")

  assert ["A", "B"]
    |> data_element_value.new_long_string
    |> result.try(data_element_value.get_string)
    == Error(data_error.new_multiplicity_mismatch())

  assert [1]
    |> data_element_value.new_unsigned_short
    |> result.try(data_element_value.get_string)
    == Error(data_error.new_value_not_present())
}

pub fn get_strings_test() {
  assert ["A", "B"]
    |> data_element_value.new_code_string
    |> result.try(data_element_value.get_strings)
    == Ok(["A", "B"])

  assert ["1.2", "3.4"]
    |> data_element_value.new_unique_identifier
    |> result.try(data_element_value.get_strings)
    == Ok(["1.2", "3.4"])

  assert ["A", "B"]
    |> data_element_value.new_long_string
    |> result.try(data_element_value.get_strings)
    == Ok(["A", "B"])

  assert ["A", "B"]
    |> data_element_value.new_short_string
    |> result.try(data_element_value.get_strings)
    == Ok(["A", "B"])

  assert ["A", "B"]
    |> data_element_value.new_unlimited_characters
    |> result.try(data_element_value.get_strings)
    == Ok(["A", "B"])

  assert data_element_value.get_strings(
      data_element_value.new_binary_unchecked(value_representation.ShortString, <<
        0xD0,
      >>),
    )
    == Error(data_error.new_value_invalid("String bytes are not valid UTF-8"))

  assert "A"
    |> data_element_value.new_long_text
    |> result.try(data_element_value.get_strings)
    == Error(data_error.new_value_not_present())

  assert [1]
    |> data_element_value.new_unsigned_short
    |> result.try(data_element_value.get_strings)
    == Error(data_error.new_value_not_present())
}

pub fn get_int_test() {
  assert value_representation.IntegerString
    |> data_element_value.new_binary_unchecked(<<"  123   ">>)
    |> data_element_value.get_int
    == Ok(123)

  assert [1234]
    |> data_element_value.new_unsigned_long
    |> result.try(data_element_value.get_int)
    == Ok(1234)

  assert [123, 456]
    |> data_element_value.new_unsigned_long
    |> result.try(data_element_value.get_int)
    == Error(data_error.new_multiplicity_mismatch())

  assert "123"
    |> data_element_value.new_long_text
    |> result.try(data_element_value.get_int)
    == Error(data_error.new_value_not_present())
}

pub fn get_ints_test() {
  assert value_representation.IntegerString
    |> data_element_value.new_binary_unchecked(<<" 123 \\456">>)
    |> data_element_value.get_ints
    == Ok([123, 456])

  assert [-{ 0x80000000 }, 0x7FFFFFFF]
    |> data_element_value.new_signed_long
    |> result.try(data_element_value.get_ints)
    == Ok([-{ 0x80000000 }, 0x7FFFFFFF])

  assert value_representation.SignedLong
    |> data_element_value.new_binary_unchecked(<<0>>)
    |> data_element_value.get_ints
    == Error(data_error.new_value_invalid("Invalid Int32 data"))

  assert [-{ 0x8000 }, 0x7FFF]
    |> data_element_value.new_signed_short
    |> result.try(data_element_value.get_ints)
    == Ok([-{ 0x8000 }, 0x7FFF])

  assert value_representation.SignedShort
    |> data_element_value.new_binary_unchecked(<<0>>)
    |> data_element_value.get_ints
    == Error(data_error.new_value_invalid("Invalid Int16 data"))

  assert [0, 0xFFFFFFFF]
    |> data_element_value.new_unsigned_long
    |> result.try(data_element_value.get_ints)
    == Ok([0, 0xFFFFFFFF])

  assert value_representation.UnsignedLong
    |> data_element_value.new_binary_unchecked(<<0>>)
    |> data_element_value.get_ints
    == Error(data_error.new_value_invalid("Invalid Uint32 data"))

  assert [0, 0xFFFF]
    |> data_element_value.new_unsigned_short
    |> result.try(data_element_value.get_ints)
    == Ok([0, 0xFFFF])

  assert value_representation.UnsignedShort
    |> data_element_value.new_binary_unchecked(<<0>>)
    |> data_element_value.get_ints
    == Error(data_error.new_value_invalid("Invalid Uint16 data"))

  assert value_representation.SignedShort
    |> data_element_value.new_lookup_table_descriptor_unchecked(<<
      0x34, 0x12, 0x00, 0x80, 0x78, 0x56,
    >>)
    |> data_element_value.get_ints
    == Ok([0x1234, -{ 0x8000 }, 0x5678])

  assert value_representation.UnsignedShort
    |> data_element_value.new_lookup_table_descriptor_unchecked(<<
      0x34, 0x12, 0x00, 0x80, 0x78, 0x56,
    >>)
    |> data_element_value.get_ints
    == Ok([0x1234, 0x8000, 0x5678])

  assert value_representation.OtherWordString
    |> data_element_value.new_lookup_table_descriptor_unchecked(<<
      0, 0, 0, 0, 0, 0,
    >>)
    |> data_element_value.get_ints
    == Error(data_error.new_value_invalid("Invalid lookup table descriptor"))

  assert value_representation.UnsignedShort
    |> data_element_value.new_lookup_table_descriptor_unchecked(<<0, 0, 0, 0>>)
    |> data_element_value.get_ints
    == Error(data_error.new_value_invalid("Invalid lookup table descriptor"))

  assert [ieee_float.finite(123.0)]
    |> data_element_value.new_floating_point_single
    |> result.try(data_element_value.get_ints)
    == Error(data_error.new_value_not_present())

  assert "123"
    |> data_element_value.new_long_text
    |> result.try(data_element_value.get_ints)
    == Error(data_error.new_value_not_present())
}

pub fn get_big_int_test() {
  let assert Ok(i0) = bigi.from_string("-9223372036854775808")
  assert [i0]
    |> data_element_value.new_signed_very_long
    |> result.try(data_element_value.get_big_int)
    == Ok(i0)

  let assert Ok(i0) = bigi.from_string("9223372036854775807")
  assert [i0]
    |> data_element_value.new_unsigned_very_long
    |> result.try(data_element_value.get_big_int)
    == Ok(i0)

  let assert Ok(i0) = bigi.from_string("1234")
  let assert Ok(i1) = bigi.from_string("1234")
  assert [i0, i1]
    |> data_element_value.new_unsigned_very_long
    |> result.try(data_element_value.get_big_int)
    == Error(data_error.new_multiplicity_mismatch())

  assert "123"
    |> data_element_value.new_long_text
    |> result.try(data_element_value.get_big_int)
    == Error(data_error.new_value_not_present())
}

pub fn get_big_ints_test() {
  let assert Ok(i0) = bigi.from_string("-9223372036854775808")
  let assert Ok(i1) = bigi.from_string("9223372036854775807")
  assert [i0, i1]
    |> data_element_value.new_signed_very_long
    |> result.try(data_element_value.get_big_ints)
    == Ok([i0, i1])

  assert value_representation.SignedVeryLong
    |> data_element_value.new_binary_unchecked(<<0>>)
    |> data_element_value.get_big_ints
    == Error(data_error.new_value_invalid("Invalid Int64 data"))

  let assert Ok(i) = bigi.from_string("18446744073709551615")
  assert [bigi.zero(), i]
    |> data_element_value.new_unsigned_very_long
    |> result.try(data_element_value.get_big_ints)
    == Ok([bigi.zero(), i])

  assert value_representation.UnsignedVeryLong
    |> data_element_value.new_binary_unchecked(<<0>>)
    |> data_element_value.get_big_ints
    == Error(data_error.new_value_invalid("Invalid Uint64 data"))

  assert [ieee_float.finite(123.0)]
    |> data_element_value.new_floating_point_single
    |> result.try(data_element_value.get_big_ints)
    == Error(data_error.new_value_not_present())

  assert "123"
    |> data_element_value.new_long_text
    |> result.try(data_element_value.get_big_ints)
    == Error(data_error.new_value_not_present())
}

pub fn get_float_test() {
  assert value_representation.DecimalString
    |> data_element_value.new_binary_unchecked(<<" 1.2   ">>)
    |> data_element_value.get_float
    == Ok(ieee_float.finite(1.2))

  assert [ieee_float.finite(1.0)]
    |> data_element_value.new_floating_point_single
    |> result.try(data_element_value.get_float)
    == Ok(ieee_float.finite(1.0))

  assert [ieee_float.positive_infinity()]
    |> data_element_value.new_floating_point_single
    |> result.try(data_element_value.get_float)
    == Ok(ieee_float.positive_infinity())

  assert [ieee_float.finite(1.2), ieee_float.finite(3.4)]
    |> data_element_value.new_floating_point_double
    |> result.try(data_element_value.get_float)
    == Error(data_error.new_multiplicity_mismatch())

  assert "1.2"
    |> data_element_value.new_long_text
    |> result.try(data_element_value.get_float)
    == Error(data_error.new_value_not_present())
}

pub fn get_floats_test() {
  assert value_representation.DecimalString
    |> data_element_value.new_binary_unchecked(<<" 1.2  \\3.4">>)
    |> data_element_value.get_floats
    == Ok([ieee_float.finite(1.2), ieee_float.finite(3.4)])

  assert [ieee_float.finite(1.2), ieee_float.finite(3.4)]
    |> data_element_value.new_floating_point_double
    |> result.try(data_element_value.get_floats)
    == Ok([ieee_float.finite(1.2), ieee_float.finite(3.4)])

  assert [ieee_float.finite(1.0), ieee_float.finite(2.0)]
    |> data_element_value.new_other_double_string
    |> result.try(data_element_value.get_floats)
    == Ok([ieee_float.finite(1.0), ieee_float.finite(2.0)])

  assert [ieee_float.finite(1.0), ieee_float.finite(2.0)]
    |> data_element_value.new_other_double_string
    |> result.try(data_element_value.get_floats)
    == Ok([ieee_float.finite(1.0), ieee_float.finite(2.0)])

  assert value_representation.FloatingPointDouble
    |> data_element_value.new_binary_unchecked(<<0, 0, 0, 0>>)
    |> data_element_value.get_floats
    == Error(data_error.new_value_invalid("Invalid Float64 data"))

  assert [ieee_float.finite(1.0), ieee_float.finite(2.0)]
    |> data_element_value.new_floating_point_single
    |> result.try(data_element_value.get_floats)
    == Ok([ieee_float.finite(1.0), ieee_float.finite(2.0)])

  assert [ieee_float.finite(1.0), ieee_float.finite(2.0)]
    |> data_element_value.new_other_float_string
    |> result.try(data_element_value.get_floats)
    == Ok([ieee_float.finite(1.0), ieee_float.finite(2.0)])

  assert value_representation.FloatingPointSingle
    |> data_element_value.new_binary_unchecked(<<0, 0>>)
    |> data_element_value.get_floats
    == Error(data_error.new_value_invalid("Invalid Float32 data"))

  assert "1.2"
    |> data_element_value.new_long_text
    |> result.try(data_element_value.get_floats)
    == Error(data_error.new_value_not_present())
}

pub fn get_age_test() {
  assert value_representation.AgeString
    |> data_element_value.new_binary_unchecked(<<"001D">>)
    |> data_element_value.get_age
    == Ok(age_string.StructuredAge(1, age_string.Days))

  assert value_representation.Date
    |> data_element_value.new_binary_unchecked(<<>>)
    |> data_element_value.get_age
    == Error(data_error.new_value_not_present())
}

pub fn get_date_test() {
  assert value_representation.Date
    |> data_element_value.new_binary_unchecked(<<"20000101">>)
    |> data_element_value.get_date
    == Ok(date.StructuredDate(2000, 1, 1))

  assert value_representation.Time
    |> data_element_value.new_binary_unchecked(<<>>)
    |> data_element_value.get_date
    == Error(data_error.new_value_not_present())
}

pub fn get_date_time_test() {
  assert value_representation.DateTime
    |> data_element_value.new_binary_unchecked(<<"20000101123043.5">>)
    |> data_element_value.get_date_time
    == Ok(date_time.StructuredDateTime(
      2000,
      Some(1),
      Some(1),
      Some(12),
      Some(30),
      Some(43.5),
      None,
    ))

  assert value_representation.Date
    |> data_element_value.new_binary_unchecked(<<>>)
    |> data_element_value.get_date_time
    == Error(data_error.new_value_not_present())
}

pub fn get_time_test() {
  assert value_representation.Time
    |> data_element_value.new_binary_unchecked(<<"235921.2">>)
    |> data_element_value.get_time
    == Ok(time.StructuredTime(23, Some(59), Some(21.2)))

  assert value_representation.Date
    |> data_element_value.new_binary_unchecked(<<>>)
    |> data_element_value.get_time
    == Error(data_error.new_value_not_present())
}

pub fn get_person_name_test() {
  assert value_representation.PersonName
    |> data_element_value.new_binary_unchecked(<<"">>)
    |> data_element_value.get_person_name
    == Ok(person_name.StructuredPersonName(None, None, None))

  assert value_representation.PersonName
    |> data_element_value.new_binary_unchecked(<<"\\">>)
    |> data_element_value.get_person_name
    == Error(data_error.new_multiplicity_mismatch())
}

pub fn get_person_names_test() {
  assert value_representation.PersonName
    |> data_element_value.new_binary_unchecked(<<"\\">>)
    |> data_element_value.get_person_names
    == Ok([
      person_name.StructuredPersonName(None, None, None),
      person_name.StructuredPersonName(None, None, None),
    ])

  assert value_representation.Date
    |> data_element_value.new_binary_unchecked(<<>>)
    |> data_element_value.get_person_names
    == Error(data_error.new_value_not_present())
}

pub fn to_string_test() {
  let tag = DataElementTag(0, 0)

  assert ["DERIVED", "SECONDARY"]
    |> data_element_value.new_code_string
    |> result.map(data_element_value.to_string(_, tag, 80))
    == Ok("\"DERIVED\", \"SECONDARY\"")

  assert ["CT"]
    |> data_element_value.new_code_string
    |> result.map(data_element_value.to_string(_, dictionary.modality.tag, 80))
    == Ok("\"CT\" (Computed Tomography)")

  assert ["1.23"]
    |> data_element_value.new_unique_identifier
    |> result.map(data_element_value.to_string(_, tag, 80))
    == Ok("\"1.23\"")

  assert ["1.2.840.10008.1.2"]
    |> data_element_value.new_unique_identifier
    |> result.map(data_element_value.to_string(_, tag, 80))
    == Ok("\"1.2.840.10008.1.2\" (Implicit VR Little Endian)")

  assert value_representation.PersonName
    |> data_element_value.new_binary_unchecked(<<0xFF, 0xFF>>)
    |> data_element_value.to_string(tag, 80)
    == "!! Invalid UTF-8 data"

  assert value_representation.AttributeTag
    |> data_element_value.new_binary_unchecked(<<0x34, 0x12, 0x78, 0x56>>)
    |> data_element_value.to_string(tag, 80)
    == "(1234,5678)"

  assert value_representation.AttributeTag
    |> data_element_value.new_binary_unchecked(<<0>>)
    |> data_element_value.to_string(tag, 80)
    == "<error converting to string>"

  assert [
      ieee_float.finite(1.0),
      ieee_float.finite(2.5),
      ieee_float.positive_infinity(),
      ieee_float.negative_infinity(),
      ieee_float.nan(),
    ]
    |> data_element_value.new_floating_point_single
    |> result.map(data_element_value.to_string(_, tag, 80))
    == Ok("1.0, 2.5, Infinity, -Infinity, NaN")

  assert value_representation.FloatingPointDouble
    |> data_element_value.new_binary_unchecked(<<0, 0, 0, 0>>)
    |> data_element_value.to_string(tag, 80)
    == "<error converting to string>"

  assert <<0, 1, 2, 3>>
    |> data_element_value.new_other_byte_string
    |> result.map(data_element_value.to_string(_, tag, 80))
    == Ok("[00 01 02 03]")

  assert <<0>>
    |> list.repeat(128)
    |> bit_array.concat
    |> data_element_value.new_other_byte_string
    |> result.map(data_element_value.to_string(_, tag, 50))
    == Ok("[00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 …")

  assert [4000, -30_000]
    |> data_element_value.new_signed_short
    |> result.map(data_element_value.to_string(_, tag, 80))
    == Ok("4000, -30000")

  assert value_representation.UnsignedShort
    |> data_element_value.new_lookup_table_descriptor_unchecked(<<
      0xA0, 0x0F, 0x40, 0x9C, 0x50, 0xC3,
    >>)
    |> data_element_value.to_string(tag, 80)
    == "4000, 40000, 50000"

  assert value_representation.SignedShort
    |> data_element_value.new_lookup_table_descriptor_unchecked(<<
      0xA0, 0x0F, 0xE0, 0xB1, 0x50, 0xC3,
    >>)
    |> data_element_value.to_string(tag, 80)
    == "4000, -20000, 50000"

  assert value_representation.SignedShort
    |> data_element_value.new_binary_unchecked(<<0>>)
    |> data_element_value.to_string(tag, 80)
    == "<error converting to string>"

  assert value_representation.OtherByteString
    |> data_element_value.new_encapsulated_pixel_data_unchecked([
      <<1, 2>>,
      <<3, 4>>,
    ])
    |> data_element_value.to_string(tag, 80)
    == "Items: 2, bytes: 4"

  assert [dict.new()]
    |> data_element_value.new_sequence
    |> data_element_value.to_string(tag, 80)
    == "Items: 1"
}

pub fn validate_length_test() {
  let assert Ok(_) =
    value_representation.SignedShort
    |> data_element_value.new_lookup_table_descriptor_unchecked(<<
      0, 0, 0, 0, 0, 0,
    >>)
    |> data_element_value.validate_length

  assert value_representation.SignedShort
    |> data_element_value.new_lookup_table_descriptor_unchecked(<<0, 0, 0, 0>>)
    |> data_element_value.validate_length
    == Error(data_error.new_value_length_invalid(
      value_representation.SignedShort,
      4,
      "Lookup table descriptor length must be exactly 6 bytes",
    ))

  assert value_representation.ShortText
    |> data_element_value.new_binary_unchecked(
      <<0>>
      |> list.repeat(0x10000)
      |> bit_array.concat,
    )
    |> data_element_value.validate_length
    == Error(data_error.new_value_length_invalid(
      value_representation.ShortText,
      65_536,
      "Must not exceed 65534 bytes",
    ))

  assert value_representation.UnsignedVeryLong
    |> data_element_value.new_binary_unchecked(<<0, 0, 0, 0, 0, 0, 0>>)
    |> data_element_value.validate_length
    == Error(data_error.new_value_length_invalid(
      value_representation.UnsignedVeryLong,
      7,
      "Must be a multiple of 8 bytes",
    ))

  let assert Ok(_) =
    value_representation.OtherWordString
    |> data_element_value.new_encapsulated_pixel_data_unchecked([<<0, 0>>])
    |> data_element_value.validate_length

  assert value_representation.OtherWordString
    |> data_element_value.new_encapsulated_pixel_data_unchecked([<<0, 0, 0>>])
    |> data_element_value.validate_length
    == Error(data_error.new_value_length_invalid(
      value_representation.OtherWordString,
      3,
      "Must be a multiple of 2 bytes",
    ))

  assert value_representation.OtherWordString
    |> data_element_value.new_encapsulated_pixel_data_unchecked([
      list.repeat(<<0:64, 0:64, 0:64, 0:64, 0:64, 0:64, 0:64, 0:64>>, 8192)
      |> bit_array.concat
      |> list.repeat(8192)
      |> bit_array.concat,
    ])
    |> data_element_value.validate_length
    == Error(data_error.new_value_length_invalid(
      value_representation.OtherWordString,
      4_294_967_296,
      "Must not exceed 4294967294 bytes",
    ))

  let assert Ok(_) =
    []
    |> data_element_value.new_sequence
    |> data_element_value.validate_length
}

pub fn new_age_string_test() {
  assert data_element_value.new_age_string(age_string.StructuredAge(
      99,
      age_string.Years,
    ))
    == data_element_value.new_binary(value_representation.AgeString, <<"099Y">>)
}

pub fn new_application_entity_test() {
  assert data_element_value.new_application_entity("TEST  ")
    == data_element_value.new_binary(value_representation.ApplicationEntity, <<
      "TEST",
    >>)

  assert "A"
    |> string.repeat(17)
    |> data_element_value.new_application_entity
    == Error(data_error.new_value_length_invalid(
      value_representation.ApplicationEntity,
      18,
      "Must not exceed 16 bytes",
    ))
}

pub fn new_attribute_tag_test() {
  assert data_element_value.new_attribute_tag([
      DataElementTag(0x0123, 0x4567),
      DataElementTag(0x89AB, 0xCDEF),
    ])
    == data_element_value.new_binary(value_representation.AttributeTag, <<
      0x23, 0x01, 0x67, 0x45, 0xAB, 0x89, 0xEF, 0xCD,
    >>)
}

pub fn new_code_string_test() {
  assert data_element_value.new_code_string(["DERIVED ", "SECONDARY"])
    == data_element_value.new_binary(value_representation.CodeString, <<
      "DERIVED\\SECONDARY ",
    >>)

  assert data_element_value.new_code_string(["\\"])
    == Error(data_error.new_value_invalid(
      "String list item contains backslashes",
    ))

  assert data_element_value.new_code_string([string.repeat("A", 17)])
    == Error(data_error.new_value_invalid(
      "String list item is longer than the max length of 16",
    ))

  assert data_element_value.new_code_string(["é"])
    == Error(data_error.new_value_invalid(
      "Bytes for 'CS' has disallowed byte: 0xC3",
    ))
}

pub fn new_date_test() {
  assert data_element_value.new_date(date.StructuredDate(2024, 2, 14))
    == data_element_value.new_binary(value_representation.Date, <<"20240214">>)
}

pub fn new_date_time_test() {
  assert data_element_value.new_date_time(date_time.StructuredDateTime(
      2024,
      Some(2),
      Some(14),
      Some(22),
      Some(5),
      Some(46.1),
      Some(800),
    ))
    == data_element_value.new_binary(value_representation.DateTime, <<
      "20240214220546.1+0800 ",
    >>)
}

pub fn new_decimal_string_test() {
  assert data_element_value.new_decimal_string([1.2, -3.45])
    == data_element_value.new_binary(value_representation.DecimalString, <<
      "1.2\\-3.45 ",
    >>)
}

pub fn new_floating_point_double_test() {
  assert data_element_value.new_floating_point_double([
      ieee_float.finite(1.2),
      ieee_float.finite(-3.45),
    ])
    == data_element_value.new_binary(value_representation.FloatingPointDouble, <<
      0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0xF3, 0x3F, 0x9A, 0x99, 0x99, 0x99,
      0x99, 0x99, 0xB, 0xC0,
    >>)
}

pub fn new_floating_point_single_test() {
  assert data_element_value.new_floating_point_single([
      ieee_float.finite(1.2),
      ieee_float.finite(-3.45),
    ])
    == data_element_value.new_binary(value_representation.FloatingPointSingle, <<
      0x9A, 0x99, 0x99, 0x3F, 0xCD, 0xCC, 0x5C, 0xC0,
    >>)
}

pub fn new_integer_string_test() {
  assert data_element_value.new_integer_string([10, 2_147_483_647])
    == data_element_value.new_binary(value_representation.IntegerString, <<
      "10\\2147483647 ",
    >>)
}

pub fn new_long_string_test() {
  assert data_element_value.new_long_string(["AA", "BB"])
    == data_element_value.new_binary(value_representation.LongString, <<
      "AA\\BB ",
    >>)
}

pub fn new_long_text_test() {
  assert data_element_value.new_long_text("ABC")
    == data_element_value.new_binary(value_representation.LongText, <<"ABC ">>)
}

pub fn new_other_byte_string_test() {
  assert data_element_value.new_other_byte_string(<<1, 2>>)
    == data_element_value.new_binary(value_representation.OtherByteString, <<
      1,
      2,
    >>)
}

pub fn new_other_double_string_test() {
  assert data_element_value.new_other_double_string([
      ieee_float.finite(1.2),
      ieee_float.finite(-3.45),
    ])
    == data_element_value.new_binary(value_representation.OtherDoubleString, <<
      0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0xF3, 0x3F, 0x9A, 0x99, 0x99, 0x99,
      0x99, 0x99, 0xB, 0xC0,
    >>)
}

pub fn new_other_float_string_test() {
  assert data_element_value.new_other_float_string([
      ieee_float.finite(1.2),
      ieee_float.finite(-3.45),
    ])
    == data_element_value.new_binary(value_representation.OtherFloatString, <<
      0x9A, 0x99, 0x99, 0x3F, 0xCD, 0xCC, 0x5C, 0xC0,
    >>)
}

pub fn new_other_long_string_test() {
  assert data_element_value.new_other_long_string(<<0, 1, 2>>)
    == Error(data_error.new_value_length_invalid(
      value_representation.OtherLongString,
      3,
      "Must be a multiple of 4 bytes",
    ))

  assert data_element_value.new_other_long_string(<<0, 1, 2, 3>>)
    == data_element_value.new_binary(value_representation.OtherLongString, <<
      0, 1, 2, 3,
    >>)
}

pub fn new_other_very_long_string_test() {
  assert data_element_value.new_other_very_long_string(<<0, 1, 2, 3, 4, 5, 6>>)
    == Error(data_error.new_value_length_invalid(
      value_representation.OtherVeryLongString,
      7,
      "Must be a multiple of 8 bytes",
    ))

  assert data_element_value.new_other_very_long_string(<<
      0,
      1,
      2,
      3,
      4,
      5,
      6,
      7,
    >>)
    == data_element_value.new_binary(value_representation.OtherVeryLongString, <<
      0, 1, 2, 3, 4, 5, 6, 7,
    >>)
}

pub fn new_other_word_string_test() {
  assert data_element_value.new_other_word_string(<<0, 1, 2>>)
    == Error(data_error.new_value_length_invalid(
      value_representation.OtherWordString,
      3,
      "Must be a multiple of 2 bytes",
    ))

  assert data_element_value.new_other_word_string(<<0, 1>>)
    == data_element_value.new_binary(value_representation.OtherWordString, <<
      0,
      1,
    >>)
}

pub fn new_person_name_test() {
  assert data_element_value.new_person_name([
      person_name.StructuredPersonName(
        None,
        Some(person_name.PersonNameComponents("1", " 2 ", "3", "4", "5")),
        None,
      ),
      person_name.StructuredPersonName(
        None,
        None,
        Some(person_name.PersonNameComponents("1", "2", "3", "4", "5")),
      ),
    ])
    == data_element_value.new_binary(value_representation.PersonName, <<
      "=1^ 2^3^4^5\\==1^2^3^4^5 ",
    >>)
}

pub fn new_short_string_test() {
  assert data_element_value.new_short_string([" AA ", "BB"])
    == data_element_value.new_binary(value_representation.ShortString, <<
      "AA\\BB ",
    >>)
}

pub fn new_short_text_test() {
  assert data_element_value.new_short_text(" ABC ")
    == data_element_value.new_binary(value_representation.ShortText, <<" ABC">>)
}

pub fn new_signed_long_test() {
  [3_000_000_000, -3_000_000_000]
  |> list.each(fn(i) {
    assert data_element_value.new_signed_long([i])
      == Error(data_error.new_value_invalid(
        "Value out of range for SignedLong VR",
      ))
  })

  assert data_element_value.new_signed_long([2_000_000_000, -2_000_000_000])
    == data_element_value.new_binary(value_representation.SignedLong, <<
      0x00, 0x94, 0x35, 0x77, 0x00, 0x6C, 0xCA, 0x88,
    >>)
}

pub fn new_signed_short_test() {
  [100_000, -100_000]
  |> list.each(fn(i) {
    assert data_element_value.new_signed_short([i])
      == Error(data_error.new_value_invalid(
        "Value out of range for SignedShort VR",
      ))
  })

  assert data_element_value.new_signed_short([10_000, -10_000])
    == data_element_value.new_binary(value_representation.SignedShort, <<
      0x10, 0x27, 0xF0, 0xD8,
    >>)
}

pub fn new_signed_very_long_test() {
  let assert Ok(i0) = bigi.from_string("10000000000000000000")
  let assert Ok(i1) = bigi.from_string("-10000000000000000000")
  [i0, i1]
  |> list.each(fn(i) {
    assert data_element_value.new_signed_very_long([i])
      == Error(data_error.new_value_invalid(
        "Value out of range for SignedVeryLong VR",
      ))
  })

  let assert Ok(i0) = bigi.from_string("1000000000000000000")
  let assert Ok(i1) = bigi.from_string("-1000000000000000000")
  assert data_element_value.new_signed_very_long([i0, i1])
    == data_element_value.new_binary(value_representation.SignedVeryLong, <<
      0x00, 0x00, 0x64, 0xA7, 0xB3, 0xB6, 0xE0, 0x0D, 0x00, 0x00, 0x9C, 0x58,
      0x4C, 0x49, 0x1F, 0xF2,
    >>)
}

pub fn new_time_test() {
  assert data_element_value.new_time(time.StructuredTime(
      22,
      Some(45),
      Some(14.0),
    ))
    == data_element_value.new_binary(value_representation.Time, <<"224514">>)
}

pub fn new_unique_identifier_test() {
  assert data_element_value.new_unique_identifier(["1.2", "3.4"])
    == data_element_value.new_binary(value_representation.UniqueIdentifier, <<
      "1.2\\3.4", 0,
    >>)
}

pub fn new_universal_resource_identifier_test() {
  assert data_element_value.new_universal_resource_identifier(
      "http;//test.com  ",
    )
    == data_element_value.new_binary(
      value_representation.UniversalResourceIdentifier,
      <<"http;//test.com ">>,
    )
}

pub fn new_unknown_test() {
  assert data_element_value.new_unknown(<<1, 2>>)
    == data_element_value.new_binary(value_representation.Unknown, <<1, 2>>)
}

pub fn new_unlimited_characters_test() {
  assert data_element_value.new_unlimited_characters([" ABCD "])
    == data_element_value.new_binary(value_representation.UnlimitedCharacters, <<
      " ABCD ",
    >>)
}

pub fn new_unlimited_text_test() {
  assert data_element_value.new_unlimited_text(" ABC ")
    == data_element_value.new_binary(value_representation.UnlimitedText, <<
      " ABC",
    >>)
}

pub fn new_unsigned_long_test() {
  [-1, 5_000_000_000]
  |> list.each(fn(i) {
    assert data_element_value.new_unsigned_long([i])
      == Error(data_error.new_value_invalid(
        "Value out of range for UnsignedLong VR",
      ))
  })

  assert data_element_value.new_unsigned_long([4_000_000_000])
    == data_element_value.new_binary(value_representation.UnsignedLong, <<
      0x00, 0x28, 0x6B, 0xEE,
    >>)
}

pub fn new_unsigned_short_test() {
  [-1, 100_000]
  |> list.each(fn(i) {
    assert data_element_value.new_unsigned_short([i])
      == Error(data_error.new_value_invalid(
        "Value out of range for UnsignedShort VR",
      ))
  })

  assert data_element_value.new_unsigned_short([50_000])
    == data_element_value.new_binary(value_representation.UnsignedShort, <<
      0x50, 0xC3,
    >>)
}

pub fn new_unsigned_very_long_test() {
  let assert Ok(i0) = bigi.from_string("-1")
  let assert Ok(i1) = bigi.from_string("20000000000000000000")
  [i0, i1]
  |> list.each(fn(i) {
    assert data_element_value.new_unsigned_very_long([i])
      == Error(data_error.new_value_invalid(
        "Value out of range for UnsignedVeryLong VR",
      ))
  })

  let assert Ok(i) = bigi.from_string("10000000000000000000")
  assert data_element_value.new_unsigned_very_long([i])
    == data_element_value.new_binary(value_representation.UnsignedVeryLong, <<
      0x00, 0x00, 0xE8, 0x89, 0x04, 0x23, 0xC7, 0x8A,
    >>)
}
