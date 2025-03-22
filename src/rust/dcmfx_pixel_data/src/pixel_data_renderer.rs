#[cfg(not(feature = "std"))]
use alloc::{format, vec, vec::Vec};

use image::RgbImage;

use dcmfx_core::{
  DataElementTag, DataError, DataSet, TransferSyntax, dictionary,
  transfer_syntax,
};

use crate::{
  ColorImage, ColorPalette, ModalityLut, PixelDataDefinition, PixelDataFrame,
  SingleChannelImage, VoiLut, decode,
};

/// Defines a pixel data renderer that can take a [`PixelDataFrame`] and render
/// it into a [`SingleChannelImage`], [`ColorImage`], or [`RgbImage`].
///
#[derive(Clone, Debug, PartialEq)]
pub struct PixelDataRenderer {
  pub transfer_syntax: &'static TransferSyntax,
  pub definition: PixelDataDefinition,
  pub modality_lut: ModalityLut,
  pub voi_lut: VoiLut,
}

impl PixelDataRenderer {
  /// The tags of the data elements that are read when creating a new
  /// [`PixelDataRenderer`].
  ///
  pub const DATA_ELEMENT_TAGS: [DataElementTag; 28] = [
    dictionary::SAMPLES_PER_PIXEL.tag,
    dictionary::PHOTOMETRIC_INTERPRETATION.tag,
    dictionary::PLANAR_CONFIGURATION.tag,
    dictionary::ROWS.tag,
    dictionary::COLUMNS.tag,
    dictionary::BITS_ALLOCATED.tag,
    dictionary::BITS_STORED.tag,
    dictionary::HIGH_BIT.tag,
    dictionary::PIXEL_REPRESENTATION.tag,
    dictionary::RED_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag,
    dictionary::RED_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag,
    dictionary::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag,
    dictionary::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::LUT_DESCRIPTOR.tag,
    dictionary::LUT_EXPLANATION.tag,
    dictionary::LUT_DATA.tag,
    dictionary::MODALITY_LUT_SEQUENCE.tag,
    dictionary::MODALITY_LUT_TYPE.tag,
    dictionary::RESCALE_INTERCEPT.tag,
    dictionary::RESCALE_SLOPE.tag,
    dictionary::RESCALE_TYPE.tag,
    dictionary::VOILUT_SEQUENCE.tag,
    dictionary::WINDOW_CENTER.tag,
    dictionary::WINDOW_WIDTH.tag,
    dictionary::WINDOW_CENTER_WIDTH_EXPLANATION.tag,
    dictionary::VOILUT_FUNCTION.tag,
  ];

  /// Creates a pixel data renderer for rendering frames of pixel data in a
  /// data set into a [`SingleChannelImage`], [`ColorImage`], or [`RgbImage`].
  ///
  /// This references the data elements specified in
  /// [`Self::DATA_ELEMENT_TAGS`].
  ///
  pub fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let transfer_syntax = if data_set.has(dictionary::TRANSFER_SYNTAX_UID.tag) {
      data_set.get_transfer_syntax()?
    } else {
      &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN
    };

    let definition = PixelDataDefinition::from_data_set(data_set)?;
    let modality_lut = ModalityLut::from_data_set(data_set)?;
    let voi_lut = VoiLut::from_data_set(data_set)?;

