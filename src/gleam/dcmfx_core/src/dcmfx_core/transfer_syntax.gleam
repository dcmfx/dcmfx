//// Defines all supported DICOM transfer syntaxes.

/// The value representation serialization mode of a transfer syntax. This is
/// either implicit or explicit.
///
pub type VrSerialization {
  VrImplicit
  VrExplicit
}

/// The endianness of a transfer syntax, either little endian or big endian.
///
pub type Endianness {
  LittleEndian
  BigEndian
}

/// Describes a single DICOM transfer syntax, with its name, UID, how it
/// serializes value representations (implicit vs explicit), whether it is zlib
/// deflated, and whether it stores its pixel data as encapsulated.
///
pub type TransferSyntax {
  TransferSyntax(
    name: String,
    uid: String,
    vr_serialization: VrSerialization,
    endianness: Endianness,
    is_deflated: Bool,
    is_encapsulated: Bool,
  )
}

/// The 'Implicit VR Little Endian' transfer syntax.
///
pub const implicit_vr_little_endian = TransferSyntax(
  name: "Implicit VR Little Endian",
  uid: "1.2.840.10008.1.2",
  vr_serialization: VrImplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: False,
)

/// The 'Explicit VR Little Endian' transfer syntax.
///
pub const explicit_vr_little_endian = TransferSyntax(
  name: "Explicit VR Little Endian",
  uid: "1.2.840.10008.1.2.1",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: False,
)

