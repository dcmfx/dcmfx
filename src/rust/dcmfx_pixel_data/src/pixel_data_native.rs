use dcmfx_core::DataError;

use crate::{
  luts::LookupTable,
  pixel_data_definition::{
    BitsAllocated, PhotometricInterpretation, PlanarConfiguration,
    SamplesPerPixel,
  },
  PixelDataDefinition, RgbColor,
};

/// Creates an iterator for the pixels of native grayscale pixel data that uses
/// the [`PhotometricInterpretation::Monochrome1`] or
/// [`PhotometricInterpretation::Monochrome2`] photometric interpretations. The
/// iterator returns integer grayscale pixel values which may be signed or
/// unsigned. Such values are typically passed through a Modality LUT and/or a
/// VOI LUT to get a final display value.
///
/// Note that the iterator returns raw stored values, meaning that if the
/// photometric interpretation is [`PhotometricInterpretation::Monochrome1`]
/// then the values are inverted.
///
pub fn iter_pixels_grayscale<'a>(
  definition: PixelDataDefinition,
  data: &'a [u8],
) -> Result<Box<dyn Iterator<Item = i64> + 'a>, DataError> {
  // Check that there is one sample per pixel
  if definition.samples_per_pixel != SamplesPerPixel::One {
    return Err(DataError::new_value_invalid(
      "Samples per pixel is not one for grayscale pixel data".to_string(),
    ));
  }

  validate_definition_and_data(&definition, data)?;

  let pixel_count = definition.pixel_count();
  let bits_allocated = usize::from(definition.bits_allocated);

  match definition.photometric_interpretation {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2 => {
      Ok(Box::new(NativePixelDataIteratorGrayscale {
        definition,
        pixel_count,
        data,
        index: 0,
        offset: 0,
        stride: bits_allocated,
      }))
    }

    _ => Err(DataError::new_value_invalid(format!(
      "Photometric interpretation '{:?}' is invalid for grayscale pixel data \
       when samples per pixel is one",
      definition.photometric_interpretation
    ))),
  }
}

/// Creates an iterator for the pixels of native color pixel data that uses the
/// [`PhotometricInterpretation::Rgb`], [`PhotometricInterpretation::YbrFull`],
/// [`PhotometricInterpretation::YbrFull422`], or
/// [`PhotometricInterpretation::PaletteColor`] photometric interpretations. The
/// iterator returns RGB pixel values in the range 0-1. YBR colors are converted
/// to RGB.
///
pub fn iter_pixels_color<'a>(
  definition: PixelDataDefinition,
  data: &'a [u8],
) -> Result<Box<dyn Iterator<Item = RgbColor> + 'a>, DataError> {
  validate_definition_and_data(&definition, data)?;

  let pixel_count = definition.pixel_count();
  let bits_allocated = usize::from(definition.bits_allocated);

  match definition.samples_per_pixel {
    SamplesPerPixel::One => match definition.photometric_interpretation {
      PhotometricInterpretation::PaletteColor { ref rgb_luts } => {
        let rgb_luts = rgb_luts.clone();

        Ok(Box::new(NativePixelDataIteratorPaletteIndex {
          definition,
          rgb_luts,
          pixel_count,
          data,
          index: 0,
          offset: 0,
          stride: bits_allocated,
        }))
      }

      _ => Err(DataError::new_value_invalid(format!(
        "Photometric interpretation '{:?}' is invalid for color pixel data \
         when samples per pixel is one",
        definition.photometric_interpretation
      ))),
    },

    SamplesPerPixel::Three {
      planar_configuration,
    } => {
      let one_over_max_value =
        1.0 / (((1 << definition.bits_stored as usize) - 1) as f64);

      match definition.photometric_interpretation {
        PhotometricInterpretation::Rgb | PhotometricInterpretation::YbrFull => {
          let (offset, stride) = match planar_configuration {
            PlanarConfiguration::Interleaved => {
              (bits_allocated, usize::from(definition.bits_allocated) * 3)
            }

            PlanarConfiguration::Separate => {
              (pixel_count * bits_allocated, bits_allocated)
            }
          };

          Ok(Box::new(NativePixelDataIteratorColor {
            definition: definition.clone(),
            pixel_count,
            one_over_max_value,
            data,
            index: 0,
            offsets: (0, offset, offset * 2),
            stride,
          }))
        }

        PhotometricInterpretation::YbrFull422 => {
          if definition.columns % 2 == 1 {
            return Err(DataError::new_value_invalid(
              "The width of YBR_FULL_222 color pixel data is odd".to_string(),
            ));
          }

          let (offsets, strides) = match planar_configuration {
            PlanarConfiguration::Interleaved => (
              (0, bits_allocated * 2, bits_allocated * 3),
              (bits_allocated, bits_allocated * 4, bits_allocated * 4),
            ),

            PlanarConfiguration::Separate => (
              (
                0,
                pixel_count * bits_allocated,
                pixel_count * bits_allocated * 3 / 2,
              ),
              (bits_allocated, bits_allocated, bits_allocated),
            ),
          };

          Ok(Box::new(NativePixelDataIteratorYbrFull422 {
            definition: definition.clone(),
            planar_configuration,
            pixel_count,
            one_over_max_value,
            data,
            index: 0,
            offsets,
            strides,
          }))
        }

        _ => Err(DataError::new_value_invalid(format!(
          "Photometric interpretation '{:?}' is invalid for color pixel data \
           when samples per pixel is three",
          definition.photometric_interpretation
        ))),
      }
    }
  }
}

