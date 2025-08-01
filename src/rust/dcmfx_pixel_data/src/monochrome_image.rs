#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use crate::{
  GrayscalePipeline,
  iods::{
    image_pixel_module::BitsAllocated,
    voi_lut_module::{VoiLutFunction, VoiWindow},
  },
  transforms::CropRect,
};

/// A monochrome image that stores an integer value for each pixel.
///
#[derive(Clone, Debug, PartialEq)]
pub struct MonochromeImage {
  width: u16,
  height: u16,
  data: MonochromeImageData,
  bits_stored: u16,
  is_monochrome1: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MonochromeImageData {
  Bitmap { data: Vec<u8>, is_signed: bool },
  I8(Vec<i8>),
  U8(Vec<u8>),
  I16(Vec<i16>),
  U16(Vec<u16>),
  I32(Vec<i32>),
  U32(Vec<u32>),
}

impl MonochromeImage {
  /// Creates a new monochrome image with bitmap 1bpp data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_bitmap(
    width: u16,
    height: u16,
    data: Vec<u8>,
    is_signed: bool,
    is_monochrome1: bool,
  ) -> Result<Self, &'static str> {
    if data.len() != (usize::from(width) * usize::from(height)).div_ceil(8) {
      return Err("Monochrome image bitmap data size is incorrect");
    }

