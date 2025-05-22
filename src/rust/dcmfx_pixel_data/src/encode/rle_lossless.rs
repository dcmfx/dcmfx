#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::ToString, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataEncodeError,
  color_image::ColorImageData,
  iods::image_pixel_module::{
    ImagePixelModule, PhotometricInterpretation, PlanarConfiguration,
  },
  monochrome_image::MonochromeImageData,
};

/// Returns the Image Pixel Module resulting from encoding as RLE Lossless pixel
/// data.
///
pub fn encode_image_pixel_module(
  mut image_pixel_module: ImagePixelModule,
) -> Result<ImagePixelModule, ()> {
  match image_pixel_module.photometric_interpretation() {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2
    | PhotometricInterpretation::PaletteColor { .. }
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull => (),

    _ => return Err(()),
  }

  image_pixel_module.set_planar_configuration(PlanarConfiguration::Separate);

  Ok(image_pixel_module)
}

/// Encodes a [`MonochromeImage`] into RLE Lossless raw bytes.
///
pub fn encode_monochrome(
  image: &MonochromeImage,
  image_pixel_module: &ImagePixelModule,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let pixel_count = image.pixel_count();
  let row_size = usize::from(image.width());

  match (
    image.data(),
    image.bits_stored(),
    image_pixel_module.photometric_interpretation(),
  ) {
    (
      MonochromeImageData::Bitmap { data, .. },
      _,
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
    ) => {
      let segment_0 = data.to_vec();

      let row_size = if image.width() % 8 == 0 {
        usize::from(image.width() / 8)
      } else {
        data.len()
      };

      encode_segments(&[segment_0], row_size)
    }

    (
      MonochromeImageData::I8(data),
      8,
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
    ) => {
      let segment_0 = bytemuck::cast_slice(data).to_vec();

      encode_segments(&[segment_0], row_size)
    }

    (
      MonochromeImageData::I8(data),
      _,
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
    ) => {
      let mut segment_0 = vec![0; pixel_count];

      let mask = (1 << image.bits_stored()) - 1;

      for i in 0..pixel_count {
        segment_0[i] = (i16::from(data[i]) & mask) as u8;
      }

      encode_segments(&[segment_0], row_size)
    }

    (
      MonochromeImageData::U8(data),
      _,
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
    ) => {
      let segment_0 = data.to_vec();

      encode_segments(&[segment_0], row_size)
    }

    (
      MonochromeImageData::I16(data),
      16,
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
    ) => {
      let mut segment_0 = vec![0; pixel_count];
      let mut segment_1 = vec![0; pixel_count];

      for i in 0..pixel_count {
        let [a, b] = data[i].to_be_bytes();

        segment_0[i] = a;
        segment_1[i] = b;
      }

      encode_segments(&[segment_0, segment_1], row_size)
    }

    (
      MonochromeImageData::I16(data),
      _,
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
    ) => {
      let mut segment_0 = vec![0; pixel_count];
      let mut segment_1 = vec![0; pixel_count];

      let mask = (1 << image.bits_stored()) - 1;

      for i in 0..pixel_count {
        let [a, b] = ((i32::from(data[i]) & mask) as u16).to_be_bytes();

        segment_0[i] = a;
        segment_1[i] = b;
      }

      encode_segments(&[segment_0, segment_1], row_size)
    }

    (
      MonochromeImageData::U16(data),
      _,
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
    ) => {
      let mut segment_0 = vec![0; pixel_count];
      let mut segment_1 = vec![0; pixel_count];

      for i in 0..pixel_count {
        let [a, b] = data[i].to_be_bytes();

        segment_0[i] = a;
        segment_1[i] = b;
      }

      encode_segments(&[segment_0, segment_1], row_size)
    }

    (
      MonochromeImageData::I32(data),
      32,
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
    ) => {
      let mut segment_0 = vec![0; pixel_count];
      let mut segment_1 = vec![0; pixel_count];
      let mut segment_2 = vec![0; pixel_count];
      let mut segment_3 = vec![0; pixel_count];

      for i in 0..pixel_count {
        let [a, b, c, d] = data[i].to_be_bytes();

        segment_0[i] = a;
        segment_1[i] = b;
        segment_2[i] = c;
        segment_3[i] = d;
      }

      encode_segments(&[segment_0, segment_1, segment_2, segment_3], row_size)
    }

    (
      MonochromeImageData::I32(data),
      _,
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
    ) => {
      let mut segment_0 = vec![0; pixel_count];
      let mut segment_1 = vec![0; pixel_count];
      let mut segment_2 = vec![0; pixel_count];
      let mut segment_3 = vec![0; pixel_count];

      let mask = (1 << image.bits_stored()) - 1;

      for i in 0..pixel_count {
        let [a, b, c, d] = ((i64::from(data[i]) & mask) as u32).to_be_bytes();

        segment_0[i] = a;
        segment_1[i] = b;
        segment_2[i] = c;
        segment_3[i] = d;
      }

      encode_segments(&[segment_0, segment_1, segment_2, segment_3], row_size)
    }

    (
      MonochromeImageData::U32(data),
      _,
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
    ) => {
      let mut segment_0 = vec![0; pixel_count];
      let mut segment_1 = vec![0; pixel_count];
      let mut segment_2 = vec![0; pixel_count];
      let mut segment_3 = vec![0; pixel_count];

      for (i, pixel) in data.iter().enumerate().take(pixel_count) {
        let [a, b, c, d] = pixel.to_be_bytes();

        segment_0[i] = a;
        segment_1[i] = b;
        segment_2[i] = c;
        segment_3[i] = d;
      }

      encode_segments(&[segment_0, segment_1, segment_2, segment_3], row_size)
    }

    _ => Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: None,
    }),
  }
}

