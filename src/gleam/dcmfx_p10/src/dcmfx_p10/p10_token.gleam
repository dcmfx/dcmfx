//// Defines the various DICOM P10 tokens that are read out of raw DICOM P10
//// data by the `p10_read` module.

import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_element_value.{type DataElementValue}
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/data_set_path.{type DataSetPath}
import dcmfx_core/dictionary
import dcmfx_core/value_representation.{type ValueRepresentation}
import dcmfx_p10/internal/data_element_header
import dcmfx_p10/internal/value_length
import gleam/bit_array
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/result
import gleam/string

/// A DICOM P10 token is the smallest piece of structured DICOM P10 data, and a
/// stream of these tokens is most commonly the result of progressive reading of
/// raw DICOM P10 bytes, or from conversion of a data set into P10 tokens for
/// transmission or serialization.
///
pub type P10Token {
  /// The 128-byte File Preamble and the "DICM" prefix, which are present at the
  /// start of DICOM P10 data. The content of the File Preamble's bytes are
  /// application-defined, and in many cases are unused and set to zero.
  ///
  /// When reading DICOM P10 data that doesn't contain a File Preamble and
  /// "DICM" prefix this token is emitted with all bytes set to zero.
  FilePreambleAndDICMPrefix(preamble: BitArray)

  /// The File Meta Information dataset for the DICOM P10.
  ///
  /// When reading DICOM P10 data that doesn't contain File Meta Information
  /// this token is emitted with an empty data set.
  FileMetaInformation(data_set: DataSet)

  /// The start of the next data element. This token will always be followed by
  /// one or more `DataElementValueBytes` tokens containing the value bytes for
  /// the data element.
  DataElementHeader(
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: Int,
    path: DataSetPath,
  )

  /// Raw data for the value of the current data element. Data element values
  /// are split across multiple of these tokens when their length exceeds the
  /// maximum token size.
  DataElementValueBytes(
    tag: DataElementTag,
    vr: ValueRepresentation,
    data: BitArray,
    bytes_remaining: Int,
  )

  /// The start of a new sequence. If this is the start of a sequence of
  /// encapsulated pixel data then the VR of that data, either `OtherByteString`
  /// or `OtherWordString`, will be specified. If not, the VR will be
  /// `Sequence`.
  SequenceStart(tag: DataElementTag, vr: ValueRepresentation, path: DataSetPath)

  /// The end of the current sequence.
  SequenceDelimiter(tag: DataElementTag)

  /// The start of a new item in the current sequence.
  SequenceItemStart(index: Int)

  /// The end of the current sequence item.
  SequenceItemDelimiter

  /// The start of a new item in the current encapsulated pixel data sequence.
  /// The data for the item follows in one or more `DataElementValueBytes`
  /// tokens.
  PixelDataItem(index: Int, length: Int)

  /// The end of the DICOM P10 data has been reached with all provided data
  /// successfully parsed.
  End
}

/// Converts a DICOM P10 token to a human-readable string.
///
pub fn to_string(token: P10Token) -> String {
  case token {
    FilePreambleAndDICMPrefix(_) -> "FilePreambleAndDICMPrefix"

    FileMetaInformation(data_set) ->
      "FileMetaInformation: "
      <> data_set.map(data_set, fn(tag, value) {
        data_element_header.DataElementHeader(
          tag,
          Some(data_element_value.value_representation(value)),
          value_length.zero,
        )
        |> data_element_header.to_string
        <> ": "
        <> data_element_value.to_string(value, tag, 80)
      })
      |> string.join(", ")

    DataElementHeader(tag, vr, length, ..) ->
      "DataElementHeader: "
      <> data_element_tag.to_string(tag)
      <> ", name: "
      <> dictionary.tag_name(tag, None)
      <> ", vr: "
      <> value_representation.to_string(vr)
      <> ", length: "
      <> int.to_string(length)
      <> " bytes"

    DataElementValueBytes(data:, bytes_remaining:, ..) ->
      "DataElementValueBytes: "
      <> int.to_string(bit_array.byte_size(data))
      <> " bytes of data, "
      <> int.to_string(bytes_remaining)
      <> " bytes remaining"

    SequenceStart(tag, vr, ..) ->
      "SequenceStart: "
      <> data_element_tag.to_string(tag)
      <> ", name: "
      <> dictionary.tag_name(tag, None)
      <> ", vr: "
      <> value_representation.to_string(vr)

    SequenceDelimiter(..) -> "SequenceDelimiter"

    SequenceItemStart(index) ->
      "SequenceItemStart: item " <> int.to_string(index)

    SequenceItemDelimiter -> "SequenceItemDelimiter"

    PixelDataItem(index, length) ->
      "PixelDataItem: item "
      <> int.to_string(index)
      <> ", "
      <> int.to_string(length)
      <> " bytes"

    End -> "End"
  }
}

