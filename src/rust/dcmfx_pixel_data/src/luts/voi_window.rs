use dcmfx_core::{DataError, DataSet, DataSetPath, dictionary};

/// Describes a single VOI LUT windowing function that can be applied in order
/// to visualize pixel data.
///
/// Taken from the *'(0028,1050) Window Center'*, *'(0028,1051) Window Width'*,
/// *'(0028,1055) Window Center & Width Explanation'*, and *'(0028,1056) VOI LUT
/// Function'* data elements.
///
/// Ref: PS3.3 C.11.2.
///
#[derive(Clone, Debug, PartialEq)]
pub struct VoiWindow {
  center: f32,
  width: f32,
  explanation: String,
  function: VoiLutFunction,

  half_width: f32,
  one_over_width: f32,
}

impl VoiWindow {
  /// Creates [`VoiWindow`]s from the relevant data elements in a data set.
  ///
  pub fn from_data_set(
    data_set: &DataSet,
  ) -> Result<Vec<VoiWindow>, DataError> {
    if !data_set.has(dictionary::WINDOW_CENTER.tag) {
      return Ok(vec![]);
    }

    let centers = data_set.get_floats(dictionary::WINDOW_CENTER.tag)?;
    let widths = data_set.get_floats(dictionary::WINDOW_WIDTH.tag)?;

    let explanations =
      if data_set.has(dictionary::WINDOW_CENTER_WIDTH_EXPLANATION.tag) {
        data_set.get_strings(dictionary::WINDOW_CENTER_WIDTH_EXPLANATION.tag)?
      } else {
        vec![""; centers.len()]
      };

    let functions = if data_set.has(dictionary::VOILUT_FUNCTION.tag) {
      data_set.get_strings(dictionary::VOILUT_FUNCTION.tag)?
    } else {
      vec!["LINEAR"; centers.len()]
    };

    if centers.len() != widths.len()
      || centers.len() != explanations.len()
      || centers.len() != functions.len()
    {
      return Err(DataError::new_value_invalid(
        "There is not the same number of VOI window widths, centers, and \
            explanations"
          .to_string(),
      ));
    }

    let mut windows = vec![];

    for i in 0..centers.len() {
      let center = centers[i] as f32;
      let width = widths[i] as f32;
      let explanation = explanations[i].to_string();
      let function = VoiLutFunction::from_string(functions[i])?;

      windows.push(VoiWindow::new(center, width, explanation, function));
    }

    Ok(windows)
  }

  /// Creates a new [`VoiWindow`] from the given values.
  ///
  pub fn new(
    center: f32,
    width: f32,
    explanation: String,
    function: VoiLutFunction,
  ) -> VoiWindow {
    // Precompute center adjustment for the Linear function
    let center = match function {
      VoiLutFunction::Linear => center - 0.5,
      _ => center,
    };

    // Ensure that the width value is valid and avoids a divide by zero
    let width = match function {
      VoiLutFunction::Linear => width.max(1.001),
      _ => width.max(0.001),
    };

    // Precompute window half width
    let half_width = match function {
      VoiLutFunction::Linear => (width - 1.0) / 2.0,
      VoiLutFunction::LinearExact | VoiLutFunction::Sigmoid => width / 2.0,
    };

    // Precompute one over the width
    let one_over_width = match function {
      VoiLutFunction::Linear => 1.0 / (width - 1.0),
      VoiLutFunction::LinearExact | VoiLutFunction::Sigmoid => 1.0 / width,
    };

    Self {
      center,
      width,
      explanation,
      function,
      half_width,
      one_over_width,
    }
  }

  /// Applies this VOI window to an input value, into an output range of 0-1.
  ///
  pub fn compute(&self, x: f32) -> f32 {
    let x = x - self.center;

    match self.function {
      VoiLutFunction::Linear | VoiLutFunction::LinearExact => {
        (0.5 + x * self.one_over_width).clamp(0.0, 1.0)
      }

      VoiLutFunction::Sigmoid => {
        1.0 / (1.0 + f32::exp(-4.0 * x * self.one_over_width))
      }
    }
  }
}

/// A VOI LUT function that uses the values of *'(0028,1050) Window Center'* and
/// *'(0028,1051) Window Width'* to convert from stored pixel values (after any
/// Modality LUT or Rescale Slope and Intercept specified in the IOD have been
/// applied) to values to be displayed.
///
/// Ref: PS3.3 C.11.2.1.3.
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VoiLutFunction {
  /// A linear conversion. This is the default function if none is specified.
  ///
  /// Ref: PS3.3 C.11.2.1.2.1.
  ///
  Linear,

  /// A linear conversion that has slightly more accurate rounding behavior.
  ///
  /// Ref: PS3.3 C.11.2.1.3.2.
  ///
  LinearExact,

  /// A sigmoidal curve conversion.
  ///
  /// Ref: PS3.3 C.11.2.1.3.1.
  ///
  Sigmoid,
}

impl VoiLutFunction {
  /// Creates a [`VoiLutFunction`] from a string value.
  ///
  pub fn from_string(s: &str) -> Result<Self, DataError> {
    match s {
      "LINEAR" => Ok(Self::Linear),
      "LINEAR_EXACT" => Ok(Self::LinearExact),
      "SIGMOID" => Ok(Self::Sigmoid),

      _ => Err(
        DataError::new_value_invalid(format!(
          "VOI LUT Function '{s}' is invalid"
        ))
        .with_path(&DataSetPath::new_with_data_element(
          dictionary::VOILUT_FUNCTION.tag,
        )),
      ),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn linear_compute() {
    let window =
      VoiWindow::new(2048.0, 4096.0, "".to_string(), VoiLutFunction::Linear);

    assert_eq!(window.compute(0.0), 0.0);
    assert_eq!(window.compute(4095.0), 1.0);
    assert_eq!(window.compute(2000.0), 0.4884004884004884);
  }

  #[test]
  fn linear_exact_compute() {
    let window = VoiWindow::new(
      2000.0,
      1000.0,
      "".to_string(),
      VoiLutFunction::LinearExact,
    );

    assert_eq!(window.compute(1500.0), 0.0);
    assert_eq!(window.compute(2500.0), 1.0);
    assert_eq!(window.compute(1800.0), 0.3);
  }

  #[test]
  fn sigmoid_compute() {
    let window =
      VoiWindow::new(2000.0, 1000.0, "".to_string(), VoiLutFunction::Sigmoid);

    assert_eq!(window.compute(1500.0), 0.11920292);
    assert_eq!(window.compute(2500.0), 0.880797);
    assert_eq!(window.compute(1800.0), 0.3100255);

    assert_eq!(window.compute(1000000000.0), 1.0);
  }

  #[test]
  fn voi_lut_function_from_string() {
    assert_eq!(
      VoiLutFunction::from_string("LINEAR"),
      Ok(VoiLutFunction::Linear)
    );
    assert_eq!(
      VoiLutFunction::from_string("LINEAR_EXACT"),
      Ok(VoiLutFunction::LinearExact)
    );
    assert_eq!(
      VoiLutFunction::from_string("SIGMOID"),
      Ok(VoiLutFunction::Sigmoid)
    );
    assert!(VoiLutFunction::from_string("OTHER").is_err());
  }
}
