use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum TransformArg {
  /// Rotate by 90 degrees clockwise.
  Rotate90,

  /// Rotate by 180 degrees.
  Rotate180,

  /// Rotate by 270 degrees clockwise. Equivalent to rotating by 90 degrees
  /// counter-clockwise.
  Rotate270,

  /// Flip horizontally.
  FlipHorizontal,

  /// Flip vertically.
  FlipVertical,

  /// Rotate by 90 degrees clockwise then flip horizontally.
  Rotate90FlipH,

  /// Rotate by 270 degrees clockwise then flip horizontally.
  Rotate270FlipH,
}

impl TransformArg {
  pub fn orientation(&self) -> image::metadata::Orientation {
    match self {
      Self::Rotate90 => image::metadata::Orientation::Rotate90,
      Self::Rotate180 => image::metadata::Orientation::Rotate180,
      Self::Rotate270 => image::metadata::Orientation::Rotate270,
      Self::FlipHorizontal => image::metadata::Orientation::FlipHorizontal,
      Self::FlipVertical => image::metadata::Orientation::FlipVertical,
      Self::Rotate90FlipH => image::metadata::Orientation::Rotate90FlipH,
      Self::Rotate270FlipH => image::metadata::Orientation::Rotate270FlipH,
    }
  }
}
