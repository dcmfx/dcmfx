#[cfg(not(feature = "std"))]
use alloc::format;

use dcmfx_core::{
  DataElementTag, DataError, DataSet, DataSetPath, IodModule,
  ValueRepresentation, dictionary,
};

use crate::LookupTable;

/// The attributes of the Softcopy Presentation LUT Module that define how
/// grayscale image pixel values are transformed for display on softcopy
/// devices such as monitors.
///
/// This module supports either a linear transformation or a custom LUT, but not
/// both simultaneously. It is typically used to adjust brightness, contrast,
/// and rendering for diagnostic viewing.
///
/// Ref: PS3.3 C.11.4.
///
#[derive(Clone, Debug, PartialEq)]
pub enum SoftcopyPresentationLutModule {
  LookupTable { lut: LookupTable },
  Shape { shape: PresentationLutShape },
}

impl IodModule for SoftcopyPresentationLutModule {
  fn is_iod_module_data_element(
    tag: DataElementTag,
    _vr: ValueRepresentation,
    _length: Option<u32>,
    path: &DataSetPath,
  ) -> bool {
    if path.last_sequence_tag() == Ok(dictionary::PRESENTATION_LUT_SEQUENCE.tag)
    {
      tag == dictionary::LUT_DESCRIPTOR.tag
        || tag == dictionary::LUT_EXPLANATION.tag
        || tag == dictionary::LUT_DATA.tag
    } else {
      tag == dictionary::PRESENTATION_LUT_SEQUENCE.tag
        || tag == dictionary::PRESENTATION_LUT_SHAPE.tag
    }
  }

  fn iod_module_highest_tag() -> DataElementTag {
    dictionary::PRESENTATION_LUT_SHAPE.tag
  }

  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    match data_set.get_sequence_items(dictionary::PRESENTATION_LUT_SEQUENCE.tag)
    {
      Ok(sequence_items) => match sequence_items {
        [lut] => Ok(Self::LookupTable {
          lut: LookupTable::from_data_set(
            lut,
            dictionary::LUT_DESCRIPTOR.tag,
            dictionary::LUT_DATA.tag,
            None,
            Some(dictionary::LUT_EXPLANATION.tag),
          )?,
        }),

        _ => Err(DataError::new_multiplicity_mismatch()),
      },

      // If there's no Presentation LUT Sequence data element then look for the
      // Presentation LUT Shape data element
      Err(_) => Ok(Self::Shape {
        shape: PresentationLutShape::from_data_set(data_set)?,
      }),
    }
  }
}

impl SoftcopyPresentationLutModule {
  /// Applies this Softcopy Presentation LUT to a normalized value in the range
  /// 0-1, and outputs a normalized value in the range 0-1.
  ///
  pub fn apply(&self, value: f32) -> f32 {
    match self {
      Self::LookupTable { lut } => {
        // The input value is expanded to cover the full input range of the
        // Presentation LUT. Ref: C.11.6.1.
        lut.lookup_normalized(
          (value * ((lut.entry_count() - 1) as f32)).round() as i64,
        )
      }

      Self::Shape { shape } => match shape {
        PresentationLutShape::Identity => value,
        PresentationLutShape::Inverse => 1.0 - value,
      },
    }
  }
}

/// Specifies a predefined Presentation LUT transformation.
///
/// Ref: PS3.3 C.11.6.
///
#[derive(Clone, Debug, PartialEq)]
pub enum PresentationLutShape {
  /// No further translation necessary, input values are P-Values.
  Identity,

  /// Output values after inversion are P-Values.
  Inverse,
}

impl PresentationLutShape {
  /// Creates a new [`PresentationLutShape`] from the *'(2050,0020) Presentation
  /// LUT Shape'* data element in the given data set.
  ///
  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let tag = dictionary::PRESENTATION_LUT_SHAPE.tag;

    if !data_set.has(tag) {
      return Ok(Self::Identity);
    }

    match data_set.get_string(tag)? {
      "" | "IDENTITY" => Ok(Self::Identity),
      "INVERSE" => Ok(Self::Inverse),

      value => Err(
        DataError::new_value_invalid(format!(
          "Presentation LUT shape value of '{value}' is invalid"
        ))
        .with_path(&DataSetPath::new_with_data_element(tag)),
      ),
    }
  }
}
