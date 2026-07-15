//! A single item of a Channel Definition Sequence.

#[cfg(not(feature = "std"))]
use alloc::{
  string::{String, ToString},
  vec,
  vec::Vec,
};

use dcmfx_core::{DataElementTag, DataError, DataSet, dictionary};

use crate::iods::coded_concept::CodedConcept;
use crate::iods::waveform_module::WaveformSampleInterpretation;
use crate::iods::{
  get_optional_float, get_optional_stored_value, get_optional_string,
  insert_stored_value,
};

/// The definition of a single channel of a waveform multiplex group, read
/// from an item of the *'(003A,0200) Channel Definition Sequence'*.
///
/// Ref: PS3.3 C.10.9.
///
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelDefinition {
  /// *'(003A,0202) Waveform Channel Number'*.
  pub waveform_channel_number: Option<i32>,

  /// *'(003A,0203) Channel Label'*.
  pub label: Option<String>,

  /// *'(003A,0205) Channel Status'*. Empty when not present in the channel
  /// definition.
  pub status: Vec<ChannelStatus>,

  /// *'(003A,0208) Channel Source Sequence'*.
  pub source: CodedConcept,

  /// *'(003A,020C) Channel Derivation Description'*.
  pub derivation_description: Option<String>,

  /// *'(003A,0210) Channel Sensitivity'*.
  pub sensitivity: Option<f64>,

  /// *'(003A,0211) Channel Sensitivity Units Sequence'*.
  pub sensitivity_units: Option<CodedConcept>,

  /// *'(003A,0212) Channel Sensitivity Correction Factor'*.
  pub sensitivity_correction_factor: Option<f64>,

  /// *'(003A,0213) Channel Baseline'*.
  pub baseline: Option<f64>,

  /// *'(003A,0214) Channel Time Skew'*.
  pub time_skew: Option<f64>,

  /// *'(003A,0215) Channel Sample Skew'*.
  pub sample_skew: Option<f64>,

  /// *'(003A,0218) Channel Offset'*.
  pub offset: Option<f64>,

  /// *'(003A,021A) Waveform Bits Stored'*.
  pub bits_stored: u16,

  /// *'(003A,0317) Waveform Amplifier Type'*.
  pub amplifier_type: Option<String>,

  /// *'(003A,0220) Filter Low Frequency'*.
  pub filter_low_frequency: Option<f64>,

  /// *'(003A,0221) Filter High Frequency'*.
  pub filter_high_frequency: Option<f64>,

  /// *'(003A,0222) Notch Filter Frequency'*.
  pub notch_filter_frequency: Option<f64>,

  /// *'(003A,0223) Notch Filter Bandwidth'*.
  pub notch_filter_bandwidth: Option<f64>,

  /// *'(5400,0110) Channel Minimum Value'*, decoded to the raw stored value
  /// using the multiplex group's sample interpretation.
  pub minimum_value: Option<i64>,

  /// *'(5400,0112) Channel Maximum Value'*, decoded to the raw stored value
  /// using the multiplex group's sample interpretation.
  pub maximum_value: Option<i64>,
}

impl ChannelDefinition {
  /// The data element tags used when reading [`ChannelDefinition`].
  ///
  pub const TAGS: [DataElementTag; 20] = [
    dictionary::WAVEFORM_CHANNEL_NUMBER.tag,
    dictionary::CHANNEL_LABEL.tag,
    dictionary::CHANNEL_STATUS.tag,
    dictionary::CHANNEL_SOURCE_SEQUENCE.tag,
    dictionary::CHANNEL_DERIVATION_DESCRIPTION.tag,
    dictionary::CHANNEL_SENSITIVITY.tag,
    dictionary::CHANNEL_SENSITIVITY_UNITS_SEQUENCE.tag,
    dictionary::CHANNEL_SENSITIVITY_CORRECTION_FACTOR.tag,
    dictionary::CHANNEL_BASELINE.tag,
    dictionary::CHANNEL_TIME_SKEW.tag,
    dictionary::CHANNEL_SAMPLE_SKEW.tag,
    dictionary::CHANNEL_OFFSET.tag,
    dictionary::WAVEFORM_BITS_STORED.tag,
    dictionary::WAVEFORM_AMPLIFIER_TYPE.tag,
    dictionary::FILTER_LOW_FREQUENCY.tag,
    dictionary::FILTER_HIGH_FREQUENCY.tag,
    dictionary::NOTCH_FILTER_FREQUENCY.tag,
    dictionary::NOTCH_FILTER_BANDWIDTH.tag,
    dictionary::CHANNEL_MINIMUM_VALUE.tag,
    dictionary::CHANNEL_MAXIMUM_VALUE.tag,
  ];

