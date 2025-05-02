#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use crate::{
  ColorImage, MonochromeImage, PixelDataEncodeError, PixelDataFrame,
  color_image::ColorImageData,
  iods::{
    ImagePixelModule,
    image_pixel_module::{PhotometricInterpretation, PlanarConfiguration},
  },
  monochrome_image::MonochromeImageData,
};

/// Encodes a [`MonochromeImage`] into native pixel data raw bytes.
///
pub fn encode_monochrome(image: &MonochromeImage) -> PixelDataFrame {
  let bit_size =
    image.pixel_count() as u64 * u64::from(u8::from(image.bits_allocated()));
  let mut result = vec![0u8; bit_size.div_ceil(8) as usize];

  match (image.data(), image.bits_stored()) {
    (MonochromeImageData::Bitmap { data, .. }, _) => {
      result.copy_from_slice(data)
    }

    (MonochromeImageData::I8(data), 8) => {
      result.copy_from_slice(bytemuck::cast_slice(data))
    }

    (MonochromeImageData::I8(data), _) => {
      let mask = (1 << image.bits_stored()) - 1;

      for (i, pixel) in data.iter().enumerate() {
        result[i] = (i16::from(*pixel) & mask) as u8;
      }
    }

    (MonochromeImageData::U8(data), _) => result.copy_from_slice(data),

    (MonochromeImageData::I16(data), 16) => {
      #[cfg(target_endian = "little")]
      unsafe {
        core::ptr::copy_nonoverlapping(
          data.as_ptr(),
          result.as_mut_ptr() as *mut i16,
          data.len(),
        );
      }

      #[cfg(target_endian = "big")]
      for pixel in data {
        result.copy_from_slice(&pixel.to_le_bytes());
      }
    }

    (MonochromeImageData::I16(data), _) => {
      let mask = (1 << image.bits_stored()) - 1;

      for (i, pixel) in data.iter().enumerate() {
        result[(i * 2)..(i * 2 + 2)]
          .copy_from_slice(&((i32::from(*pixel) & mask) as u16).to_le_bytes());
      }
    }

    (MonochromeImageData::U16(data), _) => {
      #[cfg(target_endian = "little")]
      unsafe {
        core::ptr::copy_nonoverlapping(
          data.as_ptr(),
          result.as_mut_ptr() as *mut u16,
          data.len(),
        );
      }

      #[cfg(target_endian = "big")]
      for pixel in data {
        result.copy_from_slice(&pixel.to_le_bytes());
      }
    }

    (MonochromeImageData::I32(data), 16) => {
      result.copy_from_slice(bytemuck::cast_slice(data))
    }

    (MonochromeImageData::I32(data), _) => {
      let mask = (1 << image.bits_stored()) - 1;

      for (i, pixel) in data.iter().enumerate() {
        result[(i * 4)..(i * 4 + 4)]
          .copy_from_slice(&((i64::from(*pixel) & mask) as u32).to_le_bytes());
      }
    }

    (MonochromeImageData::U32(data), _) => {
      #[cfg(target_endian = "little")]
      unsafe {
        core::ptr::copy_nonoverlapping(
          data.as_ptr(),
          result.as_mut_ptr() as *mut u32,
          data.len(),
        );
      }

      #[cfg(target_endian = "big")]
      for pixel in data {
        result.copy_from_slice(&pixel.to_le_bytes());
      }
    }
  }

  let mut frame = PixelDataFrame::new();
  frame.push_bits(result.into(), bit_size);

  frame
}

