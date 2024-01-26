//// Extracts frames of pixel data present in a data set.

import bigi.{type BigInt}
import dcmfx_core/data_element_value.{type DataElementValue}
import dcmfx_core/data_error.{type DataError}
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/internal/bit_array_utils
import dcmfx_core/registry
import dcmfx_core/transfer_syntax.{type TransferSyntax}
import dcmfx_core/value_representation.{type ValueRepresentation}
import gleam/bit_array
import gleam/bool
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result

/// Returns all frames of image data present in a data set. Each returned frame
/// is made up of one or more fragments of binary data. This function handles
/// both encapsulated and non-encapsulated pixel data, and requires that the
/// '(7FE0,0010) Pixel Data' data element is present in the data set.
///
/// The '(0028,0008) Number of Frames', '(7FE0,0001) Extended Offset Table', and
/// '(7FE0,0002) Extended Offset Table Lengths' data elements are used when
/// present and relevant.
///
pub fn get_pixel_data(
  data_set: DataSet,
) -> Result(#(ValueRepresentation, List(List(BitArray))), DataError) {
  // Get the pixel data value
  let pixel_data = data_set.get_value(data_set, registry.pixel_data.tag)
  use pixel_data <- result.try(pixel_data)

  // Get the extended offset table value, if present
  let extended_offset_table = case parse_extended_offset_table(data_set) {
    Ok(table) -> Ok(Some(table))
    Error(e) ->
      case data_error.is_tag_not_present(e) {
        True -> Ok(None)
        False -> Error(e)
      }
  }
  use extended_offset_table <- result.try(extended_offset_table)

  // Get the number of frames value, if present
  let number_of_frames =
    data_set.get_int(data_set, registry.number_of_frames.tag)

  // Validate the number of frames value
  let number_of_frames = case number_of_frames {
    Ok(n) ->
      case n >= 0 {
        True -> Ok(Some(n))
        False ->
          Error(data_error.new_value_invalid(
            "Number of frames is invalid: " <> int.to_string(n),
          ))
      }
    Error(_) -> Ok(None)
  }
  use number_of_frames <- result.try(number_of_frames)

  use frames <- result.map(do_all_frames(
    pixel_data,
    number_of_frames,
    extended_offset_table,
  ))

  #(data_element_value.value_representation(pixel_data), frames)
}

