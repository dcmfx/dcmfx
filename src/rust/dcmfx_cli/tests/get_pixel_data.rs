use std::path::PathBuf;

mod utils;

use assert_cmd::Command;

#[macro_use]
mod assert_image_snapshot;
use utils::{generate_temp_filename, to_native_path};

#[test]
fn single_bit_unaligned() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/liver_nonbyte_aligned.dcm";
  let output_files = [
    (
      format!("{}.0000.bin", dicom_file),
      format!("{}.0000.png", dicom_file),
    ),
    (
      format!("{}.0001.bin", dicom_file),
      format!("{}.0001.png", dicom_file),
    ),
    (
      format!("{}.0002.bin", dicom_file),
      format!("{}.0002.png", dicom_file),
    ),
  ];

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\nWriting \"{}\" …\n",
      to_native_path(&output_files[0].0),
      to_native_path(&output_files[1].0),
      to_native_path(&output_files[2].0)
    ));

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\nWriting \"{}\" …\n",
      to_native_path(&output_files[0].1),
      to_native_path(&output_files[1].1),
      to_native_path(&output_files[2].1)
    ));

  for (i, output_file) in output_files.iter().enumerate() {
    let filename = format!("get_pixel_data__single_bit_unaligned.000{i}.bin");

    assert_eq!(
      std::fs::read(&output_file.0).unwrap(),
      std::fs::read(format!("tests/snapshots/{filename}")).unwrap()
    );

    let filename = format!("single_bit_unaligned.000{i}.png");
    assert_image_snapshot!(&output_file.1, &filename);
  }
}

#[test]
fn single_bit_unaligned_cropped_to_png() {
  let dicom_file =
    "../../../test/assets/other/liver_nonbyte_aligned_cropped.dcm";
  let (output_file, output_directory) =
    prepare_outputs(dicom_file, ".0000.png");

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--output-directory")
    .arg(output_directory)
    .arg("-f")
    .arg("png")
    .assert();

  assert_image_snapshot!(
    output_file,
    "single_bit_unaligned_cropped_to_png.png"
  );
}

#[test]
fn rgb_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rgb_to_png.png");
}

#[test]
fn ybr_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_ybr_full_422_uncompressed.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "ybr_to_png.png");
}

#[test]
fn modality_lut_sequence() {
  let dicom_file = "../../../test/assets/fo-dicom/CR-ModalitySequenceLUT.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "modality_lut_sequence.png");
}

#[test]
fn rle_lossless_to_jpg() {
  let dicom_file = "../../../test/assets/pydicom/test_files/MR_small_RLE.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .arg("--voi-window")
    .arg("1136")
    .arg("2018")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_to_jpg.jpg");
}

#[test]
fn rle_lossless_bitmap_to_png() {
  let dicom_file = "../../../test/assets/other/liver_1frame.rle_lossless.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_bitmap_to_png.png");
}

#[test]
fn rle_lossless_color_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_rle_32bit.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_color_to_png.png");
}

#[test]
fn rle_lossless_color_palette_to_jpg() {
  let dicom_file =
    "../../../test/assets/other/TestPattern_Palette.rle_lossless.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_color_palette_to_jpg.jpg");
}

#[test]
fn to_jpg_with_custom_window() {
  let dicom_file = "../../../test/assets/fo-dicom/GH177_D_CLUNIE_CT1_IVRLE_BigEndian_ELE_undefinded_length.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .arg("--voi-window")
    .arg("500")
    .arg("2000")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "to_jpg_with_custom_window.jpg");
}

#[test]
fn missing_voi_lut_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/rtdose_expb_1frame.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "missing_voi_lut_to_png.png");
}

#[test]
fn palette_color_to_png() {
  let dicom_file = "../../../test/assets/fo-dicom/TestPattern_Palette_16.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "palette_color_to_png.png");
}

#[test]
fn resize_using_lanczos3() {
  let dicom_file = "../../../test/assets/fo-dicom/TestPattern_Palette_16.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .arg("--resize")
    .arg("100")
    .arg("0")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "resize_using_lanczos3_filter.png");
}

#[test]
fn resize_using_bilinear_filter() {
  let dicom_file = "../../../test/assets/fo-dicom/TestPattern_Palette_16.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .arg("--resize")
    .arg("100")
    .arg("0")
    .arg("--resize-filter")
    .arg("bilinear")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "resize_using_bilinear_filter.png");
}

#[test]
fn jpeg_2000_monochrome_to_jpg() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/MR_small_jp2klossless.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .arg("--voi-window")
    .arg("1136")
    .arg("2018")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_to_jpg.jpg");
}

#[test]
fn jpeg_2000_monochrome_to_png_16bit() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/MR_small_jp2klossless.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png16")
    .arg("--voi-window")
    .arg("1136")
    .arg("2018")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_2000_monochrome_to_png_16bit.png");
}

#[test]
fn jpeg_2000_color_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/GDCMJ2K_TextGBR.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_2000_color_to_png.png");
}

#[test]
fn jpeg_2000_ybr_color_space_to_jpg() {
  let dicom_file = "../../../test/assets/other/jpeg_2000_ybr_color_space.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_2000_ybr_color_space_to_jpg.jpg");
}

