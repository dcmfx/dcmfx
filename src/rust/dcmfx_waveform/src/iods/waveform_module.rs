//! Specifies values of data elements in the Waveform module.

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec::Vec,
};

use dcmfx_core::{
  DataElementTag, DataElementValue, DataError, DataSet, DataSetPath, IodModule,
  ValueRepresentation, dictionary,
};

use crate::encode;
use crate::iods::channel_definition::ChannelDefinition;
use crate::iods::coded_concept::CodedConcept;
use crate::iods::{
  get_optional_float, get_optional_stored_value, get_optional_string,
  insert_stored_value,
};

/// Holds values of all of the data elements in the Waveform module, which
/// describes multi-channel time-based digitized waveforms such as ECGs,
/// hemodynamic waveforms, and audio.
///
/// The waveform data itself is not held; it is accessed as
/// [`WaveformChunk`](crate::WaveformChunk)s, e.g. via
/// [`crate::transforms::P10WaveformChunkTransform`].
///
/// Ref: PS3.3 C.10.9.
///
#[derive(Clone, Debug, PartialEq)]
pub struct WaveformModule {
  multiplex_groups: Vec<WaveformMultiplexGroup>,
}

impl IodModule for WaveformModule {
  fn is_iod_module_data_element(
    tag: DataElementTag,
    _vr: ValueRepresentation,
    _length: Option<u32>,
    path: &DataSetPath,
  ) -> bool {
    if path.is_root() {
      return tag == dictionary::WAVEFORM_SEQUENCE.tag;
    }

    match path.last_sequence_tag() {
      Ok(sequence_tag) => {
        if sequence_tag == dictionary::WAVEFORM_SEQUENCE.tag {
          WaveformMultiplexGroup::TAGS.contains(&tag)
        } else if sequence_tag == dictionary::CHANNEL_DEFINITION_SEQUENCE.tag {
          ChannelDefinition::TAGS.contains(&tag)
        } else if sequence_tag == dictionary::CHANNEL_SOURCE_SEQUENCE.tag
          || sequence_tag == dictionary::CHANNEL_SENSITIVITY_UNITS_SEQUENCE.tag
        {
          CodedConcept::TAGS.contains(&tag)
        } else {
          false
        }
      }

      Err(()) => false,
    }
  }

  fn iod_module_highest_tag() -> DataElementTag {
    dictionary::WAVEFORM_DATA.tag
  }

  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let items =
      data_set.get_sequence_items(dictionary::WAVEFORM_SEQUENCE.tag)?;

    let multiplex_groups = items
      .iter()
      .map(WaveformMultiplexGroup::from_data_set)
      .collect::<Result<Vec<_>, DataError>>()?;

    Ok(Self { multiplex_groups })
  }
}

impl WaveformModule {
  /// Creates a new [`WaveformModule`] with the given waveform multiplex
  /// groups.
  ///
  pub fn new(multiplex_groups: Vec<WaveformMultiplexGroup>) -> Self {
    Self { multiplex_groups }
  }

  /// Returns this Waveform module's multiplex groups, i.e. the items of its
  /// *'(5400,0100) Waveform Sequence'*.
  ///
  pub fn multiplex_groups(&self) -> &[WaveformMultiplexGroup] {
    &self.multiplex_groups
  }
}

/// A single waveform multiplex group, i.e. an item of the *'(5400,0100)
/// Waveform Sequence'*, which holds a set of channels that are digitized
/// synchronously at a common sampling frequency.
///
/// The waveform data itself is not held; it is accessed as
/// [`WaveformChunk`](crate::WaveformChunk)s that are decoded with
/// [`WaveformChunk::channel_samples()`](crate::WaveformChunk::channel_samples).
///
/// Ref: PS3.3 C.10.9.
///
#[derive(Clone, Debug, PartialEq)]
pub struct WaveformMultiplexGroup {
  originality: WaveformOriginality,
  number_of_channels: u16,
  number_of_samples: u32,
  sampling_frequency: f64,
  time_offset: Option<f64>,
  trigger_time_offset: Option<f64>,
  trigger_sample_position: Option<u32>,
  label: Option<String>,
  uid: Option<String>,
  channels: Vec<ChannelDefinition>,
  bits_allocated: WaveformBitsAllocated,
  sample_interpretation: WaveformSampleInterpretation,
  padding_value: Option<i64>,
}

