#[cfg(feature = "std")]
use std::collections::VecDeque;

#[cfg(not(feature = "std"))]
use alloc::{collections::VecDeque, vec, vec::Vec};

use dcmfx_core::RcByteSlice;

/// A byte stream that takes incoming chunks of binary data of any size and
/// allows the resulting data to to read and peeked as if it were one large
/// stream of bytes.
///
/// Incoming bytes can optionally be passed through zlib inflate prior to being
/// made available for reading.
///
#[derive(Debug)]
pub struct ByteStream {
  bytes_queue: VecDeque<RcByteSlice>,
  bytes_queue_size: u64,
  bytes_read: u64,
  is_writing_finished: bool,
  zlib_stream: Option<flate2::Decompress>,
  zlib_input_queue: VecDeque<RcByteSlice>,
  zlib_inflate_complete: bool,
}

#[derive(Debug)]
pub enum ByteStreamError {
  /// Data was not read because the byte stream does not have the number of
  /// bytes requested available and needs more bytes to be written to it first.
  DataRequired,

  /// Data was not read because it would go past the end of the byte stream.
  DataEnd,

  /// Data written to a byte stream that has zlib inflate active was not valid
  /// zlib data.
  ZlibDataError,

  /// Data was written to a byte stream after its final bytes have already been
  /// written.
  WriteAfterCompletion,
}

/// Zlib data is inflated into chunks of at most this size to protect against
/// zlib bombs.
///
const ZLIB_INFLATE_CHUNK_SIZE: usize = 64 * 1024;

impl ByteStream {
  /// Creates a new empty byte stream.
  ///
  pub fn new() -> ByteStream {
    ByteStream {
      bytes_queue: VecDeque::new(),
      bytes_queue_size: 0,
      bytes_read: 0,
      is_writing_finished: false,
      zlib_stream: None,
      zlib_input_queue: VecDeque::new(),
      zlib_inflate_complete: false,
    }
  }

  /// Returns the total number of bytes that have been successfully read out of
  /// a byte stream.
  ///
  pub fn bytes_read(&self) -> u64 {
    self.bytes_read
  }

  /// Returns whether the byte stream is fully consumed, i.e. no bytes are
  /// unread and the end of the stream has been reached.
  ///
  pub fn is_fully_consumed(&self) -> bool {
    self.bytes_queue_size == 0
      && self.is_writing_finished
      && (self.zlib_stream.is_none() || self.zlib_inflate_complete)
  }

  /// Writes bytes to a byte stream so they are available to be read by
  /// subsequent calls to `read`. If `done` is true then this signals that no
  /// more bytes will be written to the byte stream, and any further calls to
  /// `write` will error.
  ///
  /// If the byte stream has zlib inflate enabled then the given bytes will be
  /// passed through zlib inflate and the output made available to be read.
  ///
  pub fn write(
    &mut self,
    data: RcByteSlice,
    done: bool,
  ) -> Result<(), ByteStreamError> {
    if self.is_writing_finished {
      return Err(ByteStreamError::WriteAfterCompletion);
    }

    self.is_writing_finished = done;

    if data.is_empty() {
      return Ok(());
    }

    // If zlib inflate is active then add the bytes to the zlib input queue
    if self.zlib_stream.is_some() {
      self.zlib_input_queue.push_back(data);
    } else {
      // Add the new bytes to the back of the output queue
      self.bytes_queue_size += data.len() as u64;
      self.bytes_queue.push_back(data);
    };

    Ok(())
  }

  /// Reads bytes out of a byte stream.
  ///
  pub fn read(
    &mut self,
    byte_count: usize,
  ) -> Result<RcByteSlice, ByteStreamError> {
    if byte_count == 0 {
      return Ok(RcByteSlice::empty());
    }

    self.inflate_up_to_read_size(byte_count)?;

    // Check there are sufficient bytes available to serve the read request
    if byte_count as u64 > self.bytes_queue_size {
      if self.is_writing_finished {
        return Err(ByteStreamError::DataEnd);
      } else {
        return Err(ByteStreamError::DataRequired);
      }
    }

    self.bytes_queue_size -= byte_count as u64;
    self.bytes_read += byte_count as u64;

    match byte_count.cmp(&self.bytes_queue.front().unwrap().len()) {
      // Return a byte slice inside the first queue item if possible
      core::cmp::Ordering::Less => {
        let result = self.bytes_queue.front().unwrap().take(byte_count);

        let queue_item = self.bytes_queue.front_mut().unwrap();
        *queue_item = queue_item.drop(byte_count);

        Ok(result)
      }

      core::cmp::Ordering::Equal => Ok(self.bytes_queue.pop_front().unwrap()),

      // The read request spans multiple queue items, so a new buffer has to be
      // allocated
      core::cmp::Ordering::Greater => {
        let mut result = Vec::with_capacity(byte_count);

        while result.len() < byte_count {
          let queue_item = self.bytes_queue.front_mut().unwrap();

          // Slice off the required amount and copy into the final result
          let end = core::cmp::min(queue_item.len(), byte_count - result.len());
          result.extend_from_slice(&queue_item[..end]);

          *queue_item = queue_item.drop(end);

          // If the chunk was fully consumed then remove it from the queue
          if queue_item.is_empty() {
            self.bytes_queue.pop_front();
          }
        }

        Ok(result.into())
      }
    }
  }

