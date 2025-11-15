//! Reads and writes the DICOM Part 10 (P10) binary format used to store and
//! transmit DICOM-based medical imaging information.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::ToString, vec, vec::Vec};

pub mod data_set_builder;
pub mod p10_error;
pub mod p10_read;
pub mod p10_read_config;
pub mod p10_token;
pub mod p10_write;
pub mod p10_write_config;
pub mod transforms;
pub mod uids;

mod internal;
mod io;

#[cfg(feature = "std")]
use std::path::Path;

pub use io::{IoError, IoRead, IoWrite};

#[cfg(feature = "async")]
pub use io::{IoAsyncRead, IoAsyncWrite};

use dcmfx_core::{DataElementTag, DataSet, DataSetPath, RcByteSlice};

pub use data_set_builder::DataSetBuilder;
pub use p10_error::P10Error;
pub use p10_read::P10ReadContext;
pub use p10_read_config::P10ReadConfig;
pub use p10_token::P10Token;
pub use p10_write::P10WriteContext;
pub use p10_write_config::P10WriteConfig;
pub use transforms::p10_custom_type_transform::{
  P10CustomTypeTransform, P10CustomTypeTransformError,
};
pub use transforms::p10_filter_transform::P10FilterTransform;
pub use transforms::p10_insert_transform::P10InsertTransform;
pub use transforms::p10_print_transform::P10PrintTransform;

/// Returns whether a file contains DICOM P10 data by checking for the presence
/// of the 'DICM' prefix at offset 128.
///
#[cfg(feature = "std")]
pub fn is_valid_file<P: AsRef<Path>>(filename: P) -> bool {
  use std::io::Read;

  match std::fs::File::open(filename) {
    Ok(mut file) => {
      let mut buffer = [0u8; 132];
      match file.read_exact(&mut buffer) {
        Ok(_) => is_valid_bytes(&buffer),
        Err(_) => false,
      }
    }

    Err(_) => false,
  }
}

/// Returns whether a file contains DICOM P10 data by checking for the presence
/// of the 'DICM' prefix at offset 128.
///
#[cfg(feature = "async")]
pub async fn is_valid_file_async<P: AsRef<Path>>(filename: P) -> bool {
  use tokio::io::AsyncReadExt;

  match tokio::fs::File::open(filename).await {
    Ok(mut file) => {
      let mut buffer = [0u8; 132];
      match file.read_exact(&mut buffer).await {
        Ok(_) => is_valid_bytes(&buffer),
        Err(_) => false,
      }
    }

    Err(_) => false,
  }
}

/// Returns whether the given bytes contain DICOM P10 data by checking for the
/// presence of the 'DICM' prefix at offset 128.
///
pub fn is_valid_bytes(bytes: &[u8]) -> bool {
  bytes.len() >= 132 && bytes[128..132] == *b"DICM".as_slice()
}

/// Reads DICOM P10 data from a file into an in-memory data set.
///
#[cfg(feature = "std")]
pub fn read_file<P: AsRef<Path>>(filename: P) -> Result<DataSet, P10Error> {
  match read_file_returning_builder_on_error(filename) {
    Ok(data_set) => Ok(data_set),
    Err((e, _)) => Err(e),
  }
}

/// Reads DICOM P10 data from a file into an in-memory data set.
///
#[cfg(feature = "async")]
pub async fn read_file_async<P: AsRef<Path>>(
  filename: P,
) -> Result<DataSet, P10Error> {
  match read_file_returning_builder_on_error_async(filename).await {
    Ok(data_set) => Ok(data_set),
    Err((e, _)) => Err(e),
  }
}

/// Reads DICOM P10 data from a file into an in-memory data set. In the case of
/// an error occurring during the read both the error and the data set builder
/// at the time of the error are returned.
///
/// This allows for the data that was successfully read prior to the error to be
/// converted into a partially-complete data set.
///
#[cfg(feature = "std")]
pub fn read_file_returning_builder_on_error<P: AsRef<Path>>(
  filename: P,
) -> Result<DataSet, (P10Error, Box<DataSetBuilder>)> {
  match std::fs::File::open(filename) {
    Ok(mut file) => read_stream(&mut file),

    Err(e) => Err((
      P10Error::FileError {
        when: "Opening file".to_string(),
        details: e.to_string(),
      },
      Box::new(DataSetBuilder::new()),
    )),
  }
}