impl WaveformMultiplexGroup {
  /// The data element tags used when reading [`WaveformMultiplexGroup`].
  ///
  pub const TAGS: [DataElementTag; 13] = [
    dictionary::MULTIPLEX_GROUP_TIME_OFFSET.tag,
    dictionary::TRIGGER_TIME_OFFSET.tag,
    dictionary::TRIGGER_SAMPLE_POSITION.tag,
    dictionary::WAVEFORM_ORIGINALITY.tag,
    dictionary::NUMBER_OF_WAVEFORM_CHANNELS.tag,
    dictionary::NUMBER_OF_WAVEFORM_SAMPLES.tag,
    dictionary::SAMPLING_FREQUENCY.tag,
    dictionary::MULTIPLEX_GROUP_LABEL.tag,
    dictionary::MULTIPLEX_GROUP_UID.tag,
    dictionary::CHANNEL_DEFINITION_SEQUENCE.tag,
    dictionary::WAVEFORM_BITS_ALLOCATED.tag,
    dictionary::WAVEFORM_SAMPLE_INTERPRETATION.tag,
    dictionary::WAVEFORM_PADDING_VALUE.tag,
  ];

  /// Creates a new [`WaveformMultiplexGroup`] with the given values. A number
  /// of validations are performed to ensure the values are internally
  /// consistent.
  ///
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    originality: WaveformOriginality,
    number_of_samples: u32,
    sampling_frequency: f64,
    time_offset: Option<f64>,
    trigger_time_offset: Option<f64>,
    trigger_sample_position: Option<u32>,
    label: Option<String>,
    uid: Option<String>,
    channels: Vec<ChannelDefinition>,
    bits_allocated: WaveformBitsAllocated,
    sample_interpretation: WaveformSampleInterpretation,
    padding_value: Option<i64>,
  ) -> Result<Self, DataError> {
    // Check that there is at least one channel when there are samples
    if channels.is_empty() && number_of_samples > 0 {
      return Err(DataError::new_value_invalid(
        "Waveform multiplex group with samples must have at least one channel"
          .to_string(),
      ));
    }

    // Check that the number of channels can be stored in the '(003A,0005)
    // Number of Waveform Channels' data element
    let number_of_channels = u16::try_from(channels.len()).map_err(|_| {
      DataError::new_value_invalid(format!(
        "Number of waveform channels '{}' exceeds the maximum of 2^16 - 1",
        channels.len()
      ))
    })?;

    // Check that the sample interpretation is valid for the bits allocated
    if !sample_interpretation.is_compatible_with(bits_allocated) {
      return Err(DataError::new_value_invalid(format!(
        "Waveform sample interpretation '{sample_interpretation}' is invalid \
         for bits allocated '{}'",
        u16::from(bits_allocated),
      )));
    }

    // Check that the padding value is in range for the sample interpretation
    if let Some(padding_value) = padding_value {
      encode::validate_raw_sample(padding_value, sample_interpretation)
        .map_err(|_| {
          DataError::new_value_invalid(format!(
            "Waveform padding value '{padding_value}' is out of range for \
             the sample interpretation"
          ))
        })?;
    }

    Ok(Self {
      originality,
      number_of_channels,
      number_of_samples,
      sampling_frequency,
      time_offset,
      trigger_time_offset,
      trigger_sample_position,
      label,
      uid,
      channels,
      bits_allocated,
      sample_interpretation,
      padding_value,
    })
  }

  /// Creates a new [`WaveformMultiplexGroup`] from an item of a *'(5400,0100)
  /// Waveform Sequence'*, ignoring any *'(5400,1010) Waveform Data'* it
  /// contains.
  ///
  pub fn from_data_set(item: &DataSet) -> Result<Self, DataError> {
    let originality = WaveformOriginality::from_data_set(item)?;

    let number_of_channels =
      item.get_int::<u16>(dictionary::NUMBER_OF_WAVEFORM_CHANNELS.tag)?;
    let number_of_samples =
      item.get_int::<u32>(dictionary::NUMBER_OF_WAVEFORM_SAMPLES.tag)?;
    let sampling_frequency =
      item.get_float(dictionary::SAMPLING_FREQUENCY.tag)?;

    let time_offset =
      get_optional_float(item, dictionary::MULTIPLEX_GROUP_TIME_OFFSET.tag)?;

    let trigger_time_offset =
      get_optional_float(item, dictionary::TRIGGER_TIME_OFFSET.tag)?;

    let trigger_sample_position =
      if item.has(dictionary::TRIGGER_SAMPLE_POSITION.tag) {
        Some(item.get_int::<u32>(dictionary::TRIGGER_SAMPLE_POSITION.tag)?)
      } else {
        None
      };

    let label =
      get_optional_string(item, dictionary::MULTIPLEX_GROUP_LABEL.tag)?;

    let uid = get_optional_string(item, dictionary::MULTIPLEX_GROUP_UID.tag)?;

    let bits_allocated = WaveformBitsAllocated::from_data_set(item)?;
    let sample_interpretation =
      WaveformSampleInterpretation::from_data_set(item)?;

    let padding_value = get_optional_stored_value(
      item,
      dictionary::WAVEFORM_PADDING_VALUE.tag,
      sample_interpretation,
    )?;

    let channel_items =
      item.get_sequence_items(dictionary::CHANNEL_DEFINITION_SEQUENCE.tag)?;

    let channels = channel_items
      .iter()
      .map(|item| ChannelDefinition::from_data_set(item, sample_interpretation))
      .collect::<Result<Vec<_>, DataError>>()?;

    if channels.len() != usize::from(number_of_channels) {
      return Err(
        DataError::new_value_invalid(format!(
          "Channel definition sequence has {} items which does not match the \
           number of waveform channels '{}'",
          channels.len(),
          number_of_channels,
        ))
        .with_path(&DataSetPath::new_with_data_element(
          dictionary::CHANNEL_DEFINITION_SEQUENCE.tag,
        )),
      );
    }

    Self::new(
      originality,
      number_of_samples,
      sampling_frequency,
      time_offset,
      trigger_time_offset,
      trigger_sample_position,
      label,
      uid,
      channels,
      bits_allocated,
      sample_interpretation,
      padding_value,
    )
  }

  /// Returns this waveform multiplex group's originality, i.e. whether it is
  /// an original or derived recording.
  ///
  pub fn originality(&self) -> WaveformOriginality {
    self.originality
  }

  /// Returns this waveform multiplex group's number of channels.
  ///
  pub fn number_of_channels(&self) -> u16 {
    self.number_of_channels
  }

  /// Returns this waveform multiplex group's number of samples per channel.
  ///
  pub fn number_of_samples(&self) -> u32 {
    self.number_of_samples
  }

  /// Returns this waveform multiplex group's sampling frequency in Hz.
  ///
  pub fn sampling_frequency(&self) -> f64 {
    self.sampling_frequency
  }

  /// Returns this waveform multiplex group's time offset in milliseconds.
  ///
  pub fn time_offset(&self) -> Option<f64> {
    self.time_offset
  }

  /// Sets this waveform multiplex group's time offset in milliseconds.
  ///
  pub fn set_time_offset(&mut self, time_offset: Option<f64>) {
    self.time_offset = time_offset;
  }

  /// Returns this waveform multiplex group's trigger time offset in
  /// milliseconds.
  ///
  pub fn trigger_time_offset(&self) -> Option<f64> {
    self.trigger_time_offset
  }

  /// Sets this waveform multiplex group's trigger time offset in
  /// milliseconds.
  ///
  pub fn set_trigger_time_offset(&mut self, trigger_time_offset: Option<f64>) {
    self.trigger_time_offset = trigger_time_offset;
  }

  /// Returns this waveform multiplex group's trigger sample position.
  ///
  pub fn trigger_sample_position(&self) -> Option<u32> {
    self.trigger_sample_position
  }

  /// Sets this waveform multiplex group's trigger sample position.
  ///
  pub fn set_trigger_sample_position(
    &mut self,
    trigger_sample_position: Option<u32>,
  ) {
    self.trigger_sample_position = trigger_sample_position;
  }

  /// Returns this waveform multiplex group's label.
  ///
  pub fn label(&self) -> Option<&str> {
    self.label.as_deref()
  }

  /// Sets this waveform multiplex group's label.
  ///
  pub fn set_label(&mut self, label: Option<String>) {
    self.label = label;
  }

  /// Returns this waveform multiplex group's UID.
  ///
  pub fn uid(&self) -> Option<&str> {
    self.uid.as_deref()
  }

  /// Sets this waveform multiplex group's UID.
  ///
  pub fn set_uid(&mut self, uid: Option<String>) {
    self.uid = uid;
  }

  /// Returns this waveform multiplex group's channel definitions.
  ///
  pub fn channels(&self) -> &[ChannelDefinition] {
    &self.channels
  }

  /// Sets this waveform multiplex group's channel definitions. The number of
  /// channels can't be altered as the channel definitions must stay
  /// consistent with the waveform data.
  ///
  pub fn set_channels(
    &mut self,
    channels: Vec<ChannelDefinition>,
  ) -> Result<(), String> {
    if channels.len() != usize::from(self.number_of_channels) {
      return Err(format!(
        "Waveform multiplex group number of channels cannot be changed from \
         {} to {}",
        self.number_of_channels,
        channels.len(),
      ));
    }

    self.channels = channels;

    Ok(())
  }

  /// Returns this waveform multiplex group's number of bits allocated per
  /// sample.
  ///
  pub fn bits_allocated(&self) -> WaveformBitsAllocated {
    self.bits_allocated
  }

  /// Returns this waveform multiplex group's sample interpretation, i.e. how
  /// sample values are stored in the waveform data.
  ///
  pub fn sample_interpretation(&self) -> WaveformSampleInterpretation {
    self.sample_interpretation
  }

  /// Returns this waveform multiplex group's padding value as a raw stored
  /// value. Samples with this value are not real measurements, e.g. they may
  /// fill out a channel that has fewer samples than the rest of its multiplex
  /// group.
  ///
  pub fn padding_value(&self) -> Option<i64> {
    self.padding_value
  }

  /// Returns the size in bytes of a single sample set, i.e. of one sample for
  /// every channel at a single instant.
  ///
  pub(crate) fn sample_set_size(&self) -> usize {
    usize::from(self.number_of_channels)
      * self.sample_interpretation.bytes_per_sample()
  }

  /// Returns the length in bytes of this waveform multiplex group's waveform
  /// data.
  ///
  pub(crate) fn waveform_data_length(&self) -> u64 {
    u64::from(self.number_of_samples) * self.sample_set_size() as u64
  }

  /// Converts this waveform multiplex group to a data set for storing in an
  /// item of a *'(5400,0100) Waveform Sequence'*. The returned data set does
  /// not include a *'(5400,1010) Waveform Data'* data element.
  ///
  pub fn to_data_set(&self) -> Result<DataSet, DataError> {
    let mut data_set = DataSet::new();

    data_set.insert(
      dictionary::WAVEFORM_ORIGINALITY.tag,
      self.originality.to_data_element_value(),
    );

    data_set.insert_int_value(
      &dictionary::NUMBER_OF_WAVEFORM_CHANNELS,
      &[i64::from(self.number_of_channels)],
    )?;

    data_set.insert_int_value(
      &dictionary::NUMBER_OF_WAVEFORM_SAMPLES,
      &[i64::from(self.number_of_samples)],
    )?;

    data_set.insert_float_value(
      &dictionary::SAMPLING_FREQUENCY,
      &[self.sampling_frequency],
    )?;

    if let Some(time_offset) = self.time_offset {
      data_set.insert_float_value(
        &dictionary::MULTIPLEX_GROUP_TIME_OFFSET,
        &[time_offset],
      )?;
    }

    if let Some(trigger_time_offset) = self.trigger_time_offset {
      data_set.insert_float_value(
        &dictionary::TRIGGER_TIME_OFFSET,
        &[trigger_time_offset],
      )?;
    }

    if let Some(trigger_sample_position) = self.trigger_sample_position {
      data_set.insert_int_value(
        &dictionary::TRIGGER_SAMPLE_POSITION,
        &[i64::from(trigger_sample_position)],
      )?;
    }

    if let Some(label) = &self.label {
      data_set
        .insert_string_value(&dictionary::MULTIPLEX_GROUP_LABEL, &[label])?;
    }

    if let Some(uid) = &self.uid {
      data_set.insert_string_value(&dictionary::MULTIPLEX_GROUP_UID, &[uid])?;
    }

    let channel_items = self
      .channels
      .iter()
      .map(|channel| channel.to_data_set(self.sample_interpretation))
      .collect::<Result<Vec<_>, DataError>>()?;

    data_set.insert_sequence_value(
      &dictionary::CHANNEL_DEFINITION_SEQUENCE,
      channel_items,
    )?;

    data_set.insert(
      dictionary::WAVEFORM_BITS_ALLOCATED.tag,
      self.bits_allocated.to_data_element_value(),
    );

    data_set.insert(
      dictionary::WAVEFORM_SAMPLE_INTERPRETATION.tag,
      self.sample_interpretation.to_data_element_value(),
    );

    if let Some(padding_value) = self.padding_value {
      insert_stored_value(
        &mut data_set,
        dictionary::WAVEFORM_PADDING_VALUE.tag,
        padding_value,
        self.sample_interpretation,
      )?;
    }

    Ok(data_set)
  }
}

