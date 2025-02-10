import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_error
import dcmfx_core/data_set.{type DataSet}
import dcmfx_p10/data_set_builder.{type DataSetBuilder}
import dcmfx_p10/p10_error
import dcmfx_p10/p10_token.{type P10Token}
import dcmfx_p10/transforms/p10_filter_transform.{type P10FilterTransform}
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/order
import gleam/result

/// Transforms a stream of DICOM P10 tokens into a custom type. This is done by:
/// 
/// 1. Specifying the tags of the data elements needed to create the custom
///    type.
/// 2. Extracting the specified data elements from the incoming DICOM P10 token
///    stream into a data set.
/// 3. Passing the data set to a function that creates the custom type.
/// 
/// The result is then accessed using `get_output()` which returns `None` if the
/// target is not yet available or was unable to be created.
///
pub opaque type P10CustomTypeTransform(a) {
  P10CustomTypeTransform(
    filter: Option(#(P10FilterTransform, DataSetBuilder)),
    last_tag: DataElementTag,
    target_from_data_set: fn(DataSet) -> Result(a, data_error.DataError),
    target: Option(a),
  )
}

/// An error that occurred in the process of converting a stream DICOM P10
/// tokens to a custom type.
///
pub type P10CustomTypeTransformError {
  /// An error that occurred when adding a P10 token to the data set builder.
  /// This can happen when the stream of DICOM P10 tokens is invalid.
  P10Error(p10_error.P10Error)

  /// An error that occurred when creating the custom type from the gathered
  /// data set.
  DataError(data_error.DataError)
}

/// Creates a new transform for converting a stream of DICOM P10 tokens to
/// a custom type.
///
pub fn new(
  tags: List(DataElementTag),
  target_from_data_set: fn(DataSet) -> Result(a, data_error.DataError),
) -> P10CustomTypeTransform(a) {
  let filter =
    p10_filter_transform.new(fn(tag, _vr, _location) {
      list.contains(tags, tag)
    })

  let last_tag =
    tags
    |> list.max(data_element_tag.compare)
    |> result.unwrap(data_element_tag.zero)

  P10CustomTypeTransform(
    filter: Some(#(filter, data_set_builder.new())),
    last_tag:,
    target_from_data_set:,
    target: None,
  )
}

/// Adds the next token in the DICOM P10 token stream.
///
pub fn add_token(
  transform: P10CustomTypeTransform(a),
  token: P10Token,
) -> Result(P10CustomTypeTransform(a), P10CustomTypeTransformError) {
  case transform.filter {
    Some(#(filter, builder)) -> {
      let is_at_root = p10_filter_transform.is_at_root(filter)

      use #(filter, builder) <- result.try(case
        p10_filter_transform.add_token(filter, token)
      {
        #(True, filter) -> {
          let builder =
            builder
            |> data_set_builder.add_token(token)
            |> result.map_error(P10Error)
          use builder <- result.map(builder)

          #(filter, builder)
        }
        #(False, filter) -> Ok(#(filter, builder))
      })

      // Check whether all the relevant tags have now been read. If they have
      // then the final type can be constructed.
      let is_complete =
        is_at_root
        && case token {
          p10_token.DataElementHeader(tag:, ..)
          | p10_token.SequenceStart(tag:, ..) ->
            data_element_tag.compare(tag, transform.last_tag) == order.Gt

          p10_token.DataElementValueBytes(tag:, bytes_remaining: 0, ..)
          | p10_token.SequenceDelimiter(tag:) -> tag == transform.last_tag

          p10_token.End -> True

          _ -> False
        }

      case is_complete {
        True -> {
          let assert Ok(data_set) =
            builder
            |> data_set_builder.force_end
            |> data_set_builder.final_data_set

          let target =
            transform.target_from_data_set(data_set)
            |> result.map_error(DataError)
          use target <- result.try(target)

          P10CustomTypeTransform(
            ..transform,
            filter: None,
            target: Some(target),
          )
          |> Ok
        }

        False ->
          P10CustomTypeTransform(..transform, filter: Some(#(filter, builder)))
          |> Ok
      }
    }

    _ -> Ok(transform)
  }
}

/// Returns the custom type created by this transform. This is set once all the
/// required data elements have been gathered from the stream of DICOM P10
/// tokens and successfully constructed into the custom type.
///
pub fn get_output(transform: P10CustomTypeTransform(a)) -> Option(a) {
  transform.target
}
