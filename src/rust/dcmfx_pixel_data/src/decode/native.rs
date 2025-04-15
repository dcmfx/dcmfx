#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec};

use dcmfx_core::DataError;

use crate::{
  ColorImage, ColorSpace, SingleChannelImage,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation, PlanarConfiguration, SamplesPerPixel,
  },
};

/// Decodes stored values for native single channel pixel data that uses the
/// [`PhotometricInterpretation::Monochrome1`] or
/// [`PhotometricInterpretation::Monochrome2`] photometric interpretations.
///
pub fn decode_single_channel(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
  data_bit_offset: usize,
) -> Result<SingleChannelImage, DataError> {
  // Check that there is one sample per pixel
  if image_pixel_module.samples_per_pixel() != SamplesPerPixel::One {
    return Err(DataError::new_value_invalid(
      "Samples per pixel is not one for grayscale pixel data".to_string(),
    ));
  }

  validate_data_length(image_pixel_module, data)?;

  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let pixel_count = image_pixel_module.pixel_count();
  let bits_stored = image_pixel_module.bits_stored();

  match image_pixel_module.photometric_interpretation() {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2 => {
      match (
        image_pixel_module.pixel_representation(),
        image_pixel_module.bits_allocated(),
      ) {
        (_, BitsAllocated::One) => {
          let is_signed = image_pixel_module.pixel_representation().is_signed();
          let mut data = data.to_vec();

          if data_bit_offset > 0 {
            for i in 0..data.len() {
              let next_byte = data.get(i + 1).unwrap_or(&0);
              data[i] = (data[i] >> data_bit_offset)
                | (next_byte << (8 - data_bit_offset));
            }

            // It's possible there will be an unneeded trailing byte after
            // adjusting for the bit offset, so remove it if present
            data.resize_with(pixel_count.div_ceil(8), || 0);
          }

          SingleChannelImage::new_bitmap(width, height, data, is_signed)
        }

        (PixelRepresentation::Signed, BitsAllocated::Eight) => {
          let mut pixels = vec![0; pixel_count];

          if image_pixel_module.has_unused_high_bits() {
            let threshold = 2i8.pow(u32::from(bits_stored) - 1);

            for i in 0..pixel_count {
              let mut pixel = data[i] as i8;

              if pixel >= threshold {
                pixel -= threshold;
                pixel -= threshold;
              }

              pixels[i] = pixel;
            }
          } else {
            pixels.copy_from_slice(&bytemuck::cast_slice(data)[..pixel_count]);
          }

          SingleChannelImage::new_i8(width, height, pixels, bits_stored)
        }

        (PixelRepresentation::Unsigned, BitsAllocated::Eight) => {
          SingleChannelImage::new_u8(width, height, data.to_vec(), bits_stored)
        }

        (PixelRepresentation::Signed, BitsAllocated::Sixteen) => {
          let mut pixels = vec![0; pixel_count];

          if image_pixel_module.has_unused_high_bits() {
            let threshold = 2i16.pow(u32::from(bits_stored) - 1);

            for i in 0..pixel_count {
              let mut pixel =
                i16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);

              if pixel >= threshold {
                pixel -= threshold;
                pixel -= threshold;
              }

              pixels[i] = pixel;
            }
          } else {
            #[cfg(target_endian = "little")]
            unsafe {
              core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                pixels.as_mut_ptr() as *mut u8,
                pixels.len() * core::mem::size_of::<i16>(),
              );
            }

            #[cfg(target_endian = "big")]
            for i in 0..pixel_count {
              pixels[i] = i16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
            }
          }

          SingleChannelImage::new_i16(width, height, pixels, bits_stored)
        }

        (PixelRepresentation::Unsigned, BitsAllocated::Sixteen) => {
          let mut pixels = vec![0; pixel_count];

          #[cfg(target_endian = "little")]
          unsafe {
            core::ptr::copy_nonoverlapping(
              data.as_ptr(),
              pixels.as_mut_ptr() as *mut u8,
              pixels.len() * core::mem::size_of::<u16>(),
            );
          }

          #[cfg(target_endian = "big")]
          for i in 0..pixel_count {
            pixels[i] = u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
          }

          SingleChannelImage::new_u16(width, height, pixels, bits_stored)
        }

        (PixelRepresentation::Signed, BitsAllocated::ThirtyTwo) => {
          let mut pixels = vec![0; pixel_count];

          if image_pixel_module.has_unused_high_bits() {
            let threshold = 2i32.pow(u32::from(bits_stored) - 1);

            for i in 0..pixel_count {
              let mut pixel = i32::from_le_bytes([
                data[i * 4],
                data[i * 4 + 1],
                data[i * 4 + 2],
                data[i * 4 + 3],
              ]);

              if pixel >= threshold {
                pixel -= threshold;
                pixel -= threshold;
              }

              pixels[i] = pixel;
            }
          } else {
            #[cfg(target_endian = "little")]
            unsafe {
              core::ptr::copy_nonoverlapping(
                data.as_ptr(),
                pixels.as_mut_ptr() as *mut u8,
                pixels.len() * core::mem::size_of::<i32>(),
              );
            }

            #[cfg(target_endian = "big")]
            for i in 0..pixel_count {
              pixels[i] = i32::from_le_bytes([
                data[i * 4],
                data[i * 4 + 1],
                data[i * 4 + 2],
                data[i * 4 + 3],
              ]);
            }
          }

          SingleChannelImage::new_i32(width, height, pixels, bits_stored)
        }

        (PixelRepresentation::Unsigned, BitsAllocated::ThirtyTwo) => {
          let mut pixels = vec![0u32; pixel_count];

          #[cfg(target_endian = "little")]
          unsafe {
            core::ptr::copy_nonoverlapping(
              data.as_ptr(),
              pixels.as_mut_ptr() as *mut u8,
              pixels.len() * core::mem::size_of::<u32>(),
            );
          }

          #[cfg(target_endian = "big")]
          for i in 0..pixel_count {
            pixels[i] = u32::from_le_bytes([
              data[i * 4],
              data[i * 4 + 1],
              data[i * 4 + 2],
              data[i * 4 + 3],
            ]);
          }

          SingleChannelImage::new_u32(width, height, pixels, bits_stored)
        }
      }
    }

    _ => Err(DataError::new_value_invalid(format!(
      "Photometric interpretation '{}' is invalid for grayscale pixel data \
       when samples per pixel is one",
      image_pixel_module.photometric_interpretation()
    ))),
  }
}