/// Whether a waveform is an original or a derived recording, as stored in the
/// *'(003A,0004) Waveform Originality'* data element.
///
/// Ref: PS3.3 C.10.9.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WaveformOriginality {
  Original,
  Derived,
}

impl WaveformOriginality {
  /// Creates a new [`WaveformOriginality`] from the *'(003A,0004) Waveform
  /// Originality'* data element in the given data set.
  ///
  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let tag = dictionary::WAVEFORM_ORIGINALITY.tag;

    match data_set.get_string(tag)? {
      "ORIGINAL" => Ok(Self::Original),
      "DERIVED" => Ok(Self::Derived),
      value => Err(
        DataError::new_value_invalid(format!(
          "Waveform originality '{value}' is invalid"
        ))
        .with_path(&DataSetPath::new_with_data_element(tag)),
      ),
    }
  }

  /// Converts this [`WaveformOriginality`] to a data element value that uses
  /// the [`ValueRepresentation::CodeString`] value representation.
  ///
  pub fn to_data_element_value(&self) -> DataElementValue {
    let s = match self {
      Self::Original => "ORIGINAL",
      Self::Derived => "DERIVED",
    };

    DataElementValue::new_code_string(&[s]).unwrap()
  }
}

/// The number of bits allocated per waveform sample, as stored in the
/// *'(5400,1004) Waveform Bits Allocated'* data element.
///
/// Ref: PS3.3 C.10.9.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WaveformBitsAllocated {
  Eight,
  Sixteen,
  ThirtyTwo,
  SixtyFour,
}

