#[cfg(feature = "std")]
use std::rc::Rc;

#[cfg(not(feature = "std"))]
use alloc::{rc::Rc, string::ToString, vec::Vec};

use dcmfx_core::DataError;

use crate::{PixelDataDefinition, RgbLut};

/// A color image that stores RGB or YBR color values for each pixel.
///
#[derive(Clone, Debug, PartialEq)]
pub struct ColorImage {
  width: u16,
  height: u16,
  data: ColorImageData,
}

#[derive(Clone, Debug, PartialEq)]
enum ColorImageData {
  U8 {
    data: Vec<u8>,
    color_space: ColorSpace,
  },
  U16 {
    data: Vec<u16>,
    color_space: ColorSpace,
  },
  U32 {
    data: Vec<u32>,
    color_space: ColorSpace,
  },
  PaletteU8 {
    data: Vec<u8>,
    palette: Rc<RgbLut>,
  },
  PaletteU16 {
    data: Vec<u16>,
    palette: Rc<RgbLut>,
  },
}

/// The color space of the color image's data. This is used when unsigned
/// integer data is being stored,a s it can be either YBR or RGB.
///
#[derive(Clone, Debug, PartialEq)]
pub enum ColorSpace {
  RGB,
  YBR,
}

impl ColorImage {
  /// Creates a new color image with `u8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u8(
    width: u16,
    height: u16,
    data: Vec<u8>,
    color_space: ColorSpace,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) * 3 {
      return Err(DataError::new_value_invalid(
        "Color image u8 data size is incorrect".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::U8 { data, color_space },
    })
  }

  /// Creates a new color image with `u8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u16(
    width: u16,
    height: u16,
    data: Vec<u16>,
    color_space: ColorSpace,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) * 3 {
      return Err(DataError::new_value_invalid(
        "Color image u16 data size is incorrect".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::U16 { data, color_space },
    })
  }

  /// Creates a new color image with `u32` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u32(
    width: u16,
    height: u16,
    data: Vec<u32>,
    color_space: ColorSpace,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) * 3 {
      return Err(DataError::new_value_invalid(
        "Color image u32 data size is incorrect".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::U32 { data, color_space },
    })
  }

  /// Creates a new color palette image with `u8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_palette8(
    width: u16,
    height: u16,
    data: Vec<u8>,
    palette: Rc<RgbLut>,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Color image palette8 data size is incorrect".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::PaletteU8 { data, palette },
    })
  }

  /// Creates a new color palette image with `u16` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_palette16(
    width: u16,
    height: u16,
    data: Vec<u16>,
    palette: Rc<RgbLut>,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Color image palette16 data size is incorrect".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::PaletteU16 { data, palette },
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
    usize::from(self.width) * usize::from(self.height)
  }

  /// Converts this color image to an RGB8 image.
  ///
  pub fn to_rgb_u8_image(
    &self,
    definition: &PixelDataDefinition,
  ) -> image::RgbImage {
    let mut rgb_pixels = Vec::with_capacity(self.pixel_count() * 3);

    fn unsigned_data_to_rgb_pixels<T>(
      data: &[T],
      color_space: &ColorSpace,
      rgb_pixels: &mut Vec<u8>,
      definition: &PixelDataDefinition,
    ) where
      T: Copy + Into<f64> + Into<u64>,
    {
      match color_space {
        ColorSpace::RGB => {
          let max_value: u64 = definition.int_max().into();

          for rgb in data.chunks_exact(3) {
            let r: u64 = rgb[0].into();
            let g: u64 = rgb[1].into();
            let b: u64 = rgb[2].into();

            rgb_pixels.push((r * 255 / max_value).min(0xFF) as u8);
            rgb_pixels.push((g * 255 / max_value).min(0xFF) as u8);
            rgb_pixels.push((b * 255 / max_value).min(0xFF) as u8);
          }
        }

        ColorSpace::YBR => {
          let scale = 1.0 / f64::from(definition.int_max());

          for ybr in data.chunks_exact(3) {
            let y: f64 = ybr[0].into();
            let cb: f64 = ybr[1].into();
            let cr: f64 = ybr[2].into();

            let rgb = ybr_to_rgb(y * scale, cb * scale, cr * scale);

            rgb_pixels.push((rgb[0] * 255.0).clamp(0.0, 255.0) as u8);
            rgb_pixels.push((rgb[1] * 255.0).clamp(0.0, 255.0) as u8);
            rgb_pixels.push((rgb[2] * 255.0).clamp(0.0, 255.0) as u8);
          }
        }
      }
    }

    match &self.data {
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::RGB,
      } if definition.bits_stored() == 8 => {
        rgb_pixels.extend_from_slice(data);
      }

      ColorImageData::U8 { data, color_space } => unsigned_data_to_rgb_pixels(
        data,
        color_space,
        &mut rgb_pixels,
        definition,
      ),

      ColorImageData::U16 { data, color_space } => unsigned_data_to_rgb_pixels(
        data,
        color_space,
        &mut rgb_pixels,
        definition,
      ),

      ColorImageData::U32 { data, color_space } => unsigned_data_to_rgb_pixels(
        data,
        color_space,
        &mut rgb_pixels,
        definition,
      ),

      ColorImageData::PaletteU8 { data, palette } => {
        for pixel in data {
          let rgb = palette.lookup_normalized_u8((*pixel).into());
          rgb_pixels.extend_from_slice(&rgb);
        }
      }

      ColorImageData::PaletteU16 { data, palette } => {
        for pixel in data {
          let rgb = palette.lookup_normalized_u8((*pixel).into());
          rgb_pixels.extend_from_slice(&rgb);
        }
      }
    }

    image::RgbImage::from_raw(self.width.into(), self.height.into(), rgb_pixels)
      .unwrap()
  }

  /// Converts this color image to an RGB F64 image where each value is in the
  /// range 0-1.
  ///
  pub fn to_rgb_f64_image(
    &self,
    definition: &PixelDataDefinition,
  ) -> image::ImageBuffer<image::Rgb<f64>, Vec<f64>> {
    let mut rgb_pixels = Vec::with_capacity(self.pixel_count() * 3);

    fn unsigned_data_to_rgb_pixels<T>(
      data: &[T],
      color_space: &ColorSpace,
      rgb_pixels: &mut Vec<f64>,
      definition: &PixelDataDefinition,
    ) where
      T: Copy + Into<f64>,
    {
      let scale = 1.0 / f64::from(definition.int_max());

      match color_space {
        ColorSpace::RGB => {
          for rgb in data.chunks_exact(3) {
            rgb_pixels.push(rgb[0].into() * scale);
            rgb_pixels.push(rgb[1].into() * scale);
            rgb_pixels.push(rgb[2].into() * scale);
          }
        }

        ColorSpace::YBR => {
          for ybr in data.chunks_exact(3) {
            let rgb = ybr_to_rgb(
              ybr[0].into() * scale,
              ybr[1].into() * scale,
              ybr[2].into() * scale,
            );

            rgb_pixels.extend_from_slice(&rgb);
          }
        }
      }
    }

    match &self.data {
      ColorImageData::U8 { data, color_space } => unsigned_data_to_rgb_pixels(
        data,
        color_space,
        &mut rgb_pixels,
        definition,
      ),

      ColorImageData::U16 { data, color_space } => unsigned_data_to_rgb_pixels(
        data,
        color_space,
        &mut rgb_pixels,
        definition,
      ),

      ColorImageData::U32 { data, color_space } => unsigned_data_to_rgb_pixels(
        data,
        color_space,
        &mut rgb_pixels,
        definition,
      ),

      ColorImageData::PaletteU8 { data, palette } => {
        for pixel in data {
          let [r, g, b] = palette.lookup_normalized((*pixel).into());

          rgb_pixels.push(f64::from(r));
          rgb_pixels.push(f64::from(g));
          rgb_pixels.push(f64::from(b));
        }
      }

      ColorImageData::PaletteU16 { data, palette } => {
        for pixel in data {
          let [r, g, b] = palette.lookup_normalized((*pixel).into());

          rgb_pixels.push(f64::from(r));
          rgb_pixels.push(f64::from(g));
          rgb_pixels.push(f64::from(b));
        }
      }
    }

    image::ImageBuffer::from_raw(
      self.width.into(),
      self.height.into(),
      rgb_pixels,
    )
    .unwrap()
  }
}

/// Converts a YBR color into RGB.
///
pub fn ybr_to_rgb(y: f64, cb: f64, cr: f64) -> [f64; 3] {
  let r = y + 1.402 * (cr - 0.5);
  let g = y - 0.3441362862 * (cb - 0.5) - 0.7141362862 * (cr - 0.5);
  let b = y + 1.772 * (cb - 0.5);

  [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
}
