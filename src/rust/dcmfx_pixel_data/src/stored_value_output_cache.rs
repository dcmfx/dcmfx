#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Given a range of input pixel data stored values, caches the result of
/// passing each stored value in that range through a function, and caches the
/// result. The cache is then used to quickly find the output for any given
/// stored value.
///
#[derive(Clone, Debug, PartialEq)]
pub struct StoredValueOutputCache<T: Copy> {
  first_stored_value: i64,
  outputs: Vec<T>,
}

impl<T: Copy> StoredValueOutputCache<T> {
  /// Creates a new [`StoredValueOutputCache`] by caching the specified range of
  /// stored values passed through the given conversion function.
  ///
  /// As a general rule, caches should not be used when the range of stored
  /// values is extremely large. In practice, when [`SingleChannelImage`]
  /// creates and uses these caches it only does so when the range of stored
  /// values has <= 2^16 items.
  ///
  pub fn new(
    stored_value_range: &core::ops::RangeInclusive<i64>,
    stored_value_to_pixel: impl Fn(i64) -> T,
  ) -> Self {
    let mut outputs = Vec::with_capacity(
      (stored_value_range.end() - stored_value_range.start() + 1) as usize,
    );

    for stored_value in stored_value_range.clone() {
      outputs.push(stored_value_to_pixel(stored_value));
    }

    Self {
      first_stored_value: *stored_value_range.start(),
      outputs,
    }
  }

  /// Looks up the precomputed cached output value for the specified stored
  /// value. If the stored value is out of range then it is clamped.
  ///
  pub fn get(&self, stored_value: i64) -> T {
    let mut index = stored_value - self.first_stored_value;

    index = index.clamp(0, self.outputs.len() as i64 - 1);

    self.outputs[index as usize]
  }
}
