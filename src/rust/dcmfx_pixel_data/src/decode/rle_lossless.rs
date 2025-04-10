#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use byteorder::ByteOrder;

use dcmfx_core::DataError;

use crate::{
  ColorImage, ColorSpace, SingleChannelImage,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation, SamplesPerPixel,
  },
};

/// Decodes stored values for RLE Lossless pixel data that uses the
/// [`PhotometricInterpretation::Monochrome1`] or
/// [`PhotometricInterpretation::Monochrome2`] photometric interpretations.
///
pub fn decode_single_channel(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  // Check that there is one sample per pixel
  if image_pixel_module.samples_per_pixel() != SamplesPerPixel::One {
    return Err(DataError::new_value_invalid(
      "Samples per pixel is not one for grayscale pixel data".to_string(),
    ));
  }

  let expected_segment_length =
    if image_pixel_module.bits_allocated() == BitsAllocated::One {
      image_pixel_module.frame_size_in_bytes()
    } else {
      image_pixel_module.pixel_count()
    };

  let mut segments = decode_rle_segments(data, expected_segment_length)?;

  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let pixel_count = image_pixel_module.pixel_count();

  match (
    &image_pixel_module.photometric_interpretation(),
    image_pixel_module.pixel_representation(),
    image_pixel_module.bits_allocated(),
    segments.as_slice(),
  ) {
    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      _,
      BitsAllocated::One,
      [_],
    ) => {
      let segment = segments.pop().unwrap();
      let is_signed = image_pixel_module.pixel_representation().is_signed();

      SingleChannelImage::new_bitmap(width, height, segment, is_signed)
    }

    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      PixelRepresentation::Signed,
      BitsAllocated::Eight,
      [_],
    ) => {
      let segment = segments.pop().unwrap();
      let mut pixels = bytemuck::cast_vec(segment);

      if image_pixel_module.has_unused_high_bits() {
        let threshold = 2i8.pow(image_pixel_module.bits_stored() as u32 - 1);

        for pixel in pixels.iter_mut() {
          if *pixel >= threshold {
            *pixel -= threshold;
            *pixel -= threshold;
          }
        }
      }

      SingleChannelImage::new_i8(width, height, pixels)
    }

    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      PixelRepresentation::Unsigned,
      BitsAllocated::Eight,
      [_],
    ) => {
      let pixels = segments.pop().unwrap();
      SingleChannelImage::new_u8(width, height, pixels)
    }

    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      PixelRepresentation::Signed,
      BitsAllocated::Sixteen,
      [segment_0, segment_1],
    ) => {
      let mut pixels = vec![0i16; pixel_count];

      if image_pixel_module.has_unused_high_bits() {
        let threshold = 2i16.pow(image_pixel_module.bits_stored() as u32 - 1);

        for i in 0..pixel_count {
          pixels[i] = i16::from_be_bytes([segment_0[i], segment_1[i]]);

          if pixels[i] >= threshold {
            pixels[i] -= threshold;
            pixels[i] -= threshold;
          }
        }
      } else {
        for i in 0..pixel_count {
          pixels[i] = i16::from_be_bytes([segment_0[i], segment_1[i]]);
        }
      }

      SingleChannelImage::new_i16(width, height, pixels)
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

      SingleChannelImage::new_u16(width, height, pixels)
    }

    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      PixelRepresentation::Signed,
      BitsAllocated::ThirtyTwo,
      [segment_0, segment_1, segment_2, segment_3],
    ) => {
      let mut pixels = vec![0i32; pixel_count];

      if image_pixel_module.has_unused_high_bits() {
        let threshold = 2i32.pow(image_pixel_module.bits_stored() as u32 - 1);

        for i in 0..pixel_count {
          pixels[i] = i32::from_be_bytes([
            segment_0[i],
            segment_1[i],
            segment_2[i],
            segment_3[i],
          ]);

          if pixels[i] >= threshold {
            pixels[i] -= threshold;
            pixels[i] -= threshold;
          }
        }
      } else {
        for i in 0..pixel_count {
          pixels[i] = i32::from_be_bytes([
            segment_0[i],
            segment_1[i],
            segment_2[i],
            segment_3[i],
          ]);
        }
      }

      SingleChannelImage::new_i32(width, height, pixels)
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

      SingleChannelImage::new_u32(width, height, pixels)
    }

    _ => Err(DataError::new_value_invalid(format!(
      "RLE Lossless decode not supported with photometric interpretation '{}', \
       bits allocated '{}', segment count '{}'",
      image_pixel_module.photometric_interpretation(),
      u8::from(image_pixel_module.bits_allocated()),
      segments.len(),
    ))),
  }
}

