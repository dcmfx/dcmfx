#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use dcmfx_core::{
  DataElementTag, DataError, DataSet, DataSetPath, IodModule, TransferSyntax,
  ValueRepresentation, dictionary, transfer_syntax,
};

use crate::{
  ColorImage, GrayscalePipeline, PixelDataFrame, SingleChannelImage,
  StandardColorPalette, decode, iods::ImagePixelModule,
};

/// Defines a pixel data renderer that can take a [`PixelDataFrame`] and render
/// it into a [`SingleChannelImage`], [`ColorImage`], or [`image::RgbImage`].
///
#[derive(Clone, Debug, PartialEq)]
pub struct PixelDataRenderer {
  pub transfer_syntax: &'static TransferSyntax,
  pub image_pixel_module: ImagePixelModule,
  pub grayscale_pipeline: GrayscalePipeline,
}

impl IodModule for PixelDataRenderer {
  fn is_iod_module_data_element(
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: Option<u32>,
    path: &DataSetPath,
  ) -> bool {
    ImagePixelModule::is_iod_module_data_element(tag, vr, length, path)
      || GrayscalePipeline::is_iod_module_data_element(tag, vr, length, path)
  }

  fn iod_module_highest_tag() -> DataElementTag {
    ImagePixelModule::iod_module_highest_tag()
      .max(GrayscalePipeline::iod_module_highest_tag())
  }

  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let transfer_syntax = if data_set.has(dictionary::TRANSFER_SYNTAX_UID.tag) {
      data_set.get_transfer_syntax()?
    } else {
      &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN
    };

    let image_pixel_module = ImagePixelModule::from_data_set(data_set)?;
    let grayscale_pipeline = GrayscalePipeline::from_data_set(
      data_set,
      image_pixel_module.stored_value_range(),
    )?;

    Ok(PixelDataRenderer {
      transfer_syntax,
      image_pixel_module,
      grayscale_pipeline,
    })
  }
}

impl PixelDataRenderer {
  /// Renders a frame of pixel data to an RGB 8-bit image. The Modality LUT and
  /// VOI LUT are applied to single channel images, and resulting grayscale
  /// values are then expanded to RGB.
  ///
  /// Grayscale frames can optionally be visualized using a color palette. The
  /// well-known color palettes defined in PS3.6 B.1 are provided in
  /// [`crate::standard_color_palettes`].
  ///
  pub fn render_frame(
    &self,
    frame: &mut PixelDataFrame,
    color_palette: Option<&StandardColorPalette>,
  ) -> Result<image::RgbImage, DataError> {
    if self.image_pixel_module.is_grayscale() {
      let image = self.decode_single_channel_frame(frame)?;
      Ok(self.render_single_channel_image(&image, color_palette))
    } else {
      let image = self.decode_color_frame(frame)?;
      Ok(image.into_rgb_u8_image())
    }
  }

  /// Renders a [`SingleChannelImage`] to an RGB 8-bit image. The grayscale
  /// pipeline is applied, and resulting grayscale values are then expanded
  /// to RGB.
  ///
  /// The result can optionally be visualized using a color palette. The
  /// well-known color palettes defined in PS3.6 B.1 are provided in
  /// [`crate::standard_color_palettes`].
  ///
  pub fn render_single_channel_image(
    &self,
    image: &SingleChannelImage,
    color_palette: Option<&StandardColorPalette>,
  ) -> image::RgbImage {
    let mut pixels = Vec::with_capacity(image.pixel_count() * 3);

    let gray_image = image.to_gray_u8_image(&self.grayscale_pipeline);

    if let Some(color_palette) = color_palette {
      for pixel in gray_image.pixels() {
        pixels.extend_from_slice(&color_palette.lookup(pixel.0[0]));
      }
    } else {
      for pixel in gray_image.pixels() {
        pixels.push(pixel.0[0]);
        pixels.push(pixel.0[0]);
        pixels.push(pixel.0[0]);
      }
    }

    image::RgbImage::from_raw(gray_image.width(), gray_image.height(), pixels)
      .unwrap()
  }

