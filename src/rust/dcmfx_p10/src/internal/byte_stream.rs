#[cfg(feature = "std")]
use std::collections::VecDeque;

#[cfg(not(feature = "std"))]
use alloc::{collections::VecDeque, vec, vec::Vec};

/// A byte stream that takes incoming chunks of binary data of any size and
/// allows the resulting data to to read and peeked as if it were one large
/// stream of bytes.
///
/// Incoming bytes can optionally be passed through zlib inflate prior to being
/// made available for reading.
///
#[derive(Debug)]
pub struct ByteStream {
  bytes_queue: VecDeque<QueueItem>,
  bytes_queue_size: u64,
  bytes_read: u64,
  is_writing_finished: bool,
  zlib_stream: Option<flate2::Decompress>,
  zlib_input_queue: VecDeque<QueueItem>,
  zlib_inflate_complete: bool,
}

#[derive(Debug)]
struct QueueItem {
  data: Vec<u8>,
  bytes_read: usize,
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
    data: Vec<u8>,
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
      self.zlib_input_queue.push_back(QueueItem {
        data,
        bytes_read: 0,
      });
    } else {
      // Add the new bytes to the back of the output queue
      self.bytes_queue_size += data.len() as u64;
      self.bytes_queue.push_back(QueueItem {
        data,
        bytes_read: 0,
      });
    };

    Ok(())
  }

  /// Reads bytes out of a byte stream.
  ///
  pub fn read(
    &mut self,
    byte_count: usize,
  ) -> Result<Vec<u8>, ByteStreamError> {
    if byte_count == 0 {
      return Ok(vec![]);
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

    let mut result = Vec::with_capacity(byte_count);

    while result.len() < byte_count {
      let queue_item = self.bytes_queue.front_mut().unwrap();

      // Slice off the required amount and copy into the final result
      let start = queue_item.bytes_read;
      let end = start
        + core::cmp::min(
          queue_item.data.len() - start,
          byte_count - result.len(),
        );
      result.extend_from_slice(&queue_item.data[start..end]);

      queue_item.bytes_read += end - start;

      // If only part of the chunk was consumed then push the remainder back
      // onto the front of the queue
      if queue_item.bytes_read == queue_item.data.len() {
        self.bytes_queue.pop_front();
      }
    }

    self.bytes_queue_size -= byte_count as u64;
    self.bytes_read += byte_count as u64;

    Ok(result)
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
      let start = queue_item.bytes_read;
      let end = start
        + core::cmp::min(
          queue_item.data.len() - start,
          byte_count - result.len(),
        );
      result.extend_from_slice(&queue_item.data[start..end]);

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
    // Store all current bytes so they can be re-written as zlib bytes
    let bytes_queue: Vec<QueueItem> = self.bytes_queue.drain(..).collect();
    let is_writing_finished = self.is_writing_finished;

    // Clear byte stream and update it to have an active zlib decompression
    // stream
    self.bytes_queue = VecDeque::new();
    self.bytes_queue_size = 0;
    self.is_writing_finished = false;
    self.zlib_stream = Some(flate2::Decompress::new(false));

    // Rewrite existing bytes to the stream so they'll be interpreted as
    // deflated data and inflated
    for queue_item in bytes_queue.into_iter() {
      self.write(
        queue_item.data[queue_item.bytes_read..].to_vec(),
        is_writing_finished,
      )?;
    }

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

      let input_slice = &queue_item.data[queue_item.bytes_read..];

      match zlib_stream.decompress(
        input_slice,
        output_buffer.as_mut_slice(),
        flate2::FlushDecompress::None,
      ) {
        Ok(status) => {
          let bytes_consumed = zlib_stream.total_in() - initial_total_in;
          let bytes_produced = zlib_stream.total_out() - initial_total_out;

          // If not all the supplied input bytes were consumed, e.g. because
          // they result in more data than can be held in the output buffer,
          // then keep the remaining bytes for the next decompression call
          if bytes_consumed < input_slice.len() as u64 {
            self.zlib_input_queue.push_front(QueueItem {
              data: queue_item.data,
              bytes_read: queue_item.bytes_read + bytes_consumed as usize,
            });
          }

          // Put any inflated bytes onto the bytes queue
          if bytes_produced > 0 {
            output_buffer.resize(bytes_produced as usize, 0);
            self.bytes_queue.push_back(QueueItem {
              data: output_buffer,
              bytes_read: 0,
            });
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
