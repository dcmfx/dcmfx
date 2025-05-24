use clap::{ValueEnum, builder::PossibleValue};
use dcmfx::core::{TransferSyntax, transfer_syntax};

/// Enum for specifying a transfer syntax name as a CLI argument, with detailed
/// help documentation.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransferSyntaxArg {
  PassThrough,
  ImplicitVrLittleEndian,
  ExplicitVrLittleEndian,
  ExplicitVrBigEndian,
  EncapsulatedUncompressedExplicitVrLittleEndian,
  DeflatedExplicitVrLittleEndian,
  DeflatedImageFrameCompression,
  RleLossless,
  JpegBaseline8Bit,
  Jpeg2kLosslessOnly,
  Jpeg2k,
}

impl TransferSyntaxArg {
  /// Converts to the underlying [`TransferSyntax`].
  ///
  pub fn as_transfer_syntax(&self) -> Option<&'static TransferSyntax> {
    match self {
      Self::PassThrough => None,
      Self::ImplicitVrLittleEndian => {
        Some(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN)
      }
      Self::ExplicitVrLittleEndian => {
        Some(&transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN)
      }
      Self::EncapsulatedUncompressedExplicitVrLittleEndian => Some(
        &transfer_syntax::ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN,
      ),
      Self::DeflatedExplicitVrLittleEndian => {
        Some(&transfer_syntax::DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN)
      }
      Self::ExplicitVrBigEndian => {
        Some(&transfer_syntax::EXPLICIT_VR_BIG_ENDIAN)
      }
      Self::RleLossless => Some(&transfer_syntax::RLE_LOSSLESS),
      Self::DeflatedImageFrameCompression => {
        Some(&transfer_syntax::DEFLATED_IMAGE_FRAME_COMPRESSION)
      }
      Self::JpegBaseline8Bit => Some(&transfer_syntax::JPEG_BASELINE_8BIT),
      Self::Jpeg2kLosslessOnly => Some(&transfer_syntax::JPEG_2K_LOSSLESS_ONLY),
      Self::Jpeg2k => Some(&transfer_syntax::JPEG_2K),
    }
  }
}

impl ValueEnum for TransferSyntaxArg {
  fn value_variants<'a>() -> &'a [Self] {
    &[
      Self::PassThrough,
      Self::ImplicitVrLittleEndian,
      Self::ExplicitVrLittleEndian,
      Self::ExplicitVrBigEndian,
      Self::EncapsulatedUncompressedExplicitVrLittleEndian,
      Self::DeflatedExplicitVrLittleEndian,
      Self::DeflatedImageFrameCompression,
      Self::RleLossless,
      Self::JpegBaseline8Bit,
      Self::Jpeg2kLosslessOnly,
      Self::Jpeg2k,
    ]
  }