/// Validates various parts of the definition for use with the given raw data.
///
fn validate_definition_and_data(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<(), DataError> {
  // Check that the number of bits stored does not exceed the number of bits
  // allocated
  if definition.bits_stored as usize > definition.bits_allocated.into() {
    return Err(DataError::new_value_invalid(format!(
      "Bits stored '{}' is greater than the bits allocated which is '{}'",
      definition.bits_stored,
      usize::from(definition.bits_allocated),
    )));
  }

  // Check that the high bit is one less than the bits stored
  if definition.high_bit as u32 + 1 != definition.bits_stored as u32 {
    return Err(DataError::new_value_invalid(format!(
      "High bit '{}' is not one less than the bits stored which is '{}'",
      definition.high_bit, definition.bits_stored
    )));
  }

  let expected_size_in_bits =
    definition.pixel_count() * definition.pixel_size_in_bits();

  // Validate that the provided data is of the expected size
  if data.len() * 8 < expected_size_in_bits {
    return Err(DataError::new_value_invalid(format!(
      "Pixel data has incorrect length, expected {} bits but found {} bits",
      expected_size_in_bits,
      data.len() * 8,
    )));
  }

  Ok(())
}

/// Iterator for native integer grayscale pixel data.
///
struct NativePixelDataIteratorGrayscale<'a> {
  definition: PixelDataDefinition,
  pixel_count: usize,

  data: &'a [u8],
  index: usize,
  offset: usize,
  stride: usize,
}

impl Iterator for NativePixelDataIteratorGrayscale<'_> {
  type Item = i64;

  fn next(&mut self) -> Option<i64> {
    if self.index >= self.pixel_count {
      return None;
    }

    let sample = read_sample(&self.definition, self.data, self.offset);

    self.index += 1;
    self.offset += self.stride;

    Some(sample)
  }
}

/// Iterator for native palette index pixel data. This iterator looks up the
/// relevant palettes and emits RGB colors in the range 0-1.
///
struct NativePixelDataIteratorPaletteIndex<'a> {
  definition: PixelDataDefinition,
  rgb_luts: (LookupTable, LookupTable, LookupTable),
  pixel_count: usize,

  data: &'a [u8],
  index: usize,
  offset: usize,
  stride: usize,
}

impl Iterator for NativePixelDataIteratorPaletteIndex<'_> {
  type Item = RgbColor;

  fn next(&mut self) -> Option<RgbColor> {
    if self.index >= self.pixel_count {
      return None;
    }

    let index = read_sample(&self.definition, self.data, self.offset);

    let r = self.rgb_luts.0.lookup_normalized(index);
    let g = self.rgb_luts.1.lookup_normalized(index);
    let b = self.rgb_luts.2.lookup_normalized(index);

    self.index += 1;
    self.offset += self.stride;

    Some((r, g, b))
  }
}

/// Iterator for native color pixel data stored in RGB or YBR. Emits RGB colors
/// in the range 0-1.
///
struct NativePixelDataIteratorColor<'a> {
  definition: PixelDataDefinition,
  pixel_count: usize,
  one_over_max_value: f64,

  data: &'a [u8],
  index: usize,
  offsets: (usize, usize, usize),
  stride: usize,
}

