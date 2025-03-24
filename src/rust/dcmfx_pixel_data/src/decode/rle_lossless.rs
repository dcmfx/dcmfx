#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use byteorder::ByteOrder;

use dcmfx_core::DataError;

use crate::{
  BitsAllocated, PixelDataDefinition, PixelRepresentation, SingleChannelImage,
  color_image::ColorImage,
  decode::ybr_to_rgb,
  pixel_data_definition::{PhotometricInterpretation, SamplesPerPixel},
};

/// Decodes stored values for RLE Lossless pixel data that uses the
/// [`PhotometricInterpretation::Monochrome1`] or
/// [`PhotometricInterpretation::Monochrome2`] photometric interpretations.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  // Check that there is one sample per pixel
  if definition.samples_per_pixel != SamplesPerPixel::One {
    return Err(DataError::new_value_invalid(
      "Samples per pixel is not one for grayscale pixel data".to_string(),
    ));
  }

  let expected_segment_length =
    if definition.bits_allocated == BitsAllocated::One {
      definition.frame_size_in_bytes()
    } else {
      definition.pixel_count()
    };

  let segments = decode_rle_segments(data, expected_segment_length)?;

  let width = definition.columns;
  let height = definition.rows;
  let pixel_count = definition.pixel_count();
  let bits_allocated = usize::from(definition.bits_allocated);

  match (
    &definition.photometric_interpretation,
    definition.pixel_representation,
    definition.bits_allocated,
    segments.as_slice(),
  ) {
    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      PixelRepresentation::Signed,
      BitsAllocated::One,
      [segment],
    ) => {
      let mut pixels = vec![0i8; pixel_count];

      for i in 0..pixel_count {
        pixels[i] = -(((segment[i / 8] >> (i % 8)) & 1) as i8);
      }

      Ok(SingleChannelImage::new_i8(width, height, pixels).unwrap())
    }

    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      PixelRepresentation::Unsigned,
      BitsAllocated::One,
      [segment],
    ) => {
      let mut pixels = vec![0u8; pixel_count];

      for i in 0..pixel_count {
        pixels[i] = (segment[i / 8] >> (i % 8)) & 1;
      }

      Ok(SingleChannelImage::new_u8(width, height, pixels).unwrap())
    }

    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      PixelRepresentation::Signed,
      BitsAllocated::Eight,
      [segment],
    ) => {
      let mut pixels = vec![0i8; pixel_count];

      for (i, pixel) in segment.iter().enumerate() {
        pixels[i] = i8::from_be_bytes([*pixel]);
      }

      Ok(SingleChannelImage::new_i8(width, height, pixels).unwrap())
    }

    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      PixelRepresentation::Unsigned,
      BitsAllocated::Eight,
      [segment],
    ) => {
      Ok(SingleChannelImage::new_u8(width, height, segment.to_vec()).unwrap())
    }

    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      PixelRepresentation::Signed,
      BitsAllocated::Sixteen,
      [segment_0, segment_1],
    ) => {
      let mut pixels = vec![0i16; pixel_count];

      for i in 0..pixel_count {
        pixels[i] = i16::from_be_bytes([segment_0[i], segment_1[i]]);
      }

      Ok(SingleChannelImage::new_i16(width, height, pixels).unwrap())
    }

    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      PixelRepresentation::Unsigned,
      BitsAllocated::Sixteen,
      [segment_0, segment_1],
    ) => {
      let mut pixels = vec![0u16; pixel_count];

      for i in 0..pixel_count {
        pixels[i] = u16::from_be_bytes([segment_0[i], segment_1[i]]);
      }

      Ok(SingleChannelImage::new_u16(width, height, pixels).unwrap())
    }

    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      PixelRepresentation::Signed,
      BitsAllocated::ThirtyTwo,
      [segment_0, segment_1, segment_2, segment_3],
    ) => {
      let mut pixels = vec![0i32; pixel_count];

      for i in 0..pixel_count {
        pixels[i] = i32::from_be_bytes([
          segment_0[i],
          segment_1[i],
          segment_2[i],
          segment_3[i],
        ]);
      }

      Ok(SingleChannelImage::new_i32(width, height, pixels).unwrap())
    }

    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      PixelRepresentation::Unsigned,
      BitsAllocated::ThirtyTwo,
      [segment_0, segment_1, segment_2, segment_3],
    ) => {
      let mut pixels = vec![0u32; pixel_count];

      for i in 0..pixel_count {
        pixels[i] = u32::from_be_bytes([
          segment_0[i],
          segment_1[i],
          segment_2[i],
          segment_3[i],
        ]);
      }

      Ok(SingleChannelImage::new_u32(width, height, pixels).unwrap())
    }

    _ => Err(DataError::new_value_invalid(format!(
      "RLE Lossless decode not supported with photometric interpretation \
        '{}', bits allocated '{}', segment count '{}'",
      definition.photometric_interpretation,
      bits_allocated,
      segments.len(),
    ))),
  }
}

