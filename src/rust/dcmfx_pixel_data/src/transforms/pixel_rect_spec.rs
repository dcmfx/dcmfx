#[cfg(not(feature = "std"))]
use alloc::{
  string::{String, ToString},
  vec::Vec,
};

use crate::PixelRect;

/// Specifies a rectangle of pixels in an image whose dimensions aren't yet
/// known, e.g. the rectangle to crop or mask pixel data to. The rectangle is
/// defined by top and left values for its top-left corner, and two optional
/// values that, if positive, define the width and height, or if less than or
/// equal to zero, define the offset from the right and bottom edges of the
/// image respectively.
///
/// Use [`PixelRectSpec::to_pixel_rect()`] to resolve this into a [`PixelRect`]
/// for an image with specific dimensions.
///
#[derive(Clone, Copy, Debug)]
pub struct PixelRectSpec {
  pub left: u16,
  pub top: u16,
  pub width_or_right: Option<i32>,
  pub height_or_bottom: Option<i32>,
}

impl PixelRectSpec {
  /// Returns the rows and columns values of the rectangle that results from
  /// applying this rect spec to an image with the passed dimensions.
  ///
  pub fn apply(&self, rows: u16, columns: u16) -> (u16, u16) {
    let mut new_rows = rows;
    if let Some(height_or_bottom) = self.height_or_bottom {
      new_rows = if height_or_bottom < 1 {
        rows
          .saturating_sub(self.top)
          .saturating_sub(-height_or_bottom as u16)
      } else {
        height_or_bottom as u16
      };
    }

    new_rows = new_rows.min(rows.saturating_sub(self.top));

    let mut new_columns = columns;
    if let Some(width_or_right) = self.width_or_right {
      new_columns = if width_or_right < 1 {
        columns
          .saturating_sub(self.left)
          .saturating_sub(-width_or_right as u16)
      } else {
        width_or_right as u16
      };
    }

    new_columns = new_columns.min(columns.saturating_sub(self.left));

    (new_rows, new_columns)
  }

  /// Returns the rectangle of pixels selected by this rect spec in an image
  /// with the passed dimensions. The returned rect lies entirely inside the
  /// image, and is empty if this rect spec selects no pixels.
  ///
  pub fn to_pixel_rect(&self, rows: u16, columns: u16) -> PixelRect {
    let (height, width) = self.apply(rows, columns);

    PixelRect::new(self.left, self.top, width, height)
  }
}

impl core::str::FromStr for PixelRectSpec {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let parts: Vec<_> = s.split(',').collect();

    if parts.len() < 2 || parts.len() > 4 {
      return Err(
        "Expected rect to use format: left,top[,(width|right)[,(height|bottom)]]"
          .to_string(),
      );
    }

    let left = parts[0]
      .parse()
      .map_err(|_| "Invalid value for rect.left")?;
    let top = parts[1].parse().map_err(|_| "Invalid value for rect.top")?;

    let width_or_right = if parts.len() > 2 {
      Some(
        parts[2]
          .parse()
          .map_err(|_| "Invalid value for rect.width|right")?,
      )
    } else {
      None
    };

    let height_or_bottom = if parts.len() > 3 {
      Some(
        parts[3]
          .parse()
          .map_err(|_| "Invalid value for rect.height|bottom")?,
      )
    } else {
      None
    };

    Ok(PixelRectSpec {
      left,
      top,
      width_or_right,
      height_or_bottom,
    })
  }
}
