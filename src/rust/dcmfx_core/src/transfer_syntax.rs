//! Defines all supported DICOM transfer syntaxes.

/// The value representation serialization mode of a transfer syntax. This is
/// either implicit or explicit.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VrSerialization {
  VrImplicit,
  VrExplicit,
}

/// The endianness of a transfer syntax, either little endian or big endian.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Endianness {
  LittleEndian,
  BigEndian,
}

impl Endianness {
  /// Returns whether the endianness enum specifies big endian byte ordering.
  ///
  pub fn is_big(&self) -> bool {
    *self == Self::BigEndian
  }
}

/// Describes a single DICOM transfer syntax, with its name, UID, how it
/// serializes value representations (implicit vs explicit), whether it is zlib
/// deflated, and whether it stores its pixel data as encapsulated.
///
#[derive(Debug, PartialEq)]
pub struct TransferSyntax {
  pub name: &'static str,
  pub uid: &'static str,
  pub vr_serialization: VrSerialization,
  pub endianness: Endianness,
  pub is_deflated: bool,
  pub is_encapsulated: bool,
}

/// The 'Implicit VR Little Endian' transfer syntax.
///
pub const IMPLICIT_VR_LITTLE_ENDIAN: TransferSyntax = TransferSyntax {
  name: "Implicit VR Little Endian",
  uid: "1.2.840.10008.1.2",
  vr_serialization: VrSerialization::VrImplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: false,
};

/// The 'Explicit VR Little Endian' transfer syntax.
///
pub const EXPLICIT_VR_LITTLE_ENDIAN: TransferSyntax = TransferSyntax {
  name: "Explicit VR Little Endian",
  uid: "1.2.840.10008.1.2.1",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: false,
};

/// The 'Encapsulated Uncompressed Explicit VR Little Endian' transfer syntax.
///
pub const ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN: TransferSyntax =
  TransferSyntax {
    name: "Encapsulated Uncompressed Explicit VR Little Endian",
    uid: "1.2.840.10008.1.2.1.98",
    vr_serialization: VrSerialization::VrExplicit,
    endianness: Endianness::LittleEndian,
    is_deflated: false,
    is_encapsulated: true,
  };

/// The 'Deflated Explicit VR Little Endian' transfer syntax.
///
pub const DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN: TransferSyntax = TransferSyntax {
  name: "Deflated Explicit VR Little Endian",
  uid: "1.2.840.10008.1.2.1.99",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: true,
  is_encapsulated: false,
};

/// The 'Explicit VR Big Endian' transfer syntax.
///
pub const EXPLICIT_VR_BIG_ENDIAN: TransferSyntax = TransferSyntax {
  name: "Explicit VR Big Endian",
  uid: "1.2.840.10008.1.2.2",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::BigEndian,
  is_deflated: false,
  is_encapsulated: false,
};

