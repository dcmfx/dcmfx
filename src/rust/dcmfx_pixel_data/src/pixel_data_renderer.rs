#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use dcmfx_core::{
  DataElementTag, DataError, DataSet, DataSetPath, IodModule, TransferSyntax,
  ValueRepresentation, dictionary, transfer_syntax,
};

use crate::{
  ColorImage, GrayscalePipeline, MonochromeImage, PixelDataDecodeError,
  PixelDataFrame, StandardColorPalette, decode, iods::ImagePixelModule,
};

/// Defines a pixel data renderer that can take a [`PixelDataFrame`] and render
/// it into a [`MonochromeImage`], [`ColorImage`], or [`image::RgbImage`].
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
  /// Renders a frame of pixel data to an RGB 8-bit image. The grayscale
  /// pipeline is applied to monochrome images, and resulting grayscale values
  /// are then expanded to RGB.
  ///
  /// Monochrome frames can optionally be visualized using a color palette. The
  /// well-known color palettes defined in PS3.6 B.1 are provided in
  /// [`crate::standard_color_palettes`].
  ///
  pub fn render_frame(
    &self,
    frame: &mut PixelDataFrame,
    color_palette: Option<&StandardColorPalette>,
  ) -> Result<image::RgbImage, PixelDataDecodeError> {
    if self.image_pixel_module.is_monochrome() {
      let image = decode::decode_monochrome(
        frame,
        self.transfer_syntax,
        &self.image_pixel_module,
      )?;

      Ok(self.render_monochrome_image(&image, color_palette))
    } else {
      let image = decode::decode_color(
        frame,
        self.transfer_syntax,
        &self.image_pixel_module,
      )?;

      Ok(image.into_rgb_u8_image())
    }
  }

  /// Renders a [`MonochromeImage`] to an RGB 8-bit image. The grayscale
  /// pipeline is applied, and resulting grayscale values are then expanded
  /// to RGB.
  ///
  /// The result can optionally be visualized using a color palette. The
  /// well-known color palettes defined in PS3.6 B.1 are provided in
  /// [`crate::standard_color_palettes`].
  ///
  pub fn render_monochrome_image(
    &self,
    image: &MonochromeImage,
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

  /// Decodes a frame of monochrome pixel data into a [`MonochromeImage`]. The
  /// returned image needs to have a grayscale pipeline applied in order to
  /// reach final grayscale display values.
  ///
  pub fn decode_monochrome_frame(
    &self,
    frame: &mut PixelDataFrame,
  ) -> Result<MonochromeImage, PixelDataDecodeError> {
    decode::decode_monochrome(
      frame,
      self.transfer_syntax,
      &self.image_pixel_module,
    )
  }

  /// Decodes a frame of color pixel data into a [`ColorImage`].
  ///
  pub fn decode_color_frame(
    &self,
    frame: &mut PixelDataFrame,
  ) -> Result<ColorImage, PixelDataDecodeError> {
    decode::decode_color(frame, self.transfer_syntax, &self.image_pixel_module)
  }
}
