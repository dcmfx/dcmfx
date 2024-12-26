//! Fuzzing of DCMfx using AFL. Run using ./fuzz.sh.

#[macro_use]
extern crate afl;

use dcmfx::core::dictionary;
use dcmfx::p10::DataSetP10Extensions;

fn main() {
  fuzz!(|data: &[u8]| {
    // Reading P10 bytes should never panic, but a well-formed error is fine
    // because the input is being fuzzed and so may be invalid
    if let Ok(mut data_set) = dcmfx::p10::read_bytes(data.to_vec()) {
      // Write the data set to a buffer
      let mut cursor = std::io::Cursor::new(vec![]);
      data_set
        .write_p10_stream(&mut cursor, None)
        .expect("Writing data set should succeed");

      // Read the written data set
      let mut new_data_set = dcmfx::p10::read_bytes(cursor.into_inner())
        .expect("Reading back written data should succeed");

      // Check that the read version is the same
      prepare_data_set_for_comparison(&mut data_set);
      prepare_data_set_for_comparison(&mut new_data_set);
      if data_set != new_data_set {
        panic!("Rewritten data set should be identical");
      }
    }
  });
}

fn prepare_data_set_for_comparison(ds: &mut dcmfx::core::DataSet) {
  let tags_to_remove = vec![
    dictionary::FILE_META_INFORMATION_VERSION.tag,
    dictionary::IMPLEMENTATION_CLASS_UID.tag,
    dictionary::SPECIFIC_CHARACTER_SET.tag,
    dictionary::IMPLEMENTATION_VERSION_NAME.tag,
    dictionary::MEDIA_STORAGE_SOP_CLASS_UID.tag,
    dictionary::MEDIA_STORAGE_SOP_INSTANCE_UID.tag,
    dictionary::SOP_CLASS_UID.tag,
    dictionary::SOP_INSTANCE_UID.tag,
  ];

  for tag in tags_to_remove {
    ds.delete(tag);
  }
}