/// Decodes RLE Lossless color pixel data that uses the
/// [`PhotometricInterpretation::Rgb`] or
/// [`PhotometricInterpretation::YbrFull`] photometric interpretations.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let width = definition.columns;
  let height = definition.rows;
  let pixel_count = definition.pixel_count();

  let segments = decode_rle_segments(data, pixel_count)?;

  match (
    &definition.photometric_interpretation,
    definition.bits_allocated,
    segments.as_slice(),
  ) {
    (
      PhotometricInterpretation::PaletteColor { rgb_luts },
      BitsAllocated::Eight,
      [segment],
    ) => {
      let (red_lut, green_lut, blue_lut) = rgb_luts;

      let mut pixels = vec![0u16; pixel_count * 3];

      for i in 0..pixel_count {
        let index = segment[i] as i64;

        pixels[i * 3] = red_lut.lookup(index);
        pixels[i * 3 + 1] = green_lut.lookup(index);
        pixels[i * 3 + 2] = blue_lut.lookup(index);
      }

      Ok(ColorImage::new_u16(width, height, pixels).unwrap())
    }

    (
      PhotometricInterpretation::PaletteColor { rgb_luts },
      BitsAllocated::Sixteen,
      [segment_0, segment_1],
    ) => {
      let (red_lut, green_lut, blue_lut) = rgb_luts;

      let mut pixels = vec![0u16; pixel_count * 3];

      for i in 0..pixel_count {
        let index = u16::from_be_bytes([segment_0[i], segment_1[i]]) as i64;

        pixels[i * 3] = red_lut.lookup(index);
        pixels[i * 3 + 1] = green_lut.lookup(index);
        pixels[i * 3 + 2] = blue_lut.lookup(index);
      }

      Ok(ColorImage::new_u16(width, height, pixels).unwrap())
    }

    (
      PhotometricInterpretation::Rgb | PhotometricInterpretation::YbrFull,
      BitsAllocated::Eight,
      [red_segment, green_segment, blue_segment],
    ) => {
      let mut pixels = vec![0u8; pixel_count * 3];

      for i in 0..pixel_count {
        pixels[i * 3] = red_segment[i];
        pixels[i * 3 + 1] = green_segment[i];
        pixels[i * 3 + 2] = blue_segment[i];
      }

      if definition.photometric_interpretation.is_ybr() {
        ybr_to_rgb::convert_u8(&mut pixels, definition);
      }

      Ok(ColorImage::new_u8(width, height, pixels).unwrap())
    }

    (
      PhotometricInterpretation::Rgb | PhotometricInterpretation::YbrFull,
      BitsAllocated::Sixteen,
      [
        red_segment_0,
        red_segment_1,
        green_segment_0,
        green_segment_1,
        blue_segment_0,
        blue_segment_1,
      ],
    ) => {
      let mut pixels = vec![0u16; pixel_count * 3];

      for i in 0..pixel_count {
        pixels[i * 3] =
          u16::from_be_bytes([red_segment_0[i], red_segment_1[i]]);
        pixels[i * 3 + 1] =
          u16::from_be_bytes([green_segment_0[i], green_segment_1[i]]);
        pixels[i * 3 + 2] =
          u16::from_be_bytes([blue_segment_0[i], blue_segment_1[i]]);
      }

      if definition.photometric_interpretation.is_ybr() {
        ybr_to_rgb::convert_u16(&mut pixels, definition);
      }

      Ok(ColorImage::new_u16(width, height, pixels).unwrap())
    }

    (
      PhotometricInterpretation::Rgb | PhotometricInterpretation::YbrFull,
      BitsAllocated::ThirtyTwo,
      [
        red_segment_0,
        red_segment_1,
        red_segment_2,
        red_segment_3,
        green_segment_0,
        green_segment_1,
        green_segment_2,
        green_segment_3,
        blue_segment_0,
        blue_segment_1,
        blue_segment_2,
        blue_segment_3,
      ],
    ) => {
      let mut pixels = vec![0u32; pixel_count * 3];

      for i in 0..pixel_count {
        pixels[i * 3] = u32::from_be_bytes([
          red_segment_0[i],
          red_segment_1[i],
          red_segment_2[i],
          red_segment_3[i],
        ]);
        pixels[i * 3 + 1] = u32::from_be_bytes([
          green_segment_0[i],
          green_segment_1[i],
          green_segment_2[i],
          green_segment_3[i],
        ]);
        pixels[i * 3 + 2] = u32::from_be_bytes([
          blue_segment_0[i],
          blue_segment_1[i],
          blue_segment_2[i],
          blue_segment_3[i],
        ]);
      }

      if definition.photometric_interpretation.is_ybr() {
        ybr_to_rgb::convert_u32(&mut pixels, definition);
      }

      Ok(ColorImage::new_u32(width, height, pixels).unwrap())
    }

    _ => Err(DataError::new_value_invalid(format!(
      "Photometric interpretation '{}' is invalid for RLE Lossless color \
       pixel data when bits allocated is {} and there are {} segments",
      definition.photometric_interpretation,
      usize::from(definition.bits_allocated),
      segments.len(),
    ))),
  }
}