/// Encodes a [`ColorImage`] into native pixel data raw bytes.
///
pub fn encode_color(
  image: &ColorImage,
  image_pixel_module: &ImagePixelModule,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let mut result = vec![0; image_pixel_module.frame_size_in_bytes()];

  let photometric_interpretation =
    image_pixel_module.photometric_interpretation();
  let planar_configuration = image_pixel_module.planar_configuration();

  match photometric_interpretation {
    PhotometricInterpretation::PaletteColor { .. }
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull => {
      match (image.data(), planar_configuration) {
        (ColorImageData::U8 { data, .. }, PlanarConfiguration::Interleaved)
        | (ColorImageData::PaletteU8 { data, .. }, _) => {
          result.copy_from_slice(data)
        }

        (ColorImageData::U8 { data, .. }, PlanarConfiguration::Separate) => {
          for i in 0..image.pixel_count() {
            result[i] = data[i * 3];
            result[image.pixel_count() + i] = data[i * 3 + 1];
            result[image.pixel_count() * 2 + i] = data[i * 3 + 2];
          }
        }

        (
          ColorImageData::U16 { data, .. },
          PlanarConfiguration::Interleaved,
        )
        | (ColorImageData::PaletteU16 { data, .. }, _) => {
          #[cfg(target_endian = "little")]
          unsafe {
            core::ptr::copy_nonoverlapping(
              data.as_ptr(),
              result.as_mut_ptr() as *mut u16,
              data.len(),
            );
          }

          #[cfg(target_endian = "big")]
          for pixel in data {
            result.copy_from_slice(&pixel.to_le_bytes());
          }
        }

        (ColorImageData::U16 { data, .. }, PlanarConfiguration::Separate) => {
          let mut i0 = 0;
          let mut i1 = image.pixel_count() * 2;
          let mut i2 = image.pixel_count() * 4;

          for i in 0..image.pixel_count() {
            result[i0..(i0 + 2)].copy_from_slice(&data[i * 3].to_le_bytes());
            result[i1..(i1 + 2)]
              .copy_from_slice(&data[i * 3 + 1].to_le_bytes());
            result[i2..(i2 + 2)]
              .copy_from_slice(&data[i * 3 + 2].to_le_bytes());

            i0 += 2;
            i1 += 2;
            i2 += 2;
          }
        }

        (
          ColorImageData::U32 { data, .. },
          PlanarConfiguration::Interleaved,
        ) => {
          #[cfg(target_endian = "little")]
          unsafe {
            core::ptr::copy_nonoverlapping(
              data.as_ptr(),
              result.as_mut_ptr() as *mut u32,
              data.len(),
            );
          }

          #[cfg(target_endian = "big")]
          for pixel in data {
            result.copy_from_slice(&pixel.to_le_bytes());
          }
        }

        (ColorImageData::U32 { data, .. }, PlanarConfiguration::Separate) => {
          let mut i0 = 0;
          let mut i1 = image.pixel_count() * 4;
          let mut i2 = image.pixel_count() * 8;

          for i in 0..image.pixel_count() {
            result[i0..(i0 + 4)].copy_from_slice(&data[i * 3].to_le_bytes());
            result[i1..(i1 + 4)]
              .copy_from_slice(&data[i * 3 + 1].to_le_bytes());
            result[i2..(i2 + 4)]
              .copy_from_slice(&data[i * 3 + 2].to_le_bytes());

            i0 += 4;
            i1 += 4;
            i2 += 4;
          }
        }
      }
    }

    PhotometricInterpretation::YbrFull422 => {
      if image.width() % 2 == 1 {
        return Err(PixelDataEncodeError::NotSupported {
          details: format!(
            "The YBR_FULL_422 photometric interpretation requires width to be \
             even but it is {} pixels",
            image.width()
          ),
        });
      }

      match (image.data(), planar_configuration) {
        (ColorImageData::U8 { data, .. }, PlanarConfiguration::Interleaved) => {
          for (i, pixels) in data.chunks_exact(6).enumerate() {
            let y0 = pixels[0];
            let y1 = pixels[3];
            let cb =
              ((usize::from(pixels[1]) + usize::from(pixels[4])) / 2) as u8;
            let cr =
              ((usize::from(pixels[2]) + usize::from(pixels[5])) / 2) as u8;

            let i = i * 4;
            result[i] = y0;
            result[i + 1] = y1;
            result[i + 2] = cb;
            result[i + 3] = cr;
          }
        }

        (ColorImageData::U8 { data, .. }, PlanarConfiguration::Separate) => {
          for (i, pixels) in data.chunks_exact(6).enumerate() {
            let y0 = pixels[0];
            let y1 = pixels[3];
            let cb =
              ((usize::from(pixels[1]) + usize::from(pixels[4])) / 2) as u8;
            let cr =
              ((usize::from(pixels[2]) + usize::from(pixels[5])) / 2) as u8;

            let j = i * 2;
            result[j] = y0;
            result[j + 1] = y1;

            let j = image.pixel_count() + i;
            result[j] = cb;

            let j = j + image.pixel_count() / 2;
            result[j] = cr;
          }
        }

        (
          ColorImageData::U16 { data, .. },
          PlanarConfiguration::Interleaved,
        ) => {
          for (i, pixels) in data.chunks_exact(6).enumerate() {
            let y0 = pixels[0];
            let y1 = pixels[3];
            let cb =
              ((usize::from(pixels[1]) + usize::from(pixels[4])) / 2) as u16;
            let cr =
              ((usize::from(pixels[2]) + usize::from(pixels[5])) / 2) as u16;

            let i = i * 8;
            result[i..(i + 2)].copy_from_slice(&y0.to_le_bytes());
            result[(i + 2)..(i + 4)].copy_from_slice(&y1.to_le_bytes());
            result[(i + 4)..(i + 6)].copy_from_slice(&cb.to_le_bytes());
            result[(i + 6)..(i + 8)].copy_from_slice(&cr.to_le_bytes());
          }
        }

        (ColorImageData::U16 { data, .. }, PlanarConfiguration::Separate) => {
          for (i, pixels) in data.chunks_exact(6).enumerate() {
            let y0 = pixels[0];
            let y1 = pixels[3];
            let cb =
              ((usize::from(pixels[1]) + usize::from(pixels[4])) / 2) as u16;
            let cr =
              ((usize::from(pixels[2]) + usize::from(pixels[5])) / 2) as u16;

            let j = i * 4;
            result[j..(j + 2)].copy_from_slice(&y0.to_le_bytes());
            result[(j + 2)..(j + 4)].copy_from_slice(&y1.to_le_bytes());

            let j = (image.pixel_count() + i) * 2;
            result[j..(j + 2)].copy_from_slice(&cb.to_le_bytes());

            let j = j + image.pixel_count();
            result[j..(j + 2)].copy_from_slice(&cr.to_le_bytes());
          }
        }

        (
          ColorImageData::U32 { data, .. },
          PlanarConfiguration::Interleaved,
        ) => {
          for (i, pixels) in data.chunks_exact(6).enumerate() {
            let y0 = pixels[0];
            let y1 = pixels[3];
            let cb = ((u64::from(pixels[1]) + u64::from(pixels[4])) / 2) as u32;
            let cr = ((u64::from(pixels[2]) + u64::from(pixels[5])) / 2) as u32;

            let i = i * 16;
            result[i..(i + 4)].copy_from_slice(&y0.to_le_bytes());
            result[(i + 4)..(i + 8)].copy_from_slice(&y1.to_le_bytes());
            result[(i + 8)..(i + 12)].copy_from_slice(&cb.to_le_bytes());
            result[(i + 12)..(i + 16)].copy_from_slice(&cr.to_le_bytes());
          }
        }

        (ColorImageData::U32 { data, .. }, PlanarConfiguration::Separate) => {
          for (i, pixels) in data.chunks_exact(6).enumerate() {
            let y0 = pixels[0];
            let y1 = pixels[3];
            let cb = ((u64::from(pixels[1]) + u64::from(pixels[4])) / 2) as u32;
            let cr = ((u64::from(pixels[2]) + u64::from(pixels[5])) / 2) as u32;

            let j = i * 8;
            result[j..(j + 4)].copy_from_slice(&y0.to_le_bytes());
            result[(j + 4)..(j + 8)].copy_from_slice(&y1.to_le_bytes());

            let j = (image.pixel_count() + i) * 4;
            result[j..(j + 4)].copy_from_slice(&cb.to_le_bytes());

            let j = j + image.pixel_count() * 2;
            result[j..(j + 4)].copy_from_slice(&cr.to_le_bytes());
          }
        }

        (ColorImageData::PaletteU8 { .. }, _)
        | (ColorImageData::PaletteU16 { .. }, _) => {
          return Err(PixelDataEncodeError::NotSupported {
            details: "Palette color images can't be encoded to YBR 422"
              .to_string(),
          });
        }
      }
    }

    _ => {
      return Err(PixelDataEncodeError::NotSupported {
        details: format!(
          "Photometric interpretation '{}' is not able to be encoded into \
           native pixel data",
          photometric_interpretation
        ),
      });
    }
  }

  Ok(result)
}
