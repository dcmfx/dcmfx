mod utils;

use assert_cmd::Command;
use insta::assert_snapshot;

#[macro_use]
mod assert_image_snapshot;
use utils::{generate_temp_filename, get_stderr, get_stdout, to_native_path};

#[test]
fn modify() {
  let dicom_file = "../../../test/assets/fo-dicom/CT-MONO2-16-ankle.dcm";
  let temp_path = generate_temp_filename();

  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("print")
    .arg(dicom_file)
    .assert()
    .success();

  assert_snapshot!("modify_before", get_stdout(assert));

  Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("modify")
    .arg("--transfer-syntax")
    .arg("explicit-vr-big-endian")
    .arg("--anonymize")
    .arg(dicom_file)
    .arg("--output-filename")
    .arg(&temp_path)
    .arg("--delete-tag")
    .arg("00080064")
    .arg("--delete-tag")
    .arg("00181020")
    .arg("--implementation-version-name")
    .arg("DCMfx Test")
    .assert()
    .success()
    .stdout(format!(
      "Modifying \"{}\" => \"{}\" …\n",
      to_native_path(&dicom_file),
      temp_path.display()
    ));

  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("print")
    .arg(&temp_path)
    .assert()
    .success();

  assert_snapshot!("modify_after", get_stdout(assert));
}

#[test]
fn modify_in_place() {
  let dicom_file = "../../../test/assets/fo-dicom/CR-MONO1-10-chest.dcm";
  let temp_path = generate_temp_filename();

  std::fs::copy(dicom_file, &temp_path).unwrap();

  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("print")
    .arg(&temp_path)
    .assert()
    .success();

  assert_snapshot!("modify_in_place_before", get_stdout(assert));

  Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("modify")
    .arg("--transfer-syntax")
    .arg("deflated-explicit-vr-little-endian")
    .arg("--in-place")
    .arg("--implementation-version-name")
    .arg("DCMfx Test")
    .arg(&temp_path)
    .assert()
    .success()
    .stdout(format!(
      "Modifying \"{}\" in place …\n",
      temp_path.display()
    ));

  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("print")
    .arg(&temp_path)
    .assert()
    .success();

  assert_snapshot!("modify_in_place_after", get_stdout(assert));
}

#[test]
fn errors_on_photometric_interpretation_monochrome_without_transfer_syntax() {
  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("modify")
    .arg("--photometric-interpretation-monochrome")
    .arg("MONOCHROME1")
    .arg("--in-place")
    .arg("tmp.dcm")
    .assert()
    .failure();

  assert_snapshot!(
    "errors_on_photometric_interpretation_monochrome_without_transfer_syntax",
    get_stderr(assert)
  );
}

#[test]
fn errors_on_photometric_interpretation_color_without_transfer_syntax() {
  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("modify")
    .arg("--photometric-interpretation-color")
    .arg("RGB")
    .arg("--in-place")
    .arg("tmp.dcm")
    .assert()
    .failure();

  assert_snapshot!(
    "errors_on_photometric_interpretation_color_without_transfer_syntax",
    get_stderr(assert)
  );
}

#[test]
fn errors_on_quality_without_transfer_syntax() {
  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("modify")
    .arg("--quality")
    .arg("50")
    .arg("--in-place")
    .arg("tmp.dcm")
    .assert()
    .failure();

  assert_snapshot!(
    "errors_on_quality_without_transfer_syntax",
    get_stderr(assert)
  );
}

#[test]
fn explicit_vr_little_endian_monochrome1_to_monochrome2() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/dicomdirtests/77654033/CR1/6154.dcm",
    "pass-through",
    "explicit_vr_little_endian_monochrome1_to_monochrome2",
    &["--photometric-interpretation-monochrome", "MONOCHROME2"],
  );
}

#[test]
fn explicit_vr_little_endian_rgb_to_ybr_full() {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/TestPattern_RGB.dcm",
    "pass-through",
    "explicit_vr_little_endian_rgb_to_ybr_full",
    &["--photometric-interpretation-color", "YBR_FULL"],
  );
}

#[test]
fn explicit_vr_little_endian_rgb_to_ybr_full_422() {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/TestPattern_RGB.dcm",
    "pass-through",
    "explicit_vr_little_endian_rgb_to_ybr_full_422",
    &["--photometric-interpretation-color", "YBR_FULL_422"],
  );
}

#[test]
fn explicit_vr_little_endian_planar_configuration_interleaved_to_separate() {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/TestPattern_RGB.dcm",
    "pass-through",
    "explicit_vr_little_endian_planar_configuration_interleaved_to_separate",
    &["--planar-configuration", "separate"],
  );
}

#[test]
fn explicit_vr_big_endian_rgb_planar_configuration_separate_to_interleaved() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/ExplVR_BigEnd.dcm",
    "pass-through",
    "explicit_vr_big_endian_rgb_planar_configuration_separate_to_interleaved",
    &["--planar-configuration", "interleaved"],
  );
}

