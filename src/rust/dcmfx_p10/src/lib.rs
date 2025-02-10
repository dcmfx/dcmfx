//! Reads and writes the DICOM Part 10 (P10) binary format used to store and
//! transmit DICOM-based medical imaging information.

pub mod data_set_builder;
pub mod p10_error;
pub mod p10_read;
pub mod p10_token;
pub mod p10_write;
pub mod transforms;
pub mod uids;

mod internal;

use std::fs::File;
use std::io::Read;
use std::rc::Rc;

use dcmfx_core::DataSet;

pub use data_set_builder::DataSetBuilder;
pub use p10_error::P10Error;
pub use p10_read::{P10ReadConfig, P10ReadContext};
pub use p10_token::P10Token;
pub use p10_write::{P10WriteConfig, P10WriteContext};
pub use transforms::p10_custom_type_transform::{
  P10CustomTypeTransform, P10CustomTypeTransformError,
};
pub use transforms::p10_filter_transform::P10FilterTransform;
pub use transforms::p10_insert_transform::P10InsertTransform;
pub use transforms::p10_print_transform::P10PrintTransform;

/// Returns whether a file contains DICOM P10 data by checking for the presence
/// of the DICOM P10 header and the start of a File Meta Information Group
/// Length data element.
///
pub fn is_valid_file(filename: String) -> bool {
  match File::open(filename) {
    Ok(mut file) => {
      let mut buffer = vec![0u8, 138];
      match file.read_exact(&mut buffer) {
        Ok(_) => is_valid_bytes(&buffer),
        Err(_) => false,
      }
    }
    Err(_) => false,
  }
}

/// Returns whether the given bytes contain DICOM P10 data by checking for the
/// presence of the DICOM P10 header and the start of a File Meta Information
/// Group Length data element.
///
pub fn is_valid_bytes(bytes: &[u8]) -> bool {
  if bytes.len() < 138 {
    return false;
  }

  bytes[128..138] == *b"DICM\x02\0\0\0UL".as_slice()
}