/// Reads DICOM P10 data from a file into an in-memory data set. In the case of
/// an error occurring during the read both the error and the data set builder
/// at the time of the error are returned.
///
/// This allows for the data that was successfully read prior to the error to be
/// converted into a partially-complete data set.
///
#[cfg(feature = "async")]
pub async fn read_file_returning_builder_on_error_async<P: AsRef<Path>>(
  filename: P,
) -> Result<DataSet, (P10Error, Box<DataSetBuilder>)> {
  match tokio::fs::File::open(filename).await {
    Ok(mut file) => read_stream_async(&mut file).await,

    Err(e) => Err((
      P10Error::FileError {
        when: "Opening file".to_string(),
        details: e.to_string(),
      },
      Box::new(DataSetBuilder::new()),
    )),
  }
}

/// Reads DICOM P10 data from a read stream into an in-memory data set. This
/// will attempt to consume all data available in the read stream.
///
pub fn read_stream<S: IoRead>(
  stream: &mut S,
) -> Result<DataSet, (P10Error, Box<DataSetBuilder>)> {
  let mut context = P10ReadContext::new(None);
  let mut builder = Box::new(DataSetBuilder::new());

  loop {
    // Read the next tokens from the stream
    let tokens = match read_tokens_from_stream(stream, &mut context, None) {
      Ok(tokens) => tokens,
      Err(e) => return Err((e, builder)),
    };

    // Add the new tokens to the data set builder
    match builder.add_tokens(&tokens) {
      Ok(()) => (),
      Err(e) => return Err((e, builder)),
    };

    // If the data set builder is now complete then return the final data set
    if let Ok(final_data_set) = builder.final_data_set() {
      return Ok(final_data_set);
    }
  }
}

/// Reads DICOM P10 data from a read stream into an in-memory data set. This
/// will attempt to consume all data available in the read stream.
///
#[cfg(feature = "async")]
pub async fn read_stream_async<I: IoAsyncRead>(
  stream: &mut I,
) -> Result<DataSet, (P10Error, Box<DataSetBuilder>)> {
  let mut context = P10ReadContext::new(None);
  let mut builder = Box::new(DataSetBuilder::new());

  loop {
    // Read the next tokens from the stream
    let tokens =
      match read_tokens_from_stream_async(stream, &mut context, None).await {
        Ok(tokens) => tokens,
        Err(e) => return Err((e, builder)),
      };

    // Add the new tokens to the data set builder
    match builder.add_tokens(&tokens) {
      Ok(()) => (),
      Err(e) => return Err((e, builder)),
    }

    // If the data set builder is now complete then return the final data set
    if let Ok(final_data_set) = builder.final_data_set() {
      return Ok(final_data_set);
    }
  }
}

/// Reads the next DICOM P10 tokens from a read stream. This repeatedly reads
/// chunks of bytes from the read stream until at least one DICOM P10 token is
/// made available by the read context or an error occurs.
///
/// The chunk size defaults to 256 KiB if not specified.
///
pub fn read_tokens_from_stream<S: IoRead>(
  stream: &mut S,
  context: &mut P10ReadContext,
  chunk_size: Option<usize>,
) -> Result<Vec<P10Token>, P10Error> {
  let chunk_size = chunk_size.unwrap_or(256 * 1024);

  loop {
    match context.read_tokens() {
      Ok(tokens) => {
        if tokens.is_empty() {
          continue;
        } else {
          return Ok(tokens);
        }
      }

      // If the read context needs more data then read bytes from the stream,
      // write them to the read context, and try again
      Err(P10Error::DataRequired { .. }) => {
        let mut buffer = vec![0u8; chunk_size];

        let read_bytes_count =
          stream.read(&mut buffer).map_err(|e| P10Error::FileError {
            when: "Reading from stream".to_string(),
            details: e.to_string(),
          })?;

        if read_bytes_count == 0 {
          context.write_bytes(RcByteSlice::empty(), true)?;
        } else {
          buffer.resize(read_bytes_count, 0);
          context.write_bytes(buffer.into(), false)?;
        }
      }

      e => return e,
    }
  }
}

