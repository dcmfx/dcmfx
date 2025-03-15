import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_element_value.{type DataElementValue}
import dcmfx_core/data_set.{type DataSet}
import dcmfx_p10/p10_token.{type P10Token}
import dcmfx_p10/transforms/p10_filter_transform.{type P10FilterTransform}
import gleam/bool
import gleam/list

/// Transform that inserts data elements into a stream of DICOM P10 tokens.
///
pub opaque type P10InsertTransform {
  P10InsertTransform(
    data_elements_to_insert: List(#(DataElementTag, DataElementValue)),
    filter_transform: P10FilterTransform,
  )
}

/// Creates a new context for inserting data elements into the root data set
/// of a stream of DICOM P10 tokens.
///
pub fn new(data_elements_to_insert: DataSet) -> P10InsertTransform {
  let tags_to_insert = data_set.tags(data_elements_to_insert)

  // Create a filter transform that filters out the data elements that are going
  // to be inserted. This ensures there are no duplicate data elements in the
  // resulting token stream.
  let filter_transform =
    p10_filter_transform.new(fn(tag, _vr, _length, location) {
      location != [] || !list.contains(tags_to_insert, tag)
    })

  P10InsertTransform(
    data_elements_to_insert: data_set.to_list(data_elements_to_insert),
    filter_transform:,
  )
}

/// Adds the next available token to a P10 insert transform and returns the
/// resulting tokens.
///
pub fn add_token(
  context: P10InsertTransform,
  token: P10Token,
) -> #(List(P10Token), P10InsertTransform) {
  // If there are no more data elements to be inserted then pass the token
  // straight through
  use <- bool.guard(context.data_elements_to_insert == [], #([token], context))

  let is_at_root = p10_filter_transform.is_at_root(context.filter_transform)

  // Pass the token through the filter transform
  let #(filter_result, filter_transform) =
    p10_filter_transform.add_token(context.filter_transform, token)

  let context = P10InsertTransform(..context, filter_transform:)

  use <- bool.guard(!filter_result, #([], context))

  // Data element insertion is only supported in the root data set, so if the
  // stream is not at the root data set then there's nothing to do
  use <- bool.guard(!is_at_root, #([token], context))

  case token {
    // If this token is the start of a new data element, and there are data
    // elements still to be inserted, then insert any that should appear prior
    // to this next data element
    p10_token.SequenceStart(tag, ..) | p10_token.DataElementHeader(tag, ..) -> {
      let #(tokens_to_insert, data_elements_to_insert) =
        tokens_to_insert_before_tag(tag, context.data_elements_to_insert, [])

      let context = P10InsertTransform(..context, data_elements_to_insert:)
      let tokens = [token, ..tokens_to_insert] |> list.reverse

      #(tokens, context)
    }

    // If this token is the end of the P10 tokens and there are still data
    // elements to be inserted then insert them now prior to the end
    p10_token.End -> {
      let tokens =
        context.data_elements_to_insert
        |> list.fold([], fn(acc, data_element) {
          prepend_data_element_tokens(data_element, acc)
        })

      let context = P10InsertTransform(..context, data_elements_to_insert: [])
      let tokens = [p10_token.End, ..tokens] |> list.reverse

      #(tokens, context)
    }

    _ -> #([token], context)
  }
}

/// Removes all data elements to insert off the list that have a tag value lower
/// than the specified tag, converts them to P10 tokens, and prepends the tokens
/// to the accumulator
///
fn tokens_to_insert_before_tag(
  tag: DataElementTag,
  data_elements_to_insert: List(#(DataElementTag, DataElementValue)),
  acc: List(P10Token),
) -> #(List(P10Token), List(#(DataElementTag, DataElementValue))) {
  case data_elements_to_insert {
    [data_element, ..rest] ->
      case
        data_element_tag.to_int(data_element.0) < data_element_tag.to_int(tag)
      {
        True ->
          data_element
          |> prepend_data_element_tokens(acc)
          |> tokens_to_insert_before_tag(tag, rest, _)

        False -> #(acc, data_elements_to_insert)
      }

    _ -> #(acc, data_elements_to_insert)
  }
}

fn prepend_data_element_tokens(
  data_element: #(DataElementTag, DataElementValue),
  acc: List(P10Token),
) -> List(P10Token) {
  let #(tag, value) = data_element

  // This assert is safe because the function that gathers the tokens for the
  // data set never errors
  let assert Ok(tokens) =
    p10_token.data_element_to_tokens(tag, value, acc, fn(acc, token) {
      Ok([token, ..acc])
    })

  tokens
}