#[test]
fn jpeg_2000_monochrome_with_mismatched_pixel_representation_to_jpg() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/J2K_pixelrep_mismatch.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "jpeg_2000_monochrome_with_mismatched_pixel_representation_to_jpg.jpg"
  );
}

#[test]
fn jpeg_2000_monochrome_2bpp_to_png() {
  let dicom_file =
    "../../../test/assets/other/examples_jpeg_2000.monochrome_2bpp.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_2000_monochrome_2bpp_to_png.png");
}

#[test]
fn jpeg_ls_monochrome_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/JPEGLSNearLossless_16.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_ls_monochrome_to_png.png");
}

#[test]
fn jpeg_ls_color_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_jls_lossy_sample.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_ls_color_to_png.png");
}

#[test]
fn jpeg_ls_ybr_color_space_to_jpg() {
  let dicom_file = "../../../test/assets/other/jpeg_ls_ybr_color_space.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_ls_ybr_color_space_to_jpg.jpg");
}

#[test]
fn jpeg_ls_palette_color_to_png() {
  let dicom_file =
    "../../../test/assets/other/TestPattern_Palette_16.jpeg_ls_lossless.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_ls_palette_color_to_png.png");
}

#[test]
fn jpeg_lossless_12bit_to_jpg() {
  let dicom_file = "../../../test/assets/fo-dicom/IM-0001-0001-0001.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_lossless_12bit_to_jpg.jpg");
}

#[test]
fn jpeg_lossless_color_to_jpg() {
  let dicom_file = "../../../test/assets/fo-dicom/GH538-jpeg14sv1.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_lossless_color_to_jpg.jpg");
}

#[test]
fn jpeg_lossless_12bit_to_jpg_with_inverse_presentation_lut_shape() {
  let dicom_file = "../../../test/assets/other/jpeg_lossless_with_inverse_presentation_lut_shape.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "jpeg_lossless_12bit_to_jpg_with_inverse_presentation_lut_shape.jpg"
  );
}

#[test]
fn jpeg_lossless_12bit_to_jpg_with_presentation_lut() {
  let dicom_file =
    "../../../test/assets/other/jpeg_lossless_with_presentation_lut.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "jpeg_lossless_12bit_to_jpg_with_presentation_lut.jpg"
  );
}

#[test]
fn jpeg_extended_12bit_monochrome_to_png() {
  let dicom_file = "../../../test/assets/pydicom/test_files/JPEG-lossy.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "jpeg_extended_12bit_monochrome_to_png.png"
  );
}

#[test]
fn jpeg_xl_monochrome_to_png() {
  let dicom_file = "../../../test/assets/other/monochrome_jpeg_xl.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_xl_monochrome_to_png.png");
}

#[test]
fn jpeg_xl_monochrome_12bit_to_png() {
  let dicom_file = "../../../test/assets/other/monochrome_jpeg_xl_12bit.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .arg("--jpeg-xl-decoder")
    .arg("jxl-oxide")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_xl_monochrome_12bit_to_png.png");
}

#[test]
fn jpeg_xl_color_to_jpg() {
  let dicom_file = "../../../test/assets/other/ultrasound_jpeg_xl.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_xl_color_to_png.png");
}

#[test]
fn deflated_image_frame_compression() {
  let dicom_file = "../../../test/assets/other/SC_ybr_full_422_uncompressed.deflated_image_frame_compression.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "ybr_to_png.png");
}

#[test]
fn render_overlays() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/examples_overlay.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .arg("--overlays")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "render_overlays.jpg");
}

#[test]
fn render_overlays_out_of_bounds() {
  let dicom_file = "../../../test/assets/fo-dicom/OutOfBoundsOverlay.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .arg("--overlays")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "render_overlays_out_of_bounds.jpg");
}

#[test]
fn render_overlays_multiframe_unaligned() {
  let dicom_file = "../../../test/assets/other/mr_brucker_with_unaligned_multiframe_overlay.dcm";
  let output_files = [
    format!("{}.0000.jpg", dicom_file),
    format!("{}.0001.jpg", dicom_file),
    format!("{}.0002.jpg", dicom_file),
    format!("{}.0003.jpg", dicom_file),
  ];

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--overwrite")
    .arg("-f")
    .arg("jpg")
    .arg("--overlays")
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\n\
       Writing \"{}\" …\nWriting \"{}\" …\n",
      to_native_path(&output_files[0]),
      to_native_path(&output_files[1]),
      to_native_path(&output_files[2]),
      to_native_path(&output_files[3])
    ));

  assert_image_snapshot!(
    &output_files[1],
    "render_overlays_multiframe_unaligned.0001.jpg"
  );
  assert_image_snapshot!(
    &output_files[2],
    "render_overlays_multiframe_unaligned.0002.jpg"
  );
}

