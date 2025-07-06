import bigi
import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_element_value.{type DataElementValue}
import dcmfx_core/data_element_value/decimal_string
import dcmfx_core/data_element_value/integer_string
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/data_set_path.{type DataSetPath}
import dcmfx_core/dictionary
import dcmfx_core/internal/bit_array_utils
import dcmfx_core/internal/utils
import dcmfx_core/transfer_syntax.{type TransferSyntax}
import dcmfx_core/value_representation.{type ValueRepresentation}
import dcmfx_json/json_error.{type JsonDeserializeError}
import gleam/bit_array
import gleam/bool
import gleam/dict.{type Dict}
import gleam/dynamic/decode.{type Dynamic}
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result
import gleam/string
import ieee_float.{type IEEEFloat}

/// Converts DICOM JSON into a data set. This is used to read the root data set
/// and also recursively when reading sequences.
///
pub fn convert_json_to_data_set(
  in: Dynamic,
  path: DataSetPath,
) -> Result(DataSet, JsonDeserializeError) {
  let raw_dict =
    in
    |> decode.run(decode.dict(decode.string, decode.dynamic))
    |> result.replace_error(json_error.JsonInvalid(
      "Data set is not an object",
      path,
    ))
  use raw_dict <- result.try(raw_dict)

  raw_dict
  |> dict.fold(Ok(#(data_set.new(), None)), fn(context, raw_tag, raw_value) {
    use context <- result.try(context)

    let #(data_set, transfer_syntax) = context

    // Parse the data element tag
    let tag =
      raw_tag
      |> data_element_tag.from_hex_string
      |> result.replace_error(json_error.JsonInvalid(
        "Invalid data set tag: " <> raw_tag,
        path,
      ))
    use tag <- result.try(tag)

    let assert Ok(path) = data_set_path.add_data_element(path, tag)

    // Parse the data element value
    let value =
      convert_json_to_data_element(raw_value, tag, transfer_syntax, path)
    use value <- result.map(value)

    // Add data element to the final result
    let data_set = data_set.insert(data_set, tag, value)

    // Look up the transfer syntax if this is the relevant tag
    let transfer_syntax = case tag == dictionary.transfer_syntax_uid.tag {
      True ->
        case data_set.get_transfer_syntax(data_set) {
          Ok(ts) -> Some(ts)
          _ -> None
        }
      False -> transfer_syntax
    }

    #(data_set, transfer_syntax)
  })
  |> result.map(fn(x) { x.0 })
}

/// Reads the value of a single item in DICOM JSON as a native data element
/// value.
///
fn convert_json_to_data_element(
  in: Dynamic,
  tag: DataElementTag,
  transfer_syntax: Option(TransferSyntax),
  path: DataSetPath,
) -> Result(DataElementValue, JsonDeserializeError) {
  let raw_value =
    in
    |> decode.run(decode.dict(decode.string, decode.dynamic))
    |> result.replace_error(json_error.JsonInvalid(
      "Data element is not an object",
      path,
    ))
  use raw_value <- result.try(raw_value)

  // Read the VR for this value
  use vr <- result.try(read_dicom_json_vr(raw_value, path))

  // To read the data element value, first look for a "Value" property, then
  // look for an "InlineBinary" property, then finally look for a "BulkDataURI"
  // property (which is not supported and generates an error)
  case dict.get(raw_value, "Value") {
    Ok(value) -> read_dicom_json_primitive_value(tag, vr, value, path)

    Error(Nil) ->
      case dict.get(raw_value, "InlineBinary") {
        Ok(inline_binary) ->
          read_dicom_json_inline_binary_value(
            inline_binary,
            tag,
            vr,
            transfer_syntax,
            path,
          )

        Error(Nil) ->
          case dict.get(raw_value, "BulkDataURI") {
            Ok(_) ->
              Error(json_error.JsonInvalid(
                "DICOM JSON BulkDataURI values are not supported",
                path,
              ))

            // No value is present, so fall back to an empty value
            _ ->
              case vr {
                value_representation.Sequence ->
                  data_element_value.new_sequence([])
                _ -> data_element_value.new_binary_unchecked(vr, <<>>)
              }
              |> Ok
          }
      }
  }
}

