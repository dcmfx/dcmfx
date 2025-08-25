use std::ops::RangeInclusive;

/// Represents a selection of frames in a DICOM file. Negative indexes are
/// treated as offsets from the end.
///
#[derive(Debug, Clone)]
pub enum FrameSelection {
  /// A selection of individual frames.
  Individual { indexes: Vec<isize> },

  /// A selection of frames in an inclusive range.
  Range { range: RangeInclusive<isize> },
}

impl core::fmt::Display for FrameSelection {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      FrameSelection::Individual { indexes } => {
        let indexes = indexes
          .iter()
          .map(|i| i.to_string())
          .collect::<Vec<_>>()
          .join(",");

        write!(f, "{indexes}")
      }

      FrameSelection::Range { range } => {
        if *range.end() == isize::MAX {
          write!(f, "{}..", range.start())
        } else {
          write!(f, "{}..{}", range.start(), range.end())
        }
      }
    }
  }
}

impl core::str::FromStr for FrameSelection {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if let Ok(indexes) = s
      .split(",")
      .map(|s| s.parse::<isize>())
      .collect::<Result<Vec<_>, _>>()
    {
      return Ok(FrameSelection::Individual { indexes });
    }

    if let Some((a, b)) = s.split_once("..") {
      let start = a.parse::<isize>().map_err(|e| e.to_string())?;

      let end = if b.is_empty() {
        isize::MAX
      } else {
        b.parse::<isize>().map_err(|e| e.to_string())?
      };

      return Ok(FrameSelection::Range { range: start..=end });
    }

    Err(format!("Invalid frame range: {s}"))
  }
}

impl FrameSelection {
  /// Checks if the given frame index is contained within this frame selection.
  ///
  pub fn contains(&self, frame_index: usize, number_of_frames: usize) -> bool {
    match self {
      FrameSelection::Individual { indexes } => {
        for index in indexes.iter() {
          if frame_index as isize
            == FrameSelection::standardize_index(*index, number_of_frames)
          {
            return true;
          }
        }

        false
      }

      FrameSelection::Range { range } => {
        (FrameSelection::standardize_index(*range.start(), number_of_frames)
          ..=FrameSelection::standardize_index(*range.end(), number_of_frames))
          .contains(&(frame_index as isize))
      }
    }
  }

  /// Checks if all frames for this frame selection are done, given the
  /// specified frame has been processed.
  ///
  pub fn is_complete(
    &self,
    frame_index: usize,
    number_of_frames: usize,
  ) -> bool {
    match self {
      FrameSelection::Individual { indexes } => {
        let max = indexes
          .iter()
          .map(|i| FrameSelection::standardize_index(*i, number_of_frames))
          .max()
          .unwrap();

        max <= frame_index as isize
      }

      FrameSelection::Range { range } => {
        let end =
          FrameSelection::standardize_index(*range.end(), number_of_frames);

        frame_index as isize >= end
      }
    }
  }

  /// Converts negative frame indexes to positive ones by treating them as
  /// offsets from the end.
  ///
  fn standardize_index(index: isize, number_of_frames: usize) -> isize {
    if index >= 0 {
      index
    } else {
      number_of_frames as isize + index
    }
  }
}