/// The 'Encapsulated Uncompressed Explicit VR Little Endian' transfer syntax.
///
pub const encapsulated_uncompressed_explicit_vr_little_endian = TransferSyntax(
  name: "Encapsulated Uncompressed Explicit VR Little Endian",
  uid: "1.2.840.10008.1.2.1.98",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'Deflated Explicit VR Little Endian' transfer syntax.
///
pub const deflated_explicit_vr_little_endian = TransferSyntax(
  name: "Deflated Explicit VR Little Endian",
  uid: "1.2.840.10008.1.2.1.99",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: True,
  is_encapsulated: False,
)

/// The 'Explicit VR Big Endian' transfer syntax.
///
pub const explicit_vr_big_endian = TransferSyntax(
  name: "Explicit VR Big Endian",
  uid: "1.2.840.10008.1.2.2",
  vr_serialization: VrExplicit,
  endianness: BigEndian,
  is_deflated: False,
  is_encapsulated: False,
)

/// The 'JPEG Baseline (Process 1)' transfer syntax.
///
pub const jpeg_baseline_8bit = TransferSyntax(
  name: "JPEG Baseline (Process 1)",
  uid: "1.2.840.10008.1.2.4.50",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPEG Extended (Process 2 & 4)' transfer syntax.
///
pub const jpeg_extended_12bit = TransferSyntax(
  name: "JPEG Extended (Process 2 & 4)",
  uid: "1.2.840.10008.1.2.4.51",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPEG Lossless, Non-Hierarchical (Process 14)' transfer syntax.
///
pub const jpeg_lossless_non_hierarchical = TransferSyntax(
  name: "JPEG Lossless, Non-Hierarchical (Process 14)",
  uid: "1.2.840.10008.1.2.4.57",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPEG Lossless, Non-Hierarchical, First-Order Prediction (Process 14
/// [Selection Value 1])' transfer syntax.
///
pub const jpeg_lossless_non_hierarchical_sv1 = TransferSyntax(
  name: "JPEG Lossless, Non-Hierarchical, First-Order Prediction (Process 14 [Selection Value 1])",
  uid: "1.2.840.10008.1.2.4.70",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPEG-LS Lossless Image Compression' transfer syntax.
///
pub const jpeg_ls_lossless = TransferSyntax(
  name: "JPEG-LS Lossless Image Compression",
  uid: "1.2.840.10008.1.2.4.80",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPEG-LS Lossy (Near-Lossless) Image Compression' transfer syntax.
///
pub const jpeg_ls_lossy_near_lossless = TransferSyntax(
  name: "JPEG-LS Lossy (Near-Lossless) Image Compression",
  uid: "1.2.840.10008.1.2.4.81",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPEG 2000 Image Compression (Lossless Only)' transfer syntax.
///
pub const jpeg_2k_lossless_only = TransferSyntax(
  name: "JPEG 2000 Image Compression (Lossless Only)",
  uid: "1.2.840.10008.1.2.4.90",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPEG 2000 Image Compression' transfer syntax.
///
pub const jpeg_2k = TransferSyntax(
  name: "JPEG 2000 Image Compression",
  uid: "1.2.840.10008.1.2.4.91",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPEG 2000 Part 2 Multi-component Image Compression (Lossless Only)'
/// transfer syntax.
///
pub const jpeg_2k_multi_component_lossless_only = TransferSyntax(
  name: "JPEG 2000 Part 2 Multi-component Image Compression (Lossless Only)",
  uid: "1.2.840.10008.1.2.4.92",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPEG 2000 Part 2 Multi-component Image Compression' transfer syntax.
///
pub const jpeg_2k_multi_component = TransferSyntax(
  name: "JPEG 2000 Part 2 Multi-component Image Compression",
  uid: "1.2.840.10008.1.2.4.93",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPIP Referenced' transfer syntax.
///
pub const jpip_referenced = TransferSyntax(
  name: "JPIP Referenced",
  uid: "1.2.840.10008.1.2.4.94",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: False,
)

/// The 'JPIP Referenced Deflate' transfer syntax.
///
pub const jpip_referenced_deflate = TransferSyntax(
  name: "JPIP Referenced Deflate",
  uid: "1.2.840.10008.1.2.4.95",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: True,
  is_encapsulated: False,
)

/// The 'MPEG2 Main Profile @ Main Level' transfer syntax.
///
pub const mpeg2_main_profile_main_level = TransferSyntax(
  name: "MPEG2 Main Profile @ Main Level",
  uid: "1.2.840.10008.1.2.4.100",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'Fragmentable MPEG2 Main Profile @ Main Level' transfer syntax.
///
pub const fragmentable_mpeg2_main_profile_main_level = TransferSyntax(
  name: "MPEG2 Main Profile @ Main Level",
  uid: "1.2.840.10008.1.2.4.100.1",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'MPEG2 Main Profile @ High Level' transfer syntax.
///
pub const mpeg2_main_profile_high_level = TransferSyntax(
  name: "MPEG2 Main Profile @ High Level",
  uid: "1.2.840.10008.1.2.4.101",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'Fragmentable MPEG2 Main Profile @ High Level' transfer syntax.
///
pub const fragmentable_mpeg2_main_profile_high_level = TransferSyntax(
  name: "Fragmentable MPEG2 Main Profile @ High Level",
  uid: "1.2.840.10008.1.2.4.101.1",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'MPEG-4 AVC/H.264 High Profile / Level 4.1' transfer syntax.
///
pub const mpeg4_avc_h264_high_profile = TransferSyntax(
  name: "MPEG-4 AVC/H.264 High Profile / Level 4.1",
  uid: "1.2.840.10008.1.2.4.102",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.1' transfer
/// syntax.
///
pub const fragmentable_mpeg4_avc_h264_high_profile = TransferSyntax(
  name: "Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.1",
  uid: "1.2.840.10008.1.2.4.102.1",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1' transfer
/// syntax.
///
pub const mpeg4_avc_h264_bd_compatible_high_profile = TransferSyntax(
  name: "MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1",
  uid: "1.2.840.10008.1.2.4.103",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'Fragmentable MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1'
/// transfer syntax.
///
pub const fragmentable_mpeg4_avc_h264_bd_compatible_high_profile = TransferSyntax(
  name: "Fragmentable MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1",
  uid: "1.2.840.10008.1.2.4.103.1",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video' transfer
/// syntax.
///
pub const mpeg4_avc_h264_high_profile_for_2d_video = TransferSyntax(
  name: "MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video",
  uid: "1.2.840.10008.1.2.4.104",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video'
/// transfer syntax.
///
pub const fragmentable_mpeg4_avc_h264_high_profile_for_2d_video = TransferSyntax(
  name: "Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video",
  uid: "1.2.840.10008.1.2.4.104.1",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video' transfer
/// syntax.
///
pub const mpeg4_avc_h264_high_profile_for_3d_video = TransferSyntax(
  name: "MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video",
  uid: "1.2.840.10008.1.2.4.105",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video'
/// transfer syntax.
///
pub const fragmentable_mpeg4_avc_h264_high_profile_for_3d_video = TransferSyntax(
  name: "Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video",
  uid: "1.2.840.10008.1.2.4.105.1",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2' transfer syntax.
///
pub const mpeg4_avc_h264_stereo_high_profile = TransferSyntax(
  name: "MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2",
  uid: "1.2.840.10008.1.2.4.106",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'Fragmentable MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2' transfer
/// syntax.
///
pub const fragmentable_mpeg4_avc_h264_stereo_high_profile = TransferSyntax(
  name: "Fragmentable MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2",
  uid: "1.2.840.10008.1.2.4.106.1",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'HEVC/H.265 Main Profile / Level 5.1' transfer syntax.
///
pub const hevc_h265_main_profile = TransferSyntax(
  name: "HEVC/H.265 Main Profile / Level 5.1",
  uid: "1.2.840.10008.1.2.4.107",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'HEVC/H.265 Main 10 Profile / Level 5.1' transfer syntax.
///
pub const hevc_h265_main_10_profile = TransferSyntax(
  name: "HEVC/H.265 Main 10 Profile / Level 5.1",
  uid: "1.2.840.10008.1.2.4.108",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPEG XL Lossless' transfer syntax.
///
pub const jpeg_xl_lossless = TransferSyntax(
  name: "JPEG XL Lossless",
  uid: "1.2.840.10008.1.2.4.110",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPEG XL JPEG Recompression' transfer syntax.
///
pub const jpeg_xl_jpeg_recompression = TransferSyntax(
  name: "JPEG XL JPEG Recompression",
  uid: "1.2.840.10008.1.2.4.111",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPEG XL' transfer syntax.
///
pub const jpeg_xl = TransferSyntax(
  name: "JPEG XL",
  uid: "1.2.840.10008.1.2.4.112",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'High-Throughput JPEG 2000 (Lossless Only)' transfer syntax.
///
pub const high_throughput_jpeg_2k_lossless_only = TransferSyntax(
  name: "High-Throughput JPEG 2000 (Lossless Only)",
  uid: "1.2.840.10008.1.2.4.201",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'High-Throughput JPEG 2000 with RPCL Options (Lossless Only)' transfer
/// syntax.
///
pub const high_throughput_jpeg_2k_with_rpcl_options_lossless_only = TransferSyntax(
  name: "High-Throughput JPEG 2000 with RPCL Options (Lossless Only)",
  uid: "1.2.840.10008.1.2.4.202",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'High-Throughput JPEG 2000' transfer syntax.
///
pub const high_throughput_jpeg_2k = TransferSyntax(
  name: "High-Throughput JPEG 2000",
  uid: "1.2.840.10008.1.2.4.203",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'JPIP HTJ2K Referenced' transfer syntax.
///
pub const jpip_high_throughput_jpeg_2k_referenced = TransferSyntax(
  name: "JPIP HTJ2K Referenced",
  uid: "1.2.840.10008.1.2.4.204",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: False,
)

/// The 'JPIP HTJ2K Referenced Deflate' transfer syntax.
///
pub const jpip_high_throughput_jpeg_2k_referenced_deflate = TransferSyntax(
  name: "JPIP HTJ2K Referenced Deflate",
  uid: "1.2.840.10008.1.2.4.205",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: True,
  is_encapsulated: False,
)

/// The 'RLE Lossless' transfer syntax.
///
pub const rle_lossless = TransferSyntax(
  name: "RLE Lossless",
  uid: "1.2.840.10008.1.2.5",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'SMPTE ST 2110-20 Uncompressed Progressive Active Video' transfer
/// syntax.
///
pub const smpte_st_2110_20_uncompressed_progressive_active_video = TransferSyntax(
  name: "SMPTE ST 2110-20 Uncompressed Progressive Active Video",
  uid: "1.2.840.10008.1.2.7.1",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'SMPTE ST 2110-20 Uncompressed Interlaced Active Video' transfer syntax.
///
pub const smpte_st_2110_20_uncompressed_interlaced_active_video = TransferSyntax(
  name: "SMPTE ST 2110-20 Uncompressed Interlaced Active Video",
  uid: "1.2.840.10008.1.2.7.2",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// The 'SMPTE ST 2110-30 PCM Audio' transfer syntax.
///
pub const smpte_st_2110_30_pcm_audio = TransferSyntax(
  name: "SMPTE ST 2110-30 PCM Audio",
  uid: "1.2.840.10008.1.2.7.3",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: False,
)

/// The 'Deflated Image Frame Compression' transfer syntax.
///
pub const deflated_image_frame_compression = TransferSyntax(
  name: "Deflated Image Frame Compression",
  uid: "1.2.840.10008.1.2.8.1",
  vr_serialization: VrExplicit,
  endianness: LittleEndian,
  is_deflated: False,
  is_encapsulated: True,
)

/// Returns a list of the supported transfer syntaxes.
///
pub const all = [
  implicit_vr_little_endian,
  explicit_vr_little_endian,
  encapsulated_uncompressed_explicit_vr_little_endian,
  deflated_explicit_vr_little_endian,
  explicit_vr_big_endian,
  jpeg_baseline_8bit,
  jpeg_extended_12bit,
  jpeg_lossless_non_hierarchical,
  jpeg_lossless_non_hierarchical_sv1,
  jpeg_ls_lossless,
  jpeg_ls_lossy_near_lossless,
  jpeg_2k_lossless_only,
  jpeg_2k,
  jpeg_2k_multi_component_lossless_only,
  jpeg_2k_multi_component,
  jpip_referenced,
  jpip_referenced_deflate,
  mpeg2_main_profile_main_level,
  fragmentable_mpeg2_main_profile_main_level,
  mpeg2_main_profile_high_level,
  fragmentable_mpeg2_main_profile_high_level,
  mpeg4_avc_h264_high_profile,
  fragmentable_mpeg4_avc_h264_high_profile,
  mpeg4_avc_h264_bd_compatible_high_profile,
  fragmentable_mpeg4_avc_h264_bd_compatible_high_profile,
  mpeg4_avc_h264_high_profile_for_2d_video,
  fragmentable_mpeg4_avc_h264_high_profile_for_2d_video,
  mpeg4_avc_h264_high_profile_for_3d_video,
  fragmentable_mpeg4_avc_h264_high_profile_for_3d_video,
  mpeg4_avc_h264_stereo_high_profile,
  fragmentable_mpeg4_avc_h264_stereo_high_profile,
  hevc_h265_main_profile,
  hevc_h265_main_10_profile,
  jpeg_xl_lossless,
  jpeg_xl_jpeg_recompression,
  jpeg_xl,
  high_throughput_jpeg_2k_lossless_only,
  high_throughput_jpeg_2k_with_rpcl_options_lossless_only,
  high_throughput_jpeg_2k,
  jpip_high_throughput_jpeg_2k_referenced,
  jpip_high_throughput_jpeg_2k_referenced_deflate,
  rle_lossless,
  smpte_st_2110_20_uncompressed_progressive_active_video,
  smpte_st_2110_20_uncompressed_interlaced_active_video,
  smpte_st_2110_30_pcm_audio,
  deflated_image_frame_compression,
]

/// Returns the transfer syntax with the given UID. If the UID isn't recognized
/// then an error is returned.
///
pub fn from_uid(uid: String) -> Result(TransferSyntax, Nil) {
  case uid {
    "1.2.840.10008.1.2" -> Ok(implicit_vr_little_endian)
    "1.2.840.10008.1.2.1" -> Ok(explicit_vr_little_endian)
    "1.2.840.10008.1.2.1.98" ->
      Ok(encapsulated_uncompressed_explicit_vr_little_endian)
    "1.2.840.10008.1.2.1.99" -> Ok(deflated_explicit_vr_little_endian)
    "1.2.840.10008.1.2.2" -> Ok(explicit_vr_big_endian)
    "1.2.840.10008.1.2.4.50" -> Ok(jpeg_baseline_8bit)
    "1.2.840.10008.1.2.4.51" -> Ok(jpeg_extended_12bit)
    "1.2.840.10008.1.2.4.57" -> Ok(jpeg_lossless_non_hierarchical)
    "1.2.840.10008.1.2.4.70" -> Ok(jpeg_lossless_non_hierarchical_sv1)
    "1.2.840.10008.1.2.4.80" -> Ok(jpeg_ls_lossless)
    "1.2.840.10008.1.2.4.81" -> Ok(jpeg_ls_lossy_near_lossless)
    "1.2.840.10008.1.2.4.90" -> Ok(jpeg_2k_lossless_only)
    "1.2.840.10008.1.2.4.91" -> Ok(jpeg_2k)
    "1.2.840.10008.1.2.4.92" -> Ok(jpeg_2k_multi_component_lossless_only)
    "1.2.840.10008.1.2.4.93" -> Ok(jpeg_2k_multi_component)
    "1.2.840.10008.1.2.4.94" -> Ok(jpip_referenced)
    "1.2.840.10008.1.2.4.95" -> Ok(jpip_referenced_deflate)
    "1.2.840.10008.1.2.4.100" -> Ok(mpeg2_main_profile_main_level)
    "1.2.840.10008.1.2.4.100.1" ->
      Ok(fragmentable_mpeg2_main_profile_main_level)
    "1.2.840.10008.1.2.4.101" -> Ok(mpeg2_main_profile_high_level)
    "1.2.840.10008.1.2.4.101.1" ->
      Ok(fragmentable_mpeg2_main_profile_high_level)
    "1.2.840.10008.1.2.4.102" -> Ok(mpeg4_avc_h264_high_profile)
    "1.2.840.10008.1.2.4.102.1" -> Ok(fragmentable_mpeg4_avc_h264_high_profile)
    "1.2.840.10008.1.2.4.103" -> Ok(mpeg4_avc_h264_bd_compatible_high_profile)
    "1.2.840.10008.1.2.4.103.1" ->
      Ok(fragmentable_mpeg4_avc_h264_bd_compatible_high_profile)
    "1.2.840.10008.1.2.4.104" -> Ok(mpeg4_avc_h264_high_profile_for_2d_video)
    "1.2.840.10008.1.2.4.104.1" ->
      Ok(fragmentable_mpeg4_avc_h264_high_profile_for_2d_video)
    "1.2.840.10008.1.2.4.105" -> Ok(mpeg4_avc_h264_high_profile_for_3d_video)
    "1.2.840.10008.1.2.4.105.1" ->
      Ok(fragmentable_mpeg4_avc_h264_high_profile_for_3d_video)
    "1.2.840.10008.1.2.4.106" -> Ok(mpeg4_avc_h264_stereo_high_profile)
    "1.2.840.10008.1.2.4.106.1" ->
      Ok(fragmentable_mpeg4_avc_h264_stereo_high_profile)
    "1.2.840.10008.1.2.4.107" -> Ok(hevc_h265_main_profile)
    "1.2.840.10008.1.2.4.108" -> Ok(hevc_h265_main_10_profile)
    "1.2.840.10008.1.2.4.110" -> Ok(jpeg_xl_lossless)
    "1.2.840.10008.1.2.4.111" -> Ok(jpeg_xl_jpeg_recompression)
    "1.2.840.10008.1.2.4.112" -> Ok(jpeg_xl)
    "1.2.840.10008.1.2.4.201" -> Ok(high_throughput_jpeg_2k_lossless_only)
    "1.2.840.10008.1.2.4.202" ->
      Ok(high_throughput_jpeg_2k_with_rpcl_options_lossless_only)
    "1.2.840.10008.1.2.4.203" -> Ok(high_throughput_jpeg_2k)
    "1.2.840.10008.1.2.4.204" -> Ok(jpip_high_throughput_jpeg_2k_referenced)
    "1.2.840.10008.1.2.4.205" ->
      Ok(jpip_high_throughput_jpeg_2k_referenced_deflate)
    "1.2.840.10008.1.2.5" -> Ok(rle_lossless)
    "1.2.840.10008.1.2.7.1" ->
      Ok(smpte_st_2110_20_uncompressed_progressive_active_video)
    "1.2.840.10008.1.2.7.2" ->
      Ok(smpte_st_2110_20_uncompressed_interlaced_active_video)
    "1.2.840.10008.1.2.7.3" -> Ok(smpte_st_2110_30_pcm_audio)
    "1.2.840.10008.1.2.8.1" -> Ok(deflated_image_frame_compression)

    _ -> Error(Nil)
  }
}