impl Iterator for NativePixelDataIteratorColor<'_> {
  type Item = RgbColor;

  fn next(&mut self) -> Option<RgbColor> {
    if self.index >= self.pixel_count {
      return None;
    }

    let mut samples = (
      read_sample(&self.definition, self.data, self.offsets.0) as f64,
      read_sample(&self.definition, self.data, self.offsets.1) as f64,
      read_sample(&self.definition, self.data, self.offsets.2) as f64,
    );

    samples.0 *= self.one_over_max_value;
    samples.1 *= self.one_over_max_value;
    samples.2 *= self.one_over_max_value;

    if self.definition.photometric_interpretation.is_ybr() {
      samples = ybr_to_rgb(samples);
    }

    self.index += 1;
    self.offsets.0 += self.stride;
    self.offsets.1 += self.stride;
    self.offsets.2 += self.stride;

    Some(samples)
  }
}

/// Iterator for native color pixel data stored in YbrFull422. Emits RGB colors
/// in the range 0-1.
///
struct NativePixelDataIteratorYbrFull422<'a> {
  definition: PixelDataDefinition,
  planar_configuration: PlanarConfiguration,
  pixel_count: usize,
  one_over_max_value: f64,

  data: &'a [u8],
  index: usize,
  offsets: (usize, usize, usize),
  strides: (usize, usize, usize),
}

impl Iterator for NativePixelDataIteratorYbrFull422<'_> {
  type Item = RgbColor;

  fn next(&mut self) -> Option<RgbColor> {
    if self.index >= self.pixel_count {
      return None;
    }

    let mut samples = (
      read_sample(&self.definition, self.data, self.offsets.0) as f64,
      read_sample(&self.definition, self.data, self.offsets.1) as f64,
      read_sample(&self.definition, self.data, self.offsets.2) as f64,
    );

    samples.0 *= self.one_over_max_value;
    samples.1 *= self.one_over_max_value;
    samples.2 *= self.one_over_max_value;

    samples = ybr_to_rgb(samples);

    self.index += 1;
    self.offsets.0 += self.strides.0;

    // Move the Cb and Cr offsets after every second pixel is read
    if self.index & 1 == 0 {
      self.offsets.1 += self.strides.1;
      self.offsets.2 += self.strides.2;

      if self.planar_configuration == PlanarConfiguration::Interleaved {
        self.offsets.0 += self.strides.0 * 2;
      }
    }

    Some(samples)
  }
}

/// Reads a single sample out of the given pixel data at the specified bit
/// offset.
///
fn read_sample(
  definition: &PixelDataDefinition,
  data: &[u8],
  offset: usize,
) -> i64 {
  let mut value: i64 = match definition.bits_allocated {
    BitsAllocated::One => ((data[offset / 8] >> (offset % 8)) & 1).into(),
    BitsAllocated::Eight => data[offset / 8].into(),
    BitsAllocated::Sixteen => {
      u16::from_le_bytes([data[offset / 8], data[offset / 8 + 1]]).into()
    }
    BitsAllocated::ThirtyTwo => u32::from_le_bytes([
      data[offset / 8],
      data[offset / 8 + 1],
      data[offset / 8 + 2],
      data[offset / 8 + 3],
    ])
    .into(),
  };

  // Reinterpret as a signed integer for signed pixel representations
  if definition.pixel_representation.is_signed() {
    let high_bit = 1 << definition.high_bit;
    if value >= high_bit {
      value -= high_bit * 2;
    }
  }

  value
}