fn do_all_frames(
  value: DataElementValue,
  number_of_frames: Option(Int),
  extended_offset_table: Option(ExtendedOffsetTable),
) -> Result(List(List(BitArray)), DataError) {
  let vr = data_element_value.value_representation(value)

  case data_element_value.bytes(value) {
    // Non-encapsulated OB or OW pixel data
    Ok(bytes) -> {
      use <- bool.guard(
        vr != value_representation.OtherByteString
          && vr != value_representation.OtherWordString,
        Error(data_error.new_value_not_present()),
      )

      case number_of_frames {
        None | Some(0) | Some(1) -> Ok([[bytes]])

        Some(number_of_frames) -> {
          let bytes_size = bit_array.byte_size(bytes)
          let frame_size = bytes_size / number_of_frames

          // Check that the pixel data divides exactly into the number of
          // frames. If it doesn't then it's either due to an inconsistency in
          // the pixel data, or the pixel data for a single frame is not aligned
          // on byte boundaries, which is not supported by this library. The
          // latter is possible when the bits allocated value isn't a multiple
          // of 8.
          case number_of_frames * frame_size == bytes_size {
            True -> {
              list.range(0, number_of_frames - 1)
              |> list.map(fn(i) {
                let assert Ok(bytes) =
                  bit_array.slice(bytes, i * frame_size, frame_size)

                [bytes]
              })
              |> Ok
            }

            False ->
              Error(data_error.new_value_invalid(
                "Multi-frame pixel data of length "
                <> int.to_string(bytes_size)
                <> " does not divide evenly into "
                <> int.to_string(number_of_frames)
                <> " frames",
              ))
          }
        }
      }
    }

    Error(_) ->
      case data_element_value.encapsulated_pixel_data(value) {
        Ok(items) -> {
          let empty_items_validation = case list.is_empty(items) {
            True -> Error(data_error.new_value_not_present())
            False -> Ok(Nil)
          }
          use _ <- result.try(empty_items_validation)

          case items, extended_offset_table {
            // Encapsulated pixel data with an extended offset table present in
            // the data set. There should be no basic offset table, and the
            // extended offset table is used to define the frames.
            [basic_offset_table, ..fragments], Some(extended_offset_table) -> {
              // The basic offset table must be empty when an extended offset
              // table is present
              use <- bool.guard(
                basic_offset_table != <<>>,
                Error(data_error.new_value_invalid(
                  "Encapsulated pixel data has both a basic offset table and "
                  <> "an extended offset table, but only one of these is "
                  <> "allowed",
                )),
              )

              let frames =
                fragments_to_frames_using_extended_offset_table(
                  fragments,
                  extended_offset_table,
                  bigi.zero(),
                  [],
                )
              use frames <- result.map(frames)

              list.map(frames, list.wrap)
            }

            // Encapsulated pixel data with an empty basic offset table and a
            // single fragment. The sole fragment is treated as a single frame
            // of pixel data.
            [<<>>, fragment], _ -> Ok([[fragment]])

            // Encapsulated pixel data with an empty basic offset table and
            // multiple fragments. Use the number of frames to decide what to
            // do.
            [<<>>, ..fragments], _ -> {
              let fragment_count = list.length(fragments)

              case number_of_frames {
                // Exactly one frame, so all fragments must belong to it
                None | Some(1) -> Ok([fragments])

                // The same number of fragments as frames, so each fragment is
                // its own frame
                Some(number_of_frames) if number_of_frames == fragment_count -> {
                  Ok(list.map(fragments, list.wrap))
                }

                // There is a different number of fragments and frames. Given
                // there is no basic offset table, this means there's no way to
                // allocate fragments to frames.
                _ -> {
                  Error(data_error.new_value_invalid(
                    "Encapsulated pixel data structure can't be determined",
                  ))
                }
              }
            }

            // Encapsulated pixel data with a basic offset table. A single frame
            // can be spread over one or more fragments.
            [basic_offset_table, ..items], _ -> {
              // Decode the 32-bit integers in the basic offset table data
              let basic_offset_table =
                bit_array_utils.to_uint32_list(basic_offset_table)
                |> result.replace_error(data_error.new_value_invalid(
                  "Encapsulated pixel data basic offset table is invalid",
                ))
              use basic_offset_table <- result.try(basic_offset_table)

              // Check the basic offset table is sorted
              use <- bool.guard(
                basic_offset_table != list.sort(basic_offset_table, int.compare),
                Error(data_error.new_value_invalid(
                  "Encapsulated pixel data basic offset table is not sorted",
                )),
              )

              // The first item in the basic offset table should always be zero
              let basic_offset_table = case basic_offset_table {
                [0, ..basic_offset_table] -> Ok(basic_offset_table)
                _ ->
                  Error(data_error.new_value_invalid(
                    "Encapsulated pixel data basic offset table does not "
                    <> "start at zero",
                  ))
              }
              use basic_offset_table <- result.try(basic_offset_table)

              // Turn the flat list of fragments into a list of frames
              let frames =
                fragments_to_frames_using_basic_offset_table(
                  [],
                  0,
                  basic_offset_table,
                  items,
                  [],
                )
              use frames <- result.map(frames)

              frames
            }

            _, _ -> Error(data_error.new_value_not_present())
          }
        }

        _ -> Error(data_error.new_value_not_present())
      }
  }
}

