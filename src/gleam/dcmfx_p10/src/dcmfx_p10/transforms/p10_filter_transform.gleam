import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/dictionary
import dcmfx_core/value_representation.{type ValueRepresentation}
import dcmfx_p10/p10_token.{type P10Token}
import gleam/list
import gleam/option.{type Option, None, Some}

/// Transform that applies a data element filter to a stream of DICOM P10
/// tokens.
///
pub opaque type P10FilterTransform {
  P10FilterTransform(
    predicate: PredicateFunction,
    location: List(LocationEntry),
  )
}

pub type LocationEntry {
  LocationEntry(tag: DataElementTag, filter_result: Bool)
}

/// Defines a function called by a `P10FilterTransform` that determines whether
/// a data element should pass through the filter.
///
pub type PredicateFunction =
  fn(DataElementTag, ValueRepresentation, Option(Int), List(LocationEntry)) ->
    Bool

/// Creates a new filter transform for filtering a stream of DICOM P10 tokens.
///
/// The predicate function is called as tokens are added to the context, and
/// only those data elements that return `True` from the predicate function
/// will pass through this filter transform.
///
pub fn new(predicate: PredicateFunction) -> P10FilterTransform {
  P10FilterTransform(predicate: predicate, location: [])
}

/// Returns whether the current position of the P10 filter context is the root
/// data set, i.e. there are no nested sequences currently active.
///
pub fn is_at_root(context: P10FilterTransform) -> Bool {
  context.location == []
}

/// Adds the next token to the P10 filter transform and returns whether it
/// should be included in the filtered token stream.
///
pub fn add_token(
  context: P10FilterTransform,
  token: P10Token,
) -> #(Bool, P10FilterTransform) {
  let push_location_entry = fn(
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: Option(Int),
  ) {
    let filter_result = case context.location {
      [] | [LocationEntry(_, True), ..] ->
        context.predicate(tag, vr, length, context.location)

      // The predicate function is skipped if a parent has already been filtered
      // out
      _ -> False
    }

    let new_location = [LocationEntry(tag, filter_result), ..context.location]

    let new_context = P10FilterTransform(..context, location: new_location)

    #(filter_result, new_context)
  }

  case token {
    // If this is a new sequence or data element then run the predicate function
    // to see if it passes the filter, then add it to the location
    p10_token.SequenceStart(tag, vr) -> push_location_entry(tag, vr, None)
    p10_token.DataElementHeader(tag, vr, length) ->
      push_location_entry(tag, vr, Some(length))

    // If this is a new pixel data item then add it to the location
    p10_token.PixelDataItem(_) -> {
      let filter_result = case context.location {
        [LocationEntry(filter_result:, ..), ..] -> filter_result
        _ -> True
      }

      let new_location = [
        LocationEntry(dictionary.item.tag, filter_result),
        ..context.location
      ]

      let new_context = P10FilterTransform(..context, location: new_location)

      #(filter_result, new_context)
    }

    // Detect the end of the entry at the head of the location and pop it off
    p10_token.SequenceDelimiter(..)
    | p10_token.DataElementValueBytes(bytes_remaining: 0, ..) -> {
      let filter_result = case context.location {
        [LocationEntry(filter_result:, ..), ..] -> filter_result
        _ -> True
      }

      let assert Ok(new_location) = list.rest(context.location)
      let new_context = P10FilterTransform(..context, location: new_location)

      #(filter_result, new_context)
    }

    _ ->
      case context.location {
        // If tokens are currently being filtered out then swallow this one
        [LocationEntry(_, False), ..] -> #(False, context)

        // Otherwise this token passes through the filter
        _ -> #(True, context)
      }
  }
}
