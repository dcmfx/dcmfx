#[cfg(not(feature = "std"))]
use alloc::{string::ToString, vec::Vec};

use dcmfx_core::DataError;

use crate::{
  ModalityLut, PhotometricInterpretation, PixelDataDefinition, VoiLut,
  VoiWindow,
};

/// A single channel image that stores an integer value for each pixel.
///
#[derive(Clone, Debug, PartialEq)]
pub struct SingleChannelImage {
  width: u16,
  height: u16,
  data: SingleChannelImageData,
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
    })
  }

  /// Creates a new single channel image with `i8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_i8(
    width: u16,
    height: u16,
    data: Vec<i8>,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Single channel image i8 data size is incorrect".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::I8(data),
    })
  }

  /// Creates a new single channel image with `u8` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u8(
    width: u16,
    height: u16,
    data: Vec<u8>,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Single channel image u8 data size is incorrect".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::U8(data),
    })
  }

  /// Creates a new single channel image with `i16` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_i16(
    width: u16,
    height: u16,
    data: Vec<i16>,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Single channel image i16 data size is incorrect".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::I16(data),
    })
  }

  /// Creates a new single channel image with `u16` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u16(
    width: u16,
    height: u16,
    data: Vec<u16>,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Single channel image u16 data size is incorrect".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::U16(data),
    })
  }

  /// Creates a new single channel image with `i32` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_i32(
    width: u16,
    height: u16,
    data: Vec<i32>,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Single channel image i32 data size is incorrect".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::I32(data),
    })
  }

  /// Creates a new single channel image with `u32` data.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn new_u32(
    width: u16,
    height: u16,
    data: Vec<u32>,
  ) -> Result<Self, DataError> {
    if data.len() != usize::from(width) * usize::from(height) {
      return Err(DataError::new_value_invalid(
        "Single channel image u32 data size is incorrect".to_string(),
      ));
    }

    Ok(Self {
      width,
      height,
      data: SingleChannelImageData::U32(data),
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
    if definition.photometric_interpretation()
      != &PhotometricInterpretation::Monochrome1
    {
      return;
    }

    // Calculate the offset to add after negating the stored pixel value in
    // order to convert to Monochrome2
    let offset: i64 = if definition.pixel_representation().is_signed() {
      -1
    } else {
      definition.int_max().into()
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

  /// Converts this single channel image to a grayscale image by passing its
  /// values through the given Modality LUT and VOI LUT.
  ///
  pub fn to_gray_image(
    &self,
    modality_lut: &ModalityLut,
    voi_lut: &VoiLut,
  ) -> image::GrayImage {
    let mut gray_pixels = Vec::with_capacity(self.pixel_count());

    let i64_to_u8 = |pixel: i64| {
      // Apply LUTs
      let x = modality_lut.apply_to_stored_value(pixel);
      let x = voi_lut.apply(x);

      // Convert to u8
      (x * 255.0).clamp(0.0, 255.0) as u8
    };

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

            gray_pixels.push(i64_to_u8(value));
          }
        }
      }

      SingleChannelImageData::I8(data) => {
        for pixel in data.iter() {
          gray_pixels.push(i64_to_u8((*pixel).into()));
        }
      }

      SingleChannelImageData::U8(data) => {
        for pixel in data.iter() {
          gray_pixels.push(i64_to_u8((*pixel).into()));
        }
      }

      SingleChannelImageData::I16(data) => {
        for pixel in data.iter() {
          gray_pixels.push(i64_to_u8((*pixel).into()));
        }
      }

      SingleChannelImageData::U16(data) => {
        for pixel in data.iter() {
          gray_pixels.push(i64_to_u8((*pixel).into()));
        }
      }

      SingleChannelImageData::I32(data) => {
        for pixel in data.iter() {
          gray_pixels.push(i64_to_u8((*pixel).into()));
        }
      }

      SingleChannelImageData::U32(data) => {
        for pixel in data.iter() {
          gray_pixels.push(i64_to_u8((*pixel).into()));
        }
      }
    }

    image::GrayImage::from_raw(
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
