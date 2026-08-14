use std::{
  path::{Path, PathBuf},
  pin::Pin,
  sync::{Arc, LazyLock, atomic::AtomicBool},
  task::{Context, Poll},
  unreachable,
};

use object_store::{
  MultipartUpload, ObjectStore, ObjectStoreExt, PutPayload,
  path::Path as ObjectStorePath,
};
use tokio::{
  io::{AsyncWriteExt, BufWriter, stdout},
  sync::{Mutex, mpsc},
  task::JoinHandle,
};

use dcmfx::p10::{IoAsyncWrite, P10Error};

use crate::utils::{InputSource, object_store::object_url_to_store_and_path};

static OVERWRITE: AtomicBool = AtomicBool::new(false);

/// An output target that abstracts over the different locations that output can
/// be sent to.
///
pub enum OutputTarget {
  /// An output target that writes to stdout. There is a shared stdout stream,
  /// so only one task can write to it at a time. This avoids output from
  /// multiple tasks being interleaved.
  StdOut,

  /// An output target that writes to an object in an object store.
  Object {
    object_store: Arc<dyn ObjectStore>,
    object_path: ObjectStorePath,

    /// The path to display for this output target in output and error messages.
    /// This is the path as specified on the CLI where possible, and is only
    /// for display purposes.
    display_path: PathBuf,
  },
}

impl OutputTarget {
  /// Returns whether existing files should be overwritten when writing to an
  /// output target.
  ///
  pub fn overwrite() -> bool {
    OVERWRITE.load(std::sync::atomic::Ordering::Relaxed)
  }

  /// Sets whether existing files should be overwritten when writing to an
  /// output target.
  ///
  /// This is a global setting.
  ///
  pub fn set_overwrite(overwrite: bool) {
    OVERWRITE.store(overwrite, std::sync::atomic::Ordering::Relaxed);
  }

  /// Creates a new output target for the specified path.
  ///
  pub async fn new<P: AsRef<Path>>(path: P) -> Self {
    let path = path.as_ref();

    if path == "-" {
      return Self::StdOut;
    }

    // Try parsing as an object URL first
    if let Ok((object_store, object_path)) =
      object_url_to_store_and_path(&path.to_string_lossy()).await
    {
      return Self::Object {
        object_store,
        object_path,
        display_path: PathBuf::from(path),
      };
    }

    // Otherwise, treat as a local path
    let (object_store, object_path) =
      crate::utils::object_store::local_path_to_store_and_path(path).await;

    Self::Object {
      object_store,
      object_path,
      display_path: PathBuf::from(path),
    }
  }

  /// Creates an output target that writes to the same object as the given
  /// input source, i.e. for in-place modification of that input.
  ///
  pub fn in_place(input_source: &InputSource) -> Self {
    match input_source {
      // Commands that support in-place output reject stdin as an input
      InputSource::Stdin => unreachable!(),

      InputSource::Object {
        object_store,
        object_path,
        display_path,
      } => Self::Object {
        object_store: object_store.clone(),
        object_path: object_path.clone(),
        display_path: display_path.clone(),
      },
    }
  }

  /// Creates an output target for an input source with the specified output
  /// suffix appended, and located in the specified directory if specified.
  ///
  pub async fn from_input_source(
    input_source: &InputSource,
    output_suffix: &str,
    output_directory: &Option<PathBuf>,
  ) -> Self {
    // An output directory is a location in its own right, so an output target
    // for it is created from scratch
    if let Some(output_directory) = output_directory {
      let file_name = format!("{}{output_suffix}", input_source.file_name());

      return Self::new(output_directory.join(file_name)).await;
    }

    match input_source {
      // Stdin has no location for its output to sit alongside, so the output
      // filename is derived from the '-' that specifies it
      InputSource::Stdin => {
        Self::new(PathBuf::from(format!("-{output_suffix}"))).await
      }

      // The output sits alongside the input, so reuse the input's object store
      // and path rather than re-parsing its display path
      InputSource::Object { .. } => {
        Self::in_place(input_source).append(output_suffix)
      }
    }
  }

  /// Returns whether this output target writes to stdout.
  ///
  pub fn is_stdout(&self) -> bool {
    matches!(self, Self::StdOut)
  }

  /// Returns the path to display for this output target in output and error
  /// messages.
  ///
  pub fn display_path(&self) -> PathBuf {
    match self {
      Self::StdOut => PathBuf::from("-"),
      Self::Object { display_path, .. } => display_path.clone(),
    }
  }

  /// Returns a new output target based on this one, with the specified suffix
  /// appended to its path.
  ///
  pub fn append(&self, suffix: &str) -> Self {
    match self {
      Self::StdOut => Self::StdOut,

      Self::Object {
        object_store,
        object_path,
        display_path,
      } => {
        let object_path =
          ObjectStorePath::parse(format!("{object_path}{suffix}"))
            .expect("object path is valid after appending suffix");

        let mut display_path = display_path.clone();
        if let Some(file_name) = display_path.file_name() {
          display_path
            .set_file_name(format!("{}{suffix}", file_name.to_string_lossy()));
        }

        Self::Object {
          object_store: object_store.clone(),
          object_path,
          display_path,
        }
      }
    }
  }