impl WaveformBitsAllocated {
  /// Creates a new [`WaveformBitsAllocated`] from the *'(5400,1004) Waveform
  /// Bits Allocated'* data element in the given data set.
  ///
  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let tag = dictionary::WAVEFORM_BITS_ALLOCATED.tag;

    Self::try_from(data_set.get_int::<u16>(tag)?).map_err(|e| {
      DataError::new_value_invalid(e)
        .with_path(&DataSetPath::new_with_data_element(tag))
    })
  }

  /// Converts this [`WaveformBitsAllocated`] to a data element value that
  /// uses the [`ValueRepresentation::UnsignedShort`] value representation.
  ///
  pub fn to_data_element_value(&self) -> DataElementValue {
    DataElementValue::new_unsigned_short(&[u16::from(*self)]).unwrap()
  }
}

impl From<WaveformBitsAllocated> for u16 {
  fn from(bits_allocated: WaveformBitsAllocated) -> u16 {
    match bits_allocated {
      WaveformBitsAllocated::Eight => 8,
      WaveformBitsAllocated::Sixteen => 16,
      WaveformBitsAllocated::ThirtyTwo => 32,
      WaveformBitsAllocated::SixtyFour => 64,
    }
  }
}

impl TryFrom<u16> for WaveformBitsAllocated {
  type Error = String;

