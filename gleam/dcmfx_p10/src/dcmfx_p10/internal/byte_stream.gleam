import dcmfx_p10/internal/zlib.{type ZlibStream}
import dcmfx_p10/internal/zlib/inflate_result
import gleam/bit_array
import gleam/bool
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/queue.{type Queue}
import gleam/result

/// A byte stream that takes incoming chunks of binary data of any size and
/// allows the resulting data to to read and peeked as if it were one large
/// stream of bytes.
///
/// Incoming bytes can optionally be passed through zlib inflate prior to being
/// made available for reading.
///
pub opaque type ByteStream {
  ByteStream(
    bytes_queue: Queue(BitArray),
    bytes_queue_size: Int,
    bytes_read: Int,
    max_read_size: Int,
    is_writing_finished: Bool,
    zlib_stream: Option(ZlibStream),
  )
}

pub type ByteStreamError {
  /// The read or peek request was larger than the maximum allowed read size set
  /// on the byte stream.
  ReadOversized

  /// Data was not read because the byte stream does not have the number of
  /// bytes requested available and needs more bytes to be written to it first.
  DataRequired

  /// Data was not read because it would go past the end of the byte stream.
  DataEnd

  /// Data written to a byte stream that has zlib inflate active was not valid
  /// zlib data.
  ZlibDataError

  /// Data was written to a byte stream after its final bytes have already been
  /// written.
  WriteAfterCompletion
}

/// Creates a new empty byte stream with the given maximum read size.
///
/// The max read size is a limit on the maximum number of bytes that can be read
/// or peeked in the byte stream in a single call, which helps to protect
/// against zlib bombs.
///
pub fn new(max_read_size: Int) -> ByteStream {
  ByteStream(
    bytes_queue: queue.new(),
    bytes_queue_size: 0,
    bytes_read: 0,
    max_read_size: max_read_size,
    is_writing_finished: False,
    zlib_stream: None,
  )
}

/// Returns the total number of bytes that have been successfully read out of
/// a byte stream.
///
pub fn bytes_read(stream: ByteStream) -> Int {
  stream.bytes_read
}

/// Returns whether the byte stream is fully consumed, i.e. no bytes are unread
/// and the end of the stream has been reached.
///
pub fn is_fully_consumed(stream: ByteStream) -> Bool {
  stream.bytes_queue_size == 0
  && stream.is_writing_finished
  && stream.zlib_stream == None
}

/// Writes bytes to a byte stream so they are available to be read by subsequent
/// calls to `read`. If `done` is true then this signals that no more bytes
/// will be written to the byte stream, and any further calls to `write` will
/// error.
///
/// If the byte stream has zlib inflate enabled then the given bytes will be
/// passed through zlib inflate and the output made available to be read.
///
pub fn write(
  stream: ByteStream,
  data: BitArray,
  done: Bool,
) -> Result(ByteStream, ByteStreamError) {
  use <- bool.guard(stream.is_writing_finished, Error(WriteAfterCompletion))

  // If zlib inflate is active then add the bytes to the zlib stream and read
  // out the next chunk of inflated bytes if any are now available
  let new_bytes = case stream.zlib_stream {
    Some(zlib_stream) ->
      case zlib.safe_inflate(zlib_stream, data) {
        Ok(inflate_result.Continue(bytes)) | Ok(inflate_result.Finished(bytes)) ->
          Ok(bytes)
        Error(Nil) -> Error(ZlibDataError)
      }

    None -> Ok(data)
  }
  use new_bytes <- result.try(new_bytes)

  // Add the new bytes to the back of the queue
  let bytes_queue = case new_bytes {
    <<>> -> stream.bytes_queue
    _ -> queue.push_back(stream.bytes_queue, new_bytes)
  }

  // Increase the count of available bytes in the queue
  let bytes_queue_size =
    stream.bytes_queue_size + bit_array.byte_size(new_bytes)

  let stream =
    ByteStream(
      ..stream,
      bytes_queue: bytes_queue,
      bytes_queue_size: bytes_queue_size,
      is_writing_finished: done,
    )

  inflate_up_to_max_read_size(stream)
}

