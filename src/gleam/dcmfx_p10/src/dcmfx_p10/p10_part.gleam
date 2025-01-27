//// Defines the various parts of a DICOM P10 that are read out of raw DICOM P10
//// data by the `p10_read` module.

import dcmfx_core/data_element_tag.{type DataElementTag, DataElementTag}
import dcmfx_core/data_element_value.{type DataElementValue}
import dcmfx_core/data_set.{type DataSet}
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

/// A DICOM P10 part is the smallest piece of structured DICOM P10 data, and a
/// stream of these parts is most commonly the result of progressive reading of
/// raw DICOM P10 bytes, or from conversion of a data set into P10 parts for
/// transmission or serialization.
///
pub type P10Part {
  /// The 128-byte File Preamble and the "DICM" prefix, which are present at the
  /// start of DICOM P10 data. The content of the File Preamble's bytes are
  /// application-defined, and in many cases are unused and set to zero.
  ///
  /// When reading DICOM P10 data that doesn't contain a File Preamble and
  /// "DICM" prefix this part is emitted with all bytes set to zero.
  FilePreambleAndDICMPrefix(preamble: BitArray)

  /// The File Meta Information dataset for the DICOM P10.
  ///
  /// When reading DICOM P10 data that doesn't contain File Meta Information
  /// this part is emitted with an empty data set.
  FileMetaInformation(data_set: DataSet)

  /// The start of the next data element. This part will always be followed by
  /// one or more `DataElementValueBytes` parts containing the value bytes for
  /// the data element.
  DataElementHeader(tag: DataElementTag, vr: ValueRepresentation, length: Int)

  /// Raw data for the value of the current data element. Data element values
  /// are split across multiple of these parts when their length exceeds the
  /// maximum part size.
  DataElementValueBytes(
    vr: ValueRepresentation,
    data: BitArray,
    bytes_remaining: Int,
  )

  /// The start of a new sequence. If this is the start of a sequence of
  /// encapsulated pixel data then the VR of that data, either `OtherByteString`
  /// or `OtherWordString`, will be specified. If not, the VR will be
  /// `Sequence`.
  SequenceStart(tag: DataElementTag, vr: ValueRepresentation)

  /// The end of the current sequence.
  SequenceDelimiter

  /// The start of a new item in the current sequence.
  SequenceItemStart

  /// The end of the current sequence item.
  SequenceItemDelimiter

  /// The start of a new item in the current encapsulated pixel data sequence.
  /// The data for the item follows in one or more `DataElementValueBytes`
  /// parts.
  PixelDataItem(length: Int)

  /// The end of the DICOM P10 data has been reached with all provided data
  /// successfully parsed.
  End
}

/// Converts a DICOM P10 part to a human-readable string.
///
pub fn to_string(part: P10Part) -> String {
  case part {
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

    DataElementHeader(tag, vr, length) ->
      "DataElementHeader: "
      <> data_element_tag.to_string(tag)
      <> ", name: "
      <> dictionary.tag_name(tag, None)
      <> ", vr: "
      <> value_representation.to_string(vr)
      <> ", length: "
      <> int.to_string(length)
      <> " bytes"

    DataElementValueBytes(_vr, data, bytes_remaining) ->
      "DataElementValueBytes: "
      <> int.to_string(bit_array.byte_size(data))
      <> " bytes of data, "
      <> int.to_string(bytes_remaining)
      <> " bytes remaining"

    SequenceStart(tag, vr) ->
      "SequenceStart: "
      <> data_element_tag.to_string(tag)
      <> ", name: "
      <> dictionary.tag_name(tag, None)
      <> ", vr: "
      <> value_representation.to_string(vr)

    SequenceDelimiter -> "SequenceDelimiter"

    SequenceItemStart -> "SequenceItemStart"

    SequenceItemDelimiter -> "SequenceItemDelimiter"

    PixelDataItem(length) ->
      "PixelDataItem: " <> int.to_string(length) <> " bytes"

    End -> "End"
  }
}

/// Returns whether this DICOM P10 part is part of the file header or File Meta
/// Information prior to the main data set, i.e. is it a
/// `FilePreambleAndDICMPrefix` or `FileMetaInformation` part.
///
pub fn is_header_part(part: P10Part) -> Bool {
  case part {
    FilePreambleAndDICMPrefix(..) | FileMetaInformation(..) -> True
    _ -> False
  }
}

/// Converts all the data elements in a data set directly to DICOM P10 parts.
/// Each part is returned via a callback.
///
pub fn data_elements_to_parts(
  data_set: DataSet,
  context: a,
  part_callback: fn(a, P10Part) -> Result(a, b),
) -> Result(a, b) {
  data_set
  |> data_set.try_fold(context, fn(context, tag, value) {
    data_element_to_parts(tag, value, context, part_callback)
  })
}

/// Converts a DICOM data element to DICOM P10 parts. Each part is returned via
/// a callback.
///
pub fn data_element_to_parts(
  tag: DataElementTag,
  value: DataElementValue,
  context: a,
  part_callback: fn(a, P10Part) -> Result(a, b),
) -> Result(a, b) {
  let vr = data_element_value.value_representation(value)

  case data_element_value.bytes(value) {
    // For values that have their bytes directly available write them out as-is
    Ok(bytes) -> {
      let header_part = DataElementHeader(tag, vr, bit_array.byte_size(bytes))
      use context <- result.try(part_callback(context, header_part))

      DataElementValueBytes(vr, bytes, bytes_remaining: 0)
      |> part_callback(context, _)
    }

    Error(_) ->
      case data_element_value.encapsulated_pixel_data(value) {
        // For encapsulated pixel data, write all of the items individually,
        // followed by a sequence delimiter
        Ok(items) -> {
          let header_part = SequenceStart(tag, vr)
          use context <- result.try(part_callback(context, header_part))

          let context =
            items
            |> list.try_fold(context, fn(context, item) {
              let length = bit_array.byte_size(item)
              let item_header_part = PixelDataItem(length)
              let context = part_callback(context, item_header_part)
              use context <- result.try(context)

              let value_bytes_part = DataElementValueBytes(vr, item, 0)
              part_callback(context, value_bytes_part)
            })
          use context <- result.try(context)

          // Write delimiter for the encapsulated pixel data sequence
          part_callback(context, SequenceDelimiter)
        }

        Error(_) -> {
          // For sequences, write the item data sets recursively, followed by a
          // sequence delimiter
          let assert Ok(items) = data_element_value.sequence_items(value)

          let header_part = SequenceStart(tag, vr)
          use context <- result.try(part_callback(context, header_part))

          let context =
            items
            |> list.try_fold(context, fn(context, item) {
              let item_start_part = SequenceItemStart
              let context = part_callback(context, item_start_part)
              use context <- result.try(context)

              use context <- result.try(data_elements_to_parts(
                item,
                context,
                part_callback,
              ))

              // Write delimiter for the item
              let item_delimiter_part = SequenceItemDelimiter
              part_callback(context, item_delimiter_part)
            })

          use context <- result.try(context)

          // Write delimiter for the sequence
          part_callback(context, SequenceDelimiter)
        }
      }
  }
}
