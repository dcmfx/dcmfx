//! Specifies values of data elements relevant to parsing pixel data.

#[cfg(feature = "std")]
use std::rc::Rc;

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  rc::Rc,
  string::{String, ToString},
};

use dcmfx_core::{
  DataElementTag, DataError, DataSet, DataSetPath, IodModule, RcByteSlice,
  ValueRepresentation, dictionary,
};

use crate::iods::PaletteColorLookupTableModule;

/// Holds values of all of the data elements relevant to decoding and
/// decompressing pixel data.
///
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePixelModule {
  samples_per_pixel: SamplesPerPixel,
  photometric_interpretation: PhotometricInterpretation,
  rows: u16,
  columns: u16,
  bits_allocated: BitsAllocated,
  bits_stored: u16,
  high_bit: u16,
  pixel_representation: PixelRepresentation,
  pixel_aspect_ratio: Option<(i64, i64)>,
  smallest_image_pixel_value: Option<i64>,
  largest_image_pixel_value: Option<i64>,
  icc_profile: Option<RcByteSlice>,
  color_space: Option<String>,
}

impl IodModule for ImagePixelModule {
  fn is_iod_module_data_element(
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: Option<u32>,
    path: &DataSetPath,
  ) -> bool {
    if !path.is_empty() {
      return false;
    }

    if PaletteColorLookupTableModule::is_iod_module_data_element(
      tag, vr, length, path,
    ) {
      return true;
    }

    tag == dictionary::SAMPLES_PER_PIXEL.tag
      || tag == dictionary::PHOTOMETRIC_INTERPRETATION.tag
      || tag == dictionary::PLANAR_CONFIGURATION.tag
      || tag == dictionary::ROWS.tag
      || tag == dictionary::COLUMNS.tag
      || tag == dictionary::BITS_ALLOCATED.tag
      || tag == dictionary::BITS_STORED.tag
      || tag == dictionary::HIGH_BIT.tag
      || tag == dictionary::PIXEL_REPRESENTATION.tag
      || tag == dictionary::PIXEL_ASPECT_RATIO.tag
      || tag == dictionary::SMALLEST_IMAGE_PIXEL_VALUE.tag
      || tag == dictionary::LARGEST_IMAGE_PIXEL_VALUE.tag
      || tag == dictionary::ICC_PROFILE.tag
      || tag == dictionary::COLOR_SPACE.tag
  }

  fn iod_module_highest_tag() -> DataElementTag {
    PaletteColorLookupTableModule::iod_module_highest_tag()
      .max(dictionary::COLOR_SPACE.tag)
  }

  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let samples_per_pixel = SamplesPerPixel::from_data_set(data_set)?;

    let photometric_interpretation =
      PhotometricInterpretation::from_data_set(data_set)?;

    let pixel_representation = PixelRepresentation::from_data_set(data_set)?;

    let rows = data_set.get_int::<u16>(dictionary::ROWS.tag)?;
    let columns = data_set.get_int::<u16>(dictionary::COLUMNS.tag)?;
    let bits_allocated = BitsAllocated::from_data_set(data_set)?;
    let bits_stored = data_set.get_int::<u16>(dictionary::BITS_STORED.tag)?;
    let high_bit = data_set.get_int::<u16>(dictionary::HIGH_BIT.tag)?;

    let pixel_aspect_ratio = if data_set.has(dictionary::PIXEL_ASPECT_RATIO.tag)
    {
      match data_set
        .get_ints::<i64>(dictionary::PIXEL_ASPECT_RATIO.tag)?
        .as_slice()
      {
        [] => None,
        [a, b] => Some((*a, *b)),
        _ => {
          return Err(DataError::MultiplicityMismatch {
            path: Some(DataSetPath::new_with_data_element(
              dictionary::PIXEL_ASPECT_RATIO.tag,
            )),
          });
        }
      }
    } else {
      None
    };

    let smallest_image_pixel_value = if data_set
      .has(dictionary::SMALLEST_IMAGE_PIXEL_VALUE.tag)
    {
      Some(data_set.get_int::<i64>(dictionary::SMALLEST_IMAGE_PIXEL_VALUE.tag)?)
    } else {
      None
    };

