#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::ToString, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataEncodeError,
  color_image::ColorImageData,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation,
  },
  monochrome_image::MonochromeImageData,
};

use super::PixelDataEncodeConfig;

/// Encodes a [`MonochromeImage`] into JPEG Baseline (Process 1) raw bytes using
/// libjpeg-turbo.
///
pub fn encode_monochrome(
  image: &MonochromeImage,
  image_pixel_module: &ImagePixelModule,
  encode_config: &PixelDataEncodeConfig,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  match (
    image.data(),
    image.is_monochrome1(),
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      MonochromeImageData::U8(data),
      true,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Eight,
    )
    | (
      MonochromeImageData::U8(data),
      false,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Eight,
    ) => encode(
      data,
      image.width(),
      image.height(),
      InputColorSpace::Grayscale,
      encode_config.quality,
    ),

    _ => Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: None,
    }),
  }
}

/// Encodes a [`ColorImage`] into JPEG Baseline (Process 1) raw bytes using
/// libjpeg-turbo.
///
pub fn encode_color(
  image: &ColorImage,
  image_pixel_module: &ImagePixelModule,
  encode_config: &PixelDataEncodeConfig,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  match (
    image.data(),
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
      BitsAllocated::Eight,
    ) => encode(
      data,
      image.width(),
      image.height(),
      InputColorSpace::Rgb,
      encode_config.quality,
    ),

    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Ybr { is_422: true },
      },
      PhotometricInterpretation::YbrFull422,
      BitsAllocated::Eight,
    ) => encode(
      data,
      image.width(),
      image.height(),
      InputColorSpace::YCbCr422,
      encode_config.quality,
    ),

    _ => Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: Some(image.color_space()),
    }),
  }
}

/// The color space and chroma subsampling of the data passed to libjpeg-turbo.
///
#[derive(Clone, Copy)]
enum InputColorSpace {
  /// Single-channel grayscale data.
  Grayscale,

  /// RGB data, which is converted to YCbCr and encoded with 4:4:4 sampling.
  Rgb,

  /// YCbCr data, which is encoded as-is with 4:2:2 sampling.
  YCbCr422,
}

fn encode(
  data: &[u8],
  width: u16,
  height: u16,
  input_color_space: InputColorSpace,
  quality: u8,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  // These values must match the DCMFX_COLOR_SPACE_* defines in
  // libjpeg_turbo_interface.c
  let (samples_per_pixel, color_space, is_422) = match input_color_space {
    InputColorSpace::Grayscale => (1, 0, false),
    InputColorSpace::Rgb => (3, 1, false),
    InputColorSpace::YCbCr422 => (3, 2, true),
  };

  if data.len() != usize::from(width) * usize::from(height) * samples_per_pixel
  {
    return Err(PixelDataEncodeError::OtherError {
      name: "libjpeg-turbo encode failed".to_string(),
      details: "Input data has incorrect size".to_string(),
    });
  }

  let mut output_buffer = Vec::with_capacity(data.len() / 4);
  let mut error_message = [0 as core::ffi::c_char; 200];

  let result = unsafe {
    ffi::libjpeg_turbo_encode_8bit(
      data.as_ptr(),
      width.into(),
      height.into(),
      samples_per_pixel,
      color_space,
      is_422.into(),
      quality.into(),
      output_data_callback,
      &mut output_buffer as *mut Vec<u8> as *mut core::ffi::c_void,
      error_message.as_mut_ptr(),
    )
  };

  if result != 0 {
    let error = unsafe { core::ffi::CStr::from_ptr(error_message.as_ptr()) }
      .to_str()
      .unwrap_or("<invalid error>");

    return Err(PixelDataEncodeError::OtherError {
      name: "libjpeg-turbo encode failed".to_string(),
      details: error.to_string(),
    });
  }

  Ok(output_buffer)
}

/// This function is passed as a callback to
/// [`ffi::libjpeg_turbo_encode_8bit()`] and is then called to receive output
/// data.
///
extern "C" fn output_data_callback(
  data: *const u8,
  len: usize,
  context: *mut core::ffi::c_void,
) {
  unsafe {
    let output_buffer = &mut *(context as *mut Vec<u8>);

    output_buffer.extend_from_slice(core::slice::from_raw_parts(data, len));
  }
}

mod ffi {
  unsafe extern "C" {
    pub fn libjpeg_turbo_encode_8bit(
      input_data: *const u8,
      width: usize,
      height: usize,
      samples_per_pixel: usize,
      input_color_space: usize,
      is_422: usize,
      quality: usize,
      output_data_callback: extern "C" fn(
        *const u8,
        usize,
        *mut core::ffi::c_void,
      ),
      output_data_context: *mut core::ffi::c_void,
      error_message: *mut core::ffi::c_char,
    ) -> usize;
  }
}