/// Reads a native value representation from a DICOM JSON "vr" property.
///
fn read_dicom_json_vr(
  raw_value: Dict(String, Dynamic),
  path: DataSetPath,
) -> Result(ValueRepresentation, JsonDeserializeError) {
  // Read the VR
  let raw_vr =
    raw_value
    |> dict.get("vr")
    |> result.replace_error(json_error.JsonInvalid(
      "Data element value has no VR",
      path,
    ))
  use raw_vr <- result.try(raw_vr)

  // Get the VR string value
  let vr_string =
    raw_vr
    |> decode.run(decode.string)
    |> result.replace_error(json_error.JsonInvalid("VR is not a string", path))
  use vr_string <- result.try(vr_string)

  // Convert to a native VR
  vr_string
  |> bit_array.from_string
  |> value_representation.from_bytes
  |> result.replace_error(json_error.JsonInvalid(
    "VR is invalid: " <> vr_string,
    path,
  ))
}

/// Reads a data element value from a DICOM JSON "Value" property.
///
fn read_dicom_json_primitive_value(
  tag: DataElementTag,
  vr: ValueRepresentation,
  value: Dynamic,
  path: DataSetPath,
) -> Result(DataElementValue, JsonDeserializeError) {
  case vr {
    value_representation.AgeString
    | value_representation.ApplicationEntity
    | value_representation.CodeString
    | value_representation.Date
    | value_representation.DateTime
    | value_representation.LongString
    | value_representation.LongText
    | value_representation.ShortString
    | value_representation.ShortText
    | value_representation.Time
    | value_representation.UnlimitedCharacters
    | value_representation.UnlimitedText
    | value_representation.UniqueIdentifier
    | value_representation.UniversalResourceIdentifier -> {
      let value =
        value
        |> decode.run(decode.list(decode.optional(decode.string)))
        |> result.map_error(fn(_) {
          json_error.JsonInvalid("String value is invalid", path)
        })
      use value <- result.map(value)

      value
      |> list.map(option.unwrap(_, ""))
      |> string.join("\\")
      |> bit_array.from_string
      |> value_representation.pad_bytes_to_even_length(vr, _)
      |> data_element_value.new_binary_unchecked(vr, _)
    }

    value_representation.DecimalString ->
      value
      |> decode.run(decode.list(decode.dynamic))
      |> result.try(fn(lst) {
        list.map(lst, fn(i) {
          case decode.run(i, decode.float) {
            Ok(i) -> Ok(i)
            Error(_) ->
              case decode.run(i, decode.int) {
                Ok(i) -> Ok(int.to_float(i))
                Error(e) -> Error(e)
              }
          }
        })
        |> result.all
      })
      |> result.replace_error(json_error.JsonInvalid(
        "DecimalString value is invalid",
        path,
      ))
      |> result.map(decimal_string.to_bytes)
      |> result.map(data_element_value.new_binary_unchecked(vr, _))

    value_representation.IntegerString -> {
      let ints =
        value
        |> decode.run(decode.list(decode.int))
        |> result.replace_error(json_error.JsonInvalid(
          "IntegerString value is invalid",
          path,
        ))
      use ints <- result.try(ints)

      let bytes =
        ints
        |> integer_string.to_bytes
        |> result.replace_error(json_error.JsonInvalid(
          "IntegerString value is invalid",
          path,
        ))
      use bytes <- result.try(bytes)

      Ok(data_element_value.new_binary_unchecked(vr, bytes))
    }

    value_representation.PersonName ->
      read_dicom_json_person_name_value(value, path)

    value_representation.SignedLong -> {
      let ints =
        value
        |> decode.run(decode.list(decode.int))
        |> result.replace_error(json_error.JsonInvalid(
          "SignedLong value is invalid",
          path,
        ))
      use ints <- result.try(ints)

      let is_valid =
        list.all(ints, fn(i) { i >= { -1 * 0x80000000 } && i <= 0x7FFFFFFF })
      use <- bool.guard(
        !is_valid,
        Error(json_error.JsonInvalid("SignedLong value is out of range", path)),
      )

      ints
      |> list.map(fn(x) { <<x:32-little>> })
      |> bit_array.concat
      |> data_element_value.new_binary_unchecked(vr, _)
      |> Ok
    }

    value_representation.SignedShort | value_representation.UnsignedShort -> {
      let ints =
        value
        |> decode.run(decode.list(decode.int))
        |> result.replace_error(json_error.JsonInvalid(
          "Short value is invalid",
          path,
        ))
      use ints <- result.try(ints)

      case dictionary.is_lut_descriptor_tag(tag), ints {
        True, [entry_count, first_input_value, bits_per_entry] ->
          <<
            entry_count:16-little,
            first_input_value:16-little,
            bits_per_entry:16-little,
          >>
          |> data_element_value.new_lookup_table_descriptor_unchecked(vr, _)
          |> Ok

        _, _ -> {
          let #(min, max) = case vr {
            value_representation.SignedShort -> #(-1 * 0x8000, 0x7FFF)
            _ -> #(0, 0xFFFF)
          }

          let is_valid = list.all(ints, fn(i) { i >= min && i <= max })
          use <- bool.guard(
            !is_valid,
            Error(json_error.JsonInvalid("Short value is out of range", path)),
          )

          ints
          |> list.map(fn(i) { <<i:16-little>> })
          |> bit_array.concat
          |> data_element_value.new_binary_unchecked(vr, _)
          |> Ok
        }
      }
    }

    value_representation.SignedVeryLong | value_representation.UnsignedVeryLong -> {
      let values =
        value
        |> decode.run(decode.list(decode.dynamic))
        |> result.replace_error(json_error.JsonInvalid(
          "Very long value is not a list",
          path,
        ))
      use values <- result.try(values)

      // Allow both int and string values. The latter is used when the integer
      // is too large to be represented by a JavaScript number.
      let big_ints =
        values
        |> list.map(fn(i) {
          case decode.run(i, decode.int) {
            Ok(i) -> Ok(bigi.from_int(i))
            Error(_) ->
              case decode.run(i, decode.string) {
                Ok(i) -> bigi.from_string(i)
                Error(_) -> Error(Nil)
              }
          }
        })
        |> result.all
        |> result.replace_error(json_error.JsonInvalid(
          "Very long value is invalid",
          path,
        ))
      use big_ints <- result.try(big_ints)

      let signedness = case vr {
        value_representation.SignedVeryLong -> bigi.Signed
        _ -> bigi.Unsigned
      }

      big_ints
      |> list.map(bigi.to_bytes(_, bigi.LittleEndian, signedness, 8))
      |> result.all
      |> result.map_error(fn(_) {
        json_error.JsonInvalid("Very long value is out of range", path)
      })
      |> result.map(bit_array.concat)
      |> result.map(data_element_value.new_binary_unchecked(vr, _))
    }

    value_representation.UnsignedLong -> {
      let ints =
        value
        |> decode.run(decode.list(decode.int))
        |> result.replace_error(json_error.JsonInvalid(
          "UnsignedLong value is invalid",
          path,
        ))
      use ints <- result.try(ints)

      let is_valid = list.all(ints, fn(i) { i >= 0 && i <= 0xFFFFFFFF })
      use <- bool.guard(
        !is_valid,
        Error(json_error.JsonInvalid("UnsignedLong value is out of range", path)),
      )

      ints
      |> list.map(fn(x) { <<x:32-little>> })
      |> bit_array.concat
      |> data_element_value.new_binary_unchecked(vr, _)
      |> Ok
    }

    value_representation.FloatingPointDouble -> {
      let floats =
        value
        |> decode.run(decode.list(decode_ieee_float()))
        |> result.replace_error(json_error.JsonInvalid(
          "FloatingPointDouble value is invalid",
          path,
        ))
      use floats <- result.try(floats)

      floats
      |> list.map(ieee_float.to_bytes_64_le)
      |> bit_array.concat
      |> data_element_value.new_binary_unchecked(vr, _)
      |> Ok
    }

    value_representation.FloatingPointSingle -> {
      let floats =
        value
        |> decode.run(decode.list(decode_ieee_float()))
        |> result.replace_error(json_error.JsonInvalid(
          "FloatingPointSingle value is invalid",
          path,
        ))
      use floats <- result.try(floats)

      floats
      |> list.map(ieee_float.to_bytes_32_le)
      |> bit_array.concat
      |> data_element_value.new_binary_unchecked(vr, _)
      |> Ok
    }

    value_representation.AttributeTag -> {
      let tags =
        value
        |> decode.run(decode.list(decode.string))
        |> result.replace_error(json_error.JsonInvalid(
          "AttributeTag value is invalid",
          path,
        ))
      use tags <- result.try(tags)

      let tags =
        tags
        |> list.map(fn(tag) {
          tag
          |> data_element_tag.from_hex_string
          |> result.map(fn(tag) {
            <<tag.group:16-little, tag.element:16-little>>
          })
        })
        |> result.all
        |> result.replace_error(json_error.JsonInvalid(
          "AttributeTag value is invalid",
          path,
        ))
      use tags <- result.try(tags)

      tags
      |> bit_array.concat
      |> data_element_value.new_binary_unchecked(vr, _)
      |> Ok
    }

    value_representation.Sequence ->
      value
      |> decode.run(decode.list(decode.dynamic))
      |> result.replace_error(json_error.JsonInvalid(
        "Sequence value is invalid",
        path,
      ))
      |> result.map(
        list.map(_, fn(json) {
          convert_json_to_data_set(json, data_set_path.new())
        }),
      )
      |> result.map(result.all)
      |> result.flatten
      |> result.map(data_element_value.new_sequence)

    _ ->
      Error(json_error.JsonInvalid(
        "Invalid 'Value' data element with VR '"
          <> value_representation.to_string(vr)
          <> "'",
        path,
      ))
  }
}