/// Encodes a [`MonochromeImage`] into RLE Lossless raw bytes.
///
pub fn encode_color(
  image: &ColorImage,
  image_pixel_module: &ImagePixelModule,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let row_size = usize::from(image.width());
  let pixel_count = image.pixel_count();

  match (
    image.data(),
    image_pixel_module.photometric_interpretation(),
  ) {
    (
      ColorImageData::PaletteU8 { data, .. },
      PhotometricInterpretation::PaletteColor { .. },
    ) => {
      let segment_0 = data.to_vec();

      encode_segments(&[segment_0], row_size)
    }

    (
      ColorImageData::PaletteU16 { data, .. },
      PhotometricInterpretation::PaletteColor { .. },
    ) => {
      let mut segment_0 = vec![0; pixel_count];
      let mut segment_1 = vec![0; pixel_count];

      for i in 0..pixel_count {
        let [a, b] = data[i].to_be_bytes();

        segment_0[i] = a;
        segment_1[i] = b;
      }

      encode_segments(&[segment_0, segment_1], row_size)
    }

    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
    )
    | (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PhotometricInterpretation::YbrFull,
    ) => {
      let mut segment_0 = vec![0; pixel_count];
      let mut segment_1 = vec![0; pixel_count];
      let mut segment_2 = vec![0; pixel_count];

      for i in 0..pixel_count {
        segment_0[i] = data[i * 3];
        segment_1[i] = data[i * 3 + 1];
        segment_2[i] = data[i * 3 + 2];
      }

      encode_segments(&[segment_0, segment_1, segment_2], row_size)
    }

    (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
    )
    | (
      ColorImageData::U16 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PhotometricInterpretation::YbrFull,
    ) => {
      let mut segment_0 = vec![0; pixel_count];
      let mut segment_1 = vec![0; pixel_count];
      let mut segment_2 = vec![0; pixel_count];
      let mut segment_3 = vec![0; pixel_count];
      let mut segment_4 = vec![0; pixel_count];
      let mut segment_5 = vec![0; pixel_count];

      for i in 0..pixel_count {
        let [a, b] = data[i * 3].to_be_bytes();
        segment_0[i] = a;
        segment_1[i] = b;

        let [a, b] = data[i * 3 + 1].to_be_bytes();
        segment_2[i] = a;
        segment_3[i] = b;

        let [a, b] = data[i * 3 + 2].to_be_bytes();
        segment_4[i] = a;
        segment_5[i] = b;
      }

      encode_segments(
        &[
          segment_0, segment_1, segment_2, segment_3, segment_4, segment_5,
        ],
        row_size,
      )
    }

    (
      ColorImageData::U32 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
    )
    | (
      ColorImageData::U32 {
        data,
        color_space: ColorSpace::Ybr { is_422: false },
      },
      PhotometricInterpretation::YbrFull,
    ) => {
      let mut segment_0 = vec![0; pixel_count];
      let mut segment_1 = vec![0; pixel_count];
      let mut segment_2 = vec![0; pixel_count];
      let mut segment_3 = vec![0; pixel_count];
      let mut segment_4 = vec![0; pixel_count];
      let mut segment_5 = vec![0; pixel_count];
      let mut segment_6 = vec![0; pixel_count];
      let mut segment_7 = vec![0; pixel_count];
      let mut segment_8 = vec![0; pixel_count];
      let mut segment_9 = vec![0; pixel_count];
      let mut segment_10 = vec![0; pixel_count];
      let mut segment_11 = vec![0; pixel_count];

      for i in 0..pixel_count {
        let [a, b, c, d] = data[i * 3].to_be_bytes();
        segment_0[i] = a;
        segment_1[i] = b;
        segment_2[i] = c;
        segment_3[i] = d;

        let [a, b, c, d] = data[i * 3 + 1].to_be_bytes();
        segment_4[i] = a;
        segment_5[i] = b;
        segment_6[i] = c;
        segment_7[i] = d;

        let [a, b, c, d] = data[i * 3 + 2].to_be_bytes();
        segment_8[i] = a;
        segment_9[i] = b;
        segment_10[i] = c;
        segment_11[i] = d;
      }

      encode_segments(
        &[
          segment_0, segment_1, segment_2, segment_3, segment_4, segment_5,
          segment_6, segment_7, segment_8, segment_9, segment_10, segment_11,
        ],
        row_size,
      )
    }

    _ => Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: Some(image.color_space()),
    }),
  }
}