    let largest_image_pixel_value = if data_set
      .has(dictionary::LARGEST_IMAGE_PIXEL_VALUE.tag)
    {
      Some(data_set.get_int::<i64>(dictionary::LARGEST_IMAGE_PIXEL_VALUE.tag)?)
    } else {
      None
    };

    let icc_profile = if data_set.has(dictionary::ICC_PROFILE.tag) {
      Some(
        data_set
          .get_value_bytes(dictionary::ICC_PROFILE.tag)?
          .clone(),
      )
    } else {
      None
    };

    let color_space = if data_set.has(dictionary::COLOR_SPACE.tag) {
      Some(
        data_set
          .get_string(dictionary::COLOR_SPACE.tag)?
          .to_string(),
      )
    } else {
      None
    };

    Self::new(
      samples_per_pixel,
      photometric_interpretation,
      rows,
      columns,
      bits_allocated,
      bits_stored,
      high_bit,
      pixel_representation,
      pixel_aspect_ratio,
      smallest_image_pixel_value,
      largest_image_pixel_value,
      icc_profile,
      color_space,
    )
  }
}

impl ImagePixelModule {
  /// Creates a new [`ImagePixelModule`] with the given values. A number of
  /// validations are performed to ensure the values are internally consistent.
  ///
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    samples_per_pixel: SamplesPerPixel,
    photometric_interpretation: PhotometricInterpretation,
    rows: u16,
    columns: u16,
    bits_allocated: BitsAllocated,
    bits_stored: u16,
    high_bit: u16,
    pixel_representation: PixelRepresentation,
    pixel_aspect_ratio: Option<(i64, i64)>,
    smallest_image_pixel_value: Option<i64>,
    largest_image_pixel_value: Option<i64>,
    icc_profile: Option<RcByteSlice>,
    color_space: Option<String>,
  ) -> Result<Self, DataError> {
    // Check that the number of bits stored does not exceed the number of bits
    // allocated
    if bits_stored == 0 || bits_stored > u8::from(bits_allocated).into() {
      return Err(DataError::new_value_invalid(format!(
        "Bits stored '{}' is invalid for bits allocated '{}'",
        bits_stored,
        u8::from(bits_allocated),
      )));
    }

    // Check that the high bit is one less than the bits stored
    if high_bit != bits_stored - 1 {
      return Err(DataError::new_value_invalid(format!(
        "High bit '{}' is not one less than the bits stored '{}'",
        high_bit, bits_stored
      )));
    }

    Ok(Self {
      samples_per_pixel,
      photometric_interpretation,
      rows,
      columns,
      bits_allocated,
      bits_stored,
      high_bit,
      pixel_representation,
      pixel_aspect_ratio,
      smallest_image_pixel_value,
      largest_image_pixel_value,
      icc_profile,
      color_space,
    })
  }

  /// Creates a new [`ImagePixelModule`] from just those values that are
  /// required.
  ///
  #[allow(clippy::too_many_arguments)]
  pub fn new_basic(
    samples_per_pixel: SamplesPerPixel,
    photometric_interpretation: PhotometricInterpretation,
    rows: u16,
    columns: u16,
    bits_allocated: BitsAllocated,
    bits_stored: u16,
    pixel_representation: PixelRepresentation,
  ) -> Result<Self, DataError> {
    Self::new(
      samples_per_pixel,
      photometric_interpretation,
      rows,
      columns,
      bits_allocated,
      bits_stored,
      bits_stored.saturating_sub(1),
      pixel_representation,
      None,
      None,
      None,
      None,
      None,
    )
  }

  /// Returns this image pixel module's number of samples per pixel.
  ///
  pub fn samples_per_pixel(&self) -> SamplesPerPixel {
    self.samples_per_pixel
  }

  /// Returns this image pixel module's photometric interpretation.
  ///
  pub fn photometric_interpretation(&self) -> &PhotometricInterpretation {
    &self.photometric_interpretation
  }

  /// Returns this image pixel module's number of rows, i.e. its height.
  ///
  pub fn rows(&self) -> u16 {
    self.rows
  }

  /// Returns this image pixel module's number of columns, i.e. its width.
  ///
  pub fn columns(&self) -> u16 {
    self.columns
  }

  /// Returns this image pixel module's number of bits allocated per pixel.
  ///
  pub fn bits_allocated(&self) -> BitsAllocated {
    self.bits_allocated
  }

  /// Returns this image pixel module's number of bits stored per pixel.
  /// This will never exceed the number of bits allocated per pixel.
  ///
  pub fn bits_stored(&self) -> u16 {
    self.bits_stored
  }

  /// Returns this image pixel module's high bit. This is always equal to
  /// the number of bits stored per pixel minus one.
  ///
  pub fn high_bit(&self) -> u16 {
    self.high_bit
  }

  /// Returns this image pixel module's pixel representation, i.e. whether
  /// it stores signed or unsigned values.
  ///
  pub fn pixel_representation(&self) -> PixelRepresentation {
    self.pixel_representation
  }

  /// Returns the range of integer values that can be stored.
  ///
  pub fn stored_value_range(&self) -> core::ops::RangeInclusive<i64> {
    let min = if self.pixel_representation.is_signed() {
      -(1i64 << (self.bits_stored - 1))
    } else {
      0
    };

    let max = if self.pixel_representation.is_signed() {
      (1i64 << (self.bits_stored - 1)) - 1
    } else {
      (1i64 << self.bits_stored) - 1
    };

    min..=max
  }

  /// Returns the number of bits consumed by a single pixel.
  ///
  pub fn pixel_size_in_bits(&self) -> usize {
    match self.photometric_interpretation {
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2
      | PhotometricInterpretation::PaletteColor { .. }
      | PhotometricInterpretation::Rgb
      | PhotometricInterpretation::YbrFull
      | PhotometricInterpretation::YbrIct
      | PhotometricInterpretation::YbrRct
      | PhotometricInterpretation::Xyb => {
        usize::from(u8::from(self.samples_per_pixel))
          * usize::from(u8::from(self.bits_allocated))
      }

      PhotometricInterpretation::YbrFull422 => {
        usize::from(u8::from(self.bits_allocated)) * 2
      }
    }
  }

  /// Returns the number of pixels.
  ///
  pub fn pixel_count(&self) -> usize {
    usize::from(self.rows) * usize::from(self.columns)
  }

  /// Returns the number of bytes consumed by a single frame of image data.
  ///
  /// If the size of a single frame of image data is not a whole number of
  /// bytes, which is possible when [`Self::bits_allocated`] is
  /// [`BitsAllocated::One`], then the result is rounded up to a whole number of
  /// bytes.
  ///
  pub fn frame_size_in_bytes(&self) -> usize {
    (self.pixel_count() * self.pixel_size_in_bits()).div_ceil(8)
  }

  /// Returns whether this image pixel module defines grayscale pixel data.
  ///
  pub fn is_grayscale(&self) -> bool {
    match self.photometric_interpretation {
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2 => true,

      PhotometricInterpretation::PaletteColor { .. }
      | PhotometricInterpretation::Rgb
      | PhotometricInterpretation::YbrFull
      | PhotometricInterpretation::YbrFull422
      | PhotometricInterpretation::YbrIct
      | PhotometricInterpretation::YbrRct
      | PhotometricInterpretation::Xyb => false,
    }
  }

  /// Returns whether this image pixel module defines RGB color data.
  ///
  pub fn is_rgb(&self) -> bool {
    !self.is_grayscale()
  }

  /// Returns whether this image pixel module has unused high bits in its
  /// components, i.e. whether the number of bits stored is less than the number
  /// of bits allocated.
  ///
  pub fn has_unused_high_bits(&self) -> bool {
    self.bits_stored < u8::from(self.bits_allocated).into()
  }
}