/// Decodes JSON to an `IEEEFloat`. Because gleam_json on Erlang doesn't
/// natively support Infinity and NaN values, these are instead handled as
/// strings.
///
fn decode_ieee_float() -> decode.Decoder(IEEEFloat) {
  decode.one_of(decode.float |> decode.map(ieee_float.finite), or: [
    decode.int |> decode.map(int.to_float) |> decode.map(ieee_float.finite),
    {
      use s <- decode.then(decode.string)

      case s {
        "Infinity" -> decode.success(ieee_float.positive_infinity())
        "-Infinity" -> decode.success(ieee_float.negative_infinity())
        "NaN" -> decode.success(ieee_float.nan())
        _ -> decode.failure(ieee_float.finite(0.0), "Number")
      }
    },
  ])
}

type PersonNameVariants {
  PersonNameVariants(
    alphabetic: Option(String),
    ideographic: Option(String),
    phonetic: Option(String),
  )
}

/// Reads a data element value from a DICOM JSON person name.
///
fn read_dicom_json_person_name_value(
  value: Dynamic,
  path: DataSetPath,
) -> Result(DataElementValue, JsonDeserializeError) {
  let person_name_variants =
    value
    |> decode.run(
      decode.list({
        use alphabetic <- decode.optional_field("Alphabetic", "", decode.string)
        use ideographic <- decode.optional_field(
          "Ideographic",
          "",
          decode.string,
        )
        use phonetic <- decode.optional_field("Phonetic", "", decode.string)

        decode.success(PersonNameVariants(
          alphabetic: Some(alphabetic),
          ideographic: Some(ideographic),
          phonetic: Some(phonetic),
        ))
      }),
    )
    |> result.replace_error(json_error.JsonInvalid(
      "PersonName value is invalid",
      path,
    ))
  use person_name_variants <- result.try(person_name_variants)

  person_name_variants
  |> list.map(fn(raw_person_name) {
    [
      option.unwrap(raw_person_name.alphabetic, ""),
      option.unwrap(raw_person_name.ideographic, ""),
      option.unwrap(raw_person_name.phonetic, ""),
    ]
    |> string.join("=")
    |> utils.trim_ascii_end(0x3D)
  })
  |> string.join("\\")
  |> bit_array.from_string
  |> bit_array_utils.pad_to_even_length(0x20)
  |> data_element_value.new_binary_unchecked(value_representation.PersonName, _)
  |> Ok
}

