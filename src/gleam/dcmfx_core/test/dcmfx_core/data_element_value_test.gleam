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
import gleeunit/should
import ieee_float

pub fn value_representation_test() {
  ["123"]
  |> data_element_value.new_long_string
  |> result.map(data_element_value.value_representation)
  |> should.equal(Ok(value_representation.LongString))

  [ieee_float.finite(1.0)]
  |> data_element_value.new_floating_point_single
  |> result.map(data_element_value.value_representation)
  |> should.equal(Ok(value_representation.FloatingPointSingle))

  value_representation.UnsignedShort
  |> data_element_value.new_lookup_table_descriptor_unchecked(<<>>)
  |> data_element_value.value_representation
  |> should.equal(value_representation.UnsignedShort)

  value_representation.OtherWordString
  |> data_element_value.new_encapsulated_pixel_data_unchecked([])
  |> data_element_value.value_representation
  |> should.equal(value_representation.OtherWordString)

  []
  |> data_element_value.new_sequence
  |> data_element_value.value_representation
  |> should.equal(value_representation.Sequence)
}

pub fn bytes_test() {
  data_element_value.new_long_string(["12"])
  |> result.then(data_element_value.bytes)
  |> should.equal(Ok(<<"12">>))

  data_element_value.new_floating_point_single([ieee_float.finite(1.0)])
  |> result.then(data_element_value.bytes)
  |> should.equal(Ok(<<0x00, 0x00, 0x80, 0x3F>>))

  value_representation.UnsignedShort
  |> data_element_value.new_lookup_table_descriptor_unchecked(<<
    0, 1, 2, 3, 4, 5,
  >>)
  |> data_element_value.bytes
  |> should.equal(Ok(<<0, 1, 2, 3, 4, 5>>))

  value_representation.OtherWordString
  |> data_element_value.new_encapsulated_pixel_data_unchecked([])
  |> data_element_value.bytes
  |> should.be_error

  data_element_value.new_sequence([])
  |> data_element_value.bytes
  |> should.be_error
}