/// Specifies the number of separate planes in the pixel data image. For
/// monochrome (grayscale) and palette color images, the number of planes is 1.
/// For RGB and other three vector color models, the number of planes is 3.
///
/// Ref: PS3.3 C.7.6.3.1.1.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SamplesPerPixel {
  /// One sample per pixel.
  One,

  /// Three samples per pixel. This is accompanied by a planar configuration
  /// that specifies whether the values are interleaved or stored as separate
  /// planes.
  Three {
    planar_configuration: PlanarConfiguration,
  },
}

impl SamplesPerPixel {
  /// Creates a new `SamplesPerPixel` from the *'(0028,0002) Samples per Pixel'*
  /// data element in the given data set.
  ///
  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let tag = dictionary::SAMPLES_PER_PIXEL.tag;

    match data_set.get_int(tag)? {
      1 => Ok(Self::One),
      3 => {
        let planar_configuration =
          PlanarConfiguration::from_data_set(data_set)?;
        Ok(Self::Three {
          planar_configuration,
        })
      }
      value => Err(
        DataError::new_value_invalid(format!(
          "Samples per pixel value of '{:?}' is invalid",
          value
        ))
        .with_path(&DataSetPath::new_with_data_element(tag)),
      ),
    }
  }
}

impl From<SamplesPerPixel> for u8 {
  fn from(samples_per_pixel: SamplesPerPixel) -> u8 {
    match samples_per_pixel {
      SamplesPerPixel::One => 1,
      SamplesPerPixel::Three { .. } => 3,
    }
  }
}

