#[cfg(not(feature = "std"))]
use alloc::{format, vec::Vec};

use core::time::Duration;

use dcmfx_core::{
  DataElementTag, DataError, DataSet, DataSetPath, IodModule,
  ValueRepresentation, dictionary,
};

use super::MultiFrameModule;

/// The attributes of the Cine Module, which describe a Multi-frame Cine Image.
///
/// Ref: PS3.3 C.7.6.5.
///
#[derive(Clone, Debug, PartialEq)]
pub struct CineModule {
  pub preferred_playback_sequencing: Option<PreferredPlaybackSequencing>,
  pub frame_time: Option<f64>,
  pub frame_time_vector: Option<Vec<f64>>,
  pub start_trim: Option<usize>,
  pub stop_trim: Option<usize>,
  pub recommended_display_frame_rate: Option<u32>,
  pub cine_rate: Option<u32>,
  pub frame_delay: Option<f64>,
  pub image_trigger_delay: Option<f64>,
  pub effective_duration: Option<f64>,
  pub actual_frame_duration: Option<usize>,
}

impl IodModule for CineModule {
  fn is_iod_module_data_element(
    tag: DataElementTag,
    _vr: ValueRepresentation,
    _length: Option<u32>,
    path: &DataSetPath,
  ) -> bool {
    if !path.is_empty() {
      return false;
    }

    tag == dictionary::PREFERRED_PLAYBACK_SEQUENCING.tag
      || tag == dictionary::FRAME_TIME.tag
      || tag == dictionary::FRAME_TIME_VECTOR.tag
      || tag == dictionary::START_TRIM.tag
      || tag == dictionary::STOP_TRIM.tag
      || tag == dictionary::RECOMMENDED_DISPLAY_FRAME_RATE.tag
      || tag == dictionary::CINE_RATE.tag
      || tag == dictionary::FRAME_DELAY.tag
      || tag == dictionary::IMAGE_TRIGGER_DELAY.tag
      || tag == dictionary::EFFECTIVE_DURATION.tag
      || tag == dictionary::ACTUAL_FRAME_DURATION.tag
  }

  fn iod_module_highest_tag() -> DataElementTag {
    dictionary::PREFERRED_PLAYBACK_SEQUENCING.tag
  }

  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let preferred_playback_sequencing =
      PreferredPlaybackSequencing::from_data_set(data_set)?;

    let frame_time = if data_set.has(dictionary::FRAME_TIME.tag) {
      Some(data_set.get_float(dictionary::FRAME_TIME.tag)?)
    } else {
      None
    };

    let frame_time_vector = if data_set.has(dictionary::FRAME_TIME_VECTOR.tag) {
      Some(data_set.get_floats(dictionary::FRAME_TIME_VECTOR.tag)?)
    } else {
      None
    };

    let start_trim = if data_set.has(dictionary::START_TRIM.tag) {
      Some(data_set.get_int::<usize>(dictionary::START_TRIM.tag)?)
    } else {
      None
    };

    let stop_trim = if data_set.has(dictionary::STOP_TRIM.tag) {
      Some(data_set.get_int::<usize>(dictionary::STOP_TRIM.tag)?)
    } else {
      None
    };

    let recommended_display_frame_rate =
      if data_set.has(dictionary::RECOMMENDED_DISPLAY_FRAME_RATE.tag) {
        Some(
          data_set
            .get_int::<u32>(dictionary::RECOMMENDED_DISPLAY_FRAME_RATE.tag)?,
        )
      } else {
        None
      };

    let cine_rate = if data_set.has(dictionary::CINE_RATE.tag) {
      Some(data_set.get_int::<u32>(dictionary::CINE_RATE.tag)?)
    } else {
      None
    };

    let frame_delay = if data_set.has(dictionary::FRAME_DELAY.tag) {
      Some(data_set.get_float(dictionary::FRAME_DELAY.tag)?)
    } else {
      None
    };

    let image_trigger_delay =
      if data_set.has(dictionary::IMAGE_TRIGGER_DELAY.tag) {
        Some(data_set.get_float(dictionary::IMAGE_TRIGGER_DELAY.tag)?)
      } else {
        None
      };

    let effective_duration = if data_set.has(dictionary::EFFECTIVE_DURATION.tag)
    {
      Some(data_set.get_float(dictionary::EFFECTIVE_DURATION.tag)?)
    } else {
      None
    };

