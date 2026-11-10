/// Describes an axis-aligned rectangular region of pixels in an image. The
/// origin is the top left corner of the image.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PixelRect {
  pub left: u16,
  pub top: u16,
  pub width: u16,
  pub height: u16,
}

impl PixelRect {
  /// Creates a new [`PixelRect`] with the given position and size.
  ///
  pub fn new(left: u16, top: u16, width: u16, height: u16) -> Self {
    Self {
      left,
      top,
      width,
      height,
    }
  }

  /// Returns whether this rect contains no pixels.
  ///
  pub fn is_empty(&self) -> bool {
    self.width == 0 || self.height == 0
  }

  /// Returns the exclusive right edge of this rect.
  ///
  pub fn right(&self) -> u32 {
    u32::from(self.left) + u32::from(self.width)
  }

  /// Returns the exclusive bottom edge of this rect.
  ///
  pub fn bottom(&self) -> u32 {
    u32::from(self.top) + u32::from(self.height)
  }

  /// Returns this rect grown by `amount` pixels on all four sides, saturating
  /// at the limits of a `u16`.
  ///
  pub fn expanded(&self, amount: u16) -> Self {
    let left = self.left.saturating_sub(amount);
    let top = self.top.saturating_sub(amount);

    Self {
      left,
      top,
      width: self
        .width
        .saturating_add(self.left - left)
        .saturating_add(amount),
      height: self
        .height
        .saturating_add(self.top - top)
        .saturating_add(amount),
    }
  }

  /// Returns the intersection of this rect with an image of the given
  /// dimensions, or `None` if the two don't overlap.
  ///
  /// The returned rect is guaranteed to lie entirely inside the image, i.e. its
  /// right edge is not greater than `width` and its bottom edge is not greater
  /// than `height`.
  ///
  pub fn clamped_to_image(&self, width: u16, height: u16) -> Option<Self> {
    if self.left >= width || self.top >= height || self.is_empty() {
      return None;
    }

    Some(Self {
      left: self.left,
      top: self.top,
      width: (self.right().min(width.into()) as u16) - self.left,
      height: (self.bottom().min(height.into()) as u16) - self.top,
    })
  }

  /// Returns the rects that together cover every pixel of an image of the
  /// given dimensions that is outside of this rect. The returned rects don't
  /// overlap each other, and some of them may be empty.
  ///
  pub fn outside_rects(&self, width: u16, height: u16) -> [Self; 4] {
    // If this rect doesn't overlap the image then the whole image is outside it
    let Some(rect) = self.clamped_to_image(width, height) else {
      return [
        Self::new(0, 0, width, height),
        Self::new(0, 0, 0, 0),
        Self::new(0, 0, 0, 0),
        Self::new(0, 0, 0, 0),
      ];
    };

    let right = rect.right() as u16;
    let bottom = rect.bottom() as u16;

    [
      // Full-width bands above and below
      Self::new(0, 0, width, rect.top),
      Self::new(0, bottom, width, height - bottom),
      // Bands to the left and right, between the two bands above
      Self::new(0, rect.top, rect.left, rect.height),
      Self::new(right, rect.top, width - right, rect.height),
    ]
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn expanded_test() {
    assert_eq!(
      PixelRect::new(10, 20, 30, 40).expanded(5),
      PixelRect::new(5, 15, 40, 50)
    );

    // Expansion is clamped at the top left corner, and the width and height
    // only grow by the amount the position actually moved
    assert_eq!(
      PixelRect::new(2, 0, 30, 40).expanded(5),
      PixelRect::new(0, 0, 37, 45)
    );

    assert_eq!(
      PixelRect::new(0, 0, u16::MAX, u16::MAX).expanded(5),
      PixelRect::new(0, 0, u16::MAX, u16::MAX)
    );
  }

  #[test]
  fn clamped_to_image_test() {
    assert_eq!(
      PixelRect::new(10, 20, 30, 40).clamped_to_image(100, 100),
      Some(PixelRect::new(10, 20, 30, 40))
    );

    assert_eq!(
      PixelRect::new(10, 20, 30, 40).clamped_to_image(20, 30),
      Some(PixelRect::new(10, 20, 10, 10))
    );

    assert_eq!(
      PixelRect::new(10, 20, 30, 40).clamped_to_image(10, 100),
      None
    );
    assert_eq!(
      PixelRect::new(10, 20, 30, 40).clamped_to_image(100, 20),
      None
    );
    assert_eq!(
      PixelRect::new(10, 20, 0, 40).clamped_to_image(100, 100),
      None
    );
  }

  #[test]
  fn outside_rects_test() {
    assert_eq!(
      PixelRect::new(10, 20, 30, 40).outside_rects(100, 200),
      [
        PixelRect::new(0, 0, 100, 20),
        PixelRect::new(0, 60, 100, 140),
        PixelRect::new(0, 20, 10, 40),
        PixelRect::new(40, 20, 60, 40),
      ]
    );

    // A rect covering the whole image has nothing outside it
    assert!(
      PixelRect::new(0, 0, 100, 200)
        .outside_rects(100, 200)
        .iter()
        .all(PixelRect::is_empty)
    );

    // A rect that doesn't overlap the image has the whole image outside it
    for rect in [PixelRect::new(100, 0, 10, 10), PixelRect::new(0, 0, 0, 10)] {
      let outside_rects = rect.outside_rects(100, 200);

      assert_eq!(outside_rects[0], PixelRect::new(0, 0, 100, 200));
      assert!(outside_rects[1..].iter().all(PixelRect::is_empty));
    }
  }
}