  /// Opens an async write stream for this output target. Once the write is
  /// complete the client must call [`OutputTarget::commit()`] to finalize the
  /// write.
  ///
  pub async fn open_write_stream(
    &self,
    log_write_to_stdout: bool,
  ) -> Result<Arc<Mutex<Box<dyn IoAsyncWrite>>>, P10Error> {
    match self {
      Self::StdOut => Ok(GLOBAL_STDOUT.clone()),

      Self::Object {
        object_store,
        object_path,
        display_path,
      } => {
        if !Self::overwrite() && object_store.head(object_path).await.is_ok() {
          crate::utils::exit_with_error(
            &format!(
              "Output file \"{}\" already exists.\n\nHint: Specify \
               --overwrite to automatically overwrite existing files",
              display_path.display()
            ),
            "",
          );
        }

        if log_write_to_stdout {
          println!("Writing \"{}\" …", display_path.display());
        }

        // Start a multipart upload to the object store
        let multipart_upload = object_store
          .put_multipart(object_path)
          .await
          .map_err(|e| P10Error::FileError {
            when: "Initiating put to object store".to_string(),
            details: e.to_string(),
          })?;

        // Create an async write stream that uploads multipart data
        let writer = Box::new(MultipartUploadAsyncWrite::new(multipart_upload));

        Ok(Arc::new(Mutex::new(writer)))
      }
    }
  }

  /// Commits the write of the output target once it is complete. This is
  /// necessary to finalize the write.
  ///
  pub async fn commit(
    self,
    stream: &mut Box<dyn IoAsyncWrite>,
  ) -> Result<(), P10Error> {
    match self {
      Self::StdOut => stream.flush().await.map_err(|e| P10Error::FileError {
        when: "Flushing stdout".to_string(),
        details: e.to_string(),
      }),

      Self::Object { .. } => {
        stream.shutdown().await.map_err(|e| P10Error::FileError {
          when: "Shutting down output stream".to_string(),
          details: e.to_string(),
        })
      }
    }
  }
}

/// Shared stdout write stream used for synchronization across async tasks so
/// that their output isn't interleaved.
///
static GLOBAL_STDOUT: LazyLock<Arc<Mutex<Box<dyn dcmfx::p10::IoAsyncWrite>>>> =
  LazyLock::new(|| Arc::new(Mutex::new(Box::new(BufWriter::new(stdout())))));

/// Makes an [`object_store::MultipartUpload`] usable as a
/// [`tokio::io::AsyncWrite`] stream by buffering data into parts of at least
/// 5 MiB before sending them.
///
struct MultipartUploadAsyncWrite {
  // Sender for complete parts ready to be uploaded. Parts are always at least
  // 5 MiB in size, except for the final part. Sending `None` indicates that all
  // parts have been written.
  tx: Option<mpsc::UnboundedSender<Option<Vec<u8>>>>,

  // Join handle for the task that's uploading the complete parts.
  join_handle: JoinHandle<object_store::Result<bool>>,

  // The current part being buffered and that will be sent via the channel once
  // it reaches 5 MiB in size.
  current_part: Vec<u8>,
}

const MINIMUM_PART_SIZE: usize = 5 * 1024 * 1024;

impl MultipartUploadAsyncWrite {
  fn new(mut multipart_upload: Box<dyn MultipartUpload>) -> Self {
    let (tx, mut rx) = mpsc::unbounded_channel::<Option<Vec<u8>>>();

    // Spawn a task that puts received parts for the multipart upload as they
    // are received.
    let join_handle = tokio::spawn(async move {
      while let Some(data) = rx.recv().await {
        match data {
          Some(data) => {
            if !data.is_empty() {
              multipart_upload.put_part(PutPayload::from(data)).await?;
            }
          }

          None => {
            multipart_upload.complete().await?;
            return Ok(true);
          }
        }
      }

      // The channel was closed before a `None` was received to indicate the
      // end of the parts, so abort the upload
      multipart_upload.abort().await?;

      Ok(false)
    });

    Self {
      tx: Some(tx),
      join_handle,
      current_part: vec![],
    }
  }
}

impl tokio::io::AsyncWrite for MultipartUploadAsyncWrite {
  fn poll_write(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<std::io::Result<usize>> {
    let this = self.get_mut();

    // Error if the stream has been shutdown
    let Some(tx) = this.tx.as_mut() else {
      return Poll::Ready(Err(std::io::Error::from(
        std::io::ErrorKind::BrokenPipe,
      )));
    };

    // Add data to the current part buffer
    this.current_part.extend_from_slice(buf);

    // Buffer until the minimum part size is reached
    if this.current_part.len() < MINIMUM_PART_SIZE {
      return Poll::Ready(Ok(buf.len()));
    }

    // Send the current part for upload
    match tx.send(Some(std::mem::take(&mut this.current_part))) {
      Ok(()) => Poll::Ready(Ok(buf.len())),
      Err(_) => Poll::Ready(Err(std::io::Error::new(
        std::io::ErrorKind::BrokenPipe,
        "Multipart uploadtask ended unexpectedly",
      ))),
    }
  }

  fn poll_flush(
    self: Pin<&mut Self>,
    _cx: &mut Context<'_>,
  ) -> Poll<std::io::Result<()>> {
    Poll::Ready(Ok(()))
  }

  fn poll_shutdown(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<std::io::Result<()>> {
    let this = self.get_mut();

    if let Some(tx) = this.tx.as_mut() {
      // Send the last part, and also a None to complete the upload
      let _ = tx.send(Some(std::mem::take(&mut this.current_part)));
      let _ = tx.send(None);

      // Drop/close the sender
      this.tx = None;
    }

    match futures::Future::poll(Pin::new(&mut this.join_handle), cx) {
      Poll::Ready(Ok(Ok(true))) => Poll::Ready(Ok(())),

      Poll::Ready(Ok(Ok(false))) => {
        Poll::Ready(Err(std::io::Error::other("Multipart upload aborted")))
      }

      Poll::Ready(Ok(Err(e))) => Poll::Ready(Err(std::io::Error::other(e))),

      Poll::Ready(Err(_)) => Poll::Ready(Err(std::io::Error::other(
        "Multipart upload task panicked",
      ))),

      Poll::Pending => Poll::Pending,
    }
  }
}
