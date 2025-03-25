#[cfg(feature = "std")]
use std::rc::Rc;

#[cfg(not(feature = "std"))]
use alloc::{rc::Rc, vec::Vec};

use image::{ImageBuffer, Rgb, RgbImage};

use crate::{PixelDataDefinition, RgbLookupTables};

/// A color image that stores integer RGB color values for each pixel.
///
#[derive(Clone, Debug, PartialEq)]
pub struct ColorImage {
  width: u16,
  height: u16,
  data: ColorImageData,
}

#[derive(Clone, Debug, PartialEq)]
enum ColorImageData {
  U8(Vec<u8>),
  U16(Vec<u16>),
  U32(Vec<u32>),
  F32(Vec<f32>),
  PaletteU8(Vec<u8>, Rc<RgbLookupTables>),
  PaletteU16(Vec<u16>, Rc<RgbLookupTables>),
}

impl ColorImage {
  /// Creates a new color image with `u8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u8(width: u16, height: u16, data: Vec<u8>) -> Result<Self, ()> {
    if data.len() != width as usize * height as usize * 3 {
      return Err(());
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::U8(data),
    })
  }

  /// Creates a new color image with `u8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u16(width: u16, height: u16, data: Vec<u16>) -> Result<Self, ()> {
    if data.len() != width as usize * height as usize * 3 {
      return Err(());
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::U16(data),
    })
  }

  /// Creates a new color image with `u32` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u32(width: u16, height: u16, data: Vec<u32>) -> Result<Self, ()> {
    if data.len() != width as usize * height as usize * 3 {
      return Err(());
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::U32(data),
    })
  }

  /// Creates a new color image with `f32` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_f32(width: u16, height: u16, data: Vec<f32>) -> Result<Self, ()> {
    if data.len() != width as usize * height as usize * 3 {
      return Err(());
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::F32(data),
    })
  }

  /// Creates a new color palette image with `u8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_palette8(
    width: u16,
    height: u16,
    data: Vec<u8>,
    palette: Rc<RgbLookupTables>,
  ) -> Result<Self, ()> {
    if data.len() != width as usize * height as usize {
      return Err(());
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::PaletteU8(data, palette),
    })
  }

  /// Creates a new color palette image with `u16` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_palette16(
    width: u16,
    height: u16,
    data: Vec<u16>,
    palette: Rc<RgbLookupTables>,
  ) -> Result<Self, ()> {
    if data.len() != width as usize * height as usize {
      return Err(());
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::PaletteU16(data, palette),
    })
  }

  /// Returns whether this color image is empty, i.e. it has no pixels.
  ///
  pub fn is_empty(&self) -> bool {
    self.width == 0 || self.height == 0
  }

  /// Returns the width in pixels of this color image.
  ///
  pub fn width(&self) -> u16 {
    self.width
  }

  /// Returns the height in pixels of this color image.
  ///
  pub fn height(&self) -> u16 {
    self.height
  }

  /// Returns the total number of pixels in this color image.
  ///
  pub fn pixel_count(&self) -> usize {
    self.width as usize * self.height as usize
  }

  /// Converts this color image to an RGB8 image.
  ///
  pub fn to_rgb_u8_image(self, definition: &PixelDataDefinition) -> RgbImage {
    if definition.bits_stored == 8 {
      if let ColorImageData::U8(data) = self.data {
        return RgbImage::from_raw(self.width.into(), self.height.into(), data)
          .unwrap();
      }
    }

    let mut rgb_pixels = Vec::with_capacity(self.pixel_count() * 3);

    let scale = 255.0 / (((1u64 << definition.bits_stored as u64) - 1) as f32);

    match &self.data {
      ColorImageData::U8(data) => {
        for pixel in data.chunks_exact(3) {
          rgb_pixels.push((pixel[0] as f32 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel[1] as f32 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel[2] as f32 * scale).min(255.0) as u8);
        }
      }

      ColorImageData::U16(data) => {
        for pixel in data.chunks_exact(3) {
          rgb_pixels.push((pixel[0] as f32 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel[1] as f32 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel[2] as f32 * scale).min(255.0) as u8);
        }
      }

      ColorImageData::U32(data) => {
        for pixel in data.chunks_exact(3) {
          rgb_pixels.push((pixel[0] as f32 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel[1] as f32 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel[2] as f32 * scale).min(255.0) as u8);
        }
      }

      ColorImageData::F32(data) => {
        for pixel in data.chunks_exact(3) {
          rgb_pixels.push((pixel[0].clamp(0.0, 1.0) * 255.0) as u8);
          rgb_pixels.push((pixel[1].clamp(0.0, 1.0) * 255.0) as u8);
          rgb_pixels.push((pixel[2].clamp(0.0, 1.0) * 255.0) as u8);
        }
      }

      ColorImageData::PaletteU8(data, palette) => {
        for pixel in data {
          let [r, g, b] = palette.lookup_normalized(*pixel as i64);

          rgb_pixels.push((r * 255.0) as u8);
          rgb_pixels.push((g * 255.0) as u8);
          rgb_pixels.push((b * 255.0) as u8);
        }
      }

      ColorImageData::PaletteU16(data, palette) => {
        for pixel in data {
          let [r, g, b] = palette.lookup_normalized(*pixel as i64);

          rgb_pixels.push((r * 255.0) as u8);
          rgb_pixels.push((g * 255.0) as u8);
          rgb_pixels.push((b * 255.0) as u8);
        }
      }
    }

    RgbImage::from_raw(self.width.into(), self.height.into(), rgb_pixels)
      .unwrap()
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

    match &self.data {
      ColorImageData::U8(data) => {
        for pixel in data.chunks_exact(3) {
          rgb_pixels.push((pixel[0] as f64 * scale) as f32);
          rgb_pixels.push((pixel[1] as f64 * scale) as f32);
          rgb_pixels.push((pixel[2] as f64 * scale) as f32);
        }
      }

      ColorImageData::U16(data) => {
        for pixel in data.chunks_exact(3) {
          rgb_pixels.push((pixel[0] as f64 * scale) as f32);
          rgb_pixels.push((pixel[1] as f64 * scale) as f32);
          rgb_pixels.push((pixel[2] as f64 * scale) as f32);
        }
      }

      ColorImageData::U32(data) => {
        for pixel in data.chunks_exact(3) {
          rgb_pixels.push((pixel[0] as f64 * scale) as f32);
          rgb_pixels.push((pixel[1] as f64 * scale) as f32);
          rgb_pixels.push((pixel[2] as f64 * scale) as f32);
        }
      }

      ColorImageData::F32(data) => {
        for pixel in data.chunks_exact(3) {
          rgb_pixels.push(pixel[0].clamp(0.0, 1.0));
          rgb_pixels.push(pixel[1].clamp(0.0, 1.0));
          rgb_pixels.push(pixel[2].clamp(0.0, 1.0));
        }
      }

      ColorImageData::PaletteU8(data, palette) => {
        for pixel in data {
          rgb_pixels
            .extend_from_slice(&palette.lookup_normalized(*pixel as i64));
        }
      }

      ColorImageData::PaletteU16(data, palette) => {
        for pixel in data {
          rgb_pixels
            .extend_from_slice(&palette.lookup_normalized(*pixel as i64));
        }
      }
    }

    ImageBuffer::from_raw(self.width.into(), self.height.into(), rgb_pixels)
      .unwrap()
  }
}