/// The 'JPEG Baseline (Process 1)' transfer syntax.
///
pub const JPEG_BASELINE_8BIT: TransferSyntax = TransferSyntax {
  name: "JPEG Baseline (Process 1)",
  uid: "1.2.840.10008.1.2.4.50",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPEG Extended (Process 2 & 4)' transfer syntax.
///
pub const JPEG_EXTENDED_12BIT: TransferSyntax = TransferSyntax {
  name: "JPEG Extended (Process 2 & 4)",
  uid: "1.2.840.10008.1.2.4.51",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPEG Lossless, Non-Hierarchical (Process 14)' transfer syntax.
///
pub const JPEG_LOSSLESS_NON_HIERARCHICAL: TransferSyntax = TransferSyntax {
  name: "JPEG Lossless, Non-Hierarchical (Process 14)",
  uid: "1.2.840.10008.1.2.4.57",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPEG Lossless, Non-Hierarchical, First-Order Prediction (Process 14
/// [Selection Value 1])' transfer syntax.
///
pub const JPEG_LOSSLESS_NON_HIERARCHICAL_SV1: TransferSyntax = TransferSyntax {
  name: "JPEG Lossless, Non-Hierarchical, First-Order Prediction (Process 14 [Selection Value 1])",
  uid: "1.2.840.10008.1.2.4.70",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPEG-LS Lossless Image Compression' transfer syntax.
///
pub const JPEG_LS_LOSSLESS: TransferSyntax = TransferSyntax {
  name: "JPEG-LS Lossless Image Compression",
  uid: "1.2.840.10008.1.2.4.80",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPEG-LS Lossy (Near-Lossless) Image Compression' transfer syntax.
///
pub const JPEG_LS_LOSSY_NEAR_LOSSLESS: TransferSyntax = TransferSyntax {
  name: "JPEG-LS Lossy (Near-Lossless) Image Compression",
  uid: "1.2.840.10008.1.2.4.81",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPEG 2000 Image Compression (Lossless Only)' transfer syntax.
///
pub const JPEG_2K_LOSSLESS_ONLY: TransferSyntax = TransferSyntax {
  name: "JPEG 2000 Image Compression (Lossless Only)",
  uid: "1.2.840.10008.1.2.4.90",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPEG 2000 Image Compression' transfer syntax.
///
pub const JPEG_2K: TransferSyntax = TransferSyntax {
  name: "JPEG 2000 Image Compression",
  uid: "1.2.840.10008.1.2.4.91",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPEG 2000 Part 2 Multi-component Image Compression (Lossless Only)'
/// transfer syntax.
///
pub const JPEG_2K_MULTI_COMPONENT_LOSSLESS_ONLY: TransferSyntax =
  TransferSyntax {
    name: "JPEG 2000 Part 2 Multi-component Image Compression (Lossless Only)",
    uid: "1.2.840.10008.1.2.4.92",
    vr_serialization: VrSerialization::VrExplicit,
    endianness: Endianness::LittleEndian,
    is_deflated: false,
    is_encapsulated: true,
  };

/// The 'JPEG 2000 Part 2 Multi-component Image Compression' transfer syntax.
///
pub const JPEG_2K_MULTI_COMPONENT: TransferSyntax = TransferSyntax {
  name: "JPEG 2000 Part 2 Multi-component Image Compression",
  uid: "1.2.840.10008.1.2.4.93",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPIP Referenced' transfer syntax.
///
pub const JPIP_REFERENCED: TransferSyntax = TransferSyntax {
  name: "JPIP Referenced",
  uid: "1.2.840.10008.1.2.4.94",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: false,
};

/// The 'JPIP Referenced Deflate' transfer syntax.
///
pub const JPIP_REFERENCED_DEFLATE: TransferSyntax = TransferSyntax {
  name: "JPIP Referenced Deflate",
  uid: "1.2.840.10008.1.2.4.95",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: true,
  is_encapsulated: false,
};

/// The 'MPEG2 Main Profile @ Main Level' transfer syntax.
///
pub const MPEG2_MAIN_PROFILE_MAIN_LEVEL: TransferSyntax = TransferSyntax {
  name: "MPEG2 Main Profile @ Main Level",
  uid: "1.2.840.10008.1.2.4.100",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'Fragmentable MPEG2 Main Profile @ Main Level' transfer syntax.
///
pub const FRAGMENTABLE_MPEG2_MAIN_PROFILE_MAIN_LEVEL: TransferSyntax =
  TransferSyntax {
    name: "Fragmentable MPEG2 Main Profile @ Main Level",
    uid: "1.2.840.10008.1.2.4.100.1",
    vr_serialization: VrSerialization::VrExplicit,
    endianness: Endianness::LittleEndian,
    is_deflated: false,
    is_encapsulated: true,
  };

/// The 'MPEG2 Main Profile @ High Level' transfer syntax.
///
pub const MPEG2_MAIN_PROFILE_HIGH_LEVEL: TransferSyntax = TransferSyntax {
  name: "MPEG2 Main Profile @ High Level",
  uid: "1.2.840.10008.1.2.4.101",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'Fragmentable MPEG2 Main Profile @ High Level' transfer syntax.
///
pub const FRAGMENTABLE_MPEG2_MAIN_PROFILE_HIGH_LEVEL: TransferSyntax =
  TransferSyntax {
    name: "Fragmentable MPEG2 Main Profile @ High Level",
    uid: "1.2.840.10008.1.2.4.101.1",
    vr_serialization: VrSerialization::VrExplicit,
    endianness: Endianness::LittleEndian,
    is_deflated: false,
    is_encapsulated: true,
  };

/// The 'MPEG-4 AVC/H.264 High Profile / Level 4.1' transfer syntax.
///
pub const MPEG4_AVC_H264_HIGH_PROFILE: TransferSyntax = TransferSyntax {
  name: "MPEG-4 AVC/H.264 High Profile / Level 4.1",
  uid: "1.2.840.10008.1.2.4.102",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.1' transfer
/// syntax.
///
pub const FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE: TransferSyntax =
  TransferSyntax {
    name: "Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.1",
    uid: "1.2.840.10008.1.2.4.102.1",
    vr_serialization: VrSerialization::VrExplicit,
    endianness: Endianness::LittleEndian,
    is_deflated: false,
    is_encapsulated: true,
  };

/// The 'MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1' transfer
/// syntax.
///
pub const MPEG4_AVC_H264_BD_COMPATIBLE_HIGH_PROFILE: TransferSyntax =
  TransferSyntax {
    name: "MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1",
    uid: "1.2.840.10008.1.2.4.103",
    vr_serialization: VrSerialization::VrExplicit,
    endianness: Endianness::LittleEndian,
    is_deflated: false,
    is_encapsulated: true,
  };

/// The 'Fragmentable MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1'
/// transfer syntax.
///
pub const FRAGMENTABLE_MPEG4_AVC_H264_BD_COMPATIBLE_HIGH_PROFILE:
  TransferSyntax = TransferSyntax {
  name: "Fragmentable MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1",
  uid: "1.2.840.10008.1.2.4.103.1",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video' transfer
/// syntax.
///
pub const MPEG4_AVC_H264_HIGH_PROFILE_FOR_2D_VIDEO: TransferSyntax =
  TransferSyntax {
    name: "MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video",
    uid: "1.2.840.10008.1.2.4.104",
    vr_serialization: VrSerialization::VrExplicit,
    endianness: Endianness::LittleEndian,
    is_deflated: false,
    is_encapsulated: true,
  };

/// The 'Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video'
/// transfer syntax.
///
pub const FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE_FOR_2D_VIDEO:
  TransferSyntax = TransferSyntax {
  name: "Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video",
  uid: "1.2.840.10008.1.2.4.104.1",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video' transfer
/// syntax.
///
pub const MPEG4_AVC_H264_HIGH_PROFILE_FOR_3D_VIDEO: TransferSyntax =
  TransferSyntax {
    name: "MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video",
    uid: "1.2.840.10008.1.2.4.105",
    vr_serialization: VrSerialization::VrExplicit,
    endianness: Endianness::LittleEndian,
    is_deflated: false,
    is_encapsulated: true,
  };

/// The 'Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video'
/// transfer syntax.
///
pub const FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE_FOR_3D_VIDEO:
  TransferSyntax = TransferSyntax {
  name: "Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video",
  uid: "1.2.840.10008.1.2.4.105.1",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2' transfer syntax.
///
pub const MPEG4_AVC_H264_STEREO_HIGH_PROFILE: TransferSyntax = TransferSyntax {
  name: "MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2",
  uid: "1.2.840.10008.1.2.4.106",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'Fragmentable MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2' transfer
/// syntax.
///
pub const FRAGMENTABLE_MPEG4_AVC_H264_STEREO_HIGH_PROFILE: TransferSyntax =
  TransferSyntax {
    name: "Fragmentable MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2",
    uid: "1.2.840.10008.1.2.4.106.1",
    vr_serialization: VrSerialization::VrExplicit,
    endianness: Endianness::LittleEndian,
    is_deflated: false,
    is_encapsulated: true,
  };

/// The 'HEVC/H.265 Main Profile / Level 5.1' transfer syntax.
///
pub const HEVC_H265_MAIN_PROFILE: TransferSyntax = TransferSyntax {
  name: "HEVC/H.265 Main Profile / Level 5.1",
  uid: "1.2.840.10008.1.2.4.107",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'HEVC/H.265 Main 10 Profile / Level 5.1' transfer syntax.
///
pub const HEVC_H265_MAIN_10_PROFILE: TransferSyntax = TransferSyntax {
  name: "HEVC/H.265 Main 10 Profile / Level 5.1",
  uid: "1.2.840.10008.1.2.4.108",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPEG XL Lossless' transfer syntax.
///
pub const JPEG_XL_LOSSLESS: TransferSyntax = TransferSyntax {
  name: "JPEG XL Lossless",
  uid: "1.2.840.10008.1.2.4.110",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPEG XL JPEG Recompression' transfer syntax.
///
pub const JPEG_XL_JPEG_RECOMPRESSION: TransferSyntax = TransferSyntax {
  name: "JPEG XL JPEG Recompression",
  uid: "1.2.840.10008.1.2.4.111",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPEG XL' transfer syntax.
///
pub const JPEG_XL: TransferSyntax = TransferSyntax {
  name: "JPEG XL",
  uid: "1.2.840.10008.1.2.4.112",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'High-Throughput JPEG 2000 (Lossless Only)' transfer syntax.
///
pub const HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY: TransferSyntax =
  TransferSyntax {
    name: "High-Throughput JPEG 2000 (Lossless Only)",
    uid: "1.2.840.10008.1.2.4.201",
    vr_serialization: VrSerialization::VrExplicit,
    endianness: Endianness::LittleEndian,
    is_deflated: false,
    is_encapsulated: true,
  };

/// The 'High-Throughput JPEG 2000 with RPCL Options (Lossless Only)' transfer
/// syntax.
///
pub const HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY:
  TransferSyntax = TransferSyntax {
  name: "High-Throughput JPEG 2000 with RPCL Options (Lossless Only)",
  uid: "1.2.840.10008.1.2.4.202",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'High-Throughput JPEG 2000' transfer syntax.
///
pub const HIGH_THROUGHPUT_JPEG_2K: TransferSyntax = TransferSyntax {
  name: "High-Throughput JPEG 2000",
  uid: "1.2.840.10008.1.2.4.203",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'JPIP HTJ2K Referenced' transfer syntax.
///
pub const JPIP_HIGH_THROUGHPUT_JPEG_2K_REFERENCED: TransferSyntax =
  TransferSyntax {
    name: "JPIP HTJ2K Referenced",
    uid: "1.2.840.10008.1.2.4.204",
    vr_serialization: VrSerialization::VrExplicit,
    endianness: Endianness::LittleEndian,
    is_deflated: false,
    is_encapsulated: false,
  };

/// The 'JPIP HTJ2K Referenced Deflate' transfer syntax.
///
pub const JPIP_HIGH_THROUGHPUT_JPEG_2K_REFERENCED_DEFLATE: TransferSyntax =
  TransferSyntax {
    name: "JPIP HTJ2K Referenced Deflate",
    uid: "1.2.840.10008.1.2.4.205",
    vr_serialization: VrSerialization::VrExplicit,
    endianness: Endianness::LittleEndian,
    is_deflated: true,
    is_encapsulated: false,
  };

/// The 'RLE Lossless' transfer syntax.
///
pub const RLE_LOSSLESS: TransferSyntax = TransferSyntax {
  name: "RLE Lossless",
  uid: "1.2.840.10008.1.2.5",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'SMPTE ST 2110-20 Uncompressed Progressive Active Video' transfer
/// syntax.
///
pub const SMPTE_ST_2110_20_UNCOMPRESSED_PROGRESSIVE_ACTIVE_VIDEO:
  TransferSyntax = TransferSyntax {
  name: "SMPTE ST 2110-20 Uncompressed Progressive Active Video",
  uid: "1.2.840.10008.1.2.7.1",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'SMPTE ST 2110-20 Uncompressed Interlaced Active Video' transfer syntax.
///
pub const SMPTE_ST_2110_20_UNCOMPRESSED_INTERLACED_ACTIVE_VIDEO:
  TransferSyntax = TransferSyntax {
  name: "SMPTE ST 2110-20 Uncompressed Interlaced Active Video",
  uid: "1.2.840.10008.1.2.7.2",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// The 'SMPTE ST 2110-30 PCM Audio' transfer syntax.
///
pub const SMPTE_ST_2110_30_PCM_AUDIO: TransferSyntax = TransferSyntax {
  name: "SMPTE ST 2110-30 PCM Audio",
  uid: "1.2.840.10008.1.2.7.3",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: false,
};

/// The 'Deflated Image Frame Compression' transfer syntax.
///
pub const DEFLATED_IMAGE_FRAME_COMPRESSION: TransferSyntax = TransferSyntax {
  name: "Deflated Image Frame Compression",
  uid: "1.2.840.10008.1.2.8.1",
  vr_serialization: VrSerialization::VrExplicit,
  endianness: Endianness::LittleEndian,
  is_deflated: false,
  is_encapsulated: true,
};

/// A list of all supported transfer syntaxes.
///
pub const ALL: [TransferSyntax; 46] = [
  IMPLICIT_VR_LITTLE_ENDIAN,
  EXPLICIT_VR_LITTLE_ENDIAN,
  ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN,
  DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN,
  EXPLICIT_VR_BIG_ENDIAN,
  JPEG_BASELINE_8BIT,
  JPEG_EXTENDED_12BIT,
  JPEG_LOSSLESS_NON_HIERARCHICAL,
  JPEG_LOSSLESS_NON_HIERARCHICAL_SV1,
  JPEG_LS_LOSSLESS,
  JPEG_LS_LOSSY_NEAR_LOSSLESS,
  JPEG_2K_LOSSLESS_ONLY,
  JPEG_2K,
  JPEG_2K_MULTI_COMPONENT_LOSSLESS_ONLY,
  JPEG_2K_MULTI_COMPONENT,
  JPIP_REFERENCED,
  JPIP_REFERENCED_DEFLATE,
  MPEG2_MAIN_PROFILE_MAIN_LEVEL,
  FRAGMENTABLE_MPEG2_MAIN_PROFILE_MAIN_LEVEL,
  MPEG2_MAIN_PROFILE_HIGH_LEVEL,
  FRAGMENTABLE_MPEG2_MAIN_PROFILE_HIGH_LEVEL,
  MPEG4_AVC_H264_HIGH_PROFILE,
  FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE,
  MPEG4_AVC_H264_BD_COMPATIBLE_HIGH_PROFILE,
  FRAGMENTABLE_MPEG4_AVC_H264_BD_COMPATIBLE_HIGH_PROFILE,
  MPEG4_AVC_H264_HIGH_PROFILE_FOR_2D_VIDEO,
  FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE_FOR_2D_VIDEO,
  MPEG4_AVC_H264_HIGH_PROFILE_FOR_3D_VIDEO,
  FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE_FOR_3D_VIDEO,
  MPEG4_AVC_H264_STEREO_HIGH_PROFILE,
  FRAGMENTABLE_MPEG4_AVC_H264_STEREO_HIGH_PROFILE,
  HEVC_H265_MAIN_PROFILE,
  HEVC_H265_MAIN_10_PROFILE,
  JPEG_XL_LOSSLESS,
  JPEG_XL_JPEG_RECOMPRESSION,
  JPEG_XL,
  HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY,
  HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY,
  HIGH_THROUGHPUT_JPEG_2K,
  JPIP_HIGH_THROUGHPUT_JPEG_2K_REFERENCED,
  JPIP_HIGH_THROUGHPUT_JPEG_2K_REFERENCED_DEFLATE,
  RLE_LOSSLESS,
  SMPTE_ST_2110_20_UNCOMPRESSED_PROGRESSIVE_ACTIVE_VIDEO,
  SMPTE_ST_2110_20_UNCOMPRESSED_INTERLACED_ACTIVE_VIDEO,
  SMPTE_ST_2110_30_PCM_AUDIO,
  DEFLATED_IMAGE_FRAME_COMPRESSION,
];

impl TransferSyntax {
  /// Returns the transfer syntax with the given UID. If the UID isn't
  /// recognized then an error is returned.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn from_uid(uid: &str) -> Result<&'static Self, ()> {
    match uid {
      "1.2.840.10008.1.2" => Ok(&IMPLICIT_VR_LITTLE_ENDIAN),
      "1.2.840.10008.1.2.1" => Ok(&EXPLICIT_VR_LITTLE_ENDIAN),
      "1.2.840.10008.1.2.1.98" => {
        Ok(&ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN)
      }
      "1.2.840.10008.1.2.1.99" => Ok(&DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN),
      "1.2.840.10008.1.2.2" => Ok(&EXPLICIT_VR_BIG_ENDIAN),
      "1.2.840.10008.1.2.4.50" => Ok(&JPEG_BASELINE_8BIT),
      "1.2.840.10008.1.2.4.51" => Ok(&JPEG_EXTENDED_12BIT),
      "1.2.840.10008.1.2.4.57" => Ok(&JPEG_LOSSLESS_NON_HIERARCHICAL),
      "1.2.840.10008.1.2.4.70" => Ok(&JPEG_LOSSLESS_NON_HIERARCHICAL_SV1),
      "1.2.840.10008.1.2.4.80" => Ok(&JPEG_LS_LOSSLESS),
      "1.2.840.10008.1.2.4.81" => Ok(&JPEG_LS_LOSSY_NEAR_LOSSLESS),
      "1.2.840.10008.1.2.4.90" => Ok(&JPEG_2K_LOSSLESS_ONLY),
      "1.2.840.10008.1.2.4.91" => Ok(&JPEG_2K),
      "1.2.840.10008.1.2.4.92" => Ok(&JPEG_2K_MULTI_COMPONENT_LOSSLESS_ONLY),
      "1.2.840.10008.1.2.4.93" => Ok(&JPEG_2K_MULTI_COMPONENT),
      "1.2.840.10008.1.2.4.94" => Ok(&JPIP_REFERENCED),
      "1.2.840.10008.1.2.4.95" => Ok(&JPIP_REFERENCED_DEFLATE),
      "1.2.840.10008.1.2.4.100" => Ok(&MPEG2_MAIN_PROFILE_MAIN_LEVEL),
      "1.2.840.10008.1.2.4.100.1" => {
        Ok(&FRAGMENTABLE_MPEG2_MAIN_PROFILE_MAIN_LEVEL)
      }
      "1.2.840.10008.1.2.4.101" => Ok(&MPEG2_MAIN_PROFILE_HIGH_LEVEL),
      "1.2.840.10008.1.2.4.101.1" => {
        Ok(&FRAGMENTABLE_MPEG2_MAIN_PROFILE_HIGH_LEVEL)
      }
      "1.2.840.10008.1.2.4.102" => Ok(&MPEG4_AVC_H264_HIGH_PROFILE),
      "1.2.840.10008.1.2.4.102.1" => {
        Ok(&FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE)
      }
      "1.2.840.10008.1.2.4.103" => {
        Ok(&MPEG4_AVC_H264_BD_COMPATIBLE_HIGH_PROFILE)
      }
      "1.2.840.10008.1.2.4.103.1" => {
        Ok(&FRAGMENTABLE_MPEG4_AVC_H264_BD_COMPATIBLE_HIGH_PROFILE)
      }
      "1.2.840.10008.1.2.4.104" => {
        Ok(&MPEG4_AVC_H264_HIGH_PROFILE_FOR_2D_VIDEO)
      }
      "1.2.840.10008.1.2.4.104.1" => {
        Ok(&FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE_FOR_2D_VIDEO)
      }
      "1.2.840.10008.1.2.4.105" => {
        Ok(&MPEG4_AVC_H264_HIGH_PROFILE_FOR_3D_VIDEO)
      }
      "1.2.840.10008.1.2.4.105.1" => {
        Ok(&FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE_FOR_3D_VIDEO)
      }
      "1.2.840.10008.1.2.4.106" => Ok(&MPEG4_AVC_H264_STEREO_HIGH_PROFILE),
      "1.2.840.10008.1.2.4.106.1" => {
        Ok(&FRAGMENTABLE_MPEG4_AVC_H264_STEREO_HIGH_PROFILE)
      }
      "1.2.840.10008.1.2.4.107" => Ok(&HEVC_H265_MAIN_PROFILE),
      "1.2.840.10008.1.2.4.108" => Ok(&HEVC_H265_MAIN_10_PROFILE),
      "1.2.840.10008.1.2.4.110" => Ok(&JPEG_XL_LOSSLESS),
      "1.2.840.10008.1.2.4.111" => Ok(&JPEG_XL_JPEG_RECOMPRESSION),
      "1.2.840.10008.1.2.4.112" => Ok(&JPEG_XL),
      "1.2.840.10008.1.2.4.201" => Ok(&HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY),
      "1.2.840.10008.1.2.4.202" => {
        Ok(&HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY)
      }
      "1.2.840.10008.1.2.4.203" => Ok(&HIGH_THROUGHPUT_JPEG_2K),
      "1.2.840.10008.1.2.4.204" => Ok(&JPIP_HIGH_THROUGHPUT_JPEG_2K_REFERENCED),
      "1.2.840.10008.1.2.4.205" => {
        Ok(&JPIP_HIGH_THROUGHPUT_JPEG_2K_REFERENCED_DEFLATE)
      }
      "1.2.840.10008.1.2.5" => Ok(&RLE_LOSSLESS),
      "1.2.840.10008.1.2.7.1" => {
        Ok(&SMPTE_ST_2110_20_UNCOMPRESSED_PROGRESSIVE_ACTIVE_VIDEO)
      }
      "1.2.840.10008.1.2.7.2" => {
        Ok(&SMPTE_ST_2110_20_UNCOMPRESSED_INTERLACED_ACTIVE_VIDEO)
      }
      "1.2.840.10008.1.2.7.3" => Ok(&SMPTE_ST_2110_30_PCM_AUDIO),
      "1.2.840.10008.1.2.8.1" => Ok(&DEFLATED_IMAGE_FRAME_COMPRESSION),

      _ => Err(()),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const TRANSFER_SYNTAX_UIDS: [&str; 46] = [
    "1.2.840.10008.1.2",
    "1.2.840.10008.1.2.1",
    "1.2.840.10008.1.2.1.98",
    "1.2.840.10008.1.2.1.99",
    "1.2.840.10008.1.2.2",
    "1.2.840.10008.1.2.4.50",
    "1.2.840.10008.1.2.4.51",
    "1.2.840.10008.1.2.4.57",
    "1.2.840.10008.1.2.4.70",
    "1.2.840.10008.1.2.4.80",
    "1.2.840.10008.1.2.4.81",
    "1.2.840.10008.1.2.4.90",
    "1.2.840.10008.1.2.4.91",
    "1.2.840.10008.1.2.4.92",
    "1.2.840.10008.1.2.4.93",
    "1.2.840.10008.1.2.4.94",
    "1.2.840.10008.1.2.4.95",
    "1.2.840.10008.1.2.4.100",
    "1.2.840.10008.1.2.4.100.1",
    "1.2.840.10008.1.2.4.101",
    "1.2.840.10008.1.2.4.101.1",
    "1.2.840.10008.1.2.4.102",
    "1.2.840.10008.1.2.4.102.1",
    "1.2.840.10008.1.2.4.103",
    "1.2.840.10008.1.2.4.103.1",
    "1.2.840.10008.1.2.4.104",
    "1.2.840.10008.1.2.4.104.1",
    "1.2.840.10008.1.2.4.105",
    "1.2.840.10008.1.2.4.105.1",
    "1.2.840.10008.1.2.4.106",
    "1.2.840.10008.1.2.4.106.1",
    "1.2.840.10008.1.2.4.107",
    "1.2.840.10008.1.2.4.108",
    "1.2.840.10008.1.2.4.110",
    "1.2.840.10008.1.2.4.111",
    "1.2.840.10008.1.2.4.112",
    "1.2.840.10008.1.2.4.201",
    "1.2.840.10008.1.2.4.202",
    "1.2.840.10008.1.2.4.203",
    "1.2.840.10008.1.2.4.204",
    "1.2.840.10008.1.2.4.205",
    "1.2.840.10008.1.2.5",
    "1.2.840.10008.1.2.7.1",
    "1.2.840.10008.1.2.7.2",
    "1.2.840.10008.1.2.7.3",
    "1.2.840.10008.1.2.8.1",
  ];

  #[test]
  pub fn all_test() {
    assert_eq!(ALL.map(|ts| ts.uid), TRANSFER_SYNTAX_UIDS);
  }

  #[test]
  pub fn from_uid_test() {
    for uid in TRANSFER_SYNTAX_UIDS {
      assert!(TransferSyntax::from_uid(uid).is_ok());
    }

    assert!(TransferSyntax::from_uid("1.2.3.4").is_err());
  }
}