/// Reads the next DICOM P10 tokens from a read stream. This repeatedly reads
/// chunks of bytes from the read stream until at least one DICOM P10 token is
/// made available by the read context or an error occurs.
///
/// The chunk size defaults to 256 KiB if not specified.
///
#[cfg(feature = "async")]
pub async fn read_tokens_from_stream_async<I: IoAsyncRead>(
  stream: &mut I,
  context: &mut P10ReadContext,
  chunk_size: Option<usize>,
) -> Result<Vec<P10Token>, P10Error> {
  use tokio::io::AsyncReadExt;

  let chunk_size = chunk_size.unwrap_or(256 * 1024);

  loop {
    match context.read_tokens() {
      Ok(tokens) => {
        if tokens.is_empty() {
          continue;
        } else {
          return Ok(tokens);
        }
      }

      // If the read context needs more data then read bytes from the stream,
      // write them to the read context, and try again
      Err(P10Error::DataRequired { .. }) => {
        let mut buffer = vec![0u8; chunk_size];

        let read_bytes_count =
          stream
            .read(&mut buffer)
            .await
            .map_err(|e| P10Error::FileError {
              when: "Reading from stream".to_string(),
              details: e.to_string(),
            })?;

        if read_bytes_count == 0 {
          context.write_bytes(RcByteSlice::empty(), true)?;
        } else {
          buffer.resize(read_bytes_count, 0);
          context.write_bytes(buffer.into(), false)?;
        }
      }

      e => return e,
    }
  }
}

/// Reads DICOM P10 data from a vector of bytes into a data set.
///
pub fn read_bytes(
  bytes: RcByteSlice,
) -> Result<DataSet, (P10Error, Box<DataSetBuilder>)> {
  let mut context = P10ReadContext::new(None);
  let mut builder = Box::new(DataSetBuilder::new());

  // Add the bytes to the P10 read context
  match context.write_bytes(bytes, true) {
    Ok(()) => (),
    Err(e) => return Err((e, builder)),
  };

  loop {
    // Read the next tokens from the context
    match context.read_tokens() {
      Ok(tokens) => {
        // Add the new tokens to the data set builder
        for token in tokens.iter() {
          match builder.add_token(token) {
            Ok(()) => (),
            Err(e) => return Err((e, builder)),
          };
        }

        // If the data set builder is now complete then return the final data
        // set
        if let Ok(final_data_set) = builder.final_data_set() {
          return Ok(final_data_set);
        }
      }

      Err(e) => return Err((e, builder)),
    }
  }
}

/// Reads DICOM P10 data from a file into an in-memory data set. Only the
/// specified data elements at the root of the main data set are read, if
/// present. The file will only be read up to the point required to return the
/// requested data elements.
///
#[cfg(feature = "std")]
pub fn read_file_partial<P: AsRef<Path>>(
  filename: P,
  tags: &[DataElementTag],
  config: Option<P10ReadConfig>,
) -> Result<DataSet, P10Error> {
  match std::fs::File::open(filename) {
    Ok(mut file) => read_stream_partial(&mut file, tags, config),

    Err(e) => Err(P10Error::FileError {
      when: "Opening file".to_string(),
      details: e.to_string(),
    }),
  }
}

/// Reads DICOM P10 data from a file into an in-memory data set. Only the
/// specified data elements at the root of the main data set are read, if
/// present. The file will only be read up to the point required to return the
/// requested data elements.
///
#[cfg(feature = "async")]
pub async fn read_file_partial_async<P: AsRef<Path>>(
  filename: P,
  tags: &[DataElementTag],
  config: Option<P10ReadConfig>,
) -> Result<DataSet, P10Error> {
  match tokio::fs::File::open(filename).await {
    Ok(mut file) => read_stream_partial_async(&mut file, tags, config).await,

    Err(e) => Err(P10Error::FileError {
      when: "Opening file".to_string(),
      details: e.to_string(),
    }),
  }
}