  /// Creates a new [`ChannelDefinition`] from an item of a *'(003A,0200)
  /// Channel Definition Sequence'*. The sample interpretation of the multiplex
  /// group is needed to decode the channel minimum and maximum values.
  ///
  pub fn from_data_set(
    item: &DataSet,
    sample_interpretation: WaveformSampleInterpretation,
  ) -> Result<Self, DataError> {
    let waveform_channel_number =
      if item.has(dictionary::WAVEFORM_CHANNEL_NUMBER.tag) {
        Some(item.get_int::<i32>(dictionary::WAVEFORM_CHANNEL_NUMBER.tag)?)
      } else {
        None
      };

    let label = if item.has(dictionary::CHANNEL_LABEL.tag) {
      Some(item.get_string(dictionary::CHANNEL_LABEL.tag)?.to_string())
    } else {
      None
    };

    let status = if item.has(dictionary::CHANNEL_STATUS.tag) {
      item
        .get_strings(dictionary::CHANNEL_STATUS.tag)?
        .iter()
        .map(|s| ChannelStatus::from_string(s))
        .collect()
    } else {
      vec![]
    };

    let source = CodedConcept::from_single_item_sequence(
      item,
      dictionary::CHANNEL_SOURCE_SEQUENCE.tag,
    )?;

    let derivation_description = get_optional_string(
      item,
      dictionary::CHANNEL_DERIVATION_DESCRIPTION.tag,
    )?;

    let sensitivity =
      get_optional_float(item, dictionary::CHANNEL_SENSITIVITY.tag)?;

    let sensitivity_units =
      if item.has(dictionary::CHANNEL_SENSITIVITY_UNITS_SEQUENCE.tag) {
        Some(CodedConcept::from_single_item_sequence(
          item,
          dictionary::CHANNEL_SENSITIVITY_UNITS_SEQUENCE.tag,
        )?)
      } else {
        None
      };

    let sensitivity_correction_factor = get_optional_float(
      item,
      dictionary::CHANNEL_SENSITIVITY_CORRECTION_FACTOR.tag,
    )?;

    let baseline = get_optional_float(item, dictionary::CHANNEL_BASELINE.tag)?;
    let time_skew =
      get_optional_float(item, dictionary::CHANNEL_TIME_SKEW.tag)?;
    let sample_skew =
      get_optional_float(item, dictionary::CHANNEL_SAMPLE_SKEW.tag)?;
    let offset = get_optional_float(item, dictionary::CHANNEL_OFFSET.tag)?;

    let bits_stored =
      item.get_int::<u16>(dictionary::WAVEFORM_BITS_STORED.tag)?;

    let amplifier_type =
      get_optional_string(item, dictionary::WAVEFORM_AMPLIFIER_TYPE.tag)?;

    let filter_low_frequency =
      get_optional_float(item, dictionary::FILTER_LOW_FREQUENCY.tag)?;
    let filter_high_frequency =
      get_optional_float(item, dictionary::FILTER_HIGH_FREQUENCY.tag)?;
    let notch_filter_frequency =
      get_optional_float(item, dictionary::NOTCH_FILTER_FREQUENCY.tag)?;
    let notch_filter_bandwidth =
      get_optional_float(item, dictionary::NOTCH_FILTER_BANDWIDTH.tag)?;

    let minimum_value = get_optional_stored_value(
      item,
      dictionary::CHANNEL_MINIMUM_VALUE.tag,
      sample_interpretation,
    )?;
    let maximum_value = get_optional_stored_value(
      item,
      dictionary::CHANNEL_MAXIMUM_VALUE.tag,
      sample_interpretation,
    )?;

    Ok(Self {
      waveform_channel_number,
      label,
      status,
      source,
      derivation_description,
      sensitivity,
      sensitivity_units,
      sensitivity_correction_factor,
      baseline,
      time_skew,
      sample_skew,
      offset,
      bits_stored,
      amplifier_type,
      filter_low_frequency,
      filter_high_frequency,
      notch_filter_frequency,
      notch_filter_bandwidth,
      minimum_value,
      maximum_value,
    })
  }

