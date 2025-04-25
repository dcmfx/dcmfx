#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use crate::{
  ColorImage, PixelDataEncodeError, SingleChannelImage,
  color_image::ColorImageData,
  iods::image_pixel_module::{ImagePixelModule, PhotometricInterpretation},
  single_channel_image::SingleChannelImageData,
};

/// Encodes a [`SingleChannelImage`] into RLE Lossless raw bytes.
///
pub fn encode_single_channel(
  image: &SingleChannelImage,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let mut segments: Vec<Vec<u8>> = vec![];
  let mut row_size = usize::from(image.width());

  match (image.data(), image.bits_stored()) {
    (SingleChannelImageData::Bitmap { data, .. }, _) => {
      segments.push(data.to_vec());
      row_size = if image.width() % 8 == 0 {
        usize::from(image.width() / 8)
      } else {
        data.len()
      };
    }

    (SingleChannelImageData::I8(data), 8) => {
      segments.push(bytemuck::cast_slice(data).to_vec());
    }

    (SingleChannelImageData::I8(data), _) => {
      segments.push(vec![0; image.pixel_count()]);

      let mask = (1 << image.bits_stored()) - 1;

      for (i, pixel) in data.iter().enumerate().take(image.pixel_count()) {
        segments[0][i] = (i16::from(*pixel) & mask) as u8;
      }
    }

    (SingleChannelImageData::U8(data), _) => {
      segments.push(data.to_vec());
    }

    (SingleChannelImageData::I16(data), 16) => {
      for _ in 0..2 {
        segments.push(vec![0; image.pixel_count()]);
      }

      for (i, pixel) in data.iter().enumerate().take(image.pixel_count()) {
        let [a, b] = pixel.to_be_bytes();

        segments[0][i] = a;
        segments[1][i] = b;
      }
    }

    (SingleChannelImageData::I16(data), _) => {
      for _ in 0..2 {
        segments.push(vec![0; image.pixel_count()]);
      }

      let mask = (1 << image.bits_stored()) - 1;

      for (i, pixel) in data.iter().enumerate().take(image.pixel_count()) {
        let [a, b] = ((i32::from(*pixel) & mask) as u16).to_be_bytes();

        segments[0][i] = a;
        segments[1][i] = b;
      }
    }

    (SingleChannelImageData::U16(data), _) => {
      for _ in 0..2 {
        segments.push(vec![0; image.pixel_count()]);
      }

      for (i, pixel) in data.iter().enumerate().take(image.pixel_count()) {
        let [a, b] = pixel.to_be_bytes();

        segments[0][i] = a;
        segments[1][i] = b;
      }
    }

    (SingleChannelImageData::I32(data), 32) => {
      for _ in 0..4 {
        segments.push(vec![0; image.pixel_count()]);
      }

      for (i, pixel) in data.iter().enumerate().take(image.pixel_count()) {
        let [a, b, c, d] = pixel.to_be_bytes();

        segments[0][i] = a;
        segments[1][i] = b;
        segments[2][i] = c;
        segments[3][i] = d;
      }
    }

    (SingleChannelImageData::I32(data), _) => {
      for _ in 0..4 {
        segments.push(vec![0; image.pixel_count()]);
      }

      let mask = (1 << image.bits_stored()) - 1;

      for (i, pixel) in data.iter().enumerate().take(image.pixel_count()) {
        let [a, b, c, d] = ((i64::from(*pixel) & mask) as u32).to_be_bytes();

        segments[0][i] = a;
        segments[1][i] = b;
        segments[2][i] = c;
        segments[3][i] = d;
      }
    }

    (SingleChannelImageData::U32(data), _) => {
      for _ in 0..4 {
        segments.push(vec![0; image.pixel_count()]);
      }

      for (i, pixel) in data.iter().enumerate().take(image.pixel_count()) {
        let [a, b, c, d] = pixel.to_be_bytes();

        segments[0][i] = a;
        segments[1][i] = b;
        segments[2][i] = c;
        segments[3][i] = d;
      }
    }
  }

  encode_segments(segments, row_size).map_err(|e| {
    PixelDataEncodeError::OtherError {
      details: e.to_string(),
    }
  })
}

