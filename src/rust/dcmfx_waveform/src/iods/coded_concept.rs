//! A coded concept stored in an item of a code sequence.

#[cfg(not(feature = "std"))]
use alloc::string::String;

use dcmfx_core::{DataElementTag, DataError, DataSet, dictionary};

use crate::iods::{get_optional_string, get_single_sequence_item};

/// A single coded concept read from an item of a code sequence, e.g. the
/// *'(003A,0208) Channel Source Sequence'* or the *'(003A,0211) Channel
/// Sensitivity Units Sequence'*.
///
/// Ref: PS3.3 Section 8.8.
///
#[derive(Clone, Debug, PartialEq)]
pub struct CodedConcept {
  pub code_value: Option<String>,
  pub coding_scheme_designator: Option<String>,
  pub coding_scheme_version: Option<String>,
  pub code_meaning: Option<String>,
}

impl CodedConcept {
  /// The data element tags used when reading [`CodedConcept`].
  ///
  pub const TAGS: [DataElementTag; 4] = [
    dictionary::CODE_VALUE.tag,
    dictionary::CODING_SCHEME_DESIGNATOR.tag,
    dictionary::CODING_SCHEME_VERSION.tag,
    dictionary::CODE_MEANING.tag,
  ];

  /// Creates a new [`CodedConcept`] from an item of a code sequence.
  ///
  pub fn from_data_set(item: &DataSet) -> Result<Self, DataError> {
    Ok(Self {
      code_value: get_optional_string(item, dictionary::CODE_VALUE.tag)?,
      coding_scheme_designator: get_optional_string(
        item,
        dictionary::CODING_SCHEME_DESIGNATOR.tag,
      )?,
      coding_scheme_version: get_optional_string(
        item,
        dictionary::CODING_SCHEME_VERSION.tag,
      )?,
      code_meaning: get_optional_string(item, dictionary::CODE_MEANING.tag)?,
    })
  }

  /// Creates a new [`CodedConcept`] from a code sequence that is required to
  /// have exactly one item.
  ///
  pub(crate) fn from_single_item_sequence(
    data_set: &DataSet,
    tag: DataElementTag,
  ) -> Result<Self, DataError> {
    Self::from_data_set(get_single_sequence_item(data_set, tag)?)
  }

  /// Converts this coded concept to a code sequence item data set.
  ///
  pub fn to_data_set(&self) -> Result<DataSet, DataError> {
    let mut item = DataSet::new();

    if let Some(code_value) = &self.code_value {
      item.insert_string_value(&dictionary::CODE_VALUE, &[code_value])?;
    }

    if let Some(coding_scheme_designator) = &self.coding_scheme_designator {
      item.insert_string_value(
        &dictionary::CODING_SCHEME_DESIGNATOR,
        &[coding_scheme_designator],
      )?;
    }

    if let Some(coding_scheme_version) = &self.coding_scheme_version {
      item.insert_string_value(
        &dictionary::CODING_SCHEME_VERSION,
        &[coding_scheme_version],
      )?;
    }

    if let Some(code_meaning) = &self.code_meaning {
      item.insert_string_value(&dictionary::CODE_MEANING, &[code_meaning])?;
    }

    Ok(item)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_data_set_from_data_set_round_trip() {
    let coded_concept = CodedConcept {
      code_value: Some("5.6.3-9-1".to_string()),
      coding_scheme_designator: Some("SCPECG".to_string()),
      coding_scheme_version: Some("1.3".to_string()),
      code_meaning: Some("Lead I (Einthoven)".to_string()),
    };

    assert_eq!(
      CodedConcept::from_data_set(&coded_concept.to_data_set().unwrap()),
      Ok(coded_concept)
    );

    let coded_concept = CodedConcept {
      code_value: Some("uV".to_string()),
      coding_scheme_designator: Some("UCUM".to_string()),
      coding_scheme_version: None,
      code_meaning: None,
    };

    assert_eq!(
      CodedConcept::from_data_set(&coded_concept.to_data_set().unwrap()),
      Ok(coded_concept)
    );
  }

  #[test]
  fn from_data_set_on_empty_item() {
    assert_eq!(
      CodedConcept::from_data_set(&DataSet::new()),
      Ok(CodedConcept {
        code_value: None,
        coding_scheme_designator: None,
        coding_scheme_version: None,
        code_meaning: None,
      })
    );
  }
}
