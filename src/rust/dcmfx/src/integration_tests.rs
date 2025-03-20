#[cfg(feature = "std")]
#[cfg(test)]
mod tests {
  const RNG_SEED: u64 = 1023;

  use std::{ffi::OsStr, fs::File, io::Read, io::Write, path::Path, rc::Rc};

  use rand::rngs::SmallRng;
  use rand::{Rng, SeedableRng};
  use walkdir::WalkDir;

  use dcmfx_core::*;
  use dcmfx_json::*;
  use dcmfx_p10::*;
  use dcmfx_pixel_data::*;

  #[test]
  fn integration_tests() -> Result<(), ()> {
    let test_assets_dir = if Path::new("../../test/assets").is_dir() {
      "../../test/assets"
    } else {
      "../../../test/assets"
    };

    // List all files in the test assets directory
    let data_files = WalkDir::new(test_assets_dir)
      .into_iter()
      .collect::<Result<Vec<_>, _>>()
      .unwrap();

    // Narrow down to just the DICOM files
    let mut dicoms = data_files
      .iter()
      .filter(|f| f.path().extension() == Some(OsStr::new("dcm")))
      .map(|f| f.path())
      .collect::<Vec<_>>();
    dicoms.sort();

    // Validate each file
    let validation_results: Vec<_> = dicoms
      .iter()
      .map(|dicom| validate_dicom(dicom).map_err(|e| (dicom, e)))
      .collect();

    // Print results
    if validation_results.iter().all(|r| r.is_ok()) {
      Ok(())
    } else {
      // Report details on failures
      for validation_result in validation_results {
        match validation_result {
          Ok(()) => (),

          Err((dicom, DicomValidationError::LoadError { error })) => {
            error.print(&format!("reading {:?}", dicom));
          }

          Err((dicom, DicomValidationError::PrintedOutputMissing)) => {
            eprintln!("Error: No printed output file for {:?}", dicom);
          }

          Err((dicom, DicomValidationError::PrintedOutputMismatch)) => {
            eprintln!(
              "Error: printed output mismatch with {:?}, compare the two files",
              dicom
            );
          }

          Err((dicom, DicomValidationError::JsonOutputMissing)) => {
            eprintln!("Error: No JSON file for {:?}", dicom);
          }

          Err((dicom, DicomValidationError::JsonOutputMismatch)) => {
            eprintln!(
              "Error: JSON mismatch with {:?}, compare the two files",
              dicom
            );
          }

          Err((dicom, DicomValidationError::RewriteMismatch)) => {
            eprintln!("Error: Rewrite of {:?} was different", dicom);
          }

          Err((dicom, DicomValidationError::JitteredReadError { error })) => {
            error.print(&format!("reading {:?} (jittered)", dicom));
          }

          Err((dicom, DicomValidationError::JitteredReadMismatch)) => {
            eprintln!("Error: Jittered read of {:?} was different", dicom);
          }

          Err((
            dicom,
            DicomValidationError::PixelDataFilterError { error },
          )) => {
            let task_description =
              format!("reading pixel data from {:?}", dicom);

            match error {
              PixelDataFilterError::DataError(error) => {
                error.print(&task_description)
              }
              PixelDataFilterError::P10Error(error) => {
                error.print(&task_description)
              }
            }
          }

          Err((
            dicom,
            DicomValidationError::PixelDataReadError { details },
          )) => {
            eprintln!(
              "Error: Pixel data read of {:?}, details: {}",
              dicom, details
            );
          }
        }
      }

      Err(())
    }
  }

  enum DicomValidationError {
    LoadError { error: P10Error },
    PrintedOutputMissing,
    PrintedOutputMismatch,
    JsonOutputMissing,
    JsonOutputMismatch,
    RewriteMismatch,
    JitteredReadError { error: P10Error },
    JitteredReadMismatch,
    PixelDataFilterError { error: PixelDataFilterError },
    PixelDataReadError { details: String },
  }

