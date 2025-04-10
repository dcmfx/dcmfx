import bigi
import dcmfx_core/data_element_tag.{type DataElementTag, DataElementTag}
import dcmfx_core/data_element_value.{type DataElementValue}
import dcmfx_core/data_element_value/attribute_tag
import dcmfx_core/data_error.{type DataError}
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/data_set_path.{type DataSetPath}
import dcmfx_core/dictionary
import dcmfx_core/internal/utils
import dcmfx_core/value_representation.{type ValueRepresentation}
import dcmfx_json/json_config.{type DicomJsonConfig}
import dcmfx_json/json_error.{type JsonSerializeError}
import dcmfx_p10/p10_error
import dcmfx_p10/p10_token.{type P10Token}
import gleam/bit_array
import gleam/bool
import gleam/float
import gleam/int
import gleam/json
import gleam/list
import gleam/order
import gleam/result
import gleam/string
import ieee_float.{type IEEEFloat}

/// Transform that converts a stream of DICOM P10 tokens to the DICOM JSON
/// model.
///
pub type P10JsonTransform {
  P10JsonTransform(
    /// The DICOM JSON config to use when serializing the token stream to JSON.
    config: DicomJsonConfig,
    /// Whether a comma needs to be inserted before the next JSON value.
    insert_comma: Bool,
    /// The data element that value bytes are currently being gathered for.
    current_data_element: #(DataElementTag, List(BitArray)),
    /// Whether to ignore DataElementValueBytes tokens when they're received.
    /// This is used to stop certain data elements being included in the JSON.
    ignore_data_element_value_bytes: Bool,
    /// Whether tokens for encapsulated pixel data are currently being received.
    in_encapsulated_pixel_data: Bool,
    /// When multiple binary tokens are being directly streamed as an
    /// InlineBinary, there can be 0, 1, or 2 bytes left over from the previous
    /// chunk due to Base64 converting in three byte chunks. These leftover
    /// bytes are prepended to the next chunk of data when it arrives for Base64
    /// conversion.
    pending_base64_input: BitArray,
    /// The data set path to where JSON serialization is currently up to. This
    /// is used to provide precise location information when an error occurs.
    data_set_path: DataSetPath,
    /// The number of items in each active sequence in the data set path. This
    /// is used to provide precise location information when an error occurs.
    sequence_item_counts: List(Int),
  )
}

/// Constructs a new P10 tokens to DICOM JSON transform.
///
pub fn new(config: DicomJsonConfig) -> P10JsonTransform {
  P10JsonTransform(
    config:,
    insert_comma: False,
    current_data_element: #(DataElementTag(0, 0), []),
    ignore_data_element_value_bytes: False,
    in_encapsulated_pixel_data: False,
    pending_base64_input: <<>>,
    data_set_path: data_set_path.new(),
    sequence_item_counts: [],
  )
}

