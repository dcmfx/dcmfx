//// Extracts frames of pixel data present in a data set.

import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/dictionary
import dcmfx_core/transfer_syntax.{type TransferSyntax}
import dcmfx_p10/p10_write
import dcmfx_pixel_data/pixel_data_filter.{type PixelDataFilterError}
import dcmfx_pixel_data/pixel_data_frame.{type PixelDataFrame}
import gleam/list
import gleam/pair
import gleam/result

/// Returns the frames of pixel data present in a data set.
///
/// The *'(7FE0,0010) Pixel Data'* data element must be present in the data set,
/// and the *'(0028,0008) Number of Frames'*, *'(7FE0,0001) Extended Offset
/// Table'*, and *'(7FE0,0002) Extended Offset Table Lengths'* data elements are
/// used when present and relevant.
///
pub fn get_pixel_data_frames(
  data_set: DataSet,
) -> Result(List(PixelDataFrame), PixelDataFilterError) {
  // Create a new data set containing only the data elements needed by the pixel
  // data filter. This avoids calling `data_elements_to_tokens()` on the
  // whole data set.
  let ds =
    [
      dictionary.number_of_frames.tag,
      dictionary.extended_offset_table.tag,
      dictionary.extended_offset_table_lengths.tag,
      dictionary.pixel_data.tag,
    ]
    |> list.fold(data_set.new(), fn(ds, tag) {
      case data_set.get_value(data_set, tag) {
        Ok(value) -> data_set.insert(ds, tag, value)
        _ -> ds
      }
    })

  // Pass the cut down data set through a pixel data filter and collect all
  // emitted frames
  let context = #([], pixel_data_filter.new())
  ds
  |> p10_write.data_set_to_tokens(context, fn(context, token) {
    let #(frames, filter) = context

    use #(new_frames, filter) <- result.map(pixel_data_filter.add_token(
      filter,
      token,
    ))

    let frames = list.append(frames, new_frames)

    #(frames, filter)
  })
  |> result.map(pair.first)
}

/// Returns the file extension to use for image data in the given transfer
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

    // Everything else uses the .bin extension as there isn't a more meaningful
    // image extension to use
    _ -> ".bin"
  }
}