/// RLE encodes the data for a set of segments where each row of the image data
/// has the given size.
///
/// The returned data includes the RLE Lossless header that specifies the number
/// of segments and their size.
///
/// Ref: PS3.5 G.3.1, PS3.5 G.4, PS3.5 G.5.
///
#[allow(dead_code, clippy::result_unit_err)]
fn encode_segments(
  segments: &[Vec<u8>],
  row_size: usize,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  // The maximum number of segments allowed by RLE Lossless is 15
  if segments.len() > 15 {
    return Err(PixelDataEncodeError::OtherError {
      name: "RLE Lossless encode failed".to_string(),
      details: "Segment count exceeds 15".to_string(),
    });
  }

  let mut encoded_segments = Vec::with_capacity(segments.len());
  let mut total_segment_length = 0;

  // RLE encode all segments
  for segment in segments {
    let data = encode_segment(segment, row_size)?;

    total_segment_length += data.len();
    encoded_segments.push(data);
  }

  // Check total output size doesn't exceed a u32
  if 64 + total_segment_length > u32::MAX as usize {
    return Err(PixelDataEncodeError::OtherError {
      name: "RLE Lossless encode failed".to_string(),
      details: "Segment data is too long".to_string(),
    });
  }

  let mut output_length = 64 + total_segment_length;
  if output_length % 2 == 1 {
    output_length += 1;
  }

  let mut output: Vec<u8> = Vec::with_capacity(output_length);

  // Append number of segments
  output.extend_from_slice(&(encoded_segments.len() as u32).to_le_bytes());

  // Append segment offsets
  let mut offset = 64;
  for segment in encoded_segments.iter() {
    output.extend_from_slice(&(offset as u32).to_le_bytes());
    offset += segment.len();
  }

  // Pad header to 64 bytes
  output.resize(64, 0);

  // Append encoded segment data
  for segment in encoded_segments.iter() {
    output.extend_from_slice(segment);
  }

  // Ensure even length
  if output.len() % 2 == 1 {
    output.push(0)
  }

  Ok(output)
}