  fn to_possible_value(&self) -> Option<PossibleValue> {
    Some(match self {
      Self::PassThrough => PossibleValue::new("pass-through").help(
        "\n\
          Keep the original transfer syntax when transcoding. This option can \
          be used to force a full decode/encode cycle that allows for \
          modifications such as changing the photometric interpretation, but \
          without having to explicitly specify an output transfer syntax.",
      ),

      Self::ImplicitVrLittleEndian => {
        PossibleValue::new("implicit-vr-little-endian").help(
          "\n\
          The default lowest common denominator DICOM transfer syntax. Uses \
          little endian byte order and implicit value representations (VR). \
          Prefer the 'Explicit VR Little Endian' transfer syntax over this one \
          whenever possible.\n\
          \n\
          Pixel data: Native uncompressed\n\
          Encapsulated: No\n\
          UID: 1.2.840.10008.1.2
          ",
        )
      }

      Self::ExplicitVrLittleEndian => {
        PossibleValue::new("explicit-vr-little-endian").help(
          "\n\
          Similar to Implicit VR Little Endian but with explicit value \
          representations that improve reliability and clarity of the DICOM \
          P10 data.\n\
          \n\
          Pixel data: Native uncompressed\n\
          Encapsulated: No\n\
          UID: 1.2.840.10008.1.2.1
          ",
        )
      }

      Self::ExplicitVrBigEndian => PossibleValue::new("explicit-vr-big-endian")
        .help(
          "\n\
          Similar to Explicit VR Little Endian but with big endian byte \
          ordering. This transfer syntax was retired in DICOM 2017c and is \
          only relevant for legacy compatibility.\n\
          \n\
          Pixel data: Native uncompressed\n\
          Encapsulated: No\n\
          UID: 1.2.840.10008.1.2.2",
        ),

      Self::EncapsulatedUncompressedExplicitVrLittleEndian => {
        PossibleValue::new(
          "encapsulated-uncompressed-explicit-vr-little-endian",
        )
        .help(
          "\n\
          Similar to Explicit VR Little Endian but stores the pixel data as \
          uncompressed encapsulated data.\n\
          \n\
          Pixel data: Native uncompressed\n\
          Encapsulated: Yes\n\
          UID: 1.2.840.10008.1.2.1.98",
        )
      }

      Self::DeflatedExplicitVrLittleEndian => {
        PossibleValue::new("deflated-explicit-vr-little-endian").help(
          "\n\
          Similar to Explicit VR Little Endian but with the whole data set \
          compressed using the DEFLATE algorithm. The compression level can be \
          set with the --zlib-compression-level argument.\n\
          \n\
          Pixel data: Native uncompressed\n\
          Encapsulated: No\n\
          UID: 1.2.840.10008.1.2.1.99
          ",
        )
      }

      Self::DeflatedImageFrameCompression => {
        PossibleValue::new("deflated-image-frame-compression").help(
          "\n\
          Similar to Explicit VR Little Endian but stores the pixel data as \
          encapsulated data and compresses each pixel data fragment using the \
          DEFLATE algorithm. The compression level can be set with the \
          --zlib-compression-level argument.\n\
          \n\
          Pixel data: Native deflated\n\
          Encapsulated: Yes\n\
          UID: 1.2.840.10008.1.2.1.98",
        )
      }

      Self::RleLossless => PossibleValue::new("rle-lossless").help(
        "\n\
        Encodes pixel data using DICOM's Run-Length Encoding for lossless \
        compression of monochrome and color pixel data.\n\
        \n\
        Pixel data: RLE Lossless compressed\n\
        Encapsulated: Yes\n\
        UID: 1.2.840.10008.1.2.5",
      ),

      Self::JpegBaseline8Bit => PossibleValue::new("jpeg-baseline-8bit").help(
        "\n\
          Lossy image compression using the widely supported JPEG Baseline \
          (Process 1) format. Limited to 8-bit pixel data. The quality level \
          to use for the JPEG encoding can be set with the --quality \
          argument.\n\
          \n\
          Pixel data: JPEG Baseline (8-bit) compressed\n\
          Encapsulated: Yes\n\
          UID: 1.2.840.10008.1.2.4.50",
      ),

      Self::Jpeg2kLosslessOnly => PossibleValue::new("jpeg-2k-lossless-only")
        .help(
          "\n\
          Lossless image compression using the JPEG 2000 image compression \
          format.\n\
          \n\
          Pixel data: JPEG 2000 Image Compression (Lossless Only)\n\
          Encapsulated: Yes\n\
          UID: 1.2.840.10008.1.2.4.90",
        ),

      Self::Jpeg2k => PossibleValue::new("jpeg-2k").help(
        "\n\
          Lossy image compression using the JPEG 2000 image compression \
          format. The quality level to use for the JPEG encoding can be set \
          with the --quality argument.\n\
          \n\
          Pixel data: JPEG 2000 Image Compression\n\
          Encapsulated: Yes\n\
          UID: 1.2.840.10008.1.2.4.91",
      ),
    })
  }
}

/// Returns whether a transfer syntax supports the `PALETTE_COLOR` photometric
/// interpretation.
///
pub fn supports_palette_color(transfer_syntax: &TransferSyntax) -> bool {
  transfer_syntax == &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN
    || transfer_syntax == &transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN
    || transfer_syntax
      == &transfer_syntax::ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    || transfer_syntax == &transfer_syntax::DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    || transfer_syntax == &transfer_syntax::EXPLICIT_VR_BIG_ENDIAN
    || transfer_syntax == &transfer_syntax::DEFLATED_IMAGE_FRAME_COMPRESSION
    || transfer_syntax == &transfer_syntax::RLE_LOSSLESS
    || transfer_syntax == &transfer_syntax::JPEG_2K_LOSSLESS_ONLY
}

/// Returns whether a transfer syntax supports the `YBR_FULL_422` photometric
/// interpretation.
///
pub fn supports_ybr_full_422(transfer_syntax: &TransferSyntax) -> bool {
  transfer_syntax == &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN
    || transfer_syntax == &transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN
    || transfer_syntax
      == &transfer_syntax::ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    || transfer_syntax == &transfer_syntax::DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    || transfer_syntax == &transfer_syntax::EXPLICIT_VR_BIG_ENDIAN
    || transfer_syntax == &transfer_syntax::DEFLATED_IMAGE_FRAME_COMPRESSION
    || transfer_syntax == &transfer_syntax::JPEG_BASELINE_8BIT
}

/// Returns whether a transfer syntax supports control of the planar
/// configuration.
///
pub fn supports_planar_configuration(transfer_syntax: &TransferSyntax) -> bool {
  transfer_syntax == &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN
    || transfer_syntax == &transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN
    || transfer_syntax
      == &transfer_syntax::ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    || transfer_syntax == &transfer_syntax::DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    || transfer_syntax == &transfer_syntax::EXPLICIT_VR_BIG_ENDIAN
    || transfer_syntax == &transfer_syntax::DEFLATED_IMAGE_FRAME_COMPRESSION
}
