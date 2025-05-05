use std::path::PathBuf;

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

pub fn image_matches_snapshot<P: AsRef<std::path::Path>>(
  path1: P,
  snapshot: &str,
) -> Result<(), String> {
  let image_1 = image::ImageReader::open(&path1)
    .unwrap()
    .decode()
    .unwrap()
    .to_rgb16();

  let image_snapshot_path =
    PathBuf::from(format!("tests/snapshots/{snapshot}"));

  let copy_command = format!(
    "To update snapshot run `cp {} {}`.",
    path1.as_ref().canonicalize().unwrap().display(),
    std::env::current_dir()
      .unwrap()
      .join(&image_snapshot_path)
      .display()
  );

  if !std::path::PathBuf::from(&image_snapshot_path).exists() {
    return Err(format!("Snapshot file is missing. {}", copy_command));
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
      "Image dimensions don't match snapshot. {}",
      copy_command
    ));
  }

  // Check that the pixels are the same within a small epsilon
  for y in 0..image_1.height() {
    for x in 0..image_1.width() {
      let a = image_1.get_pixel(x, y);
      let b = image_2.get_pixel(x, y);

      if (i32::from(a[0]) - i32::from(b[0])).abs() > 257
        || (i32::from(a[1]) - i32::from(b[1])).abs() > 257
        || (i32::from(a[2]) - i32::from(b[2])).abs() > 257
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
