#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec};

use dcmfx_core::DataError;

use crate::{
  PixelDataDefinition, PixelRepresentation, SingleChannelImage,
  color_image::ColorImage,
  decode::ybr_to_rgb,
  pixel_data_definition::{
    BitsAllocated, PhotometricInterpretation, PlanarConfiguration,
    SamplesPerPixel,
  },
};

/// Decodes stored values for native single channel pixel data that uses the
/// [`PhotometricInterpretation::Monochrome1`] or
/// [`PhotometricInterpretation::Monochrome2`] photometric interpretations.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
  data_bit_offset: usize,
) -> Result<SingleChannelImage, DataError> {
  // Check that there is one sample per pixel
  if definition.samples_per_pixel != SamplesPerPixel::One {
    return Err(DataError::new_value_invalid(
      "Samples per pixel is not one for grayscale pixel data".to_string(),
    ));
  }

  validate_data_length(definition, data)?;

  let width = definition.columns;
  let height = definition.rows;
  let pixel_count = definition.pixel_count();

  match definition.photometric_interpretation {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2 => {
      match (definition.pixel_representation, definition.bits_allocated) {
        (_, BitsAllocated::One) => {
          let is_signed = definition.pixel_representation.is_signed();
          let mut data = data.to_vec();

          if data_bit_offset > 0 {
            for i in 0..data.len() {
              let next_byte = data.get(i + 1).unwrap_or(&0);
              data[i] = (data[i] >> data_bit_offset)
                | (next_byte << (8 - data_bit_offset));
            }

            // It's possible there will be an unneeded trailing byte after
            // adjusting for the bit offset, so remove it if present
            data.resize_with((definition.pixel_count() + 7) / 8, || 0);
          }

          Ok(
            SingleChannelImage::new_bitmap(width, height, data, is_signed)
              .unwrap(),
          )
        }

        (PixelRepresentation::Signed, BitsAllocated::Eight) => {
          let mut pixels = vec![0i8; pixel_count];

          for i in 0..pixel_count {
            pixels[i] = i8::from_le_bytes([data[i]]);
          }

          Ok(SingleChannelImage::new_i8(width, height, pixels).unwrap())
        }

        (PixelRepresentation::Unsigned, BitsAllocated::Eight) => {
          Ok(SingleChannelImage::new_u8(width, height, data.to_vec()).unwrap())
        }

        (PixelRepresentation::Signed, BitsAllocated::Sixteen) => {
          let mut pixels = vec![0i16; pixel_count];

          for i in 0..pixel_count {
            pixels[i] = i16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
          }

          Ok(SingleChannelImage::new_i16(width, height, pixels).unwrap())
        }

        (PixelRepresentation::Unsigned, BitsAllocated::Sixteen) => {
          let mut pixels = vec![0u16; pixel_count];

          for i in 0..pixel_count {
            pixels[i] = u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
          }

          Ok(SingleChannelImage::new_u16(width, height, pixels).unwrap())
        }

        (PixelRepresentation::Signed, BitsAllocated::ThirtyTwo) => {
          let mut pixels = vec![0i32; pixel_count];

          for i in 0..pixel_count {
            pixels[i] = i32::from_le_bytes([
              data[i * 4],
              data[i * 4 + 1],
              data[i * 4 + 2],
              data[i * 4 + 3],
            ]);
          }

          Ok(SingleChannelImage::new_i32(width, height, pixels).unwrap())
        }

        (PixelRepresentation::Unsigned, BitsAllocated::ThirtyTwo) => {
          let mut pixels = vec![0u32; pixel_count];

          for i in 0..pixel_count {
            pixels[i] = u32::from_le_bytes([
              data[i * 4],
              data[i * 4 + 1],
              data[i * 4 + 2],
              data[i * 4 + 3],
            ]);
          }

          Ok(SingleChannelImage::new_u32(width, height, pixels).unwrap())
        }
      }
    }

    _ => Err(DataError::new_value_invalid(format!(
      "Photometric interpretation '{}' is invalid for grayscale pixel data \
       when samples per pixel is one",
      definition.photometric_interpretation
    ))),
  }
}

