import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_element_value.{type DataElementValue}
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/data_set_path.{type DataSetPath}
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_token.{type P10Token}
import dcmfx_p10/transforms/p10_filter_transform.{type P10FilterTransform}
import gleam/bool
import gleam/list
import gleam/result

/// Transform that inserts data elements into a stream of DICOM P10 tokens.
///
pub opaque type P10InsertTransform {
  P10InsertTransform(
    data_elements_to_insert: List(#(DataElementTag, DataElementValue)),
    filter_transform: P10FilterTransform,
  )
}

/// Creates a new transform for inserting data elements into the root data set
/// of a stream of DICOM P10 tokens.
///
pub fn new(data_elements_to_insert: DataSet) -> P10InsertTransform {
  let tags_to_insert = data_set.tags(data_elements_to_insert)

  // Create a filter transform that filters out the data elements that are going
  // to be inserted. This ensures there are no duplicate data elements in the
  // resulting token stream.
  let filter_transform =
    p10_filter_transform.new(fn(tag, _vr, _length, path) {
      !data_set_path.is_root(path) || !list.contains(tags_to_insert, tag)
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
  transform: P10InsertTransform,
  token: P10Token,
) -> Result(#(List(P10Token), P10InsertTransform), P10Error) {
  // If there are no more data elements to be inserted then pass the token
  // straight through
  use <- bool.guard(
    transform.data_elements_to_insert == [],
    Ok(#([token], transform)),
  )

  let is_at_root = p10_filter_transform.is_at_root(transform.filter_transform)

  // Pass the token through the filter transform
  let add_token_result =
    p10_filter_transform.add_token(transform.filter_transform, token)
  use #(filter_result, filter_transform) <- result.try(add_token_result)

  let transform = P10InsertTransform(..transform, filter_transform:)

  use <- bool.guard(!filter_result, Ok(#([], transform)))

  // Data element insertion is only supported in the root data set, so if the
  // stream is not at the root data set then there's nothing to do
  use <- bool.guard(!is_at_root, Ok(#([token], transform)))

  case token {
    // If this token is the start of a new data element, and there are data
    // elements still to be inserted, then insert any that should appear prior
    // to this next data element
    p10_token.SequenceStart(tag:, path:, ..)
    | p10_token.DataElementHeader(tag:, path:, ..) -> {
      use #(tokens_to_insert, data_elements_to_insert) <- result.map(
        tokens_to_insert_before_tag(
          tag,
          path,
          transform.data_elements_to_insert,
          token,
          [],
        ),
      )

      let transform = P10InsertTransform(..transform, data_elements_to_insert:)
      let tokens = [token, ..tokens_to_insert] |> list.reverse

      #(tokens, transform)
    }

    // If this token is the end of the P10 tokens and there are still data
    // elements to be inserted then insert them now prior to the end
    p10_token.End -> {
      let #(tokens, transform) = flush(transform)
      let tokens = [p10_token.End, ..tokens] |> list.reverse

      Ok(#(tokens, transform))
    }

    _ -> Ok(#([token], transform))
  }
}

/// If there are any remaining data elements for this transform to insert,
/// returns their P10 tokens.
///
/// These tokens are returned automatically when an end token is received, but
/// in some circumstances may need to be requested manually.
///
pub fn flush(
  transform: P10InsertTransform,
) -> #(List(P10Token), P10InsertTransform) {
  let tokens =
    transform.data_elements_to_insert
    |> list.fold([], fn(acc, data_element) {
      prepend_data_element_tokens(
        data_element,
        data_set_path.new_with_data_element(data_element.0),
        acc,
      )
    })

  #(tokens, P10InsertTransform(..transform, data_elements_to_insert: []))
}

/// Removes all data elements to insert off the list that have a tag value lower
/// than the specified tag, converts them to P10 tokens, and prepends the tokens
/// to the accumulator
///
fn tokens_to_insert_before_tag(
  tag: DataElementTag,
  path: DataSetPath,
  data_elements_to_insert: List(#(DataElementTag, DataElementValue)),
  token: P10Token,
  acc: List(P10Token),
) -> Result(
  #(List(P10Token), List(#(DataElementTag, DataElementValue))),
  P10Error,
) {
  case data_elements_to_insert {
    [data_element, ..rest] ->
      case
        data_element_tag.to_int(data_element.0) < data_element_tag.to_int(tag)
      {
        True -> {
          let path =
            path
            |> data_set_path.pop
            |> result.try(data_set_path.add_data_element(_, data_element.0))
            |> result.map_error(fn(_) {
              p10_error.TokenStreamInvalid(
                when: "Adding token to insert transform",
                details: "Failed altering path for data element to insert",
                token:,
              )
            })
          use path <- result.try(path)

          data_element
          |> prepend_data_element_tokens(path, acc)
          |> tokens_to_insert_before_tag(tag, path, rest, token, _)
        }

        False -> Ok(#(acc, data_elements_to_insert))
      }

    _ -> Ok(#(acc, data_elements_to_insert))
  }
}

fn prepend_data_element_tokens(
  data_element: #(DataElementTag, DataElementValue),
  path: DataSetPath,
  acc: List(P10Token),
) -> List(P10Token) {
  let #(tag, value) = data_element

  // This assert is safe because the function that gathers the tokens for the
  // data set never errors
  let assert Ok(tokens) =
    p10_token.data_element_to_tokens(tag, value, path, acc, fn(acc, token) {
      Ok([token, ..acc])
    })

  tokens
}
