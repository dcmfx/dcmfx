//// A data element value that can hold any of the DICOM value representations.
//// Data element values are usually stored in a `DataSet` which maps data
//// element tags to data element values.

import bigi.{type BigInt}
import dcmfx_core/code_strings
import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_element_value/age_string.{type StructuredAge}
import dcmfx_core/data_element_value/attribute_tag
import dcmfx_core/data_element_value/date
import dcmfx_core/data_element_value/date_time
import dcmfx_core/data_element_value/decimal_string
import dcmfx_core/data_element_value/integer_string
import dcmfx_core/data_element_value/person_name.{type StructuredPersonName}
import dcmfx_core/data_element_value/time
import dcmfx_core/data_element_value/unique_identifier
import dcmfx_core/data_error.{type DataError}
import dcmfx_core/dictionary
import dcmfx_core/internal/bit_array_utils
import dcmfx_core/internal/utils
import dcmfx_core/value_representation.{type ValueRepresentation}
import gleam/bit_array
import gleam/bool
import gleam/dict.{type Dict}
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/result
import gleam/string
import ieee_float.{type IEEEFloat}

/// A DICOM data element value that holds one of the following types of data:
///
/// 1. Binary value. A data element value that holds raw bytes for a specific
///    VR. This is the most common case. When the VR is a string type then the
///    bytes should be UTF-8 encoded. The data is always little endian.
///
/// 2. Lookup table descriptor value. A data element value that holds a lookup
///    table descriptor. The VR should be either `SignedShort` or
///    `UnsignedShort`, and there should be exactly six bytes. The bytes contain
///    three 16-bit integer values, the first and last of which are unsigned,
///    and the second of which is interpreted using the specified VR, i.e. it
///    can be either a signed or unsigned 16-bit integer. The data is always
///    little endian.
///
/// 3. Encapsulated pixel data value. A data element value that holds the raw
///    items for an encapsulated pixel data sequence. The VR must be either
///    `OtherByteString` or `OtherWordString`.
///
/// 4. Sequence value. A data element value that holds a sequence, which is a
///    list of nested data sets used to create hierarchies of data elements in a
///    DICOM data set.
///
/// Data element values that hold binary data always store it in a `BitArray`
/// which is parsed and converted to a more usable type on request. This
/// improves efficiency as parsing only occurs when the value of a data element
/// is requested, and allows any data to be passed through even if it is
/// non-conformant with the DICOM standard, which is a common occurrence.
///
/// Ref: PS3.5 6.2.
///
pub opaque type DataElementValue {
  BinaryValue(vr: ValueRepresentation, bytes: BitArray)
  LookupTableDescriptorValue(vr: ValueRepresentation, bytes: BitArray)
  EncapsulatedPixelDataValue(vr: ValueRepresentation, items: List(BitArray))
  SequenceValue(items: List(Dict(DataElementTag, DataElementValue)))
}