/// Specifies the intended interpretation of pixel data.
///
/// Ref: PS3.3 C.7.6.3.1.2
///
#[derive(Clone, Debug, PartialEq)]
pub enum PhotometricInterpretation {
  /// Pixel data represent a single monochrome image plane. The minimum sample
  /// value is intended to be displayed as white after any VOI grayscale
  /// transformations have been performed.
  ///
  /// This photometric interpretation may be used only when the samples per
  /// pixel has a value of [`SamplesPerPixel::One`].
  Monochrome1,

  /// Pixel data represent a single monochrome image plane. The minimum sample
  /// value is intended to be displayed as black after any VOI grayscale
  /// transformations have been performed.
  ///
  /// This photometric interpretation may be used only when the samples per
  /// pixel is one.
  Monochrome2,

  /// Pixel data describe a color image with a single sample per pixel (single
  /// image plane). The pixel value is used as an index into each of the Red,
  /// Blue, and Green Palette Color Lookup Tables.
  PaletteColor {
    palette: Rc<PaletteColorLookupTableModule>,
  },

  /// Pixel data represent a color image described by red, green, and blue image
  /// planes. The minimum sample value for each color plane represents minimum
  /// intensity of the color. This value may be used only when the samples per
  /// pixel is three.
  Rgb,

  /// Pixel data represent a color image described by one luminance (Y) and two
  /// chrominance planes (CB and CR). This photometric interpretation may be
  /// used only when the samples per pixel is three.
  YbrFull,

  /// The same as [`PhotometricInterpretation::YBRFull`] except that the CB and
  /// CR values are sampled horizontally at half the Y rate and as a result
  /// there are half as many CB and CR values as Y values.
  YbrFull422,

  /// Irreversible Color Transformation.
  ///
  /// Pixel data represent a color image described by one luminance (Y) and two
  /// chrominance planes (CB and CR). This photometric interpretation may be
  /// used only when samples per pixel is three, the planar configuration is
  /// 0, and the transfer syntax is encapsulated.
  YbrIct,

  /// Reversible Color Transformation.
  ///
  /// Pixel data represent a color image described by one luminance (Y) and two
  /// chrominance planes (CB and CR). This photometric interpretation may be
  /// used only when samples per pixel is three and the planar configuration is
  /// 0, and the transfer syntax is encapsulated.
  YbrRct,

  /// Pixel data represent a color image described by XYB, the long/medium/short
  /// wavelength (LMS) based color model inspired by the human visual system,
  /// facilitating perceptually uniform quantization. It uses a gamma of 3 for
  /// computationally efficient decoding. The exact details of the XYB encoding
  /// are defined as part of a specific image being encoded in order to optimize
  /// image fidelity. Images in XYB transcoded to other Transfer Syntaxes will
  /// use RGB or the appropriate equivalent (e.g., YBR_FULL_422 for JPEG).
  ///
  /// This is a possible color space used in JPEG XL [ISO/IEC 18181-1].
  Xyb,
}