  fn try_from(bits_allocated: u16) -> Result<WaveformBitsAllocated, String> {
    match bits_allocated {
      8 => Ok(Self::Eight),
      16 => Ok(Self::Sixteen),
      32 => Ok(Self::ThirtyTwo),
      64 => Ok(Self::SixtyFour),
      value => Err(format!(
        "Waveform bits allocated value of '{value}' is not supported",
      )),
    }
  }
}

/// How waveform sample values are stored in the waveform data, as specified
/// by the *'(5400,1006) Waveform Sample Interpretation'* data element.
///
/// Ref: PS3.3 C.10.9.1.5.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WaveformSampleInterpretation {
  /// Signed 16-bit linear. The `SS` sample interpretation.
  SignedShort,

  /// Unsigned 16-bit linear. The `US` sample interpretation.
  UnsignedShort,

  /// Signed 32-bit linear. The `SL` sample interpretation. This is used by
  /// the General 32-bit ECG Waveform Storage SOP class.
  SignedLong,

  /// Unsigned 32-bit linear. The `UL` sample interpretation.
  UnsignedLong,

  /// Signed 64-bit linear. The `SV` sample interpretation.
  SignedVeryLong,

  /// Unsigned 64-bit linear. The `UV` sample interpretation.
  UnsignedVeryLong,

  /// Signed 8-bit linear. The `SB` sample interpretation.
  SignedByte,

  /// Unsigned 8-bit linear. The `UB` sample interpretation.
  UnsignedByte,

  /// 8-bit ITU-T G.711 µ-law companded. The `MB` sample interpretation.
  MuLawByte,

  /// 8-bit ITU-T G.711 A-law companded. The `AB` sample interpretation.
  ALawByte,
}

