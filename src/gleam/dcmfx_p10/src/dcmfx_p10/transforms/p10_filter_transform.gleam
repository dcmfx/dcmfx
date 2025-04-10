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
    path: DataSetPath,
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
/// The predicate function is called as tokens are added to the context, and
/// only those data elements that return `True` from the predicate function
/// will pass through the filter.
///
pub fn new(predicate: PredicateFunction) -> P10FilterTransform {
  P10FilterTransform(
    predicate: predicate,
    path: data_set_path.new(),
    path_filter_results: [],
  )
}

/// Returns whether the current position of the P10 filter context is the root
/// data set, i.e. there are no nested sequences currently active.
///
pub fn is_at_root(context: P10FilterTransform) -> Bool {
  data_set_path.is_empty(context.path)
}

/// Adds the next token to the P10 filter transform and returns whether it
/// should be included in the filtered token stream.
///
pub fn add_token(
  context: P10FilterTransform,
  token: P10Token,
) -> Result(#(Bool, P10FilterTransform), P10Error) {
  let current_filter_state = case context.path_filter_results {
    [filter_result, ..] -> filter_result
    _ -> True
  }

  let map_data_set_path_error = fn(details: String) -> P10Error {
    p10_error.TokenStreamInvalid(
      when: "Filtering P10 token stream",
      details:,
      token:,
    )
  }

  let run_predicate = fn(
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: Option(Int),
  ) {
    let filter_result = case context.path_filter_results {
      [] | [True, ..] -> context.predicate(tag, vr, length, context.path)

      // The predicate function is skipped if a parent has already been filtered
      // out
      _ -> False
    }

    let path =
      context.path
      |> data_set_path.add_data_element(tag)
      |> result.map_error(map_data_set_path_error)
    use path <- result.map(path)

    let path_filter_results = [filter_result, ..context.path_filter_results]

    let new_context = P10FilterTransform(..context, path:, path_filter_results:)

    #(filter_result, new_context)
  }

  case token {
    // If this is a new sequence or data element then run the predicate function
    // to see if it passes the filter, then add it to the location
    p10_token.SequenceStart(tag, vr) -> run_predicate(tag, vr, None)
    p10_token.DataElementHeader(tag, vr, length) ->
      run_predicate(tag, vr, Some(length))

    p10_token.SequenceItemStart(index) -> {
      let path =
        context.path
        |> data_set_path.add_sequence_item(index)
        |> result.map_error(map_data_set_path_error)
      use path <- result.map(path)

      let new_context = P10FilterTransform(..context, path:)

      #(current_filter_state, new_context)
    }

    p10_token.SequenceItemDelimiter -> {
      let path =
        context.path
        |> data_set_path.pop
        |> result.map_error(map_data_set_path_error)
      use path <- result.map(path)

      let new_context = P10FilterTransform(..context, path:)

      #(current_filter_state, new_context)
    }

    // If this is a new pixel data item then add it to the location
    p10_token.PixelDataItem(index:, ..) -> {
      let path =
        context.path
        |> data_set_path.add_sequence_item(index)
        |> result.map_error(map_data_set_path_error)
      use path <- result.map(path)

      let path_filter_results = [
        current_filter_state,
        ..context.path_filter_results
      ]

      let new_context =
        P10FilterTransform(..context, path:, path_filter_results:)

      #(current_filter_state, new_context)
    }

    // Detect the end of the entry at the head of the location and pop it off
    p10_token.SequenceDelimiter(..)
    | p10_token.DataElementValueBytes(bytes_remaining: 0, ..) -> {
      let path =
        context.path
        |> data_set_path.pop
        |> result.map_error(map_data_set_path_error)
      use path <- result.map(path)

      let assert Ok(path_filter_results) =
        list.rest(context.path_filter_results)

      let new_context =
        P10FilterTransform(..context, path:, path_filter_results:)

      #(current_filter_state, new_context)
    }

    _ -> Ok(#(current_filter_state, context))
  }
}
