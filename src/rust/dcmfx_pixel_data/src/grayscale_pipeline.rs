use core::cell::{Ref, RefCell};

#[cfg(not(feature = "std"))]
use alloc::vec;

use dcmfx_core::{
  DataElementTag, DataError, DataSet, DataSetPath, IodModule,
  ValueRepresentation,
};

use crate::{
  StoredValueOutputCache,
  iods::{
    ModalityLutModule, SoftcopyPresentationLutModule, VoiLutModule,
    voi_lut_module::VoiWindow,
  },
};

/// The grayscale pipeline consists of the Modality LUT, the VOI LUT, and the
/// Presentation LUT, which are each defined by a separate IOD module. Together
/// they make up the grayscale pipeline which defines how raw stored values in
/// pixel data are transformed into final grayscale values suitable for display.
///
/// Pixel Data
///    ↓
/// Modality LUT (→ physical units)
///    ↓
/// VOI LUT (→ region of interest emphasized)
///    ↓
/// Softcopy Presentation LUT (→ display intensities)
///    ↓
/// Final Display
///
/// Ref: PS3.3 C.11.
///
#[derive(Clone, Debug, PartialEq)]
pub struct GrayscalePipeline {
  stored_value_range: core::ops::RangeInclusive<i64>,
  modality_lut_module: ModalityLutModule,
  modality_lut_output_range: core::ops::RangeInclusive<f32>,
  voi_lut_module: VoiLutModule,
  softcopy_presentation_lut_module: SoftcopyPresentationLutModule,

  // Internal caches used when the stored value range has <= 2^16 items
  output_cache_u8: RefCell<Option<StoredValueOutputCache<u8>>>,
  output_cache_u16: RefCell<Option<StoredValueOutputCache<u16>>>,
}

impl GrayscalePipeline {
  /// Returns whether the specified data element is needed for the construction
  /// of a [`GrayscalePipeline`].
  ///
  pub fn is_iod_module_data_element(
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: Option<u32>,
    path: &DataSetPath,
  ) -> bool {
    ModalityLutModule::is_iod_module_data_element(tag, vr, length, path)
      || VoiLutModule::is_iod_module_data_element(tag, vr, length, path)
      || SoftcopyPresentationLutModule::is_iod_module_data_element(
        tag, vr, length, path,
      )
  }

  /// Returns the highest data element tag that can return true from
  /// [`Self::is_iod_module_data_element()`]
  ///
  pub fn iod_module_highest_tag() -> DataElementTag {
    ModalityLutModule::iod_module_highest_tag()
      .max(VoiLutModule::iod_module_highest_tag())
      .max(SoftcopyPresentationLutModule::iod_module_highest_tag())
  }

  /// Creates a [`GrayscalePipeline`] from a data set and a range of possible
  /// input stored values.
  ///
  pub fn from_data_set(
    data_set: &DataSet,
    stored_value_range: core::ops::RangeInclusive<i64>,
  ) -> Result<Self, DataError> {
    let modality_lut_module = ModalityLutModule::from_data_set(data_set)?;
    let voi_lut_module = VoiLutModule::from_data_set(data_set)?;
    let softcopy_presentation_lut_module =
      SoftcopyPresentationLutModule::from_data_set(data_set)?;

    let modality_lut_output_range =
      modality_lut_module.output_range(&stored_value_range);

    Ok(GrayscalePipeline {
      stored_value_range,
      modality_lut_module,
      modality_lut_output_range,
      voi_lut_module,
      softcopy_presentation_lut_module,

      output_cache_u8: RefCell::new(None),
      output_cache_u16: RefCell::new(None),
    })
  }

  /// Returns the Modality LUT for this grayscale pipeline.
  ///
  pub fn modality_lut(&self) -> &ModalityLutModule {
    &self.modality_lut_module
  }

  /// Returns the VOI LUT for this grayscale pipeline.
  ///
  pub fn voi_lut(&self) -> &VoiLutModule {
    &self.voi_lut_module
  }

  /// Returns the Softcopy Presentation LUT for this grayscale pipeline.
  ///
  pub fn softcopy_presentation_lut(&self) -> &SoftcopyPresentationLutModule {
    &self.softcopy_presentation_lut_module
  }

  /// Sets the VOI window to use in the VOI LUT, overriding the currently active
  /// VOI LUT configuration.
  ///
  pub fn set_voi_window(&mut self, window: VoiWindow) {
    self.voi_lut_module = VoiLutModule {
      luts: vec![],
      windows: vec![window],
    };

    // Clear caches
    *self.output_cache_u8.get_mut() = None;
    *self.output_cache_u16.get_mut() = None;
  }

  /// Takes a stored value from pixel data and passes in through the Modality
  /// LUT, VOI LUT, and Softcopy Presentation LUT modules to get a normalized
  /// final Presentation Value (P-Value).
  ///
  pub fn apply(&self, stored_value: i64) -> f32 {
    let mut x = self.modality_lut_module.apply_to_stored_value(stored_value);

    x = if self.voi_lut_module.is_empty() {
      let start = self.modality_lut_output_range.start();
      let end = self.modality_lut_output_range.end();

      // Normalize value inside the input range
      ((x - start) / (end - start)).clamp(0.0, 1.0)
    } else {
      self.voi_lut_module.apply(x)
    };

    self.softcopy_presentation_lut_module.apply(x)
  }