impl WaveformSampleInterpretation {
  /// Creates a new [`WaveformSampleInterpretation`] from the *'(5400,1006)
  /// Waveform Sample Interpretation'* data element in the given data set.
  ///
  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let tag = dictionary::WAVEFORM_SAMPLE_INTERPRETATION.tag;

    match data_set.get_string(tag)? {
      "SS" => Ok(Self::SignedShort),
      "US" => Ok(Self::UnsignedShort),
      "SL" => Ok(Self::SignedLong),
      "UL" => Ok(Self::UnsignedLong),
      "SV" => Ok(Self::SignedVeryLong),
      "UV" => Ok(Self::UnsignedVeryLong),
      "SB" => Ok(Self::SignedByte),
      "UB" => Ok(Self::UnsignedByte),
      "MB" => Ok(Self::MuLawByte),
      "AB" => Ok(Self::ALawByte),
      value => Err(
        DataError::new_value_invalid(format!(
          "Waveform sample interpretation '{value}' is invalid"
        ))
        .with_path(&DataSetPath::new_with_data_element(tag)),
      ),
    }
  }

  /// Returns the number of bytes used to store a single sample.
  ///
  pub fn bytes_per_sample(&self) -> usize {
    match self {
      Self::SignedByte
      | Self::UnsignedByte
      | Self::MuLawByte
      | Self::ALawByte => 1,
      Self::SignedShort | Self::UnsignedShort => 2,
      Self::SignedLong | Self::UnsignedLong => 4,
      Self::SignedVeryLong | Self::UnsignedVeryLong => 8,
    }
  }

  /// Returns whether this sample interpretation is valid for the given number
  /// of bits allocated.
  ///
  pub fn is_compatible_with(
    &self,
    bits_allocated: WaveformBitsAllocated,
  ) -> bool {
    self.bytes_per_sample() * 8 == usize::from(u16::from(bits_allocated))
  }

  /// Returns the string value for this sample interpretation, e.g. `"SS"`.
  ///
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::SignedShort => "SS",
      Self::UnsignedShort => "US",
      Self::SignedLong => "SL",
      Self::UnsignedLong => "UL",
      Self::SignedVeryLong => "SV",
      Self::UnsignedVeryLong => "UV",
      Self::SignedByte => "SB",
      Self::UnsignedByte => "UB",
      Self::MuLawByte => "MB",
      Self::ALawByte => "AB",
    }
  }

  /// Converts this [`WaveformSampleInterpretation`] to a data element value
  /// that uses the [`ValueRepresentation::CodeString`] value representation.
  ///
  pub fn to_data_element_value(&self) -> DataElementValue {
    DataElementValue::new_code_string(&[self.as_str()]).unwrap()
  }

  /// Returns the value representation used for binary data elements that
  /// store values in this sample encoding, i.e. the waveform data, the
  /// waveform padding value, and the channel minimum and maximum values.
  ///
  pub(crate) fn binary_value_representation(&self) -> ValueRepresentation {
    if self.bytes_per_sample() == 1 {
      ValueRepresentation::OtherByteString
    } else {
      ValueRepresentation::OtherWordString
    }
  }
}

