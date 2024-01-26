pub mod data_set_builder;
pub mod p10_error;
pub mod p10_part;
pub mod p10_read;
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
pub use p10_part::P10Part;
pub use p10_read::{P10ReadConfig, P10ReadContext};
pub use p10_write::{P10WriteConfig, P10WriteContext};
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

/// Returns whether `bytes` contains DICOM P10 data by checking for the presence
/// of the DICOM P10 header and the start of a File Meta Information Group
/// Length data element.
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
    // Read the next parts from the stream
    let parts = match read_parts_from_stream(stream, &mut context) {
      Ok(parts) => parts,
      Err(e) => return Err((e, builder)),
    };

    // Add the new parts to the data set builder
    for part in parts {
      match builder.add_part(&part) {
        Ok(_) => (),
        Err(e) => return Err((e, builder)),
      };
    }

    // If the data set builder is now complete then return the final data set
    if builder.is_complete() {
      return Ok(builder.final_data_set().unwrap());
    }
  }
}

/// Reads the next DICOM P10 parts from a read stream. This repeatedly reads
/// bytes from the read stream in 256 KiB chunks until at least one DICOM P10
/// part is made available by the read context or an error occurs.
///
pub fn read_parts_from_stream(
  stream: &mut dyn std::io::Read,
  context: &mut P10ReadContext,
) -> Result<Vec<P10Part>, P10Error> {
  loop {
    match context.read_parts() {
      Ok(parts) if !parts.is_empty() => return Ok(parts),

      // If the read context needs more data then read bytes from the stream,
      // write them to the read context, and try again
      Err(P10Error::DataRequired { .. }) => {
        let mut buffer = vec![0; 256 * 1024];
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
    // Read the next parts from the context
    match context.read_parts() {
      Ok(parts) => {
        // Add the new parts to the data set builder
        for part in parts.iter() {
          match builder.add_part(part) {
            Ok(_) => (),
            Err(e) => return Err((e, builder)),
          };
        }

        // If the data set builder is now complete then return the final data
        // set
        if builder.is_complete() {
          return Ok(builder.final_data_set().unwrap());
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

/// Adds functions to [`DataSet`] for converting to and from the DICOM P10
/// format.
///
pub trait DataSetP10Extensions
where
  Self: Sized,
{
  fn read_p10_file(filename: &str) -> Result<Self, P10Error>;
  fn read_p10_stream(
    stream: &mut dyn std::io::Read,
  ) -> Result<DataSet, P10Error>;
  fn write_p10_file(
    &self,
    filename: &str,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error>;
  fn write_p10_stream(
    &self,
    stream: &mut dyn std::io::Write,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error>;
  fn to_p10_parts<E>(
    &self,
    part_callback: &mut impl FnMut(&P10Part) -> Result<(), E>,
  ) -> Result<(), E>;
  fn to_p10_bytes(
    &self,
    bytes_callback: &mut impl FnMut(Rc<Vec<u8>>) -> Result<(), P10Error>,
    config: &P10WriteConfig,
  ) -> Result<(), P10Error>;
}

impl DataSetP10Extensions for DataSet {
  /// Reads DICOM P10 data from a file into an in-memory data set.
  ///
  fn read_p10_file(filename: &str) -> Result<Self, P10Error> {
    read_file(filename)
  }

  /// Reads DICOM P10 data from a read stream into an in-memory data set. This
  /// will attempt to consume all data available in the read stream.
  ///
  fn read_p10_stream(
    stream: &mut dyn std::io::Read,
  ) -> Result<DataSet, P10Error> {
    read_stream(stream).map_err(|e| e.0)
  }

  /// Writes a data set to a DICOM P10 file. This will overwrite any existing
  /// file with the given name.
  ///
  fn write_p10_file(
    &self,
    filename: &str,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error> {
    write_file(filename, self, config)
  }

  /// Writes a data set as DICOM P10 bytes directly to a write stream.
  ///
  fn write_p10_stream(
    &self,
    stream: &mut dyn std::io::Write,
    config: Option<P10WriteConfig>,
  ) -> Result<(), P10Error> {
    write_stream(stream, self, config)
  }

  /// Converts a data set to DICOM P10 parts that are returned via the passed
  /// callback.
  ///
  fn to_p10_parts<E>(
    &self,
    part_callback: &mut impl FnMut(&P10Part) -> Result<(), E>,
  ) -> Result<(), E> {
    p10_write::data_set_to_parts(self, part_callback)
  }

  /// Converts a data set to DICOM P10 bytes that are returned via the passed
  /// callback.
  ///
  fn to_p10_bytes(
    &self,
    bytes_callback: &mut impl FnMut(Rc<Vec<u8>>) -> Result<(), P10Error>,
    config: &P10WriteConfig,
  ) -> Result<(), P10Error> {
    p10_write::data_set_to_bytes(self, bytes_callback, config)
  }
}
