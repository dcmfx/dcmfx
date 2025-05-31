#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataEncodeError,
  PixelDataFrame,
  color_image::ColorImageData,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation, PlanarConfiguration,
  },
  monochrome_image::MonochromeImageData,
};

/// Returns the Image Pixel Module resulting from encoding into native pixel
/// data.
///
pub fn encode_image_pixel_module(
  image_pixel_module: ImagePixelModule,
) -> Result<ImagePixelModule, ()> {
  match image_pixel_module.photometric_interpretation() {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. }
    | PhotometricInterpretation::PaletteColor { .. }
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull422
    | PhotometricInterpretation::YbrFull => Ok(image_pixel_module),

    _ => Err(()),
  }
}

/// Encodes a [`MonochromeImage`] into native pixel data raw bytes.
///
pub fn encode_monochrome(
  image: &MonochromeImage,
  image_pixel_module: &ImagePixelModule,
) -> Result<PixelDataFrame, PixelDataEncodeError> {
  let bit_size =
    image.pixel_count() as u64 * u64::from(u8::from(image.bits_allocated()));
  let mut result = vec![0u8; bit_size.div_ceil(8) as usize];

  match (
    image.data(),
    image.is_monochrome1(),
    image.bits_stored(),
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      MonochromeImageData::Bitmap { data, .. },
      true,
      1,
      PhotometricInterpretation::Monochrome1 { .. },
      BitsAllocated::One,
    )
    | (
      MonochromeImageData::Bitmap { data, .. },
      false,
      1,
      PhotometricInterpretation::Monochrome2 { .. },
      BitsAllocated::One,
    ) => result.copy_from_slice(data),

    (
      MonochromeImageData::I8(data),
      true,
      8,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Eight,
    )
    | (
      MonochromeImageData::I8(data),
      false,
      8,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Eight,
    ) => result.copy_from_slice(bytemuck::cast_slice(data)),

    (
      MonochromeImageData::I8(data),
      true,
      _,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Eight,
    )
    | (
      MonochromeImageData::I8(data),
      false,
      _,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Eight,
    ) => {
      let mask = (1 << image.bits_stored()) - 1;

      for (i, pixel) in data.iter().enumerate() {
        result[i] = (i16::from(*pixel) & mask) as u8;
      }
    }

    (
      MonochromeImageData::U8(data),
      true,
      _,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Eight,
    )
    | (
      MonochromeImageData::U8(data),
      false,
      _,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Eight,
    ) => result.copy_from_slice(data),

    (
      MonochromeImageData::I16(data),
      true,
      16,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Sixteen,
    )
    | (
      MonochromeImageData::I16(data),
      false,
      16,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Sixteen,
    ) => {
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

    (
      MonochromeImageData::I16(data),
      true,
      _,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Sixteen,
    )
    | (
      MonochromeImageData::I16(data),
      false,
      _,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Sixteen,
    ) => {
      let mask = (1 << image.bits_stored()) - 1;

      for (i, pixel) in data.iter().enumerate() {
        result[(i * 2)..(i * 2 + 2)]
          .copy_from_slice(&((i32::from(*pixel) & mask) as u16).to_le_bytes());
      }
    }

    (
      MonochromeImageData::U16(data),
      true,
      _,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Sixteen,
    )
    | (
      MonochromeImageData::U16(data),
      false,
      _,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Sixteen,
    ) => {
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

    (
      MonochromeImageData::I32(data),
      true,
      16,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::ThirtyTwo,
    )
    | (
      MonochromeImageData::I32(data),
      false,
      16,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::ThirtyTwo,
    ) => result.copy_from_slice(bytemuck::cast_slice(data)),

    (
      MonochromeImageData::I32(data),
      true,
      _,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::ThirtyTwo,
    )
    | (
      MonochromeImageData::I32(data),
      false,
      _,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::ThirtyTwo,
    ) => {
      let mask = (1 << image.bits_stored()) - 1;

      for (i, pixel) in data.iter().enumerate() {
        result[(i * 4)..(i * 4 + 4)]
          .copy_from_slice(&((i64::from(*pixel) & mask) as u32).to_le_bytes());
      }
    }

    (
      MonochromeImageData::U32(data),
      true,
      _,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::ThirtyTwo,
    )
    | (
      MonochromeImageData::U32(data),
      false,
      _,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::ThirtyTwo,
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

    _ => {
      return Err(PixelDataEncodeError::NotSupported {
        image_pixel_module: Box::new(image_pixel_module.clone()),
        input_bits_allocated: image.bits_allocated(),
        input_color_space: None,
      });
    }
  }

  let mut frame = PixelDataFrame::new();
  frame.push_bits(result.into(), bit_size);

  Ok(frame)
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

  match (
    image.data(),
    planar_configuration,
    photometric_interpretation,
    image_pixel_module.bits_allocated(),
  ) {
    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PlanarConfiguration::Interleaved,
      PhotometricInterpretation::Rgb,
      BitsAllocated::Eight,
    )
    | (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PlanarConfiguration::Interleaved,
      PhotometricInterpretation::YbrFull,
      BitsAllocated::Eight,
    )
    | (
      ColorImageData::PaletteU8 { data, .. },
      _,
      PhotometricInterpretation::PaletteColor { .. },
      BitsAllocated::Eight,
    ) => result.copy_from_slice(data),

    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PlanarConfiguration::Separate,
      PhotometricInterpretation::Rgb,
      BitsAllocated::Eight,
    )
    | (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PlanarConfiguration::Separate,
      PhotometricInterpretation::YbrFull,
      BitsAllocated::Eight,
    ) => {
      for i in 0..image.pixel_count() {
        result[i] = data[i * 3];
        result[image.pixel_count() + i] = data[i * 3 + 1];
        result[image.pixel_count() * 2 + i] = data[i * 3 + 2];
      }
    }

    (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PlanarConfiguration::Interleaved,
      PhotometricInterpretation::Rgb,
      BitsAllocated::Sixteen,
    )
    | (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PlanarConfiguration::Interleaved,
      PhotometricInterpretation::YbrFull,
      BitsAllocated::Sixteen,
    )
    | (
      ColorImageData::PaletteU16 { data, .. },
      _,
      PhotometricInterpretation::PaletteColor { .. },
      BitsAllocated::Sixteen,
    ) => {
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

    (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PlanarConfiguration::Separate,
      PhotometricInterpretation::Rgb,
      BitsAllocated::Sixteen,
    )
    | (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PlanarConfiguration::Separate,
      PhotometricInterpretation::YbrFull,
      BitsAllocated::Sixteen,
    ) => {
      let mut i0 = 0;
      let mut i1 = image.pixel_count() * 2;
      let mut i2 = image.pixel_count() * 4;

      for i in 0..image.pixel_count() {
        result[i0..(i0 + 2)].copy_from_slice(&data[i * 3].to_le_bytes());
        result[i1..(i1 + 2)].copy_from_slice(&data[i * 3 + 1].to_le_bytes());
        result[i2..(i2 + 2)].copy_from_slice(&data[i * 3 + 2].to_le_bytes());

        i0 += 2;
        i1 += 2;
        i2 += 2;
      }
    }

    (
      ColorImageData::U32 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PlanarConfiguration::Interleaved,
      PhotometricInterpretation::Rgb,
      BitsAllocated::ThirtyTwo,
    )
    | (
      ColorImageData::U32 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PlanarConfiguration::Interleaved,
      PhotometricInterpretation::YbrFull,
      BitsAllocated::ThirtyTwo,
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

    (
      ColorImageData::U32 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PlanarConfiguration::Separate,
      PhotometricInterpretation::Rgb,
      BitsAllocated::ThirtyTwo,
    )
    | (
      ColorImageData::U32 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PlanarConfiguration::Separate,
      PhotometricInterpretation::YbrFull,
      BitsAllocated::ThirtyTwo,
    ) => {
      let mut i0 = 0;
      let mut i1 = image.pixel_count() * 4;
      let mut i2 = image.pixel_count() * 8;

      for i in 0..image.pixel_count() {
        result[i0..(i0 + 4)].copy_from_slice(&data[i * 3].to_le_bytes());
        result[i1..(i1 + 4)].copy_from_slice(&data[i * 3 + 1].to_le_bytes());
        result[i2..(i2 + 4)].copy_from_slice(&data[i * 3 + 2].to_le_bytes());

        i0 += 4;
        i1 += 4;
        i2 += 4;
      }
    }

    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Ybr { is_422: true },
      },
      PlanarConfiguration::Interleaved,
      PhotometricInterpretation::YbrFull422,
      BitsAllocated::Eight,
    ) => {
      for (i, pixels) in data.chunks_exact(6).enumerate() {
        let y0 = pixels[0];
        let y1 = pixels[3];
        let cb = ((usize::from(pixels[1]) + usize::from(pixels[4])) / 2) as u8;
        let cr = ((usize::from(pixels[2]) + usize::from(pixels[5])) / 2) as u8;

        let i = i * 4;
        result[i] = y0;
        result[i + 1] = y1;
        result[i + 2] = cb;
        result[i + 3] = cr;
      }
    }

    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Ybr { is_422: true },
      },
      PlanarConfiguration::Separate,
      PhotometricInterpretation::YbrFull422,
      BitsAllocated::Eight,
    ) => {
      for (i, pixels) in data.chunks_exact(6).enumerate() {
        let y0 = pixels[0];
        let y1 = pixels[3];
        let cb = ((usize::from(pixels[1]) + usize::from(pixels[4])) / 2) as u8;
        let cr = ((usize::from(pixels[2]) + usize::from(pixels[5])) / 2) as u8;

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
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Ybr { is_422: true },
      },
      PlanarConfiguration::Interleaved,
      PhotometricInterpretation::YbrFull422,
      BitsAllocated::Sixteen,
    ) => {
      for (i, pixels) in data.chunks_exact(6).enumerate() {
        let y0 = pixels[0];
        let y1 = pixels[3];
        let cb = ((usize::from(pixels[1]) + usize::from(pixels[4])) / 2) as u16;
        let cr = ((usize::from(pixels[2]) + usize::from(pixels[5])) / 2) as u16;

        let i = i * 8;
        result[i..(i + 2)].copy_from_slice(&y0.to_le_bytes());
        result[(i + 2)..(i + 4)].copy_from_slice(&y1.to_le_bytes());
        result[(i + 4)..(i + 6)].copy_from_slice(&cb.to_le_bytes());
        result[(i + 6)..(i + 8)].copy_from_slice(&cr.to_le_bytes());
      }
    }

    (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Ybr { is_422: true },
      },
      PlanarConfiguration::Separate,
      PhotometricInterpretation::YbrFull422,
      BitsAllocated::Sixteen,
    ) => {
      for (i, pixels) in data.chunks_exact(6).enumerate() {
        let y0 = pixels[0];
        let y1 = pixels[3];
        let cb = ((usize::from(pixels[1]) + usize::from(pixels[4])) / 2) as u16;
        let cr = ((usize::from(pixels[2]) + usize::from(pixels[5])) / 2) as u16;

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
      ColorImageData::U32 {
        data,
        color_space: ColorSpace::Ybr { is_422: true },
      },
      PlanarConfiguration::Interleaved,
      PhotometricInterpretation::YbrFull422,
      BitsAllocated::ThirtyTwo,
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

    (
      ColorImageData::U32 {
        data,
        color_space: ColorSpace::Ybr { is_422: true },
      },
      PlanarConfiguration::Separate,
      PhotometricInterpretation::YbrFull422,
      BitsAllocated::ThirtyTwo,
    ) => {
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

    _ => {
      return Err(PixelDataEncodeError::NotSupported {
        image_pixel_module: Box::new(image_pixel_module.clone()),
        input_bits_allocated: image.bits_allocated(),
        input_color_space: Some(image.color_space()),
      });
    }
  }

  Ok(result)
}
