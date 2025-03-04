use image::{GrayImage, RgbImage};

use dcmfx_core::{
  DataElementTag, DataError, DataSet, TransferSyntax, dictionary,
  transfer_syntax,
};

use crate::{
  ColorPalette, ModalityLut, PixelDataDefinition, PixelDataFrame, VoiLut,
  decode,
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
  /// A color palette can optionally be applied to grayscale images. The
  /// well-known color palettes defined in PS3.6 B.1 are provided in
  /// [`crate::luts::color_palettes`].
  ///
  pub fn read_frame(
    &self,
    frame: &mut PixelDataFrame,
    color_palette: Option<&ColorPalette>,
  ) -> Result<RgbImage, DataError> {
    if self.definition.is_grayscale() {
      let image = self.read_single_channel_frame(frame)?;

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
      self.read_color_frame(frame)
    }
  }

  /// Reads a frame of grayscale pixel data into a [`GrayImage`], applying the
  /// Modality LUT and VOI LUT.
  ///
  pub fn read_single_channel_frame(
    &self,
    frame: &mut PixelDataFrame,
  ) -> Result<GrayImage, DataError> {
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

      &JPEG_2K
      | &JPEG_2K_LOSSLESS_ONLY
      | &HIGH_THROUGHPUT_JPEG_2K
      | &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY
      | &HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY => {
        decode::openjpeg::decode_single_channel(&self.definition, data)
      }

      _ => Err(DataError::new_value_unsupported(format!(
        "The transfer syntax '{}' is not supported for grayscale decode",
        self.transfer_syntax.name,
      ))),
    }?;

    image.invert_monochrome1_data(&self.definition);

    let mut voi_lut = &self.voi_lut;

    // If the VOI LUT is empty then fall back to using a VOI window that covers
    // the entire range of values in the image
    let mut fallback_voi_lut = VoiLut {
      luts: vec![],
      windows: vec![],
    };
    if voi_lut.is_empty() {
      if let Some(voi_window) = image.fallback_voi_window() {
        dbg!(&voi_window);
        fallback_voi_lut.windows.push(voi_window);
        voi_lut = &fallback_voi_lut;
      }
    }

    Ok(image.to_gray_image(&self.modality_lut, voi_lut))
  }

  /// Reads a frame of color pixel data into an [`RgbImage`].
  ///
  pub fn read_color_frame(
    &self,
    frame: &mut PixelDataFrame,
  ) -> Result<RgbImage, DataError> {
    let data = frame.combine_fragments();

    use transfer_syntax::*;

    let image = match self.transfer_syntax {
      &IMPLICIT_VR_LITTLE_ENDIAN
      | &EXPLICIT_VR_LITTLE_ENDIAN
      | &ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
      | &DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
      | &EXPLICIT_VR_BIG_ENDIAN => {
        let mut img = decode::native::decode_color(&self.definition, data)?;

        // Convert YBR to RGB if needed
        if self.definition.photometric_interpretation.is_ybr() {
          img.convert_ybr_to_rgb(&self.definition);
        }

        Ok(img)
      }

      &RLE_LOSSLESS => {
        let mut img =
          decode::rle_lossless::decode_color(&self.definition, data)?;

        // Convert YBR to RGB if needed
        if self.definition.photometric_interpretation.is_ybr() {
          img.convert_ybr_to_rgb(&self.definition);
        }

        Ok(img)
      }

      &JPEG_BASELINE_8BIT => decode::jpeg::decode_color(&self.definition, data),

      &JPEG_2K
      | &JPEG_2K_LOSSLESS_ONLY
      | &HIGH_THROUGHPUT_JPEG_2K
      | &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY
      | &HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY => {
        decode::openjpeg::decode_color(&self.definition, data)
      }

      _ => Err(DataError::new_value_unsupported(format!(
        "Reading transfer syntax '{}' is not supported",
        self.transfer_syntax.name
      ))),
    }?;

    Ok(image.to_rgb_u8_image(&self.definition))
  }
}