    Ok(Self {
      width,
      height,
      data: MonochromeImageData::Bitmap { data, is_signed },
      bits_stored: 1,
      is_monochrome1,
    })
  }

  /// Creates a new monochrome image with `i8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_i8(
    width: u16,
    height: u16,
    data: Vec<i8>,
    bits_stored: u16,
    is_monochrome1: bool,
  ) -> Result<Self, &'static str> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err("Monochrome image i8 data size is incorrect");
    }

    if bits_stored == 0 || bits_stored > 8 {
      return Err("Monochrome image i8 bits stored must be <= 8");
    }

    Ok(Self {
      width,
      height,
      data: MonochromeImageData::I8(data),
      bits_stored,
      is_monochrome1,
    })
  }

  /// Creates a new monochrome image with `u8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u8(
    width: u16,
    height: u16,
    data: Vec<u8>,
    bits_stored: u16,
    is_monochrome1: bool,
  ) -> Result<Self, &'static str> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err("Monochrome image u8 data size is incorrect");
    }

    if bits_stored == 0 || bits_stored > 8 {
      return Err("Monochrome image u8 bits stored must be <= 8");
    }

    Ok(Self {
      width,
      height,
      data: MonochromeImageData::U8(data),
      bits_stored,
      is_monochrome1,
    })
  }

  /// Creates a new monochrome image with `i16` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_i16(
    width: u16,
    height: u16,
    data: Vec<i16>,
    bits_stored: u16,
    is_monochrome1: bool,
  ) -> Result<Self, &'static str> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err("Monochrome image i16 data size is incorrect");
    }

    if bits_stored == 0 || bits_stored > 16 {
      return Err("Monochrome image i16 bits stored must be <= 16");
    }

    Ok(Self {
      width,
      height,
      data: MonochromeImageData::I16(data),
      bits_stored,
      is_monochrome1,
    })
  }

  /// Creates a new monochrome image with `u16` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u16(
    width: u16,
    height: u16,
    data: Vec<u16>,
    bits_stored: u16,
    is_monochrome1: bool,
  ) -> Result<Self, &'static str> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err("Monochrome image u16 data size is incorrect");
    }

    if bits_stored == 0 || bits_stored > 16 {
      return Err("Monochrome image u16 bits stored must be <= 16");
    }

    Ok(Self {
      width,
      height,
      data: MonochromeImageData::U16(data),
      bits_stored,
      is_monochrome1,
    })
  }

  /// Creates a new monochrome image with `i32` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_i32(
    width: u16,
    height: u16,
    data: Vec<i32>,
    bits_stored: u16,
    is_monochrome1: bool,
  ) -> Result<Self, &'static str> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err("Monochrome image i32 data size is incorrect");
    }

    if bits_stored == 0 || bits_stored > 32 {
      return Err("Monochrome image i32 bits stored must be <= 32");
    }

    Ok(Self {
      width,
      height,
      data: MonochromeImageData::I32(data),
      bits_stored,
      is_monochrome1,
    })
  }

  /// Creates a new monochrome image with `u32` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u32(
    width: u16,
    height: u16,
    data: Vec<u32>,
    bits_stored: u16,
    is_monochrome1: bool,
  ) -> Result<Self, &'static str> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err("Monochrome image u32 data size is incorrect");
    }

    if bits_stored == 0 || bits_stored > 32 {
      return Err("Monochrome image u32 bits stored must be <= 32");
    }

    Ok(Self {
      width,
      height,
      data: MonochromeImageData::U32(data),
      bits_stored,
      is_monochrome1,
    })
  }

  /// Returns whether this monochrome image is empty, i.e. it has no pixels.
  ///
  pub fn is_empty(&self) -> bool {
    self.width == 0 || self.height == 0
  }

  /// Returns the width in pixels of this monochrome image.
  ///
  pub fn width(&self) -> u16 {
    self.width
  }

  /// Returns the height in pixels of this monochrome image.
  ///
  pub fn height(&self) -> u16 {
    self.height
  }

  /// Returns the internal data of this monochrome image.
  ///
  pub fn data(&self) -> &MonochromeImageData {
    &self.data
  }

  /// Returns the total number of pixels in this monochrome image.
  ///
  pub fn pixel_count(&self) -> usize {
    usize::from(self.width()) * usize::from(self.height())
  }

  /// Returns the number of bits allocated for each stored value.
  ///
  pub fn bits_allocated(&self) -> BitsAllocated {
    match self.data {
      MonochromeImageData::Bitmap { .. } => BitsAllocated::One,

      MonochromeImageData::I8(..) | MonochromeImageData::U8(..) => {
        BitsAllocated::Eight
      }

      MonochromeImageData::I16(..) | MonochromeImageData::U16(..) => {
        BitsAllocated::Sixteen
      }

      MonochromeImageData::I32(..) | MonochromeImageData::U32(..) => {
        BitsAllocated::ThirtyTwo
      }
    }
  }

  /// Returns the number of bits stored for each stored value. This will never
  /// exceed the number of bits allocated.
  ///
  pub fn bits_stored(&self) -> u16 {
    self.bits_stored
  }

  /// Returns whether this monochrome image stores signed pixel data.
  ///
  pub fn is_signed(&self) -> bool {
    match self.data {
      MonochromeImageData::Bitmap { is_signed, .. } => is_signed,

      MonochromeImageData::I8(..)
      | MonochromeImageData::I16(..)
      | MonochromeImageData::I32(..) => true,

      MonochromeImageData::U8(..)
      | MonochromeImageData::U16(..)
      | MonochromeImageData::U32(..) => false,
    }
  }

  /// Returns the minimum and maximum stored values in this monochrome image.
  ///
  pub fn min_max_values(&self) -> Option<(i64, i64)> {
    fn min_max<I: Iterator<Item = i64>>(iter: I) -> Option<(i64, i64)> {
      iter.fold(None, |acc, x| match acc {
        Some((min, max)) => Some((min.min(x), max.max(x))),
        None => Some((x, x)),
      })
    }

    match &self.data {
      MonochromeImageData::Bitmap { data, is_signed } => {
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

      MonochromeImageData::I8(data) => {
        min_max(data.iter().map(|pixel| (*pixel).into()))
      }

      MonochromeImageData::U8(data) => {
        min_max(data.iter().map(|pixel| (*pixel).into()))
      }

      MonochromeImageData::I16(data) => {
        min_max(data.iter().map(|pixel| (*pixel).into()))
      }

      MonochromeImageData::U16(data) => {
        min_max(data.iter().map(|pixel| (*pixel).into()))
      }

      MonochromeImageData::I32(data) => {
        min_max(data.iter().map(|pixel| (*pixel).into()))
      }

      MonochromeImageData::U32(data) => {
        min_max(data.iter().map(|pixel| (*pixel).into()))
      }
    }
  }

  /// Returns a VOI Window that covers the full range of values in this
  /// monochrome channel image.
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

  /// Returns whether this monochrome image's data uses the `MONOCHROME1`
  /// representation internally.
  ///
  pub fn is_monochrome1(&self) -> bool {
    self.is_monochrome1
  }

  /// Converts between `MONOCHROME1` and `MONOCHROME2` internal representations.
  ///
  pub fn change_monochrome_representation(&mut self) {
    self.is_monochrome1 = !self.is_monochrome1;

    match &mut self.data {
      MonochromeImageData::Bitmap { data, .. } => {
        for pixel in data.iter_mut() {
          *pixel = !*pixel;
        }
      }

      MonochromeImageData::I8(data) => {
        for pixel in data.iter_mut() {
          *pixel = (-isize::from(*pixel) - 1) as i8;
        }
      }

      MonochromeImageData::U8(data) => {
        let offset = (1u16 << self.bits_stored) - 1;
        for pixel in data.iter_mut() {
          *pixel = (offset - u16::from(*pixel)) as u8;
        }
      }

      MonochromeImageData::I16(data) => {
        for pixel in data.iter_mut() {
          *pixel = (-i32::from(*pixel) - 1) as i16;
        }
      }

      MonochromeImageData::U16(data) => {
        let offset = (1u32 << self.bits_stored) - 1;
        for pixel in data.iter_mut() {
          *pixel = (offset - u32::from(*pixel)) as u16;
        }
      }

      MonochromeImageData::I32(data) => {
        for pixel in data.iter_mut() {
          *pixel = (-i64::from(*pixel) - 1) as i32;
        }
      }

      MonochromeImageData::U32(data) => {
        let offset = (1u64 << self.bits_stored) - 1;
        for pixel in data.iter_mut() {
          *pixel = (offset - u64::from(*pixel)) as u32;
        }
      }
    }
  }

  /// Crops this monochrome image to the specified rectangle.
  ///
  pub fn crop(&mut self, crop_rect: &CropRect) {
    let left = crop_rect.left;
    let top = crop_rect.top;
    let (height, width) = crop_rect.apply(self.height(), self.width());

    if left == 0 && top == 0 && width == self.width && height == self.height {
      return;
    }

    // Helper for cropping non-bitmap data
    fn crop<T: Clone>(
      data: &mut Vec<T>,
      old_width: u16,
      left: u16,
      top: u16,
      width: u16,
      height: u16,
    ) {
      let mut new_data = Vec::with_capacity(width as usize * height as usize);

      for row in top..top + height {
        let start = row as usize * old_width as usize + left as usize;
        let end = start + width as usize;
        new_data.extend_from_slice(&data[start..end]);
      }

      *data = new_data;
    }

    match &mut self.data {
      // Crop 1-bit packed bitmap data
      MonochromeImageData::Bitmap { data, .. } => {
        let mut new_data =
          vec![0u8; (width as usize * height as usize).div_ceil(8)];
        let old_width = self.width;

        let mut output_bit = 0;
        for row in top..top + height {
          let mut input_bit = row as usize * old_width as usize + left as usize;
          for _ in 0..width {
            let bit = (data[input_bit / 8] >> (input_bit % 8)) & 1;
            new_data[output_bit / 8] |= bit << (output_bit % 8);

            input_bit += 1;
            output_bit += 1;
          }
        }

        *data = new_data;
      }
      MonochromeImageData::I8(data) => {
        crop(data, self.width, left, top, width, height)
      }
      MonochromeImageData::U8(data) => {
        crop(data, self.width, left, top, width, height);
      }
      MonochromeImageData::I16(data) => {
        crop(data, self.width, left, top, width, height);
      }
      MonochromeImageData::U16(data) => {
        crop(data, self.width, left, top, width, height);
      }
      MonochromeImageData::I32(data) => {
        crop(data, self.width, left, top, width, height);
      }
      MonochromeImageData::U32(data) => {
        crop(data, self.width, left, top, width, height);
      }
    }

    self.width = width;
    self.height = height;
  }

  /// Converts this monochrome image to an 8-bit grayscale image by passing
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

  /// Converts this monochrome image to a 16-bit grayscale image by passing
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
    let gray_pixels = match &self.data {
      MonochromeImageData::Bitmap { data, is_signed } => {
        let mut gray_pixels = Vec::with_capacity(self.pixel_count());

        let monochrome1_offset = self.monochrome1_offset();

        for pixel in data.iter() {
          for b in 0..8 {
            if gray_pixels.len() == gray_pixels.capacity() {
              break;
            }

            let mut value = i64::from((*pixel >> b) & 1);
            if *is_signed {
              value = -value;
            }
            if self.is_monochrome1 {
              value = -value + monochrome1_offset;
            }

            gray_pixels.push(stored_value_to_gray(value));
          }
        }

        gray_pixels
      }

      MonochromeImageData::I8(data) => {
        self.to_gray_image_internal(data, stored_value_to_gray)
      }
      MonochromeImageData::U8(data) => {
        self.to_gray_image_internal(data, stored_value_to_gray)
      }
      MonochromeImageData::I16(data) => {
        self.to_gray_image_internal(data, stored_value_to_gray)
      }
      MonochromeImageData::U16(data) => {
        self.to_gray_image_internal(data, stored_value_to_gray)
      }
      MonochromeImageData::I32(data) => {
        self.to_gray_image_internal(data, stored_value_to_gray)
      }
      MonochromeImageData::U32(data) => {
        self.to_gray_image_internal(data, stored_value_to_gray)
      }
    };

    image::ImageBuffer::from_raw(
      self.width.into(),
      self.height.into(),
      gray_pixels,
    )
    .unwrap()
  }

  fn to_gray_image_internal<T, U>(
    &self,
    data: &[T],
    stored_value_to_gray: impl Fn(i64) -> U,
  ) -> Vec<U>
  where
    T: Copy,
    i64: From<T>,
  {
    let mut gray_pixels = Vec::with_capacity(self.pixel_count());

    if self.is_monochrome1 {
      let offset = self.monochrome1_offset();

      for stored_value in data.iter() {
        gray_pixels
          .push(stored_value_to_gray(-i64::from(*stored_value) + offset));
      }
    } else {
      for stored_value in data.iter() {
        gray_pixels.push(stored_value_to_gray((*stored_value).into()));
      }
    }

    gray_pixels
  }

  /// Calculates the offset to add after negating the stored pixel value in
  /// order to convert to Monochrome2.
  ///
  fn monochrome1_offset(&self) -> i64 {
    if self.is_signed() {
      -1
    } else {
      (1i64 << self.bits_stored) - 1
    }
  }

  /// Returns this monochrome image's stored values.
  ///
  pub fn to_stored_values(&self) -> Vec<i64> {
    let mut stored_values = Vec::with_capacity(self.pixel_count());

    match &self.data {
      MonochromeImageData::Bitmap { data, is_signed } => {
        for pixel in data.iter() {
          for b in 0..8 {
            if stored_values.len() == stored_values.capacity() {
              break;
            }

            let mut value = i64::from((*pixel >> b) & 1);
            if *is_signed {
              value = -value;
            }

            stored_values.push(value);
          }
        }
      }

      MonochromeImageData::I8(data) => {
        for stored_value in data {
          stored_values.push(i64::from(*stored_value));
        }
      }

      MonochromeImageData::U8(data) => {
        for stored_value in data {
          stored_values.push(i64::from(*stored_value));
        }
      }

      MonochromeImageData::I16(data) => {
        for stored_value in data {
          stored_values.push(i64::from(*stored_value));
        }
      }

      MonochromeImageData::U16(data) => {
        for stored_value in data {
          stored_values.push(i64::from(*stored_value));
        }
      }

      MonochromeImageData::I32(data) => {
        for stored_value in data {
          stored_values.push(i64::from(*stored_value));
        }
      }

      MonochromeImageData::U32(data) => {
        for stored_value in data {
          stored_values.push(i64::from(*stored_value));
        }
      }
    };

    stored_values
  }
}
