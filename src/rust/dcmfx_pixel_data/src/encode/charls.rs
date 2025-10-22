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
  lossless: bool,
) -> Result<ImagePixelModule, ()> {
  match image_pixel_module.photometric_interpretation() {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. }
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull => (),

    // PALETTE_COLOR is only permitted for lossless JPEG-LS encodes
    PhotometricInterpretation::PaletteColor { .. } => {
      if !lossless {
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
  quality: Option<u8>,
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
    ) if image_pixel_module.bits_stored() >= 2 => {
      encode(data, width, height, image_pixel_module, quality)
    }

    (
      MonochromeImageData::U16(data),
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Sixteen,
    ) if image_pixel_module.bits_stored() >= 2 => encode(
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

/// Encodes a [`ColorImage`] into JPEG-LS raw bytes using CharLS.
///
pub fn encode_color(
  image: &ColorImage,
  image_pixel_module: &ImagePixelModule,
  quality: Option<u8>,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let width = image.width();
  let height = image.height();

  match (
    image.data(),
    image_pixel_module.photometric_interpretation(),
    quality,
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
      None,
    ) if image_pixel_module.bits_stored() >= 2 => {
      encode(data, width, height, image_pixel_module, quality)
    }

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
      None,
    ) if image_pixel_module.bits_stored() >= 2 => encode(
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
  data: &[u8],
  width: u16,
  height: u16,
  image_pixel_module: &ImagePixelModule,
  quality: Option<u8>,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let mut output_buffer = vec![];

  let mut error_buffer = [0 as core::ffi::c_char; 256];

  let near_lossless: u8 = if let Some(quality) = quality {
    // Determine the maximum near_lossless value
    let max_near_lossless =
      2u16.pow(image_pixel_module.bits_stored() as u32 - 1) - 1;

    // Convert input u8 quality in range 1-100 to normalized value
    let quality = 1.0 - (quality - 1) as f32 / 99.0;

    // Map into the lossy range for CharLS compressor
    (1.0 + (max_near_lossless - 1) as f32 * quality) as u8
  } else {
    0
  };

  let bytes_written = unsafe {
    ffi::charls_encode(
      data.as_ptr() as *const core::ffi::c_void,
      width.into(),
      height.into(),
      u8::from(image_pixel_module.samples_per_pixel()).into(),
      u8::from(image_pixel_module.bits_allocated()).into(),
      near_lossless.into(),
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
      near_lossless: usize,
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