#[test]
fn explicit_vr_little_endian_planar_configuration_interleaved_rgb_to_separate_ybr()
 {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/TestPattern_RGB.dcm",
    "pass-through",
    "explicit_vr_little_endian_planar_configuration_interleaved_rgb_to_separate_ybr",
    &[
      "--photometric-interpretation-color",
      "YBR_FULL",
      "--planar-configuration",
      "separate",
    ],
  );
}

#[test]
fn rle_lossless_to_explicit_vr_little_endian() {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/D_CLUNIE_CT1_RLE_FRAGS.dcm",
    "explicit-vr-little-endian",
    "rle_lossless_to_explicit_vr_little_endian",
    &[],
  );
}

#[test]
fn explicit_vr_little_endian_palette_color_to_rle_lossless() {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/TestPattern_Palette.dcm",
    "rle-lossless",
    "explicit_vr_little_endian_palette_color_to_rle_lossless",
    &[],
  );
}

#[test]
fn rle_lossless_to_deflated_explicit_vr_little_endian() {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/D_CLUNIE_CT1_RLE_FRAGS.dcm",
    "deflated-explicit-vr-little-endian",
    "rle_lossless_to_deflated_explicit_vr_little_endian",
    &[],
  );
}

#[test]
fn jpeg_baseline_to_implicit_vr_little_endian() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/examples_ybr_color.dcm",
    "implicit-vr-little-endian",
    "jpeg_baseline_to_implicit_vr_little_endian",
    &[],
  );
}

#[test]
fn jpeg_baseline_to_rle_lossless() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/examples_ybr_color.dcm",
    "rle-lossless",
    "jpeg_baseline_to_rle_lossless",
    &[],
  );
}

#[test]
fn jpeg_baseline_to_rle_lossless_with_rgb_conversion() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/examples_ybr_color.dcm",
    "implicit-vr-little-endian",
    "jpeg_baseline_to_rle_lossless_with_rgb_conversion",
    &["--photometric-interpretation-color", "RGB"],
  );
}

#[test]
fn monochrome_jpeg_xl_to_rle_lossless() {
  modify_transfer_syntax(
    "../../../test/assets/other/monochrome_jpeg_xl.dcm",
    "rle-lossless",
    "monochrome_jpeg_xl_to_rle_lossless",
    &[],
  );
}

#[test]
fn monochrome_jpeg_xl_to_rle_lossless_monochrome1() {
  modify_transfer_syntax(
    "../../../test/assets/other/monochrome_jpeg_xl.dcm",
    "rle-lossless",
    "monochrome_jpeg_xl_to_rle_lossless_monochrome1",
    &["--photometric-interpretation-monochrome", "MONOCHROME1"],
  );
}

#[test]
fn jpeg_ls_to_encapsulated_uncompressed_explicit_vr_little_endian() {
  modify_transfer_syntax(
    "../../../test/assets/other/jpeg_ls_ybr_color_space.dcm",
    "encapsulated-uncompressed-explicit-vr-little-endian",
    "jpeg_ls_to_encapsulated_uncompressed_explicit_vr_little_endian",
    &[],
  );
}

// The following test isn't run on Windows because the zlib-ng feature of flate2
// isn't used on that platform, which causes it to have different compression
// output
#[cfg(not(windows))]
#[test]
fn jpeg_2000_to_deflated_image_frame_compression() {
  modify_transfer_syntax(
    "../../../test/assets/other/jpeg_2000_ybr_color_space.dcm",
    "deflated-image-frame-compression",
    "jpeg_2000_to_deflated_image_frame_compression",
    &[],
  );
}

#[test]
fn monochrome_jpeg_xl_to_jpeg_baseline() {
  modify_transfer_syntax(
    "../../../test/assets/other/monochrome_jpeg_xl.dcm",
    "jpeg-baseline-8bit",
    "monochrome_jpeg_xl_to_jpeg_baseline",
    &[],
  );
}

#[test]
fn explicit_vr_little_endian_rgb_to_jpeg_baseline() {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/TestPattern_RGB.dcm",
    "jpeg-baseline-8bit",
    "explicit_vr_little_endian_rgb_to_jpeg_baseline",
    &[],
  );
}

#[test]
fn explicit_vr_little_endian_ybr_to_jpeg_baseline() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/SC_ybr_full_422_uncompressed.dcm",
    "jpeg-baseline-8bit",
    "explicit_vr_little_endian_ybr_to_jpeg_baseline",
    &[],
  );
}

#[test]
fn jpeg_baseline_to_jpeg_baseline_with_low_quality() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/examples_ybr_color.dcm",
    "jpeg-baseline-8bit",
    "jpeg_baseline_to_jpeg_baseline_with_low_quality",
    &["--quality", "10"],
  );
}

#[test]
fn palette_color_to_jpeg_baseline() {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/TestPattern_Palette.dcm",
    "jpeg-baseline-8bit",
    "palette_color_to_jpeg_baseline",
    &[],
  );
}

#[test]
fn monochrome_jpeg_xl_to_jpeg_2000_lossless_only() {
  modify_transfer_syntax(
    "../../../test/assets/other/monochrome_jpeg_xl.dcm",
    "jpeg-2k-lossless-only",
    "monochrome_jpeg_xl_to_jpeg_2000_lossless_only",
    &[],
  );
}