  /// Converts this channel definition to a data set for storing in an item of
  /// a *'(003A,0200) Channel Definition Sequence'*. The sample interpretation
  /// of the multiplex group is needed to encode the channel minimum and
  /// maximum values.
  ///
  pub fn to_data_set(
    &self,
    sample_interpretation: WaveformSampleInterpretation,
  ) -> Result<DataSet, DataError> {
    let mut item = DataSet::new();

    if let Some(waveform_channel_number) = self.waveform_channel_number {
      item.insert_int_value(
        &dictionary::WAVEFORM_CHANNEL_NUMBER,
        &[i64::from(waveform_channel_number)],
      )?;
    }

    if let Some(label) = &self.label {
      item.insert_string_value(&dictionary::CHANNEL_LABEL, &[label])?;
    }

    if !self.status.is_empty() {
      let status: Vec<&str> = self.status.iter().map(|s| s.as_str()).collect();

      item.insert_string_value(&dictionary::CHANNEL_STATUS, &status)?;
    }

    item.insert_sequence_value(
      &dictionary::CHANNEL_SOURCE_SEQUENCE,
      vec![self.source.to_data_set()?],
    )?;

    if let Some(derivation_description) = &self.derivation_description {
      item.insert_string_value(
        &dictionary::CHANNEL_DERIVATION_DESCRIPTION,
        &[derivation_description],
      )?;
    }

    if let Some(sensitivity) = self.sensitivity {
      item
        .insert_float_value(&dictionary::CHANNEL_SENSITIVITY, &[sensitivity])?;
    }

    if let Some(sensitivity_units) = &self.sensitivity_units {
      item.insert_sequence_value(
        &dictionary::CHANNEL_SENSITIVITY_UNITS_SEQUENCE,
        vec![sensitivity_units.to_data_set()?],
      )?;
    }

    if let Some(sensitivity_correction_factor) =
      self.sensitivity_correction_factor
    {
      item.insert_float_value(
        &dictionary::CHANNEL_SENSITIVITY_CORRECTION_FACTOR,
        &[sensitivity_correction_factor],
      )?;
    }

    if let Some(baseline) = self.baseline {
      item.insert_float_value(&dictionary::CHANNEL_BASELINE, &[baseline])?;
    }

    if let Some(time_skew) = self.time_skew {
      item.insert_float_value(&dictionary::CHANNEL_TIME_SKEW, &[time_skew])?;
    }

    if let Some(sample_skew) = self.sample_skew {
      item
        .insert_float_value(&dictionary::CHANNEL_SAMPLE_SKEW, &[sample_skew])?;
    }

    if let Some(offset) = self.offset {
      item.insert_float_value(&dictionary::CHANNEL_OFFSET, &[offset])?;
    }

    item.insert_int_value(
      &dictionary::WAVEFORM_BITS_STORED,
      &[i64::from(self.bits_stored)],
    )?;

    if let Some(amplifier_type) = &self.amplifier_type {
      item.insert_string_value(
        &dictionary::WAVEFORM_AMPLIFIER_TYPE,
        &[amplifier_type],
      )?;
    }

    if let Some(filter_low_frequency) = self.filter_low_frequency {
      item.insert_float_value(
        &dictionary::FILTER_LOW_FREQUENCY,
        &[filter_low_frequency],
      )?;
    }

    if let Some(filter_high_frequency) = self.filter_high_frequency {
      item.insert_float_value(
        &dictionary::FILTER_HIGH_FREQUENCY,
        &[filter_high_frequency],
      )?;
    }

    if let Some(notch_filter_frequency) = self.notch_filter_frequency {
      item.insert_float_value(
        &dictionary::NOTCH_FILTER_FREQUENCY,
        &[notch_filter_frequency],
      )?;
    }

    if let Some(notch_filter_bandwidth) = self.notch_filter_bandwidth {
      item.insert_float_value(
        &dictionary::NOTCH_FILTER_BANDWIDTH,
        &[notch_filter_bandwidth],
      )?;
    }

    if let Some(minimum_value) = self.minimum_value {
      insert_stored_value(
        &mut item,
        dictionary::CHANNEL_MINIMUM_VALUE.tag,
        minimum_value,
        sample_interpretation,
      )?;
    }

    if let Some(maximum_value) = self.maximum_value {
      insert_stored_value(
        &mut item,
        dictionary::CHANNEL_MAXIMUM_VALUE.tag,
        maximum_value,
        sample_interpretation,
      )?;
    }

    Ok(item)
  }
}

