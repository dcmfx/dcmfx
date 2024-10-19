use owo_colors::{OwoColorize, Stream::Stdout};

/// Shared trait implemented by all error types in DCMfx.
///
pub trait DcmfxError {
  /// Returns lines of text that describe an error in a human-readable format.
  ///
  fn to_lines(&self, task_description: &str) -> Vec<String>;

  /// Prints details on the error to stderr. This will include all details and
  /// contextual information stored in the error.
  ///
  fn print(&self, task_description: &str) {
    eprintln!();
    eprintln!("{}", "-----".if_supports_color(Stdout, |text| text.red()));

    for line in self.to_lines(task_description) {
      eprintln!("{}", line.if_supports_color(Stdout, |text| text.red()));
    }

    eprintln!();
  }
}
