pub mod input_source;
pub mod mp4_encoder;
pub mod object_store;
pub mod output_target;

pub use input_source::InputSource;
pub use output_target::OutputTarget;

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use futures::stream::StreamExt;

/// Runs tasks in parallel up to the specified task count, passing each item
/// from the given stream to the provided body function.
///
/// Each task runs on its own thread taken from the async runtime's blocking
/// thread pool, which allows all available cores to be used.
///
/// Returns an error as soon as any of the tasks return an error. Tasks that are
/// still in-flight at that time continue running until they complete.
///
pub async fn run_tasks<InputStream, Item, F, Fut, E>(
  task_count: usize,
  inputs: InputStream,
  body_func: F,
) -> Result<(), E>
where
  InputStream: futures::stream::Stream<Item = Item>,
  Item: Send + 'static,
  F: Fn(Item) -> Fut + Send + Sync + 'static,
  Fut: Future<Output = Result<(), E>>,
  E: Send + 'static,
{
  let task_count = task_count.max(1);

  let runtime = tokio::runtime::Handle::current();
  let body_func = Arc::new(body_func);

  let mut inputs = std::pin::pin!(inputs);
  let mut inputs_exhausted = false;

  let mut tasks = tokio::task::JoinSet::new();

  loop {
    // Start new tasks until the task count is reached or there are no further
    // inputs
    while !inputs_exhausted && tasks.len() < task_count {
      match inputs.next().await {
        Some(item) => {
          let runtime = runtime.clone();
          let body_func = body_func.clone();

          tasks.spawn_blocking(move || runtime.block_on(body_func(item)));
        }

        None => inputs_exhausted = true,
      }
    }

    match tasks.join_next().await {
      Some(Ok(Ok(()))) => (),
      Some(Ok(Err(e))) => return Err(e),
      Some(Err(e)) => std::panic::resume_unwind(e.into_panic()),

      // There are no tasks left to run, so all inputs have been processed
      None => return Ok(()),
    }
  }
}

/// Normalizes a path by making it absolute if it is a relative path, and
/// removing '.' and '..' components when present.
///
pub fn normalize_path<P: AsRef<Path>>(input: P) -> PathBuf {
  let path = input.as_ref();

  let absolute_path = if path.is_absolute() {
    PathBuf::from(path)
  } else {
    std::env::current_dir()
      .unwrap_or_else(|_| PathBuf::from("/"))
      .join(path)
  };

  let mut normalized_path = PathBuf::new();
  for component in absolute_path.components() {
    match component {
      Component::CurDir => (),

      Component::ParentDir => {
        normalized_path.pop();
      }

      Component::RootDir => normalized_path.push(component.as_os_str()),

      Component::Normal(_) | Component::Prefix(_) => {
        normalized_path.push(component.as_os_str())
      }
    }
  }

  normalized_path
}

/// Exits the process with an error message and non-zero exit code.
///
pub fn exit_with_error<E: std::fmt::Display>(message: &str, details: E) -> ! {
  let mut lines = vec![];

  lines.push(format!("Error: {}", message));

  let details = format!("{}", details);
  if !details.is_empty() {
    lines.push("".to_string());
    lines.push(format!("Details: {}", details));
  }

  for line in lines {
    use owo_colors::OwoColorize;

    eprintln!(
      "{}",
      line.if_supports_color(owo_colors::Stream::Stderr, |text| text.red())
    );
  }

  std::process::exit(1);
}
