//// Reads and writes the DICOM Part 10 (P10) binary format used to store and
//// transmit DICOM-based medical imaging information.

import dcmfx_core/data_set.{type DataSet}
import dcmfx_p10/data_set_builder.{type DataSetBuilder}
import dcmfx_p10/p10_error.{type P10Error}
@target(erlang)
import dcmfx_p10/p10_part.{type P10Part}
import dcmfx_p10/p10_read.{type P10ReadContext}
import dcmfx_p10/p10_write.{type P10WriteConfig}
@target(erlang)
import file_streams/file_stream.{type FileStream}
@target(erlang)
import file_streams/file_stream_error
import gleam/bit_array
import gleam/list
import gleam/option.{type Option}
import gleam/result
@target(javascript)
import simplifile

@target(erlang)
/// Returns whether a file contains DICOM P10 data by checking for the presence
/// of the DICOM P10 header and the start of a File Meta Information Group
/// Length data element.
///
pub fn is_valid_file(filename: String) -> Bool {
  filename
  |> file_stream.open_read
  |> result.map(fn(stream) {
    let bytes = file_stream.read_bytes_exact(stream, 138)
    let _ = file_stream.close(stream)

    case bytes {
      Ok(bytes) -> is_valid_bytes(bytes)
      _ -> False
    }
  })
  |> result.unwrap(False)
}

@target(javascript)
pub fn is_valid_file(filename: String) -> Bool {
  case simplifile.read_bits(filename) {
    Ok(bytes) -> is_valid_bytes(bytes)
    Error(_) -> False
  }
}

/// Returns whether `bytes` contains DICOM P10 data by checking for the presence
/// of the DICOM P10 header and the start of a File Meta Information Group
/// Length data element.
///
pub fn is_valid_bytes(bytes: BitArray) -> Bool {
  case bytes {
    <<_:bytes-128, "DICM", 2:16-little, 0:16-little, "UL", _:bytes>> -> True
    _ -> False
  }
}

/// Reads DICOM P10 data from a file into an in-memory data set.
///
pub fn read_file(filename: String) -> Result(DataSet, P10Error) {
  filename
  |> read_file_returning_builder_on_error
  |> result.map_error(fn(e) { e.0 })
}

@target(erlang)
/// Reads DICOM P10 data from a file into an in-memory data set. In the case of
/// an error occurring during the read both the error and the data set builder
/// at the time of the error are returned.
///
/// This allows for the data that was successfully read prior to the error to be
/// converted into a partially-complete data set.
///
pub fn read_file_returning_builder_on_error(
  filename: String,
) -> Result(DataSet, #(P10Error, DataSetBuilder)) {
  filename
  |> file_stream.open_read
  |> result.map_error(fn(e) {
    #(p10_error.FileStreamError("Opening file", e), data_set_builder.new())
  })
  |> result.try(read_stream)
}

