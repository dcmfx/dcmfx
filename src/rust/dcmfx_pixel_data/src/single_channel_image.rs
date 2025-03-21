#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use image::{GrayImage, ImageBuffer, Luma};

use crate::{
  ModalityLut, PhotometricInterpretation, PixelDataDefinition, VoiLut,
  VoiWindow,
};

/// A single channel image that stores an integer value for each pixel.
///
#[derive(Clone, Debug, PartialEq)]
pub enum SingleChannelImage {
  Int8(ImageBuffer<Luma<i8>, Vec<i8>>),
  Uint8(ImageBuffer<Luma<u8>, Vec<u8>>),
  Int16(ImageBuffer<Luma<i16>, Vec<i16>>),
  Uint16(ImageBuffer<Luma<u16>, Vec<u16>>),
  Int32(ImageBuffer<Luma<i32>, Vec<i32>>),
  Uint32(ImageBuffer<Luma<u32>, Vec<u32>>),
}

impl SingleChannelImage {
  /// Returns whether this single channel image is empty, i.e. it has no pixels.
  ///
  pub fn is_empty(&self) -> bool {
    match self {
      SingleChannelImage::Int8(data) => data.is_empty(),
      SingleChannelImage::Uint8(data) => data.is_empty(),
      SingleChannelImage::Int16(data) => data.is_empty(),
      SingleChannelImage::Uint16(data) => data.is_empty(),
      SingleChannelImage::Int32(data) => data.is_empty(),
      SingleChannelImage::Uint32(data) => data.is_empty(),
    }
  }

  /// Returns the width in pixels of this single channel image.
  ///
  pub fn width(&self) -> u32 {
    match self {
      SingleChannelImage::Int8(data) => data.width(),
      SingleChannelImage::Uint8(data) => data.width(),
      SingleChannelImage::Int16(data) => data.width(),
      SingleChannelImage::Uint16(data) => data.width(),
      SingleChannelImage::Int32(data) => data.width(),
      SingleChannelImage::Uint32(data) => data.width(),
    }
  }

  /// Returns the height in pixels of this single channel image.
  ///
  pub fn height(&self) -> u32 {
    match self {
      SingleChannelImage::Int8(data) => data.height(),
      SingleChannelImage::Uint8(data) => data.height(),
      SingleChannelImage::Int16(data) => data.height(),
      SingleChannelImage::Uint16(data) => data.height(),
      SingleChannelImage::Int32(data) => data.height(),
      SingleChannelImage::Uint32(data) => data.height(),
    }
  }

  /// Returns the total number of pixels in this single channel image.
  ///
  pub fn pixel_count(&self) -> usize {
    self.width() as usize * self.height() as usize
  }

  /// Returns the minimum and maximum values in this single channel image.
  ///
  pub fn min_max_values(&self) -> Option<(i64, i64)> {
    fn min_max<I: Iterator<Item = i64>>(iter: I) -> Option<(i64, i64)> {
      iter.fold(None, |acc, x| match acc {
        Some((min, max)) => Some((min.min(x), max.max(x))),
        None => Some((x, x)),
      })
    }

    match self {
      SingleChannelImage::Int8(data) => {
        min_max(data.pixels().map(|x| x.0[0] as i64))
      }
      SingleChannelImage::Uint8(data) => {
        min_max(data.pixels().map(|x| x.0[0] as i64))
      }
      SingleChannelImage::Int16(data) => {
        min_max(data.pixels().map(|x| x.0[0] as i64))
      }
      SingleChannelImage::Uint16(data) => {
        min_max(data.pixels().map(|x| x.0[0] as i64))
      }
      SingleChannelImage::Int32(data) => {
        min_max(data.pixels().map(|x| x.0[0] as i64))
      }
      SingleChannelImage::Uint32(data) => {
        min_max(data.pixels().map(|x| x.0[0] as i64))
      }
    }
  }

  /// Returns a VOI Window that covers the full range of values in this single
  /// channel image.
  ///
  pub fn fallback_voi_window(&self) -> Option<VoiWindow> {
    self.min_max_values().map(|(min, max)| {
      VoiWindow::new(
        (max + min) as f32 * 0.5,
        (max - min) as f32,
        "".into(),
        super::VoiLutFunction::LinearExact,
      )
    })
  }

