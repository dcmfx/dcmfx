pub mod input_source;
pub mod mp4_encoder;
pub mod object_store;
pub mod output_target;

pub use input_source::InputSource;
pub use output_target::OutputTarget;

use std::path::{Component, Path, PathBuf};

use futures::{TryStreamExt, stream::StreamExt};

/// Runs tasks concurrently up to the specified task count, passing each item
/// from the given stream to the provided async body function.
///
/// Returns an error as soon as any of the tasks return an error.
///
pub async fn run_tasks<InputStream, Item, E>(
  task_count: usize,
  inputs: InputStream,
  body_func: impl AsyncFn(Item) -> Result<(), E>,
) -> Result<(), E>
where
  InputStream: futures::stream::Stream<Item = Item>,
{
  inputs
    .map(async |i| body_func(i).await)
    .buffer_unordered(task_count.max(1))
    .try_collect::<()>()
    .await
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