#[test]
fn monochrome_jpeg_xl_to_jpeg_2000() {
  modify_transfer_syntax(
    "../../../test/assets/other/monochrome_jpeg_xl.dcm",
    "jpeg-2k",
    "monochrome_jpeg_xl_to_jpeg_2000",
    &[],
  );
}

#[test]
fn palette_color_to_jpeg_2000_lossless_only() {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/TestPattern_Palette.dcm",
    "jpeg-2k-lossless-only",
    "palette_color_to_jpeg_2000_lossless_only",
    &[],
  );
}

#[test]
fn palette_color_to_jpeg_2000() {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/TestPattern_Palette.dcm",
    "jpeg-2k",
    "palette_color_to_jpeg_2000",
    &[],
  );
}

#[test]
fn explicit_vr_little_endian_rgb_to_jpeg_2000_lossless_only() {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/TestPattern_RGB.dcm",
    "jpeg-2k-lossless-only",
    "explicit_vr_little_endian_rgb_to_jpeg_2000_lossless_only",
    &[],
  );
}

#[test]
fn explicit_vr_little_endian_rgb_to_jpeg_2000() {
  modify_transfer_syntax(
    "../../../test/assets/fo-dicom/TestPattern_RGB.dcm",
    "jpeg-2k",
    "explicit_vr_little_endian_rgb_to_jpeg_2000",
    &["--quality", "10"],
  );
}

#[test]
fn explicit_vr_little_endian_ybr_to_jpeg_2000_lossless_only() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/SC_ybr_full_422_uncompressed.dcm",
    "jpeg-2k-lossless-only",
    "explicit_vr_little_endian_ybr_to_jpeg_2000_lossless_only",
    &[],
  );
}

#[test]
fn explicit_vr_little_endian_ybr_to_jpeg_2000() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/SC_ybr_full_422_uncompressed.dcm",
    "jpeg-2k",
    "explicit_vr_little_endian_ybr_to_jpeg_2000",
    &["--quality", "25"],
  );
}

#[test]
fn rle_lossless_rgb_16_bit_to_jpeg_2000_lossless_only() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/SC_rgb_rle_16bit_2frame.dcm",
    "jpeg-2k-lossless-only",
    "rle_lossless_rgb_16_bit_to_jpeg_2000_lossless_only",
    &[],
  );
}

#[test]
fn rle_lossless_rgb_16_bit_to_jpeg_2000() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/SC_rgb_rle_16bit_2frame.dcm",
    "jpeg-2k",
    "rle_lossless_rgb_16_bit_to_jpeg_2000",
    &["--quality", "40"],
  );
}

#[test]
fn jpeg_baseline_to_jpeg_2000_lossless_only_rgb() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/examples_ybr_color.dcm",
    "jpeg-2k-lossless-only",
    "jpeg_baseline_to_jpeg_2000_lossless_only_rgb",
    &["--photometric-interpretation-color", "RGB"],
  );
}

#[test]
fn jpeg_baseline_to_jpeg_2000_lossless_only_ybr_full() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/examples_ybr_color.dcm",
    "jpeg-2k-lossless-only",
    "jpeg_baseline_to_jpeg_2000_lossless_only_ybr_full",
    &["--photometric-interpretation-color", "YBR_FULL"],
  );
}

#[test]
fn jpeg_ls_monochrome_to_jpeg_2000_lossless_only() {
  modify_transfer_syntax(
    "../../../test/assets/pydicom/test_files/JPEGLSNearLossless_16.dcm",
    "jpeg-2k-lossless-only",
    "jpeg_ls_monochrome_to_jpeg_2000_lossless_only",
    &[],
  );
}

fn modify_transfer_syntax(
  dicom_file: &str,
  transfer_syntax: &str,
  snapshot_prefix: &str,
  extra_args: &[&str],
) {
  let temp_path = generate_temp_filename();

  std::fs::copy(dicom_file, &temp_path).unwrap();

  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("print")
    .arg(&temp_path)
    .assert()
    .success();

  assert_snapshot!(format!("{}_before", snapshot_prefix), get_stdout(assert));

  Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("modify")
    .arg("--transfer-syntax")
    .arg(transfer_syntax)
    .arg("--in-place")
    .arg("--implementation-version-name")
    .arg("DCMfx Test")
    .arg(&temp_path)
    .args(extra_args)
    .assert()
    .success()
    .stdout(format!(
      "Modifying \"{}\" in place …\n",
      temp_path.display()
    ));

  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("print")
    .arg(&temp_path)
    .assert()
    .success();

  assert_snapshot!(format!("{}_after", snapshot_prefix), get_stdout(assert));

  Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("get-pixel-data")
    .arg(&temp_path)
    .arg("-f")
    .arg("png16")
    .assert()
    .success();

  let output_file = format!("{}.0000.png", temp_path.display());
  assert_image_snapshot!(output_file, format!("{}.png", snapshot_prefix));
}
