//// Extracts frames of pixel data from a stream of DICOM P10 tokens.

import bigi
import dcmfx_core/data_element_value.{type DataElementValue}
import dcmfx_core/data_error
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/dictionary
import dcmfx_core/internal/bit_array_utils
import dcmfx_core/value_representation
import dcmfx_p10/p10_error
import dcmfx_p10/p10_token.{type P10Token}
import dcmfx_p10/transforms/p10_custom_type_transform.{
  type P10CustomTypeTransform,
}
import dcmfx_p10/transforms/p10_filter_transform.{type P10FilterTransform}
import dcmfx_pixel_data/pixel_data_frame.{type PixelDataFrame}
import gleam/bit_array
import gleam/bool
import gleam/deque.{type Deque}
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result

/// This filter takes a stream of DICOM P10 tokens and emits the frames of pixel
/// data it contains. Each frame is returned with no copying of pixel data,
/// allowing for memory-efficient stream processing.
///
/// All native and encapsulated pixel data is supported, with the exception of
/// native pixel data that stores 1 bit per pixel and has a number of pixels in
/// each frame that is not divisible by eight. Such frames of pixel data do not
/// consume a whole number of bytes.
///
pub opaque type P10PixelDataFrameFilter {
  P10PixelDataFrameFilter(
    is_encapsulated: Bool,
    // Extracts the value of relevant data elements from the stream
    details: P10CustomTypeTransform(PixelDataFilterDetails),
    // Filter used to extract only the '(7FE0,0010) Pixel Data' data element
    pixel_data_filter: P10FilterTransform,
    // When reading native pixel data, the size of a single frame in bytes
    native_pixel_data_frame_size: Int,
    // Chunks of pixel data that have not yet been emitted as part of a frame
    pixel_data: Deque(BitArray),
    pixel_data_write_offset: Int,
    pixel_data_read_offset: Int,
    // The offset table used with encapsulated pixel data. This can come from
    // either the Basic Offset Table stored in the first pixel data item, or
    // from an Extended Offset Table.
    offset_table: Option(OffsetTable),
    next_frame_index: Int,
  )
}