/// Decodes native color pixel data that uses the
/// [`PhotometricInterpretation::Rgb`], [`PhotometricInterpretation::YbrFull`],
/// [`PhotometricInterpretation::YbrFull422`], or
/// [`PhotometricInterpretation::PaletteColor`] photometric interpretations.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  validate_data_length(image_pixel_module, data)?;

  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let pixel_count = image_pixel_module.pixel_count();
  let bits_stored = image_pixel_module.bits_stored();

  let color_space = if image_pixel_module.photometric_interpretation().is_ybr()
  {
    ColorSpace::YBR
  } else {
    ColorSpace::RGB
  };

  match image_pixel_module.samples_per_pixel() {
    SamplesPerPixel::One => match (
      &image_pixel_module.photometric_interpretation(),
      image_pixel_module.bits_allocated(),
    ) {
      (
        PhotometricInterpretation::PaletteColor { palette },
        BitsAllocated::Eight,
      ) => ColorImage::new_palette8(
        width,
        height,
        data.to_vec(),
        palette.clone(),
        bits_stored,
      ),

      (
        PhotometricInterpretation::PaletteColor { palette },
        BitsAllocated::Sixteen,
      ) => {
        let mut pixels = vec![0u16; pixel_count * 3];

        for i in 0..pixel_count {
          pixels.push(u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]));
        }

        ColorImage::new_palette16(
          width,
          height,
          pixels,
          palette.clone(),
          bits_stored,
        )
      }

      (photometric_interpretation, bits_allocated) => {
        Err(DataError::new_value_invalid(format!(
          "Photometric interpretation '{}' is invalid for color pixel data \
           when samples per pixel is one and bits allocated is '{}'",
          photometric_interpretation,
          u8::from(bits_allocated)
        )))
      }
    },

    SamplesPerPixel::Three {
      planar_configuration,
    } => match image_pixel_module.photometric_interpretation() {
      PhotometricInterpretation::Rgb | PhotometricInterpretation::YbrFull => {
        match (planar_configuration, image_pixel_module.bits_allocated()) {
          (_, BitsAllocated::One) => Err(DataError::new_value_invalid(
            "Bits allocated value '1' is not supported for color data"
              .to_string(),
          )),

          (PlanarConfiguration::Interleaved, BitsAllocated::Eight) => {
            let pixels = data[..(pixel_count * 3)].to_vec();
            ColorImage::new_u8(width, height, pixels, color_space, bits_stored)
          }

          (PlanarConfiguration::Interleaved, BitsAllocated::Sixteen) => {
            let mut pixels = vec![0u16; pixel_count * 3];

            for i in 0..(pixel_count * 3) {
              pixels[i] = u16::from_le_bytes([data[i * 2], data[i * 2] + 1]);
            }

            ColorImage::new_u16(width, height, pixels, color_space, bits_stored)
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

            ColorImage::new_u32(width, height, pixels, color_space, bits_stored)
          }

          (PlanarConfiguration::Separate, BitsAllocated::Eight) => {
            let mut pixels = vec![0u8; pixel_count * 3];

            for i in 0..pixel_count {
              pixels[i * 3] = data[i];
              pixels[i * 3 + 1] = data[pixel_count + i];
              pixels[i * 3 + 2] = data[pixel_count * 2 + i];
            }

            ColorImage::new_u8(width, height, pixels, color_space, bits_stored)
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

            ColorImage::new_u16(width, height, pixels, color_space, bits_stored)
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

            ColorImage::new_u32(width, height, pixels, color_space, bits_stored)
          }
        }
      }

      PhotometricInterpretation::YbrFull422 => {
        if image_pixel_module.columns() % 2 == 1 {
          return Err(DataError::new_value_invalid(
            "YBR_FULL_222 pixel data width is odd".to_string(),
          ));
        }

        match (planar_configuration, image_pixel_module.bits_allocated()) {
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

            ColorImage::new_u8(
              width,
              height,
              pixels,
              ColorSpace::YBR,
              bits_stored,
            )
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

            ColorImage::new_u16(
              width,
              height,
              pixels,
              ColorSpace::YBR,
              bits_stored,
            )
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

            ColorImage::new_u32(
              width,
              height,
              pixels,
              ColorSpace::YBR,
              bits_stored,
            )
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

            ColorImage::new_u8(
              width,
              height,
              pixels,
              ColorSpace::YBR,
              bits_stored,
            )
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

            ColorImage::new_u16(
              width,
              height,
              pixels,
              ColorSpace::YBR,
              bits_stored,
            )
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

            ColorImage::new_u32(
              width,
              height,
              pixels,
              ColorSpace::YBR,
              bits_stored,
            )
          }
        }
      }

      _ => Err(DataError::new_value_invalid(format!(
        "Photometric interpretation '{}' is invalid for color pixel data \
           when samples per pixel is three",
        image_pixel_module.photometric_interpretation()
      ))),
    },
  }
}