/// Encodes a [`SingleChannelImage`] into RLE Lossless raw bytes.
///
pub fn encode_color(
  image: &ColorImage,
  image_pixel_module: &ImagePixelModule,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let mut segments: Vec<Vec<u8>> = vec![];

  let photometric_interpretation =
    image_pixel_module.photometric_interpretation();

  match photometric_interpretation {
    PhotometricInterpretation::PaletteColor { .. }
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull => match image.data() {
      ColorImageData::U8 { data, .. } => {
        for _ in 0..3 {
          segments.push(vec![0; image.pixel_count()]);
        }

        for i in 0..image.pixel_count() {
          segments[0][i] = data[i * 3];
          segments[1][i] = data[i * 3 + 1];
          segments[2][i] = data[i * 3 + 2];
        }
      }

      ColorImageData::U16 { data, .. } => {
        for _ in 0..6 {
          segments.push(vec![0; image.pixel_count()]);
        }

        for i in 0..image.pixel_count() {
          let [a, b] = data[i * 3].to_be_bytes();
          segments[0][i] = a;
          segments[1][i] = b;

          let [a, b] = data[i * 3 + 1].to_be_bytes();
          segments[2][i] = a;
          segments[3][i] = b;

          let [a, b] = data[i * 3 + 2].to_be_bytes();
          segments[4][i] = a;
          segments[5][i] = b;
        }
      }

      ColorImageData::U32 { data, .. } => {
        for _ in 0..12 {
          segments.push(vec![0; image.pixel_count()]);
        }

        for i in 0..image.pixel_count() {
          let [a, b, c, d] = data[i * 3].to_be_bytes();
          segments[0][i] = a;
          segments[1][i] = b;
          segments[2][i] = c;
          segments[3][i] = d;

          let [a, b, c, d] = data[i * 3 + 1].to_be_bytes();
          segments[4][i] = a;
          segments[5][i] = b;
          segments[6][i] = c;
          segments[7][i] = d;

          let [a, b, c, d] = data[i * 3 + 2].to_be_bytes();
          segments[8][i] = a;
          segments[9][i] = b;
          segments[10][i] = c;
          segments[11][i] = d;
        }
      }

      ColorImageData::PaletteU8 { data, .. } => {
        segments.push(data.to_vec());
      }

      ColorImageData::PaletteU16 { data, .. } => {
        for _ in 0..2 {
          segments.push(vec![0; image.pixel_count()]);
        }

        for (i, pixel) in data.iter().enumerate().take(image.pixel_count()) {
          let [a, b] = pixel.to_be_bytes();

          segments[0][i] = a;
          segments[1][i] = b;
        }
      }
    },

    _ => {
      return Err(PixelDataEncodeError::NotSupported {
        details: format!(
          "Photometric interpretation '{}' is not able to be encoded into \
           RLE Lossless pixel data",
          photometric_interpretation
        ),
      });
    }
  }

  encode_segments(segments, usize::from(image.width())).map_err(|e| {
    PixelDataEncodeError::OtherError {
      details: e.to_string(),
    }
  })
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
  segments: Vec<Vec<u8>>,
  row_size: usize,
) -> Result<Vec<u8>, &'static str> {
  // The maximum number of segments allowed by RLE Lossless is 15
  if segments.len() > 15 {
    return Err("RLE Lossless segment count exceeds 15");
  }

  let mut encoded_segments = Vec::with_capacity(segments.len());
  let mut total_segment_length = 0;

  // RLE encode all segments
  for segment in segments.iter() {
    let mut data: Vec<u8> = encode_segment(segment, row_size)?;

    // Ensure encoded segment has even length
    if data.len() % 2 == 1 {
      data.push(0)
    }

    total_segment_length += data.len();
    encoded_segments.push(data);
  }

  // Check total output size doesn't exceed a u32
  if 64 + total_segment_length > u32::MAX as usize {
    return Err("RLE Lossless segment data is too long");
  }

  let mut output: Vec<u8> = Vec::with_capacity(64 + total_segment_length);

  // Append number of segments
  output.extend_from_slice(&(segments.len() as u32).to_le_bytes());

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
) -> Result<Vec<u8>, &'static str> {
  let mut output = vec![];

  if row_size == 0 || data.len() % row_size != 0 {
    return Err("RLE Lossless segment data is not a multiple of the row size");
  }

  for row in data.chunks_exact(row_size) {
    encode_row(row, &mut output)
  }

  Ok(output)
}

/// RLE encodes the data for a single row.
///
#[allow(clippy::result_unit_err)]
fn encode_row(mut data: &[u8], output: &mut Vec<u8>) {
  while !data.is_empty() {
    let mut run_length = 1;

    // See how many times this byte repeats, up to a maximum of 128 which is
    // all the can be encoded in a single run
    while run_length < data.len()
      && data[0] == data[run_length]
      && run_length < 128
    {
      run_length += 1;
    }

    // If there are repeats then encode as a Replicate Run
    if run_length > 1 {
      output.push((257 - run_length) as u8);
      output.push(data[0]);

      data = &data[run_length..];
    }
    // Otherwise encode as a Literal Run
    else {
      let mut length = 1;

      while length < data.len()
        && data[length] != data[length - 1]
        && length < 128
      {
        length += 1;
      }

      // If the last byte in this Literal Run could be part of an immediately
      // following Replicate Run then don't include its final byte
      if length + 1 < data.len() && data[length - 1] == data[length] {
        length -= 1;
      }

      output.push((length - 1) as u8);
      output.extend_from_slice(&data[0..length]);

      data = &data[length..];
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_empty_input() {
    assert_eq!(encode_segment(&[], 1), Ok(vec![]),);
  }

  #[test]
  fn test_single_byte() {
    assert_eq!(encode_segment(&[42], 1), Ok(vec![0, 42]),);
  }

  #[test]
  fn test_all_unique_bytes() {
    assert_eq!(
      encode_segment(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 10),
      Ok(vec![9, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
    );
  }

  #[test]
  fn test_all_repeating_bytes() {
    assert_eq!(encode_segment(&[5, 5, 5, 5], 4), Ok(vec![253, 5]));
  }

  #[test]
  fn test_mixed_repeated_and_unique_bytes() {
    assert_eq!(
      encode_segment(&[1, 2, 3, 4, 5, 5, 5, 6, 7, 8, 8, 9], 12),
      Ok(vec![3, 1, 2, 3, 4, 254, 5, 1, 6, 7, 255, 8, 0, 9]),
    );
  }

  #[test]
  fn test_maximum_rle_run_length() {
    assert_eq!(encode_segment(&[99; 129], 129), Ok(vec![129, 99, 0, 99]));
  }
}