/// Reads bytes out of a byte stream. On success, returns the read bytes and an
/// updated byte stream.
///
pub fn read(
  stream: ByteStream,
  byte_count: Int,
) -> Result(#(BitArray, ByteStream), ByteStreamError) {
  use <- bool.guard(byte_count == 0, Ok(#(<<>>, stream)))
  use <- bool.guard(byte_count > stream.max_read_size, Error(ReadOversized))

  case byte_count <= stream.bytes_queue_size {
    // If there are sufficient bytes available to serve the read request then
    // pop them off the front of the queue
    True -> {
      let #(bytes, new_bytes_queue) =
        do_read(stream.bytes_queue, byte_count, [])

      let new_stream =
        ByteStream(
          ..stream,
          bytes_queue: new_bytes_queue,
          bytes_queue_size: stream.bytes_queue_size - byte_count,
          bytes_read: stream.bytes_read + byte_count,
        )

      use new_stream <- result.try(inflate_up_to_max_read_size(new_stream))

      Ok(#(bytes, new_stream))
    }

    // There aren't enough available bytes for this read request
    False ->
      case stream.is_writing_finished {
        True -> Error(DataEnd)
        False -> Error(DataRequired)
      }
  }
}

fn do_read(
  bytes_queue: Queue(BitArray),
  byte_count: Int,
  acc: List(BitArray),
) -> #(BitArray, Queue(BitArray)) {
  // Pop the next item off the front of the bytes queue
  let assert Ok(#(queue_item, bytes_queue)) = queue.pop_front(bytes_queue)
  let queue_item_size = bit_array.byte_size(queue_item)

  case byte_count <= queue_item_size {
    // This is the last chunk needed to satisfy the read request
    True -> {
      // Slice off the required amount and construct the final result by
      // concatenating all accumulated chunks
      let assert Ok(read_bytes) = bit_array.slice(queue_item, 0, byte_count)

      let final_bytes =
        [read_bytes, ..acc]
        |> list.reverse
        |> bit_array.concat

      // If only part of the chunk was consumed then push the remainder back
      // onto the front of the queue
      let unread_bytes_count = queue_item_size - byte_count
      let bytes_queue = case unread_bytes_count == 0 {
        True -> bytes_queue
        False -> {
          let assert Ok(unread_bytes) =
            bit_array.slice(queue_item, byte_count, unread_bytes_count)

          queue.push_front(bytes_queue, unread_bytes)
        }
      }

      #(final_bytes, bytes_queue)
    }

    // Further bytes are needed for the read request
    False ->
      do_read(bytes_queue, byte_count - queue_item_size, [queue_item, ..acc])
  }
}

/// Peeks at the next bytes that will be read out of a byte stream without
/// actually consuming them.
///
pub fn peek(
  stream: ByteStream,
  byte_count: Int,
) -> Result(BitArray, ByteStreamError) {
  use <- bool.guard(byte_count == 0, Ok(<<>>))
  use <- bool.guard(byte_count > stream.max_read_size, Error(ReadOversized))

  case byte_count <= stream.bytes_queue_size {
    True -> Ok(do_peek(stream.bytes_queue, byte_count, []))

    False ->
      case stream.is_writing_finished {
        True -> Error(DataEnd)
        False -> Error(DataRequired)
      }
  }
}

fn do_peek(
  bytes_queue: Queue(BitArray),
  byte_count: Int,
  acc: List(BitArray),
) -> BitArray {
  // Pop the next item off the front of the bytes queue
  let assert Ok(#(queue_item, bytes_queue)) = queue.pop_front(bytes_queue)
  let queue_item_size = bit_array.byte_size(queue_item)

  case byte_count <= queue_item_size {
    // This is the last chunk needed to satisfy the peek request
    True -> {
      // Slice off the required amount and construct the final result by
      // concatenating all accumulated chunks
      let assert Ok(bytes) = bit_array.slice(queue_item, 0, byte_count)

      [bytes, ..acc]
      |> list.reverse
      |> bit_array.concat
    }

    // Further items on the bytes queue are needed for the peek request
    False ->
      do_peek(bytes_queue, byte_count - queue_item_size, [queue_item, ..acc])
  }
}

/// Converts an uncompressed byte stream to a zlib deflated stream. All
/// currently unread bytes, and all subsequently written bytes, will be passed
/// through streaming zlib decompression and the result made available to be
/// read out.
///
/// This is used when reading DICOM P10 data that uses a deflated transfer
/// syntax.
///
pub fn start_zlib_inflate(
  stream: ByteStream,
) -> Result(ByteStream, ByteStreamError) {
  // Create new zlib stream and initialize with a negative window size to
  // indicate that the zlib header and trailing checksum aren't present
  let zlib_stream = zlib.open()
  let window_bits = -15
  zlib.inflate_init(zlib_stream, window_bits)

  // Store all current bytes so they can be re-written as zlib bytes
  let available_bytes =
    stream.bytes_queue
    |> queue.to_list
    |> bit_array.concat
  let is_writing_finished = stream.is_writing_finished

  // Create new byte stream with an active zlib stream for decompression
  let stream =
    ByteStream(
      ..stream,
      bytes_queue: queue.new(),
      bytes_queue_size: 0,
      is_writing_finished: False,
      zlib_stream: Some(zlib_stream),
    )

  // Rewrite existing bytes to the stream so they'll be interpreted as deflated
  // data and inflated
  write(stream, available_bytes, is_writing_finished)
}

/// When zlib inflate is enabled, this function reads all pending inflated data
/// from the zlib stream, up to the max read size limit. This ensures the stream
/// is ready to service the next call to `read` or `peek`.
///
/// Depending on what deflated data has been written, and the max read size of
/// the stream, this function may leave data in the zlib stream. This is
/// desirable in order to protect against zlib bombs, as it means the maximum
/// memory consumption of a byte stream is capped at its max read size.
///
fn inflate_up_to_max_read_size(
  stream: ByteStream,
) -> Result(ByteStream, ByteStreamError) {
  use <- bool.guard(stream.bytes_queue_size >= stream.max_read_size, Ok(stream))

  case stream.zlib_stream {
    Some(zlib_stream) ->
      case zlib.safe_inflate(zlib_stream, <<>>) {
        // Once the zlib stream finishes decompressing all data set it to None
        // as is has nothing left to do. Exhaustion of the zlib stream after the
        // final deflated bytes have been written is necessary for the byte
        // stream being considered fully consumed.
        Ok(inflate_result.Finished(<<>>)) if stream.is_writing_finished ->
          Ok(ByteStream(..stream, zlib_stream: None))

        // Put inflated bytes onto the bytes queue
        Ok(inflate_result.Continue(bytes)) | Ok(inflate_result.Finished(bytes)) ->
          case bytes {
            <<>> -> Ok(stream)
            bytes ->
              ByteStream(
                ..stream,
                bytes_queue: queue.push_back(stream.bytes_queue, bytes),
                bytes_queue_size: stream.bytes_queue_size
                  + bit_array.byte_size(bytes),
              )
              |> inflate_up_to_max_read_size
          }

        Error(Nil) -> Error(ZlibDataError)
      }

    None -> Ok(stream)
  }
}
