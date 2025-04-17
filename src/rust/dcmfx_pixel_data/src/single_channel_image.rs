#[cfg(not(feature = "std"))]
use alloc::{string::ToString, vec::Vec};

use dcmfx_core::DataError;

use crate::{
  GrayscalePipeline,
  iods::voi_lut_module::{VoiLutFunction, VoiWindow},
};

/// A single channel image that stores an integer value for each pixel.
///
#[derive(Clone, Debug, PartialEq)]
pub struct SingleChannelImage {
  width: u16,
  height: u16,
  data: SingleChannelImageData,
  bits_stored: u16,
}

#[derive(Clone, Debug, PartialEq)]
enum SingleChannelImageData {
  Bitmap { data: Vec<u8>, is_signed: bool },
  I8(Vec<i8>),
  U8(Vec<u8>),
  I16(Vec<i16>),
  U16(Vec<u16>),
  I32(Vec<i32>),
  U32(Vec<u32>),
}

impl SingleChannelImage {
  /// Creates a new single channel image with bitmap 1bpp data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_bitmap(
    width: u16,
    height: u16,
    data: Vec<u8>,
    is_signed: bool,
  ) -> Result<Self, DataError> {
    if data.len() != (usize::from(width) * usize::from(height)).div_ceil(8) {
      return Err(DataError::new_value_invalid(
        "Single channel image bitmap data size is incorrect".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::Bitmap { data, is_signed },
      bits_stored: 1,
    })
  }

  /// Creates a new single channel image with `i8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_i8(
    width: u16,
    height: u16,
    data: Vec<i8>,
    bits_stored: u16,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Single channel image i8 data size is incorrect".to_string(),
      ));
    }

    if bits_stored == 0 || bits_stored > 8 {
      return Err(DataError::new_value_invalid(
        "Single channel image i8 bits stored must be <= 8".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::I8(data),
      bits_stored,
    })
  }

  /// Creates a new single channel image with `u8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u8(
    width: u16,
    height: u16,
    data: Vec<u8>,
    bits_stored: u16,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Single channel image u8 data size is incorrect".to_string(),
      ));
    }

    if bits_stored == 0 || bits_stored > 8 {
      return Err(DataError::new_value_invalid(
        "Single channel image u8 bits stored must be <= 8".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::U8(data),
      bits_stored,
    })
  }

  /// Creates a new single channel image with `i16` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_i16(
    width: u16,
    height: u16,
    data: Vec<i16>,
    bits_stored: u16,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Single channel image i16 data size is incorrect".to_string(),
      ));
    }

    if bits_stored == 0 || bits_stored > 16 {
      return Err(DataError::new_value_invalid(
        "Single channel image i16 bits stored must be <= 16".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::I16(data),
      bits_stored,
    })
  }

  /// Creates a new single channel image with `u16` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u16(
    width: u16,
    height: u16,
    data: Vec<u16>,
    bits_stored: u16,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Single channel image u16 data size is incorrect".to_string(),
      ));
    }

    if bits_stored == 0 || bits_stored > 16 {
      return Err(DataError::new_value_invalid(
        "Single channel image u16 bits stored must be <= 16".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::U16(data),
      bits_stored,
    })
  }

  /// Creates a new single channel image with `i32` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_i32(
    width: u16,
    height: u16,
    data: Vec<i32>,
    bits_stored: u16,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Single channel image i32 data size is incorrect".to_string(),
      ));
    }

    if bits_stored == 0 || bits_stored > 32 {
      return Err(DataError::new_value_invalid(
        "Single channel image i32 bits stored must be <= 32".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::I32(data),
      bits_stored,
    })
  }

  /// Creates a new single channel image with `u32` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u32(
    width: u16,
    height: u16,
    data: Vec<u32>,
    bits_stored: u16,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Single channel image u32 data size is incorrect".to_string(),
      ));
    }

    if bits_stored == 0 || bits_stored > 32 {
      return Err(DataError::new_value_invalid(
        "Single channel image u32 bits stored must be <= 32".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::U32(data),
      bits_stored,
    })
  }

  /// Returns whether this single channel image is empty, i.e. it has no pixels.
  ///
  pub fn is_empty(&self) -> bool {
    self.width == 0 || self.height == 0
  }

  /// Returns the width in pixels of this single channel image.
  ///
  pub fn width(&self) -> u16 {
    self.width
  }

  /// Returns the height in pixels of this single channel image.
  ///
  pub fn height(&self) -> u16 {
    self.height
  }

  /// Returns the total number of pixels in this single channel image.
  ///
  pub fn pixel_count(&self) -> usize {
    usize::from(self.width()) * usize::from(self.height())
  }

  /// Returns whether this single channel image stores signed pixel data.
  ///
  pub fn is_signed(&self) -> bool {
    match self.data {
      SingleChannelImageData::Bitmap { is_signed, .. } => is_signed,

      SingleChannelImageData::I8(..)
      | SingleChannelImageData::I16(..)
      | SingleChannelImageData::I32(..) => true,

      SingleChannelImageData::U8(..)
      | SingleChannelImageData::U16(..)
      | SingleChannelImageData::U32(..) => false,
    }
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

    match &self.data {
      SingleChannelImageData::Bitmap { data, is_signed } => {
        if data.iter().any(|pixel| *pixel != 0) {
          if *is_signed {
            Some((-1, 0))
          } else {
            Some((0, 1))
          }
        } else {
          Some((0, 0))
        }
      }

      SingleChannelImageData::I8(data) => {
        min_max(data.iter().map(|pixel| (*pixel).into()))
      }

      SingleChannelImageData::U8(data) => {
        min_max(data.iter().map(|pixel| (*pixel).into()))
      }

      SingleChannelImageData::I16(data) => {
        min_max(data.iter().map(|pixel| (*pixel).into()))
      }

      SingleChannelImageData::U16(data) => {
        min_max(data.iter().map(|pixel| (*pixel).into()))
      }

      SingleChannelImageData::I32(data) => {
        min_max(data.iter().map(|pixel| (*pixel).into()))
      }

      SingleChannelImageData::U32(data) => {
        min_max(data.iter().map(|pixel| (*pixel).into()))
      }
    }
  }

  /// Returns a VOI Window that covers the full range of values in this single
  /// channel image.
  ///
  pub fn default_voi_window(&self) -> Option<VoiWindow> {
    self.min_max_values().map(|(min, max)| {
      VoiWindow::new(
        (max + min) as f32 * 0.5,
        (max - min) as f32,
        "".into(),
        VoiLutFunction::LinearExact,
      )
    })
  }

  /// Converts Monochrome1 pixel data to Monochrome2.
  ///
  pub fn invert_monochrome1_data(&mut self) {
    // Calculate the offset to add after negating the stored pixel value in
    // order to convert to Monochrome2
    let offset: i64 = if self.is_signed() {
      -1
    } else {
      (1i64 << self.bits_stored) - 1
    };

    match &mut self.data {
      SingleChannelImageData::Bitmap { data, .. } => {
        for pixel in data.iter_mut() {
          *pixel = !*pixel;
        }
      }

      SingleChannelImageData::I8(data) => {
        for pixel in data.iter_mut() {
          *pixel = (-i64::from(*pixel) + offset)
            .clamp(i8::MIN as i64, i8::MAX as i64) as i8;
        }
      }

      SingleChannelImageData::U8(data) => {
        for pixel in data.iter_mut() {
          *pixel = (-i64::from(*pixel) + offset)
            .clamp(u8::MIN as i64, u8::MAX as i64) as u8;
        }
      }

      SingleChannelImageData::I16(data) => {
        for pixel in data.iter_mut() {
          *pixel = (-i64::from(*pixel) + offset)
            .clamp(i16::MIN as i64, i16::MAX as i64) as i16;
        }
      }

      SingleChannelImageData::U16(data) => {
        for pixel in data.iter_mut() {
          *pixel = (-i64::from(*pixel) + offset)
            .clamp(u16::MIN as i64, u16::MAX as i64) as u16;
        }
      }

      SingleChannelImageData::I32(data) => {
        for pixel in data.iter_mut() {
          *pixel = (-i64::from(*pixel) + offset)
            .clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        }
      }

      SingleChannelImageData::U32(data) => {
        for pixel in data.iter_mut() {
          *pixel = (-i64::from(*pixel) + offset)
            .clamp(u32::MIN as i64, u32::MAX as i64) as u32;
        }
      }
    }
  }

  /// Converts this single channel image to an 8-bit grayscale image by passing
  /// its values through the given grayscale LUT pipeline.
  ///
  pub fn to_gray_u8_image(
    &self,
    grayscale_pipeline: &GrayscalePipeline,
  ) -> image::GrayImage {
    match &*grayscale_pipeline.output_cache_u8() {
      Some(cache) => {
        self.to_gray_image(|stored_value: i64| cache.get(stored_value))
      }

      None => self.to_gray_image(|stored_value: i64| {
        grayscale_pipeline.apply_u8(stored_value)
      }),
    }
  }

  /// Converts this single channel image to a 16-bit grayscale image by passing
  /// its values through the given grayscale LUT pipeline.
  ///
  pub fn to_gray_u16_image(
    &self,
    grayscale_pipeline: &GrayscalePipeline,
  ) -> image::ImageBuffer<image::Luma<u16>, Vec<u16>> {
    match &*grayscale_pipeline.output_cache_u16() {
      Some(cache) => {
        self.to_gray_image(|stored_value: i64| cache.get(stored_value))
      }

      None => self.to_gray_image(|stored_value: i64| {
        grayscale_pipeline.apply_u16(stored_value)
      }),
    }
  }

  fn to_gray_image<T: image::Primitive>(
    &self,
    stored_value_to_gray: impl Fn(i64) -> T,
  ) -> image::ImageBuffer<image::Luma<T>, Vec<T>> {
    let mut gray_pixels = Vec::with_capacity(self.pixel_count());

    match &self.data {
      SingleChannelImageData::Bitmap { data, is_signed } => {
        for pixel in data.iter() {
          for b in 0..8 {
            if gray_pixels.len() == gray_pixels.capacity() {
              break;
            }

            let mut value = i64::from((*pixel >> b) & 1);
            if *is_signed {
              value = -value;
            }

            gray_pixels.push(stored_value_to_gray(value));
          }
        }
      }

      SingleChannelImageData::I8(data) => {
        for pixel in data.iter() {
          gray_pixels.push(stored_value_to_gray((*pixel).into()));
        }
      }

      SingleChannelImageData::U8(data) => {
        for pixel in data.iter() {
          gray_pixels.push(stored_value_to_gray((*pixel).into()));
        }
      }

      SingleChannelImageData::I16(data) => {
        for pixel in data.iter() {
          gray_pixels.push(stored_value_to_gray((*pixel).into()));
        }
      }

      SingleChannelImageData::U16(data) => {
        for pixel in data.iter() {
          gray_pixels.push(stored_value_to_gray((*pixel).into()));
        }
      }

      SingleChannelImageData::I32(data) => {
        for pixel in data.iter() {
          gray_pixels.push(stored_value_to_gray((*pixel).into()));
        }
      }

      SingleChannelImageData::U32(data) => {
        for pixel in data.iter() {
          gray_pixels.push(stored_value_to_gray((*pixel).into()));
        }
      }
    }

    image::ImageBuffer::from_raw(
      self.width.into(),
      self.height.into(),
      gray_pixels,
    )
    .unwrap()
  }

  /// Converts this single channel image to a [`Vec<i64>`].
  ///
  pub fn to_i64_pixels(&self) -> Vec<i64> {
    match &self.data {
      SingleChannelImageData::Bitmap { data, is_signed } => {
        let mut i64_pixels = Vec::with_capacity(self.pixel_count());

        for pixel in data.iter() {
          for b in 0..8 {
            if i64_pixels.len() == i64_pixels.capacity() {
              break;
            }

            let mut value = i64::from((*pixel >> b) & 1);
            if *is_signed {
              value = -value;
            }

            i64_pixels.push(value);
          }
        }

        i64_pixels
      }

      SingleChannelImageData::I8(data) => {
        data.iter().map(|pixel| (*pixel).into()).collect()
      }

      SingleChannelImageData::U8(data) => {
        data.iter().map(|pixel| (*pixel).into()).collect()
      }

      SingleChannelImageData::I16(data) => {
        data.iter().map(|pixel| (*pixel).into()).collect()
      }

      SingleChannelImageData::U16(data) => {
        data.iter().map(|pixel| (*pixel).into()).collect()
      }

      SingleChannelImageData::I32(data) => {
        data.iter().map(|pixel| (*pixel).into()).collect()
      }

      SingleChannelImageData::U32(data) => {
        data.iter().map(|pixel| (*pixel).into()).collect()
      }
    }
  }
}