/// RLE encodes the data for a single segment where each row of the image data
/// has the given size.
///
/// The row size is needed because each row is RLE encoded separately and then
/// concatenated.
///
/// Ref: PS3.5 G.3.1.
///
#[allow(clippy::result_unit_err)]
fn encode_segment(
  data: &[u8],
  row_size: usize,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let mut output = Vec::with_capacity(data.len());

  if row_size == 0 || data.len() % row_size != 0 {
    return Err(PixelDataEncodeError::OtherError {
      name: "RLE Lossless encode failed".to_string(),
      details: "Segment data length is not a multiple of the row size"
        .to_string(),
    });
  }

  for row in data.chunks_exact(row_size) {
    encode_row(row, &mut output)
  }

  output.shrink_to_fit();

  Ok(output)
}

/// RLE encodes the data for a single row.
///
#[allow(clippy::result_unit_err)]
fn encode_row(mut data: &[u8], output: &mut Vec<u8>) {
  while !data.is_empty() {
    let first_byte = data[0];

    let mut run_length = 1;
    let max_run_length = data.len().min(128);

    // Count how many times this byte repeats, up to a maximum of 128 which is
    // the most that can be encoded in a single run
    while run_length < max_run_length && first_byte == data[run_length] {
      run_length += 1;
    }

    // If there are repeated bytes then encode as a replicate run
    if run_length > 1 {
      output.push((257 - run_length) as u8);
      output.push(first_byte);

      data = &data[run_length..];

      continue;
    }

    // Otherwise, encode as a literal run
    let mut length = 1;
    while length < max_run_length {
      // Check if a new replicate run of three or more bytes starts here
      if data[length] == data[length - 1]
        && length + 1 < data.len()
        && data[length] == data[length + 1]
      {
        length -= 1;
        break;
      }

      length += 1;
    }

    output.push((length - 1) as u8);
    output.extend_from_slice(&data[..length]);

    data = &data[length..];
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_input() {
    assert_eq!(encode_segment(&[], 1), Ok(vec![]),);
  }

  #[test]
  fn single_byte() {
    assert_eq!(encode_segment(&[42], 1), Ok(vec![0, 42]),);
  }

  #[test]
  fn all_unique_bytes() {
    assert_eq!(
      encode_segment(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 10),
      Ok(vec![9, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
    );
  }

  #[test]
  fn all_repeating_bytes() {
    assert_eq!(encode_segment(&[5, 5, 5, 5], 4), Ok(vec![253, 5]));
  }

  #[test]
  fn mixed_repeated_and_unique_bytes() {
    assert_eq!(
      encode_segment(&[1, 2, 3, 4, 5, 5, 5, 6, 7, 8, 8, 9], 12),
      Ok(vec![3, 1, 2, 3, 4, 254, 5, 4, 6, 7, 8, 8, 9]),
    );
  }

  #[test]
  fn maximum_rle_run_length() {
    assert_eq!(encode_segment(&[99; 129], 129), Ok(vec![129, 99, 0, 99]));
  }

  #[test]
  fn two_byte_repeat_in_literal_run() {
    assert_eq!(
      encode_segment(&[1, 2, 3, 3, 4, 5, 6], 7),
      Ok(vec![6, 1, 2, 3, 3, 4, 5, 6])
    );
  }

  #[test]
  fn odd_length_segment_adds_padding_byte() {
    assert_eq!(
      encode_segments(&[vec![1, 2]], 2),
      Ok(vec![
        1, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 0
      ])
    );
  }
}
