use dcmfx_core::{DataElementTag, DataError, DataSet, DataSetPath, dictionary};

use crate::luts::lookup_table::LookupTable;

/// Defines a Modality LUT that can be applied to raw pixel data stored values.
/// A modality LUT is defined either by a lookup table, or by rescale intercept
/// and slope values.
///
/// Ref: PS3.3 C.11.1.
///
#[derive(Clone, Debug, PartialEq)]
pub enum ModalityLut {
  LookupTable {
    lut: LookupTable,
    lut_type: ModalityLutOutputType,
  },

  Rescale {
    rescale_intercept: f32,
    rescale_slope: f32,
    rescale_type: ModalityLutOutputType,
  },
}

impl ModalityLut {
  /// The tags of the data elements relevant to construction of a
  /// [`ModalityLUT`].
  ///
  pub const DATA_ELEMENT_TAGS: [DataElementTag; 8] = [
    dictionary::MODALITY_LUT_SEQUENCE.tag,
    dictionary::LUT_DESCRIPTOR.tag,
    dictionary::LUT_EXPLANATION.tag,
    dictionary::MODALITY_LUT_TYPE.tag,
    dictionary::LUT_DATA.tag,
    dictionary::RESCALE_INTERCEPT.tag,
    dictionary::RESCALE_SLOPE.tag,
    dictionary::RESCALE_TYPE.tag,
  ];

  /// Creates a [`ModalityLUT`] from the relevant data elements in the given
  /// data set.
  ///
  pub fn from_data_set(data_set: &DataSet) -> Result<ModalityLut, DataError> {
    if let Ok(items) =
      data_set.get_sequence_items(dictionary::MODALITY_LUT_SEQUENCE.tag)
    {
      Self::from_modality_lut_sequence(items)
    } else if data_set.has(dictionary::RESCALE_INTERCEPT.tag) {
      Self::from_rescale(data_set)
    } else {
      Ok(Self::Rescale {
        rescale_intercept: 0.0,
        rescale_slope: 1.0,
        rescale_type: ModalityLutOutputType::Unspecified,
      })
    }
  }

  /// Creates a [`ModalityLUT`] from a Modality LUT Sequence value.
  ///
  fn from_modality_lut_sequence(items: &[DataSet]) -> Result<Self, DataError> {
    match items {
      [item] => {
        let lut = LookupTable::from_data_set(
          item,
          dictionary::LUT_DESCRIPTOR.tag,
          dictionary::LUT_DATA.tag,
          None,
          Some(dictionary::LUT_EXPLANATION.tag),
        )?;

        let lut_type = ModalityLutOutputType::from_string(
          item.get_string(dictionary::MODALITY_LUT_TYPE.tag)?,
        );

        Ok(Self::LookupTable { lut, lut_type })
      }

      _ => Err(
        DataError::new_value_invalid(
          "Modality LUT sequence does not have exactly one item".to_string(),
        )
        .with_path(&DataSetPath::new_with_data_element(
          dictionary::MODALITY_LUT_SEQUENCE.tag,
        )),
      ),
    }
  }

  /// Creates a [`ModalityLUT`] from rescale intercept and slope values.
  ///
  fn from_rescale(data_set: &DataSet) -> Result<Self, DataError> {
    let rescale_intercept =
      data_set.get_float(dictionary::RESCALE_INTERCEPT.tag)? as f32;
    let rescale_slope =
      data_set.get_float(dictionary::RESCALE_SLOPE.tag)? as f32;

    let rescale_type = if data_set.has(dictionary::RESCALE_TYPE.tag) {
      ModalityLutOutputType::from_string(
        data_set.get_string(dictionary::RESCALE_TYPE.tag)?,
      )
    } else {
      ModalityLutOutputType::Unspecified
    };

    Ok(Self::Rescale {
      rescale_intercept,
      rescale_slope,
      rescale_type,
    })
  }

  /// Returns the output type of values returned by this Modality LUT.
  ///
  pub fn output_type(&self) -> &ModalityLutOutputType {
    match self {
      Self::LookupTable { lut_type, .. } => lut_type,
      Self::Rescale { rescale_type, .. } => rescale_type,
    }
  }

  /// Looks up a stored value in this Modality LUT and returns the result. The
  /// type of the resulting value is given by [`Self::output_type()`].
  ///
  pub fn apply(&self, stored_value: i64) -> f32 {
    match self {
      Self::LookupTable { lut, .. } => lut.lookup(stored_value) as f32,

      Self::Rescale {
        rescale_intercept,
        rescale_slope,
        ..
      } => rescale_intercept + rescale_slope * (stored_value as f32),
    }
  }
}

/// Specifies the output units of a Modality LUT.
///
/// Ref: PS3.3 C.11.1.1.2.
///
#[derive(Debug, Clone, PartialEq)]
pub enum ModalityLutOutputType {
  OpticalDensity,
  HounsfieldUnits,
  Unspecified,
  MilligramsPerMilliliter,
  EffectiveAtomicNumber,
  ElectronDensity,
  ElectronDensityNormalized,
  HounsfieldUnitsModified,
  Percentage,
  Unrecognized(String),
}

impl ModalityLutOutputType {
  /// Creates a [`ModalityLutOutputType`] from a string value.
  ///
  pub fn from_string(s: &str) -> Self {
    match s {
      "OD" => Self::OpticalDensity,
      "HU" => Self::HounsfieldUnits,
      "US" => Self::Unspecified,
      "MGML" => Self::MilligramsPerMilliliter,
      "Z_EFF" => Self::EffectiveAtomicNumber,
      "ED" => Self::ElectronDensity,
      "EDW" => Self::ElectronDensityNormalized,
      "HU_MOD" => Self::HounsfieldUnitsModified,
      "PCT" => Self::Percentage,
      _ => Self::Unrecognized(s.to_string()),
    }
  }
}
