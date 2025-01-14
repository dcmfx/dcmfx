import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/dictionary
import dcmfx_core/value_representation.{type ValueRepresentation}
import dcmfx_p10/data_set_builder.{type DataSetBuilder}
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_part.{type P10Part}
import gleam/list
import gleam/option.{type Option, None, Some}

/// Transform that applies a data element filter to a stream of DICOM P10 parts.
///
pub opaque type P10FilterTransform {
  P10FilterTransform(
    predicate: PredicateFunction,
    location: List(LocationEntry),
    data_set_builder: Option(Result(DataSetBuilder, P10Error)),
  )
}

pub type LocationEntry {
  LocationEntry(tag: DataElementTag, filter_result: Bool)
}

type PredicateFunction =
  fn(DataElementTag, ValueRepresentation, List(LocationEntry)) -> Bool

/// Creates a new filter transform for filtering a stream of DICOM P10 parts.
///
/// The predicate function is called as parts are added to the context, and
/// only those data elements that return `True` from the predicate function
/// will pass through this filter transform.
///
/// If `create_data_set` is `True` then the data elements that are permitted
/// by the predicate are collected into an in-memory data set that can be
/// retrieved with `data_set()`.
///
pub fn new(
  predicate: PredicateFunction,
  create_data_set: Bool,
) -> P10FilterTransform {
  let data_set_builder = case create_data_set {
    True -> Some(Ok(data_set_builder.new()))
    False -> None
  }

  P10FilterTransform(
    predicate: predicate,
    location: [],
    data_set_builder: data_set_builder,
  )
}

/// Returns whether the current position of the P10 filter context is the root
/// data set, i.e. there are no nested sequences currently active.
///
pub fn is_at_root(context: P10FilterTransform) -> Bool {
  context.location == []
}

/// Returns a data set containing all data elements allowed by the predicate
/// function for the context. This is only available if `create_data_set` was
/// set to true when the context was created.
///
pub fn data_set(context: P10FilterTransform) -> Result(DataSet, P10Error) {
  case context.data_set_builder {
    Some(Ok(builder)) -> {
      let assert Ok(builder) =
        builder
        |> data_set_builder.force_end
        |> data_set_builder.final_data_set

      Ok(builder)
    }

    Some(Error(e)) -> Error(e)

    None -> Ok(data_set.new())
  }
}

/// Adds the next part to the P10 filter transform and returns whether it should
/// be included in the filtered part stream or not.
///
pub fn add_part(
  context: P10FilterTransform,
  part: P10Part,
) -> #(Bool, P10FilterTransform) {
  let #(filter_result, context) = case part {
    // If this is a new sequence or data element then run the predicate function
    // to see if it passes the filter, then add it to the location
    p10_part.SequenceStart(tag, vr) | p10_part.DataElementHeader(tag, vr, _) -> {
      // The predicate function is skipped if a parent has already been filtered
      // out
      let filter_result = case context.location {
        [] | [LocationEntry(_, True), ..] ->
          context.predicate(tag, vr, context.location)

        _ -> False
      }

      let new_location = [LocationEntry(tag, filter_result), ..context.location]

      let new_context = P10FilterTransform(..context, location: new_location)

      #(filter_result, new_context)
    }

    // If this is a new pixel data item then add it to the location
    p10_part.PixelDataItem(_) -> {
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
    p10_part.SequenceDelimiter
    | p10_part.DataElementValueBytes(bytes_remaining: 0, ..) -> {
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
        // If parts are currently being filtered out then swallow this one
        [LocationEntry(_, False), ..] -> #(False, context)

        // Otherwise this part passes through the filter
        _ -> #(True, context)
      }
  }

  // Pass the filtered parts through the data set builder if a data set of the
  // retained parts is being constructed
  let data_set_builder = case filter_result {
    True ->
      case context.data_set_builder {
        Some(Ok(builder)) ->
          case part {
            p10_part.FileMetaInformation(..) -> Some(Ok(builder))
            _ -> Some(data_set_builder.add_part(builder, part))
          }

        a -> a
      }
    False -> context.data_set_builder
  }

  let context = P10FilterTransform(..context, data_set_builder:)

  #(filter_result, context)
}
