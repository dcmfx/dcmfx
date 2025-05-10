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
  DataElementTag, DataElementValue, DataError, DataSet, DataSetPath, IodModule,
  RcByteSlice, ValueRepresentation, dictionary,
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
  pixel_aspect_ratio: Option<(i32, i32)>,
  smallest_image_pixel_value: Option<i64>,
  largest_image_pixel_value: Option<i64>,
  icc_profile: Option<RcByteSlice>,
  color_space: Option<String>,
}

impl core::fmt::Display for ImagePixelModule {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("ImagePixelModule")
      .field("samples_per_pixel", &self.samples_per_pixel)
      .field(
        "photometric_interpretation",
        &self.photometric_interpretation.to_string(),
      )
      .field("rows", &self.rows)
      .field("columns", &self.columns)
      .field("bits_allocated", &self.bits_allocated)
      .field("bits_stored", &self.bits_stored)
      .field("pixel_representation", &self.pixel_representation)
      .finish()
  }
}

impl IodModule for ImagePixelModule {
  fn is_iod_module_data_element(
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: Option<u32>,
    path: &DataSetPath,
  ) -> bool {
    if !path.is_root() {
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
        .get_ints::<i32>(dictionary::PIXEL_ASPECT_RATIO.tag)?
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
    pixel_aspect_ratio: Option<(i32, i32)>,
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

    // Check that the image width is even when using YBR 422
    if photometric_interpretation == PhotometricInterpretation::YbrFull422
      && columns % 2 == 1
    {
      return Err(DataError::new_value_invalid(format!(
        "Uneven width '{}' is not allowed with the YBR 422 photometric \
         interpretation",
        columns
      )));
    }

    let image_pixel_module = Self {
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
    };

    // Check that the the frame size in bytes isn't too large to represent
    if image_pixel_module.frame_size_in_bits().div_ceil(8)
      > u64::from(u32::MAX - 1)
    {
      return Err(DataError::new_value_invalid(
        "Frame size exceeds 2^32 - 2".to_string(),
      ));
    }

    Ok(image_pixel_module)
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

  /// Sets this image pixel module's photometric interpretation. The samples per
  /// pixel value is also updated appropriately.
  ///
  pub fn set_photometric_interpretation(
    &mut self,
    new_photometric_interpretation: PhotometricInterpretation,
  ) -> &Self {
    self.photometric_interpretation = new_photometric_interpretation;

    match self.photometric_interpretation {
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2
      | PhotometricInterpretation::PaletteColor { .. } => {
        self.samples_per_pixel = SamplesPerPixel::One;
      }

      _ => {
        self.samples_per_pixel = SamplesPerPixel::Three {
          planar_configuration: PlanarConfiguration::Interleaved,
        };
      }
    }

    self
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

  /// Returns the planar configuration in use when there are three samples per
  /// pixel. If there is only one sample per pixel then a fallback of
  /// [`PlanarConfiguration::Interleaved`] is returned, although it is not
  /// relevant to how the pixel data is stored.
  ///
  pub fn planar_configuration(&self) -> PlanarConfiguration {
    match self.samples_per_pixel {
      SamplesPerPixel::One => PlanarConfiguration::Interleaved,
      SamplesPerPixel::Three {
        planar_configuration,
      } => planar_configuration,
    }
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
  pub fn pixel_size_in_bits(&self) -> u8 {
    match self.photometric_interpretation {
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2
      | PhotometricInterpretation::PaletteColor { .. }
      | PhotometricInterpretation::Rgb
      | PhotometricInterpretation::YbrFull
      | PhotometricInterpretation::YbrIct
      | PhotometricInterpretation::YbrRct
      | PhotometricInterpretation::Xyb => {
        u8::from(self.samples_per_pixel) * u8::from(self.bits_allocated)
      }

      PhotometricInterpretation::YbrFull422 => {
        u8::from(self.bits_allocated) * 2
      }
    }
  }

  /// Returns the number of pixels.
  ///
  pub fn pixel_count(&self) -> usize {
    usize::from(self.rows) * usize::from(self.columns)
  }

  /// Returns the number of bits consumed by a single frame of pixel data.
  ///
  pub fn frame_size_in_bits(&self) -> u64 {
    self.pixel_count() as u64 * u64::from(self.pixel_size_in_bits())
  }

  /// Returns the number of bytes consumed by a single frame of pixel data.
  ///
  /// If the size of a single frame of image data is not a whole number of
  /// bytes, which is possible when [`Self::bits_allocated`] is
  /// [`BitsAllocated::One`], then the result is rounded up to a whole number of
  /// bytes.
  ///
  pub fn frame_size_in_bytes(&self) -> usize {
    self.frame_size_in_bits().div_ceil(8) as usize
  }

  /// Returns whether this image pixel module defines grayscale pixel data.
  ///
  pub fn is_grayscale(&self) -> bool {
    self.photometric_interpretation.is_grayscale()
  }

  /// Returns whether this image pixel module defines color data.
  ///
  pub fn is_color(&self) -> bool {
    !self.photometric_interpretation.is_grayscale()
  }

  /// Returns whether this image pixel module has unused high bits in its
  /// components, i.e. whether the number of bits stored is less than the number
  /// of bits allocated.
  ///
  pub fn has_unused_high_bits(&self) -> bool {
    self.bits_stored < u8::from(self.bits_allocated).into()
  }

  /// Converts this Image Pixel Module to a data set.
  ///
  pub fn to_data_set(&self) -> Result<DataSet, DataError> {
    let mut data_set = DataSet::new();

    data_set.insert(
      dictionary::SAMPLES_PER_PIXEL.tag,
      self.samples_per_pixel.to_data_element_value(),
    );

    data_set.insert(
      dictionary::PLANAR_CONFIGURATION.tag,
      self.planar_configuration().to_data_element_value(),
    );

    data_set.insert(
      dictionary::PHOTOMETRIC_INTERPRETATION.tag,
      self.photometric_interpretation.to_data_element_value(),
    );

    data_set.insert(
      dictionary::ROWS.tag,
      DataElementValue::new_unsigned_short(&[self.rows])?,
    );

    data_set.insert(
      dictionary::COLUMNS.tag,
      DataElementValue::new_unsigned_short(&[self.columns])?,
    );

    data_set.insert(
      dictionary::BITS_ALLOCATED.tag,
      self.bits_allocated.to_data_element_value(),
    );

    data_set.insert(
      dictionary::BITS_STORED.tag,
      DataElementValue::new_unsigned_short(&[self.bits_stored])?,
    );

    data_set.insert(
      dictionary::HIGH_BIT.tag,
      DataElementValue::new_unsigned_short(&[self.high_bit])?,
    );

    data_set.insert(
      dictionary::PIXEL_REPRESENTATION.tag,
      self.pixel_representation.to_data_element_value(),
    );

    if let Some((a, b)) = self.pixel_aspect_ratio {
      data_set.insert(
        dictionary::PIXEL_ASPECT_RATIO.tag,
        DataElementValue::new_integer_string(&[a, b])?,
      );
    }

    if let Some(smallest_image_pixel_value) = self.smallest_image_pixel_value {
      if self.pixel_representation.is_signed() {
        data_set.insert(
          dictionary::SMALLEST_IMAGE_PIXEL_VALUE.tag,
          DataElementValue::new_signed_short(&[
            smallest_image_pixel_value as i16
          ])?,
        );
      } else {
        data_set.insert(
          dictionary::SMALLEST_IMAGE_PIXEL_VALUE.tag,
          DataElementValue::new_unsigned_short(&[
            smallest_image_pixel_value as u16
          ])?,
        );
      }
    }

    if let Some(largest_image_pixel_value) = self.largest_image_pixel_value {
      if self.pixel_representation.is_signed() {
        data_set.insert(
          dictionary::LARGEST_IMAGE_PIXEL_VALUE.tag,
          DataElementValue::new_signed_short(&[
            largest_image_pixel_value as i16
          ])?,
        );
      } else {
        data_set.insert(
          dictionary::LARGEST_IMAGE_PIXEL_VALUE.tag,
          DataElementValue::new_unsigned_short(&[
            largest_image_pixel_value as u16
          ])?,
        );
      }
    }

    if let Some(icc_profile) = &self.icc_profile {
      data_set.insert(
        dictionary::ICC_PROFILE.tag,
        DataElementValue::new_binary_unchecked(
          ValueRepresentation::OtherByteString,
          icc_profile.clone(),
        ),
      );
    }

    if let Some(color_space) = &self.color_space {
      data_set.insert(
        dictionary::COLOR_SPACE.tag,
        DataElementValue::new_code_string(&[color_space])?,
      );
    }

    Ok(data_set)
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
  /// Creates a new [`SamplesPerPixel`] from the *'(0028,0002) Samples per
  /// Pixel'* data element in the given data set.
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

  /// Converts this [`SamplesPerPixel`] to a data element value that uses
  /// the [`ValueRepresentation::UnsignedShort`] value representation.
  ///
  pub fn to_data_element_value(&self) -> DataElementValue {
    DataElementValue::new_unsigned_short(&[u16::from(u8::from(*self))]).unwrap()
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
  /// Creates a new [`PhotometricInterpretation`] from the *'(0028,0004)
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

  /// Returns whether this photometric interpretation stores grayscale pixel
  /// data.
  ///
  pub fn is_grayscale(&self) -> bool {
    match self {
      Self::Monochrome1 | Self::Monochrome2 => true,

      Self::PaletteColor { .. }
      | Self::Rgb
      | Self::YbrFull
      | Self::YbrFull422
      | Self::YbrIct
      | Self::YbrRct
      | Self::Xyb => false,
    }
  }

  /// Returns whether this photometric interpretation defines color data.
  ///
  pub fn is_color(&self) -> bool {
    !self.is_grayscale()
  }

  /// Returns whether this photometric interpretation specifies YBR color data.
  ///
  pub fn is_ybr(&self) -> bool {
    match self {
      Self::Monochrome1
      | Self::Monochrome2
      | Self::PaletteColor { .. }
      | Self::Rgb
      | Self::Xyb => false,

      Self::YbrFull | Self::YbrFull422 | Self::YbrIct | Self::YbrRct => true,
    }
  }

  /// Returns whether this photometric interpretation specifies YBR 422 color
  /// data.
  ///
  pub fn is_ybr_422(&self) -> bool {
    self == &Self::YbrFull422
  }

  /// Returns whether this photometric interpretation is
  /// [`PhotometricInterpretation::Monochrome1`].
  ///
  pub fn is_monochrome1(&self) -> bool {
    self == &Self::Monochrome1
  }

  /// Returns whether this photometric interpretation is
  /// [`PhotometricInterpretation::PaletteColor`].
  ///
  pub fn is_palette_color(&self) -> bool {
    matches!(self, Self::PaletteColor { .. })
  }

  /// Converts this photometric interpretation to a data element value that uses
  /// the [`ValueRepresentation::CodeString`] value representation.
  ///
  pub fn to_data_element_value(&self) -> DataElementValue {
    let s = match self {
      Self::Monochrome1 => "MONOCHROME1",
      Self::Monochrome2 => "MONOCHROME2",
      Self::PaletteColor { .. } => "PALETTE COLOR",
      Self::Rgb => "RGB",
      Self::YbrFull => "YBR_FULL",
      Self::YbrFull422 => "YBR_FULL_422",
      Self::YbrIct => "YBR_ICT",
      Self::YbrRct => "YBR_RCT",
      Self::Xyb => "XYB",
    };

    DataElementValue::new_code_string(&[s]).unwrap()
  }
}

impl core::fmt::Display for PhotometricInterpretation {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    let s = match self {
      Self::Monochrome1 => "Monochrome1",
      Self::Monochrome2 => "Monochrome2",
      Self::PaletteColor { .. } => "PaletteColor",
      Self::Rgb => "Rgb",
      Self::YbrFull => "YbrFull",
      Self::YbrFull422 => "YbrFull422",
      Self::YbrIct => "YbrIct",
      Self::YbrRct => "YbrRct",
      Self::Xyb => "Xyb",
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

  /// Converts this [`PlanarConfiguration`] to a data element value that uses
  /// the [`ValueRepresentation::UnsignedShort`] value representation.
  ///
  pub fn to_data_element_value(&self) -> DataElementValue {
    let value = match self {
      Self::Interleaved => 0,
      Self::Separate => 1,
    };

    DataElementValue::new_unsigned_short(&[value]).unwrap()
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
  /// Creates a new [`BitsAllocated`] from the *'(0028,0100) Bits Allocated'*
  /// data element in the given data set.
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

  /// Converts this [`BitsAllocated`] to a data element value that uses the
  /// [`ValueRepresentation::UnsignedShort`] value representation.
  ///
  pub fn to_data_element_value(&self) -> DataElementValue {
    let value = u8::from(*self);

    DataElementValue::new_unsigned_short(&[u16::from(value)]).unwrap()
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
  /// Creates a new [`PixelRepresentation`] from the *'(0028,0103) Pixel
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

  /// Converts this [`PixelRepresentation`] to a data element value that uses
  /// the [`ValueRepresentation::UnsignedShort`] value representation.
  ///
  pub fn to_data_element_value(&self) -> DataElementValue {
    let value = u8::from(*self);

    DataElementValue::new_unsigned_short(&[u16::from(value)]).unwrap()
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
