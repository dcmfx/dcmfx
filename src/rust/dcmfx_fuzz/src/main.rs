//! Fuzzing of DCMfx using AFL. Run using ./fuzz.sh.

#[macro_use]
extern crate afl;

use dcmfx::core::dictionary;
use dcmfx::p10::DataSetP10Extensions;
use dcmfx::pixel_data::{
  DataSetPixelDataExtensions, Overlays, PixelDataDefinition, PixelDataRenderer,
};

fn main() {
  fuzz!(|data: &[u8]| {
    // Reading P10 bytes should never panic, but a well-formed error is fine
    // because the input is being fuzzed and so may be invalid
    if let Ok(mut data_set) = dcmfx::p10::read_bytes(data.to_vec().into()) {
      // Write the data set to a buffer
      let mut cursor = std::io::Cursor::new(vec![]);
      data_set
        .write_p10_stream(&mut cursor, None)
        .expect("Writing data set should succeed");

      // Read the written data set
      let mut new_data_set = dcmfx::p10::read_bytes(cursor.into_inner().into())
        .expect("Reading back written data should succeed");

      // Check that the read version is the same
      prepare_data_set_for_comparison(&mut data_set);
      prepare_data_set_for_comparison(&mut new_data_set);
      if data_set != new_data_set {
        panic!("Rewritten data set should be identical");
      }

      // Render the pixel data. This should never panic.
      let frames = data_set.get_pixel_data_frames();
      if let Ok(frames) = frames {
        if let Ok(renderer) = PixelDataRenderer::from_data_set(&data_set) {
          for mut frame in frames {
            let _ = renderer.render_frame(&mut frame, None);
          }
        }
      }

      // Render the overlays. This should never panic.
      if let Ok(overlays) = Overlays::from_data_set(&data_set) {
        if !overlays.is_empty() {
          if let Ok(definition) = PixelDataDefinition::from_data_set(&data_set) {
            if definition.pixel_count() <= 4096 * 4096 {
              // Allocate output image
              let mut rgb_image = image::DynamicImage::new_rgb8(
                definition.rows().into(),
                definition.columns().into(),
              );

              overlays.render_to_rgb_image(&mut rgb_image, 0).unwrap();
            }
          }
        }
      }
    }
  });
}

fn prepare_data_set_for_comparison(data_set: &mut dcmfx::core::DataSet) {
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
    data_set.delete(tag);
  }
}
