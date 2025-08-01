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
  JpegExtended12Bit,
  JpegLsLossless,
  JpegLsLossyNearLossless,
  Jpeg2000LosslessOnly,
  Jpeg2000,
  HighThroughputJpeg2000LosslessOnly,
  HighThroughputJpeg2000,
  JpegXlLossless,
  JpegXlJpegRecompression,
  JpegXl,
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
      Self::JpegExtended12Bit => Some(&transfer_syntax::JPEG_EXTENDED_12BIT),
      Self::JpegLsLossless => Some(&transfer_syntax::JPEG_LS_LOSSLESS),
      Self::JpegLsLossyNearLossless => {
        Some(&transfer_syntax::JPEG_LS_LOSSY_NEAR_LOSSLESS)
      }
      Self::Jpeg2000LosslessOnly => {
        Some(&transfer_syntax::JPEG_2000_LOSSLESS_ONLY)
      }
      Self::Jpeg2000 => Some(&transfer_syntax::JPEG_2000),
      Self::HighThroughputJpeg2000LosslessOnly => {
        Some(&transfer_syntax::HIGH_THROUGHPUT_JPEG_2000_LOSSLESS_ONLY)
      }
      Self::HighThroughputJpeg2000 => {
        Some(&transfer_syntax::HIGH_THROUGHPUT_JPEG_2000)
      }
      Self::JpegXlLossless => Some(&transfer_syntax::JPEG_XL_LOSSLESS),
      Self::JpegXlJpegRecompression => {
        Some(&transfer_syntax::JPEG_XL_JPEG_RECOMPRESSION)
      }
      Self::JpegXl => Some(&transfer_syntax::JPEG_XL),
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
      Self::JpegExtended12Bit,
      Self::JpegLsLossless,
      Self::JpegLsLossyNearLossless,
      Self::Jpeg2000LosslessOnly,
      Self::Jpeg2000,
      Self::HighThroughputJpeg2000LosslessOnly,
      Self::HighThroughputJpeg2000,
      Self::JpegXlLossless,
      Self::JpegXlJpegRecompression,
      Self::JpegXl,
    ]
  }

  fn to_possible_value(&self) -> Option<PossibleValue> {
    Some(match self {
      Self::PassThrough => PossibleValue::new("pass-through").help(
        "\n\
        Keep the original transfer syntax when transcoding. This option can be \
        used to perform a full decode/encode cycle that allows for \
        modifications such as cropping, recompressing at a different quality \
        level, or changing the photometric interpretation, without altering \
        the current transfer syntax.",
      ),

      Self::ImplicitVrLittleEndian => {
        PossibleValue::new("implicit-vr-little-endian").help(
          "\n\
          The default, lowest common denominator DICOM transfer syntax. Uses \
          little endian byte order and implicit value representations (VR). \
          Prefer the 'Explicit VR Little Endian' transfer syntax over this one \
          whenever possible.\n\
          \n\
          Encapsulated: No\n\
          UID: 1.2.840.10008.1.2
          ",
        )
      }

      Self::ExplicitVrLittleEndian => {
        PossibleValue::new("explicit-vr-little-endian").help(
          "\n\
          Similar to 'Implicit VR Little Endian' but with explicit value \
          representations stored in the DICOM P10 data.\n\
          \n\
          Encapsulated: No\n\
          UID: 1.2.840.10008.1.2.1
          ",
        )
      }

      Self::ExplicitVrBigEndian => PossibleValue::new("explicit-vr-big-endian")
        .help(
          "\n\
          Similar to 'Explicit VR Little Endian' but with big endian byte \
          ordering. This transfer syntax was retired in 2006 and is only \
          relevant for legacy compatibility.\n\
          \n\
          Encapsulated: No\n\
          UID: 1.2.840.10008.1.2.2",
        ),

      Self::EncapsulatedUncompressedExplicitVrLittleEndian => {
        PossibleValue::new(
          "encapsulated-uncompressed-explicit-vr-little-endian",
        )
        .help(
          "\n\
          Similar to 'Explicit VR Little Endian' but stores the pixel data \
          encapsulated and uncompressed.\n\
          \n\
          Encapsulated: Yes\n\
          UID: 1.2.840.10008.1.2.1.98",
        )
      }

      Self::DeflatedExplicitVrLittleEndian => {
        PossibleValue::new("deflated-explicit-vr-little-endian").help(
          "\n\
          Similar to 'Explicit VR Little Endian' but with the whole data set \
          compressed using the DEFLATE algorithm. The compression level can be \
          set with the --zlib-compression-level argument.\n\
          \n\
          Encapsulated: No\n\
          UID: 1.2.840.10008.1.2.1.99
          ",
        )
      }

      Self::DeflatedImageFrameCompression => {
        PossibleValue::new("deflated-image-frame-compression").help(
          "\n\
          Similar to 'Explicit VR Little Endian' but encapsulates the pixel \
          data and compresses each frame using the DEFLATE algorithm. The \
          compression level can be set with the --zlib-compression-level \
          argument.\n\
          \n\
          Encapsulated: Yes\n\
          UID: 1.2.840.10008.1.2.1.98",
        )
      }

      Self::RleLossless => PossibleValue::new("rle-lossless").help(
        "\n\
        Encodes pixel data using DICOM's Run-Length Encoding for lossless \
        compression of monochrome and color pixel data.\n\
        \n\
        Encapsulated: Yes\n\
        UID: 1.2.840.10008.1.2.5",
      ),

      Self::JpegBaseline8Bit => PossibleValue::new("jpeg-baseline-8bit").help(
        "\n\
        Lossy image compression using the widely supported JPEG Baseline \
        (Process 1) format. Limited to 8-bit pixel data. The quality level to \
        use for the encoding can be set with the --quality argument.\n\
        \n\
        Encapsulated: Yes\n\
        UID: 1.2.840.10008.1.2.4.50",
      ),

      Self::JpegExtended12Bit => PossibleValue::new("jpeg-extended-12bit")
        .help(
          "\n\
          Lossy image compression using the JPEG Extended (Process 2 & 4) \
          format. Limited to 12-bit pixel data. The quality level to use for \
          the encoding can be set with the --quality argument.\n\
          \n\
          Encapsulated: Yes\n\
          UID: 1.2.840.10008.1.2.4.51",
        ),

      Self::JpegLsLossless => PossibleValue::new("jpeg-ls-lossless").help(
        "\n\
        Lossless image compression using the JPEG-LS format.\n\
        \n\
        Encapsulated: Yes\n\
        UID: 1.2.840.10008.1.2.4.80",
      ),

      Self::JpegLsLossyNearLossless => {
        PossibleValue::new("jpeg-ls-lossy-near-lossless").help(
          "\n\
          Lossy near-lossless image compression using the JPEG-LS format.\n\
          \n\
          Encapsulated: Yes\n\
          UID: 1.2.840.10008.1.2.4.81",
        )
      }

      Self::Jpeg2000LosslessOnly => {
        PossibleValue::new("jpeg-2000-lossless-only").help(
          "\n\
          Lossless image compression using the JPEG 2000 format.\n\
          \n\
          Encapsulated: Yes\n\
          UID: 1.2.840.10008.1.2.4.90",
        )
      }

      Self::Jpeg2000 => PossibleValue::new("jpeg-2000").help(
        "\n\
        Lossy image compression using the JPEG 2000 format. The quality level \
        to use for the encoding can be set with the --quality argument.\n\
        \n\
        Encapsulated: Yes\n\
        UID: 1.2.840.10008.1.2.4.91",
      ),

      Self::HighThroughputJpeg2000LosslessOnly => {
        PossibleValue::new("high-throughput-jpeg-2000-lossless-only").help(
          "\n\
          Lossless image compression using the High-Throughput JPEG 2000 \
          format.\n\
          \n\
          Encapsulated: Yes\n\
          UID: 1.2.840.10008.1.2.4.201",
        )
      }

      &Self::HighThroughputJpeg2000 => {
        PossibleValue::new("high-throughput-jpeg-2000").help(
          "\n\
        Lossy image compression using the High-Throughput JPEG 2000 format. \
        The quality level to use for the encoding can be set with the \
        --quality argument.\n\
        \n\
        Encapsulated: Yes\n\
        UID: 1.2.840.10008.1.2.4.202",
        )
      }

      Self::JpegXlLossless => PossibleValue::new("jpeg-xl-lossless").help(
        "\n\
        Lossless image compression using the JPEG XL format. The compression \
        effort to use can be set with the --effort argument.\n\
        \n\
        Encapsulated: Yes\n\
        UID: 1.2.840.10008.1.2.4.110",
      ),

      Self::JpegXlJpegRecompression => {
        PossibleValue::new("jpeg-xl-jpeg-recompression").help(
          "\n\
          Lossy image compression using the JPEG XL format where the input \
          data was originally compressed as 'JPEG Baseline 8-bit'. Storing \
          such data in JPEG XL can reduce its size by 15-35% with no change \
          to the image.\n\
          \n\
          The only transfer syntax that can be transcoded into 'JPEG XL JPEG \
          Recompression' is 'JPEG Baseline 8-bit'. No aspect of the pixel data \
          can be altered when doing this transcode, e.g. the photometric \
          interpretation can't be changed, nor can the --quality argument be \
          applied.\n\
          \n\
          Encapsulated: Yes\n\
          UID: 1.2.840.10008.1.2.4.111",
        )
      }

      Self::JpegXl => PossibleValue::new("jpeg-xl").help(
        "\n\
        Lossy image compression using the JPEG XL format. The quality level to \
        use for the encoding can be set with the --quality argument. The \
        compression effort to use can be set with the --effort argument.\n\
        \n\
        Encapsulated: Yes\n\
        UID: 1.2.840.10008.1.2.4.112",
      ),
    })
  }
}