/// Converts a YBR color in the range 0-1 to an RGB color in the range 0-1.
///
fn ybr_to_rgb(color: RgbColor) -> RgbColor {
  let (y, cb, cr) = color;

  let r = y + 1.402 * (cr - 0.5);
  let g = y - 0.3441362862 * (cb - 0.5) - 0.7141362862 * (cr - 0.5);
  let b = y + 1.772 * (cb - 0.5);

  (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::PixelRepresentation;

  #[test]
  fn decode_monochrome_16bit_unsigned() {
    let definition = PixelDataDefinition {
      samples_per_pixel: SamplesPerPixel::One,
      photometric_interpretation: PhotometricInterpretation::Monochrome2,
      rows: 2,
      columns: 2,
      bits_allocated: BitsAllocated::Eight,
      bits_stored: 8,
      high_bit: 7,
      pixel_representation: PixelRepresentation::Unsigned,
    };

    let data = [0, 1, 2, 3];

    assert_eq!(
      iter_pixels_grayscale(definition, &data)
        .unwrap()
        .collect::<Vec<_>>(),
      vec![0, 1, 2, 3]
    );
  }

  #[test]
  fn decode_rgb_8bit_interleaved() {
    let definition = PixelDataDefinition {
      samples_per_pixel: SamplesPerPixel::Three {
        planar_configuration: PlanarConfiguration::Interleaved,
      },
      photometric_interpretation: PhotometricInterpretation::Rgb,
      rows: 2,
      columns: 2,
      bits_allocated: BitsAllocated::Eight,
      bits_stored: 8,
      high_bit: 7,
      pixel_representation: PixelRepresentation::Unsigned,
    };

    let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

    assert_eq!(
      iter_pixels_color(definition, &data)
        .unwrap()
        .collect::<Vec<_>>(),
      vec![
        (0.0 / 255.0, 1.0 / 255.0, 2.0 / 255.0),
        (3.0 / 255.0, 4.0 / 255.0, 5.0 / 255.0),
        (6.0 / 255.0, 7.0 / 255.0, 8.0 / 255.0),
        (9.0 / 255.0, 10.0 / 255.0, 11.0 / 255.0)
      ]
    );
  }

  #[test]
  fn decode_rgb_16bit_separate() {
    let definition = PixelDataDefinition {
      samples_per_pixel: SamplesPerPixel::Three {
        planar_configuration: PlanarConfiguration::Separate,
      },
      photometric_interpretation: PhotometricInterpretation::Rgb,
      rows: 2,
      columns: 2,
      bits_allocated: BitsAllocated::Sixteen,
      bits_stored: 16,
      high_bit: 15,
      pixel_representation: PixelRepresentation::Unsigned,
    };

    let data = vec![
      0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8, 0, 9, 0, 10, 0, 11, 0,
    ];

    assert_eq!(
      iter_pixels_color(definition, &data)
        .unwrap()
        .collect::<Vec<_>>(),
      vec![
        (0.0 / 65535.0, 4.0 / 65535.0, 8.0 / 65535.0),
        (1.0 / 65535.0, 5.0 / 65535.0, 9.0 / 65535.0),
        (2.0 / 65535.0, 6.0 / 65535.0, 10.0 / 65535.0),
        (3.0 / 65535.0, 7.0 / 65535.0, 11.0 / 65535.0)
      ]
    );
  }

  #[test]
  fn decode_ybr_full_8bit() {
    let definition = PixelDataDefinition {
      samples_per_pixel: SamplesPerPixel::Three {
        planar_configuration: PlanarConfiguration::Interleaved,
      },
      photometric_interpretation: PhotometricInterpretation::YbrFull,
      rows: 2,
      columns: 2,
      bits_allocated: BitsAllocated::Eight,
      bits_stored: 8,
      high_bit: 7,
      pixel_representation: PixelRepresentation::Unsigned,
    };

    let data = [142, 122, 111, 148, 118, 122, 101, 123, 127, 116, 133, 142];

    assert_eq!(
      iter_pixels_color(definition, &data)
        .unwrap()
        .collect::<Vec<_>>(),
      vec![
        (0.4661450980392157, 0.6104941109662745, 0.518643137254902),
        (0.5501529411764705, 0.608615859972549, 0.5143764705882352),
        (0.39332941176470587, 0.4035516918862745, 0.3648078431372549),
        (0.5346235294117647, 0.40687166382745094, 0.49312156862745105)
      ]
    );
  }

  #[test]
  fn decode_ybr_full_422_8bit() {
    let definition = PixelDataDefinition {
      samples_per_pixel: SamplesPerPixel::Three {
        planar_configuration: PlanarConfiguration::Interleaved,
      },
      photometric_interpretation: PhotometricInterpretation::YbrFull422,
      rows: 2,
      columns: 2,
      bits_allocated: BitsAllocated::Eight,
      bits_stored: 8,
      high_bit: 7,
      pixel_representation: PixelRepresentation::Unsigned,
    };

    let data = [142, 122, 111, 148, 118, 122, 101, 123];

    assert_eq!(
      iter_pixels_color(definition, &data)
        .unwrap()
        .collect::<Vec<_>>(),
      vec![
        (0.6695725490196077, 0.521719430804706, 0.4422039215686275),
        (0.5911411764705881, 0.4432880582556863, 0.3637725490196078),
        (0.4380039215686275, 0.5111106857733333, 0.2785960784313726),
        (0.45369019607843136, 0.5267969602831372, 0.29428235294117644)
      ]
    );
  }
}
