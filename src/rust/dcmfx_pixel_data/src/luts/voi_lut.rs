use dcmfx_core::{DataElementTag, DataError, DataSet, dictionary};

use crate::luts::{LookupTable, VoiWindow};

/// Defines a Value Of Interest (VOI) LUT that is used to define how grayscale
/// pixel data samples are visualized.
///
/// A VOI LUT can contain multiple LUTs and multiple windows, however only one
/// of these is applied at at time.
///
/// Ref: PS3.3 C.11.2.
///
#[derive(Clone, Debug, PartialEq)]
pub struct VoiLut {
  /// The grayscale LUTs for this VOI LUT.
  pub luts: Vec<LookupTable>,

  /// The windows for this VOI LUT.
  pub windows: Vec<VoiWindow>,
}

impl VoiLut {
  /// The tags of the data elements relevant to construction of a [`VoiLut`].
  ///
  pub const DATA_ELEMENT_TAGS: [DataElementTag; 8] = [
    dictionary::VOILUT_SEQUENCE.tag,
    dictionary::LUT_DESCRIPTOR.tag,
    dictionary::LUT_EXPLANATION.tag,
    dictionary::LUT_DATA.tag,
    dictionary::WINDOW_CENTER.tag,
    dictionary::WINDOW_WIDTH.tag,
    dictionary::WINDOW_CENTER_WIDTH_EXPLANATION.tag,
    dictionary::VOILUT_FUNCTION.tag,
  ];

  /// Creates a [`VoiLut`] from the relevant data elements in a data set.
  ///
  pub fn from_data_set(data_set: &DataSet) -> Result<VoiLut, DataError> {
    let luts = if let Ok(luts_sequence) =
      data_set.get_sequence_items(dictionary::VOILUT_SEQUENCE.tag)
    {
      luts_sequence
        .iter()
        .map(|lut| {
          LookupTable::from_data_set(
            lut,
            dictionary::LUT_DESCRIPTOR.tag,
            dictionary::LUT_DATA.tag,
            None,
            Some(dictionary::LUT_EXPLANATION.tag),
          )
        })
        .collect::<Result<Vec<_>, DataError>>()?
    } else {
      vec![]
    };

    let windows = VoiWindow::from_data_set(data_set)?;

    Ok(Self { luts, windows })
  }

  /// Returns if this VOI LUT specifies either a grayscale LUT or a VOI window.
  /// Empty VOI LUTs return values unchanged from [`Self::apply()`].
  ///
  pub fn is_empty(&self) -> bool {
    self.luts.is_empty() && self.windows.is_empty()
  }

  /// Returns the grayscale LUTs specified in this VOI LUT, if any.
  ///
  pub fn luts(&self) -> &[LookupTable] {
    &self.luts
  }

  /// Returns the windows specified in this VOI LUT, if any.
  ///
  pub fn windows(&self) -> &[VoiWindow] {
    &self.windows
  }

  /// Applies this VOI LUT to an input value. If there are any grayscale LUTs
  /// specified then the first one is used, otherwise if there are any windows
  /// specified then the first one is used. If there are no grayscale LUTs and
  /// no windows then the input value is returned unaltered.
  ///
  pub fn apply(&self, x: f32) -> f32 {
    if let Some(lut) = self.luts.first() {
      lut.lookup_normalized(x as i64)
    } else if let Some(window) = self.windows.first() {
      window.compute(x)
    } else {
      x
    }
  }
}
