use core::ops::{Add, Div};

/// Unsigned integer division that rounds to the nearest integer instead of
/// truncating towards zero like standard integer division.
///
#[inline]
pub fn udiv_round<T>(numer: T, denom: T) -> T
where
  T: Copy + Add<Output = T> + Div<Output = T> + From<u8>,
{
  (numer + (denom / T::from(2))) / denom
}