    Ok(PixelDataRenderer {
      transfer_syntax,
      definition,
      modality_lut,
      voi_lut,
    })
  }

  /// Renders a frame of pixel data to an RGB 8-bit image. The Modality LUT and
  /// VOI LUT are applied to single channel images, and resulting grayscale
  /// values are then expanded to RGB.
  ///
  /// Grayscale values can optionally be visualized using a color palette. The
  /// well-known color palettes defined in PS3.6 B.1 are provided in
  /// [`crate::luts::color_palettes`].
  ///
  pub fn render_frame(
    &self,
    frame: &mut PixelDataFrame,
    color_palette: Option<&ColorPalette>,
  ) -> Result<RgbImage, DataError> {
    if self.definition.is_grayscale() {
      let image = self.render_single_channel_frame(frame)?;

      let mut voi_lut = &self.voi_lut;

      // If the VOI LUT is empty then fall back to using a VOI window that
      // covers the entire range of values in the image
      let mut fallback_voi_lut = VoiLut {
        luts: vec![],
        windows: vec![],
      };
      if voi_lut.is_empty() {
        if let Some(voi_window) = image.fallback_voi_window() {
          fallback_voi_lut.windows.push(voi_window);
          voi_lut = &fallback_voi_lut;
        }
      }

      let image = image.to_gray_image(&self.modality_lut, voi_lut);

      let mut pixels = Vec::with_capacity(
        image.width() as usize * image.height() as usize * 3,
      );

      if let Some(color_palette) = color_palette {
        for pixel in image.pixels() {
          pixels.extend_from_slice(&color_palette.lookup(pixel.0[0]));
        }
      } else {
        for pixel in image.pixels() {
          pixels.push(pixel.0[0]);
          pixels.push(pixel.0[0]);
          pixels.push(pixel.0[0]);
        }
      }

      Ok(RgbImage::from_raw(image.width(), image.height(), pixels).unwrap())
    } else {
      Ok(
        self
          .render_color_frame(frame)?
          .to_rgb_u8_image(&self.definition),
      )
    }
  }

  /// Renders a frame of single channel pixel data into a
  /// [`SingleChannelImage`]. The returned image needs to have the Modality LUT
  /// and VOI LUT applied in order to reach final grayscale display values.
  ///
  pub fn render_single_channel_frame(
    &self,
    frame: &mut PixelDataFrame,
  ) -> Result<SingleChannelImage, DataError> {
    let data = frame.combine_fragments();

    use transfer_syntax::*;

    let mut image = match self.transfer_syntax {
      &IMPLICIT_VR_LITTLE_ENDIAN
      | &EXPLICIT_VR_LITTLE_ENDIAN
      | &ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
      | &DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
      | &EXPLICIT_VR_BIG_ENDIAN => {
        decode::native::decode_single_channel(&self.definition, data)
      }

      &RLE_LOSSLESS => {
        decode::rle_lossless::decode_single_channel(&self.definition, data)
      }

      &JPEG_BASELINE_8BIT => {
        decode::jpeg::decode_single_channel(&self.definition, data)
      }

      &JPEG_EXTENDED_12BIT => {
        decode::libjpeg_12bit::decode_single_channel(&self.definition, data)
      }

      &JPEG_LOSSLESS_NON_HIERARCHICAL | &JPEG_LOSSLESS_NON_HIERARCHICAL_SV1 => {
        decode::jpeg_decoder::decode_single_channel(&self.definition, data)
      }

      &JPEG_2K
      | &JPEG_2K_LOSSLESS_ONLY
      | &HIGH_THROUGHPUT_JPEG_2K
      | &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY
      | &HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY => {
        decode::openjpeg::decode_single_channel(&self.definition, data)
      }

      &JPEG_XL_LOSSLESS | &JPEG_XL_JPEG_RECOMPRESSION | &JPEG_XL => {
        decode::jxl_oxide::decode_single_channel(&self.definition, data)
      }

      #[cfg(not(target_arch = "wasm32"))]
      &JPEG_LS_LOSSLESS | &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
        decode::charls::decode_single_channel(&self.definition, data)
      }

      _ => Err(DataError::new_value_unsupported(format!(
        "Transfer syntax '{}' is not supported",
        self.transfer_syntax.name,
      ))),
    }?;

    image.invert_monochrome1_data(&self.definition);

    Ok(image)
  }

  /// Renders a frame of color pixel data into a [`ColorImage`].
  ///
  pub fn render_color_frame(
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
        decode::native::decode_color(&self.definition, data)
      }

      &RLE_LOSSLESS => {
        decode::rle_lossless::decode_color(&self.definition, data)
      }

      &JPEG_BASELINE_8BIT => decode::jpeg::decode_color(&self.definition, data),

      &JPEG_EXTENDED_12BIT => {
        decode::libjpeg_12bit::decode_color(&self.definition, data)
      }

      &JPEG_LOSSLESS_NON_HIERARCHICAL | &JPEG_LOSSLESS_NON_HIERARCHICAL_SV1 => {
        decode::jpeg_decoder::decode_color(&self.definition, data)
      }

      &JPEG_2K
      | &JPEG_2K_LOSSLESS_ONLY
      | &HIGH_THROUGHPUT_JPEG_2K
      | &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY
      | &HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY => {
        decode::openjpeg::decode_color(&self.definition, data)
      }

      &JPEG_XL_LOSSLESS | &JPEG_XL_JPEG_RECOMPRESSION | &JPEG_XL => {
        decode::jxl_oxide::decode_color(&self.definition, data)
      }

      #[cfg(not(target_arch = "wasm32"))]
      &JPEG_LS_LOSSLESS | &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
        decode::charls::decode_color(&self.definition, data)
      }

      _ => Err(DataError::new_value_unsupported(format!(
        "Transfer syntax '{}' is not supported",
        self.transfer_syntax.name
      ))),
    }
  }
}
