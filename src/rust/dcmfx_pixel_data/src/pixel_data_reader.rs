use image::{GrayImage, RgbImage};

use dcmfx_core::{
  dictionary, transfer_syntax, DataElementTag, DataError, DataSet,
  TransferSyntax,
};

use crate::{
  pixel_data_native::{iter_pixels_color, iter_pixels_grayscale},
  ModalityLut, PhotometricInterpretation, PixelDataDefinition, PixelDataFrame,
  VoiLut,
};

/// Defines a pixel data reader that can take a [`PixelDataFrame`] and decode it
/// into an [`GrayImage`] or [`RgbImage`].
///
#[derive(Clone, Debug, PartialEq)]
pub struct PixelDataReader {
  pub transfer_syntax: &'static TransferSyntax,
  pub definition: PixelDataDefinition,
  pub modality_lut: ModalityLut,
  pub voi_lut: VoiLut,
}

impl PixelDataReader {
  /// The tags of the data elements that are read when creating a new
  /// [`PixelDataReader`].
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

  /// Creates a pixel data reader for reading frames of pixel data from a data
  /// set containing the tags listed in [`Self::DATA_ELEMENT_TAGS`].
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

    Ok(PixelDataReader {
      transfer_syntax,
      definition,
      modality_lut,
      voi_lut,
    })
  }

  /// Decodes a frame of pixel data to an RGB image, applying the Modality LUT
  /// and VOI LUT to grayscale pixels. Grayscale images are automatically
  /// expanded to RGB.
  ///
  pub fn decode_frame(
    &self,
    frame: &mut PixelDataFrame,
  ) -> Result<RgbImage, DataError> {
    if self.transfer_syntax.is_encapsulated {
      return Err(DataError::new_value_unsupported(
        "Reading encapsulated pixel data is not supported".to_string(),
      ));
    }

    let width = self.definition.columns;
    let height = self.definition.rows;

    let mut pixels = Vec::with_capacity(width as usize * height as usize * 3);

    let data = frame.combine_fragments();

    if self.definition.is_grayscale() {
      let pixel_iterator =
        iter_pixels_grayscale(self.definition.clone(), data)?;
      let monochrome1_conversion_offset = self.monochrome1_conversion_offset();

      for mut pixel in pixel_iterator {
        // Invert Monochrome1 data if needed
        if let Some(offset) = monochrome1_conversion_offset.as_ref() {
          pixel = offset - pixel;
        }

        // Apply LUTs
        let x = self.modality_lut.apply(pixel);
        let x = self.voi_lut.apply(x);

        // Convert to u8
        let x = (x * 255.0).clamp(0.0, 255.0) as u8;

        pixels.push(x);
        pixels.push(x);
        pixels.push(x);
      }
    } else {
      for pixel in iter_pixels_color(self.definition.clone(), data)? {
        pixels.push((pixel.0 * 255.0).clamp(0.0, 255.0) as u8);
        pixels.push((pixel.1 * 255.0).clamp(0.0, 255.0) as u8);
        pixels.push((pixel.2 * 255.0).clamp(0.0, 255.0) as u8);
      }
    }

    Ok(RgbImage::from_raw(width as u32, height as u32, pixels).unwrap())
  }

  /// Decodes a frame of grayscale pixel data to a [`GrayImage`], applying the
  /// Modality LUT and VOI LUT.
  ///
  pub fn decode_grayscale_frame(
    &self,
    frame: &mut PixelDataFrame,
  ) -> Result<GrayImage, DataError> {
    if self.transfer_syntax.is_encapsulated {
      return Err(DataError::new_value_unsupported(
        "Reading encapsulated pixel data is not supported".to_string(),
      ));
    }

    let width = self.definition.columns as u32;
    let height = self.definition.rows as u32;

    let mut pixels = Vec::with_capacity(width as usize * height as usize);

    let data = frame.combine_fragments();
    let pixel_iterator = iter_pixels_grayscale(self.definition.clone(), data)?;
    let monochrome1_conversion_offset = self.monochrome1_conversion_offset();

    for mut pixel in pixel_iterator {
      // Invert Monochrome1 data if needed
      if let Some(offset) = monochrome1_conversion_offset.as_ref() {
        pixel = offset - pixel;
      }

      // Apply LUTs
      let x = self.modality_lut.apply(pixel);
      let x = self.voi_lut.apply(x);

      // Convert to u8
      let x = (x * 255.0).clamp(0.0, 255.0) as u8;

      pixels.push(x);
    }

    Ok(GrayImage::from_raw(width, height, pixels).unwrap())
  }

  /// For Monochrome1 pixel data, returns the offset to add after negating the
  /// stored pixel value in order to convert to Monochrome2.
  ///
  fn monochrome1_conversion_offset(&self) -> Option<i64> {
    if self.definition.photometric_interpretation
      == PhotometricInterpretation::Monochrome1
    {
      if self.definition.pixel_representation.is_signed() {
        Some(-1)
      } else {
        Some((1i64 << self.definition.bits_stored) - 1)
      }
    } else {
      None
    }
  }

  /// Decodes a frame of color pixel data to an [`RgbImage`].
  ///
  pub fn decode_color_frame(
    &self,
    frame: &mut PixelDataFrame,
  ) -> Result<RgbImage, DataError> {
    if self.transfer_syntax.is_encapsulated {
      return Err(DataError::new_value_unsupported(
        "Reading encapsulated pixel data is not supported".to_string(),
      ));
    }

    let width = self.definition.columns;
    let height = self.definition.rows;

    let mut pixels = Vec::with_capacity(width as usize * height as usize * 3);

    let data = frame.combine_fragments();
    for pixel in iter_pixels_color(self.definition.clone(), data)? {
      pixels.push((pixel.0 * 255.0).clamp(0.0, 255.0) as u8);
      pixels.push((pixel.1 * 255.0).clamp(0.0, 255.0) as u8);
      pixels.push((pixel.2 * 255.0).clamp(0.0, 255.0) as u8);
    }

    Ok(RgbImage::from_raw(width as u32, height as u32, pixels).unwrap())
  }
}