/// Decodes RLE Lossless color pixel data that uses the
/// [`PhotometricInterpretation::Rgb`] or
/// [`PhotometricInterpretation::YbrFull`] photometric interpretations.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let pixel_count = image_pixel_module.pixel_count();

  let color_space = if image_pixel_module.photometric_interpretation().is_ybr()
  {
    ColorSpace::YBR
  } else {
    ColorSpace::RGB
  };

  let mut segments = decode_rle_segments(data, pixel_count)?;

  match (
    &image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
    segments.as_slice(),
  ) {
    (
      PhotometricInterpretation::PaletteColor { palette },
      BitsAllocated::Eight,
      [_],
    ) => {
      let data = segments.pop().unwrap();
      ColorImage::new_palette8(width, height, data, palette.clone())
    }

    (
      PhotometricInterpretation::PaletteColor { palette },
      BitsAllocated::Sixteen,
      [segment_0, segment_1],
    ) => {
      let mut pixels = vec![0u16; pixel_count * 3];

      for i in 0..pixel_count {
        pixels.push(u16::from_be_bytes([segment_0[i], segment_1[i]]));
      }

      ColorImage::new_palette16(width, height, pixels, palette.clone())
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

      ColorImage::new_u8(width, height, pixels, color_space)
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

      ColorImage::new_u16(width, height, pixels, color_space)
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

      ColorImage::new_u32(width, height, pixels, color_space)
    }

    _ => Err(DataError::new_value_invalid(format!(
      "Photometric interpretation '{}' is invalid for RLE Lossless color \
       pixel data when bits allocated is {} and there are {} segments",
      image_pixel_module.photometric_interpretation(),
      u8::from(image_pixel_module.bits_allocated()),
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
  let mut result = Vec::with_capacity(expected_length);

  loop {
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
      let length = usize::from(n) + 1;

      if let Some(slice) = rle_data.get(1..(1 + length)) {
        // Check expected length won't be exceeded
        if result.len() + length > expected_length {
          return Err(());
        }

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

      let length = 257 - usize::from(n);

      // Check expected length won't be exceeded
      if result.len() + length > expected_length {
        return Err(());
      }

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

#[cfg(test)]
mod tests {
  use super::*;

  use crate::encode::rle_lossless::encode_segments;

  #[test]
  pub fn decode_monochrome_8_bit_unsigned() {
    let image_pixel_module = ImagePixelModule::new_basic(
      SamplesPerPixel::One,
      PhotometricInterpretation::Monochrome2,
      2,
      2,
      BitsAllocated::Eight,
      8,
      PixelRepresentation::Unsigned,
    )
    .unwrap();

    let data = encode_segments(&[&[0, 1, 2, 3]], 2).unwrap();

    assert_eq!(
      decode_single_channel(&image_pixel_module, &data),
      SingleChannelImage::new_u8(2, 2, vec![0, 1, 2, 3])
    );
  }

  #[test]
  fn decode_monochrome_8_bit_signed_with_7_bits_stored() {
    let image_pixel_module = ImagePixelModule::new_basic(
      SamplesPerPixel::One,
      PhotometricInterpretation::Monochrome2,
      8,
      16,
      BitsAllocated::Eight,
      7,
      PixelRepresentation::Signed,
    )
    .unwrap();

    let data: Vec<_> = (0..=127).collect();
    let data = encode_segments(&[&data], 8).unwrap();

    assert_eq!(
      decode_single_channel(&image_pixel_module, &data),
      SingleChannelImage::new_i8(16, 8, (0..64).chain(-64..0).collect())
    );
  }

  #[test]
  fn decode_monochrome_16_bit_signed_with_12_bits_stored() {
    let image_pixel_module = ImagePixelModule::new_basic(
      SamplesPerPixel::One,
      PhotometricInterpretation::Monochrome2,
      64,
      64,
      BitsAllocated::Sixteen,
      12,
      PixelRepresentation::Signed,
    )
    .unwrap();

    let segment_0: Vec<_> = (0..4096u16).map(|i| i.to_be_bytes()[0]).collect();
    let segment_1: Vec<_> = (0..4096u16).map(|i| i.to_be_bytes()[1]).collect();
    let data = encode_segments(&[&segment_0, &segment_1], 64).unwrap();

    assert_eq!(
      decode_single_channel(&image_pixel_module, &data),
      SingleChannelImage::new_i16(64, 64, (0..2048).chain(-2048..0).collect())
    );
  }
}