/// Decodes native color pixel data that uses the
/// [`PhotometricInterpretation::Rgb`], [`PhotometricInterpretation::YbrFull`],
/// [`PhotometricInterpretation::YbrFull422`], or
/// [`PhotometricInterpretation::PaletteColor`] photometric interpretations.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  validate_data_length(definition, data)?;

  let width = definition.columns;
  let height = definition.rows;
  let pixel_count = definition.pixel_count();

  match definition.samples_per_pixel {
    SamplesPerPixel::One => match (
      &definition.photometric_interpretation,
      definition.bits_allocated,
    ) {
      (
        PhotometricInterpretation::PaletteColor { palette },
        BitsAllocated::Eight,
      ) => Ok(
        ColorImage::new_palette8(width, height, data.to_vec(), palette.clone())
          .unwrap(),
      ),

      (
        PhotometricInterpretation::PaletteColor { palette },
        BitsAllocated::Sixteen,
      ) => {
        let mut pixels = vec![0u16; pixel_count * 3];

        for i in 0..pixel_count {
          pixels.push(u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]));
        }

        Ok(
          ColorImage::new_palette16(width, height, pixels, palette.clone())
            .unwrap(),
        )
      }

      (photometric_interpretation, bits_allocated) => {
        Err(DataError::new_value_invalid(format!(
          "Photometric interpretation '{}' is invalid for color pixel data \
           when samples per pixel is one and bits allocated is '{}'",
          photometric_interpretation,
          usize::from(bits_allocated)
        )))
      }
    },

    SamplesPerPixel::Three {
      planar_configuration,
    } => match definition.photometric_interpretation {
      PhotometricInterpretation::Rgb | PhotometricInterpretation::YbrFull => {
        match (planar_configuration, definition.bits_allocated) {
          (_, BitsAllocated::One) => Err(DataError::new_value_invalid(
            "Bits allocated value '1' is not supported for color data"
              .to_string(),
          )),

          (PlanarConfiguration::Interleaved, BitsAllocated::Eight) => {
            let mut pixels = data[..(pixel_count * 3)].to_vec();

            if definition.photometric_interpretation.is_ybr() {
              ybr_to_rgb::convert_u8(&mut pixels, definition);
            }

            Ok(ColorImage::new_u8(width, height, pixels).unwrap())
          }

          (PlanarConfiguration::Interleaved, BitsAllocated::Sixteen) => {
            let mut pixels = vec![0u16; pixel_count * 3];

            for i in 0..(pixel_count * 3) {
              pixels[i] = u16::from_le_bytes([data[i * 2], data[i * 2] + 1]);
            }

            if definition.photometric_interpretation.is_ybr() {
              ybr_to_rgb::convert_u16(&mut pixels, definition);
            }

            Ok(ColorImage::new_u16(width, height, pixels).unwrap())
          }

          (PlanarConfiguration::Interleaved, BitsAllocated::ThirtyTwo) => {
            let mut pixels = vec![0u32; pixel_count * 3];

            for i in 0..(pixel_count * 3) {
              pixels[i] = u32::from_le_bytes([
                data[i * 4],
                data[i * 4] + 1,
                data[i * 4] + 2,
                data[i * 4] + 3,
              ]);
            }

            if definition.photometric_interpretation.is_ybr() {
              ybr_to_rgb::convert_u32(&mut pixels, definition);
            }

            Ok(ColorImage::new_u32(width, height, pixels).unwrap())
          }

          (PlanarConfiguration::Separate, BitsAllocated::Eight) => {
            let mut pixels = vec![0u8; pixel_count * 3];

            for i in 0..pixel_count {
              pixels[i * 3] = data[i];
              pixels[i * 3 + 1] = data[pixel_count + i];
              pixels[i * 3 + 2] = data[pixel_count * 2 + i];
            }

            if definition.photometric_interpretation.is_ybr() {
              ybr_to_rgb::convert_u8(&mut pixels, definition);
            }

            Ok(ColorImage::new_u8(width, height, pixels).unwrap())
          }

          (PlanarConfiguration::Separate, BitsAllocated::Sixteen) => {
            let mut pixels = vec![0u16; pixel_count * 3];

            for i in 0..pixel_count {
              pixels[i * 3] =
                u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);

              pixels[i * 3 + 1] = u16::from_le_bytes([
                data[(pixel_count + i) * 2],
                data[(pixel_count + i) * 2 + 1],
              ]);

              pixels[i * 3 + 2] = u16::from_le_bytes([
                data[(pixel_count * 2 + i) * 2],
                data[(pixel_count * 2 + i) * 2 + 1],
              ]);
            }

            if definition.photometric_interpretation.is_ybr() {
              ybr_to_rgb::convert_u16(&mut pixels, definition);
            }

            Ok(ColorImage::new_u16(width, height, pixels).unwrap())
          }

          (PlanarConfiguration::Separate, BitsAllocated::ThirtyTwo) => {
            let mut pixels = vec![0u32; pixel_count * 3];

            for i in 0..pixel_count {
              pixels[i * 3] = u32::from_le_bytes([
                data[i * 4],
                data[i * 4 + 1],
                data[i * 4 + 2],
                data[i * 4 + 3],
              ]);

              pixels[i * 3 + 1] = u32::from_le_bytes([
                data[(pixel_count + i) * 4],
                data[(pixel_count + i) * 4 + 1],
                data[(pixel_count + i) * 4 + 2],
                data[(pixel_count + i) * 4 + 3],
              ]);

              pixels[i * 3 + 2] = u32::from_le_bytes([
                data[(pixel_count * 2 + i) * 4],
                data[(pixel_count * 2 + i) * 4 + 1],
                data[(pixel_count * 2 + i) * 4 + 2],
                data[(pixel_count * 2 + i) * 4 + 3],
              ]);
            }

            if definition.photometric_interpretation.is_ybr() {
              ybr_to_rgb::convert_u32(&mut pixels, definition);
            }

            Ok(ColorImage::new_u32(width, height, pixels).unwrap())
          }
        }
      }

      PhotometricInterpretation::YbrFull422 => {
        if definition.columns % 2 == 1 {
          return Err(DataError::new_value_invalid(
            "YBR_FULL_222 pixel data width is odd".to_string(),
          ));
        }

        match (planar_configuration, definition.bits_allocated) {
          (_, BitsAllocated::One) => Err(DataError::new_value_invalid(
            "Bits allocated value '1' is not supported for color data"
              .to_string(),
          )),

          (PlanarConfiguration::Interleaved, BitsAllocated::Eight) => {
            let mut pixels = vec![0u8; pixel_count * 3];

            for i in 0..(pixel_count / 2) {
              let y0 = data[i * 4];
              let y1 = data[i * 4 + 1];
              let cb = data[i * 4 + 2];
              let cr = data[i * 4 + 3];

              pixels[i * 6] = y0;
              pixels[i * 6 + 1] = cb;
              pixels[i * 6 + 2] = cr;
              pixels[i * 6 + 3] = y1;
              pixels[i * 6 + 4] = cb;
              pixels[i * 6 + 5] = cr;
            }

            ybr_to_rgb::convert_u8(&mut pixels, definition);

            Ok(ColorImage::new_u8(width, height, pixels).unwrap())
          }

          (PlanarConfiguration::Interleaved, BitsAllocated::Sixteen) => {
            let mut pixels = vec![0u16; pixel_count * 3];

            for i in 0..(pixel_count / 2) {
              let y0 = u16::from_le_bytes([data[i * 8], data[i * 8 + 1]]);
              let y1 = u16::from_le_bytes([data[i * 8 + 2], data[i * 8 + 3]]);
              let cb = u16::from_le_bytes([data[i * 8 + 4], data[i * 8 + 5]]);
              let cr = u16::from_le_bytes([data[i * 8 + 6], data[i * 8 + 7]]);

              pixels[i * 6] = y0;
              pixels[i * 6 + 1] = cb;
              pixels[i * 6 + 2] = cr;
              pixels[i * 6 + 3] = y1;
              pixels[i * 6 + 4] = cb;
              pixels[i * 6 + 5] = cr;
            }

            ybr_to_rgb::convert_u16(&mut pixels, definition);

            Ok(ColorImage::new_u16(width, height, pixels).unwrap())
          }

          (PlanarConfiguration::Interleaved, BitsAllocated::ThirtyTwo) => {
            let mut pixels = vec![0u32; pixel_count * 3];

            for i in 0..(pixel_count / 2) {
              let y0 = u32::from_le_bytes([
                data[i * 16],
                data[i * 16 + 1],
                data[i * 16 + 2],
                data[i * 16 + 3],
              ]);
              let y1 = u32::from_le_bytes([
                data[i * 16 + 4],
                data[i * 16 + 5],
                data[i * 16 + 6],
                data[i * 16 + 7],
              ]);
              let cb = u32::from_le_bytes([
                data[i * 16 + 8],
                data[i * 16 + 9],
                data[i * 16 + 10],
                data[i * 16 + 11],
              ]);
              let cr = u32::from_le_bytes([
                data[i * 16 + 12],
                data[i * 16 + 13],
                data[i * 16 + 14],
                data[i * 16 + 15],
              ]);

              pixels[i * 6] = y0;
              pixels[i * 6 + 1] = cb;
              pixels[i * 6 + 2] = cr;
              pixels[i * 6 + 3] = y1;
              pixels[i * 6 + 4] = cb;
              pixels[i * 6 + 5] = cr;
            }

            ybr_to_rgb::convert_u32(&mut pixels, definition);

            Ok(ColorImage::new_u32(width, height, pixels).unwrap())
          }

          (PlanarConfiguration::Separate, BitsAllocated::Eight) => {
            let mut pixels = vec![0u8; pixel_count * 3];

            for i in 0..(pixel_count / 2) {
              let y0 = data[i * 2];
              let y1 = data[i * 2 + 1];
              let cb = data[pixel_count + i];
              let cr = data[pixel_count + pixel_count / 2 + i];

              pixels[i * 6] = y0;
              pixels[i * 6 + 1] = cb;
              pixels[i * 6 + 2] = cr;
              pixels[i * 6 + 3] = y1;
              pixels[i * 6 + 4] = cb;
              pixels[i * 6 + 5] = cr;
            }

            ybr_to_rgb::convert_u8(&mut pixels, definition);

            Ok(ColorImage::new_u8(width, height, pixels).unwrap())
          }

          (PlanarConfiguration::Separate, BitsAllocated::Sixteen) => {
            let mut pixels = vec![0u16; pixel_count * 3];

            for i in 0..(pixel_count / 2) {
              let y0 = u16::from_le_bytes([data[i * 4], data[i * 4 + 1]]);
              let y1 = u16::from_le_bytes([data[i * 4 + 2], data[i * 4 + 3]]);
              let cb = u16::from_le_bytes([
                data[(pixel_count + i) * 2],
                data[(pixel_count + i) * 2 + 1],
              ]);
              let cr = u16::from_le_bytes([
                data[(pixel_count + pixel_count / 2 + i) * 2],
                data[(pixel_count + pixel_count / 2 + i) * 2 + 1],
              ]);

              pixels[i * 6] = y0;
              pixels[i * 6 + 1] = cb;
              pixels[i * 6 + 2] = cr;
              pixels[i * 6 + 3] = y1;
              pixels[i * 6 + 4] = cb;
              pixels[i * 6 + 5] = cr;
            }

            ybr_to_rgb::convert_u16(&mut pixels, definition);

            Ok(ColorImage::new_u16(width, height, pixels).unwrap())
          }

          (PlanarConfiguration::Separate, BitsAllocated::ThirtyTwo) => {
            let mut pixels = vec![0u32; pixel_count * 3];

            for i in 0..(pixel_count / 2) {
              let y0 = u32::from_le_bytes([
                data[i * 8],
                data[i * 8 + 1],
                data[i * 8 + 2],
                data[i * 8 + 3],
              ]);
              let y1 = u32::from_le_bytes([
                data[i * 8 + 4],
                data[i * 8 + 5],
                data[i * 8 + 6],
                data[i * 8 + 7],
              ]);
              let cb = u32::from_le_bytes([
                data[(pixel_count + i) * 4],
                data[(pixel_count + i) * 4 + 1],
                data[(pixel_count + i) * 4 + 2],
                data[(pixel_count + i) * 4 + 3],
              ]);
              let cr = u32::from_le_bytes([
                data[(pixel_count + pixel_count / 2 + i) * 4],
                data[(pixel_count + pixel_count / 2 + i) * 4 + 1],
                data[(pixel_count + pixel_count / 2 + i) * 4 + 2],
                data[(pixel_count + pixel_count / 2 + i) * 4 + 3],
              ]);

              pixels[i * 6] = y0;
              pixels[i * 6 + 1] = cb;
              pixels[i * 6 + 2] = cr;
              pixels[i * 6 + 3] = y1;
              pixels[i * 6 + 4] = cb;
              pixels[i * 6 + 5] = cr;
            }

            ybr_to_rgb::convert_u32(&mut pixels, definition);

            Ok(ColorImage::new_u32(width, height, pixels).unwrap())
          }
        }
      }

      _ => Err(DataError::new_value_invalid(format!(
        "Photometric interpretation '{}' is invalid for color pixel data \
           when samples per pixel is three",
        definition.photometric_interpretation
      ))),
    },
  }
}