@target(javascript)
pub fn read_file_returning_builder_on_error(
  filename: String,
) -> Result(DataSet, #(P10Error, DataSetBuilder)) {
  case simplifile.read_bits(filename) {
    Ok(bytes) -> read_bytes(bytes)
    Error(e) ->
      Error(#(p10_error.FileError("Reading file", e), data_set_builder.new()))
  }
}

@target(erlang)
/// Reads DICOM P10 data from a file read stream into an in-memory data set.
/// This will attempt to consume all data available in the read stream.
///
/// This function is not supported on the JavaScript target.
///
pub fn read_stream(
  stream: FileStream,
) -> Result(DataSet, #(P10Error, DataSetBuilder)) {
  let context = p10_read.new_read_context()
  let builder = data_set_builder.new()

  do_read_stream(stream, context, builder)
}

@target(erlang)
fn do_read_stream(
  stream: FileStream,
  context: P10ReadContext,
  builder: DataSetBuilder,
) -> Result(DataSet, #(P10Error, DataSetBuilder)) {
  // Read the next parts from the stream
  let parts_and_context =
    read_parts_from_stream(stream, context)
    |> result.map_error(fn(e) { #(e, builder) })
  use #(parts, context) <- result.try(parts_and_context)

  // Add the new parts to the data set builder
  let builder =
    parts
    |> list.try_fold(builder, fn(builder, part) {
      data_set_builder.add_part(builder, part)
      |> result.map_error(fn(e) { #(e, builder) })
    })
  use builder <- result.try(builder)

  // If the data set builder is now complete then return the final data set
  case data_set_builder.is_complete(builder) {
    True -> {
      let assert Ok(data_set) = data_set_builder.final_data_set(builder)
      Ok(data_set)
    }
    False -> do_read_stream(stream, context, builder)
  }
}

@target(erlang)
/// Reads the next DICOM P10 parts from a read stream. This repeatedly reads
/// bytes from the read stream in 256 KiB chunks until at least one DICOM P10
/// part is made available by the read context or an error occurs.
///
/// This function is not available on the JavaScript target.
///
pub fn read_parts_from_stream(
  stream: FileStream,
  context: P10ReadContext,
) -> Result(#(List(P10Part), P10ReadContext), P10Error) {
  case p10_read.read_parts(context) {
    Ok(#([], context)) -> read_parts_from_stream(stream, context)

    Ok(#(parts, context)) -> Ok(#(parts, context))

    // If the read context needs more data then read bytes from the stream,
    // write them to the read context, and try again
    Error(p10_error.DataRequired(..)) ->
      case file_stream.read_bytes(stream, 256 * 1024) {
        Ok(data) -> {
          use context <- result.try(p10_read.write_bytes(context, data, False))
          read_parts_from_stream(stream, context)
        }

        Error(file_stream_error.Eof) -> {
          use context <- result.try(p10_read.write_bytes(context, <<>>, True))
          read_parts_from_stream(stream, context)
        }

        Error(e) ->
          Error(p10_error.FileStreamError("Reading from file stream", e))
      }

    Error(e) -> Error(e)
  }
}

/// Reads DICOM P10 data from a `BitArray` into an in-memory data set.
///
pub fn read_bytes(
  bytes: BitArray,
) -> Result(DataSet, #(P10Error, DataSetBuilder)) {
  let assert Ok(context) =
    p10_read.new_read_context()
    |> p10_read.write_bytes(bytes, True)

  let builder = data_set_builder.new()

  do_read_bytes(context, builder)
}

fn do_read_bytes(
  context: P10ReadContext,
  builder: DataSetBuilder,
) -> Result(DataSet, #(P10Error, DataSetBuilder)) {
  // Read the next parts from the read context
  case p10_read.read_parts(context) {
    Ok(#(parts, context)) -> {
      // Add the new part to the data set builder
      let new_builder =
        parts
        |> list.try_fold(builder, fn(builder, part) {
          data_set_builder.add_part(builder, part)
        })

      case new_builder {
        // If the data set builder is now complete then return the final data
        // set
        Ok(builder) ->
          case data_set_builder.is_complete(builder) {
            True -> {
              let assert Ok(data_set) = data_set_builder.final_data_set(builder)
              Ok(data_set)
            }
            False -> do_read_bytes(context, builder)
          }

        Error(e) -> Error(#(e, builder))
      }
    }

    Error(e) -> Error(#(e, builder))
  }
}

@target(erlang)
/// Writes a data set to a DICOM P10 file. This will overwrite any existing file
/// with the given name.
///
pub fn write_file(
  filename: String,
  data_set: DataSet,
  config: Option(P10WriteConfig),
) -> Result(Nil, P10Error) {
  let stream =
    filename
    |> file_stream.open_write
    |> result.map_error(fn(e) {
      p10_error.FileStreamError("Creating write stream", e)
    })
  use stream <- result.try(stream)

  let write_result = write_stream(stream, data_set, config)

  let _ = file_stream.close(stream)

  write_result
}

@target(javascript)
pub fn write_file(
  filename: String,
  data_set: DataSet,
  config: Option(P10WriteConfig),
) -> Result(Nil, P10Error) {
  let initial_write_result = case simplifile.write_bits(filename, <<>>) {
    Ok(Nil) -> Ok(Nil)
    Error(e) -> Error(p10_error.FileError("Writing file", e))
  }
  use _ <- result.try(initial_write_result)

  let bytes_callback = fn(_, p10_bytes) {
    case simplifile.append_bits(filename, p10_bytes) {
      Ok(Nil) -> Ok(Nil)
      Error(e) -> Error(p10_error.FileError("Writing file", e))
    }
  }

  let config = option.lazy_unwrap(config, p10_write.default_config)

  p10_write.data_set_to_bytes(data_set, Nil, bytes_callback, config)
}

@target(erlang)
/// Writes a data set as DICOM P10 bytes directly to a file stream.
///
/// This function is not available on the JavaScript target.
///
pub fn write_stream(
  stream: FileStream,
  data_set: DataSet,
  config: Option(P10WriteConfig),
) -> Result(Nil, P10Error) {
  let bytes_callback = fn(_, p10_bytes) {
    stream
    |> file_stream.write_bytes(p10_bytes)
    |> result.map_error(fn(e) {
      p10_error.FileStreamError("Writing DICOM P10 data to stream", e)
    })
  }

  let config = option.lazy_unwrap(config, p10_write.default_config)

  p10_write.data_set_to_bytes(data_set, Nil, bytes_callback, config)
}

/// Writes a data set to in-memory DICOM P10 bytes.
///
pub fn write_bytes(
  data_set: DataSet,
  config: Option(P10WriteConfig),
) -> Result(BitArray, P10Error) {
  let config = option.lazy_unwrap(config, p10_write.default_config)

  p10_write.data_set_to_bytes(
    data_set,
    [],
    fn(chunks, bytes) { Ok([bytes, ..chunks]) },
    config,
  )
  |> result.map(fn(chunks) {
    chunks
    |> list.reverse
    |> bit_array.concat
  })
}
