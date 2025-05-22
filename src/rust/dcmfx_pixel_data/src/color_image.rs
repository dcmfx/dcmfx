#[cfg(feature = "std")]
use std::rc::Rc;

#[cfg(not(feature = "std"))]
use alloc::{rc::Rc, string::ToString, vec::Vec};

use dcmfx_core::DataError;

use crate::{
  iods::{PaletteColorLookupTableModule, image_pixel_module::BitsAllocated},
  utils::udiv_round,
};

/// A color image that stores an RGB, YBR, or palette color for each pixel.
///
#[derive(Clone, Debug, PartialEq)]
pub struct ColorImage {
  width: u16,
  height: u16,
  data: ColorImageData,
  bits_stored: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColorImageData {
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
    palette: Rc<PaletteColorLookupTableModule>,
  },
  PaletteU16 {
    data: Vec<u16>,
    palette: Rc<PaletteColorLookupTableModule>,
  },
}

/// The color space of the color image's data. This is used when unsigned
/// integer data is being stored, as it can be either RGB or YBR.
///
/// Ref: PS3.5 C.7.6.3.1.2.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorSpace {
  /// Pixel data represent a color image described by red, green, and blue image
  /// planes. The minimum sample value for each color plane represents minimum
  /// intensity of the color.
  Rgb,

  /// Pixel data represent a color image described by one luminance (Y) and two
  /// chrominance planes (CB and CR).
  ///
  /// Black is represented by Y equal to zero. The absence of color is
  /// represented by both CB and CR values equal to half full scale.
  ///
  /// If [`ColorSpace::Ybr::is_422`] is true then it indicates that the source
  /// of the CB and CR data was down-sampled as 4:2:2, i.e. half-resolution
  /// horizontally. Encoders may elect to take advantage of this hint in order
  /// to themselves output YBR 4:2:2 data instead of full-resolution
  /// chrominance.
  Ybr { is_422: bool },
}

impl ColorSpace {
  /// Returns whether this color space is RGB.
  ///
  pub fn is_rgb(&self) -> bool {
    matches!(self, Self::Rgb)
  }