/// Takes a list of pixel data fragments and turns them into a list of frames
/// using a basic offset table. A single frame can be made up of one or more
/// fragments, and the basic offset table specifies where the frame boundaries
/// lie.
///
fn fragments_to_frames_using_basic_offset_table(
  current_frame: List(BitArray),
  offset: Int,
  basic_offset_table: List(Int),
  fragments: List(BitArray),
  acc: List(List(BitArray)),
) -> Result(List(List(BitArray)), DataError) {
  case basic_offset_table, fragments {
    // When the basic offset table has no more entries, all remaining fragments
    // constitute the final frame
    [], _ -> [fragments, ..acc] |> list.reverse |> Ok

    // Add the next fragment to the current frame
    [next_frame_offset, ..next_frame_offsets], [fragment, ..fragments] -> {
      let current_frame = [fragment, ..current_frame]

      // Increment the offset, with an extra 8 bytes for the item header
      let offset = offset + bit_array.byte_size(fragment) + 8

      // If the offset now exceeds the offset to the next frame, then the values
      // in the basic offset table are invalid
      use <- bool.guard(
        offset > next_frame_offset,
        Error(data_error.new_value_invalid(
          "Encapsulated pixel data basic offset table is malformed",
        )),
      )

      case offset == next_frame_offset {
        // If the next offset in the basic offset table has been reached then
        // this frame is now complete, so add it to the list and start gathering
        // fragments for the next frame
        True ->
          fragments_to_frames_using_basic_offset_table(
            [],
            offset,
            next_frame_offsets,
            fragments,
            [list.reverse(current_frame), ..acc],
          )

        // Keep gathering fragments for this frame
        False ->
          fragments_to_frames_using_basic_offset_table(
            current_frame,
            offset,
            basic_offset_table,
            fragments,
            acc,
          )
      }
    }

    _, _ ->
      Error(data_error.new_value_invalid(
        "Encapsulated pixel data basic offset table is malformed",
      ))
  }
}

type ExtendedOffsetTableEntry {
  ExtendedOffsetTableEntry(offset: BigInt, length: BigInt)
}

type ExtendedOffsetTable =
  List(ExtendedOffsetTableEntry)

/// Returns the extended offset table present in the '(7FE0,0001) Extended
/// Offset Table', and '(7FE0,0001) Extended Offset Table Lengths' data
/// elements, if present in the data set.
///
fn parse_extended_offset_table(
  data_set: DataSet,
) -> Result(ExtendedOffsetTable, DataError) {
  // Get the value of the '(0x7FE0,0001) Extended Offset Table' data
  // element
  let extended_offset_table =
    data_set.get_value_bytes(
      data_set,
      registry.extended_offset_table.tag,
      value_representation.OtherVeryLongString,
    )
    |> result.try(fn(bytes) {
      bit_array_utils.to_uint64_list(bytes)
      |> result.replace_error(data_error.new_value_invalid(
        "Invalid Uint64 list",
      ))
    })

  use extended_offset_table <- result.try(extended_offset_table)

  // Get the value of the '(0x7FE0,0002) Extended Offset Table Lengths' data
  // element
  let extended_offset_table_lengths =
    data_set.get_value_bytes(
      data_set,
      registry.extended_offset_table_lengths.tag,
      value_representation.OtherVeryLongString,
    )
    |> result.try(fn(bytes) {
      bit_array_utils.to_uint64_list(bytes)
      |> result.replace_error(data_error.new_value_invalid(
        "Invalid Uint64 list",
      ))
    })
  use extended_offset_table_lengths <- result.try(extended_offset_table_lengths)

  // Check the two lists are of the same length
  use <- bool.guard(
    list.length(extended_offset_table)
      != list.length(extended_offset_table_lengths),
    Error(data_error.new_value_invalid(
      "Extended offset table and lengths are of different size",
    )),
  )

  // Return the extended offset table
  list.map2(
    extended_offset_table,
    extended_offset_table_lengths,
    ExtendedOffsetTableEntry,
  )
  |> Ok
}

/// Takes a list of pixel data fragments and turns them into a list of frames
/// using an extended offset table. Each frame is made up of exactly one
/// fragment.
///
fn fragments_to_frames_using_extended_offset_table(
  fragments: List(BitArray),
  extended_offset_table: ExtendedOffsetTable,
  current_offset: BigInt,
  frames: List(BitArray),
) -> Result(List(BitArray), DataError) {
  case extended_offset_table, fragments {
    [], [] -> frames |> list.reverse |> Ok

    [ExtendedOffsetTableEntry(offset, length), ..extended_offset_table],
      [fragment, ..fragments]
    -> {
      // Check the extended offset table's offset matches the offset of this
      // fragment
      use <- bool.guard(
        current_offset != offset,
        Error(data_error.new_value_invalid(
          "Encapsulated pixel data extended offset table is malformed",
        )),
      )

      // Slice the bytes for the frame from the fragment. The frame length is
      // allowed to be less than the size of the fragment, which can be used
      // in cases where the frame's data is of odd length, as fragment length
      // is always even.
      let length_int =
        bigi.to_int(length)
        |> result.replace_error(data_error.new_value_invalid(
          "Fragment length is larger than the maximum safe integer",
        ))
      use length_int <- result.try(length_int)

      case bit_array.slice(fragment, 0, length_int) {
        Ok(frame) ->
          fragments_to_frames_using_extended_offset_table(
            fragments,
            extended_offset_table,
            current_offset |> bigi.add(length) |> bigi.add(bigi.from_int(8)),
            [frame, ..frames],
          )

        // The length value in the extended offset table exceeds the length of
        // the corresponding fragment
        Error(Nil) ->
          Error(data_error.new_value_invalid(
            "Encapsulated pixel data extended offset table length of "
            <> bigi.to_string(length)
            <> " bytes exceeds the fragment length of "
            <> int.to_string(bit_array.byte_size(fragment))
            <> " bytes",
          ))
      }
    }

    _, _ ->
      Error(data_error.new_value_invalid(
        "Encapsulated pixel data extended offset table size does not match "
        <> "the number of pixel data fragments",
      ))
  }
}