pub fn get_string_test() {
  "A"
  |> data_element_value.new_application_entity
  |> result.then(data_element_value.get_string)
  |> should.equal(Ok("A"))

  "A"
  |> data_element_value.new_long_text
  |> result.then(data_element_value.get_string)
  |> should.equal(Ok("A"))

  "A"
  |> data_element_value.new_short_text
  |> result.then(data_element_value.get_string)
  |> should.equal(Ok("A"))

  "A"
  |> data_element_value.new_universal_resource_identifier
  |> result.then(data_element_value.get_string)
  |> should.equal(Ok("A"))

  "A"
  |> data_element_value.new_unlimited_text
  |> result.then(data_element_value.get_string)
  |> should.equal(Ok("A"))

  data_element_value.new_binary_unchecked(value_representation.ShortText, <<
    0xD0,
  >>)
  |> data_element_value.get_string
  |> should.equal(
    Error(data_error.new_value_invalid("String bytes are not valid UTF-8")),
  )

  ["A"]
  |> data_element_value.new_long_string
  |> result.then(data_element_value.get_string)
  |> should.equal(Ok("A"))

  ["A", "B"]
  |> data_element_value.new_long_string
  |> result.then(data_element_value.get_string)
  |> should.equal(Error(data_error.new_multiplicity_mismatch()))

  [1]
  |> data_element_value.new_unsigned_short
  |> result.then(data_element_value.get_string)
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn get_strings_test() {
  ["A", "B"]
  |> data_element_value.new_code_string
  |> result.then(data_element_value.get_strings)
  |> should.equal(Ok(["A", "B"]))

  ["1.2", "3.4"]
  |> data_element_value.new_unique_identifier
  |> result.then(data_element_value.get_strings)
  |> should.equal(Ok(["1.2", "3.4"]))

  ["A", "B"]
  |> data_element_value.new_long_string
  |> result.then(data_element_value.get_strings)
  |> should.equal(Ok(["A", "B"]))

  ["A", "B"]
  |> data_element_value.new_short_string
  |> result.then(data_element_value.get_strings)
  |> should.equal(Ok(["A", "B"]))

  ["A", "B"]
  |> data_element_value.new_unlimited_characters
  |> result.then(data_element_value.get_strings)
  |> should.equal(Ok(["A", "B"]))

  data_element_value.new_binary_unchecked(value_representation.ShortString, <<
    0xD0,
  >>)
  |> data_element_value.get_strings
  |> should.equal(
    Error(data_error.new_value_invalid("String bytes are not valid UTF-8")),
  )

  "A"
  |> data_element_value.new_long_text
  |> result.then(data_element_value.get_strings)
  |> should.equal(Error(data_error.new_value_not_present()))

  [1]
  |> data_element_value.new_unsigned_short
  |> result.then(data_element_value.get_strings)
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn get_int_test() {
  value_representation.IntegerString
  |> data_element_value.new_binary_unchecked(<<"  123   ">>)
  |> data_element_value.get_int
  |> should.equal(Ok(123))

  [1234]
  |> data_element_value.new_unsigned_long
  |> result.then(data_element_value.get_int)
  |> should.equal(Ok(1234))

  [123, 456]
  |> data_element_value.new_unsigned_long
  |> result.then(data_element_value.get_int)
  |> should.equal(Error(data_error.new_multiplicity_mismatch()))

  "123"
  |> data_element_value.new_long_text
  |> result.then(data_element_value.get_int)
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn get_ints_test() {
  value_representation.IntegerString
  |> data_element_value.new_binary_unchecked(<<" 123 \\456">>)
  |> data_element_value.get_ints
  |> should.equal(Ok([123, 456]))

  [-{ 0x80000000 }, 0x7FFFFFFF]
  |> data_element_value.new_signed_long
  |> result.then(data_element_value.get_ints)
  |> should.equal(Ok([-{ 0x80000000 }, 0x7FFFFFFF]))

  value_representation.SignedLong
  |> data_element_value.new_binary_unchecked(<<0>>)
  |> data_element_value.get_ints
  |> should.equal(Error(data_error.new_value_invalid("Invalid Int32 list")))

  [-{ 0x8000 }, 0x7FFF]
  |> data_element_value.new_signed_short
  |> result.then(data_element_value.get_ints)
  |> should.equal(Ok([-{ 0x8000 }, 0x7FFF]))

  value_representation.SignedShort
  |> data_element_value.new_binary_unchecked(<<0>>)
  |> data_element_value.get_ints
  |> should.equal(Error(data_error.new_value_invalid("Invalid Int16 list")))

  [0, 0xFFFFFFFF]
  |> data_element_value.new_unsigned_long
  |> result.then(data_element_value.get_ints)
  |> should.equal(Ok([0, 0xFFFFFFFF]))

  value_representation.UnsignedLong
  |> data_element_value.new_binary_unchecked(<<0>>)
  |> data_element_value.get_ints
  |> should.equal(Error(data_error.new_value_invalid("Invalid Uint32 list")))

  [0, 0xFFFF]
  |> data_element_value.new_unsigned_short
  |> result.then(data_element_value.get_ints)
  |> should.equal(Ok([0, 0xFFFF]))

  value_representation.UnsignedShort
  |> data_element_value.new_binary_unchecked(<<0>>)
  |> data_element_value.get_ints
  |> should.equal(Error(data_error.new_value_invalid("Invalid Uint16 list")))

  value_representation.SignedShort
  |> data_element_value.new_lookup_table_descriptor_unchecked(<<
    0x34, 0x12, 0x00, 0x80, 0x78, 0x56,
  >>)
  |> data_element_value.get_ints
  |> should.equal(Ok([0x1234, -{ 0x8000 }, 0x5678]))

  value_representation.UnsignedShort
  |> data_element_value.new_lookup_table_descriptor_unchecked(<<
    0x34, 0x12, 0x00, 0x80, 0x78, 0x56,
  >>)
  |> data_element_value.get_ints
  |> should.equal(Ok([0x1234, 0x8000, 0x5678]))

  value_representation.OtherWordString
  |> data_element_value.new_lookup_table_descriptor_unchecked(<<
    0, 0, 0, 0, 0, 0,
  >>)
  |> data_element_value.get_ints
  |> should.equal(
    Error(data_error.new_value_invalid("Invalid lookup table descriptor")),
  )

  value_representation.UnsignedShort
  |> data_element_value.new_lookup_table_descriptor_unchecked(<<0, 0, 0, 0>>)
  |> data_element_value.get_ints
  |> should.equal(
    Error(data_error.new_value_invalid("Invalid lookup table descriptor")),
  )

  [ieee_float.finite(123.0)]
  |> data_element_value.new_floating_point_single
  |> result.then(data_element_value.get_ints)
  |> should.equal(Error(data_error.new_value_not_present()))

  "123"
  |> data_element_value.new_long_text
  |> result.then(data_element_value.get_ints)
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn get_big_int_test() {
  let assert Ok(i0) = bigi.from_string("-9223372036854775808")
  [i0]
  |> data_element_value.new_signed_very_long
  |> result.then(data_element_value.get_big_int)
  |> should.equal(Ok(i0))

  let assert Ok(i0) = bigi.from_string("9223372036854775807")
  [i0]
  |> data_element_value.new_unsigned_very_long
  |> result.then(data_element_value.get_big_int)
  |> should.equal(Ok(i0))

  let assert Ok(i0) = bigi.from_string("1234")
  let assert Ok(i1) = bigi.from_string("1234")
  [i0, i1]
  |> data_element_value.new_unsigned_very_long
  |> result.then(data_element_value.get_big_int)
  |> should.equal(Error(data_error.new_multiplicity_mismatch()))

  "123"
  |> data_element_value.new_long_text
  |> result.then(data_element_value.get_big_int)
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn get_big_ints_test() {
  let assert Ok(i0) = bigi.from_string("-9223372036854775808")
  let assert Ok(i1) = bigi.from_string("9223372036854775807")
  [i0, i1]
  |> data_element_value.new_signed_very_long
  |> result.then(data_element_value.get_big_ints)
  |> should.equal(Ok([i0, i1]))

  value_representation.SignedVeryLong
  |> data_element_value.new_binary_unchecked(<<0>>)
  |> data_element_value.get_big_ints
  |> should.equal(Error(data_error.new_value_invalid("Invalid Int64 list")))

  let assert Ok(i) = bigi.from_string("18446744073709551615")
  [bigi.zero(), i]
  |> data_element_value.new_unsigned_very_long
  |> result.then(data_element_value.get_big_ints)
  |> should.equal(Ok([bigi.zero(), i]))

  value_representation.UnsignedVeryLong
  |> data_element_value.new_binary_unchecked(<<0>>)
  |> data_element_value.get_big_ints
  |> should.equal(Error(data_error.new_value_invalid("Invalid Uint64 list")))

  [ieee_float.finite(123.0)]
  |> data_element_value.new_floating_point_single
  |> result.then(data_element_value.get_big_ints)
  |> should.equal(Error(data_error.new_value_not_present()))

  "123"
  |> data_element_value.new_long_text
  |> result.then(data_element_value.get_big_ints)
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn get_float_test() {
  value_representation.DecimalString
  |> data_element_value.new_binary_unchecked(<<" 1.2   ">>)
  |> data_element_value.get_float
  |> should.equal(Ok(ieee_float.finite(1.2)))

  [ieee_float.finite(1.0)]
  |> data_element_value.new_floating_point_single
  |> result.then(data_element_value.get_float)
  |> should.equal(Ok(ieee_float.finite(1.0)))

  [ieee_float.positive_infinity()]
  |> data_element_value.new_floating_point_single
  |> result.then(data_element_value.get_float)
  |> should.equal(Ok(ieee_float.positive_infinity()))

  [ieee_float.finite(1.2), ieee_float.finite(3.4)]
  |> data_element_value.new_floating_point_double
  |> result.then(data_element_value.get_float)
  |> should.equal(Error(data_error.new_multiplicity_mismatch()))

  "1.2"
  |> data_element_value.new_long_text
  |> result.then(data_element_value.get_float)
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn get_floats_test() {
  value_representation.DecimalString
  |> data_element_value.new_binary_unchecked(<<" 1.2  \\3.4">>)
  |> data_element_value.get_floats
  |> should.equal(Ok([ieee_float.finite(1.2), ieee_float.finite(3.4)]))

  [ieee_float.finite(1.2), ieee_float.finite(3.4)]
  |> data_element_value.new_floating_point_double
  |> result.then(data_element_value.get_floats)
  |> should.equal(Ok([ieee_float.finite(1.2), ieee_float.finite(3.4)]))

  [ieee_float.finite(1.0), ieee_float.finite(2.0)]
  |> data_element_value.new_other_double_string
  |> result.then(data_element_value.get_floats)
  |> should.equal(Ok([ieee_float.finite(1.0), ieee_float.finite(2.0)]))

  [ieee_float.finite(1.0), ieee_float.finite(2.0)]
  |> data_element_value.new_other_double_string
  |> result.then(data_element_value.get_floats)
  |> should.equal(Ok([ieee_float.finite(1.0), ieee_float.finite(2.0)]))

  value_representation.FloatingPointDouble
  |> data_element_value.new_binary_unchecked(<<0, 0, 0, 0>>)
  |> data_element_value.get_floats
  |> should.equal(Error(data_error.new_value_invalid("Invalid Float64 list")))

  [ieee_float.finite(1.0), ieee_float.finite(2.0)]
  |> data_element_value.new_floating_point_single
  |> result.then(data_element_value.get_floats)
  |> should.equal(Ok([ieee_float.finite(1.0), ieee_float.finite(2.0)]))

  [ieee_float.finite(1.0), ieee_float.finite(2.0)]
  |> data_element_value.new_other_float_string
  |> result.then(data_element_value.get_floats)
  |> should.equal(Ok([ieee_float.finite(1.0), ieee_float.finite(2.0)]))

  value_representation.FloatingPointSingle
  |> data_element_value.new_binary_unchecked(<<0, 0>>)
  |> data_element_value.get_floats
  |> should.equal(Error(data_error.new_value_invalid("Invalid Float32 list")))

  "1.2"
  |> data_element_value.new_long_text
  |> result.then(data_element_value.get_floats)
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn get_age_test() {
  value_representation.AgeString
  |> data_element_value.new_binary_unchecked(<<"001D">>)
  |> data_element_value.get_age
  |> should.equal(Ok(age_string.StructuredAge(1, age_string.Days)))

  value_representation.Date
  |> data_element_value.new_binary_unchecked(<<>>)
  |> data_element_value.get_age
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn get_date_test() {
  value_representation.Date
  |> data_element_value.new_binary_unchecked(<<"20000101">>)
  |> data_element_value.get_date
  |> should.equal(Ok(date.StructuredDate(2000, 1, 1)))

  value_representation.Time
  |> data_element_value.new_binary_unchecked(<<>>)
  |> data_element_value.get_date
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn get_date_time_test() {
  value_representation.DateTime
  |> data_element_value.new_binary_unchecked(<<"20000101123043.5">>)
  |> data_element_value.get_date_time
  |> should.equal(
    Ok(date_time.StructuredDateTime(
      2000,
      Some(1),
      Some(1),
      Some(12),
      Some(30),
      Some(43.5),
      None,
    )),
  )

  value_representation.Date
  |> data_element_value.new_binary_unchecked(<<>>)
  |> data_element_value.get_date_time
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn get_time_test() {
  value_representation.Time
  |> data_element_value.new_binary_unchecked(<<"235921.2">>)
  |> data_element_value.get_time
  |> should.equal(Ok(time.StructuredTime(23, Some(59), Some(21.2))))

  value_representation.Date
  |> data_element_value.new_binary_unchecked(<<>>)
  |> data_element_value.get_time
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn get_person_name_test() {
  value_representation.PersonName
  |> data_element_value.new_binary_unchecked(<<"">>)
  |> data_element_value.get_person_name
  |> should.equal(Ok(person_name.StructuredPersonName(None, None, None)))

  value_representation.PersonName
  |> data_element_value.new_binary_unchecked(<<"\\">>)
  |> data_element_value.get_person_name
  |> should.equal(Error(data_error.new_multiplicity_mismatch()))
}

pub fn get_person_names_test() {
  value_representation.PersonName
  |> data_element_value.new_binary_unchecked(<<"\\">>)
  |> data_element_value.get_person_names
  |> should.equal(
    Ok([
      person_name.StructuredPersonName(None, None, None),
      person_name.StructuredPersonName(None, None, None),
    ]),
  )

  value_representation.Date
  |> data_element_value.new_binary_unchecked(<<>>)
  |> data_element_value.get_person_names
  |> should.equal(Error(data_error.new_value_not_present()))
}

pub fn to_string_test() {
  let tag = DataElementTag(0, 0)

  ["DERIVED", "SECONDARY"]
  |> data_element_value.new_code_string
  |> result.map(data_element_value.to_string(_, tag, 80))
  |> should.equal(Ok("\"DERIVED\", \"SECONDARY\""))

  ["CT"]
  |> data_element_value.new_code_string
  |> result.map(data_element_value.to_string(_, dictionary.modality.tag, 80))
  |> should.equal(Ok("\"CT\" (Computed Tomography)"))

  ["1.23"]
  |> data_element_value.new_unique_identifier
  |> result.map(data_element_value.to_string(_, tag, 80))
  |> should.equal(Ok("\"1.23\""))

  ["1.2.840.10008.1.2"]
  |> data_element_value.new_unique_identifier
  |> result.map(data_element_value.to_string(_, tag, 80))
  |> should.equal(Ok("\"1.2.840.10008.1.2\" (Implicit VR Little Endian)"))

  value_representation.PersonName
  |> data_element_value.new_binary_unchecked(<<0xFF, 0xFF>>)
  |> data_element_value.to_string(tag, 80)
  |> should.equal("!! Invalid UTF-8 data")

  value_representation.AttributeTag
  |> data_element_value.new_binary_unchecked(<<0x34, 0x12, 0x78, 0x56>>)
  |> data_element_value.to_string(tag, 80)
  |> should.equal("(1234,5678)")

  value_representation.AttributeTag
  |> data_element_value.new_binary_unchecked(<<0>>)
  |> data_element_value.to_string(tag, 80)
  |> should.equal("<error converting to string>")

  [
    ieee_float.finite(1.0),
    ieee_float.finite(2.5),
    ieee_float.positive_infinity(),
    ieee_float.negative_infinity(),
    ieee_float.nan(),
  ]
  |> data_element_value.new_floating_point_single
  |> result.map(data_element_value.to_string(_, tag, 80))
  |> should.equal(Ok("1.0, 2.5, Infinity, -Infinity, NaN"))

  value_representation.FloatingPointDouble
  |> data_element_value.new_binary_unchecked(<<0, 0, 0, 0>>)
  |> data_element_value.to_string(tag, 80)
  |> should.equal("<error converting to string>")

  <<0, 1, 2, 3>>
  |> data_element_value.new_other_byte_string
  |> result.map(data_element_value.to_string(_, tag, 80))
  |> should.equal(Ok("[00 01 02 03]"))

  <<0>>
  |> list.repeat(128)
  |> bit_array.concat
  |> data_element_value.new_other_byte_string
  |> result.map(data_element_value.to_string(_, tag, 50))
  |> should.equal(Ok("[00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 …"))

  [4000, -30_000]
  |> data_element_value.new_signed_short
  |> result.map(data_element_value.to_string(_, tag, 80))
  |> should.equal(Ok("4000, -30000"))

  value_representation.UnsignedShort
  |> data_element_value.new_lookup_table_descriptor_unchecked(<<
    0xA0, 0x0F, 0x40, 0x9C, 0x50, 0xC3,
  >>)
  |> data_element_value.to_string(tag, 80)
  |> should.equal("4000, 40000, 50000")

  value_representation.SignedShort
  |> data_element_value.new_lookup_table_descriptor_unchecked(<<
    0xA0, 0x0F, 0xE0, 0xB1, 0x50, 0xC3,
  >>)
  |> data_element_value.to_string(tag, 80)
  |> should.equal("4000, -20000, 50000")

  value_representation.SignedShort
  |> data_element_value.new_binary_unchecked(<<0>>)
  |> data_element_value.to_string(tag, 80)
  |> should.equal("<error converting to string>")

  value_representation.OtherByteString
  |> data_element_value.new_encapsulated_pixel_data_unchecked([
    <<1, 2>>,
    <<3, 4>>,
  ])
  |> data_element_value.to_string(tag, 80)
  |> should.equal("Items: 2, bytes: 4")

  [dict.new()]
  |> data_element_value.new_sequence
  |> data_element_value.to_string(tag, 80)
  |> should.equal("Items: 1")
}

pub fn validate_length_test() {
  value_representation.SignedShort
  |> data_element_value.new_lookup_table_descriptor_unchecked(<<
    0, 0, 0, 0, 0, 0,
  >>)
  |> data_element_value.validate_length
  |> should.be_ok

  value_representation.SignedShort
  |> data_element_value.new_lookup_table_descriptor_unchecked(<<0, 0, 0, 0>>)
  |> data_element_value.validate_length
  |> should.equal(
    Error(data_error.new_value_length_invalid(
      value_representation.SignedShort,
      4,
      "Lookup table descriptor length must be exactly 6 bytes",
    )),
  )

  value_representation.ShortText
  |> data_element_value.new_binary_unchecked(
    <<0>>
    |> list.repeat(0x10000)
    |> bit_array.concat,
  )
  |> data_element_value.validate_length
  |> should.equal(
    Error(data_error.new_value_length_invalid(
      value_representation.ShortText,
      65_536,
      "Must not exceed 65534 bytes",
    )),
  )

  value_representation.UnsignedVeryLong
  |> data_element_value.new_binary_unchecked(<<0, 0, 0, 0, 0, 0, 0>>)
  |> data_element_value.validate_length
  |> should.equal(
    Error(data_error.new_value_length_invalid(
      value_representation.UnsignedVeryLong,
      7,
      "Must be a multiple of 8 bytes",
    )),
  )

  value_representation.OtherWordString
  |> data_element_value.new_encapsulated_pixel_data_unchecked([<<0, 0>>])
  |> data_element_value.validate_length
  |> should.be_ok

  value_representation.OtherWordString
  |> data_element_value.new_encapsulated_pixel_data_unchecked([<<0, 0, 0>>])
  |> data_element_value.validate_length
  |> should.equal(
    Error(data_error.new_value_length_invalid(
      value_representation.OtherWordString,
      3,
      "Must be a multiple of 2 bytes",
    )),
  )

  value_representation.OtherWordString
  |> data_element_value.new_encapsulated_pixel_data_unchecked([
    list.repeat(<<0:64, 0:64, 0:64, 0:64, 0:64, 0:64, 0:64, 0:64>>, 8192)
    |> bit_array.concat
    |> list.repeat(8192)
    |> bit_array.concat,
  ])
  |> data_element_value.validate_length
  |> should.equal(
    Error(data_error.new_value_length_invalid(
      value_representation.OtherWordString,
      4_294_967_296,
      "Must not exceed 4294967294 bytes",
    )),
  )

  []
  |> data_element_value.new_sequence
  |> data_element_value.validate_length
  |> should.be_ok
}

pub fn new_age_string_test() {
  age_string.StructuredAge(99, age_string.Years)
  |> data_element_value.new_age_string
  |> should.equal(
    data_element_value.new_binary(value_representation.AgeString, <<"099Y">>),
  )
}

pub fn new_application_entity_test() {
  "TEST  "
  |> data_element_value.new_application_entity
  |> should.equal(
    data_element_value.new_binary(value_representation.ApplicationEntity, <<
      "TEST",
    >>),
  )

  "A"
  |> string.repeat(17)
  |> data_element_value.new_application_entity
  |> should.equal(
    Error(data_error.new_value_length_invalid(
      value_representation.ApplicationEntity,
      18,
      "Must not exceed 16 bytes",
    )),
  )
}

pub fn new_attribute_tag_test() {
  [DataElementTag(0x0123, 0x4567), DataElementTag(0x89AB, 0xCDEF)]
  |> data_element_value.new_attribute_tag
  |> should.equal(
    data_element_value.new_binary(value_representation.AttributeTag, <<
      0x23, 0x01, 0x67, 0x45, 0xAB, 0x89, 0xEF, 0xCD,
    >>),
  )
}

pub fn new_code_string_test() {
  ["DERIVED ", "SECONDARY"]
  |> data_element_value.new_code_string
  |> should.equal(
    data_element_value.new_binary(value_representation.CodeString, <<
      "DERIVED\\SECONDARY ",
    >>),
  )

  ["\\"]
  |> data_element_value.new_code_string
  |> should.equal(
    Error(data_error.new_value_invalid("String list item contains backslashes")),
  )

  [string.repeat("A", 17)]
  |> data_element_value.new_code_string
  |> should.equal(
    Error(data_error.new_value_invalid(
      "String list item is longer than the max length of 16",
    )),
  )

  ["é"]
  |> data_element_value.new_code_string
  |> should.equal(
    Error(data_error.new_value_invalid(
      "Bytes for 'CS' has disallowed byte: 0xC3",
    )),
  )
}

pub fn new_date_test() {
  date.StructuredDate(2024, 2, 14)
  |> data_element_value.new_date
  |> should.equal(
    data_element_value.new_binary(value_representation.Date, <<"20240214">>),
  )
}

pub fn new_date_time_test() {
  date_time.StructuredDateTime(
    2024,
    Some(2),
    Some(14),
    Some(22),
    Some(5),
    Some(46.1),
    Some(800),
  )
  |> data_element_value.new_date_time
  |> should.equal(
    data_element_value.new_binary(value_representation.DateTime, <<
      "20240214220546.1+0800 ",
    >>),
  )
}

pub fn new_decimal_string_test() {
  [1.2, -3.45]
  |> data_element_value.new_decimal_string
  |> should.equal(
    data_element_value.new_binary(value_representation.DecimalString, <<
      "1.2\\-3.45 ",
    >>),
  )
}

pub fn new_floating_point_double_test() {
  [ieee_float.finite(1.2), ieee_float.finite(-3.45)]
  |> data_element_value.new_floating_point_double
  |> should.equal(
    data_element_value.new_binary(value_representation.FloatingPointDouble, <<
      0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0xF3, 0x3F, 0x9A, 0x99, 0x99, 0x99,
      0x99, 0x99, 0xB, 0xC0,
    >>),
  )
}

pub fn new_floating_point_single_test() {
  [ieee_float.finite(1.2), ieee_float.finite(-3.45)]
  |> data_element_value.new_floating_point_single
  |> should.equal(
    data_element_value.new_binary(value_representation.FloatingPointSingle, <<
      0x9A, 0x99, 0x99, 0x3F, 0xCD, 0xCC, 0x5C, 0xC0,
    >>),
  )
}

pub fn new_integer_string_test() {
  [10, 2_147_483_647]
  |> data_element_value.new_integer_string
  |> should.equal(
    data_element_value.new_binary(value_representation.IntegerString, <<
      "10\\2147483647 ",
    >>),
  )
}

pub fn new_long_string_test() {
  ["AA", "BB"]
  |> data_element_value.new_long_string
  |> should.equal(
    data_element_value.new_binary(value_representation.LongString, <<"AA\\BB ">>),
  )
}

pub fn new_long_text_test() {
  "ABC"
  |> data_element_value.new_long_text
  |> should.equal(
    data_element_value.new_binary(value_representation.LongText, <<"ABC ">>),
  )
}

pub fn new_other_byte_string_test() {
  <<1, 2>>
  |> data_element_value.new_other_byte_string
  |> should.equal(
    data_element_value.new_binary(value_representation.OtherByteString, <<1, 2>>),
  )
}

pub fn new_other_double_string_test() {
  [ieee_float.finite(1.2), ieee_float.finite(-3.45)]
  |> data_element_value.new_other_double_string
  |> should.equal(
    data_element_value.new_binary(value_representation.OtherDoubleString, <<
      0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0xF3, 0x3F, 0x9A, 0x99, 0x99, 0x99,
      0x99, 0x99, 0xB, 0xC0,
    >>),
  )
}

pub fn new_other_float_string_test() {
  [ieee_float.finite(1.2), ieee_float.finite(-3.45)]
  |> data_element_value.new_other_float_string
  |> should.equal(
    data_element_value.new_binary(value_representation.OtherFloatString, <<
      0x9A, 0x99, 0x99, 0x3F, 0xCD, 0xCC, 0x5C, 0xC0,
    >>),
  )
}

pub fn new_other_long_string_test() {
  <<0, 1, 2>>
  |> data_element_value.new_other_long_string
  |> should.equal(
    Error(data_error.new_value_length_invalid(
      value_representation.OtherLongString,
      3,
      "Must be a multiple of 4 bytes",
    )),
  )

  <<0, 1, 2, 3>>
  |> data_element_value.new_other_long_string
  |> should.equal(
    data_element_value.new_binary(value_representation.OtherLongString, <<
      0, 1, 2, 3,
    >>),
  )
}

pub fn new_other_very_long_string_test() {
  <<0, 1, 2, 3, 4, 5, 6>>
  |> data_element_value.new_other_very_long_string
  |> should.equal(
    Error(data_error.new_value_length_invalid(
      value_representation.OtherVeryLongString,
      7,
      "Must be a multiple of 8 bytes",
    )),
  )

  <<0, 1, 2, 3, 4, 5, 6, 7>>
  |> data_element_value.new_other_very_long_string
  |> should.equal(
    data_element_value.new_binary(value_representation.OtherVeryLongString, <<
      0, 1, 2, 3, 4, 5, 6, 7,
    >>),
  )
}

pub fn new_other_word_string_test() {
  <<0, 1, 2>>
  |> data_element_value.new_other_word_string
  |> should.equal(
    Error(data_error.new_value_length_invalid(
      value_representation.OtherWordString,
      3,
      "Must be a multiple of 2 bytes",
    )),
  )

  <<0, 1>>
  |> data_element_value.new_other_word_string
  |> should.equal(
    data_element_value.new_binary(value_representation.OtherWordString, <<0, 1>>),
  )
}

pub fn new_person_name_test() {
  [
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
  ]
  |> data_element_value.new_person_name
  |> should.equal(
    data_element_value.new_binary(value_representation.PersonName, <<
      "=1^ 2^3^4^5\\==1^2^3^4^5 ",
    >>),
  )
}

pub fn new_short_string_test() {
  [" AA ", "BB"]
  |> data_element_value.new_short_string
  |> should.equal(
    data_element_value.new_binary(value_representation.ShortString, <<
      "AA\\BB ",
    >>),
  )
}

pub fn new_short_text_test() {
  " ABC "
  |> data_element_value.new_short_text
  |> should.equal(
    data_element_value.new_binary(value_representation.ShortText, <<" ABC">>),
  )
}

pub fn new_signed_long_test() {
  [3_000_000_000, -3_000_000_000]
  |> list.each(fn(i) {
    [i]
    |> data_element_value.new_signed_long
    |> should.equal(
      Error(data_error.new_value_invalid("Value out of range for SignedLong VR")),
    )
  })

  [2_000_000_000, -2_000_000_000]
  |> data_element_value.new_signed_long
  |> should.equal(
    data_element_value.new_binary(value_representation.SignedLong, <<
      0x00, 0x94, 0x35, 0x77, 0x00, 0x6C, 0xCA, 0x88,
    >>),
  )
}

pub fn new_signed_short_test() {
  [100_000, -100_000]
  |> list.each(fn(i) {
    [i]
    |> data_element_value.new_signed_short
    |> should.equal(
      Error(data_error.new_value_invalid(
        "Value out of range for SignedShort VR",
      )),
    )
  })

  [10_000, -10_000]
  |> data_element_value.new_signed_short
  |> should.equal(
    data_element_value.new_binary(value_representation.SignedShort, <<
      0x10, 0x27, 0xF0, 0xD8,
    >>),
  )
}

pub fn new_signed_very_long_test() {
  let assert Ok(i0) = bigi.from_string("10000000000000000000")
  let assert Ok(i1) = bigi.from_string("-10000000000000000000")
  [i0, i1]
  |> list.each(fn(i) {
    [i]
    |> data_element_value.new_signed_very_long
    |> should.equal(
      Error(data_error.new_value_invalid(
        "Value out of range for SignedVeryLong VR",
      )),
    )
  })

  let assert Ok(i0) = bigi.from_string("1000000000000000000")
  let assert Ok(i1) = bigi.from_string("-1000000000000000000")
  [i0, i1]
  |> data_element_value.new_signed_very_long
  |> should.equal(
    data_element_value.new_binary(value_representation.SignedVeryLong, <<
      0x00, 0x00, 0x64, 0xA7, 0xB3, 0xB6, 0xE0, 0x0D, 0x00, 0x00, 0x9C, 0x58,
      0x4C, 0x49, 0x1F, 0xF2,
    >>),
  )
}

pub fn new_time_test() {
  time.StructuredTime(22, Some(45), Some(14.0))
  |> data_element_value.new_time
  |> should.equal(
    data_element_value.new_binary(value_representation.Time, <<"224514">>),
  )
}

pub fn new_unique_identifier_test() {
  ["1.2", "3.4"]
  |> data_element_value.new_unique_identifier
  |> should.equal(
    data_element_value.new_binary(value_representation.UniqueIdentifier, <<
      "1.2\\3.4", 0,
    >>),
  )
}

pub fn new_universal_resource_identifier_test() {
  "http;//test.com  "
  |> data_element_value.new_universal_resource_identifier
  |> should.equal(
    data_element_value.new_binary(
      value_representation.UniversalResourceIdentifier,
      <<"http;//test.com ">>,
    ),
  )
}

pub fn new_unknown_test() {
  <<1, 2>>
  |> data_element_value.new_unknown
  |> should.equal(
    data_element_value.new_binary(value_representation.Unknown, <<1, 2>>),
  )
}

pub fn new_unlimited_characters_test() {
  [" ABCD "]
  |> data_element_value.new_unlimited_characters
  |> should.equal(
    data_element_value.new_binary(value_representation.UnlimitedCharacters, <<
      " ABCD ",
    >>),
  )
}

pub fn new_unlimited_text_test() {
  " ABC "
  |> data_element_value.new_unlimited_text
  |> should.equal(
    data_element_value.new_binary(value_representation.UnlimitedText, <<" ABC">>),
  )
}

pub fn new_unsigned_long_test() {
  [-1, 5_000_000_000]
  |> list.each(fn(i) {
    [i]
    |> data_element_value.new_unsigned_long
    |> should.equal(
      Error(data_error.new_value_invalid(
        "Value out of range for UnsignedLong VR",
      )),
    )
  })

  [4_000_000_000]
  |> data_element_value.new_unsigned_long
  |> should.equal(
    data_element_value.new_binary(value_representation.UnsignedLong, <<
      0x00, 0x28, 0x6B, 0xEE,
    >>),
  )
}

pub fn new_unsigned_short_test() {
  [-1, 100_000]
  |> list.each(fn(i) {
    [i]
    |> data_element_value.new_unsigned_short
    |> should.equal(
      Error(data_error.new_value_invalid(
        "Value out of range for UnsignedShort VR",
      )),
    )
  })

  [50_000]
  |> data_element_value.new_unsigned_short
  |> should.equal(
    data_element_value.new_binary(value_representation.UnsignedShort, <<
      0x50, 0xC3,
    >>),
  )
}

pub fn new_unsigned_very_long_test() {
  let assert Ok(i0) = bigi.from_string("-1")
  let assert Ok(i1) = bigi.from_string("20000000000000000000")
  [i0, i1]
  |> list.each(fn(i) {
    [i]
    |> data_element_value.new_unsigned_very_long
    |> should.equal(
      Error(data_error.new_value_invalid(
        "Value out of range for UnsignedVeryLong VR",
      )),
    )
  })

  let assert Ok(i) = bigi.from_string("10000000000000000000")
  [i]
  |> data_element_value.new_unsigned_very_long
  |> should.equal(
    data_element_value.new_binary(value_representation.UnsignedVeryLong, <<
      0x00, 0x00, 0xE8, 0x89, 0x04, 0x23, 0xC7, 0x8A,
    >>),
  )
}
