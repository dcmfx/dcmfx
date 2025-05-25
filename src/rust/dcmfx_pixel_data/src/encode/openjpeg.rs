#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::ToString, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataEncodeError,
  color_image::ColorImageData,
  iods::image_pixel_module::{
    ImagePixelModule, PhotometricInterpretation, PlanarConfiguration,
  },
  monochrome_image::MonochromeImageData,
};

/// Returns the Image Pixel Module resulting from encoding using OpenJPEG.
///
pub fn encode_image_pixel_module(
  mut image_pixel_module: ImagePixelModule,
  quality: Option<u8>,
) -> Result<ImagePixelModule, ()> {
  match image_pixel_module.photometric_interpretation() {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2
    | PhotometricInterpretation::Rgb => (),

    // YBR_ICT is only permitted for lossy JPEG 2000 encodes
    PhotometricInterpretation::YbrIct => {
      if quality.is_none() {
        return Err(());
      }
    }

    // YBR_FULL, YBR_RCT and PALETTE_COLOR are only permitted for lossless JPEG
    // 2000 encodes
    PhotometricInterpretation::YbrFull
    | PhotometricInterpretation::YbrRct
    | PhotometricInterpretation::PaletteColor { .. } => {
      if quality.is_some() {
        return Err(());
      }
    }

    _ => return Err(()),
  };

  image_pixel_module.set_planar_configuration(PlanarConfiguration::Interleaved);

  Ok(image_pixel_module)
}

/// Encodes a [`MonochromeImage`] into JPEG 2000 raw bytes using OpenJPEG.
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
    image.is_monochrome1(),
    image_pixel_module.photometric_interpretation(),
  ) {
    (
      MonochromeImageData::I8(data),
      true,
      PhotometricInterpretation::Monochrome1,
    )
    | (
      MonochromeImageData::I8(data),
      false,
      PhotometricInterpretation::Monochrome2,
    ) => encode(
      bytemuck::cast_slice(data),
      width,
      height,
      image_pixel_module,
      quality,
    ),

    (
      MonochromeImageData::U8(data),
      true,
      PhotometricInterpretation::Monochrome1,
    )
    | (
      MonochromeImageData::U8(data),
      false,
      PhotometricInterpretation::Monochrome2,
    ) => encode(data, width, height, image_pixel_module, quality),

    (
      MonochromeImageData::I16(data),
      true,
      PhotometricInterpretation::Monochrome1,
    )
    | (
      MonochromeImageData::I16(data),
      false,
      PhotometricInterpretation::Monochrome2,
    ) => encode(
      bytemuck::cast_slice(data),
      width,
      height,
      image_pixel_module,
      quality,
    ),

    (
      MonochromeImageData::U16(data),
      true,
      PhotometricInterpretation::Monochrome1,
    )
    | (
      MonochromeImageData::U16(data),
      false,
      PhotometricInterpretation::Monochrome2,
    ) => encode(
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

/// Encodes a [`ColorImage`] into JPEG 2000 raw bytes using OpenJPEG.
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
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::YbrRct,
      None,
    )
    | (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::YbrIct,
      Some(_),
    )
    | (
      ColorImageData::PaletteU8 { data, .. },
      PhotometricInterpretation::PaletteColor { .. },
      None,
    ) => encode(data, width, height, image_pixel_module, quality),

    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PhotometricInterpretation::YbrFull,
      _,
    ) => encode(data, width, height, image_pixel_module, quality),

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
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::YbrRct,
      None,
    )
    | (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::YbrIct,
      Some(_),
    )
    | (
      ColorImageData::PaletteU16 { data, .. },
      PhotometricInterpretation::PaletteColor { .. },
      None,
    ) => encode(
      bytemuck::cast_slice(data),
      width,
      height,
      image_pixel_module,
      quality,
    ),

    (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PhotometricInterpretation::YbrFull,
      _,
    ) => encode(
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
  let mut output_data = Vec::<u8>::with_capacity(512 * 1024);

  let mut error_buffer = [0 as ::core::ffi::c_char; 256];

  let color_photometric_interpretation =
    match image_pixel_module.photometric_interpretation() {
      PhotometricInterpretation::Rgb => 1,
      PhotometricInterpretation::YbrFull => 2,
      PhotometricInterpretation::YbrIct => 3,
      PhotometricInterpretation::YbrRct => 4,
      _ => 0,
    };

  let tcp_distoratio = if let Some(quality) = quality {
    quality_to_psnr(quality, image_pixel_module.bits_stored())
  } else {
    0.0
  };

  let result = unsafe {
    ffi::openjpeg_encode(
      data.as_ptr(),
      data.len() as u64,
      width as u32,
      height as u32,
      u32::from(u8::from(image_pixel_module.samples_per_pixel())),
      u32::from(u8::from(image_pixel_module.bits_allocated())),
      u32::from(image_pixel_module.bits_stored()),
      u32::from(u8::from(image_pixel_module.pixel_representation())),
      color_photometric_interpretation,
      tcp_distoratio,
      append_output_data,
      &mut output_data as *mut Vec<u8> as *mut core::ffi::c_void,
      error_buffer.as_mut_ptr(),
      error_buffer.len() as u32,
    )
  };

  if result != 0 {
    let error = unsafe { core::ffi::CStr::from_ptr(error_buffer.as_ptr()) }
      .to_str()
      .unwrap_or("<invalid error>");

    return Err(PixelDataEncodeError::OtherError {
      name: "JPEG 2000 encode failed".to_string(),
      details: error.to_string(),
    });
  }

  Ok(output_data)
}

/// Converts a quality value in the range 1-100 to a PSNR value for lossy
/// compression. The value depends on the bits stored value because higher bit
/// depths need higher PSNR values to maintain similar error characteristics.
///
fn quality_to_psnr(quality: u8, bits_stored: u16) -> f32 {
  let t = (f32::from(bits_stored) - 8.0).max(0.0);

  let min_quality_psnr = 28.0 + 1.875 * t;
  let max_quality_psnr = 50.0 + 3.750 * t;

  min_quality_psnr
    + (max_quality_psnr - min_quality_psnr)
      * ((f32::from(quality) - 1.0) / 99.0).powf(2.0)
}

/// This function is passed as a callback to [`ffi::openjpeg_encode()`] and
/// is then called with output data as it becomes available so it can be
/// accumulated in a [`Vec<u8>`].
///
extern "C" fn append_output_data(
  data: *const u8,
  len: u32,
  context: *mut core::ffi::c_void,
) {
  unsafe {
    let output_data = &mut *(context as *mut Vec<u8>);

    (*output_data)
      .extend_from_slice(core::slice::from_raw_parts(data, len as usize));
  }
}

mod ffi {
  unsafe extern "C" {
    pub fn openjpeg_encode(
      input_data: *const u8,
      input_data_size: u64,
      width: u32,
      height: u32,
      samples_per_pixel: u32,
      bits_allocated: u32,
      bits_stored: u32,
      pixel_representation: u32,
      color_photometric_interpretation: u32,
      tcp_distoratio: f32,
      output_data_callback: extern "C" fn(
        *const u8,
        u32,
        *mut core::ffi::c_void,
      ),
      output_data_context: *mut core::ffi::c_void,
      error_buffer: *mut ::core::ffi::c_char,
      error_buffer_size: u32,
    ) -> i32;
  }
}