/// Reads DICOM P10 data from a stream into an in-memory data set. Only the
/// specified data elements at the root of the main data set are read, if
/// present. The stream will only be read up to the point required to return the
/// requested data elements.
///
pub fn read_stream_partial<S: IoRead>(
  stream: &mut S,
  tags: &[DataElementTag],
  config: Option<P10ReadConfig>,
) -> Result<DataSet, P10Error> {
  let (largest_tag, mut filter, mut chunk_size) =
    read_stream_partial_prepare(tags);

  let mut context = P10ReadContext::new(config);
  let mut data_set_builder = DataSetBuilder::new();
  let mut is_done = false;

  while !is_done {
    let tokens = read_tokens_from_stream(stream, &mut context, chunk_size)?;

    read_stream_partial_process_tokens(
      &tokens,
      largest_tag,
      &mut filter,
      &mut data_set_builder,
      &mut is_done,
      &mut chunk_size,
    )?;
  }

  Ok(read_stream_partial_complete(data_set_builder, tags))
}

/// Reads DICOM P10 data from a stream into an in-memory data set. Only the
/// specified data elements at the root of the main data set are read, if
/// present. The stream will only be read up to the point required to return the
/// requested data elements.
///
#[cfg(feature = "async")]
pub async fn read_stream_partial_async<I: IoAsyncRead>(
  stream: &mut I,
  tags: &[DataElementTag],
  config: Option<P10ReadConfig>,
) -> Result<DataSet, P10Error> {
  let (largest_tag, mut filter, mut chunk_size) =
    read_stream_partial_prepare(tags);

  let mut context = P10ReadContext::new(config);
  let mut data_set_builder = DataSetBuilder::new();
  let mut is_done = false;

  while !is_done {
    let tokens =
      read_tokens_from_stream_async(stream, &mut context, chunk_size).await?;

    read_stream_partial_process_tokens(
      &tokens,
      largest_tag,
      &mut filter,
      &mut data_set_builder,
      &mut is_done,
      &mut chunk_size,
    )?;
  }

  Ok(read_stream_partial_complete(data_set_builder, tags))
}

fn read_stream_partial_prepare(
  tags: &[DataElementTag],
) -> (DataElementTag, P10FilterTransform, Option<usize>) {
  // Find the largest data element tag being read
  let largest_tag = tags.iter().max().cloned().unwrap_or(DataElementTag::ZERO);

  // Create filter transform that only allows the specified root tags
  let filter = {
    let tags = tags.to_vec();
    P10FilterTransform::new(Box::new(move |tag, _vr, _length, path| -> bool {
      !path.is_root() || tags.contains(&tag)
    }))
  };

  // The first read chunk is small because in a partial read it is common to
  // only read a few data elements at the start of a P10 file, such as the
  // Transfer Syntax UID, SOP Instance UID, SOP Class UID, etc. If this chunk
  // size is insufficient then subsequent read chunks will be the larger default
  // size.
  let chunk_size = Some(8 * 1024);

  (largest_tag, filter, chunk_size)
}

fn read_stream_partial_process_tokens(
  tokens: &[P10Token],
  largest_tag: DataElementTag,
  filter: &mut P10FilterTransform,
  data_set_builder: &mut DataSetBuilder,
  is_done: &mut bool,
  chunk_size: &mut Option<usize>,
) -> Result<(), P10Error> {
  for token in tokens {
    if filter.add_token(token)? {
      data_set_builder.add_token(token)?;
    }

    match token {
      P10Token::DataElementHeader { tag, path, .. }
      | P10Token::SequenceStart { tag, path, .. } => {
        if *tag > largest_tag && path.is_root() {
          *is_done = true;
          break;
        }
      }

      P10Token::End => {
        *is_done = true;
        break;
      }

      _ => (),
    }
  }

  *chunk_size = None;

  Ok(())
}

fn read_stream_partial_complete(
  mut data_set_builder: DataSetBuilder,
  tags: &[DataElementTag],
) -> DataSet {
  data_set_builder.force_end();
  let mut data_set = data_set_builder.final_data_set().unwrap();

  // Exclude File Meta Information tags unless they were explicitly requested
  data_set.retain(|tag, _value| {
    !tag.is_file_meta_information() || tags.contains(&tag)
  });

  data_set
}

