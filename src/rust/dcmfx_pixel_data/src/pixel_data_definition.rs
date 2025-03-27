//! Specifies values of data elements relevant to parsing pixel data.

#[cfg(feature = "std")]
use std::rc::Rc;

#[cfg(not(feature = "std"))]
use alloc::{format, rc::Rc, string::ToString};

use dcmfx_core::{DataElementTag, DataError, DataSet, DataSetPath, dictionary};

use crate::RgbLut;

/// Holds values of all of the data elements relevant to decoding and
/// decompressing pixel data.
///
#[derive(Clone, Debug, PartialEq)]
pub struct PixelDataDefinition {
  pub samples_per_pixel: SamplesPerPixel,
  pub photometric_interpretation: PhotometricInterpretation,
  pub rows: u16,
  pub columns: u16,
  pub bits_allocated: BitsAllocated,
  pub bits_stored: u16,
  pub high_bit: u16,
  pub pixel_representation: PixelRepresentation,
}

impl PixelDataDefinition {
  /// The tags of the data elements that are read when creating a new
  /// [`PixelDataDefinition`].
  ///
  pub const DATA_ELEMENT_TAGS: [DataElementTag; 18] = [
    dictionary::SAMPLES_PER_PIXEL.tag,
    dictionary::PHOTOMETRIC_INTERPRETATION.tag,
    dictionary::PLANAR_CONFIGURATION.tag,
    dictionary::ROWS.tag,
    dictionary::COLUMNS.tag,
    dictionary::BITS_ALLOCATED.tag,
    dictionary::BITS_STORED.tag,
    dictionary::HIGH_BIT.tag,
    dictionary::PIXEL_REPRESENTATION.tag,
    dictionary::RED_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag,
    dictionary::RED_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag,
    dictionary::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag,
    dictionary::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::SEGMENTED_RED_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::SEGMENTED_GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
    dictionary::SEGMENTED_BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA.tag,
  ];

  /// Creates a new [`PixelDataDefinition`] from the relevant data elements in
  /// the given data set.
  ///
  pub fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let samples_per_pixel = SamplesPerPixel::from_data_set(data_set)?;

    let photometric_interpretation =
      PhotometricInterpretation::from_data_set(data_set)?;

    let pixel_representation = PixelRepresentation::from_data_set(data_set)?;

    let rows = data_set.get_int::<u16>(dictionary::ROWS.tag)?;
    let columns = data_set.get_int::<u16>(dictionary::COLUMNS.tag)?;
    let bits_allocated = BitsAllocated::from_data_set(data_set)?;
    let bits_stored = data_set.get_int::<u16>(dictionary::BITS_STORED.tag)?;
    let high_bit = data_set.get_int::<u16>(dictionary::HIGH_BIT.tag)?;

    // Check that the number of bits stored does not exceed the number of bits
    // allocated
    if bits_stored == 0 || bits_stored as usize > usize::from(bits_allocated) {
      return Err(DataError::new_value_invalid(format!(
        "Bits stored '{}' is invalid for bits allocated '{}'",
        bits_stored,
        usize::from(bits_allocated),
      )));
    }

    // Check that the high bit is one less than the bits stored
    if high_bit as usize + 1 != bits_stored as usize {
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
    })
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
        usize::from(self.samples_per_pixel) * usize::from(self.bits_allocated)
      }

      PhotometricInterpretation::YbrFull422 => {
        2 * usize::from(self.bits_allocated)
      }
    }
  }

  /// Returns the number of pixels.
  ///
  pub fn pixel_count(&self) -> usize {
    self.rows as usize * self.columns as usize
  }

  /// Returns the number of bytes consumed by a single frame of image data.
  ///
  /// If the size of a single frame of image data is not a whole number of
  /// bytes, which is possible when [`Self::bits_allocated`] is
  /// [`BitsAllocated::One`], then the result is rounded up to a whole number of
  /// bytes.
  ///
  pub fn frame_size_in_bytes(&self) -> usize {
    (self.pixel_count() * self.pixel_size_in_bits() + 7) / 8
  }

  /// Returns whether this pixel data definition defines grayscale pixel data.
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

  /// Returns whether this pixel data definition defines RGB color data.
  ///
  pub fn is_rgb(&self) -> bool {
    !self.is_grayscale()
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

impl From<SamplesPerPixel> for usize {
  fn from(samples_per_pixel: SamplesPerPixel) -> usize {
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
  PaletteColor { palette: Rc<RgbLut> },

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
  /// used only when samples per pixel is three and the planar configuration is
  /// 0.
  YbrIct,

  /// Reversible Color Transformation.
  ///
  /// Pixel data represent a color image described by one luminance (Y) and two
  /// chrominance planes (CB and CR). This photometric interpretation may be
  /// used only when samples per pixel is three and the planar configuration is
  /// 0.
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
  pub fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let tag = dictionary::PHOTOMETRIC_INTERPRETATION.tag;

    match data_set.get_string(tag)? {
      "MONOCHROME1" => Ok(Self::Monochrome1),
      "MONOCHROME2" => Ok(Self::Monochrome2),
      "PALETTE COLOR" => Ok(Self::PaletteColor {
        palette: Rc::new(RgbLut::from_data_set(data_set)?),
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
  /// Creates a new `PlanarConfiguration` from the *'(0028,0006) Planar
  /// Configuration'* data element in the given data set.
  ///
  pub fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
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
  pub fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
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

impl From<BitsAllocated> for usize {
  fn from(samples_per_pixel: BitsAllocated) -> usize {
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
  pub fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
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

impl From<PixelRepresentation> for usize {
  fn from(pixel_representation: PixelRepresentation) -> usize {
    match pixel_representation {
      PixelRepresentation::Unsigned => 0,
      PixelRepresentation::Signed => 1,
    }
  }
}