  /// Loads a DICOM file and checks that its JSON serialization by this library
  /// matches the expected JSON serialization stored alongside it on disk.
  ///
  fn validate_dicom(dicom: &Path) -> Result<(), DicomValidationError> {
    // Load the DICOM
    let data_set = DataSet::read_p10_file(dicom.to_str().unwrap())
      .map_err(|error| DicomValidationError::LoadError { error })?;

    // Read the expected JSON output from the associated .json file
    let expected_json_string =
      std::fs::read_to_string(format!("{}.json", dicom.to_string_lossy()))
        .map_err(|_| DicomValidationError::JsonOutputMissing)?;
    let expected_json: serde_json::Value =
      serde_json::from_str(&expected_json_string).unwrap();

    // Clean up DICOMs that have string values that consist only of spaces as
    // such values aren't preserved when going through a DICOM JSON rewrite
    // cycle
    let mut json_safe_data_set = data_set.clone();
    for tag in json_safe_data_set.tags() {
      let value = json_safe_data_set.get_value(tag).unwrap();
      if value.get_string().map(|e| e.trim_matches(' ')) == Ok("") {
        json_safe_data_set
          .insert_binary_value(
            tag,
            value.value_representation(),
            Rc::new(Vec::new()),
          )
          .unwrap();
      }
    }

    test_data_set_matches_expected_print_output(dicom, &data_set)?;
    test_data_set_matches_expected_json_output(
      dicom,
      &json_safe_data_set,
      &expected_json,
      false,
    )?;
    test_data_set_matches_expected_json_output(
      dicom,
      &json_safe_data_set,
      &expected_json,
      true,
    )?;
    test_dicom_json_rewrite_cycle(dicom, &expected_json_string)?;
    test_p10_rewrite_cycle(dicom, &data_set)?;

    // Test a read using a chunk size of 15 bytes (this isn't truly jittered as
    // the chunk size is constant)
    test_jittered_read(dicom, &data_set, &mut || 15)?;

    // Test a jittered read with chunk sizes ranging from 1 to 256 bytes
    let mut rng = SmallRng::seed_from_u64(RNG_SEED);
    test_jittered_read(dicom, &data_set, &mut || rng.random_range(1..256))?;

    // Test reading pixel data
    test_pixel_data_read(dicom, &data_set)?;

    Ok(())
  }

  /// Tests that the printed output of the data is as expected.
  ///
  fn test_data_set_matches_expected_print_output(
    dicom: &Path,
    data_set: &DataSet,
  ) -> Result<(), DicomValidationError> {
    let expected_print_output: Vec<_> =
      std::fs::read_to_string(format!("{}.printed", dicom.to_string_lossy()))
        .map_err(|_| DicomValidationError::PrintedOutputMissing)?
        .lines()
        .map(String::from)
        .collect();

    // Print the data set into lines
    let mut print_result = vec![];
    data_set.to_lines(
      &DataSetPrintOptions::new().styled(false).max_width(100),
      &mut |s| print_result.push(s),
    );

    // Compare the actual print output to the expected print output
    if print_result == expected_print_output {
      Ok(())
    } else {
      // The printed output didn't match so write what was generated to a
      // separate file so it can be manually compared to find the discrepancy
      let mut file = File::create(format!(
        "{}.validation_failure.printed",
        dicom.to_string_lossy()
      ))
      .unwrap();

      file.write_all(print_result.join("\n").as_bytes()).unwrap();
      file.flush().unwrap();

      Err(DicomValidationError::PrintedOutputMismatch)
    }
  }

  /// Tests that the JSON conversion of the data set matches the expected JSON
  /// content for the DICOM.
  ///
  fn test_data_set_matches_expected_json_output(
    dicom: &Path,
    data_set: &DataSet,
    expected_json: &serde_json::Value,
    pretty_print: bool,
  ) -> Result<(), DicomValidationError> {
    let json_config = DicomJsonConfig {
      store_encapsulated_pixel_data: true,
      pretty_print,
    };

    // Convert the data set to JSON
    let data_set_json: serde_json::Value =
      serde_json::from_str(&data_set.to_json(json_config).unwrap()).unwrap();

    // Compare the actual JSON to the expected JSON
    if data_set_json == *expected_json {
      Ok(())
    } else {
      // The JSON didn't match so write what was generated to a separate JSON
      // file so it can be manually compared to find the discrepancy
      let data_set_json = data_set_json.to_string();
      let mut file = File::create(format!(
        "{}.validation_failure.json",
        dicom.to_string_lossy()
      ))
      .unwrap();

      file.write_all(data_set_json.as_bytes()).unwrap();
      file.flush().unwrap();

      Err(DicomValidationError::JsonOutputMismatch)
    }
  }