/// Validates the length of the supplied pixel data.
///
fn validate_data_length(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<(), DataError> {
  let expected_size_in_bits =
    definition.pixel_count() * definition.pixel_size_in_bits();

  // Validate that the provided data is of the expected size
  if data.len() * 8 < expected_size_in_bits {
    return Err(DataError::new_value_invalid(format!(
      "Pixel data has incorrect length, expected {} bits but found {} bits",
      expected_size_in_bits,
      data.len() * 8,
    )));
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(not(feature = "std"))]
  use alloc::vec::Vec;

  use crate::PixelRepresentation;

  #[test]
  fn decode_monochrome_16bit_unsigned() {
    let definition = PixelDataDefinition {
      samples_per_pixel: SamplesPerPixel::One,
      photometric_interpretation: PhotometricInterpretation::Monochrome2,
      rows: 2,
      columns: 2,
      bits_allocated: BitsAllocated::Eight,
      bits_stored: 8,
      high_bit: 7,
      pixel_representation: PixelRepresentation::Unsigned,
    };

    let data = [0, 1, 2, 3];

    assert_eq!(
      decode_single_channel(&definition, &data, 0).unwrap(),
      SingleChannelImage::new_u8(2, 2, vec![0, 1, 2, 3]).unwrap()
    );
  }

  #[test]
  fn decode_rgb_8bit_interleaved() {
    let definition = PixelDataDefinition {
      samples_per_pixel: SamplesPerPixel::Three {
        planar_configuration: PlanarConfiguration::Interleaved,
      },
      photometric_interpretation: PhotometricInterpretation::Rgb,
      rows: 2,
      columns: 2,
      bits_allocated: BitsAllocated::Eight,
      bits_stored: 8,
      high_bit: 7,
      pixel_representation: PixelRepresentation::Unsigned,
    };

    let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

    assert_eq!(
      decode_color(&definition, &data).unwrap(),
      ColorImage::new_u8(2, 2, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])
        .unwrap()
    );
  }

  #[test]
  fn decode_rgb_16bit_separate() {
    let definition = PixelDataDefinition {
      samples_per_pixel: SamplesPerPixel::Three {
        planar_configuration: PlanarConfiguration::Separate,
      },
      photometric_interpretation: PhotometricInterpretation::Rgb,
      rows: 2,
      columns: 2,
      bits_allocated: BitsAllocated::Sixteen,
      bits_stored: 16,
      high_bit: 15,
      pixel_representation: PixelRepresentation::Unsigned,
    };

    let data = vec![
      0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8, 0, 9, 0, 10, 0, 11, 0,
    ];

    assert_eq!(
      decode_color(&definition, &data).unwrap(),
      ColorImage::new_u16(2, 2, vec![0, 4, 8, 1, 5, 9, 2, 6, 10, 3, 7, 11,])
        .unwrap()
    );
  }

  #[test]
  fn decode_ybr_full_8bit() {
    let definition = PixelDataDefinition {
      samples_per_pixel: SamplesPerPixel::Three {
        planar_configuration: PlanarConfiguration::Interleaved,
      },
      photometric_interpretation: PhotometricInterpretation::YbrFull,
      rows: 2,
      columns: 2,
      bits_allocated: BitsAllocated::Eight,
      bits_stored: 8,
      high_bit: 7,
      pixel_representation: PixelRepresentation::Unsigned,
    };

    let data = [142, 122, 111, 148, 118, 122, 101, 123, 127, 116, 133, 142];

    assert_eq!(
      decode_color(&definition, &data).unwrap(),
      ColorImage::new_u8(
        2,
        2,
        vec![118, 155, 132, 140, 155, 131, 100, 102, 93, 136, 103, 125]
      )
      .unwrap()
    );
  }

  #[test]
  fn decode_ybr_full_422_8bit() {
    let definition = PixelDataDefinition {
      samples_per_pixel: SamplesPerPixel::Three {
        planar_configuration: PlanarConfiguration::Interleaved,
      },
      photometric_interpretation: PhotometricInterpretation::YbrFull422,
      rows: 2,
      columns: 2,
      bits_allocated: BitsAllocated::Eight,
      bits_stored: 8,
      high_bit: 7,
      pixel_representation: PixelRepresentation::Unsigned,
    };

    let data = [142, 122, 111, 148, 118, 122, 101, 123];

    assert_eq!(
      decode_color(&definition, &data).unwrap(),
      ColorImage::new_u8(
        2,
        2,
        vec![170, 133, 112, 150, 113, 92, 111, 130, 71, 115, 134, 75]
      )
      .unwrap()
    );
  }
}
