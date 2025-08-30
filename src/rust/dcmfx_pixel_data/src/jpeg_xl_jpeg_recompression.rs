#[cfg(not(feature = "std"))]
use alloc::{string::ToString, vec::Vec};

use crate::PixelDataEncodeError;

/// Recompresses JPEG Baseline 8-bit data into JPEG XL data such that the
/// original JPEG is exactly preserved but is smaller in size. This is much
/// faster than a full recompression into JPEG XL and avoids a reduction in
/// image quality whilst also reducing size.
///
pub fn jpeg_to_jpeg_xl(
  jpeg_data: &[u8],
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let mut output_data = Vec::with_capacity(jpeg_data.len());
  let mut error_buffer = [0 as ::core::ffi::c_char; 256];

  let result = unsafe {
    ffi::libjxl_recompress_jpeg(
      jpeg_data.as_ptr() as *const core::ffi::c_void,
      jpeg_data.len(),
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
      name: "libjxl recompression failed".to_string(),
      details: error.to_string(),
    });
  }

  Ok(output_data)
}

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
    pub fn libjxl_recompress_jpeg(
      jpeg_data: *const core::ffi::c_void,
      jpeg_data_size: usize,
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