/// Validates the length of the supplied pixel data.
///
fn validate_data_length(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<(), DataError> {
  let expected_size_in_bits =
    image_pixel_module.pixel_count() * image_pixel_module.pixel_size_in_bits();

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

  use crate::iods::image_pixel_module::PixelRepresentation;

  #[test]
  fn decode_monochrome_8_bit_unsigned() {
    let image_pixel_module = ImagePixelModule::new_basic(
      SamplesPerPixel::One,
      PhotometricInterpretation::Monochrome2,
      2,
      2,
      BitsAllocated::Eight,
      8,
      PixelRepresentation::Unsigned,
    )
    .unwrap();

    let data = [0, 1, 2, 3];

    assert_eq!(
      decode_single_channel(&image_pixel_module, &data, 0),
      SingleChannelImage::new_u8(2, 2, vec![0, 1, 2, 3], 8)
    );
  }

  #[test]
  fn decode_monochrome_8_bit_signed_with_7_bits_stored() {
    let image_pixel_module = ImagePixelModule::new_basic(
      SamplesPerPixel::One,
      PhotometricInterpretation::Monochrome2,
      8,
      16,
      BitsAllocated::Eight,
      7,
      PixelRepresentation::Signed,
    )
    .unwrap();

    let data: Vec<_> = (0..=127).collect();

    assert_eq!(
      decode_single_channel(&image_pixel_module, &data, 0),
      SingleChannelImage::new_i8(16, 8, (0..64).chain(-64..0).collect(), 7)
    );
  }

  #[test]
  fn decode_monochrome_16_bit_signed_with_12_bits_stored() {
    let image_pixel_module = ImagePixelModule::new_basic(
      SamplesPerPixel::One,
      PhotometricInterpretation::Monochrome2,
      64,
      64,
      BitsAllocated::Sixteen,
      12,
      PixelRepresentation::Signed,
    )
    .unwrap();

    let data: Vec<_> = (0..4096u16).flat_map(|i| i.to_le_bytes()).collect();

    assert_eq!(
      decode_single_channel(&image_pixel_module, &data, 0),
      SingleChannelImage::new_i16(
        64,
        64,
        (0..2048).chain(-2048..0).collect(),
        12
      )
    );
  }

  #[test]
  fn decode_rgb_8_bit_interleaved() {
    let image_pixel_module = ImagePixelModule::new_basic(
      SamplesPerPixel::Three {
        planar_configuration: PlanarConfiguration::Interleaved,
      },
      PhotometricInterpretation::Rgb,
      2,
      2,
      BitsAllocated::Eight,
      8,
      PixelRepresentation::Unsigned,
    )
    .unwrap();

    let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

    assert_eq!(
      decode_color(&image_pixel_module, &data),
      ColorImage::new_u8(
        2,
        2,
        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        ColorSpace::RGB,
        8
      )
    );
  }

  #[test]
  fn decode_rgb_16_bit_separate() {
    let image_pixel_module = ImagePixelModule::new_basic(
      SamplesPerPixel::Three {
        planar_configuration: PlanarConfiguration::Separate,
      },
      PhotometricInterpretation::Rgb,
      2,
      2,
      BitsAllocated::Sixteen,
      16,
      PixelRepresentation::Unsigned,
    )
    .unwrap();

    let data = vec![
      0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8, 0, 9, 0, 10, 0, 11, 0,
    ];

    assert_eq!(
      decode_color(&image_pixel_module, &data),
      ColorImage::new_u16(
        2,
        2,
        vec![0, 4, 8, 1, 5, 9, 2, 6, 10, 3, 7, 11,],
        ColorSpace::RGB,
        16
      )
    );
  }

  #[test]
  fn decode_ybr_full_8_bit() {
    let image_pixel_module = ImagePixelModule::new_basic(
      SamplesPerPixel::Three {
        planar_configuration: PlanarConfiguration::Interleaved,
      },
      PhotometricInterpretation::YbrFull,
      2,
      2,
      BitsAllocated::Eight,
      8,
      PixelRepresentation::Unsigned,
    )
    .unwrap();

    let data = vec![142, 122, 111, 148, 118, 122, 101, 123, 127, 116, 133, 142];

    assert_eq!(
      decode_color(&image_pixel_module, &data),
      ColorImage::new_u8(2, 2, data, ColorSpace::YBR, 8)
    );
  }

  #[test]
  fn decode_ybr_full_422_8_bit() {
    let image_pixel_module = ImagePixelModule::new_basic(
      SamplesPerPixel::Three {
        planar_configuration: PlanarConfiguration::Interleaved,
      },
      PhotometricInterpretation::YbrFull422,
      2,
      2,
      BitsAllocated::Eight,
      8,
      PixelRepresentation::Unsigned,
    )
    .unwrap();

    let data = vec![142, 122, 111, 148, 118, 122, 101, 123];

    assert_eq!(
      decode_color(&image_pixel_module, &data),
      ColorImage::new_u8(
        2,
        2,
        vec![142, 111, 148, 122, 111, 148, 118, 101, 123, 122, 101, 123],
        ColorSpace::YBR,
        8
      )
    );
  }
}
