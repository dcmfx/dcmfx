//// Reads and writes the DICOM Part 10 (P10) binary format used to store and
//// transmit DICOM-based medical imaging information.

import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/data_set_path
import dcmfx_p10/data_set_builder.{type DataSetBuilder}
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_read.{type P10ReadContext}
import dcmfx_p10/p10_read_config.{type P10ReadConfig}
import dcmfx_p10/p10_token.{type P10Token}
import dcmfx_p10/p10_write.{type P10WriteContext}
import dcmfx_p10/p10_write_config.{type P10WriteConfig}
import dcmfx_p10/transforms/p10_filter_transform.{type P10FilterTransform}
import file_streams/file_stream.{type FileStream}
import file_streams/file_stream_error
import gleam/bit_array
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/order
import gleam/result

/// Returns whether a file contains DICOM P10 data by checking for the presence
/// of the 'DICM' prefix at offset 128.
///
pub fn is_valid_file(filename: String) -> Bool {
  filename
  |> file_stream.open_read
  |> result.map(fn(stream) {
    let bytes = file_stream.read_bytes_exact(stream, 132)
    let _ = file_stream.close(stream)

    case bytes {
      Ok(bytes) -> is_valid_bytes(bytes)
      _ -> False
    }
  })
  |> result.unwrap(False)
}

