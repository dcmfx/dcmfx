import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_set_path.{type DataSetPath}
import dcmfx_core/value_representation.{type ValueRepresentation}
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_token.{type P10Token}
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result

/// Transform that applies a data element filter to a stream of DICOM P10
/// tokens. Incoming data elements are passed to a predicate function that
/// determines whether they should be present in the output DICOM P10 token
/// stream.
///
pub opaque type P10FilterTransform {
  P10FilterTransform(
    predicate: PredicateFunction,
    path_filter_results: List(Bool),
  )
}

/// Defines a function called by a `P10FilterTransform` that determines whether
/// a data element should pass through the filter.
///
pub type PredicateFunction =
  fn(DataElementTag, ValueRepresentation, Option(Int), DataSetPath) -> Bool

/// Creates a new filter transform for filtering a stream of DICOM P10 tokens.
///
/// The predicate function is called as tokens are added to the transform, and
/// only those data elements that return `True` from the predicate function
/// will pass through the filter.
///
pub fn new(predicate: PredicateFunction) -> P10FilterTransform {
  P10FilterTransform(predicate: predicate, path_filter_results: [])
}

/// Returns whether the current position of the P10 filter transform is the root
/// data set, i.e. there are no nested sequences currently active.
///
pub fn is_at_root(transform: P10FilterTransform) -> Bool {
  list.length(transform.path_filter_results) <= 1
}

/// Adds the next token to the P10 filter transform and returns whether it
/// should be included in the filtered token stream.
///
pub fn add_token(
  transform: P10FilterTransform,
  token: P10Token,
) -> Result(#(Bool, P10FilterTransform), P10Error) {
  let current_filter_state = case transform.path_filter_results {
    [filter_result, ..] -> filter_result
    _ -> True
  }

  let run_predicate = fn(
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: Option(Int),
    path: DataSetPath,
  ) {
    let filter_result = case transform.path_filter_results {
      [] | [True, ..] -> transform.predicate(tag, vr, length, path)

      // The predicate function is skipped if a parent has already been filtered
      // out
      _ -> False
    }

    let path_filter_results = [filter_result, ..transform.path_filter_results]

    let new_transform = P10FilterTransform(..transform, path_filter_results:)

    Ok(#(filter_result, new_transform))
  }

  case token {
    p10_token.FilePreambleAndDICMPrefix(..) | p10_token.FileMetaInformation(..) ->
      Ok(#(True, transform))

    p10_token.SequenceStart(tag, vr, path) -> run_predicate(tag, vr, None, path)

    p10_token.SequenceDelimiter(..) -> {
      let path_filter_results =
        list.rest(transform.path_filter_results)
        |> result.map_error(fn(_) {
          p10_error.TokenStreamInvalid(
            "Adding token to filter transform",
            "Sequence delimiter received when current path is empty",
            token,
          )
        })
      use path_filter_results <- result.map(path_filter_results)

      let new_transform = P10FilterTransform(..transform, path_filter_results:)

      #(current_filter_state, new_transform)
    }

    p10_token.SequenceItemStart(..) -> {
      let path_filter_results = [
        current_filter_state,
        ..transform.path_filter_results
      ]

      let new_transform = P10FilterTransform(..transform, path_filter_results:)

      Ok(#(current_filter_state, new_transform))
    }

    p10_token.SequenceItemDelimiter -> {
      let path_filter_results =
        list.rest(transform.path_filter_results)
        |> result.map_error(fn(_) {
          p10_error.TokenStreamInvalid(
            "Adding token to filter transform",
            "Sequence item delimiter received when current path is empty",
            token,
          )
        })
      use path_filter_results <- result.map(path_filter_results)

      let new_transform = P10FilterTransform(..transform, path_filter_results:)

      #(current_filter_state, new_transform)
    }

    p10_token.DataElementHeader(tag, vr, length, path) ->
      run_predicate(tag, vr, Some(length), path)

    p10_token.DataElementValueBytes(bytes_remaining:, ..) -> {
      let path_filter_results = case bytes_remaining {
        0 -> {
          list.rest(transform.path_filter_results)
          |> result.map_error(fn(_) {
            p10_error.TokenStreamInvalid(
              "Adding token to filter transform",
              "Data element bytes ended when current path is empty",
              token,
            )
          })
        }
        _ -> Ok(transform.path_filter_results)
      }
      use path_filter_results <- result.map(path_filter_results)

      let new_transform = P10FilterTransform(..transform, path_filter_results:)

      #(current_filter_state, new_transform)
    }

    p10_token.PixelDataItem(..) -> {
      let path_filter_results = [
        current_filter_state,
        ..transform.path_filter_results
      ]

      let new_transform = P10FilterTransform(..transform, path_filter_results:)

      Ok(#(current_filter_state, new_transform))
    }

    p10_token.End -> Ok(#(True, transform))
  }
}
