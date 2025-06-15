#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::ToString, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataEncodeConfig,
  PixelDataEncodeError,
  color_image::ColorImageData,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation, PlanarConfiguration,
  },
  monochrome_image::MonochromeImageData,
};

/// Returns the Image Pixel Module resulting from encoding using libjpeg_12bit.
///
pub fn encode_image_pixel_module(
  mut image_pixel_module: ImagePixelModule,
) -> Result<ImagePixelModule, ()> {
  match image_pixel_module.photometric_interpretation() {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. }
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull
    | PhotometricInterpretation::YbrFull422 => (),

    _ => return Err(()),
  };

  image_pixel_module.set_planar_configuration(PlanarConfiguration::Interleaved);

  Ok(image_pixel_module)
}

/// Encodes a [`MonochromeImage`] into JPEG Extended 12-bit raw bytes using
/// libjpeg_12bit.
///
pub fn encode_monochrome(
  image: &MonochromeImage,
  image_pixel_module: &ImagePixelModule,
  encode_config: &PixelDataEncodeConfig,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let width = image.width();
  let height = image.height();
  let quality = encode_config.quality;

  match (
    image.data(),
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      MonochromeImageData::U16(data),
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Sixteen,
    ) if image_pixel_module.bits_stored() <= 12 => encode(
      bytemuck::cast_slice(data),
      width,
      height,
      image_pixel_module,
      quality,
    ),

    _ => Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: None,
    }),
  }
}

/// Encodes a [`ColorImage`] into JPEG Extended 12-bit raw bytes using
/// libjpeg_12bit.
///
pub fn encode_color(
  image: &ColorImage,
  image_pixel_module: &ImagePixelModule,
  encode_config: &PixelDataEncodeConfig,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let width = image.width();
  let height = image.height();
  let quality = encode_config.quality;

  match (
    image.data(),
    image_pixel_module.photometric_interpretation(),
  ) {
    (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
    )
    | (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PhotometricInterpretation::YbrFull,
    )
    | (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Ybr { is_422: true },
      },
      PhotometricInterpretation::YbrFull422,
    ) if image_pixel_module.bits_stored() <= 12 => encode(
      bytemuck::cast_slice(data),
      width,
      height,
      image_pixel_module,
      quality,
    ),

    _ => Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: Some(image.color_space()),
    }),
  }
}

fn encode(
  data: &[u16],
  width: u16,
  height: u16,
  image_pixel_module: &ImagePixelModule,
  quality: u8,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let mut output_buffer = vec![];
  let mut error_buffer = [0 as core::ffi::c_char; 256];

  let color_space = match image_pixel_module.photometric_interpretation() {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. } => 1,
    PhotometricInterpretation::Rgb => 2,
    PhotometricInterpretation::YbrFull
    | PhotometricInterpretation::YbrFull422 => 3,
    _ => unreachable!(),
  };

  let photometric_interpretation =
    match image_pixel_module.photometric_interpretation() {
      PhotometricInterpretation::Monochrome1 { .. } => 1,
      PhotometricInterpretation::Monochrome2 { .. } => 2,
      PhotometricInterpretation::Rgb => 3,
      PhotometricInterpretation::YbrFull => 4,
      PhotometricInterpretation::YbrFull422 => 5,
      _ => unreachable!(),
    };

  let result = unsafe {
    ffi::libjpeg_12bit_encode(
      data.as_ptr(),
      width.into(),
      height.into(),
      u8::from(image_pixel_module.samples_per_pixel()).into(),
      photometric_interpretation,
      color_space,
      quality.into(),
      output_data_callback,
      &mut output_buffer as *mut Vec<u8> as *mut core::ffi::c_void,
      error_buffer.as_mut_ptr(),
    )
  };

  if result != 0 {
    let error = unsafe { core::ffi::CStr::from_ptr(error_buffer.as_ptr()) }
      .to_str()
      .unwrap_or("<invalid error>");

    return Err(PixelDataEncodeError::OtherError {
      name: "libjpeg_12bit encode failed".to_string(),
      details: error.to_string(),
    });
  }

  Ok(output_buffer)
}

/// This function is passed as a callback to [`ffi::libjpeg_12bit_encode()`] and
/// is then called to receive output data.
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
    pub fn libjpeg_12bit_encode(
      input_data: *const u16,
      width: usize,
      height: usize,
      samples_per_pixel: usize,
      photometric_interpretation: usize,
      color_space: usize,
      quality: usize,
      output_data_callback: extern "C" fn(
        *const u8,
        usize,
        *mut core::ffi::c_void,
      ),
      output_buffer_context: *mut core::ffi::c_void,
      error_message: *mut core::ffi::c_char,
    ) -> usize;
  }
}
