// I/O Error type

#[cfg(not(feature = "std"))]
pub type IoError = alloc::string::String;

#[cfg(feature = "std")]
pub type IoError = std::io::Error;

// I/O Read trait

#[cfg(not(feature = "std"))]
pub trait IoRead {
  fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError>;
}

#[cfg(feature = "std")]
pub trait IoRead: std::io::Read {}

#[cfg(feature = "std")]
impl<T: std::io::Read> IoRead for T {}

// I/O Write trait

#[cfg(not(feature = "std"))]
pub trait IoWrite {
  fn write_all(&mut self, buf: &[u8]) -> Result<(), IoError>;
  fn flush(&mut self) -> Result<(), IoError>;
}

#[cfg(feature = "std")]
pub trait IoWrite: std::io::Write {}

#[cfg(feature = "std")]
impl<T: std::io::Write> IoWrite for T {}