    let actual_frame_duration =
      if data_set.has(dictionary::ACTUAL_FRAME_DURATION.tag) {
        Some(data_set.get_int::<usize>(dictionary::ACTUAL_FRAME_DURATION.tag)?)
      } else {
        None
      };

    Ok(Self {
      preferred_playback_sequencing,
      frame_time,
      frame_time_vector,
      start_trim,
      stop_trim,
      recommended_display_frame_rate,
      cine_rate,
      frame_delay,
      image_trigger_delay,
      effective_duration,
      actual_frame_duration,
    })
  }
}

impl CineModule {
  /// Returns whether the specified frame index lies outside the range defined
  /// by the start trim or stop trim values, i.e. whether it is trimmed off.
  ///
  pub fn is_frame_trimmed(&self, frame_index: usize) -> bool {
    if let Some(start_trim) = self.start_trim {
      if frame_index < start_trim {
        return true;
      }
    }

    if let Some(stop_trim) = self.stop_trim {
      if frame_index > stop_trim {
        return true;
      }
    }

    false
  }

  /// Returns the number of frames that should be shown, taking into account the
  /// Start Trim and Stop Trim values if specified.
  ///
  pub fn number_of_frames(
    &self,
    multiframe_module: &MultiFrameModule,
  ) -> usize {
    let number_of_frames = multiframe_module.number_of_frames;

    let start_trim = self.start_trim.unwrap_or(0);

    let last_frame = self
      .stop_trim
      .map(|c| (c + 1).min(number_of_frames))
      .unwrap_or(number_of_frames);

    last_frame.saturating_sub(start_trim)
  }

  /// Returns the duration of the specified frame, taking into account the Frame
  /// Increment Pointer if specified.
  ///
  pub fn frame_duration(
    &self,
    frame_index: usize,
    multiframe_module: &MultiFrameModule,
  ) -> Option<Duration> {
    match multiframe_module.frame_increment_pointer {
      Some(tag) if tag == dictionary::FRAME_TIME_VECTOR.tag => {
        if let Some(time) = self.lookup_frame_time_vector(frame_index) {
          return Some(time);
        }
      }

      Some(tag) if tag == dictionary::FRAME_TIME_VECTOR.tag => {
        if let Some(time) = self.frame_time {
          return Some(Duration::from_secs_f64(time / 1000.0));
        }
      }

      _ => (),
    }

    if let Some(time) = self.lookup_frame_time_vector(frame_index) {
      return Some(time);
    }

    if let Some(time) = self.frame_time {
      return Some(Duration::from_secs_f64(time / 1000.0));
    }

    if let Some(rate) = self.recommended_display_frame_rate {
      return Some(Duration::from_secs_f64(1.0 / f64::from(rate)));
    }

    if let Some(rate) = self.cine_rate {
      return Some(Duration::from_secs_f64(1.0 / f64::from(rate)));
    }

    None
  }

  fn lookup_frame_time_vector(&self, frame_index: usize) -> Option<Duration> {
    if let Some(frame_time_vector) = self.frame_time_vector.as_ref() {
      if let Some(time) = frame_time_vector.get(frame_index + 1) {
        return Some(Duration::from_secs_f64(*time / 1000.0));
      }
    }

    None
  }
}

/// Describes the preferred playback sequencing for a Multi-frame Image.
///
/// Ref: PS3.3 C.7.6.5.
///
#[derive(Clone, Debug, PartialEq)]
pub enum PreferredPlaybackSequencing {
  /// Looping (1, 2, …n, 1, 2, …n, 1, 2, …n, …).
  Looping,

  /// Sweeping (1, 2, …n, n-1, …2, 1, 2, …n, …).
  Sweeping,
}

impl PreferredPlaybackSequencing {
  /// Creates a new `PreferredPlaybackSequencing` from the *'(0028,0103)
  /// Preferred Playback Sequencing'* data element in the given data set.
  ///
  pub fn from_data_set(data_set: &DataSet) -> Result<Option<Self>, DataError> {
    let tag = dictionary::PREFERRED_PLAYBACK_SEQUENCING.tag;

    if !data_set.has(tag) {
      return Ok(None);
    }

    match data_set.get_int::<i64>(tag)? {
      0 => Ok(Some(Self::Looping)),
      1 => Ok(Some(Self::Sweeping)),
      value => Err(
        DataError::new_value_invalid(format!(
          "Preferred playback sequencing value of '{}' is invalid",
          value
        ))
        .with_path(&DataSetPath::new_with_data_element(tag)),
      ),
    }
  }
}