impl PhotometricInterpretation {
  /// Creates a new `PhotometricInterpretation` from the *'(0028,0004)
  /// Photometric Interpretation'* data element in the given data set.
  ///
  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let tag = dictionary::PHOTOMETRIC_INTERPRETATION.tag;

    match data_set.get_string(tag)? {
      "MONOCHROME1" => Ok(Self::Monochrome1),
      "MONOCHROME2" => Ok(Self::Monochrome2),
      "PALETTE COLOR" => Ok(Self::PaletteColor {
        palette: Rc::new(PaletteColorLookupTableModule::from_data_set(
          data_set,
        )?),
      }),
      "RGB" => Ok(Self::Rgb),
      "YBR_FULL" => Ok(Self::YbrFull),
      "YBR_FULL_422" => Ok(Self::YbrFull422),

      "YBR_PARTIAL_420" => Err(
        DataError::new_value_invalid(
          "Photometric interpretation 'YBR_PARTIAL_420' is not supported"
            .to_string(),
        )
        .with_path(&DataSetPath::new_with_data_element(tag)),
      ),

      "YBR_ICT" => Ok(Self::YbrIct),
      "YBR_RCT" => Ok(Self::YbrRct),

      value
        if value == "HSV"
          || value == "ARGB"
          || value == "CMYK"
          || value == "YBR_PARTIAL_422" =>
      {
        Err(
          DataError::new_value_invalid(format!(
            "Photometric interpretation '{}' is retired and is not supported",
            value
          ))
          .with_path(&DataSetPath::new_with_data_element(tag)),
        )
      }

      "XYB" => Ok(Self::Xyb),

      value => Err(
        DataError::new_value_invalid(format!(
          "Photometric interpretation '{}' is invalid",
          value
        ))
        .with_path(&DataSetPath::new_with_data_element(tag)),
      ),
    }
  }

  /// Returns whether this photometric interpretation specifies YBR color data.
  ///
  pub fn is_ybr(&self) -> bool {
    match self {
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2
      | PhotometricInterpretation::PaletteColor { .. }
      | PhotometricInterpretation::Rgb
      | PhotometricInterpretation::Xyb => false,

      PhotometricInterpretation::YbrFull
      | PhotometricInterpretation::YbrFull422
      | PhotometricInterpretation::YbrIct
      | PhotometricInterpretation::YbrRct => true,
    }
  }

  /// Returns whether this pixel representation is [`Self::Monochrome1`].
  ///
  pub fn is_monochrome1(&self) -> bool {
    self == &Self::Monochrome1
  }
}

impl core::fmt::Display for PhotometricInterpretation {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    let s = match self {
      PhotometricInterpretation::Monochrome1 => "Monochrome1",
      PhotometricInterpretation::Monochrome2 => "Monochrome2",
      PhotometricInterpretation::PaletteColor { .. } => "PaletteColor",
      PhotometricInterpretation::Rgb => "Rgb",
      PhotometricInterpretation::YbrFull => "YbrFull",
      PhotometricInterpretation::YbrFull422 => "YbrFull422",
      PhotometricInterpretation::YbrIct => "YbrIct",
      PhotometricInterpretation::YbrRct => "YbrRct",
      PhotometricInterpretation::Xyb => "Xyb",
    };

    write!(f, "{}", s)
  }
}

/// Indicates whether the pixel data are encoded color-by-plane or
/// color-by-pixel. Required if the samples per pixel is
/// [`SamplesPerPixel::Three`].
///
/// Ref: PS3.3 C.7.6.3.1.3.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlanarConfiguration {
  /// The sample values for the first pixel are followed by the sample values
  /// for the second pixel, etc. For RGB images, this means the order of the
  /// pixel values encoded shall be R1, G1, B1, R2, G2, B2, …, etc.
  Interleaved,

  /// Each color plane shall be encoded contiguously. For RGB images, this means
  /// the order of the pixel values encoded is R1, R2, R3, …, G1, G2, G3, …, B1,
  /// B2, B3, etc.
  Separate,
}

