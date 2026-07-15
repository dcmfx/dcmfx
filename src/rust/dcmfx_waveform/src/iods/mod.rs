pub mod channel_definition;
pub mod coded_concept;
pub mod waveform_module;

pub use channel_definition::{ChannelDefinition, ChannelStatus};
pub use coded_concept::CodedConcept;
pub use waveform_module::{
  WaveformBitsAllocated, WaveformModule, WaveformMultiplexGroup,
  WaveformOriginality, WaveformSampleInterpretation,
};

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
};

use dcmfx_core::{DataElementTag, DataError, DataSet, DataSetPath};

use crate::{decode, encode};

/// Returns the single item of a sequence that is required to have exactly one
/// item.
///
pub(crate) fn get_single_sequence_item(
  data_set: &DataSet,
  tag: DataElementTag,
) -> Result<&DataSet, DataError> {
  match data_set.get_sequence_items(tag)? {
    [item] => Ok(item),

    items => Err(
      DataError::new_value_invalid(format!(
        "Sequence does not have exactly one item, found {} items",
        items.len()
      ))
      .with_path(&DataSetPath::new_with_data_element(tag)),
    ),
  }
}

/// Returns the value of an optional string data element, or `None` when it
/// isn't present in the data set.
///
pub(crate) fn get_optional_string(
  data_set: &DataSet,
  tag: DataElementTag,
) -> Result<Option<String>, DataError> {
  if data_set.has(tag) {
    Ok(Some(data_set.get_string(tag)?.to_string()))
  } else {
    Ok(None)
  }
}

/// Returns the value of an optional float data element, or `None` when it
/// isn't present in the data set.
///
pub(crate) fn get_optional_float(
  data_set: &DataSet,
  tag: DataElementTag,
) -> Result<Option<f64>, DataError> {
  if data_set.has(tag) {
    Ok(Some(data_set.get_float(tag)?))
  } else {
    Ok(None)
  }
}

/// Reads an optional OB/OW data element that stores a single value in the
/// multiplex group's sample encoding, i.e. the waveform padding value and the
/// channel minimum and maximum values. The returned value is the raw stored
/// value.
///
pub(crate) fn get_optional_stored_value(
  item: &DataSet,
  tag: DataElementTag,
  sample_interpretation: WaveformSampleInterpretation,
) -> Result<Option<i64>, DataError> {
  if !item.has(tag) {
    return Ok(None);
  }

  let bytes = item.get_value_bytes(tag)?;

  let value = decode::decode_stored_value_bytes(bytes, sample_interpretation)
    .map_err(|details| {
    DataError::new_value_invalid(details)
      .with_path(&DataSetPath::new_with_data_element(tag))
  })?;

  Ok(Some(value))
}

/// Inserts an OB/OW data element that stores a single value in the multiplex
/// group's sample encoding, i.e. the waveform padding value and the channel
/// minimum and maximum values.
///
pub(crate) fn insert_stored_value(
  data_set: &mut DataSet,
  tag: DataElementTag,
  value: i64,
  sample_interpretation: WaveformSampleInterpretation,
) -> Result<(), DataError> {
  encode::validate_raw_sample(value, sample_interpretation).map_err(|_| {
    DataError::new_value_invalid(format!(
      "Value '{value}' is out of range for the sample interpretation"
    ))
    .with_path(&DataSetPath::new_with_data_element(tag))
  })?;

  data_set.insert_binary_value(
    tag,
    sample_interpretation.binary_value_representation(),
    encode::encode_stored_value_bytes(value, sample_interpretation).into(),
  )
}