impl core::fmt::Display for WaveformSampleInterpretation {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use dcmfx_core::Rc;

  use crate::{WaveformChunk, WaveformDecodeError};

  fn test_channel() -> ChannelDefinition {
    ChannelDefinition {
      waveform_channel_number: None,
      label: None,
      status: vec![],
      source: CodedConcept {
        code_value: Some("5.6.3-9-1".to_string()),
        coding_scheme_designator: Some("SCPECG".to_string()),
        coding_scheme_version: None,
        code_meaning: Some("Lead I (Einthoven)".to_string()),
      },
      derivation_description: None,
      sensitivity: None,
      sensitivity_units: None,
      sensitivity_correction_factor: None,
      baseline: None,
      time_skew: None,
      sample_skew: None,
      offset: None,
      bits_stored: 16,
      amplifier_type: None,
      filter_low_frequency: None,
      filter_high_frequency: None,
      notch_filter_frequency: None,
      notch_filter_bandwidth: None,
      minimum_value: None,
      maximum_value: None,
    }
  }

  fn test_multiplex_group(
    number_of_channels: usize,
    number_of_samples: u32,
  ) -> WaveformMultiplexGroup {
    WaveformMultiplexGroup::new(
      WaveformOriginality::Original,
      number_of_samples,
      500.0,
      None,
      None,
      None,
      None,
      None,
      (0..number_of_channels).map(|_| test_channel()).collect(),
      WaveformBitsAllocated::Sixteen,
      WaveformSampleInterpretation::SignedShort,
      None,
    )
    .unwrap()
  }

  #[test]
  fn new_validates_values() {
    // The sample interpretation must be valid for the bits allocated
    assert!(
      WaveformMultiplexGroup::new(
        WaveformOriginality::Original,
        2,
        500.0,
        None,
        None,
        None,
        None,
        None,
        vec![test_channel()],
        WaveformBitsAllocated::Eight,
        WaveformSampleInterpretation::SignedShort,
        None,
      )
      .is_err()
    );

    // The padding value must be in range for the sample interpretation
    assert!(
      WaveformMultiplexGroup::new(
        WaveformOriginality::Original,
        2,
        500.0,
        None,
        None,
        None,
        None,
        None,
        vec![test_channel()],
        WaveformBitsAllocated::Sixteen,
        WaveformSampleInterpretation::SignedShort,
        Some(65536),
      )
      .is_err()
    );

    // A negative padding value is invalid for an unsigned sample
    // interpretation
    assert!(
      WaveformMultiplexGroup::new(
        WaveformOriginality::Original,
        2,
        500.0,
        None,
        None,
        None,
        None,
        None,
        vec![test_channel()],
        WaveformBitsAllocated::SixtyFour,
        WaveformSampleInterpretation::UnsignedVeryLong,
        Some(-1),
      )
      .is_err()
    );

    // There must be at least one channel when there are samples
    assert!(
      WaveformMultiplexGroup::new(
        WaveformOriginality::Original,
        2,
        500.0,
        None,
        None,
        None,
        None,
        None,
        vec![],
        WaveformBitsAllocated::Sixteen,
        WaveformSampleInterpretation::SignedShort,
        None,
      )
      .is_err()
    );
  }