  /// The same as [`Self::apply()`] but the normalized final Presentation Value
  /// (P-Value) is converted to a `u8`.
  ///
  pub fn apply_u8(&self, stored_value: i64) -> u8 {
    (self.apply(stored_value) * 255.0).round().clamp(0.0, 255.0) as u8
  }

  /// The same as [`Self::apply()`] but the normalized final Presentation Value
  /// (P-Value) is converted to a `u16`.
  ///
  pub fn apply_u16(&self, stored_value: i64) -> u16 {
    (self.apply(stored_value) * 65535.0)
      .round()
      .clamp(0.0, 65535.0) as u16
  }

  /// Returns the cache for converting a pixel data stored value into a final
  /// `u8` Presentation Value (P-Value) using this grayscale pipeline.
  ///
  /// Caches are only available when the stored value range has <= 2^16 items.
  ///
  pub fn output_cache_u8(&self) -> Ref<Option<StoredValueOutputCache<u8>>> {
    if let Ok(mut output_cache_u8) = self.output_cache_u8.try_borrow_mut() {
      if output_cache_u8.is_none() && self.is_stored_value_range_cacheable() {
        *output_cache_u8 = Some(StoredValueOutputCache::new(
          &self.stored_value_range,
          |pixel| self.apply_u8(pixel),
        ));
      }
    }

    self.output_cache_u8.borrow()
  }

  /// Returns the cache for converting a pixel data stored value into a final
  /// `u16` Presentation Value (P-Value) using this grayscale pipeline.
  ///
  /// Caches are only available when the stored value range has <= 2^16 items.
  ///
  pub fn output_cache_u16(&self) -> Ref<Option<StoredValueOutputCache<u16>>> {
    if let Ok(mut output_cache_u16) = self.output_cache_u16.try_borrow_mut() {
      if output_cache_u16.is_none() && self.is_stored_value_range_cacheable() {
        *output_cache_u16 = Some(StoredValueOutputCache::new(
          &self.stored_value_range,
          |pixel| self.apply_u16(pixel),
        ));
      }
    }

    self.output_cache_u16.borrow()
  }

  /// Controls whether the stored value range will be cached. Caching only
  /// occurs when the range of stored values has <= 2^16 items.
  ///
  fn is_stored_value_range_cacheable(&self) -> bool {
    self.stored_value_range.end() - self.stored_value_range.start() < 65536
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use dcmfx_core::dictionary::*;

  #[test]
  fn test_empty_pipeline() {
    let pipeline =
      GrayscalePipeline::from_data_set(&DataSet::new(), 0..=100).unwrap();

    assert_eq!(pipeline.apply(0), 0.0);
    assert_eq!(pipeline.apply(50), 0.5);
    assert_eq!(pipeline.apply(100), 1.0);
  }

  #[test]
  fn test_modality_lut_rescale_without_voi_lut() {
    let mut data_set = DataSet::new();
    data_set
      .insert_float_value(&RESCALE_INTERCEPT, &[25.0])
      .unwrap();
    data_set.insert_float_value(&RESCALE_SLOPE, &[2.0]).unwrap();

    let pipeline =
      GrayscalePipeline::from_data_set(&data_set, -128..=127).unwrap();

    assert_eq!(pipeline.apply(-128), 0.0);
    assert_eq!(pipeline.apply(127), 1.0);
  }

  #[test]
  fn test_modality_rescale_lut_and_voi_lut() {
    let mut data_set = DataSet::new();
    data_set
      .insert_float_value(&RESCALE_INTERCEPT, &[25.0])
      .unwrap();
    data_set.insert_float_value(&RESCALE_SLOPE, &[2.0]).unwrap();

    data_set.insert_float_value(&WINDOW_CENTER, &[0.0]).unwrap();
    data_set
      .insert_float_value(&WINDOW_WIDTH, &[500.0])
      .unwrap();
    data_set
      .insert_string_value(&VOILUT_FUNCTION, &["LINEAR_EXACT"])
      .unwrap();

    let pipeline =
      GrayscalePipeline::from_data_set(&data_set, -128..=127).unwrap();

    assert_eq!(pipeline.apply(-128), 0.5 + (-128.0 * 2.0 + 25.0) / 500.0);
    assert_eq!(pipeline.apply(-50), 0.5 + (-50.0 * 2.0 + 25.0) / 500.0);
    assert_eq!(pipeline.apply(64), 0.5 + (64.0 * 2.0 + 25.0) / 500.0);
  }

  #[test]
  fn test_presentation_lut_shape_inverse() {
    let mut data_set = DataSet::new();
    data_set
      .insert_string_value(&PRESENTATION_LUT_SHAPE, &["INVERSE"])
      .unwrap();

    let pipeline =
      GrayscalePipeline::from_data_set(&data_set, 0..=100).unwrap();

    assert_eq!(pipeline.apply(100), 0.0);
    assert_eq!(pipeline.apply(50), 0.5);
    assert_eq!(pipeline.apply(0), 1.0);
  }
}
