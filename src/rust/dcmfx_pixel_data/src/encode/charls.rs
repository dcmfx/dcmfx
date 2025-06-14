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

/// Returns the Image Pixel Module resulting from encoding using CharLS.
///
pub fn encode_image_pixel_module(
  mut image_pixel_module: ImagePixelModule,
  is_near_lossless: bool,
) -> Result<ImagePixelModule, ()> {
  match image_pixel_module.photometric_interpretation() {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. }
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull => (),

    // PALETTE_COLOR is only permitted for lossless JPEG-LS encodes
    PhotometricInterpretation::PaletteColor { .. } => {
      if is_near_lossless {
        return Err(());
      }
    }

    _ => return Err(()),
  };

  image_pixel_module.set_planar_configuration(PlanarConfiguration::Interleaved);

  Ok(image_pixel_module)
}

/// Encodes a [`MonochromeImage`] into JPEG-LS raw bytes using CharLS.
///
pub fn encode_monochrome(
  image: &MonochromeImage,
  image_pixel_module: &ImagePixelModule,
  is_near_lossless: bool,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let width = image.width();
  let height = image.height();

  match (
    image.data(),
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      MonochromeImageData::U8(data),
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Eight,
    ) => encode(data, width, height, image_pixel_module, is_near_lossless),

    (
      MonochromeImageData::U16(data),
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Sixteen,
    ) => encode(
      bytemuck::cast_slice(data),
      width,
      height,
      image_pixel_module,
      is_near_lossless,
    ),

    _ => Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: None,
    }),
  }
}

/// Encodes a [`ColorImage`] into JPEG-LS raw bytes using CharLS.
///
pub fn encode_color(
  image: &ColorImage,
  image_pixel_module: &ImagePixelModule,
  is_near_lossless: bool,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let width = image.width();
  let height = image.height();

  match (
    image.data(),
    image_pixel_module.photometric_interpretation(),
    is_near_lossless,
  ) {
    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
      _,
    )
    | (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PhotometricInterpretation::YbrFull,
      _,
    )
    | (
      ColorImageData::PaletteU8 { data, .. },
      PhotometricInterpretation::PaletteColor { .. },
      false,
    ) => encode(data, width, height, image_pixel_module, is_near_lossless),

    (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
      _,
    )
    | (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PhotometricInterpretation::YbrFull,
      _,
    )
    | (
      ColorImageData::PaletteU16 { data, .. },
      PhotometricInterpretation::PaletteColor { .. },
      false,
    ) => encode(
      bytemuck::cast_slice(data),
      width,
      height,
      image_pixel_module,
      is_near_lossless,
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
  is_near_lossless: bool,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let mut output_buffer = vec![];

  let mut error_buffer = [0 as core::ffi::c_char; 256];

  let bytes_written = unsafe {
    ffi::charls_encode(
      data.as_ptr() as *const core::ffi::c_void,
      width.into(),
      height.into(),
      u8::from(image_pixel_module.samples_per_pixel()).into(),
      u8::from(image_pixel_module.bits_allocated()).into(),
      is_near_lossless.into(),
      output_buffer_allocate,
      &mut output_buffer as *mut Vec<u8> as *mut core::ffi::c_void,
      error_buffer.as_mut_ptr(),
      error_buffer.len(),
    )
  };

  if bytes_written == 0 {
    let error = unsafe { core::ffi::CStr::from_ptr(error_buffer.as_ptr()) }
      .to_str()
      .unwrap_or("<invalid error>");

    return Err(PixelDataEncodeError::OtherError {
      name: "CharLS encode failed".to_string(),
      details: error.to_string(),
    });
  }

  output_buffer.truncate(bytes_written as usize);

  Ok(output_buffer)
}

/// This function is passed as a callback to [`ffi::charls_encode()`] and
/// is then called to allocate output data.
///
extern "C" fn output_buffer_allocate(
  len: usize,
  context: *mut core::ffi::c_void,
) -> *mut core::ffi::c_void {
  unsafe {
    let output_buffer = &mut *(context as *mut Vec<u8>);

    output_buffer.resize(len, 0);
    output_buffer.as_mut_ptr() as *mut core::ffi::c_void
  }
}

mod ffi {
  unsafe extern "C" {
    pub fn charls_encode(
      input_data: *const core::ffi::c_void,
      width: usize,
      height: usize,
      samples_per_pixel: usize,
      bits_allocated: usize,
      is_near_lossless: usize,
      output_buffer_allocate: extern "C" fn(
        usize,
        *mut core::ffi::c_void,
      ) -> *mut core::ffi::c_void,
      output_buffer_context: *mut core::ffi::c_void,
      error_buffer: *mut core::ffi::c_char,
      error_buffer_size: usize,
    ) -> usize;
  }
}
