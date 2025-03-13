#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use image::{ImageBuffer, Rgb, RgbImage};

use crate::{PhotometricInterpretation, PixelDataDefinition};

/// A color image that stores integer RGB color values for each pixel.
///
#[derive(Clone, Debug, PartialEq)]
pub enum ColorImage {
  Uint8(ImageBuffer<Rgb<u8>, Vec<u8>>),
  Uint16(ImageBuffer<Rgb<u16>, Vec<u16>>),
  Uint32(ImageBuffer<Rgb<u32>, Vec<u32>>),
}

impl ColorImage {
  /// Returns whether this color image is empty, i.e. it has no pixels.
  ///
  pub fn is_empty(&self) -> bool {
    match self {
      ColorImage::Uint8(data) => data.is_empty(),
      ColorImage::Uint16(data) => data.is_empty(),
      ColorImage::Uint32(data) => data.is_empty(),
    }
  }

  /// Returns the width in pixels of this color image.
  ///
  pub fn width(&self) -> u32 {
    match self {
      ColorImage::Uint8(data) => data.width(),
      ColorImage::Uint16(data) => data.width(),
      ColorImage::Uint32(data) => data.width(),
    }
  }

  /// Returns the height in pixels of this color image.
  ///
  pub fn height(&self) -> u32 {
    match self {
      ColorImage::Uint8(data) => data.height(),
      ColorImage::Uint16(data) => data.height(),
      ColorImage::Uint32(data) => data.height(),
    }
  }

  /// Returns the total number of pixels in this color image.
  ///
  pub fn pixel_count(&self) -> usize {
    self.width() as usize * self.height() as usize
  }

  /// Returns the minimum and maximum values for each channel in this color
  /// image.
  ///
  #[allow(clippy::type_complexity)]
  pub fn min_max_values(&self) -> Option<((u64, u64), (u64, u64), (u64, u64))> {
    if self.is_empty() {
      return None;
    }

    fn min_max<I: Iterator<Item = u64>>(iter: I) -> (u64, u64) {
      iter.fold((u64::MAX, 0), |acc: (u64, u64), x| {
        (acc.0.min(x), acc.1.max(x))
      })
    }

    match self {
      ColorImage::Uint8(data) => Some((
        min_max(data.pixels().map(|x| x.0[0] as u64)),
        min_max(data.pixels().map(|x| x.0[1] as u64)),
        min_max(data.pixels().map(|x| x.0[2] as u64)),
      )),
      ColorImage::Uint16(data) => Some((
        min_max(data.pixels().map(|x| x.0[0] as u64)),
        min_max(data.pixels().map(|x| x.0[1] as u64)),
        min_max(data.pixels().map(|x| x.0[2] as u64)),
      )),
      ColorImage::Uint32(data) => Some((
        min_max(data.pixels().map(|x| x.0[0] as u64)),
        min_max(data.pixels().map(|x| x.0[1] as u64)),
        min_max(data.pixels().map(|x| x.0[2] as u64)),
      )),
    }
  }

  /// Converts this color image to an RGB8 image.
  ///
  pub fn to_rgb_u8_image(self, definition: &PixelDataDefinition) -> RgbImage {
    if definition.bits_stored == 8 {
      if let ColorImage::Uint8(img) = self {
        return img;
      }
    }

    let mut rgb_pixels = Vec::with_capacity(self.pixel_count() * 3);

    let scale = 255.0 / (((1u64 << definition.bits_stored as u64) - 1) as f64);

    match &self {
      ColorImage::Uint8(data) => {
        for pixel in data.pixels() {
          rgb_pixels.push((pixel.0[0] as f64 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel.0[1] as f64 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel.0[2] as f64 * scale).min(255.0) as u8);
        }
      }

      ColorImage::Uint16(data) => {
        if let PhotometricInterpretation::PaletteColor { rgb_luts } =
          &definition.photometric_interpretation
        {
          let (red_lut, green_lut, blue_lut) = rgb_luts;

          for pixel in data.pixels() {
            let r = pixel.0[0] as f32 * red_lut.normalization_scale * 255.0;
            let g = pixel.0[1] as f32 * green_lut.normalization_scale * 255.0;
            let b = pixel.0[2] as f32 * blue_lut.normalization_scale * 255.0;

            rgb_pixels.push(r.min(255.0) as u8);
            rgb_pixels.push(g.min(255.0) as u8);
            rgb_pixels.push(b.min(255.0) as u8);
          }
        } else {
          for pixel in data.pixels() {
            rgb_pixels.push((pixel.0[0] as f64 * scale).min(255.0) as u8);
            rgb_pixels.push((pixel.0[1] as f64 * scale).min(255.0) as u8);
            rgb_pixels.push((pixel.0[2] as f64 * scale).min(255.0) as u8);
          }
        }
      }

      ColorImage::Uint32(data) => {
        for pixel in data.pixels() {
          rgb_pixels.push((pixel.0[0] as f64 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel.0[1] as f64 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel.0[2] as f64 * scale).min(255.0) as u8);
        }
      }
    }

    RgbImage::from_raw(self.width(), self.height(), rgb_pixels).unwrap()
  }

  /// Converts this color image to an RGB F32 image where each value is in the
  /// range 0-1.
  ///
  pub fn to_rgb_f32_image(
    self,
    definition: &PixelDataDefinition,
  ) -> ImageBuffer<Rgb<f32>, Vec<f32>> {
    let mut rgb_pixels = Vec::with_capacity(self.pixel_count() * 3);

    let scale = 1.0 / (((1u64 << definition.bits_stored as u64) - 1) as f64);

    match &self {
      ColorImage::Uint8(data) => {
        for pixel in data.pixels() {
          rgb_pixels.push((pixel.0[0] as f64 * scale) as f32);
          rgb_pixels.push((pixel.0[1] as f64 * scale) as f32);
          rgb_pixels.push((pixel.0[2] as f64 * scale) as f32);
        }
      }

      ColorImage::Uint16(data) => {
        if let PhotometricInterpretation::PaletteColor { rgb_luts } =
          &definition.photometric_interpretation
        {
          let (red_lut, green_lut, blue_lut) = rgb_luts;

          for pixel in data.pixels() {
            rgb_pixels.push(pixel.0[0] as f32 * red_lut.normalization_scale);
            rgb_pixels.push(pixel.0[1] as f32 * green_lut.normalization_scale);
            rgb_pixels.push(pixel.0[2] as f32 * blue_lut.normalization_scale);
          }
        } else {
          for pixel in data.pixels() {
            rgb_pixels.push((pixel.0[0] as f64 * scale) as f32);
            rgb_pixels.push((pixel.0[1] as f64 * scale) as f32);
            rgb_pixels.push((pixel.0[2] as f64 * scale) as f32);
          }
        }
      }

      ColorImage::Uint32(data) => {
        for pixel in data.pixels() {
          rgb_pixels.push((pixel.0[0] as f64 * scale) as f32);
          rgb_pixels.push((pixel.0[1] as f64 * scale) as f32);
          rgb_pixels.push((pixel.0[2] as f64 * scale) as f32);
        }
      }
    }

    ImageBuffer::from_raw(self.width(), self.height(), rgb_pixels).unwrap()
  }
}