type OffsetTable =
  List(#(Int, Option(Int)))

type PixelDataFilterDetails {
  PixelDataFilterDetails(
    number_of_frames: Option(DataElementValue),
    extended_offset_table: Option(DataElementValue),
    extended_offset_table_lengths: Option(DataElementValue),
  )
}

fn pixel_data_filter_details_from_data_set(
  data_set: DataSet,
) -> Result(PixelDataFilterDetails, data_error.DataError) {
  Ok(PixelDataFilterDetails(
    number_of_frames: data_set.delete(data_set, dictionary.number_of_frames.tag).0,
    extended_offset_table: data_set.delete(
      data_set,
      dictionary.extended_offset_table.tag,
    ).0,
    extended_offset_table_lengths: data_set.delete(
      data_set,
      dictionary.extended_offset_table_lengths.tag,
    ).0,
  ))
}

/// An error that occurred in the process of extracting frames of pixel data
/// from a stream of DICOM P10 tokens.
///
pub type P10PixelDataFrameFilterError {
  /// An error that occurred when adding a P10 token. This can happen when the
  /// stream of DICOM P10 tokens is invalid.
  P10Error(p10_error.P10Error)

  /// An error that occurred when reading the data from the data elements in the
  /// stream of DICOM P10 tokens.
  DataError(data_error.DataError)
}

/// Creates a new P10 pixel data filter to extract frames of pixel data from a
/// stream of DICOM P10 tokens.
///
pub fn new() -> P10PixelDataFrameFilter {
  let details =
    p10_custom_type_transform.new(
      [
        dictionary.number_of_frames.tag,
        dictionary.extended_offset_table.tag,
        dictionary.extended_offset_table_lengths.tag,
      ],
      pixel_data_filter_details_from_data_set,
    )

  let pixel_data_filter =
    p10_filter_transform.new(fn(tag, _vr, _length, location) {
      tag == dictionary.pixel_data.tag && location == []
    })

  P10PixelDataFrameFilter(
    is_encapsulated: False,
    details:,
    pixel_data_filter:,
    native_pixel_data_frame_size: 0,
    pixel_data: deque.new(),
    pixel_data_write_offset: 0,
    pixel_data_read_offset: 0,
    offset_table: None,
    next_frame_index: 0,
  )
}

/// Adds the next DICOM P10 token, returning any frames of pixel data that are
/// now available.
///
pub fn add_token(
  filter: P10PixelDataFrameFilter,
  token: P10Token,
) -> Result(
  #(List(PixelDataFrame), P10PixelDataFrameFilter),
  P10PixelDataFrameFilterError,
) {
  // Add the token into the details filter if it is still active
  let details =
    p10_custom_type_transform.add_token(filter.details, token)
    |> result.map_error(fn(e) {
      case e {
        p10_custom_type_transform.DataError(e) -> DataError(e)
        p10_custom_type_transform.P10Error(e) -> P10Error(e)
      }
    })
  use details <- result.try(details)

  let filter = P10PixelDataFrameFilter(..filter, details:)

  use <- bool.guard(p10_token.is_header_token(token), Ok(#([], filter)))

  let #(is_pixel_data_token, pixel_data_filter) =
    p10_filter_transform.add_token(filter.pixel_data_filter, token)

  let filter = P10PixelDataFrameFilter(..filter, pixel_data_filter:)

  use <- bool.guard(!is_pixel_data_token, Ok(#([], filter)))

  process_next_pixel_data_token(filter, token)
}

fn process_next_pixel_data_token(
  filter: P10PixelDataFrameFilter,
  token: P10Token,
) -> Result(
  #(List(PixelDataFrame), P10PixelDataFrameFilter),
  P10PixelDataFrameFilterError,
) {
  case token {
    // The start of native pixel data
    p10_token.DataElementHeader(length:, ..) -> {
      // Check that the pixel data length divides evenly into the number of
      // frames
      use number_of_frames <- result.try(get_number_of_frames(filter))

      use <- bool.guard(
        number_of_frames != 0 && length % number_of_frames != 0,
        Error(
          DataError(data_error.new_value_invalid(
            "Multi-frame pixel data of length "
            <> int.to_string(length)
            <> " bytes does not divide evenly into "
            <> int.to_string(number_of_frames)
            <> " frames",
          )),
        ),
      )

      // Store the size of native pixel data frames
      let native_pixel_data_frame_size = case number_of_frames == 0 {
        True -> 0
        False -> length / number_of_frames
      }

      let filter =
        P10PixelDataFrameFilter(
          ..filter,
          is_encapsulated: False,
          native_pixel_data_frame_size:,
        )

      Ok(#([], filter))
    }

    // The start of encapsulated pixel data
    p10_token.SequenceStart(..) -> {
      let filter = P10PixelDataFrameFilter(..filter, is_encapsulated: True)

      Ok(#([], filter))
    }

    // The end of the encapsulated pixel data
    p10_token.SequenceDelimiter(..) -> {
      // If there is any remaining pixel data then emit it as a final frame
      let frames = case filter.pixel_data |> deque.is_empty() {
        True -> Ok([])

        False -> {
          let frame_index = filter.next_frame_index
          let filter =
            P10PixelDataFrameFilter(..filter, next_frame_index: frame_index + 1)

          let frame =
            filter.pixel_data
            |> deque.to_list
            |> list.fold(
              pixel_data_frame.new(frame_index),
              pixel_data_frame.push_fragment,
            )

          // If the frame has a length specified then apply it
          let frame = case filter.offset_table {
            Some(offset_table) -> {
              case offset_table {
                [#(_, Some(frame_length)), ..] ->
                  apply_length_to_frame(frame, frame_length)
                _ -> Ok(frame)
              }
            }
            None -> Ok(frame)
          }
          use frame <- result.map(frame)

          [frame]
        }
      }
      use frames <- result.map(frames)

      #(frames, filter)
    }

    // The start of a new encapsulated pixel data item. The size of an item
    // header is 8 bytes, and this needs to be included in the current offset.
    p10_token.PixelDataItem(..) -> {
      let filter =
        P10PixelDataFrameFilter(
          ..filter,
          pixel_data_write_offset: filter.pixel_data_write_offset + 8,
        )

      Ok(#([], filter))
    }

    p10_token.DataElementValueBytes(data:, bytes_remaining:, ..) -> {
      let pixel_data = filter.pixel_data |> deque.push_back(data)
      let pixel_data_write_offset =
        filter.pixel_data_write_offset + bit_array.byte_size(data)

      let filter =
        P10PixelDataFrameFilter(..filter, pixel_data:, pixel_data_write_offset:)

      case filter.is_encapsulated {
        True ->
          case bytes_remaining {
            0 -> get_pending_encapsulated_frames(filter)
            _ -> Ok(#([], filter))
          }

        False ->
          case filter.native_pixel_data_frame_size > 0 {
            True -> get_pending_native_frames(filter, [])
            False -> Ok(#([], filter))
          }
      }
    }

    _ -> Ok(#([], filter))
  }
}

/// Returns the value for *'(0028,0008) Number of Frames'* data element.
///
fn get_number_of_frames(
  filter: P10PixelDataFrameFilter,
) -> Result(Int, P10PixelDataFrameFilterError) {
  case p10_custom_type_transform.get_output(filter.details) {
    Some(PixelDataFilterDetails(number_of_frames: Some(number_of_frames), ..)) -> {
      let number_of_frames =
        data_element_value.get_int(number_of_frames)
        |> result.map_error(DataError)
      use number_of_frames <- result.try(number_of_frames)

      use <- bool.guard(
        number_of_frames < 0,
        Error(
          DataError(data_error.new_value_invalid(
            "Invalid number of frames value: "
            <> int.to_string(number_of_frames),
          )),
        ),
      )

      Ok(number_of_frames)
    }

    _ -> Ok(1)
  }
}

/// Consumes the native pixel data for as many frames as possible and returns
/// them.
///
fn get_pending_native_frames(
  filter: P10PixelDataFrameFilter,
  frames: List(PixelDataFrame),
) -> Result(
  #(List(PixelDataFrame), P10PixelDataFrameFilter),
  P10PixelDataFrameFilterError,
) {
  case
    filter.pixel_data_write_offset - filter.pixel_data_read_offset
    < filter.native_pixel_data_frame_size
  {
    True -> Ok(#(list.reverse(frames), filter))

    False -> {
      let frame_index = filter.next_frame_index
      let filter =
        P10PixelDataFrameFilter(..filter, next_frame_index: frame_index + 1)

      let #(frame, filter) =
        get_pending_native_frame(filter, pixel_data_frame.new(frame_index))
      get_pending_native_frames(filter, [frame, ..frames])
    }
  }
}

fn get_pending_native_frame(
  filter: P10PixelDataFrameFilter,
  frame: PixelDataFrame,
) -> #(PixelDataFrame, P10PixelDataFrameFilter) {
  let frame_size = filter.native_pixel_data_frame_size
  let frame_length = pixel_data_frame.length(frame)

  case pixel_data_frame.length(frame) < frame_size {
    True -> {
      let assert Ok(#(chunk, pixel_data)) =
        filter.pixel_data |> deque.pop_front()
      let chunk_length = bit_array.byte_size(chunk)

      let filter = P10PixelDataFrameFilter(..filter, pixel_data:)

      case chunk_length <= frame_size - frame_length {
        // If the whole of this chunk is needed for the next frame then add it
        // to the frame
        True -> {
          let frame = pixel_data_frame.push_fragment(frame, chunk)

          let filter =
            P10PixelDataFrameFilter(
              ..filter,
              pixel_data:,
              pixel_data_read_offset: filter.pixel_data_read_offset
                + chunk_length,
            )

          get_pending_native_frame(filter, frame)
        }

        // Otherwise, take just the part of this chunk of pixel data needed for
        // the frame
        False -> {
          let length = frame_size - frame_length

          let assert Ok(fragment) = bit_array.slice(chunk, 0, length)
          let frame = frame |> pixel_data_frame.push_fragment(fragment)

          // Put the unused token of the chunk back on so it can be used by the
          // next frame
          let assert Ok(chunk) =
            bit_array.slice(chunk, length, chunk_length - length)
          let pixel_data = filter.pixel_data |> deque.push_front(chunk)

          let filter =
            P10PixelDataFrameFilter(
              ..filter,
              pixel_data:,
              pixel_data_read_offset: filter.pixel_data_read_offset + length,
            )

          #(frame, filter)
        }
      }
    }

    False -> #(frame, filter)
  }
}

/// Consumes the encapsulated pixel data for as many frames as possible and
/// returns them.
///
fn get_pending_encapsulated_frames(
  filter: P10PixelDataFrameFilter,
) -> Result(
  #(List(PixelDataFrame), P10PixelDataFrameFilter),
  P10PixelDataFrameFilterError,
) {
  case filter.offset_table {
    // If the Basic Offset Table hasn't been read yet, read it now that the
    // first pixel data item is complete
    None -> {
      use offset_table <- result.try(read_offset_table(filter))

      let filter =
        P10PixelDataFrameFilter(
          ..filter,
          pixel_data: deque.new(),
          pixel_data_write_offset: 0,
          pixel_data_read_offset: 0,
          offset_table: Some(offset_table),
        )

      Ok(#([], filter))
    }

    Some(offset_table) ->
      case offset_table {
        [] -> {
          use number_of_frames <- result.map(get_number_of_frames(filter))

          // If the offset table is empty and there is more than one frame
          // then each pixel data item is treated as a single frame
          case number_of_frames > 1 {
            True -> {
              let frame_index = filter.next_frame_index
              let filter =
                P10PixelDataFrameFilter(
                  ..filter,
                  next_frame_index: frame_index + 1,
                )

              let frame =
                filter.pixel_data
                |> deque.to_list
                |> list.fold(
                  pixel_data_frame.new(frame_index),
                  fn(frame, chunk) {
                    pixel_data_frame.push_fragment(frame, chunk)
                  },
                )

              let filter =
                P10PixelDataFrameFilter(
                  ..filter,
                  pixel_data: deque.new(),
                  pixel_data_read_offset: filter.pixel_data_write_offset,
                )

              #([frame], filter)
            }

            False -> #([], filter)
          }
        }

        // Use the offset table to determine what frames to emit
        offset_table ->
          get_pending_encapsulated_frames_using_offset_table(
            filter,
            offset_table,
            [],
          )
      }
  }
}

fn get_pending_encapsulated_frames_using_offset_table(
  filter: P10PixelDataFrameFilter,
  offset_table: OffsetTable,
  frames: List(PixelDataFrame),
) -> Result(
  #(List(PixelDataFrame), P10PixelDataFrameFilter),
  P10PixelDataFrameFilterError,
) {
  case offset_table {
    [#(_, frame_length), #(offset, _), ..] -> {
      use <- bool.guard(
        filter.pixel_data_write_offset < offset,
        Ok(#(frames, filter)),
      )

      let frame_index = filter.next_frame_index
      let filter =
        P10PixelDataFrameFilter(..filter, next_frame_index: frame_index + 1)

      let #(frame, filter) =
        get_pending_encapsulated_frame(
          filter,
          pixel_data_frame.new(frame_index),
          offset,
        )

      let assert Ok(offset_table) = list.rest(offset_table)

      let filter =
        P10PixelDataFrameFilter(..filter, offset_table: Some(offset_table))

      // Check that the frame ended exactly on the expected offset
      use <- bool.guard(
        filter.pixel_data_read_offset != offset,
        Error(
          DataError(data_error.new_value_invalid(
            "Pixel data offset table is malformed",
          )),
        ),
      )

      // If this frame has a length specified then validate and apply it
      let frame = case frame_length {
        Some(frame_length) -> apply_length_to_frame(frame, frame_length)
        None -> Ok(frame)
      }
      use frame <- result.try(frame)

      get_pending_encapsulated_frames_using_offset_table(filter, offset_table, [
        frame,
        ..frames
      ])
    }

    _ -> Ok(#(list.reverse(frames), filter))
  }
}

fn get_pending_encapsulated_frame(
  filter: P10PixelDataFrameFilter,
  frame: PixelDataFrame,
  next_offset: Int,
) -> #(PixelDataFrame, P10PixelDataFrameFilter) {
  case filter.pixel_data_read_offset < next_offset {
    True ->
      case deque.pop_front(filter.pixel_data) {
        Ok(#(chunk, pixel_data)) -> {
          let frame = frame |> pixel_data_frame.push_fragment(chunk)
          let pixel_data_read_offset =
            filter.pixel_data_read_offset + 8 + bit_array.byte_size(chunk)

          let filter =
            P10PixelDataFrameFilter(
              ..filter,
              pixel_data:,
              pixel_data_read_offset:,
            )

          get_pending_encapsulated_frame(filter, frame, next_offset)
        }

        Error(Nil) -> #(frame, filter)
      }

    False -> #(frame, filter)
  }
}

fn read_offset_table(
  filter: P10PixelDataFrameFilter,
) -> Result(OffsetTable, P10PixelDataFrameFilterError) {
  use basic_offset_table <- result.try(read_basic_offset_table(filter))
  use extended_offset_table <- result.try(read_extended_offset_table(filter))

  // If the Basic Offset Table is empty then use the Extended Offset Table if
  // present. If neither are present then there is no offset table.
  case basic_offset_table {
    [] -> extended_offset_table |> option.unwrap([]) |> Ok
    _ -> {
      // Validate that the Extended Offset Table is empty. Ref: PS3.5 A.4.
      use <- bool.guard(
        option.is_some(extended_offset_table),
        Error(
          DataError(data_error.new_value_invalid(
            "Extended Offset Table must be absent when there is a Basic Offset "
            <> "Table",
          )),
        ),
      )

      Ok(basic_offset_table)
    }
  }
}

fn read_basic_offset_table(
  filter: P10PixelDataFrameFilter,
) -> Result(OffsetTable, P10PixelDataFrameFilterError) {
  // Read Basic Offset Table data into a buffer
  let offset_table_data =
    filter.pixel_data
    |> deque.to_list
    |> bit_array.concat

  use <- bool.guard(offset_table_data == <<>>, Ok([]))

  // Read data into u32 values
  let offsets =
    bit_array_utils.to_uint32_list(offset_table_data)
    |> result.map_error(fn(_) {
      DataError(data_error.new_value_invalid(
        "Basic Offset Table length is not a multiple of 4",
      ))
    })
  use offsets <- result.try(offsets)

  // Check that the first offset is zero. Ref: PS3.5 A.4.
  use <- bool.guard(
    list.first(offsets) != Ok(0),
    Error(
      DataError(data_error.new_value_invalid(
        "Basic Offset Table first value must be zero",
      )),
    ),
  )

  // Check that the offsets are sorted
  use <- bool.guard(
    !is_list_sorted(offsets),
    Error(
      DataError(data_error.new_value_invalid(
        "Basic Offset Table values are not sorted",
      )),
    ),
  )

  offsets
  |> list.map(fn(offset) { #(offset, None) })
  |> Ok
}

fn read_extended_offset_table(
  filter: P10PixelDataFrameFilter,
) -> Result(Option(OffsetTable), P10PixelDataFrameFilterError) {
  case p10_custom_type_transform.get_output(filter.details) {
    Some(PixelDataFilterDetails(
      extended_offset_table: Some(extended_offset_table),
      extended_offset_table_lengths: Some(extended_offset_table_lengths),
      ..,
    )) -> {
      // Get the value of the '(0x7FE0,0001) Extended Offset Table' data
      // element
      let extended_offset_table =
        extended_offset_table
        |> data_element_value.vr_bytes([
          value_representation.OtherVeryLongString,
        ])
        |> result.then(fn(bytes) {
          bit_array_utils.to_uint64_list(bytes)
          |> result.replace_error(data_error.new_value_invalid(
            "Extended Offset Table has invalid size",
          ))
        })
        |> result.map_error(DataError)
      use extended_offset_table <- result.try(extended_offset_table)

      let extended_offset_table =
        extended_offset_table
        |> list.map(bigi.to_int)
        |> result.all
        |> result.replace_error(
          DataError(data_error.new_value_invalid(
            "Extended Offset Table has a value greater than 2^53 - 1",
          )),
        )
      use extended_offset_table <- result.try(extended_offset_table)

      // Get the value of the '(0x7FE0,0002) Extended Offset Table Lengths' data
      // element
      let extended_offset_table_lengths =
        extended_offset_table_lengths
        |> data_element_value.vr_bytes([
          value_representation.OtherVeryLongString,
        ])
        |> result.then(fn(bytes) {
          bit_array_utils.to_uint64_list(bytes)
          |> result.replace_error(data_error.new_value_invalid(
            "Extended Offset Table Lengths has invalid size",
          ))
        })
        |> result.map_error(DataError)
      use extended_offset_table_lengths <- result.try(
        extended_offset_table_lengths,
      )

      let extended_offset_table_lengths =
        extended_offset_table_lengths
        |> list.map(bigi.to_int)
        |> result.all
        |> result.replace_error(
          DataError(data_error.new_value_invalid(
            "Extended Offset Table Lengths has a value greater than 2^53 - 1",
          )),
        )
      use extended_offset_table_lengths <- result.try(
        extended_offset_table_lengths,
      )

      // Check the two are of the same length
      use <- bool.guard(
        list.length(extended_offset_table)
          != list.length(extended_offset_table_lengths),
        Error(
          DataError(data_error.new_value_invalid(
            "Extended Offset Table and Lengths don't have the same number of items",
          )),
        ),
      )

      // Check that the first offset is zero
      use <- bool.guard(
        list.first(extended_offset_table) |> result.unwrap(0) != 0,
        Error(
          DataError(data_error.new_value_invalid(
            "Extended Offset Table first value must be zero",
          )),
        ),
      )

      // Check that the offsets are sorted
      use <- bool.guard(
        !is_list_sorted(extended_offset_table),
        Error(
          DataError(data_error.new_value_invalid(
            "Extended Offset Table values are not sorted",
          )),
        ),
      )

      // Return the offset table
      list.map2(
        extended_offset_table,
        extended_offset_table_lengths,
        fn(offset, length) { #(offset, Some(length)) },
      )
      |> Some
      |> Ok
    }

    _ -> Ok(None)
  }
}

fn apply_length_to_frame(
  frame: PixelDataFrame,
  frame_length: Int,
) -> Result(PixelDataFrame, P10PixelDataFrameFilterError) {
  case pixel_data_frame.length(frame) {
    len if len == frame_length -> Ok(frame)

    len if len > frame_length ->
      Ok(pixel_data_frame.drop_end_bytes(frame, len - frame_length))

    _ ->
      data_error.new_value_invalid(
        "Extended Offset Table Length value '"
        <> int.to_string(frame_length)
        <> "' is invalid for frame of length '"
        <> int.to_string(pixel_data_frame.length(frame))
        <> "'",
      )
      |> DataError
      |> Error
  }
}

fn is_list_sorted(list: List(Int)) -> Bool {
  case list {
    [a, b, ..rest] ->
      case a <= b {
        True -> is_list_sorted([b, ..rest])
        False -> False
      }

    _ -> True
  }
}
