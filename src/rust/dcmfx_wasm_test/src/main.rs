// This small app embeds several DICOM files which it then reads and validates
// the decoded pixel data. Running a WASM build of this on a WASM runtime serves
// as a sanity check of pixel data decoding on that target architecture.
//
// To build and run:
//
//   cargo build --target wasm32-unknown-unknown --no-default-features 
//   wasmer target/wasm32-unknown-unknown/debug/dcmfx_wasm_test.wasm \
//     --invoke dcmfx_wasm_test
//

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use dcmfx::{core::*, p10::*, pixel_data::*};

const TEST_DICOMS: [(&[u8], i64); 3] = [
  (
    include_bytes!(
      "../../../../test/assets/pydicom/test_files/MR_small_padded.dcm"
    ),
    1389360,
  ),
  (
    include_bytes!("../../../../test/assets/pydicom/test_files/693_J2KI.dcm"),
    34860957,
  ),
  (
    include_bytes!(
      "../../../../test/assets/pydicom/test_files/JPGExtended.dcm"
    ),
    661392,
  ),
];

pub fn main() {
  dcmfx_wasm_test();
}

#[unsafe(no_mangle)]
fn dcmfx_wasm_test() -> i64 {
  for (dicom, pixel_data_sum) in TEST_DICOMS {
    let data_set = DataSet::read_p10_bytes(dicom.to_vec().into()).unwrap();

    let pixel_data_renderer =
      PixelDataRenderer::from_data_set(&data_set).unwrap();

    let mut frames = data_set.get_pixel_data_frames().unwrap();
    let pixels = pixel_data_renderer
      .render_frame(&mut frames[0], None)
      .unwrap()
      .into_raw();

    let sum: i64 = pixels.iter().map(|i| *i as i64).sum();
    assert_eq!(sum, pixel_data_sum);
  }

  1
}