/// Reads DICOM P10 data from a file into an in-memory data set.
///
pub fn read_file(filename: &str) -> Result<DataSet, P10Error> {
  match read_file_returning_builder_on_error(filename) {
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
pub fn read_file_returning_builder_on_error(
  filename: &str,
) -> Result<DataSet, (P10Error, Box<DataSetBuilder>)> {
  match File::open(filename) {
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

/// Reads DICOM P10 data from a read stream into an in-memory data set. This
/// will attempt to consume all data available in the read stream.
///
pub fn read_stream(
  stream: &mut dyn std::io::Read,
) -> Result<DataSet, (P10Error, Box<DataSetBuilder>)> {
  let mut context = P10ReadContext::new();
  let mut builder = Box::new(DataSetBuilder::new());

  loop {
    // Read the next tokens from the stream
    let tokens = match read_tokens_from_stream(stream, &mut context) {
      Ok(tokens) => tokens,
      Err(e) => return Err((e, builder)),
    };

    // Add the new tokens to the data set builder
    for token in tokens {
      match builder.add_token(&token) {
        Ok(_) => (),
        Err(e) => return Err((e, builder)),
      };
    }

    // If the data set builder is now complete then return the final data set
    if let Ok(final_data_set) = builder.final_data_set() {
      return Ok(final_data_set);
    }
  }
}

/// Reads the next DICOM P10 tokens from a read stream. This repeatedly reads
/// bytes from the read stream in 256 KiB chunks until at least one DICOM P10
/// token is made available by the read context or an error occurs.
///
pub fn read_tokens_from_stream(
  stream: &mut dyn std::io::Read,
  context: &mut P10ReadContext,
) -> Result<Vec<P10Token>, P10Error> {
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
        let mut buffer = vec![0u8; 256 * 1024];
        match stream.read(&mut buffer) {
          Ok(0) => context.write_bytes(vec![], true)?,

          Ok(bytes_count) => {
            buffer.resize(bytes_count, 0);
            context.write_bytes(buffer, false)?;
          }

          Err(e) => {
            return Err(P10Error::FileError {
              when: "Reading from stream".to_string(),
              details: e.to_string(),
            })
          }
        }
      }

      e => return e,
    }
  }
}

/// Reads DICOM P10 data from an in-memory vector of bytes into an in-memory
/// data set.
///
pub fn read_bytes(
  bytes: Vec<u8>,
) -> Result<DataSet, (P10Error, Box<DataSetBuilder>)> {
  let mut context = P10ReadContext::new();
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
            Ok(_) => (),
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

/// Writes a data set to a DICOM P10 file. This will overwrite any existing file
/// with the given name.
///
pub fn write_file(
  filename: &str,
  data_set: &DataSet,
  config: Option<P10WriteConfig>,
) -> Result<(), P10Error> {
  let file = File::create(filename);

  match file {
    Ok(mut file) => write_stream(&mut file, data_set, config),
    Err(e) => Err(P10Error::FileError {
      when: "Opening file".to_string(),
      details: e.to_string(),
    }),
  }
}

/// Writes a data set as DICOM P10 bytes directly to a write stream.
///
pub fn write_stream(
  stream: &mut dyn std::io::Write,
  data_set: &DataSet,
  config: Option<P10WriteConfig>,
) -> Result<(), P10Error> {
  let mut bytes_callback = |p10_bytes: Rc<Vec<u8>>| -> Result<(), P10Error> {
    match stream.write_all(&p10_bytes) {
      Ok(_) => Ok(()),
      Err(e) => Err(P10Error::FileError {
        when: "Writing DICOM P10 data to stream".to_string(),
        details: e.to_string(),
      }),
    }
  };

  let config = config.unwrap_or_default();

  data_set.to_p10_bytes(&mut bytes_callback, &config)?;

  stream.flush().map_err(|e| P10Error::FileError {
    when: "Writing DICOM P10 data to stream".to_string(),
    details: e.to_string(),
  })
}

/// Writes the specified DICOM P10 tokens to an output stream using the given
/// write context. Returns whether a [`P10Token::End`] token was present in the
/// tokens.
///
pub fn write_tokens_to_stream(
  tokens: &[P10Token],
  stream: &mut dyn std::io::Write,
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

/// Adds functions to [`DataSet`] for converting to and from the DICOM P10
/// format.
///
pub trait DataSetP10Extensions
where
  Self: Sized,
{
  /// Reads DICOM P10 data from a file into an in-memory data set.
  ///
  fn read_p10_file(filename: &str) -> Result<Self, P10Error>;

  /// Reads DICOM P10 data from a read stream into an in-memory data set. This
  /// will attempt to consume all data available in the read stream.
  ///
  fn read_p10_stream(
    stream: &mut dyn std::io::Read,
  ) -> Result<DataSet, P10Error>;

  /// Writes a data set to a DICOM P10 file. This will overwrite any existing
  /// file with the given name.
  ///
  fn write_p10_file(
    &self,
    filename: &str,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error>;

  /// Writes a data set as DICOM P10 bytes directly to a write stream.
  ///
  fn write_p10_stream(
    &self,
    stream: &mut dyn std::io::Write,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error>;

  /// Converts a data set to DICOM P10 tokens that are returned via the passed
  /// callback.
  ///
  fn to_p10_tokens<E>(
    &self,
    token_callback: &mut impl FnMut(&P10Token) -> Result<(), E>,
  ) -> Result<(), E>;

  /// Converts a data set to DICOM P10 bytes that are returned via the passed
  /// callback.
  ///
  fn to_p10_bytes(
    &self,
    bytes_callback: &mut impl FnMut(Rc<Vec<u8>>) -> Result<(), P10Error>,
    config: &P10WriteConfig,
  ) -> Result<(), P10Error>;
}

impl DataSetP10Extensions for DataSet {
  fn read_p10_file(filename: &str) -> Result<Self, P10Error> {
    read_file(filename)
  }

  fn read_p10_stream(
    stream: &mut dyn std::io::Read,
  ) -> Result<DataSet, P10Error> {
    read_stream(stream).map_err(|e| e.0)
  }

  fn write_p10_file(
    &self,
    filename: &str,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error> {
    write_file(filename, self, config)
  }

  fn write_p10_stream(
    &self,
    stream: &mut dyn std::io::Write,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error> {
    write_stream(stream, self, config)
  }

  fn to_p10_tokens<E>(
    &self,
    token_callback: &mut impl FnMut(&P10Token) -> Result<(), E>,
  ) -> Result<(), E> {
    p10_write::data_set_to_tokens(self, token_callback)
  }

  fn to_p10_bytes(
    &self,
    bytes_callback: &mut impl FnMut(Rc<Vec<u8>>) -> Result<(), P10Error>,
    config: &P10WriteConfig,
  ) -> Result<(), P10Error> {
    p10_write::data_set_to_bytes(self, bytes_callback, config)
  }
}