/// Reads a data element value from a DICOM JSON "InlineBinary" property.
///
fn read_dicom_json_inline_binary_value(
  inline_binary: Dynamic,
  tag: DataElementTag,
  vr: ValueRepresentation,
  transfer_syntax: Option(TransferSyntax),
  path: DataSetPath,
) -> Result(DataElementValue, JsonDeserializeError) {
  let inline_binary =
    inline_binary
    |> decode.run(decode.string)
    |> result.replace_error(json_error.JsonInvalid(
      "InlineBinary is not a string",
      path,
    ))
  use inline_binary <- result.try(inline_binary)

  let bytes =
    inline_binary
    |> bit_array.base64_decode
    |> result.replace_error(json_error.JsonInvalid(
      "InlineBinary is not a string",
      path,
    ))
  use bytes <- result.try(bytes)

  // Look at the tag and the transfer syntax to see if this inline binary holds
  // encapsulated pixel data.
  case
    tag == dictionary.pixel_data.tag
    && option.map(transfer_syntax, fn(ts) { ts.is_encapsulated }) == Some(True)
  {
    True ->
      read_encapsulated_pixel_data_items(bytes, vr, [])
      |> result.replace_error(json_error.JsonInvalid(
        "InlineBinary is not valid encapsulated pixel data",
        path,
      ))

    False ->
      // This value is native pixel data, so construct a binary value directly
      // from the bytes

      case vr {
        value_representation.OtherByteString
        | value_representation.OtherDoubleString
        | value_representation.OtherFloatString
        | value_representation.OtherLongString
        | value_representation.OtherVeryLongString
        | value_representation.OtherWordString
        | value_representation.Unknown ->
          Ok(data_element_value.new_binary_unchecked(vr, bytes))

        _ ->
          Error(json_error.JsonInvalid(
            "InlineBinary for a VR that doesn't support it",
            path,
          ))
      }
  }
}

/// Reads an encapsulated pixel data value from raw bytes.
///
fn read_encapsulated_pixel_data_items(
  bytes: BitArray,
  vr: ValueRepresentation,
  items: List(BitArray),
) -> Result(DataElementValue, Nil) {
  case bytes {
    <<0xFFFE:16-little, 0xE000:16-little, length:32-little, rest:bytes>> -> {
      use item <- result.try(bit_array.slice(rest, 0, length))

      use rest <- result.try(bit_array.slice(
        rest,
        length,
        bit_array.byte_size(rest) - length,
      ))

      read_encapsulated_pixel_data_items(rest, vr, [item, ..items])
    }

    <<>> ->
      items
      |> list.reverse
      |> data_element_value.new_encapsulated_pixel_data_unchecked(vr, _)
      |> Ok

    _ -> Error(Nil)
  }
}
