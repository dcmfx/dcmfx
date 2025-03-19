//! Defines a trait implemented by all error types in DCMfx.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "std")]
use std::io::Write;

#[cfg(feature = "std")]
use owo_colors::{OwoColorize, Stream::Stderr};

/// Error trait implemented by all error types in DCMfx.
///
pub trait DcmfxError {
  /// Returns lines of text that describe an error in a human-readable format.
  ///
  fn to_lines(&self, task_description: &str) -> Vec<String>;

  /// Prints details on the error to stderr. This will include all details and
  /// contextual information stored in the error.
  ///
  #[cfg(feature = "std")]
  fn print(&self, task_description: &str) {
    print_error_lines(&self.to_lines(task_description));
  }
}

/// Prints lines of error information to stderr.
///
#[cfg(feature = "std")]
pub fn print_error_lines(lines: &[String]) {
  let _ = std::io::stdout().flush();
  let _ = std::io::stderr().flush();

  eprintln!();
  eprintln!("{}", "-----".if_supports_color(Stderr, |text| text.red()));

  for line in lines {
    eprintln!("{}", line.if_supports_color(Stderr, |text| text.red()));
  }

  eprintln!();
}