/// Writes a data set to a DICOM P10 file. This will overwrite any existing file
/// with the given name.
///
#[cfg(feature = "std")]
pub fn write_file<P: AsRef<Path>>(
  filename: P,
  data_set: &DataSet,
  config: Option<P10WriteConfig>,
) -> Result<(), P10Error> {
  let file = std::fs::File::create(filename);

  match file {
    Ok(mut file) => write_stream(&mut file, data_set, config),

    Err(e) => Err(P10Error::FileError {
      when: "Opening file".to_string(),
      details: e.to_string(),
    }),
  }
}

/// Writes a data set to a DICOM P10 file. This will overwrite any existing file
/// with the given name.
///
#[cfg(feature = "async")]
pub async fn write_file_async<P: AsRef<Path>>(
  filename: P,
  data_set: &DataSet,
  config: Option<P10WriteConfig>,
) -> Result<(), P10Error> {
  let file = tokio::fs::File::create(filename).await;

  match file {
    Ok(mut file) => write_stream_async(&mut file, data_set, config).await,

    Err(e) => Err(P10Error::FileError {
      when: "Opening file".to_string(),
      details: e.to_string(),
    }),
  }
}

/// Writes a data set as DICOM P10 bytes directly to a write stream.
///
pub fn write_stream<S: IoWrite>(
  stream: &mut S,
  data_set: &DataSet,
  config: Option<P10WriteConfig>,
) -> Result<(), P10Error> {
  let mut bytes_callback = |p10_bytes: RcByteSlice| -> Result<(), P10Error> {
    match stream.write_all(&p10_bytes) {
      Ok(()) => Ok(()),

      Err(e) => Err(P10Error::FileError {
        when: "Writing DICOM P10 data to stream".to_string(),
        details: e.to_string(),
      }),
    }
  };

  data_set.to_p10_bytes(&mut bytes_callback, config)?;

  stream.flush().map_err(|e| P10Error::FileError {
    when: "Writing DICOM P10 data to stream".to_string(),
    details: e.to_string(),
  })
}

/// Writes a data set as DICOM P10 bytes directly to a write stream.
///
#[cfg(feature = "async")]
pub async fn write_stream_async<S: IoAsyncWrite>(
  stream: &mut S,
  data_set: &DataSet,
  config: Option<P10WriteConfig>,
) -> Result<(), P10Error> {
  use tokio::io::AsyncWriteExt;

  let mut bytes_callback =
    async |p10_bytes: RcByteSlice| -> Result<(), P10Error> {
      match stream.write_all(&p10_bytes).await {
        Ok(()) => Ok(()),

        Err(e) => Err(P10Error::FileError {
          when: "Writing DICOM P10 data to stream".to_string(),
          details: e.to_string(),
        }),
      }
    };

  data_set
    .to_p10_bytes_async(&mut bytes_callback, config)
    .await?;

  stream.flush().await.map_err(|e| P10Error::FileError {
    when: "Writing DICOM P10 data to stream".to_string(),
    details: e.to_string(),
  })
}

/// Writes the specified DICOM P10 tokens to an output stream using the given
/// write context. Returns whether a [`P10Token::End`] token was present in the
/// tokens.
///
pub fn write_tokens_to_stream<S: IoWrite>(
  tokens: &[P10Token],
  stream: &mut S,
  context: &mut P10WriteContext,
) -> Result<bool, P10Error> {
  for token in tokens.iter() {
    context.write_token(token)?;
  }

  let p10_bytes = context.read_bytes();
  for bytes in p10_bytes.iter() {
    stream.write_all(bytes).map_err(|e| P10Error::FileError {
      when: "Writing to output stream".to_string(),
      details: e.to_string(),
    })?;
  }

  if tokens.last() == Some(&P10Token::End) {
    stream.flush().map_err(|e| P10Error::FileError {
      when: "Writing to output stream".to_string(),
      details: e.to_string(),
    })?;

    Ok(true)
  } else {
    Ok(false)
  }
}

