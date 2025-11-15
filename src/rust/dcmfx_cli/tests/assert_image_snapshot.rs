use std::path::{Path, PathBuf};

/// Macro to compare an image file to a snapshot.
///
#[macro_export]
macro_rules! assert_image_snapshot {
  ($left:expr, $right:expr) => {
    assert_eq!(
      crate::assert_image_snapshot::image_matches_snapshot(
        $left,
        &format!("{}__{}", module_path!(), $right)
      ),
      Ok(())
    )
  };
}

pub fn image_matches_snapshot<P: AsRef<Path>>(
  input_file: P,
  snapshot_name: &str,
) -> Result<(), String> {
  if !input_file.as_ref().exists() {
    return Err(format!(
      "Input file is missing: {}",
      input_file.as_ref().display()
    ));
  }

  let image_1 = image::ImageReader::open(&input_file)
    .unwrap()
    .decode()
    .unwrap()
    .to_rgb16();

  let image_snapshot_path =
    PathBuf::from(format!("tests/snapshots/{snapshot_name}"));

  let copy_command = format!(
    "To update snapshot run `cp {} {}`.",
    input_file.as_ref().canonicalize().unwrap().display(),
    std::env::current_dir()
      .unwrap()
      .join(&image_snapshot_path)
      .display()
  );

  if !PathBuf::from(&image_snapshot_path).exists() {
    return Err(format!("Snapshot file is missing: {}", copy_command));
  }

  let image_2 = image::ImageReader::open(&image_snapshot_path)
    .unwrap()
    .decode()
    .expect(&format!(
      "{} to be a valid image file",
      image_snapshot_path.display()
    ))
    .to_rgb16();

  if image_1.width() != image_2.width() || image_1.height() != image_2.height()
  {
    return Err(format!(
      "Image dimensions ({}x{}) don't match snapshot ({}x{}). {}",
      image_1.width(),
      image_1.height(),
      image_2.width(),
      image_2.height(),
      copy_command
    ));
  }

  // Check that the pixels are the same within a small epsilon
  for y in 0..image_1.height() {
    for x in 0..image_1.width() {
      let a = image_1.get_pixel(x, y);
      let b = image_2.get_pixel(x, y);

      #[cfg(target_arch = "aarch64")]
      let epsilon = 257;

      // When compiled for different architectures some codec libraries, e.g.
      // OpenJPEG, give a slightly different result, so increase the epsilon
      // value a little in order to account for this
      #[cfg(not(target_arch = "aarch64"))]
      let epsilon = 771;

      if (i32::from(a[0]) - i32::from(b[0])).abs() > epsilon
        || (i32::from(a[1]) - i32::from(b[1])).abs() > epsilon
        || (i32::from(a[2]) - i32::from(b[2])).abs() > epsilon
      {
        return Err(format!(
          "Image differs at pixel {},{}: expected {:?} but got {:?}. {}",
          x, y, b, a, copy_command
        ));
      }
    }
  }

  Ok(())
}