#[test]
fn single_bit_unaligned_to_mp4_h264() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/liver_nonbyte_aligned.dcm";
  let (output_file, output_directory) = prepare_outputs(dicom_file, ".mp4");

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--output-directory")
    .arg(&output_directory)
    .arg("--overwrite")
    .arg("-f")
    .arg("mp4")
    .arg("--mp4-preset")
    .arg("fast")
    .arg("--mp4-pixel-format")
    .arg("yuv420p10")
    .assert()
    .success()
    .stdout(format!("Writing \"{0}\" …\n", to_native_path(&output_file),));

  assert_eq!(
    get_video_stream_details(&output_file),
    Ok(VideoStreamDetails {
      codec_name: "h264".to_string(),
      profile: "High 10".to_string(),
      width: 510,
      height: 510,
      pix_fmt: "yuv420p10le".to_string(),
      nb_frames: "3".to_string(),
      r_frame_rate: "1/1".to_string(),
    })
  );
}

#[test]
fn single_bit_unaligned_to_mp4_h265() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/liver_nonbyte_aligned.dcm";
  let (output_file, output_directory) = prepare_outputs(dicom_file, ".mp4");

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--output-directory")
    .arg(&output_directory)
    .arg("--overwrite")
    .arg("-f")
    .arg("mp4")
    .arg("--mp4-codec")
    .arg("libx265")
    .arg("--mp4-crf")
    .arg("4")
    .arg("--mp4-preset")
    .arg("fast")
    .arg("--mp4-pixel-format")
    .arg("yuv422p12")
    .arg("--mp4-frame-rate")
    .arg("2")
    .assert()
    .success()
    .stdout(format!("Writing \"{0}\" …\n", to_native_path(&output_file),));

  assert_eq!(
    get_video_stream_details(&output_file),
    Ok(VideoStreamDetails {
      codec_name: "hevc".to_string(),
      profile: "Rext".to_string(),
      width: 510,
      height: 510,
      pix_fmt: "yuv422p12le".to_string(),
      nb_frames: "3".to_string(),
      r_frame_rate: "2/1".to_string(),
    })
  );
}

#[test]
fn render_overlays_and_rotate90() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/examples_overlay.dcm";
  let (output_file, output_directory) =
    prepare_outputs(dicom_file, ".0000.png");

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--output-directory")
    .arg(output_directory)
    .arg("--overwrite")
    .arg("-f")
    .arg("png16")
    .arg("--overlays")
    .arg("--transform")
    .arg("rotate90")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "render_overlays_and_rotate90.png");
}

#[test]
fn render_overlays_and_flip_horizontal() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/examples_overlay.dcm";
  let (output_file, output_directory) =
    prepare_outputs(dicom_file, ".0000.png");

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--output-directory")
    .arg(output_directory)
    .arg("--overwrite")
    .arg("-f")
    .arg("png16")
    .arg("--overlays")
    .arg("--transform")
    .arg("flip-horizontal")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "render_overlays_and_flip_horizontal.png"
  );
}

#[test]
fn with_output_directory() {
  let dicom_file = "../../../test/assets/other/mr_brucker_with_unaligned_multiframe_overlay.dcm";

  let output_directory = generate_temp_filename();
  std::fs::create_dir(&output_directory).unwrap();

  let output_files: Vec<String> = (0..4)
    .map(|i| {
      format!(
        "{}/mr_brucker_with_unaligned_multiframe_overlay.dcm.000{}.bin",
        output_directory.display(),
        i
      )
    })
    .collect();

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("--output-directory")
    .arg(output_directory)
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\n\
       Writing \"{}\" …\nWriting \"{}\" …\n",
      to_native_path(&output_files[0]),
      to_native_path(&output_files[1]),
      to_native_path(&output_files[2]),
      to_native_path(&output_files[3])
    ));
}

/// For a given input file, returns a newly created temporary output directory
/// and the path to the output file in that directory for the input file.
///
/// This ensures test outputs don't conflict when run in parallel.
///
fn prepare_outputs(
  input_file: &str,
  output_file_suffix: &str,
) -> (String, PathBuf) {
  let input_file = std::path::PathBuf::from(input_file);

  let output_directory = generate_temp_filename();
  std::fs::create_dir(&output_directory).unwrap();

  let output_file = format!(
    "{}/{}{}",
    output_directory.display(),
    input_file.file_name().unwrap().display(),
    output_file_suffix
  );

  (output_file, output_directory)
}

/// Returns details on the video stream of a video file.
///
fn get_video_stream_details(path: &str) -> Result<VideoStreamDetails, String> {
  let output = Command::new("ffprobe")
    .args([
      "-v",
      "error",
      "-select_streams",
      "v:0",
      "-show_entries",
      "stream=codec_name,profile,width,height,pix_fmt,nb_frames,r_frame_rate",
      "-of",
      "json",
      &path,
    ])
    .output()
    .map_err(|e| e.to_string())?;

  let mut parsed: FfprobeOutput =
    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;

  Ok(parsed.streams.remove(0))
}

#[derive(Debug, serde::Deserialize)]
struct FfprobeOutput {
  streams: Vec<VideoStreamDetails>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct VideoStreamDetails {
  pub codec_name: String,
  pub profile: String,
  pub width: i32,
  pub height: i32,
  pub pix_fmt: String,
  pub nb_frames: String,
  pub r_frame_rate: String,
}
