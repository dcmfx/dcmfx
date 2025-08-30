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

const OPENJPH_BITS_STORED_RANGE: core::ops::RangeInclusive<u16> = 2..=30;

/// Encodes a [`MonochromeImage`] into High-Throughput JPEG 2000 raw bytes using
/// OpenJPH.
///
pub fn encode_monochrome(
  image: &MonochromeImage,
  image_pixel_module: &ImagePixelModule,
  quality: Option<u8>,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  if !OPENJPH_BITS_STORED_RANGE.contains(&image_pixel_module.bits_stored()) {
    return Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: None,
    });
  }

  let width = image.width();
  let height = image.height();

  match (
    image.data(),
    image.is_monochrome1(),
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      MonochromeImageData::I8(data),
      true,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Eight,
    )
    | (
      MonochromeImageData::I8(data),
      false,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Eight,
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
    ) => encode(data, width, height, image_pixel_module, quality),

    (
      MonochromeImageData::I16(data),
      true,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Sixteen,
    )
    | (
      MonochromeImageData::I16(data),
      false,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Sixteen,
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
      quality,
    ),

    (
      MonochromeImageData::I32(data),
      true,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::ThirtyTwo,
    )
    | (
      MonochromeImageData::I32(data),
      false,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::ThirtyTwo,
    ) => encode(
      bytemuck::cast_slice(data),
      width,
      height,
      image_pixel_module,
      quality,
    ),

    (
      MonochromeImageData::U32(data),
      true,
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::ThirtyTwo,
    )
    | (
      MonochromeImageData::U32(data),
      false,
      PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::ThirtyTwo,
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

/// Encodes a [`ColorImage`] into High-Throughput JPEG 2000 raw bytes using
/// OpenJPH.
///
pub fn encode_color(
  image: &ColorImage,
  image_pixel_module: &ImagePixelModule,
  quality: Option<u8>,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  if !OPENJPH_BITS_STORED_RANGE.contains(&image_pixel_module.bits_stored()) {
    return Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: Some(image.color_space()),
    });
  }

  let width = image.width();
  let height = image.height();

  match (
    image.data(),
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
    quality,
  ) {
    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
      BitsAllocated::Eight,
      _,
    )
    | (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PhotometricInterpretation::YbrFull,
      BitsAllocated::Eight,
      _,
    )
    | (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::YbrRct,
      BitsAllocated::Eight,
      None,
    )
    | (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::YbrIct,
      BitsAllocated::Eight,
      Some(_),
    )
    | (
      ColorImageData::PaletteU8 { data, .. },
      PhotometricInterpretation::PaletteColor { .. },
      BitsAllocated::Eight,
      None,
    ) => encode(data, width, height, image_pixel_module, quality),

    (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
      BitsAllocated::Sixteen,
      _,
    )
    | (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PhotometricInterpretation::YbrFull,
      BitsAllocated::Sixteen,
      _,
    )
    | (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::YbrRct,
      BitsAllocated::Sixteen,
      None,
    )
    | (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::YbrIct,
      BitsAllocated::Sixteen,
      Some(_),
    )
    | (
      ColorImageData::PaletteU16 { data, .. },
      PhotometricInterpretation::PaletteColor { .. },
      BitsAllocated::Sixteen,
      None,
    ) => encode(
      bytemuck::cast_slice(data),
      width,
      height,
      image_pixel_module,
      quality,
    ),

    (
      ColorImageData::U32 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
      BitsAllocated::ThirtyTwo,
      _,
    )
    | (
      ColorImageData::U32 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PhotometricInterpretation::YbrFull,
      BitsAllocated::ThirtyTwo,
      _,
    )
    | (
      ColorImageData::U32 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::YbrRct,
      BitsAllocated::ThirtyTwo,
      None,
    )
    | (
      ColorImageData::U32 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::YbrIct,
      BitsAllocated::ThirtyTwo,
      Some(_),
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

static INITIALIZE_ONCE_LOCK: std::sync::OnceLock<()> =
  std::sync::OnceLock::new();

fn encode(
  data: &[u8],
  width: u16,
  height: u16,
  image_pixel_module: &ImagePixelModule,
  quality: Option<u8>,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  INITIALIZE_ONCE_LOCK
    .get_or_init(|| unsafe { ffi::openjph_encode_initialize() });

  let mut output_data = Vec::with_capacity(512 * 1024);

  let mut error_buffer = [0 as core::ffi::c_char; 256];

  let color_photometric_interpretation =
    match image_pixel_module.photometric_interpretation() {
      PhotometricInterpretation::Rgb => 1,
      PhotometricInterpretation::YbrFull => 2,
      PhotometricInterpretation::YbrIct => 3,
      PhotometricInterpretation::YbrRct => 4,
      _ => 0,
    };

  let quantization_step_size = if let Some(quality) = quality {
    quality_to_quantization_step_size(quality)
  } else {
    0.0
  };

  let result = unsafe {
    ffi::openjph_encode(
      data.as_ptr() as *const core::ffi::c_void,
      width.into(),
      height.into(),
      u8::from(image_pixel_module.samples_per_pixel()).into(),
      u8::from(image_pixel_module.bits_allocated()).into(),
      image_pixel_module.bits_stored().into(),
      u8::from(image_pixel_module.pixel_representation()).into(),
      color_photometric_interpretation,
      quantization_step_size,
      append_output_data,
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
      name: "OpenJPH encode failed".to_string(),
      details: error.to_string(),
    });
  }

  Ok(output_data)
}

/// Converts a quality value in the range 1-100 to a quantization step size
/// value for lossy compression. This equation was inspired by the publication
/// 'Parameterization of the Quality Factor for the High Throughput JPEG 2000'
/// by Ahar et al.
///
fn quality_to_quantization_step_size(quality: u8) -> f32 {
  let ratio = 25.0;

  if quality >= 50 {
    (1.0 - f32::from(quality - 50) / 100.0) / ratio
  } else {
    (50.0 / f32::from(quality)) / ratio
  }
}

/// This function is passed as a callback to [`ffi::openjph_encode()`] and
/// is then called with output data as it becomes available so it can be
/// accumulated in a [`Vec<u8>`].
///
extern "C" fn append_output_data(
  data: *const core::ffi::c_void,
  len: usize,
  context: *mut core::ffi::c_void,
) {
  unsafe {
    let output_data = &mut *(context as *mut Vec<u8>);

    (*output_data)
      .extend_from_slice(core::slice::from_raw_parts(data as *const u8, len));
  }
}

mod ffi {
  unsafe extern "C" {
    pub fn openjph_encode_initialize();

    pub fn openjph_encode(
      input_data: *const core::ffi::c_void,
      width: usize,
      height: usize,
      samples_per_pixel: usize,
      bits_allocated: usize,
      bits_stored: usize,
      pixel_representation: usize,
      color_photometric_interpretation: usize,
      quantization_step_size: f32,
      output_data_callback: extern "C" fn(
        *const core::ffi::c_void,
        usize,
        *mut core::ffi::c_void,
      ),
      output_data_context: *mut core::ffi::c_void,
      error_buffer: *mut core::ffi::c_char,
      error_buffer_size: usize,
    ) -> usize;
  }
}