  /// Converts Monochrome1 pixel data to Monochrome2.
  ///
  pub fn invert_monochrome1_data(&mut self, definition: &PixelDataDefinition) {
    if definition.photometric_interpretation
      != PhotometricInterpretation::Monochrome1
    {
      return;
    }

    // Calculate the offset to add after negating the stored pixel value in
    // order to convert to Monochrome2
    let offset = if definition.pixel_representation.is_signed() {
      -1
    } else {
      (1i64 << definition.bits_stored) - 1
    };

    match self {
      SingleChannelImage::Int8(data) => {
        for i in data.pixels_mut() {
          i.0[0] = (-(i.0[0] as i64) + offset)
            .clamp(i8::MIN as i64, i8::MAX as i64) as i8;
        }
      }

      SingleChannelImage::Uint8(data) => {
        for i in data.pixels_mut() {
          i.0[0] = (-(i.0[0] as i64) + offset)
            .clamp(u8::MIN as i64, u8::MAX as i64) as u8;
        }
      }

      SingleChannelImage::Int16(data) => {
        for i in data.pixels_mut() {
          i.0[0] = (-(i.0[0] as i64) + offset)
            .clamp(i16::MIN as i64, i16::MAX as i64) as i16;
        }
      }

      SingleChannelImage::Uint16(data) => {
        for i in data.pixels_mut() {
          i.0[0] = (-(i.0[0] as i64) + offset)
            .clamp(u16::MIN as i64, u16::MAX as i64) as u16;
        }
      }

      SingleChannelImage::Int32(data) => {
        for i in data.pixels_mut() {
          i.0[0] = (-(i.0[0] as i64) + offset)
            .clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        }
      }

      SingleChannelImage::Uint32(data) => {
        for i in data.pixels_mut() {
          i.0[0] = (-(i.0[0] as i64) + offset)
            .clamp(u32::MIN as i64, u32::MAX as i64) as u32;
        }
      }
    }
  }

  /// Converts this single channel image to a grayscale image by passing its
  /// values through the given Modality LUT and VOI LUT.
  ///
  pub fn to_gray_image(
    self,
    modality_lut: &ModalityLut,
    voi_lut: &VoiLut,
  ) -> GrayImage {
    let mut gray_pixels = Vec::with_capacity(self.pixel_count());

    let i64_to_u8 = |pixel: i64| {
      // Apply LUTs
      let x = modality_lut.apply(pixel);
      let x = voi_lut.apply(x);

      // Convert to u8
      (x * 255.0).clamp(0.0, 255.0) as u8
    };

    match &self {
      SingleChannelImage::Int8(data) => {
        for pixel in data.pixels() {
          gray_pixels.push(i64_to_u8(pixel.0[0] as i64));
        }
      }

      SingleChannelImage::Uint8(data) => {
        for pixel in data.pixels() {
          gray_pixels.push(i64_to_u8(pixel.0[0] as i64));
        }
      }

      SingleChannelImage::Int16(data) => {
        for pixel in data.pixels() {
          gray_pixels.push(i64_to_u8(pixel.0[0] as i64));
        }
      }

      SingleChannelImage::Uint16(data) => {
        for pixel in data.pixels() {
          gray_pixels.push(i64_to_u8(pixel.0[0] as i64));
        }
      }

      SingleChannelImage::Int32(data) => {
        for pixel in data.pixels() {
          gray_pixels.push(i64_to_u8(pixel.0[0] as i64));
        }
      }

      SingleChannelImage::Uint32(data) => {
        for pixel in data.pixels() {
          gray_pixels.push(i64_to_u8(pixel.0[0] as i64));
        }
      }
    }

    GrayImage::from_raw(self.width(), self.height(), gray_pixels).unwrap()
  }

  /// Converts this single channel image to a [`Vec<i64>`].
  ///
  pub fn to_i64_pixels(&self) -> Vec<i64> {
    match self {
      SingleChannelImage::Int8(data) => {
        data.pixels().map(|x| x.0[0] as i64).collect()
      }
      SingleChannelImage::Uint8(data) => {
        data.pixels().map(|x| x.0[0] as i64).collect()
      }
      SingleChannelImage::Int16(data) => {
        data.pixels().map(|x| x.0[0] as i64).collect()
      }
      SingleChannelImage::Uint16(data) => {
        data.pixels().map(|x| x.0[0] as i64).collect()
      }
      SingleChannelImage::Int32(data) => {
        data.pixels().map(|x| x.0[0] as i64).collect()
      }
      SingleChannelImage::Uint32(data) => {
        data.pixels().map(|x| x.0[0] as i64).collect()
      }
    }
  }
}