/// Formats a data element value as a human-readable single line of text. Values
/// longer than the output width are truncated with a trailing ellipsis.
///
pub fn to_string(
  value: DataElementValue,
  tag: DataElementTag,
  output_width: Int,
) -> String {
  // Maximum number of items needed in a comma-separated list of values before
  // reaching the output width
  let output_list_max_size = { output_width + 2 } / 3

  let vr_is_string = value_representation.is_string(value_representation(value))

  case value {
    BinaryValue(vr, bytes) if vr_is_string -> {
      // If the data isn't valid UTF-8 then try to ensure the data slice ends
      // exactly on a UTF-8 character boundary so that data element values
      // with partial data are still displayable
      let utf8 = case bit_array.to_string(bytes) {
        Ok(utf8) -> Ok(utf8)
        Error(Nil) ->
          bit_array_utils.reverse_index(bytes, fn(b) {
            int.bitwise_and(b, 0b1100_0000) != 0b1000_0000
          })
          |> result.then(bit_array.slice(bytes, 0, _))
          |> result.then(bit_array.to_string)
      }

      case utf8 {
        Ok(value) -> {
          let formatted_value = case vr {
            value_representation.AgeString ->
              value
              |> bit_array.from_string
              |> age_string.from_bytes
              |> result.map(age_string.to_string)
              |> result.lazy_unwrap(fn() { string.inspect(value) })

            value_representation.ApplicationEntity ->
              value
              |> utils.trim_ascii_end(0x20)
              |> string.inspect

            value_representation.Date ->
              value
              |> bit_array.from_string
              |> date.from_bytes
              |> result.map(date.to_iso8601)
              |> result.lazy_unwrap(fn() { string.inspect(value) })

            value_representation.DateTime ->
              value
              |> bit_array.from_string
              |> date_time.from_bytes
              |> result.map(date_time.to_iso8601)
              |> result.lazy_unwrap(fn() { string.inspect(value) })

            value_representation.Time ->
              value
              |> bit_array.from_string
              |> time.from_bytes
              |> result.map(time.to_iso8601)
              |> result.lazy_unwrap(fn() { string.inspect(value) })

            // Handle string VRs that allow multiplicity
            value_representation.CodeString
            | value_representation.DecimalString
            | value_representation.UniqueIdentifier
            | value_representation.IntegerString
            | value_representation.LongString
            | value_representation.ShortString
            | value_representation.UnlimitedCharacters ->
              value
              |> string.split("\\")
              |> list.map(fn(s) {
                case vr {
                  value_representation.UniqueIdentifier ->
                    utils.trim_ascii_end(s, 0x00)
                  value_representation.UnlimitedCharacters ->
                    utils.trim_ascii_end(s, 0x20)
                  _ -> utils.trim_ascii(s, 0x20)
                }
                |> string.inspect
              })
              |> string.join(", ")

            _ ->
              value
              |> utils.trim_ascii_end(0x20)
              |> string.inspect
          }

          // Add a descriptive suffix for known UIDs and CodeStrings
          let suffix = case vr {
            value_representation.UniqueIdentifier ->
              case dictionary.uid_name(utils.trim_ascii_end(value, 0x00)) {
                Ok(uid_name) -> Some(" (" <> uid_name <> ")")
                Error(Nil) -> None
              }

            value_representation.CodeString ->
              case code_strings.describe(string.trim(value), tag) {
                Ok(description) -> Some(" (" <> description <> ")")
                Error(Nil) -> None
              }

            _ -> None
          }

          Ok(#(formatted_value, suffix))
        }

        Error(Nil) -> Ok(#("!! Invalid UTF-8 data", None))
      }
    }

    LookupTableDescriptorValue(vr, bytes) | BinaryValue(vr, bytes) ->
      case vr {
        value_representation.AttributeTag ->
          case attribute_tag.from_bytes(bytes) {
            Ok(tags) -> {
              let s =
                tags
                |> list.take(output_list_max_size)
                |> list.map(data_element_tag.to_string)
                |> string.join(", ")

              Ok(#(s, None))
            }

            Error(_) -> Error(Nil)
          }

        value_representation.FloatingPointDouble
        | value_representation.FloatingPointSingle ->
          case get_floats(value) {
            Ok(floats) -> {
              let s =
                floats
                |> list.take(output_list_max_size)
                |> list.map(ieee_float.to_string)
                |> string.join(", ")

              Ok(#(s, None))
            }

            Error(_) -> Error(Nil)
          }

        value_representation.OtherByteString
        | value_representation.OtherDoubleString
        | value_representation.OtherFloatString
        | value_representation.OtherLongString
        | value_representation.OtherVeryLongString
        | value_representation.OtherWordString
        | value_representation.Unknown ->
          Ok(#(utils.inspect_bit_array(bytes, output_list_max_size), None))

        value_representation.SignedLong
        | value_representation.SignedShort
        | value_representation.UnsignedLong
        | value_representation.UnsignedShort ->
          case get_ints(value) {
            Ok(ints) -> {
              let s =
                ints
                |> list.take(output_list_max_size)
                |> list.map(int.to_string)
                |> string.join(", ")

              Ok(#(s, None))
            }

            Error(_) -> Error(Nil)
          }

        value_representation.SignedVeryLong
        | value_representation.UnsignedVeryLong ->
          case get_big_ints(value) {
            Ok(ints) -> {
              let s =
                ints
                |> list.take(output_list_max_size)
                |> list.map(bigi.to_string)
                |> string.join(", ")

              Ok(#(s, None))
            }

            Error(_) -> Error(Nil)
          }

        _ -> Error(Nil)
      }

    EncapsulatedPixelDataValue(_vr, items) -> {
      let total_bytes =
        items
        |> list.map(bit_array.byte_size)
        |> list.fold(0, int.add)

      let size = list.length(items)

      let s =
        "Items: "
        <> int.to_string(size)
        <> ", bytes: "
        <> int.to_string(total_bytes)

      Ok(#(s, None))
    }

    SequenceValue(items) -> {
      let s =
        "Items: "
        <> {
          items
          |> list.length
          |> int.to_string
        }

      Ok(#(s, None))
    }
  }
  |> result.map(fn(res) {
    let #(s, suffix) = res

    let suffix = suffix |> option.unwrap("")

    // Calculate width available for the value once the suffix isn't taken
    // into account. Always allow at least 10 characters.
    let output_width =
      int.max(output_width - utils.string_fast_length(suffix), 10)

    // Truncate string if it's too long
    case utils.string_fast_length(s) > output_width {
      True -> string.slice(s, 0, output_width - 2) <> " …" <> suffix
      False -> s <> suffix
    }
  })
  |> result.unwrap("<error converting to string>")
}

fn validate_default_charset_bytes(bytes: BitArray) -> Result(Nil, Int) {
  case bytes {
    <<b, _:bytes>>
      if b != 0x00
      && b != 0x09
      && b != 0x0A
      && b != 0x0C
      && b != 0x0D
      && b != 0x1B
      && { b < 0x20 || b > 0x7E }
    -> Error(b)

    <<_, rest:bytes>> -> validate_default_charset_bytes(rest)

    _ -> Ok(Nil)
  }
}

/// Constructs a new data element binary value with the specified value
/// representation. The only VR that's not allowed is `Sequence`. The length
/// of `bytes` must not exceed the maximum allowed for the VR, and, where
/// applicable, must also be an exact multiple of the size of the contained data
/// type. E.g. for the `UnsignedLong` VR the length of `bytes` must be a
/// multiple of 4.
///
/// When the VR is a string type, `bytes` must be UTF-8 encoded in order for the
/// value to be readable.
///
pub fn new_binary(
  vr: ValueRepresentation,
  bytes: BitArray,
) -> Result(DataElementValue, DataError) {
  let vr_validation = case vr {
    value_representation.Sequence ->
      Error(data_error.new_value_invalid(
        "Value representation '"
        <> value_representation.to_string(vr)
        <> "' is not valid for binary data",
      ))
    _ -> Ok(BinaryValue(vr, bytes))
  }

  // Report any error in VR validation
  use _ <- result.try(vr_validation)

  // Search the string for any disallowed codepoints
  let string_validation = case value_representation.is_encoded_string(vr) {
    True ->
      case bit_array.is_utf8(bytes) {
        True -> Ok(Nil)
        False ->
          Error(data_error.new_value_invalid(
            "Bytes for '"
            <> value_representation.to_string(vr)
            <> "' are not valid UTF-8",
          ))
      }

    False ->
      case value_representation.is_string(vr) {
        True ->
          case validate_default_charset_bytes(bytes) {
            Ok(Nil) -> Ok(Nil)
            Error(invalid_byte) -> {
              let invalid_byte =
                invalid_byte
                |> int.to_base16
                |> utils.pad_start(2, "0")

              Error(data_error.new_value_invalid(
                "Bytes for '"
                <> value_representation.to_string(vr)
                <> "' has disallowed byte: 0x"
                <> invalid_byte,
              ))
            }
          }

        False -> Ok(Nil)
      }
  }

  // Report any error in string validation
  use _ <- result.try(string_validation)

  let value = new_binary_unchecked(vr, bytes)

  validate_length(value)
}

/// Constructs a new data element binary value similar to `new_binary`,
/// but does not validate `vr` or `bytes`.
///
pub fn new_binary_unchecked(
  vr: ValueRepresentation,
  bytes: BitArray,
) -> DataElementValue {
  BinaryValue(vr, bytes)
}

/// Constructs a new data element lookup table descriptor value with the
/// specified `vr`, which must be one of the following:
///
/// - `SignedShort`
/// - `UnsignedShort`
///
/// The length of `bytes` must be exactly six.
///
pub fn new_lookup_table_descriptor(
  vr: ValueRepresentation,
  bytes: BitArray,
) -> Result(DataElementValue, DataError) {
  let vr_validation = case vr {
    value_representation.SignedShort | value_representation.UnsignedShort ->
      Ok(Nil)
    _ ->
      Error(data_error.new_value_invalid(
        "Value representation '"
        <> value_representation.to_string(vr)
        <> "' is not valid for lookup table descriptor data",
      ))
  }
  use _ <- result.try(vr_validation)

  let value = new_lookup_table_descriptor_unchecked(vr, bytes)

  validate_length(value)
}

/// Constructs a new data element lookup table descriptor value similar to
/// `new_lookup_table_descriptor_value`, but does not validate `vr` or `bytes`.
///
pub fn new_lookup_table_descriptor_unchecked(
  vr: ValueRepresentation,
  bytes: BitArray,
) -> DataElementValue {
  LookupTableDescriptorValue(vr, bytes)
}

/// Constructs a new data element encapsulated pixel data value with the
/// specified `vr`, which must be one of the following:
///
/// - `OtherByteString`
/// - `OtherWordString`
///
/// Although the DICOM standard states that only `OtherByteString` is valid for
/// encapsulated pixel data, in practice this is not always followed.
///
/// `items` specifies the data of the encapsulated pixel data items, where the
/// first item is an optional basic offset table, and is followed by fragments
/// of pixel data. Each item must be of even length. Ref: PS3.5 A.4.
///
pub fn new_encapsulated_pixel_data(
  vr: ValueRepresentation,
  items: List(BitArray),
) -> Result(DataElementValue, DataError) {
  let vr_validation = case vr {
    value_representation.OtherByteString | value_representation.OtherWordString ->
      Ok(Nil)
    _ ->
      Error(data_error.new_value_invalid(
        "Value representation '"
        <> value_representation.to_string(vr)
        <> "' is not valid for encapsulated pixel data",
      ))
  }
  use _ <- result.try(vr_validation)

  let value = new_encapsulated_pixel_data_unchecked(vr, items)

  validate_length(value)
}

/// Constructs a new data element string value similar to
/// `new_encapsulated_pixel_data`, but does not validate `vr` or `items`.
///
pub fn new_encapsulated_pixel_data_unchecked(
  vr: ValueRepresentation,
  items: List(BitArray),
) -> DataElementValue {
  EncapsulatedPixelDataValue(vr, items)
}

/// Creates a new `AgeString` data element value.
///
pub fn new_age_string(
  value: StructuredAge,
) -> Result(DataElementValue, DataError) {
  value
  |> age_string.to_bytes
  |> result.map(new_binary_unchecked(value_representation.AgeString, _))
}

/// Creates a new `ApplicationEntity` data element value.
///
pub fn new_application_entity(
  value: String,
) -> Result(DataElementValue, DataError) {
  [value]
  |> list.map(utils.trim_ascii_end(_, 0x20))
  |> new_string_list(value_representation.ApplicationEntity, _)
}

/// Creates a new `AttributeTag` data element value.
///
pub fn new_attribute_tag(
  value: List(DataElementTag),
) -> Result(DataElementValue, DataError) {
  value
  |> attribute_tag.to_bytes
  |> result.then(new_binary(value_representation.AttributeTag, _))
}

/// Creates a new `CodeString` data element value.
///
pub fn new_code_string(
  value: List(String),
) -> Result(DataElementValue, DataError) {
  value
  |> list.map(utils.trim_ascii(_, 0x20))
  |> new_string_list(value_representation.CodeString, _)
}

/// Creates a new `Date` data element value.
///
pub fn new_date(
  value: date.StructuredDate,
) -> Result(DataElementValue, DataError) {
  value
  |> date.to_bytes
  |> result.map(new_binary_unchecked(value_representation.Date, _))
}

/// Creates a new `DateTime` data element value.
///
pub fn new_date_time(
  value: date_time.StructuredDateTime,
) -> Result(DataElementValue, DataError) {
  value
  |> date_time.to_bytes
  |> result.map(new_binary_unchecked(value_representation.DateTime, _))
}

/// Creates a new `DecimalString` data element value.
///
pub fn new_decimal_string(
  value: List(Float),
) -> Result(DataElementValue, DataError) {
  value
  |> decimal_string.to_bytes
  |> new_binary(value_representation.DecimalString, _)
}

/// Creates a new `FloatingPointDouble` data element value.
///
pub fn new_floating_point_double(
  value: List(IEEEFloat),
) -> Result(DataElementValue, DataError) {
  value
  |> list.map(ieee_float.to_bytes_64_le)
  |> bit_array.concat
  |> new_binary(value_representation.FloatingPointDouble, _)
}

/// Creates a new `FloatingPointSingle` data element value.
///
pub fn new_floating_point_single(
  value: List(IEEEFloat),
) -> Result(DataElementValue, DataError) {
  value
  |> list.map(ieee_float.to_bytes_32_le)
  |> bit_array.concat
  |> new_binary(value_representation.FloatingPointSingle, _)
}

/// Creates a new `IntegerString` data element value.
///
pub fn new_integer_string(
  value: List(Int),
) -> Result(DataElementValue, DataError) {
  value
  |> integer_string.to_bytes
  |> result.then(new_binary(value_representation.IntegerString, _))
}

/// Creates a new `LongString` data element value.
///
pub fn new_long_string(
  value: List(String),
) -> Result(DataElementValue, DataError) {
  value
  |> list.map(utils.trim_ascii(_, 0x20))
  |> new_string_list(value_representation.LongString, _)
}

/// Creates a new `LongText` data element value.
///
pub fn new_long_text(value: String) -> Result(DataElementValue, DataError) {
  value
  |> utils.trim_ascii_end(0x20)
  |> bit_array.from_string
  |> value_representation.pad_bytes_to_even_length(
    value_representation.LongText,
    _,
  )
  |> new_binary(value_representation.LongText, _)
}

/// Creates a new `OtherByteString` data element value.
///
pub fn new_other_byte_string(
  value: BitArray,
) -> Result(DataElementValue, DataError) {
  new_binary(value_representation.OtherByteString, value)
}

/// Creates a new `OtherDoubleString` data element value.
///
pub fn new_other_double_string(
  value: List(IEEEFloat),
) -> Result(DataElementValue, DataError) {
  value
  |> list.map(ieee_float.to_bytes_64_le)
  |> bit_array.concat
  |> new_binary(value_representation.OtherDoubleString, _)
}

/// Creates a new `OtherFloatString` data element value.
///
pub fn new_other_float_string(
  value: List(IEEEFloat),
) -> Result(DataElementValue, DataError) {
  value
  |> list.map(ieee_float.to_bytes_32_le)
  |> bit_array.concat
  |> new_binary(value_representation.OtherFloatString, _)
}

/// Creates a new `OtherLongString` data element value.
///
pub fn new_other_long_string(
  value: BitArray,
) -> Result(DataElementValue, DataError) {
  new_binary(value_representation.OtherLongString, value)
}

/// Creates a new `OtherVeryLongString` data element value.
///
pub fn new_other_very_long_string(
  value: BitArray,
) -> Result(DataElementValue, DataError) {
  new_binary(value_representation.OtherVeryLongString, value)
}

/// Creates a new `OtherWordString` data element value.
///
pub fn new_other_word_string(
  value: BitArray,
) -> Result(DataElementValue, DataError) {
  new_binary(value_representation.OtherWordString, value)
}

/// Creates a new `PersonName` data element value.
///
pub fn new_person_name(
  value: List(StructuredPersonName),
) -> Result(DataElementValue, DataError) {
  value
  |> person_name.to_bytes
  |> result.then(new_binary(value_representation.PersonName, _))
}

/// Creates a new `Sequence` data element value.
///
pub fn new_sequence(
  items: List(Dict(DataElementTag, DataElementValue)),
) -> DataElementValue {
  SequenceValue(items)
}

/// Creates a new `ShortString` data element value.
///
pub fn new_short_string(
  value: List(String),
) -> Result(DataElementValue, DataError) {
  value
  |> list.map(utils.trim_ascii(_, 0x20))
  |> new_string_list(value_representation.ShortString, _)
}

/// Creates a new `ShortText` data element value.
///
pub fn new_short_text(value: String) -> Result(DataElementValue, DataError) {
  value
  |> utils.trim_ascii_end(0x20)
  |> bit_array.from_string
  |> value_representation.pad_bytes_to_even_length(
    value_representation.ShortText,
    _,
  )
  |> new_binary(value_representation.ShortText, _)
}

/// Creates a new `SignedLong` data element value.
///
pub fn new_signed_long(value: List(Int)) -> Result(DataElementValue, DataError) {
  let is_valid =
    list.all(value, fn(i) { i >= { -1 * 0x80000000 } && i <= 0x7FFFFFFF })

  use <- bool.guard(
    !is_valid,
    Error(data_error.new_value_invalid("Value out of range for SignedLong VR")),
  )

  value
  |> list.map(fn(x) { <<x:32-little>> })
  |> bit_array.concat
  |> new_binary(value_representation.SignedLong, _)
}

/// Creates a new `SignedShort` data element value.
///
pub fn new_signed_short(value: List(Int)) -> Result(DataElementValue, DataError) {
  let is_valid = list.all(value, fn(i) { i >= { -1 * 0x8000 } && i <= 0x7FFF })

  use <- bool.guard(
    !is_valid,
    Error(data_error.new_value_invalid("Value out of range for SignedShort VR")),
  )

  value
  |> list.map(fn(x) { <<x:16-little>> })
  |> bit_array.concat
  |> new_binary(value_representation.SignedShort, _)
}

/// Creates a new `SignedVeryLong` data element value.
///
pub fn new_signed_very_long(
  value: List(BigInt),
) -> Result(DataElementValue, DataError) {
  value
  |> list.map(bigi.to_bytes(_, bigi.LittleEndian, bigi.Signed, 8))
  |> result.all
  |> result.map_error(fn(_) {
    data_error.new_value_invalid("Value out of range for SignedVeryLong VR")
  })
  |> result.map(bit_array.concat)
  |> result.then(new_binary(value_representation.SignedVeryLong, _))
}

/// Creates a new `Time` data element value.
///
pub fn new_time(
  value: time.StructuredTime,
) -> Result(DataElementValue, DataError) {
  value
  |> time.to_bytes
  |> result.map(new_binary_unchecked(value_representation.Time, _))
}

/// Creates a new `UniqueIdentifier` data element value.
///
pub fn new_unique_identifier(
  value: List(String),
) -> Result(DataElementValue, DataError) {
  value
  |> unique_identifier.to_bytes
  |> result.then(new_binary(value_representation.UniqueIdentifier, _))
}

/// Creates a new `UniversalResourceIdentifier` data element value.
///
pub fn new_universal_resource_identifier(
  value: String,
) -> Result(DataElementValue, DataError) {
  value
  |> utils.trim_ascii(0x20)
  |> bit_array.from_string
  |> value_representation.pad_bytes_to_even_length(
    value_representation.UniversalResourceIdentifier,
    _,
  )
  |> new_binary(value_representation.UniversalResourceIdentifier, _)
}

/// Creates a new `Unknown` data element value.
///
pub fn new_unknown(value: BitArray) -> Result(DataElementValue, DataError) {
  new_binary(value_representation.Unknown, value)
}

/// Creates a new `UnlimitedCharacters` data element value.
///
pub fn new_unlimited_characters(
  value: List(String),
) -> Result(DataElementValue, DataError) {
  value
  |> list.map(utils.trim_ascii_end(_, 0x20))
  |> new_string_list(value_representation.UnlimitedCharacters, _)
}

/// Creates a new `UnlimitedText` data element value.
///
pub fn new_unlimited_text(value: String) -> Result(DataElementValue, DataError) {
  value
  |> utils.trim_ascii_end(0x20)
  |> bit_array.from_string
  |> value_representation.pad_bytes_to_even_length(
    value_representation.UnlimitedText,
    _,
  )
  |> new_binary(value_representation.UnlimitedText, _)
}

/// Creates a new `UnsignedLong` data element value.
///
pub fn new_unsigned_long(
  value: List(Int),
) -> Result(DataElementValue, DataError) {
  let is_valid = list.all(value, fn(i) { i >= 0 && i <= 0xFFFFFFFF })

  use <- bool.guard(
    !is_valid,
    Error(data_error.new_value_invalid("Value out of range for UnsignedLong VR")),
  )

  value
  |> list.map(fn(x) { <<x:32-little>> })
  |> bit_array.concat
  |> new_binary(value_representation.UnsignedLong, _)
}

/// Creates a new `UnsignedShort` data element value.
///
pub fn new_unsigned_short(
  value: List(Int),
) -> Result(DataElementValue, DataError) {
  let is_valid = list.all(value, fn(i) { i >= 0 && i <= 0xFFFF })

  use <- bool.guard(
    !is_valid,
    Error(data_error.new_value_invalid(
      "Value out of range for UnsignedShort VR",
    )),
  )

  value
  |> list.map(fn(x) { <<x:16-little>> })
  |> bit_array.concat
  |> new_binary(value_representation.UnsignedShort, _)
}

/// Creates a new `UnsignedVeryLong` data element value.
///
pub fn new_unsigned_very_long(
  value: List(BigInt),
) -> Result(DataElementValue, DataError) {
  value
  |> list.map(bigi.to_bytes(_, bigi.LittleEndian, bigi.Unsigned, 8))
  |> result.all
  |> result.map_error(fn(_) {
    data_error.new_value_invalid("Value out of range for UnsignedVeryLong VR")
  })
  |> result.map(bit_array.concat)
  |> result.then(new_binary(value_representation.UnsignedVeryLong, _))
}

/// Returns the value representation for a data element value.
///
pub fn value_representation(value: DataElementValue) -> ValueRepresentation {
  case value {
    BinaryValue(vr, _)
    | LookupTableDescriptorValue(vr, _)
    | EncapsulatedPixelDataValue(vr, _) -> vr
    SequenceValue(..) -> value_representation.Sequence
  }
}

/// For data element values that hold binary data, returns that data.
///
pub fn bytes(value: DataElementValue) -> Result(BitArray, DataError) {
  case value {
    BinaryValue(_, bytes) | LookupTableDescriptorValue(_, bytes) -> Ok(bytes)
    _ -> Error(data_error.new_value_not_present())
  }
}

/// For data element values that hold encapsulated pixel data, returns a
/// reference to the encapsulated items.
///
pub fn encapsulated_pixel_data(
  value: DataElementValue,
) -> Result(List(BitArray), DataError) {
  case value {
    EncapsulatedPixelDataValue(_, items) -> Ok(items)
    _ -> Error(data_error.new_value_not_present())
  }
}

/// For data element values that hold a sequence, returns a reference to the
/// sequence's items.
///
pub fn sequence_items(
  value: DataElementValue,
) -> Result(List(Dict(DataElementTag, DataElementValue)), DataError) {
  case value {
    SequenceValue(items) -> Ok(items)
    _ -> Error(data_error.new_value_not_present())
  }
}

/// Returns the size in bytes of a data element value. This recurses through
/// sequences and also includes a fixed per-value overhead, so never returns
/// zero even for an empty data element value.
///
pub fn total_byte_size(value: DataElementValue) -> Int {
  let data_size = case value {
    BinaryValue(bytes:, ..) | LookupTableDescriptorValue(bytes:, ..) ->
      bit_array.byte_size(bytes)

    EncapsulatedPixelDataValue(items:, ..) -> {
      { list.length(items) * 8 }
      + list.fold(items, 0, fn(total, item) {
        total + bit_array.byte_size(item)
      })
    }

    SequenceValue(items:) -> {
      items
      |> list.map(fn(item) {
        item
        |> dict.fold(0, fn(total, _tag, value) {
          total + total_byte_size(value)
        })
      })
      |> int.sum()
    }
  }

  // This is just a rough estimate
  let fixed_size = 32

  data_size + fixed_size
}

/// Returns the string contained in a data element value. This is only supported
/// for value representations that either don't allow multiplicity, or those
/// that do allow multiplicity but only one string is present in the value.
///
pub fn get_string(value: DataElementValue) -> Result(String, DataError) {
  case value {
    BinaryValue(value_representation.ApplicationEntity, bytes)
    | BinaryValue(value_representation.LongText, bytes)
    | BinaryValue(value_representation.ShortText, bytes)
    | BinaryValue(value_representation.UniversalResourceIdentifier, bytes)
    | BinaryValue(value_representation.UnlimitedText, bytes) ->
      bytes
      |> bit_array.to_string
      |> result.map_error(fn(_) {
        data_error.new_value_invalid("String bytes are not valid UTF-8")
      })
      |> result.map(fn(s) {
        case value_representation(value) {
          value_representation.ApplicationEntity
          | value_representation.UniversalResourceIdentifier ->
            utils.trim_ascii(s, 0x20)
          _ -> utils.trim_ascii_end(s, 0x20)
        }
      })

    _ -> {
      use strings <- result.try(get_strings(value))

      case strings {
        [s] -> Ok(s)
        _ -> Error(data_error.new_multiplicity_mismatch())
      }
    }
  }
}

/// Returns the strings contained in a data element value. This is only
/// supported for value representations that allow multiplicity.
///
pub fn get_strings(value: DataElementValue) -> Result(List(String), DataError) {
  case value {
    BinaryValue(value_representation.CodeString, bytes)
    | BinaryValue(value_representation.UniqueIdentifier, bytes)
    | BinaryValue(value_representation.LongString, bytes)
    | BinaryValue(value_representation.ShortString, bytes)
    | BinaryValue(value_representation.UnlimitedCharacters, bytes) ->
      bytes
      |> bit_array.to_string
      |> result.map_error(fn(_) {
        data_error.new_value_invalid("String bytes are not valid UTF-8")
      })
      |> result.map(string.split(_, "\\"))
      |> result.map(
        list.map(_, fn(s) {
          case value_representation(value) {
            value_representation.UniqueIdentifier ->
              utils.trim_ascii_end(s, 0x00)
            value_representation.UnlimitedCharacters ->
              utils.trim_ascii_end(s, 0x20)
            _ -> utils.trim_ascii(s, 0x20)
          }
        }),
      )

    _ -> Error(data_error.new_value_not_present())
  }
}

/// Returns the integer contained in a data element value. This is only
/// supported for value representations that contain integer data and when
/// exactly one integer is present.
///
pub fn get_int(value: DataElementValue) -> Result(Int, DataError) {
  use ints <- result.try(get_ints(value))

  case ints {
    [i] -> Ok(i)
    _ -> Error(data_error.new_multiplicity_mismatch())
  }
}

/// Returns the integers contained in a data element value. This is only
/// supported for value representations that contain integer data.
///
pub fn get_ints(value: DataElementValue) -> Result(List(Int), DataError) {
  case value {
    BinaryValue(value_representation.IntegerString, bytes) ->
      integer_string.from_bytes(bytes)

    BinaryValue(value_representation.SignedLong, bytes) ->
      bit_array_utils.to_int32_list(bytes)
      |> result.replace_error(data_error.new_value_invalid("Invalid Int32 data"))

    BinaryValue(value_representation.SignedShort, bytes) ->
      bit_array_utils.to_int16_list(bytes)
      |> result.replace_error(data_error.new_value_invalid("Invalid Int16 data"))

    BinaryValue(value_representation.UnsignedLong, bytes) ->
      bit_array_utils.to_uint32_list(bytes)
      |> result.replace_error(data_error.new_value_invalid(
        "Invalid Uint32 data",
      ))

    BinaryValue(value_representation.UnsignedShort, bytes) ->
      bit_array_utils.to_uint16_list(bytes)
      |> result.replace_error(data_error.new_value_invalid(
        "Invalid Uint16 data",
      ))

    LookupTableDescriptorValue(..) -> {
      use #(entry_count, first_input_value, bits_per_entry) <- result.map(
        get_lookup_table_descriptor(value),
      )

      [entry_count, first_input_value, bits_per_entry]
    }

    _ -> Error(data_error.new_value_not_present())
  }
}

/// Returns the three integers contained in a lookup table descriptor data
/// element value.
///
pub fn get_lookup_table_descriptor(
  value: DataElementValue,
) -> Result(#(Int, Int, Int), DataError) {
  case value {
    LookupTableDescriptorValue(vr, bytes) ->
      // Use the lookup table descriptor value's VR to determine how to
      // interpret the second 16-bit integer it contains
      case vr, bytes {
        value_representation.SignedShort,
          <<
            entry_count:16-unsigned-little,
            first_input_value:16-signed-little,
            bits_per_entry:16-unsigned-little,
          >>
        | value_representation.UnsignedShort,
          <<
            entry_count:16-unsigned-little,
            first_input_value:16-unsigned-little,
            bits_per_entry:16-unsigned-little,
          >>
        -> Ok(#(entry_count, first_input_value, bits_per_entry))

        _, _ ->
          Error(data_error.new_value_invalid("Invalid lookup table descriptor"))
      }

    _ -> Error(data_error.new_value_not_present())
  }
}

/// Returns the big integer contained in a data element value. This is only
/// supported for value representations that contain big integer data and when
/// exactly one big integer is present.
///
pub fn get_big_int(value: DataElementValue) -> Result(BigInt, DataError) {
  use ints <- result.try(get_big_ints(value))

  case ints {
    [i] -> Ok(i)
    _ -> Error(data_error.new_multiplicity_mismatch())
  }
}

/// Returns the big integers contained in a data element value. This is only
/// supported for value representations that contain big integer data.
///
pub fn get_big_ints(value: DataElementValue) -> Result(List(BigInt), DataError) {
  case value {
    BinaryValue(value_representation.SignedVeryLong, bytes) ->
      bit_array_utils.to_int64_list(bytes)
      |> result.replace_error(data_error.new_value_invalid("Invalid Int64 data"))

    BinaryValue(value_representation.UnsignedVeryLong, bytes) ->
      bytes
      |> bit_array_utils.to_uint64_list
      |> result.replace_error(data_error.new_value_invalid(
        "Invalid Uint64 data",
      ))

    _ -> Error(data_error.new_value_not_present())
  }
}

/// Returns the float contained in a data element value. This is only supported
/// for value representations that contain floating point data and when exactly
/// one float is present.
///
pub fn get_float(value: DataElementValue) -> Result(IEEEFloat, DataError) {
  use floats <- result.try(get_floats(value))

  case floats {
    [f] -> Ok(f)
    _ -> Error(data_error.new_multiplicity_mismatch())
  }
}

/// Returns the floats contained in a data element value. This is only supported
/// for value representations containing floating point data.
///
pub fn get_floats(value: DataElementValue) -> Result(List(IEEEFloat), DataError) {
  case value {
    BinaryValue(value_representation.DecimalString, bytes) ->
      bytes
      |> decimal_string.from_bytes
      |> result.map(list.map(_, ieee_float.finite))

    BinaryValue(value_representation.FloatingPointDouble, bytes)
    | BinaryValue(value_representation.OtherDoubleString, bytes) ->
      bit_array_utils.to_float64_list(bytes)
      |> result.replace_error(data_error.new_value_invalid(
        "Invalid Float64 data",
      ))

    BinaryValue(value_representation.FloatingPointSingle, bytes)
    | BinaryValue(value_representation.OtherFloatString, bytes) ->
      bit_array_utils.to_float32_list(bytes)
      |> result.replace_error(data_error.new_value_invalid(
        "Invalid Float32 data",
      ))

    _ -> Error(data_error.new_value_not_present())
  }
}

/// Returns the structured age contained in a data element value. This is only
/// supported for the `AgeString` value representation.
///
pub fn get_age(
  value: DataElementValue,
) -> Result(age_string.StructuredAge, DataError) {
  case value {
    BinaryValue(value_representation.AgeString, bytes) ->
      age_string.from_bytes(bytes)
    _ -> Error(data_error.new_value_not_present())
  }
}

/// Returns the data element tags contained in a data element value. This is
/// only supported for the `AttributeTag` value representation.
///
pub fn get_attribute_tags(
  value: DataElementValue,
) -> Result(List(DataElementTag), DataError) {
  case value {
    BinaryValue(value_representation.AttributeTag, bytes) ->
      attribute_tag.from_bytes(bytes)
    _ -> Error(data_error.new_value_not_present())
  }
}

/// Returns the structured date contained in a data element value. This is only
/// supported for the `Date` value representation.
///
pub fn get_date(
  value: DataElementValue,
) -> Result(date.StructuredDate, DataError) {
  case value {
    BinaryValue(value_representation.Date, bytes) -> date.from_bytes(bytes)
    _ -> Error(data_error.new_value_not_present())
  }
}

/// Returns the structured date/time contained in a data element value. This is
/// only supported for the `DateTime` value representation.
///
pub fn get_date_time(
  value: DataElementValue,
) -> Result(date_time.StructuredDateTime, DataError) {
  case value {
    BinaryValue(value_representation.DateTime, bytes) ->
      date_time.from_bytes(bytes)
    _ -> Error(data_error.new_value_not_present())
  }
}

/// Returns the structured time contained in a data element value. This is only
/// supported for the `Time` value representation.
///
pub fn get_time(
  value: DataElementValue,
) -> Result(time.StructuredTime, DataError) {
  case value {
    BinaryValue(value_representation.Time, bytes) -> time.from_bytes(bytes)
    _ -> Error(data_error.new_value_not_present())
  }
}

/// Returns the structured person name contained in a data element value. This
/// is only supported for the `PersonName` value representation when exactly one
/// person name is present.
///
pub fn get_person_name(
  value: DataElementValue,
) -> Result(person_name.StructuredPersonName, DataError) {
  use person_names <- result.try(get_person_names(value))

  case person_names {
    [n] -> Ok(n)
    _ -> Error(data_error.new_multiplicity_mismatch())
  }
}

/// Returns the structured person names contained in a data element value. This
/// is only supported for the `PersonName` value representation.
///
pub fn get_person_names(
  value: DataElementValue,
) -> Result(List(person_name.StructuredPersonName), DataError) {
  case value {
    BinaryValue(value_representation.PersonName, bytes) ->
      person_name.from_bytes(bytes)
    _ -> Error(data_error.new_value_not_present())
  }
}

/// Checks that the number of bytes stored in a data element value is valid for
/// its value representation.
///
pub fn validate_length(
  value: DataElementValue,
) -> Result(DataElementValue, DataError) {
  let value_length = bytes(value) |> result.unwrap(<<>>) |> bit_array.byte_size

  case value {
    LookupTableDescriptorValue(vr, ..) ->
      case value_length {
        6 -> Ok(value)
        _ ->
          Error(data_error.new_value_length_invalid(
            vr,
            value_length,
            "Lookup table descriptor length must be exactly 6 bytes",
          ))
      }

    BinaryValue(vr, ..) -> {
      let value_representation.LengthRequirements(
        bytes_max,
        bytes_multiple_of,
        _,
      ) = value_representation.length_requirements(vr)

      let bytes_multiple_of = bytes_multiple_of |> option.unwrap(2)

      // Check against the length requirements for this VR
      case value_length > bytes_max {
        True ->
          Error(data_error.new_value_length_invalid(
            vr,
            value_length,
            "Must not exceed " <> int.to_string(bytes_max) <> " bytes",
          ))
        False ->
          case value_length % bytes_multiple_of {
            0 -> Ok(value)
            _ ->
              Error(data_error.new_value_length_invalid(
                vr,
                value_length,
                "Must be a multiple of "
                  <> int.to_string(bytes_multiple_of)
                  <> " bytes",
              ))
          }
      }
    }

    EncapsulatedPixelDataValue(vr, items) ->
      items
      |> list.try_each(fn(item) {
        let item_length = bit_array.byte_size(item)

        case item_length > 0xFFFFFFFE {
          True ->
            Error(data_error.new_value_length_invalid(
              vr,
              item_length,
              "Must not exceed " <> int.to_string(0xFFFFFFFE) <> " bytes",
            ))
          False ->
            case item_length % 2 {
              0 -> Ok(value)
              _ ->
                Error(data_error.new_value_length_invalid(
                  vr,
                  item_length,
                  "Must be a multiple of 2 bytes",
                ))
            }
        }
      })
      |> result.replace(value)

    SequenceValue(..) -> Ok(value)
  }
}

/// Creates a data element containing a multi-valued string. This checks that
/// the individual values are valid and then combines them into final bytes.
///
fn new_string_list(
  vr: ValueRepresentation,
  value: List(String),
) -> Result(DataElementValue, DataError) {
  let string_characters_max =
    option.unwrap(
      value_representation.length_requirements(vr).string_characters_max,
      0xFFFFFFFE,
    )

  // Check no values exceed the max length or contain backslashes that would
  // affect the multiplicity once joined together
  let value_validation =
    list.try_fold(value, Nil, fn(_, s) {
      case string.length(s) > string_characters_max {
        True ->
          Error(data_error.new_value_invalid(
            "String list item is longer than the max length of "
            <> int.to_string(string_characters_max),
          ))
        False ->
          case string.contains(s, "\\") {
            True ->
              Error(data_error.new_value_invalid(
                "String list item contains backslashes",
              ))
            False -> Ok(Nil)
          }
      }
    })

  use _ <- result.try(value_validation)

  value
  |> string.join("\\")
  |> bit_array.from_string
  |> value_representation.pad_bytes_to_even_length(vr, _)
  |> new_binary(vr, _)
}