/// Returns whether this DICOM P10 token is token of the file header or File
/// Meta Information prior to the main data set, i.e. is it a
/// `FilePreambleAndDICMPrefix` or `FileMetaInformation` token.
///
pub fn is_header_token(token: P10Token) -> Bool {
  case token {
    FilePreambleAndDICMPrefix(..) | FileMetaInformation(..) -> True
    _ -> False
  }
}

/// Converts all the data elements in a data set directly to DICOM P10 tokens.
/// Each token is returned via a callback.
///
pub fn data_elements_to_tokens(
  data_set: DataSet,
  path: DataSetPath,
  context: a,
  token_callback: fn(a, P10Token) -> Result(a, b),
) -> Result(a, b) {
  data_set
  |> data_set.try_fold(context, fn(context, tag, value) {
    let assert Ok(path) = data_set_path.add_data_element(path, tag)

    data_element_to_tokens(tag, value, path, context, token_callback)
  })
}

/// Converts a DICOM data element to DICOM P10 tokens. Each token is returned
/// via a callback.
///
pub fn data_element_to_tokens(
  tag: DataElementTag,
  value: DataElementValue,
  path: DataSetPath,
  context: a,
  token_callback: fn(a, P10Token) -> Result(a, b),
) -> Result(a, b) {
  let vr = data_element_value.value_representation(value)

  case data_element_value.bytes(value) {
    // For values that have their bytes directly available write them out as-is
    Ok(bytes) -> {
      let header_token =
        DataElementHeader(tag, vr, bit_array.byte_size(bytes), path)
      use context <- result.try(token_callback(context, header_token))

      DataElementValueBytes(tag, vr, bytes, bytes_remaining: 0)
      |> token_callback(context, _)
    }

    Error(_) ->
      case data_element_value.encapsulated_pixel_data(value) {
        // For encapsulated pixel data, write all of the items individually,
        // followed by a sequence delimiter
        Ok(items) -> {
          let header_token = SequenceStart(tag, vr, path)
          use context <- result.try(token_callback(context, header_token))

          let context =
            items
            |> list.try_fold(#(context, 0), fn(acc, item) {
              let #(context, index) = acc

              let length = bit_array.byte_size(item)
              let item_header_token = PixelDataItem(index:, length:)
              let context = token_callback(context, item_header_token)
              use context <- result.try(context)

              let value_bytes_token =
                DataElementValueBytes(dictionary.item.tag, vr, item, 0)
              use context <- result.map(token_callback(
                context,
                value_bytes_token,
              ))

              #(context, index + 1)
            })
            |> result.map(fn(acc) { acc.0 })

          use context <- result.try(context)

          // Write delimiter for the encapsulated pixel data sequence
          token_callback(context, SequenceDelimiter(tag))
        }

        Error(_) -> {
          // For sequences, write the item data sets recursively, followed by a
          // sequence delimiter
          let assert Ok(items) = data_element_value.sequence_items(value)

          let header_token = SequenceStart(tag, vr, path)
          use context <- result.try(token_callback(context, header_token))

          let context =
            items
            |> list.try_fold(#(context, 0), fn(acc, item) {
              let #(context, index) = acc

              let item_start_token = SequenceItemStart(index:)
              let context = token_callback(context, item_start_token)
              use context <- result.try(context)

              let assert Ok(path) = data_set_path.add_sequence_item(path, index)

              use context <- result.try(data_elements_to_tokens(
                item,
                path,
                context,
                token_callback,
              ))

              // Write delimiter for the item
              let item_delimiter_token = SequenceItemDelimiter
              use context <- result.map(token_callback(
                context,
                item_delimiter_token,
              ))

              #(context, index + 1)
            })
            |> result.map(fn(acc) { acc.0 })

          use context <- result.try(context)

          // Write delimiter for the sequence
          token_callback(context, SequenceDelimiter(tag))
        }
      }
  }
}