  /// Decodes a frame of single channel pixel data into a
  /// [`SingleChannelImage`]. The returned image needs to have the grayscale
  /// pipeline applied in order to reach final grayscale display values.
  ///
  pub fn decode_single_channel_frame(
    &self,
    frame: &mut PixelDataFrame,
  ) -> Result<SingleChannelImage, DataError> {
    let frame_bit_offset = frame.bit_offset();
    let data = frame.combine_fragments();

    use transfer_syntax::*;

    let mut image = match self.transfer_syntax {
      &IMPLICIT_VR_LITTLE_ENDIAN
      | &EXPLICIT_VR_LITTLE_ENDIAN
      | &ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
      | &DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
      | &EXPLICIT_VR_BIG_ENDIAN => decode::native::decode_single_channel(
        &self.image_pixel_module,
        data,
        frame_bit_offset,
      ),

      &RLE_LOSSLESS => decode::rle_lossless::decode_single_channel(
        &self.image_pixel_module,
        data,
      ),

      &JPEG_BASELINE_8BIT => {
        decode::zune_jpeg::decode_single_channel(&self.image_pixel_module, data)
      }

      &JPEG_EXTENDED_12BIT => decode::libjpeg_12bit::decode_single_channel(
        &self.image_pixel_module,
        data,
      ),

      &JPEG_LOSSLESS_NON_HIERARCHICAL | &JPEG_LOSSLESS_NON_HIERARCHICAL_SV1 => {
        decode::jpeg_decoder::decode_single_channel(
          &self.image_pixel_module,
          data,
        )
      }

      &JPEG_2K
      | &JPEG_2K_LOSSLESS_ONLY
      | &HIGH_THROUGHPUT_JPEG_2K
      | &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY
      | &HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY => {
        decode::openjpeg::decode_single_channel(&self.image_pixel_module, data)
      }

      &JPEG_XL_LOSSLESS | &JPEG_XL_JPEG_RECOMPRESSION | &JPEG_XL => {
        decode::jxl_oxide::decode_single_channel(&self.image_pixel_module, data)
      }

      #[cfg(not(target_arch = "wasm32"))]
      &JPEG_LS_LOSSLESS | &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
        decode::charls::decode_single_channel(&self.image_pixel_module, data)
      }

      &DEFLATED_IMAGE_FRAME_COMPRESSION => {
        decode::native::decode_single_channel(
          &self.image_pixel_module,
          &self.inflate_frame_data(data)?,
          0,
        )
      }

      _ => Err(DataError::new_value_unsupported(format!(
        "Transfer syntax '{}' is not able to be decoded",
        self.transfer_syntax.name,
      ))),
    }?;

    if self
      .image_pixel_module
      .photometric_interpretation()
      .is_monochrome1()
    {
      image.invert_monochrome1_data();
    }

    Ok(image)
  }

  /// Decodes a frame of color pixel data into a [`ColorImage`].
  ///
  pub fn decode_color_frame(
    &self,
    frame: &mut PixelDataFrame,
  ) -> Result<ColorImage, DataError> {
    let data = frame.combine_fragments();

    use transfer_syntax::*;

    match self.transfer_syntax {
      &IMPLICIT_VR_LITTLE_ENDIAN
      | &EXPLICIT_VR_LITTLE_ENDIAN
      | &ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
      | &DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
      | &EXPLICIT_VR_BIG_ENDIAN => {
        decode::native::decode_color(&self.image_pixel_module, data)
      }

      &RLE_LOSSLESS => {
        decode::rle_lossless::decode_color(&self.image_pixel_module, data)
      }

      &JPEG_BASELINE_8BIT => {
        decode::zune_jpeg::decode_color(&self.image_pixel_module, data)
      }

      &JPEG_EXTENDED_12BIT => {
        decode::libjpeg_12bit::decode_color(&self.image_pixel_module, data)
      }

      &JPEG_LOSSLESS_NON_HIERARCHICAL | &JPEG_LOSSLESS_NON_HIERARCHICAL_SV1 => {
        decode::jpeg_decoder::decode_color(&self.image_pixel_module, data)
      }

      &JPEG_2K
      | &JPEG_2K_LOSSLESS_ONLY
      | &HIGH_THROUGHPUT_JPEG_2K
      | &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY
      | &HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY => {
        decode::openjpeg::decode_color(&self.image_pixel_module, data)
      }

      &JPEG_XL_LOSSLESS | &JPEG_XL_JPEG_RECOMPRESSION | &JPEG_XL => {
        decode::jxl_oxide::decode_color(&self.image_pixel_module, data)
      }

      #[cfg(not(target_arch = "wasm32"))]
      &JPEG_LS_LOSSLESS | &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
        decode::charls::decode_color(&self.image_pixel_module, data)
      }

      &DEFLATED_IMAGE_FRAME_COMPRESSION => decode::native::decode_color(
        &self.image_pixel_module,
        &self.inflate_frame_data(data)?,
      ),

      _ => Err(DataError::new_value_unsupported(format!(
        "Transfer syntax '{}' is not able to be decoded",
        self.transfer_syntax.name
      ))),
    }
  }

  /// Inflates deflated data for a single frame. This is used by the 'Deflated
  /// Image Frame Compression' transfer syntax.
  ///
  fn inflate_frame_data(&self, data: &[u8]) -> Result<Vec<u8>, DataError> {
    let mut decompressor = flate2::Decompress::new(false);
    let mut inflated_data =
      vec![0u8; self.image_pixel_module.frame_size_in_bytes()];

    match decompressor.decompress(
      data,
      &mut inflated_data,
      flate2::FlushDecompress::Finish,
    ) {
      Ok(status) => {
        if status != flate2::Status::StreamEnd {
          return Err(DataError::new_value_invalid(
            "Frame data inflate did not reach the end of the stream"
              .to_string(),
          ));
        }

        if decompressor.total_out() != inflated_data.len() as u64 {
          return Err(DataError::new_value_invalid(format!(
            "Frame data inflate produced {} bytes but {} bytes were expected",
            decompressor.total_out(),
            inflated_data.len()
          )));
        }

        Ok(inflated_data)
      }

      Err(_) => Err(DataError::new_value_invalid(
        "Frame data inflate failed".to_string(),
      )),
    }
  }
}