/// The status of a waveform channel, as stored in the *'(003A,0205) Channel
/// Status'* data element.
///
/// Ref: PS3.3 C.10.9.1.4.2.
///
#[derive(Clone, Debug, PartialEq)]
pub enum ChannelStatus {
  Ok,
  TestData,
  Disconnected,
  Questionable,
  Invalid,
  Uncalibrated,
  Unzeroed,
  Unrecognized(String),
}

impl ChannelStatus {
  /// Creates a [`ChannelStatus`] from a string value.
  ///
  pub fn from_string(s: &str) -> Self {
    match s {
      "OK" => Self::Ok,
      "TEST DATA" => Self::TestData,
      "DISCONNECTED" => Self::Disconnected,
      "QUESTIONABLE" => Self::Questionable,
      "INVALID" => Self::Invalid,
      "UNCALIBRATED" => Self::Uncalibrated,
      "UNZEROED" => Self::Unzeroed,
      _ => Self::Unrecognized(s.to_string()),
    }
  }

  /// Returns the string value for this channel status.
  ///
  pub fn as_str(&self) -> &str {
    match self {
      Self::Ok => "OK",
      Self::TestData => "TEST DATA",
      Self::Disconnected => "DISCONNECTED",
      Self::Questionable => "QUESTIONABLE",
      Self::Invalid => "INVALID",
      Self::Uncalibrated => "UNCALIBRATED",
      Self::Unzeroed => "UNZEROED",
      Self::Unrecognized(s) => s,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_channel_definition() -> ChannelDefinition {
    ChannelDefinition {
      waveform_channel_number: Some(1),
      label: Some("Lead I".to_string()),
      status: vec![ChannelStatus::Ok, ChannelStatus::Uncalibrated],
      source: CodedConcept {
        code_value: Some("5.6.3-9-1".to_string()),
        coding_scheme_designator: Some("SCPECG".to_string()),
        coding_scheme_version: Some("1.3".to_string()),
        code_meaning: Some("Lead I (Einthoven)".to_string()),
      },
      derivation_description: Some("Filtered and averaged".to_string()),
      sensitivity: Some(1.25),
      sensitivity_units: Some(CodedConcept {
        code_value: Some("uV".to_string()),
        coding_scheme_designator: Some("UCUM".to_string()),
        coding_scheme_version: Some("1.4".to_string()),
        code_meaning: Some("microvolt".to_string()),
      }),
      sensitivity_correction_factor: Some(1.0),
      baseline: Some(0.0),
      time_skew: None,
      sample_skew: Some(0.0),
      offset: None,
      bits_stored: 16,
      amplifier_type: Some("AC".to_string()),
      filter_low_frequency: Some(0.05),
      filter_high_frequency: Some(300.0),
      notch_filter_frequency: None,
      notch_filter_bandwidth: None,
      minimum_value: Some(-500),
      maximum_value: Some(1000),
    }
  }

  #[test]
  fn to_data_set_from_data_set_round_trip() {
    let channel_definition = test_channel_definition();

    let item = channel_definition
      .to_data_set(WaveformSampleInterpretation::SignedShort)
      .unwrap();

    assert_eq!(
      ChannelDefinition::from_data_set(
        &item,
        WaveformSampleInterpretation::SignedShort
      ),
      Ok(channel_definition)
    );
  }

  #[test]
  fn from_data_set_requires_channel_source_sequence() {
    let mut item = test_channel_definition()
      .to_data_set(WaveformSampleInterpretation::SignedShort)
      .unwrap();
    item.delete(dictionary::CHANNEL_SOURCE_SEQUENCE.tag);

    assert!(
      ChannelDefinition::from_data_set(
        &item,
        WaveformSampleInterpretation::SignedShort
      )
      .is_err()
    );
  }

  #[test]
  fn channel_status_from_string() {
    assert_eq!(ChannelStatus::from_string("OK"), ChannelStatus::Ok);
    assert_eq!(
      ChannelStatus::from_string("TEST DATA"),
      ChannelStatus::TestData
    );
    assert_eq!(
      ChannelStatus::from_string("SOMETHING ELSE"),
      ChannelStatus::Unrecognized("SOMETHING ELSE".to_string())
    );
  }
}
