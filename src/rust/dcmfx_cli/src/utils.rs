/// Returns whether the specified file paths point to the same file on the same
/// device or volume once symlinks are resolved.
///
pub fn is_same_file(file0: &str, file1: &str) -> std::io::Result<bool> {
  let metadata1 = std::fs::metadata(file0)?;
  let metadata2 = std::fs::metadata(file1)?;

  #[cfg(unix)]
  {
    use std::os::unix::fs::MetadataExt;

    Ok(metadata1.ino() == metadata2.ino() && metadata1.dev() == metadata2.dev())
  }

  #[cfg(windows)]
  {
    use std::os::windows::fs::MetadataExt;

    Ok(
      metadata1.file_index() == metadata2.file_index()
        && metadata1.volume_serial_number() == metadata2.volume_serial_number(),
    )
  }
}
