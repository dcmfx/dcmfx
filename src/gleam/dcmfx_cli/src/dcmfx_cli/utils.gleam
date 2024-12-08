import gleam/result
import simplifile

/// Returns whether the specified file paths point to the same file on the same
/// device or volume once symlinks are resolved.
///
pub fn is_same_file(
  file0: String,
  file1: String,
) -> Result(Bool, simplifile.FileError) {
  use file0_info <- result.try(simplifile.file_info(file0))
  use file1_info <- result.try(simplifile.file_info(file1))

  Ok(file0_info == file1_info)
}