  /// Returns whether this color space is YBR.
  ///
  pub fn is_ybr(&self) -> bool {
    matches!(self, Self::Ybr { .. })
  }
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
    bits_stored: u16,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) * 3 {
      return Err(DataError::new_value_invalid(
        "Color image u8 data size is incorrect".to_string(),
      ));
    }

    if bits_stored == 0 || bits_stored > 8 {
      return Err(DataError::new_value_invalid(
        "Color image u8 bits stored must be <= 8".to_string(),
      ));
    }

    if (color_space == ColorSpace::Ybr { is_422: true }) && width % 2 == 1 {
      return Err(DataError::new_value_invalid(
        "Color image in the YBR 422 color space must have even width"
          .to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::U8 { data, color_space },
      bits_stored,
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
    bits_stored: u16,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) * 3 {
      return Err(DataError::new_value_invalid(
        "Color image u16 data size is incorrect".to_string(),
      ));
    }

    if bits_stored == 0 || bits_stored > 16 {
      return Err(DataError::new_value_invalid(
        "Color image u8 bits stored must be <= 16".to_string(),
      ));
    }

    if (color_space == ColorSpace::Ybr { is_422: true }) && width % 2 == 1 {
      return Err(DataError::new_value_invalid(
        "Color image in the YBR 422 color space must have even width"
          .to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::U16 { data, color_space },
      bits_stored,
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
    bits_stored: u16,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) * 3 {
      return Err(DataError::new_value_invalid(
        "Color image u32 data size is incorrect".to_string(),
      ));
    }

    if bits_stored == 0 || bits_stored > 32 {
      return Err(DataError::new_value_invalid(
        "Color image u8 bits stored must be <= 32".to_string(),
      ));
    }

    if (color_space == ColorSpace::Ybr { is_422: true }) && width % 2 == 1 {
      return Err(DataError::new_value_invalid(
        "Color image in the YBR 422 color space must have even width"
          .to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::U32 { data, color_space },
      bits_stored,
    })
  }

  /// Creates a new color palette image with `u8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_palette8(
    width: u16,
    height: u16,
    data: Vec<u8>,
    palette: Rc<PaletteColorLookupTableModule>,
    bits_stored: u16,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Color image palette8 data size is incorrect".to_string(),
      ));
    }

    if bits_stored == 0 || bits_stored > 8 {
      return Err(DataError::new_value_invalid(
        "Color image palette8 bits stored must be <= 8".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::PaletteU8 { data, palette },
      bits_stored,
    })
  }

  /// Creates a new color palette image with `u16` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_palette16(
    width: u16,
    height: u16,
    data: Vec<u16>,
    palette: Rc<PaletteColorLookupTableModule>,
    bits_stored: u16,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Color image palette16 data size is incorrect".to_string(),
      ));
    }

    if bits_stored == 0 || bits_stored > 16 {
      return Err(DataError::new_value_invalid(
        "Color image palette8 bits stored must be <= 16".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: ColorImageData::PaletteU16 { data, palette },
      bits_stored,
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

  /// Returns the internal data of this color image.
  ///
  pub fn data(&self) -> &ColorImageData {
    &self.data
  }

  /// Returns the total number of pixels in this color image.
  ///
  pub fn pixel_count(&self) -> usize {
    usize::from(self.width) * usize::from(self.height)
  }

  /// Returns the number of samples stored per pixel. This is three except for
  /// palette colors, where there is one sample per pixel.
  ///
  pub fn samples_per_pixel(&self) -> u8 {
    match self.data {
      ColorImageData::U8 { .. }
      | ColorImageData::U16 { .. }
      | ColorImageData::U32 { .. } => 3,

      ColorImageData::PaletteU8 { .. } | ColorImageData::PaletteU16 { .. } => 1,
    }
  }

  /// Returns whether this color image stores palette color data. Palette color
  /// data can be converted to plain RGB by calling
  /// [`Self::convert_palette_color_to_rgb()`].
  ///
  pub fn is_palette_color(&self) -> bool {
    matches!(
      self.data,
      ColorImageData::PaletteU8 { .. } | ColorImageData::PaletteU16 { .. }
    )
  }

  /// Returns the number of bits allocated for each sample.
  ///
  pub fn bits_allocated(&self) -> BitsAllocated {
    match self.data {
      ColorImageData::U8 { .. } | ColorImageData::PaletteU8 { .. } => {
        BitsAllocated::Eight
      }

      ColorImageData::U16 { .. } | ColorImageData::PaletteU16 { .. } => {
        BitsAllocated::Sixteen
      }

      ColorImageData::U32 { .. } => BitsAllocated::ThirtyTwo,
    }
  }

  /// Returns the number of bits stored for each sample. This will never exceed
  /// the number of bits allocated.
  ///
  pub fn bits_stored(&self) -> u16 {
    self.bits_stored
  }

  /// Returns the color space used by the data stored in this color image.
  ///
  pub fn color_space(&self) -> ColorSpace {
    match &self.data {
      ColorImageData::U8 { color_space, .. }
      | ColorImageData::U16 { color_space, .. }
      | ColorImageData::U32 { color_space, .. } => *color_space,

      ColorImageData::PaletteU8 { .. } | ColorImageData::PaletteU16 { .. } => {
        ColorSpace::Rgb
      }
    }
  }

  /// Returns the maximum value that can be stored, based on the number of bits
  /// stored.
  ///
  fn max_storable_value(&self) -> u32 {
    ((1u64 << self.bits_stored) - 1) as u32
  }

  /// Converts this color image into the RGB color space if it's in the YBR
  /// color space.
  ///
  pub fn convert_to_rgb_color_space(&mut self) {
    let max_storable_value = f64::from(self.max_storable_value());
    let scale = 1.0 / max_storable_value;

    match &mut self.data {
      ColorImageData::U8 { data, color_space } if color_space.is_ybr() => {
        for pixel in data.chunks_exact_mut(3) {
          let y: f64 = pixel[0].into();
          let cb: f64 = pixel[1].into();
          let cr: f64 = pixel[2].into();

          let rgb = ybr_to_rgb(y * scale, cb * scale, cr * scale);

          pixel[0] = (rgb[0] * max_storable_value).round() as u8;
          pixel[1] = (rgb[1] * max_storable_value).round() as u8;
          pixel[2] = (rgb[2] * max_storable_value).round() as u8;
        }

        *color_space = ColorSpace::Rgb;
      }

      ColorImageData::U16 { data, color_space } if color_space.is_ybr() => {
        for pixel in data.chunks_exact_mut(3) {
          let y: f64 = pixel[0].into();
          let cb: f64 = pixel[1].into();
          let cr: f64 = pixel[2].into();

          let rgb = ybr_to_rgb(y * scale, cb * scale, cr * scale);

          pixel[0] = (rgb[0] * max_storable_value).round() as u16;
          pixel[1] = (rgb[1] * max_storable_value).round() as u16;
          pixel[2] = (rgb[2] * max_storable_value).round() as u16;
        }

        *color_space = ColorSpace::Rgb;
      }

      ColorImageData::U32 { data, color_space } if color_space.is_ybr() => {
        for pixel in data.chunks_exact_mut(3) {
          let y: f64 = pixel[0].into();
          let cb: f64 = pixel[1].into();
          let cr: f64 = pixel[2].into();

          let rgb = ybr_to_rgb(y * scale, cb * scale, cr * scale);

          pixel[0] = (rgb[0] * max_storable_value).round() as u32;
          pixel[1] = (rgb[1] * max_storable_value).round() as u32;
          pixel[2] = (rgb[2] * max_storable_value).round() as u32;
        }

        *color_space = ColorSpace::Rgb;
      }

      _ => (),
    }
  }

  /// Converts this color image to an image in the RGB color space if it's using
  /// a color palette. The sampled data will be either 8-bit or 16-bit depending
  /// on the bits per entry of the underlying lookup tables.
  ///
  pub fn convert_palette_color_to_rgb(&mut self) {
    fn sample_color_palette<T>(
      data: &[T],
      palette: &PaletteColorLookupTableModule,
    ) -> ColorImageData
    where
      T: Copy,
      i64: From<T>,
    {
      if palette.bits_per_entry() <= 8 {
        let mut rgb_data = Vec::with_capacity(data.len() * 3);

        for index in data {
          let pixel = palette.lookup(i64::from(*index));
          rgb_data.push(pixel[0] as u8);
          rgb_data.push(pixel[1] as u8);
          rgb_data.push(pixel[2] as u8);
        }

        ColorImageData::U8 {
          data: rgb_data,
          color_space: ColorSpace::Rgb,
        }
      } else {
        let mut rgb_data = Vec::with_capacity(data.len() * 3);

        for index in data {
          let pixel = palette.lookup(i64::from(*index));
          rgb_data.extend_from_slice(&pixel);
        }

        ColorImageData::U16 {
          data: rgb_data,
          color_space: ColorSpace::Rgb,
        }
      }
    }

    match &self.data {
      ColorImageData::PaletteU8 { data, palette } => {
        self.bits_stored = palette.bits_per_entry();
        self.data = sample_color_palette(data, palette);
      }

      ColorImageData::PaletteU16 { data, palette } => {
        self.bits_stored = palette.bits_per_entry();
        self.data = sample_color_palette(data, palette);
      }

      _ => (),
    }
  }

  /// Converts this color image into the YBR color space if it's in the RGB
  /// color space.
  ///
  pub fn convert_to_ybr_color_space(&mut self) {
    let max_storable_value = f64::from(self.max_storable_value());
    let scale = 1.0 / max_storable_value;

    match &mut self.data {
      ColorImageData::U8 { data, color_space } if color_space.is_rgb() => {
        for pixel in data.chunks_exact_mut(3) {
          let r: f64 = pixel[0].into();
          let g: f64 = pixel[1].into();
          let b: f64 = pixel[2].into();

          let ybr = rgb_to_ybr(r * scale, g * scale, b * scale);

          pixel[0] = (ybr[0] * max_storable_value).round() as u8;
          pixel[1] = (ybr[1] * max_storable_value).round() as u8;
          pixel[2] = (ybr[2] * max_storable_value).round() as u8;
        }

        *color_space = ColorSpace::Ybr { is_422: false };
      }

      ColorImageData::U8 { color_space, .. } if color_space.is_ybr() => {
        *color_space = ColorSpace::Ybr { is_422: false };
      }

      ColorImageData::U16 { data, color_space } if color_space.is_rgb() => {
        for pixel in data.chunks_exact_mut(3) {
          let r: f64 = pixel[0].into();
          let g: f64 = pixel[1].into();
          let b: f64 = pixel[2].into();

          let ybr = rgb_to_ybr(r * scale, g * scale, b * scale);

          pixel[0] = (ybr[0] * max_storable_value).round() as u16;
          pixel[1] = (ybr[1] * max_storable_value).round() as u16;
          pixel[2] = (ybr[2] * max_storable_value).round() as u16;
        }

        *color_space = ColorSpace::Ybr { is_422: false };
      }

      ColorImageData::U16 { color_space, .. } if color_space.is_ybr() => {
        *color_space = ColorSpace::Ybr { is_422: false };
      }

      ColorImageData::U32 { data, color_space } if color_space.is_rgb() => {
        for pixel in data.chunks_exact_mut(3) {
          let r: f64 = pixel[0].into();
          let g: f64 = pixel[1].into();
          let b: f64 = pixel[2].into();

          let ybr = rgb_to_ybr(r * scale, g * scale, b * scale);

          pixel[0] = (ybr[0] * max_storable_value).round() as u32;
          pixel[1] = (ybr[1] * max_storable_value).round() as u32;
          pixel[2] = (ybr[2] * max_storable_value).round() as u32;
        }

        *color_space = ColorSpace::Ybr { is_422: false };
      }

      ColorImageData::U32 { color_space, .. } if color_space.is_ybr() => {
        *color_space = ColorSpace::Ybr { is_422: false };
      }

      _ => (),
    }
  }

  /// Converts this color image into the YBR 422 color space if it's in the RGB
  /// or YBR 444 color space.
  ///
  /// An error is returned if the width of this color image is odd, as YBR 422
  /// requires even width.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn convert_to_ybr_422_color_space(&mut self) -> Result<(), ()> {
    if self.width() % 2 == 1 {
      return Err(());
    }

    self.convert_to_ybr_color_space();

    // Ensure YBR 422 data has identical Cb and Cr values
    match &mut self.data {
      ColorImageData::U8 { data, color_space } => {
        for pixels in data.chunks_exact_mut(6) {
          let cb = (u64::from(pixels[1]) + u64::from(pixels[4])).div_ceil(2);
          let cr = (u64::from(pixels[2]) + u64::from(pixels[5])).div_ceil(2);

          pixels[1] = cb as u8;
          pixels[2] = cr as u8;
          pixels[4] = cb as u8;
          pixels[5] = cr as u8;
        }

        *color_space = ColorSpace::Ybr { is_422: true };
      }

      ColorImageData::U16 { data, color_space } => {
        for pixels in data.chunks_exact_mut(6) {
          let cb = (u64::from(pixels[1]) + u64::from(pixels[4])).div_ceil(2);
          let cr = (u64::from(pixels[2]) + u64::from(pixels[5])).div_ceil(2);

          pixels[1] = cb as u16;
          pixels[2] = cr as u16;
          pixels[4] = cb as u16;
          pixels[5] = cr as u16;
        }

        *color_space = ColorSpace::Ybr { is_422: true };
      }

      ColorImageData::U32 { data, color_space } => {
        for pixels in data.chunks_exact_mut(6) {
          let cb = (u64::from(pixels[1]) + u64::from(pixels[4])).div_ceil(2);
          let cr = (u64::from(pixels[2]) + u64::from(pixels[5])).div_ceil(2);

          pixels[1] = cb as u32;
          pixels[2] = cr as u32;
          pixels[4] = cb as u32;
          pixels[5] = cr as u32;
        }

        *color_space = ColorSpace::Ybr { is_422: true };
      }

      _ => (),
    }

    Ok(())
  }

  /// Converts this color image to an 8-bit RGB image.
  ///
  pub fn into_rgb_u8_image(
    self,
  ) -> image::ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    let max_storable_value = self.max_storable_value();

    match self.data {
      // If this color image is already in RGB8 then return it directly,
      // avoiding a copy
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      } if self.bits_stored == 8 => image::ImageBuffer::from_raw(
        self.width.into(),
        self.height.into(),
        data,
      )
      .unwrap(),

      _ => {
        let mut rgb_pixels = Vec::with_capacity(self.pixel_count() * 3);

        fn unsigned_data_to_rgb_pixels<T>(
          data: Vec<T>,
          color_space: ColorSpace,
          rgb_pixels: &mut Vec<u8>,
          max_storable_value: u32,
        ) where
          T: Copy + Into<f64> + Into<u64>,
          u64: From<T>,
        {
          match color_space {
            ColorSpace::Rgb => {
              let max_storable_value: u64 = max_storable_value.into();

              for value in data {
                rgb_pixels.push(
                  udiv_round(u64::from(value) * 255, max_storable_value)
                    .min(255) as u8,
                );
              }
            }

            ColorSpace::Ybr { .. } => {
              let scale = 1.0 / f64::from(max_storable_value);

              for ybr in data.chunks_exact(3) {
                let y: f64 = ybr[0].into();
                let cb: f64 = ybr[1].into();
                let cr: f64 = ybr[2].into();

                let rgb = ybr_to_rgb(y * scale, cb * scale, cr * scale);

                rgb_pixels
                  .push((rgb[0] * 255.0).round().clamp(0.0, 255.0) as u8);
                rgb_pixels
                  .push((rgb[1] * 255.0).round().clamp(0.0, 255.0) as u8);
                rgb_pixels
                  .push((rgb[2] * 255.0).round().clamp(0.0, 255.0) as u8);
              }
            }
          }
        }

        match self.data {
          ColorImageData::U8 { data, color_space } => {
            unsigned_data_to_rgb_pixels(
              data,
              color_space,
              &mut rgb_pixels,
              max_storable_value,
            )
          }

          ColorImageData::U16 { data, color_space } => {
            unsigned_data_to_rgb_pixels(
              data,
              color_space,
              &mut rgb_pixels,
              max_storable_value,
            )
          }

          ColorImageData::U32 { data, color_space } => {
            unsigned_data_to_rgb_pixels(
              data,
              color_space,
              &mut rgb_pixels,
              max_storable_value,
            )
          }

          ColorImageData::PaletteU8 { data, palette } => {
            for pixel in data {
              let rgb = palette.lookup_normalized_u8(pixel.into());
              rgb_pixels.extend_from_slice(&rgb);
            }
          }

          ColorImageData::PaletteU16 { data, palette } => {
            for pixel in data {
              let rgb = palette.lookup_normalized_u8(pixel.into());
              rgb_pixels.extend_from_slice(&rgb);
            }
          }
        }

        image::RgbImage::from_raw(
          self.width.into(),
          self.height.into(),
          rgb_pixels,
        )
        .unwrap()
      }
    }
  }

  /// Converts this color image to a 16-bit RGB image.
  ///
  pub fn into_rgb_u16_image(
    self,
  ) -> image::ImageBuffer<image::Rgb<u16>, Vec<u16>> {
    let max_storable_value = self.max_storable_value();

    match self.data {
      // If this color image is already in RGB16 then return it directly,
      // avoiding a copy
      ColorImageData::U16 {
        color_space: ColorSpace::Rgb,
        data,
      } if self.bits_stored == 16 => image::ImageBuffer::from_raw(
        self.width.into(),
        self.height.into(),
        data,
      )
      .unwrap(),

      _ => {
        let mut rgb_pixels: Vec<u16> =
          Vec::with_capacity(self.pixel_count() * 3);

        fn unsigned_data_to_rgb_pixels<T>(
          data: Vec<T>,
          color_space: ColorSpace,
          rgb_pixels: &mut Vec<u16>,
          max_storable_value: u32,
        ) where
          T: Copy + Into<f64>,
          u64: From<T>,
        {
          match color_space {
            ColorSpace::Rgb => {
              let max_storable_value: u64 = max_storable_value.into();

              for value in data {
                rgb_pixels.push(
                  udiv_round(u64::from(value) * 65535, max_storable_value)
                    .min(65535) as u16,
                );
              }
            }

            ColorSpace::Ybr { .. } => {
              let scale = 1.0 / f64::from(max_storable_value);

              for ybr in data.chunks_exact(3) {
                let y: f64 = ybr[0].into();
                let cb: f64 = ybr[1].into();
                let cr: f64 = ybr[2].into();

                let rgb = ybr_to_rgb(y * scale, cb * scale, cr * scale);

                rgb_pixels
                  .push((rgb[0] * 65535.0).round().clamp(0.0, 65535.0) as u16);
                rgb_pixels
                  .push((rgb[1] * 65535.0).round().clamp(0.0, 65535.0) as u16);
                rgb_pixels
                  .push((rgb[2] * 65535.0).round().clamp(0.0, 65535.0) as u16);
              }
            }
          }
        }

        match self.data {
          ColorImageData::U8 { data, color_space } => {
            unsigned_data_to_rgb_pixels(
              data,
              color_space,
              &mut rgb_pixels,
              max_storable_value,
            )
          }

          ColorImageData::U16 { data, color_space } => {
            unsigned_data_to_rgb_pixels(
              data,
              color_space,
              &mut rgb_pixels,
              max_storable_value,
            )
          }

          ColorImageData::U32 { data, color_space } => {
            unsigned_data_to_rgb_pixels(
              data,
              color_space,
              &mut rgb_pixels,
              max_storable_value,
            )
          }

          ColorImageData::PaletteU8 { data, palette } => {
            for pixel in data {
              let rgb = palette.lookup_normalized_u8(pixel.into());
              rgb_pixels.push(rgb[0] as u16 * 257);
              rgb_pixels.push(rgb[1] as u16 * 257);
              rgb_pixels.push(rgb[2] as u16 * 257);
            }
          }

          ColorImageData::PaletteU16 { data, palette } => {
            for pixel in data {
              let rgb = palette.lookup_normalized_u8(pixel.into());
              rgb_pixels.push(rgb[0] as u16 * 257);
              rgb_pixels.push(rgb[1] as u16 * 257);
              rgb_pixels.push(rgb[2] as u16 * 257);
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
  }

  /// Converts this color image to an RGB F64 image where each value is in the
  /// range 0-1.
  ///
  pub fn to_rgb_f64_image(
    &self,
  ) -> image::ImageBuffer<image::Rgb<f64>, Vec<f64>> {
    let mut rgb_pixels = Vec::with_capacity(self.pixel_count() * 3);

    fn unsigned_data_to_rgb_pixels<T>(
      data: &[T],
      color_space: &ColorSpace,
      rgb_pixels: &mut Vec<f64>,
      max_storable_value: u32,
    ) where
      T: Copy + Into<f64>,
    {
      let scale = 1.0 / f64::from(max_storable_value);

      match color_space {
        ColorSpace::Rgb => {
          for rgb in data.chunks_exact(3) {
            rgb_pixels.push(rgb[0].into() * scale);
            rgb_pixels.push(rgb[1].into() * scale);
            rgb_pixels.push(rgb[2].into() * scale);
          }
        }

        ColorSpace::Ybr { .. } => {
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
        self.max_storable_value(),
      ),

      ColorImageData::U16 { data, color_space } => unsigned_data_to_rgb_pixels(
        data,
        color_space,
        &mut rgb_pixels,
        self.max_storable_value(),
      ),

      ColorImageData::U32 { data, color_space } => unsigned_data_to_rgb_pixels(
        data,
        color_space,
        &mut rgb_pixels,
        self.max_storable_value(),
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
fn ybr_to_rgb(y: f64, cb: f64, cr: f64) -> [f64; 3] {
  let r = y + 1.402 * (cr - 0.5);
  let g = y - 0.3441362862 * (cb - 0.5) - 0.7141362862 * (cr - 0.5);
  let b = y + 1.772 * (cb - 0.5);

  [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
}

/// Converts a YBR color into RGB.
///
fn rgb_to_ybr(r: f64, g: f64, b: f64) -> [f64; 3] {
  let y = 0.299 * r + 0.587 * g + 0.114 * b;
  let cb = -0.168736 * r - 0.331264 * g + 0.5 * b + 0.5;
  let cr = 0.5 * r - 0.418688 * g - 0.081312 * b + 0.5;

  [y.clamp(0.0, 1.0), cb.clamp(0.0, 1.0), cr.clamp(0.0, 1.0)]
}