  /// Peeks at the next bytes that will be read out of a byte stream without
  /// actually consuming them.
  ///
  pub fn peek(
    &mut self,
    byte_count: usize,
  ) -> Result<Vec<u8>, ByteStreamError> {
    if byte_count == 0 {
      return Ok(vec![]);
    }

    self.inflate_up_to_read_size(byte_count)?;

    // Check there are sufficient bytes available to serve the peek request
    if byte_count as u64 > self.bytes_queue_size {
      if self.is_writing_finished {
        return Err(ByteStreamError::DataEnd);
      } else {
        return Err(ByteStreamError::DataRequired);
      }
    }

    let mut result = Vec::with_capacity(byte_count);

    for queue_item in self.bytes_queue.iter() {
      // Slice off the required amount and copy into the final result
      let end = core::cmp::min(queue_item.len(), byte_count - result.len());
      result.extend_from_slice(&queue_item[..end]);

      if result.len() >= byte_count {
        break;
      }
    }

    Ok(result)
  }

  /// Converts an uncompressed byte stream to a zlib deflated stream. All
  /// currently unread bytes, and all subsequently written bytes, will be passed
  /// through streaming zlib decompression and the result made available to be
  /// read out.
  ///
  /// This is used when reading DICOM P10 data that uses a deflated transfer
  /// syntax.
  ///
  pub fn start_zlib_inflate(&mut self) -> Result<(), ByteStreamError> {
    self.zlib_stream = Some(flate2::Decompress::new(false));
    self.zlib_input_queue.append(&mut self.bytes_queue);
    self.bytes_queue_size = 0;

    Ok(())
  }

  /// When zlib inflate is enabled, this function reads all pending inflated
  /// data from the zlib stream, up to the max read size limit. This ensures the
  /// stream is ready to service the next call to `read` or `peek`.
  ///
  /// Depending on what deflated data has been written, and the max read size of
  /// the stream, this function may leave data in the zlib stream. This is
  /// desirable in order to protect against zlib bombs, as it means the maximum
  /// memory consumption of a byte stream is capped at its max read size.
  ///
  fn inflate_up_to_read_size(
    &mut self,
    read_size: usize,
  ) -> Result<(), ByteStreamError> {
    let zlib_stream = match self.zlib_stream.as_mut() {
      Some(zlib_stream) => zlib_stream,
      None => return Ok(()),
    };

    while self.bytes_queue_size < read_size as u64 {
      let queue_item = match self.zlib_input_queue.pop_front() {
        Some(queue_item) => queue_item,
        None => return Ok(()),
      };

      let initial_total_in = zlib_stream.total_in();
      let initial_total_out = zlib_stream.total_out();

      let mut output_buffer = vec![0u8; ZLIB_INFLATE_CHUNK_SIZE];

      match zlib_stream.decompress(
        &queue_item,
        &mut output_buffer,
        flate2::FlushDecompress::None,
      ) {
        Ok(status) => {
          let bytes_consumed = zlib_stream.total_in() - initial_total_in;
          let bytes_produced = zlib_stream.total_out() - initial_total_out;

          // If not all the supplied input bytes were consumed, e.g. because
          // they result in more data than can be held in the output buffer,
          // then keep the remaining bytes for the next decompression call
          if bytes_consumed < queue_item.len() as u64 {
            self
              .zlib_input_queue
              .push_front(queue_item.drop(bytes_consumed as usize));
          }

          // Put any inflated bytes onto the bytes queue
          if bytes_produced > 0 {
            output_buffer.resize(bytes_produced as usize, 0);
            self.bytes_queue.push_back(output_buffer.into());
            self.bytes_queue_size += bytes_produced;
          }

          // Record when the zlib stream finishes decompressing all data.
          // Exhaustion of the zlib stream after the final deflated bytes have
          // been written is necessary for the byte stream being considered
          // fully consumed.
          if status == flate2::Status::StreamEnd {
            self.zlib_inflate_complete = true;
            return Ok(());
          }

          // If no bytes were produced then no more data can be inflated at this
          // stage
          if bytes_produced == 0 {
            break;
          }
        }

        Err(_) => return Err(ByteStreamError::ZlibDataError),
      }
    }

    Ok(())
  }
}