/// Returns the file extension to use for raw image data in the given transfer
/// syntax. If there is no sensible file extension to use then `".bin"` is
/// returned.
///
pub fn file_extension_for_transfer_syntax(ts: TransferSyntax) -> String {
  case ts {
    // JPEG and JPEG Lossless use the .jpg extension
    ts
      if ts == transfer_syntax.jpeg_baseline_8bit
      || ts == transfer_syntax.jpeg_extended_12bit
      || ts == transfer_syntax.jpeg_lossless_non_hierarchical
      || ts == transfer_syntax.jpeg_lossless_non_hierarchical_sv1
    -> ".jpg"

    // JPEG-LS uses the .jls extension
    ts
      if ts == transfer_syntax.jpeg_ls_lossless
      || ts == transfer_syntax.jpeg_ls_lossy_near_lossless
    -> ".jls"

    // JPEG 2000 uses the .jp2 extension
    ts
      if ts == transfer_syntax.jpeg_2k_lossless_only
      || ts == transfer_syntax.jpeg_2k
      || ts == transfer_syntax.jpeg_2k_multi_component_lossless_only
      || ts == transfer_syntax.jpeg_2k_multi_component
    -> ".jp2"

    // MPEG-2 uses the .mp2 extension
    ts
      if ts == transfer_syntax.mpeg2_main_profile_main_level
      || ts == transfer_syntax.fragmentable_mpeg2_main_profile_main_level
      || ts == transfer_syntax.mpeg2_main_profile_high_level
      || ts == transfer_syntax.fragmentable_mpeg2_main_profile_high_level
    -> ".mp2"

    // MPEG-4 uses the .mp4 extension
    ts
      if ts == transfer_syntax.mpeg4_avc_h264_high_profile
      || ts == transfer_syntax.fragmentable_mpeg4_avc_h264_high_profile
      || ts == transfer_syntax.mpeg4_avc_h264_bd_compatible_high_profile
      || ts
      == transfer_syntax.fragmentable_mpeg4_avc_h264_bd_compatible_high_profile
      || ts == transfer_syntax.mpeg4_avc_h264_high_profile_for_2d_video
      || ts
      == transfer_syntax.fragmentable_mpeg4_avc_h264_high_profile_for_2d_video
      || ts == transfer_syntax.mpeg4_avc_h264_high_profile_for_3d_video
      || ts
      == transfer_syntax.fragmentable_mpeg4_avc_h264_high_profile_for_3d_video
      || ts == transfer_syntax.mpeg4_avc_h264_stereo_high_profile
      || ts == transfer_syntax.fragmentable_mpeg4_avc_h264_stereo_high_profile
    -> ".mp4"

    // HEVC/H.265 also uses the .mp4 extension
    ts
      if ts == transfer_syntax.hevc_h265_main_profile
      || ts == transfer_syntax.hevc_h265_main_10_profile
    -> ".mp4"

    // High-Throughput JPEG 2000 uses the .jph extension
    ts
      if ts == transfer_syntax.high_throughput_jpeg_2k_lossless_only
      || ts
      == transfer_syntax.high_throughput_jpeg_2k_with_rpcl_options_lossless_only
      || ts == transfer_syntax.high_throughput_jpeg_2k
    -> ".jph"

    // Everything else uses the .bin extension as there isn't a meaningful image
    // extension for them to use
    _ -> ".bin"
  }
}