/// Adds the next DICOM P10 token to this JSON transform. Bytes of JSON data are
/// returned as they become available.
///
/// If P10 tokens are provided in an invalid order then an error may be
/// returned, but this is not guaranteed for all invalid token orders, so in
/// some cases the resulting JSON stream could be invalid when the incoming
/// stream of P10 tokens is malformed.
///
pub fn add_token(
  transform: P10JsonTransform,
  token: P10Token,
) -> Result(#(String, P10JsonTransform), JsonSerializeError) {
  let token_stream_invalid_error = fn(_: a) {
    p10_error.TokenStreamInvalid(
      when: "Adding token to JSON transform",
      details: "The transform was not able to write this token",
      token:,
    )
    |> json_error.P10Error
  }

  case token {
    p10_token.FilePreambleAndDICMPrefix(..) -> Ok(#("", transform))
    p10_token.FileMetaInformation(data_set) -> Ok(begin(transform, data_set))

    p10_token.DataElementHeader(tag, vr, length) -> {
      let #(json, transform) =
        write_data_element_header(transform, tag, vr, length)

      use path <- result.map(
        data_set_path.add_data_element(transform.data_set_path, tag)
        |> result.map_error(token_stream_invalid_error),
      )

      let transform = P10JsonTransform(..transform, data_set_path: path)

      #(json, transform)
    }

    p10_token.DataElementValueBytes(vr:, data:, bytes_remaining:, ..) -> {
      use #(json, transform) <- result.try(write_data_element_value_bytes(
        transform,
        vr,
        data,
        bytes_remaining,
      ))

      let transform = case bytes_remaining {
        0 -> {
          data_set_path.pop(transform.data_set_path)
          |> result.map_error(token_stream_invalid_error)
          |> result.map(fn(path) {
            P10JsonTransform(..transform, data_set_path: path)
          })
        }
        _ -> Ok(transform)
      }

      use transform <- result.map(transform)

      #(json, transform)
    }

    p10_token.SequenceStart(tag, vr) -> {
      use #(json, transform) <- result.try(write_sequence_start(
        transform,
        tag,
        vr,
      ))

      let path =
        data_set_path.add_data_element(transform.data_set_path, tag)
        |> result.map_error(token_stream_invalid_error)
      use path <- result.map(path)

      let transform =
        P10JsonTransform(
          ..transform,
          data_set_path: path,
          sequence_item_counts: [0, ..transform.sequence_item_counts],
        )

      #(json, transform)
    }

    p10_token.SequenceDelimiter(..) -> {
      let #(json, transform) = write_sequence_end(transform)

      let path =
        data_set_path.pop(transform.data_set_path)
        |> result.map_error(token_stream_invalid_error)
      use path <- result.try(path)

      let sequence_item_counts =
        list.rest(transform.sequence_item_counts) |> result.unwrap([])

      let transform =
        P10JsonTransform(
          ..transform,
          data_set_path: path,
          sequence_item_counts:,
        )

      Ok(#(json, transform))
    }

    p10_token.SequenceItemStart(..) -> {
      let path_and_item_counts = case transform.sequence_item_counts {
        [count, ..rest] ->
          transform.data_set_path
          |> data_set_path.add_sequence_item(count)
          |> result.map_error(token_stream_invalid_error)
          |> result.map(fn(path) { #(path, [count + 1, ..rest]) })

        _ -> Ok(#(transform.data_set_path, transform.sequence_item_counts))
      }

      use #(path, sequence_item_counts) <- result.map(path_and_item_counts)

      let transform =
        P10JsonTransform(
          ..transform,
          data_set_path: path,
          sequence_item_counts:,
        )

      write_sequence_item_start(transform)
    }

    p10_token.SequenceItemDelimiter -> {
      let #(json, transform) = write_sequence_item_end(transform)

      let path =
        data_set_path.pop(transform.data_set_path)
        |> result.map_error(token_stream_invalid_error)
      use path <- result.try(path)

      let transform = P10JsonTransform(..transform, data_set_path: path)

      Ok(#(json, transform))
    }

    p10_token.PixelDataItem(length:, ..) -> {
      let path_and_item_counts = case transform.sequence_item_counts {
        [count, ..rest] ->
          transform.data_set_path
          |> data_set_path.add_sequence_item(count)
          |> result.map_error(token_stream_invalid_error)
          |> result.map(fn(path) { #(path, [count + 1, ..rest]) })

        _ -> Ok(#(transform.data_set_path, transform.sequence_item_counts))
      }

      use #(path, sequence_item_counts) <- result.try(path_and_item_counts)

      let transform =
        P10JsonTransform(
          ..transform,
          data_set_path: path,
          sequence_item_counts:,
        )

      write_encapsulated_pixel_data_item(transform, length)
    }

    p10_token.End -> Ok(#(end(transform), transform))
  }
}

fn indent(transform: P10JsonTransform, offset: Int) {
  string.repeat(
    "  ",
    1 + data_set_path.sequence_item_count(transform.data_set_path) * 3 + offset,
  )
}

fn begin(
  transform: P10JsonTransform,
  file_meta_information: DataSet,
) -> #(String, P10JsonTransform) {
  let json =
    "{"
    <> case transform.config.pretty_print {
      True -> "\n"
      False -> ""
    }

  // Exclude all File Meta Information data elements except for '(0002,0010)
  // Transfer Syntax UID' when encapsulated pixel data is being included as it
  // is needed to interpret that data
  let #(transfer_syntax_json, transform) = case
    transform.config.store_encapsulated_pixel_data
  {
    True -> {
      let transfer_syntax_uid =
        data_set.get_string(
          file_meta_information,
          dictionary.transfer_syntax_uid.tag,
        )

      case transfer_syntax_uid {
        Ok(transfer_syntax_uid) -> {
          let transform = P10JsonTransform(..transform, insert_comma: True)

          let json = case transform.config.pretty_print {
            True ->
              "  \"00020010\": {\n    \"vr\": \"UI\",\n    \"Value\": [\n      \""
              <> transfer_syntax_uid
              <> "\"\n    ]\n  }"
            False ->
              "\"00020010\":{\"vr\":\"UI\",\"Value\":[\""
              <> transfer_syntax_uid
              <> "\"]}"
          }

          #(json, transform)
        }
        _ -> #("", transform)
      }
    }
    _ -> #("", transform)
  }

  #(json <> transfer_syntax_json, transform)
}

fn write_data_element_header(
  transform: P10JsonTransform,
  tag: DataElementTag,
  vr: ValueRepresentation,
  length: Int,
) -> #(String, P10JsonTransform) {
  // Exclude group length data elements as these have no use in DICOM JSON. Also
  // exclude the '(0008,0005) Specific Character Set' data element as DICOM JSON
  // always uses UTF-8.
  use <- bool.lazy_guard(
    tag.element == 0 || tag == dictionary.specific_character_set.tag,
    fn() {
      let transform =
        P10JsonTransform(..transform, ignore_data_element_value_bytes: True)

      #("", transform)
    },
  )

  let json = case transform.insert_comma {
    True ->
      case transform.config.pretty_print {
        True -> ",\n"
        False -> ","
      }
    False -> ""
  }

  let transform =
    P10JsonTransform(
      ..transform,
      insert_comma: True,
      current_data_element: #(tag, []),
    )

  // Write the tag and VR
  let json =
    json
    <> case transform.config.pretty_print {
      True ->
        indent(transform, 0)
        <> "\""
        <> data_element_tag.to_hex_string(tag)
        <> "\": {\n"
        <> indent(transform, 1)
        <> "\"vr\": \""
        <> value_representation.to_string(vr)
        <> "\""
      False ->
        "\""
        <> data_element_tag.to_hex_string(tag)
        <> "\":{\"vr\":\""
        <> value_representation.to_string(vr)
        <> "\""
    }

  // If the value's length is zero then no 'Value' or 'InlineBinary' should be
  // added to the output. Ref: PS3.18 F.2.5.
  use <- bool.lazy_guard(length == 0, fn() {
    let json =
      json
      <> case transform.config.pretty_print {
        True -> "\n" <> indent(transform, 0) <> "}"
        False -> "}"
      }

    let transform =
      P10JsonTransform(..transform, ignore_data_element_value_bytes: True)

    #(json, transform)
  })

  // The following VRs use InlineBinary in the output
  let json =
    json
    <> case vr {
      value_representation.OtherByteString
      | value_representation.OtherDoubleString
      | value_representation.OtherFloatString
      | value_representation.OtherLongString
      | value_representation.OtherVeryLongString
      | value_representation.OtherWordString
      | value_representation.Unknown ->
        case transform.config.pretty_print {
          True -> ",\n" <> indent(transform, 1) <> "\"InlineBinary\": \""
          False -> ",\"InlineBinary\":\""
        }
      _ ->
        case transform.config.pretty_print {
          True -> ",\n" <> indent(transform, 1) <> "\"Value\": [\n"
          False -> ",\"Value\":["
        }
    }

  #(json, transform)
}

fn write_data_element_value_bytes(
  transform: P10JsonTransform,
  vr: ValueRepresentation,
  data: BitArray,
  bytes_remaining: Int,
) -> Result(#(String, P10JsonTransform), JsonSerializeError) {
  // If this data element value is being ignored then do nothing
  use <- bool.lazy_guard(transform.ignore_data_element_value_bytes, fn() {
    let transform =
      P10JsonTransform(..transform, ignore_data_element_value_bytes: False)

    Ok(#("", transform))
  })

  // The following VRs are streamed out directly as Base64
  let is_inline_binary =
    vr == value_representation.OtherByteString
    || vr == value_representation.OtherDoubleString
    || vr == value_representation.OtherFloatString
    || vr == value_representation.OtherLongString
    || vr == value_representation.OtherVeryLongString
    || vr == value_representation.OtherWordString
    || vr == value_representation.Unknown
  use <- bool.lazy_guard(is_inline_binary, fn() {
    let #(json, transform) =
      write_base64(
        transform,
        data,
        bytes_remaining == 0 && !transform.in_encapsulated_pixel_data,
      )

    let json =
      json
      <> case bytes_remaining == 0 && !transform.in_encapsulated_pixel_data {
        True ->
          case transform.config.pretty_print {
            True -> "\"\n" <> indent(transform, 0) <> "}"
            False -> "\"}"
          }
        False -> ""
      }

    Ok(#(json, transform))
  })

  // If this data element value is not an inline binary and has no data then
  // there's nothing to do
  use <- bool.guard(
    bit_array.byte_size(data) == 0 && bytes_remaining == 0,
    Ok(#("", transform)),
  )

  // Gather the final data for this data element
  let transform =
    P10JsonTransform(
      ..transform,
      current_data_element: #(transform.current_data_element.0, [
        data,
        ..transform.current_data_element.1
      ]),
    )

  // Wait until all bytes for the data element have been accumulated
  use <- bool.guard(bytes_remaining > 0, Ok(#("", transform)))

  // Create final binary data element value
  let bytes = bit_array.concat(transform.current_data_element.1)
  let value = data_element_value.new_binary_unchecked(vr, bytes)

  let json_values =
    convert_binary_value_to_json(value, bytes, transform)
    |> result.map_error(fn(e) {
      json_error.DataError(data_error.with_path(e, transform.data_set_path))
    })
  use json_values <- result.map(json_values)

  let json = case transform.config.pretty_print {
    True ->
      indent(transform, 2)
      <> string.join(json_values, ",\n" <> indent(transform, 2))
      <> "\n"
      <> indent(transform, 1)
      <> "]\n"
      <> indent(transform, 0)
      <> "}"
    False -> string.join(json_values, ",") <> "]}"
  }

  #(json, transform)
}

fn write_sequence_start(
  transform: P10JsonTransform,
  tag: DataElementTag,
  vr: ValueRepresentation,
) -> Result(#(String, P10JsonTransform), JsonSerializeError) {
  let json = case transform.insert_comma {
    True ->
      case transform.config.pretty_print {
        True -> ",\n"
        False -> ","
      }
    False -> ""
  }

  let transform = P10JsonTransform(..transform, insert_comma: True)

  case vr {
    value_representation.Sequence -> {
      let transform = P10JsonTransform(..transform, insert_comma: False)

      let json =
        json
        <> case transform.config.pretty_print {
          True ->
            indent(transform, 0)
            <> "\""
            <> data_element_tag.to_hex_string(tag)
            <> "\": {\n"
            <> indent(transform, 1)
            <> "\"vr\": \"SQ\",\n"
            <> indent(transform, 1)
            <> "\"Value\": ["

          False ->
            "\""
            <> data_element_tag.to_hex_string(tag)
            <> "\":{\"vr\":\"SQ\",\"Value\":["
        }

      Ok(#(json, transform))
    }

    _ -> {
      use <- bool.lazy_guard(
        !transform.config.store_encapsulated_pixel_data,
        fn() {
          Error(json_error.DataError(
            data_error.new_value_invalid(
              "DICOM JSON does not support encapsulated pixel data, consider "
              <> "enabling this extension in the config",
            )
            |> data_error.with_path(transform.data_set_path),
          ))
        },
      )

      let transform =
        P10JsonTransform(..transform, in_encapsulated_pixel_data: True)

      let json =
        json
        <> case transform.config.pretty_print {
          True ->
            indent(transform, 0)
            <> "\""
            <> data_element_tag.to_hex_string(tag)
            <> "\": {\n"
            <> indent(transform, 1)
            <> "\"vr\": \""
            <> value_representation.to_string(vr)
            <> "\",\n"
            <> indent(transform, 1)
            <> "\"InlineBinary\": \""

          False ->
            "\""
            <> data_element_tag.to_hex_string(tag)
            <> "\":{\"vr\":\""
            <> value_representation.to_string(vr)
            <> "\",\"InlineBinary\":\""
        }

      Ok(#(json, transform))
    }
  }
}

fn write_sequence_end(
  transform: P10JsonTransform,
) -> #(String, P10JsonTransform) {
  case transform.in_encapsulated_pixel_data {
    True -> {
      let transform =
        P10JsonTransform(..transform, in_encapsulated_pixel_data: False)

      let #(json, transform) = write_base64(transform, <<>>, True)

      let json =
        json
        <> case transform.config.pretty_print {
          True -> "\"\n" <> indent(transform, 0) <> "}"
          False -> "\"}"
        }

      #(json, transform)
    }

    False -> {
      let transform = P10JsonTransform(..transform, insert_comma: True)

      let json = case transform.config.pretty_print {
        True ->
          "\n" <> indent(transform, 1) <> "]\n" <> indent(transform, 0) <> "}"
        False -> "]}"
      }

      #(json, transform)
    }
  }
}

fn write_sequence_item_start(
  transform: P10JsonTransform,
) -> #(String, P10JsonTransform) {
  let json = case transform.insert_comma {
    True -> ","
    False -> ""
  }

  let transform = P10JsonTransform(..transform, insert_comma: False)

  let json =
    json
    <> case transform.config.pretty_print {
      True -> "\n" <> indent(transform, -1) <> "{\n"
      False -> "{"
    }

  #(json, transform)
}

fn write_sequence_item_end(
  transform: P10JsonTransform,
) -> #(String, P10JsonTransform) {
  let transform = P10JsonTransform(..transform, insert_comma: True)

  let json = case transform.config.pretty_print {
    True -> "\n" <> indent(transform, -1) <> "}"
    False -> "}"
  }

  #(json, transform)
}

fn write_encapsulated_pixel_data_item(
  transform: P10JsonTransform,
  length: Int,
) -> Result(#(String, P10JsonTransform), JsonSerializeError) {
  use <- bool.lazy_guard(!transform.config.store_encapsulated_pixel_data, fn() {
    Error(json_error.DataError(
      data_error.new_value_invalid(
        "DICOM JSON does not support encapsulated pixel data, consider "
        <> "enabling this extension in the config",
      )
      |> data_error.with_path(transform.data_set_path),
    ))
  })

  // Construct bytes for the item header
  let bytes = <<0xFE, 0xFF, 0x00, 0xE0, length:32-little>>

  Ok(write_base64(transform, bytes, False))
}

fn end(transform: P10JsonTransform) -> String {
  let json = case transform.config.pretty_print {
    True -> "\n}\n"
    False -> "}"
  }

  json
}

fn write_base64(
  transform: P10JsonTransform,
  input: BitArray,
  finish: Bool,
) -> #(String, P10JsonTransform) {
  let input_size = bit_array.byte_size(input)

  // If there's still insufficient data to encode with this new data then
  // accumulate the bytes and wait till next time
  use <- bool.lazy_guard(
    bit_array.byte_size(transform.pending_base64_input) + input_size < 3
      && !finish,
    fn() {
      let transform =
        P10JsonTransform(..transform, pending_base64_input: <<
          transform.pending_base64_input:bits,
          input:bits,
        >>)

      #("", transform)
    },
  )

  // Calculate how many of the input bytes to consume. Bytes must be fed to the
  // Base64 encoder in lots of 3, and any leftover saved till next time. If
  // these are the final bytes then all remaining bytes are encoded and the
  // encoder will add any required Base64 padding.
  let input_bytes_consumed = case finish {
    True -> input_size
    False -> {
      { bit_array.byte_size(transform.pending_base64_input) + input_size }
      / 3
      * 3
      - bit_array.byte_size(transform.pending_base64_input)
    }
  }

  let assert Ok(base64_input) = bit_array.slice(input, 0, input_bytes_consumed)

  // Base64 encode the bytes and output to the stream
  let json =
    bit_array.base64_encode(
      <<transform.pending_base64_input:bits, base64_input:bits>>,
      finish,
    )

  // Save off unencoded bytes for next time
  let transform =
    P10JsonTransform(
      ..transform,
      pending_base64_input: case
        bit_array.slice(
          input,
          input_bytes_consumed,
          input_size - input_bytes_consumed,
        )
      {
        Ok(bytes) -> bytes
        _ -> <<>>
      },
    )

  #(json, transform)
}

fn convert_binary_value_to_json(
  value: DataElementValue,
  bytes: BitArray,
  transform: P10JsonTransform,
) -> Result(List(String), DataError) {
  case data_element_value.value_representation(value) {
    // AttributeTag value representation
    value_representation.AttributeTag -> {
      use tags <- result.try(attribute_tag.from_bytes(bytes))

      tags
      |> list.map(fn(tag) {
        "\"" <> data_element_tag.to_hex_string(tag) <> "\""
      })
      |> Ok
    }

    // Floating point value representations
    value_representation.DecimalString
    | value_representation.FloatingPointDouble
    | value_representation.FloatingPointSingle -> {
      use value <- result.try(data_element_value.get_floats(value))

      Ok(list.map(value, encode_ieee_float))
    }

    // PersonName value representation
    value_representation.PersonName -> {
      let s =
        bit_array.to_string(bytes)
        |> result.replace_error(data_error.new_value_invalid(
          "PersonName is invalid UTF-8",
        ))
      use s <- result.try(s)

      s
      |> string.split("\\")
      |> list.map(fn(raw_name) {
        let component_groups =
          raw_name
          |> string.split("=")
          |> list.map(utils.trim_ascii_end(_, 0x20))
          |> list.index_map(fn(s, i) { #(i, s) })

        use <- bool.guard(
          list.length(component_groups) > 3,
          Error(data_error.new_value_invalid(
            "PersonName has too many component groups: "
            <> int.to_string(list.length(component_groups)),
          )),
        )

        let component_groups =
          component_groups |> list.filter(fn(x) { !string.is_empty(x.1) })

        let result = case transform.config.pretty_print {
          True -> indent(transform, -1) <> "{\n"
          False -> "{"
        }

        let result =
          component_groups
          |> list.index_fold(result, fn(result, x, i) {
            let name = case x.0 {
              0 -> "Alphabetic"
              1 -> "Ideographic"
              _ -> "Phonetic"
            }

            // Escape the value of the component group appropriately for JSON
            let value = json.to_string(json.string(x.1))

            result
            <> case transform.config.pretty_print {
              True -> indent(transform, 3) <> "\"" <> name <> "\": " <> value
              False -> "\"" <> name <> "\":" <> value
            }
            <> case i == list.length(component_groups) - 1 {
              True -> ""
              False ->
                case transform.config.pretty_print {
                  True -> ",\n"
                  False -> ","
                }
            }
          })

        let result =
          case transform.config.pretty_print {
            True -> result <> "\n" <> indent(transform, 2)
            False -> result
          }
          <> "}"

        Ok(result)
      })
      |> result.all
    }

    // Binary signed/unsigned integer value representations
    value_representation.SignedLong
    | value_representation.SignedShort
    | value_representation.UnsignedLong
    | value_representation.UnsignedShort
    | value_representation.IntegerString ->
      value
      |> data_element_value.get_ints
      |> result.map(list.map(_, int.to_string))

    // Binary signed/unsigned big integer value representations
    value_representation.SignedVeryLong | value_representation.UnsignedVeryLong -> {
      use value <- result.try(data_element_value.get_big_ints(value))

      // Integers outside of the range representable by a JavaScript number are
      // instead output as strings
      let assert Ok(min_safe_integer) = bigi.from_string("-9007199254740991")
      let assert Ok(max_safe_integer) = bigi.from_string("9007199254740991")

      value
      |> list.map(fn(i) {
        case
          bigi.compare(i, min_safe_integer) != order.Lt
          && bigi.compare(i, max_safe_integer) != order.Gt
        {
          True -> {
            let assert Ok(i) = bigi.to_int(i)
            int.to_string(i)
          }
          False -> "\"" <> bigi.to_string(i) <> "\""
        }
      })
      |> Ok
    }

    // Handle string VRs that have explicit internal structure. Their value
    // is deliberately not parsed or validated beyond conversion to UTF-8, and
    // is just passed straight through.
    value_representation.AgeString
    | value_representation.Date
    | value_representation.DateTime
    | value_representation.Time ->
      bytes
      |> bit_array.to_string
      |> result.map_error(fn(_) {
        data_error.new_value_invalid("String bytes are not valid UTF-8")
      })
      |> result.map(utils.trim_ascii_end(_, 0x20))
      |> result.map(prepare_json_string)
      |> result.map(fn(s) { [s] })

    // Handle string VRs that don't support multiplicity
    value_representation.ApplicationEntity
    | value_representation.LongText
    | value_representation.ShortText
    | value_representation.UniversalResourceIdentifier
    | value_representation.UnlimitedText -> {
      use value <- result.try(data_element_value.get_string(value))

      value
      |> prepare_json_string
      |> fn(s) { [s] }
      |> Ok
    }

    // Handle remaining string-based VRs that support multiplicity
    value_representation.CodeString
    | value_representation.LongString
    | value_representation.ShortString
    | value_representation.UnlimitedCharacters
    | value_representation.UniqueIdentifier -> {
      use value <- result.try(data_element_value.get_strings(value))

      value
      |> list.map(prepare_json_string)
      |> Ok
    }

    _ ->
      Error(data_error.new_value_invalid(
        "Data element value not valid for its VR",
      ))
  }
}

fn prepare_json_string(value: String) -> String {
  case value == "" {
    True -> "null"
    False -> json.to_string(json.string(value))
  }
}

/// Encodes an `IEEEFloat` to JSON. Because gleam_json on Erlang doesn't
/// natively support Infinity and NaN values, these are instead converted to
/// strings.
///
fn encode_ieee_float(f: IEEEFloat) -> String {
  case ieee_float.to_finite(f) {
    Ok(f) -> float.to_string(f)

    _ -> {
      use <- bool.guard(f == ieee_float.positive_infinity(), "\"Infinity\"")
      use <- bool.guard(f == ieee_float.negative_infinity(), "\"-Infinity\"")

      "\"NaN\""
    }
  }
}