/// Decodes all RLE segments defined in RLE Lossless data.
///
/// Ref: PS3.5 G.
///
fn decode_rle_segments(
  data: &[u8],
  expected_length: usize,
) -> Result<Vec<Vec<u8>>, DataError> {
  // Check there is a complete RLE Lossless header
  if data.len() < 64 {
    return Err(DataError::new_value_invalid(
      "RLE Lossless header is incomplete".to_string(),
    ));
  }

  // Read and validate the number of RLE segments
  let number_of_segments =
    u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
  if number_of_segments > 15 {
    return Err(DataError::new_value_invalid(format!(
      "RLE Lossless data segment count '{number_of_segments}' is invalid"
    )));
  }

  // Read the segment offsets
  let mut segment_offsets = vec![0u32; number_of_segments];
  byteorder::LittleEndian::read_u32_into(
    &data[4..(4 + number_of_segments * 4)],
    &mut segment_offsets,
  );

  let mut segments = Vec::with_capacity(number_of_segments);

  // Decode all the segments
  for i in 0..number_of_segments {
    let segment_offset = segment_offsets[i] as usize;

    let next_segment_offset = if i + 1 == number_of_segments {
      data.len()
    } else {
      segment_offsets[i + 1] as usize
    };

    if let Some(rle_data) = data.get(segment_offset..next_segment_offset) {
      match decode_rle_segment(rle_data, expected_length) {
        Ok(segment) => segments.push(segment),
        Err(()) => {
          return Err(DataError::new_value_invalid(format!(
            "RLE Lossless data segment {i} is invalid"
          )));
        }
      }
    } else {
      return Err(DataError::new_value_invalid(format!(
        "RLE Lossless data segment {}'s bounds {}-{} are invalid",
        i, segment_offset, next_segment_offset,
      )));
    }
  }

  Ok(segments)
}

fn decode_rle_segment(
  mut rle_data: &[u8],
  expected_length: usize,
) -> Result<Vec<u8>, ()> {
  let mut result = vec![];

  loop {
    // If the RLE segment is longer than expected then stop decoding
    if result.len() > expected_length {
      return Err(());
    }

    if rle_data.len() < 2 {
      if result.len() == expected_length {
        return Ok(result);
      } else {
        return Err(());
      }
    }

    let n = rle_data[0];

    // Values up to 127 indicate that the next N+1 bytes should be output
    // literally
    if n <= 127 {
      let length = n as usize + 1;

      if let Some(slice) = rle_data.get(1..(1 + length)) {
        result.extend_from_slice(slice);
        rle_data = &rle_data[(1 + length)..];
      } else {
        return Err(());
      }
    }
    // Values greater than 128 indicate that the next byte should be repeated
    // 257 - N times
    else if n > 128 {
      let repeated_byte = rle_data[1];

      let length = 257 - n as usize;
      for _ in 0..length {
        result.push(repeated_byte);
      }

      rle_data = &rle_data[2..];
    }
    // A value of 128 is a no-op and is ignored
    else {
      rle_data = &rle_data[1..];
    }
  }
}