  #[test]
  fn channel_samples_decodes_interleaved_chunks() {
    let multiplex_group = Rc::new(test_multiplex_group(2, 5));

    // A chunk holding the second three sample sets of the multiplex group
    let chunk = WaveformChunk::new(
      multiplex_group.clone(),
      0,
      2,
      3,
      vec![1, 0, 4, 0, 2, 0, 5, 0, 3, 0, 6, 0].into(),
    );

    assert_eq!(
      chunk.channel_samples(),
      Ok(vec![vec![1, 2, 3], vec![4, 5, 6]])
    );

    // A chunk without enough data for its sample sets fails to decode
    let invalid_chunk =
      WaveformChunk::new(multiplex_group.clone(), 0, 0, 3, vec![1, 0].into());

    assert_eq!(
      invalid_chunk.channel_samples(),
      Err(WaveformDecodeError::DataLengthInvalid {
        expected: 12,
        actual: 2,
      })
    );

    // A chunk with more data than its sample sets require fails to decode
    let oversized_chunk = WaveformChunk::new(
      multiplex_group.clone(),
      0,
      0,
      2,
      vec![1, 0, 4, 0, 2, 0, 5, 0, 3, 0].into(),
    );

    assert_eq!(
      oversized_chunk.channel_samples(),
      Err(WaveformDecodeError::DataLengthInvalid {
        expected: 8,
        actual: 10,
      })
    );
  }

  #[test]
  fn to_data_set_from_data_set_round_trip() {
    let mut multiplex_group = WaveformMultiplexGroup::new(
      WaveformOriginality::Original,
      10000,
      1000.0,
      Some(0.5),
      Some(1.5),
      Some(501),
      Some("RHYTHM".to_string()),
      Some("1.2.840.10008.999.1".to_string()),
      vec![test_channel(), test_channel()],
      WaveformBitsAllocated::Sixteen,
      WaveformSampleInterpretation::SignedShort,
      Some(-32768),
    )
    .unwrap();

    let mut channels = multiplex_group.channels().to_vec();
    channels[0].minimum_value = Some(-500);
    channels[0].maximum_value = Some(1000);
    multiplex_group.set_channels(channels).unwrap();

    assert_eq!(
      WaveformMultiplexGroup::from_data_set(
        &multiplex_group.to_data_set().unwrap()
      ),
      Ok(multiplex_group.clone())
    );

    // Round trip a whole Waveform module built from multiplex group items
    let waveform_module = WaveformModule::new(vec![
      multiplex_group.clone(),
      test_multiplex_group(1, 3),
    ]);

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(
        &dictionary::WAVEFORM_SEQUENCE,
        vec![
          multiplex_group.to_data_set().unwrap(),
          test_multiplex_group(1, 3).to_data_set().unwrap(),
        ],
      )
      .unwrap();

    assert_eq!(
      WaveformModule::from_data_set(&data_set),
      Ok(waveform_module)
    );
  }

  #[test]
  fn from_data_set_validates_sample_interpretation() {
    let mut item = test_multiplex_group(2, 3).to_data_set().unwrap();
    item.insert(
      dictionary::WAVEFORM_SAMPLE_INTERPRETATION.tag,
      WaveformSampleInterpretation::SignedByte.to_data_element_value(),
    );

    assert!(WaveformMultiplexGroup::from_data_set(&item).is_err());
  }

  #[test]
  fn from_data_set_validates_channel_count() {
    let mut item = test_multiplex_group(2, 3).to_data_set().unwrap();
    item
      .insert_sequence_value(
        &dictionary::CHANNEL_DEFINITION_SEQUENCE,
        vec![
          test_channel()
            .to_data_set(WaveformSampleInterpretation::SignedShort)
            .unwrap(),
        ],
      )
      .unwrap();

    assert!(WaveformMultiplexGroup::from_data_set(&item).is_err());
  }

  #[test]
  fn from_data_set_requires_originality() {
    let mut item = test_multiplex_group(2, 3).to_data_set().unwrap();
    item.delete(dictionary::WAVEFORM_ORIGINALITY.tag);

    assert!(WaveformMultiplexGroup::from_data_set(&item).is_err());
  }

  #[test]
  fn set_channels_validates_channel_count() {
    let mut multiplex_group = test_multiplex_group(2, 2);

    let mut channels = multiplex_group.channels().to_vec();
    channels[0].sensitivity = Some(1.25);
    multiplex_group.set_channels(channels).unwrap();

    assert_eq!(multiplex_group.channels()[0].sensitivity, Some(1.25));

    // The number of channels can't be changed
    assert!(multiplex_group.set_channels(vec![]).is_err());
  }
}