  /// Tests that the conversion of the given DICOM JSON content is unchanged
  /// when converted to a data set and then converted back to DICOM JSON.
  ///
  fn test_dicom_json_rewrite_cycle(
    dicom: &Path,
    expected_json_string: &str,
  ) -> Result<(), DicomValidationError> {
    let original_json: serde_json::Value =
      serde_json::from_str(expected_json_string).unwrap();

    let json_config = DicomJsonConfig {
      store_encapsulated_pixel_data: true,
      pretty_print: false,
    };

    // Check the reverse by converting the expected JSON to a data set then back
    // to JSON and checking it matches the original. This tests the reading of
    // DICOM JSON data into a data set.
    let data_set = DataSet::from_json(expected_json_string).unwrap();
    let data_set_json: serde_json::Value =
      serde_json::from_str(&data_set.to_json(json_config).unwrap()).unwrap();

    // Compare the actual JSON to the expected JSON
    if original_json == data_set_json {
      Ok(())
    } else {
      // The JSON didn't match so write what was generated to a separate JSON
      // file so it can be manually compared to find the discrepancy

      let mut file = File::create(format!(
        "{}.validation_failure.json",
        dicom.to_string_lossy()
      ))
      .unwrap();

      file
        .write_all(data_set_json.to_string().as_bytes())
        .unwrap();
      file.flush().unwrap();

      Err(DicomValidationError::JsonOutputMismatch)
    }
  }

  /// Puts a data set through a full write and read cycle and checks that
  /// nothing changes.
  ///
  fn test_p10_rewrite_cycle(
    dicom: &Path,
    data_set: &DataSet,
  ) -> Result<(), DicomValidationError> {
    let tmp_file = format!("{}.tmp", dicom.to_string_lossy());
    data_set.write_p10_file(&tmp_file, None).unwrap();
    let rewritten_data_set = DataSet::read_p10_file(&tmp_file).unwrap();
    std::fs::remove_file(tmp_file).unwrap();

    // Filter that removes File Meta Information and specific character set data
    // elements which we don't want to be part of the rewrite comparison
    let data_set_filter =
      |(tag, _value): &(&DataElementTag, &DataElementValue)| {
        tag.group != 0x0002 && **tag != dictionary::SPECIFIC_CHARACTER_SET.tag
      };

    let data_set: DataSet = data_set
      .iter()
      .filter(data_set_filter)
      .map(|(tag, value)| (*tag, value.clone()))
      .collect();

    let rewritten_data_set: DataSet = rewritten_data_set
      .iter()
      .filter(data_set_filter)
      .map(|(tag, value)| (*tag, value.clone()))
      .collect();

    if data_set == rewritten_data_set {
      Ok(())
    } else {
      Err(DicomValidationError::RewriteMismatch)
    }
  }

  /// Reads a DICOM in streaming fashion with each chunk of incoming P10 data
  /// being of a random size. This tests that DICOM reading is unaffected by
  /// different input chunk sizes and where the boundaries between chunks fall.
  ///
  fn test_jittered_read(
    dicom: &Path,
    data_set: &DataSet,
    next_chunk_size: &mut impl FnMut() -> usize,
  ) -> Result<(), DicomValidationError> {
    let mut file = File::open(dicom).unwrap();

    let mut context = P10ReadContext::new();
    let mut data_set_builder = DataSetBuilder::new();

    while !data_set_builder.is_complete() {
      match context.read_tokens() {
        Ok(tokens) => {
          for token in tokens {
            data_set_builder.add_token(&token).unwrap()
          }
        }

        Err(P10Error::DataRequired { .. }) => {
          let mut buffer = vec![0u8; next_chunk_size()];

          match file.read(&mut buffer).unwrap() {
            0 => context.write_bytes(vec![], true).unwrap(),

            bytes_count => {
              buffer.resize(bytes_count, 0);
              context.write_bytes(buffer, false).unwrap();
            }
          }
        }

        Err(error) => {
          return Err(DicomValidationError::JitteredReadError { error });
        }
      }
    }

    if *data_set != data_set_builder.final_data_set().unwrap() {
      return Err(DicomValidationError::JitteredReadMismatch);
    }

    Ok(())
  }