/// Writes the specified DICOM P10 tokens to an output stream using the given
/// write context. Returns whether a [`P10Token::End`] token was present in the
/// tokens.
///
#[cfg(feature = "async")]
pub async fn write_tokens_to_stream_async<S: IoAsyncWrite>(
  tokens: &[P10Token],
  stream: &mut S,
  context: &mut P10WriteContext,
) -> Result<bool, P10Error> {
  use tokio::io::AsyncWriteExt;

  for token in tokens.iter() {
    context.write_token(token)?;
  }

  let p10_bytes = context.read_bytes();
  for bytes in p10_bytes.iter() {
    stream
      .write_all(bytes)
      .await
      .map_err(|e| P10Error::FileError {
        when: "Writing to output stream".to_string(),
        details: e.to_string(),
      })?;
  }

  if tokens.last() == Some(&P10Token::End) {
    stream.flush().await.map_err(|e| P10Error::FileError {
      when: "Writing to output stream".to_string(),
      details: e.to_string(),
    })?;

    Ok(true)
  } else {
    Ok(false)
  }
}

/// Adds functions to [`DataSet`] for converting to and from the DICOM P10
/// format.
///
pub trait DataSetP10Extensions
where
  Self: Sized,
{
  /// Reads DICOM P10 data from a file into an in-memory data set.
  ///
  #[cfg(feature = "std")]
  fn read_p10_file<P: AsRef<Path>>(filename: P) -> Result<Self, P10Error>;

  /// Reads DICOM P10 data from a read stream into an in-memory data set. This
  /// will attempt to consume all data available in the read stream.
  ///
  fn read_p10_stream<S: IoRead>(stream: &mut S) -> Result<Self, P10Error>;

  /// Reads DICOM P10 data from a vector of bytes into a data set.
  ///
  fn read_p10_bytes(
    bytes: RcByteSlice,
  ) -> Result<Self, (P10Error, Box<DataSetBuilder>)>;

  /// Writes a data set to a DICOM P10 file. This will overwrite any existing
  /// file with the given name.
  ///
  #[cfg(feature = "std")]
  fn write_p10_file<P: AsRef<Path>>(
    &self,
    filename: P,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error>;

  /// Writes a data set as DICOM P10 bytes directly to a write stream.
  ///
  fn write_p10_stream<S: IoWrite>(
    &self,
    stream: &mut S,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error>;

  /// Converts a data set to a vector of DICOM P10 tokens.
  ///
  fn to_p10_tokens(&self) -> Vec<P10Token>;

  /// Converts a data set to DICOM P10 tokens that are returned via the passed
  /// callback.
  ///
  fn to_p10_token_stream<E>(
    &self,
    token_callback: &mut impl FnMut(P10Token) -> Result<(), E>,
  ) -> Result<(), E>;

  /// Converts a data set to DICOM P10 bytes that are returned via the passed
  /// callback.
  ///
  fn to_p10_bytes(
    &self,
    bytes_callback: &mut impl FnMut(RcByteSlice) -> Result<(), P10Error>,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error>;
}

/// Adds functions to [`DataSet`] for converting to and from the DICOM P10
/// format.
///
#[cfg(feature = "async")]
#[async_trait::async_trait(?Send)]
pub trait DataSetP10AsyncExtensions
where
  Self: Sized,
{
  /// Reads DICOM P10 data from a file into an in-memory data set.
  ///
  async fn read_p10_file_async<P: AsRef<Path>>(
    filename: P,
  ) -> Result<Self, P10Error>;

  /// Reads DICOM P10 data from a read stream into an in-memory data set. This
  /// will attempt to consume all data available in the read stream.
  ///
  async fn read_p10_stream_async<I: IoAsyncRead>(
    stream: &mut I,
  ) -> Result<Self, P10Error>;

  /// Writes a data set to a DICOM P10 file. This will overwrite any existing
  /// file with the given name.
  ///
  #[cfg(feature = "std")]
  async fn write_p10_file_async<P: AsRef<Path>>(
    &self,
    filename: P,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error>;

  /// Writes a data set as DICOM P10 bytes directly to a write stream.
  ///
  async fn write_p10_stream_async<S: IoAsyncWrite>(
    &self,
    stream: &mut S,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error>;

  async fn to_p10_token_stream_async<E>(
    &self,
    token_callback: &mut impl AsyncFnMut(P10Token) -> Result<(), E>,
  ) -> Result<(), E>;

  async fn to_p10_bytes_async(
    &self,
    bytes_callback: &mut impl AsyncFnMut(RcByteSlice) -> Result<(), P10Error>,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error>;
}

impl DataSetP10Extensions for DataSet {
  #[cfg(feature = "std")]
  fn read_p10_file<P: AsRef<Path>>(filename: P) -> Result<Self, P10Error> {
    read_file(filename)
  }

  fn read_p10_stream<S: IoRead>(stream: &mut S) -> Result<DataSet, P10Error> {
    read_stream(stream).map_err(|e| e.0)
  }

  fn read_p10_bytes(
    bytes: RcByteSlice,
  ) -> Result<Self, (P10Error, Box<DataSetBuilder>)> {
    read_bytes(bytes)
  }

  #[cfg(feature = "std")]
  fn write_p10_file<P: AsRef<Path>>(
    &self,
    filename: P,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error> {
    write_file(filename, self, config)
  }

  fn write_p10_stream<S: IoWrite>(
    &self,
    stream: &mut S,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error> {
    write_stream(stream, self, config)
  }

  fn to_p10_tokens(&self) -> Vec<P10Token> {
    let mut tokens = vec![];
    self
      .to_p10_token_stream::<()>(&mut |token: P10Token| {
        tokens.push(token);
        Ok(())
      })
      .unwrap();

    tokens
  }

  fn to_p10_token_stream<E>(
    &self,
    token_callback: &mut impl FnMut(P10Token) -> Result<(), E>,
  ) -> Result<(), E> {
    p10_write::data_set_to_tokens(self, &DataSetPath::new(), token_callback)
  }

  fn to_p10_bytes(
    &self,
    bytes_callback: &mut impl FnMut(RcByteSlice) -> Result<(), P10Error>,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error> {
    p10_write::data_set_to_bytes(
      self,
      &DataSetPath::new(),
      bytes_callback,
      config,
    )
  }
}

#[cfg(feature = "async")]
#[async_trait::async_trait(?Send)]
impl DataSetP10AsyncExtensions for DataSet {
  async fn read_p10_file_async<P: AsRef<Path>>(
    filename: P,
  ) -> Result<Self, P10Error> {
    read_file_async(filename).await
  }

  async fn read_p10_stream_async<I: IoAsyncRead>(
    stream: &mut I,
  ) -> Result<DataSet, P10Error> {
    read_stream_async(stream).await.map_err(|e| e.0)
  }

  async fn write_p10_file_async<P: AsRef<Path>>(
    &self,
    filename: P,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error> {
    write_file_async(filename, self, config).await
  }

  async fn write_p10_stream_async<S: IoAsyncWrite>(
    &self,
    stream: &mut S,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error> {
    write_stream_async(stream, self, config).await
  }

  async fn to_p10_token_stream_async<E>(
    &self,
    token_callback: &mut impl AsyncFnMut(P10Token) -> Result<(), E>,
  ) -> Result<(), E> {
    p10_write::data_set_to_tokens_async(
      self,
      &DataSetPath::new(),
      token_callback,
    )
    .await
  }

  async fn to_p10_bytes_async(
    &self,
    bytes_callback: &mut impl AsyncFnMut(RcByteSlice) -> Result<(), P10Error>,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error> {
    p10_write::data_set_to_bytes_async(
      self,
      &DataSetPath::new(),
      bytes_callback,
      config,
    )
    .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use dcmfx_core::dictionary;

  #[test]
  fn read_file_partial_test() {
    let path = "../../../test/assets/pydicom/test_files/693_J2KI.dcm";

    let ds = read_file_partial(
      path,
      &[dictionary::ROWS.tag, dictionary::COLUMNS.tag],
      None,
    )
    .unwrap();

    assert_eq!(
      ds.tags(),
      vec![dictionary::ROWS.tag, dictionary::COLUMNS.tag]
    );
  }
}