/// Returns whether the given bytes contain DICOM P10 data by checking for the
/// presence of the 'DICM' prefix at offset 128.
///
pub fn is_valid_bytes(bytes: BitArray) -> Bool {
  case bytes {
    <<_:bytes-128, "DICM", _:bytes>> -> True
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

/// Reads DICOM P10 data from a file read stream into an in-memory data set.
/// This will attempt to consume all data available in the read stream.
///
pub fn read_stream(
  stream: FileStream,
) -> Result(DataSet, #(P10Error, DataSetBuilder)) {
  let context = p10_read.new_read_context(None)
  let builder = data_set_builder.new()

  do_read_stream(stream, context, builder)
}

fn do_read_stream(
  stream: FileStream,
  context: P10ReadContext,
  builder: DataSetBuilder,
) -> Result(DataSet, #(P10Error, DataSetBuilder)) {
  // Read the next tokens from the stream
  let tokens_and_context =
    read_tokens_from_stream(stream, context, None)
    |> result.map_error(fn(e) { #(e, builder) })

  case tokens_and_context {
    Ok(#(tokens, context)) -> {
      // Add the new tokens to the data set builder
      let builder =
        tokens
        |> list.try_fold(builder, fn(builder, token) {
          data_set_builder.add_token(builder, token)
          |> result.map_error(fn(e) { #(e, builder) })
        })

      case builder {
        Ok(builder) ->
          // If the data set builder is now complete then return the final data
          // set
          case data_set_builder.final_data_set(builder) {
            Ok(final_data_set) -> Ok(final_data_set)
            Error(Nil) -> do_read_stream(stream, context, builder)
          }

        Error(e) -> Error(e)
      }
    }

    Error(e) -> Error(e)
  }
}

/// Reads the next DICOM P10 tokens from a read stream. This repeatedly reads
/// bytes from the read stream in 256 KiB chunks until at least one DICOM P10
/// token is made available by the read context or an error occurs.
///
pub fn read_tokens_from_stream(
  stream: FileStream,
  context: P10ReadContext,
  chunk_size: Option(Int),
) -> Result(#(List(P10Token), P10ReadContext), P10Error) {
  case p10_read.read_tokens(context) {
    Ok(#([], context)) -> read_tokens_from_stream(stream, context, chunk_size)

    Ok(#(tokens, context)) -> Ok(#(tokens, context))

    // If the read context needs more data then read bytes from the stream,
    // write them to the read context, and try again
    Error(p10_error.DataRequired(..)) ->
      case
        file_stream.read_bytes(stream, option.unwrap(chunk_size, 256 * 1024))
      {
        Ok(data) ->
          case p10_read.write_bytes(context, data, False) {
            Ok(context) -> read_tokens_from_stream(stream, context, chunk_size)
            Error(e) -> Error(e)
          }

        Error(file_stream_error.Eof) ->
          case p10_read.write_bytes(context, <<>>, True) {
            Ok(context) -> read_tokens_from_stream(stream, context, chunk_size)
            Error(e) -> Error(e)
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
    p10_read.new_read_context(None)
    |> p10_read.write_bytes(bytes, True)

  let builder = data_set_builder.new()

  do_read_bytes(context, builder)
}

fn do_read_bytes(
  context: P10ReadContext,
  builder: DataSetBuilder,
) -> Result(DataSet, #(P10Error, DataSetBuilder)) {
  // Read the next tokens from the read context
  case p10_read.read_tokens(context) {
    Ok(#(tokens, context)) -> {
      // Add the new token to the data set builder
      let new_builder =
        tokens
        |> list.try_fold(builder, fn(builder, token) {
          data_set_builder.add_token(builder, token)
        })

      case new_builder {
        // If the data set builder is now complete then return the final data
        // set
        Ok(builder) ->
          case data_set_builder.final_data_set(builder) {
            Ok(final_data_set) -> Ok(final_data_set)
            Error(Nil) -> do_read_bytes(context, builder)
          }

        Error(e) -> Error(#(e, builder))
      }
    }

    Error(e) -> Error(#(e, builder))
  }
}

/// Reads DICOM P10 data from a file into an in-memory data set. Only the
/// specified data elements at the root of the main data set are read, if
/// present. The file will only be read up to the point required to return the
/// requested data elements.
///
pub fn read_file_partial(
  filename: String,
  tags: List(DataElementTag),
  config: Option(P10ReadConfig),
) -> Result(DataSet, P10Error) {
  case file_stream.open_read(filename) {
    Ok(stream) -> read_stream_partial(stream, tags, config)
    Error(e) -> Error(p10_error.FileStreamError("Opening file", e))
  }
}

/// Reads DICOM P10 data from a stream into an in-memory data set. Only the
/// specified data elements at the root of the main data set are read, if
/// present. The stream will only be read up to the point required to return the
/// requested data elements.
///
pub fn read_stream_partial(
  stream: FileStream,
  tags: List(DataElementTag),
  config: Option(P10ReadConfig),
) -> Result(DataSet, P10Error) {
  let context = p10_read.new_read_context(config)

  // Find the largest data element tag being read
  let largest_tag =
    tags
    |> list.max(data_element_tag.compare)
    |> result.unwrap(data_element_tag.zero)

  // Create filter transform that only allows the specified root tags
  let filter =
    p10_filter_transform.new(fn(tag, _vr, _length, path) {
      !data_set_path.is_root(path) || list.contains(tags, tag)
    })

  use builder <- result.try(read_stream_partial_loop(
    stream,
    context,
    filter,
    data_set_builder.new(),
    largest_tag,
    Some(8 * 1024),
  ))

  let assert Ok(data_set) =
    builder |> data_set_builder.force_end |> data_set_builder.final_data_set

  // Exclude File Meta Information tags unless they were explicitly requested
  let data_set =
    data_set.filter(data_set, fn(tag, _value) { list.contains(tags, tag) })

  Ok(data_set)
}

fn read_stream_partial_loop(
  stream: FileStream,
  context: P10ReadContext,
  filter: P10FilterTransform,
  builder: DataSetBuilder,
  largest_tag: DataElementTag,
  chunk_size: Option(Int),
) -> Result(DataSetBuilder, P10Error) {
  case read_tokens_from_stream(stream, context, chunk_size) {
    Ok(#(tokens, context)) -> {
      let fold_result =
        list.fold_until(
          tokens,
          Ok(#(context, filter, builder, False)),
          fn(acc, token) {
            let assert Ok(#(context, filter, builder, _)) = acc

            case p10_filter_transform.add_token(filter, token) {
              Ok(#(filtered, filter)) -> {
                let builder = case filtered {
                  True -> data_set_builder.add_token(builder, token)
                  False -> Ok(builder)
                }

                case builder {
                  Ok(builder) -> {
                    case token {
                      p10_token.DataElementHeader(tag:, path:, ..)
                      | p10_token.SequenceStart(tag:, path:, ..) -> {
                        case
                          data_element_tag.compare(tag, largest_tag) == order.Gt
                          && data_set_path.is_root(path)
                        {
                          True ->
                            list.Stop(Ok(#(context, filter, builder, True)))
                          False ->
                            list.Continue(
                              Ok(#(context, filter, builder, False)),
                            )
                        }
                      }

                      p10_token.End ->
                        list.Stop(Ok(#(context, filter, builder, True)))

                      _ -> list.Continue(Ok(#(context, filter, builder, False)))
                    }
                  }

                  Error(e) -> list.Stop(Error(e))
                }
              }

              Error(e) -> list.Stop(Error(e))
            }
          },
        )

      case fold_result {
        Ok(#(context, filter, builder, done)) ->
          case done {
            True -> Ok(builder)
            False -> {
              read_stream_partial_loop(
                stream,
                context,
                filter,
                builder,
                largest_tag,
                None,
              )
            }
          }

        Error(e) -> Error(e)
      }
    }

    Error(e) -> Error(e)
  }
}

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

/// Writes a data set as DICOM P10 bytes directly to a file stream.
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

  p10_write.data_set_to_bytes(
    data_set,
    data_set_path.new(),
    Nil,
    bytes_callback,
    config,
  )
}

/// Writes a data set to in-memory DICOM P10 bytes.
///
pub fn write_bytes(
  data_set: DataSet,
  config: Option(P10WriteConfig),
) -> Result(BitArray, P10Error) {
  p10_write.data_set_to_bytes(
    data_set,
    data_set_path.new(),
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

/// Writes the specified DICOM P10 tokens to an output stream using the given
/// write context. Returns whether a `p10_token.End` token was present in the
/// tokens.
///
pub fn write_tokens_to_stream(
  tokens: List(P10Token),
  stream: FileStream,
  context: P10WriteContext,
) -> Result(#(Bool, P10WriteContext), P10Error) {
  use context <- result.try(
    list.try_fold(tokens, context, fn(context, token) {
      p10_write.write_token(context, token)
    }),
  )

  let #(p10_bytes, context) = p10_write.read_bytes(context)

  use _ <- result.try(
    list.try_fold(p10_bytes, Nil, fn(_, bytes) {
      file_stream.write_bytes(stream, bytes)
      |> result.map_error(fn(e) {
        p10_error.FileStreamError("Writing to stdout", e)
      })
    }),
  )

  case list.last(tokens) {
    Ok(p10_token.End) ->
      file_stream.sync(stream)
      |> result.map_error(fn(e) {
        p10_error.FileStreamError("Writing to stdout", e)
      })
      |> result.replace(#(True, context))

    _ -> Ok(#(False, context))
  }
}