impl PlanarConfiguration {
  /// Creates a new [`PlanarConfiguration`] from the *'(0028,0006) Planar
  /// Configuration'* data element in the given data set.
  ///
  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let tag = dictionary::PLANAR_CONFIGURATION.tag;

    match data_set.get_int(tag)? {
      0 => Ok(Self::Interleaved),
      1 => Ok(Self::Separate),
      value => Err(
        DataError::new_value_invalid(format!(
          "Planar configuration value of '{}' is invalid",
          value
        ))
        .with_path(&DataSetPath::new_with_data_element(tag)),
      ),
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BitsAllocated {
  One,
  Eight,
  Sixteen,
  ThirtyTwo,
}

impl BitsAllocated {
  /// Creates a new `BitsAllocated` from the *'(0028,0100) Bits Allocated'* data
  /// element in the given data set.
  ///
  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let tag = dictionary::BITS_ALLOCATED.tag;

    match data_set.get_int(tag)? {
      1 => Ok(Self::One),
      8 => Ok(Self::Eight),
      16 => Ok(Self::Sixteen),
      32 => Ok(Self::ThirtyTwo),
      value => Err(
        DataError::new_value_invalid(format!(
          "Bits allocated value of '{}' is not supported",
          value
        ))
        .with_path(&DataSetPath::new_with_data_element(tag)),
      ),
    }
  }
}

impl From<BitsAllocated> for u8 {
  fn from(samples_per_pixel: BitsAllocated) -> u8 {
    match samples_per_pixel {
      BitsAllocated::One => 1,
      BitsAllocated::Eight => 8,
      BitsAllocated::Sixteen => 16,
      BitsAllocated::ThirtyTwo => 32,
    }
  }
}

/// Data representation of the pixel samples. Each sample shall have the same
/// pixel representation.
///
/// Ref: PS3.3 C.7.6.3.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PixelRepresentation {
  /// Pixel samples are stored as unsigned integers.
  Unsigned,

  /// Pixel samples are stored as signed 2's complement integers.
  Signed,
}

impl PixelRepresentation {
  /// Creates a new `PixelRepresentation` from the *'(0028,0103) Pixel
  /// Representation'* data element in the given data set.
  ///
  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let tag = dictionary::PIXEL_REPRESENTATION.tag;

    match data_set.get_int(tag)? {
      0 => Ok(Self::Unsigned),
      1 => Ok(Self::Signed),
      value => Err(
        DataError::new_value_invalid(format!(
          "Pixel representation value of '{}' is invalid",
          value
        ))
        .with_path(&DataSetPath::new_with_data_element(tag)),
      ),
    }
  }

  /// Returns whether the pixel representation is for signed integer data.
  ///
  pub fn is_signed(&self) -> bool {
    *self == Self::Signed
  }
}

impl From<PixelRepresentation> for u8 {
  fn from(pixel_representation: PixelRepresentation) -> u8 {
    match pixel_representation {
      PixelRepresentation::Unsigned => 0,
      PixelRepresentation::Signed => 1,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_stored_value_range() {
    let mut image_pixel_module = ImagePixelModule::new_basic(
      SamplesPerPixel::One,
      PhotometricInterpretation::Monochrome2,
      1,
      1,
      BitsAllocated::Sixteen,
      12,
      PixelRepresentation::Unsigned,
    )
    .unwrap();

    assert_eq!(image_pixel_module.stored_value_range(), 0..=4095);

    image_pixel_module.pixel_representation = PixelRepresentation::Signed;
    assert_eq!(image_pixel_module.stored_value_range(), -2048..=2047);

    image_pixel_module.bits_allocated = BitsAllocated::ThirtyTwo;
    image_pixel_module.bits_stored = 32;
    assert_eq!(
      image_pixel_module.stored_value_range(),
      i64::from(i32::MIN)..=i64::from(i32::MAX)
    );

    image_pixel_module.pixel_representation = PixelRepresentation::Unsigned;
    assert_eq!(
      image_pixel_module.stored_value_range(),
      0..=i64::from(u32::MAX)
    );
  }
}