  /// Tests reading the frames of pixel data from a data set.
  ///
  fn test_pixel_data_read(
    dicom: &Path,
    data_set: &DataSet,
  ) -> Result<(), DicomValidationError> {
    // If there is no pixel data then there's nothing to test
    if !data_set.has(dictionary::PIXEL_DATA.tag) {
      return Ok(());
    }

    // Create a pixel data renderer for the data set. If this fails then either
    // the DICOM has no pixel data or the pixel data it has isn't supported
    // for reading, so skip further tests.
    let pixel_data_renderer = match PixelDataRenderer::from_data_set(data_set) {
      Ok(renderer) => renderer,
      Err(_) => return Ok(()),
    };

    // Get the raw frames of pixel data
    let mut frames = data_set
      .get_pixel_data_frames()
      .map_err(|e| DicomValidationError::PixelDataFilterError { error: e })?;

    // Test rendering of frames doesn't panic
    for frame in frames.iter_mut() {
      let _ = pixel_data_renderer.render_frame(frame, None);
    }

    // Check that a .pixel_array.json file exists
    let pixel_array_file =
      format!("{}.pixel_array.json", dicom.to_string_lossy());
    if !Path::new(&pixel_array_file).exists() {
      return Ok(());
    }

    // Read the .pixel_array.json file
    let expected_pixel_data_json =
      std::fs::read_to_string(pixel_array_file).unwrap();

    // Check the expected number of frames are present
    let expected_frames: Vec<serde_json::Value> =
      serde_json::from_str(&expected_pixel_data_json).unwrap();
    if frames.len() != expected_frames.len() {
      return Err(DicomValidationError::PixelDataReadError {
        details: format!(
          "Expected {} frames but found {} frames",
          expected_frames.len(),
          frames.len()
        ),
      });
    }

    for (mut frame, expected_frame) in
      frames.into_iter().zip(expected_frames.into_iter())
    {
      let frame_index = frame.index();

      if pixel_data_renderer.definition.is_grayscale() {
        let mut image = pixel_data_renderer
          .render_single_channel_frame(&mut frame)
          .map_err(|e| DicomValidationError::PixelDataFilterError {
            error: PixelDataFilterError::DataError(e),
          })?;

        image.invert_monochrome1_data(&pixel_data_renderer.definition);

        let pixels = image.to_i64_pixels();

        let expected_pixels =
          serde_json::from_value::<Vec<Vec<i64>>>(expected_frame)
            .unwrap()
            .into_iter()
            .flatten();

        for (index, (a, b)) in
          pixels.into_iter().zip(expected_pixels).enumerate()
        {
          if a != b {
            return Err(DicomValidationError::PixelDataReadError {
              details: format!(
                "Pixel data of frame {} is incorrect at index {}, expected {} \
                 but got {}",
                frame_index, index, b, a
              ),
            });
          }
        }
      } else {
        let image = pixel_data_renderer
          .render_color_frame(&mut frame)
          .map_err(|e| DicomValidationError::PixelDataFilterError {
            error: PixelDataFilterError::DataError(e),
          })?;

        let expected_pixels =
          serde_json::from_value::<Vec<Vec<[f64; 3]>>>(expected_frame)
            .unwrap()
            .into_iter()
            .flatten();

        for (index, (a, b)) in image
          .to_rgb_f32_image(&pixel_data_renderer.definition)
          .pixels()
          .zip(expected_pixels)
          .enumerate()
        {
          if (a.0[0] - b[0] as f32).abs() > 0.005
            || (a.0[1] - b[1] as f32).abs() > 0.005
            || (a.0[2] - b[2] as f32).abs() > 0.005
          {
            return Err(DicomValidationError::PixelDataReadError {
              details: format!(
                "Pixel data of frame {} is incorrect at index {}, expected \
                 {:?} but got {:?}",
                frame_index, index, b, a.0
              ),
            });
          }
        }
      }
    }

    Ok(())
  }
}
