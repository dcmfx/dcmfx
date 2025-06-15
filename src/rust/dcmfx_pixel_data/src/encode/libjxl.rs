#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::ToString, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataEncodeError,
  color_image::ColorImageData,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation, PlanarConfiguration,
  },
  monochrome_image::MonochromeImageData,
};

use super::PixelDataEncodeConfig;

/// Returns the Image Pixel Module resulting from encoding using libjxl.
///
pub fn encode_image_pixel_module(
  mut image_pixel_module: ImagePixelModule,
  lossless: bool,
) -> Result<ImagePixelModule, ()> {
  match image_pixel_module.photometric_interpretation() {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. } => (),

    // RGB is only permitted for lossless encodes, for lossy encodes it is
    // converted to XYB
    PhotometricInterpretation::Rgb => {
      if !lossless {
        image_pixel_module
          .set_photometric_interpretation(PhotometricInterpretation::Xyb);
      }
    }

    // XYB is only permitted for lossy encodes
    PhotometricInterpretation::Xyb => {
      if lossless {
        return Err(());
      }
    }

    _ => return Err(()),
  }

  image_pixel_module.set_planar_configuration(PlanarConfiguration::Interleaved);

  Ok(image_pixel_module)
}

/// Encodes a [`MonochromeImage`] into JPEG XL raw bytes using libjxl.
///
pub fn encode_monochrome(
  image: &MonochromeImage,
  image_pixel_module: &ImagePixelModule,
  encode_config: &PixelDataEncodeConfig,
  lossless: bool,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let width = image.width();
  let height = image.height();

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
      width,
      height,
      image_pixel_module,
      encode_config,
      lossless,
    ),

    (
      MonochromeImageData::U16(data),
      true,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Sixteen,
    )
    | (
      MonochromeImageData::U16(data),
      false,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Sixteen,
    ) => encode(
      bytemuck::cast_slice(data),
      width,
      height,
      image_pixel_module,
      encode_config,
      lossless,
    ),

    _ => Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: None,
    }),
  }
}

/// Encodes a [`ColorImage`] into JPEG XL raw bytes using libjxl.
///
pub fn encode_color(
  image: &ColorImage,
  image_pixel_module: &ImagePixelModule,
  encode_config: &PixelDataEncodeConfig,
  lossless: bool,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let width = image.width();
  let height = image.height();

  match (
    image.data(),
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
    lossless,
  ) {
    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
      BitsAllocated::Eight,
      true,
    )
    | (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Xyb,
      BitsAllocated::Eight,
      false,
    ) => encode(
      data,
      width,
      height,
      image_pixel_module,
      encode_config,
      lossless,
    ),

    (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
      BitsAllocated::Sixteen,
      true,
    )
    | (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Xyb,
      BitsAllocated::Sixteen,
      false,
    ) => encode(
      bytemuck::cast_slice(data),
      width,
      height,
      image_pixel_module,
      encode_config,
      lossless,
    ),

    _ => Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: Some(image.color_space()),
    }),
  }
}

fn encode(
  data: &[u8],
  width: u16,
  height: u16,
  image_pixel_module: &ImagePixelModule,
  encode_config: &PixelDataEncodeConfig,
  lossless: bool,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let mut output_data = Vec::<u8>::with_capacity(256 * 1024);

  let mut error_buffer = [0 as ::core::ffi::c_char; 256];

  let result = unsafe {
    ffi::libjxl_encode(
      data.as_ptr() as *const core::ffi::c_void,
      data.len(),
      width.into(),
      height.into(),
      u8::from(image_pixel_module.samples_per_pixel()).into(),
      u8::from(image_pixel_module.bits_allocated()).into(),
      image_pixel_module.is_color().into(),
      lossless.into(),
      encode_config.quality.into(),
      encode_config.effort.into(),
      output_data_callback,
      &mut output_data as *mut Vec<u8> as *mut core::ffi::c_void,
      error_buffer.as_mut_ptr(),
      error_buffer.len(),
    )
  };

  if result != 0 {
    let error = unsafe { core::ffi::CStr::from_ptr(error_buffer.as_ptr()) }
      .to_str()
      .unwrap_or("<invalid error>");

    return Err(PixelDataEncodeError::OtherError {
      name: "libjxl encode failed".to_string(),
      details: error.to_string(),
    });
  }

  Ok(output_data)
}

/// This function is passed as a callback to [`ffi::libjxl_encode()`] and
/// is then called when the size of the output buffer needs to be changed. A
/// pointer to its base address is returned.
///
extern "C" fn output_data_callback(
  new_len: usize,
  context: *mut core::ffi::c_void,
) -> *mut core::ffi::c_void {
  unsafe {
    let output_data = &mut *(context as *mut Vec<u8>);

    output_data.resize(new_len, 0);
    output_data.as_mut_ptr() as *mut core::ffi::c_void
  }
}

mod ffi {
  unsafe extern "C" {
    pub fn libjxl_encode(
      input_data: *const core::ffi::c_void,
      input_data_size: usize,
      width: usize,
      height: usize,
      samples_per_pixel: usize,
      bits_allocated: usize,
      is_color: usize,
      lossless: usize,
      quality: usize,
      effort: usize,
      output_data_callback: extern "C" fn(
        usize,
        *mut core::ffi::c_void,
      ) -> *mut core::ffi::c_void,
      output_data_context: *mut core::ffi::c_void,
      error_buffer: *const core::ffi::c_char,
      error_buffer_size: usize,
    ) -> usize;
  }
}
