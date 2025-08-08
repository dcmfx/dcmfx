//// A data set builder materializes a stream of DICOM P10 tokens into an
//// in-memory data set.
////
//// Most commonly the stream of DICOM P10 tokens originates from reading raw
//// DICOM P10 data with the `p10_read` module.

import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_element_value.{type DataElementValue}
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/dictionary
import dcmfx_core/value_representation.{type ValueRepresentation}
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_token.{type P10Token}
import gleam/bit_array
import gleam/bool
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result
import gleam/string

/// A data set builder that can be fed a stream of DICOM P10 tokens and
/// materialize them into an in-memory data set.
///
pub opaque type DataSetBuilder {
  DataSetBuilder(
    file_preamble: Option(BitArray),
    file_meta_information: Option(DataSet),
    location: List(BuilderLocation),
    pending_data_element: Option(PendingDataElement),
    is_complete: Bool,
  )
}

/// Tracks where in the data set the builder is currently at, specifically the
/// sequences and sequence items currently in the process of being created.
///
type BuilderLocation {
  RootDataSet(data_set: DataSet)
  Sequence(tag: DataElementTag, items: List(DataSet))
  SequenceItem(data_set: DataSet)
  EncapsulatedPixelDataSequence(vr: ValueRepresentation, items: List(BitArray))
}

/// The pending data element is a data element for which a `DataElementHeader`
/// token has been received, but one or more of its `DataElementValueBytes`
/// tokens are still pending.
///
type PendingDataElement {
  PendingDataElement(
    tag: DataElementTag,
    vr: ValueRepresentation,
    data: List(BitArray),
  )
}

/// Creates a new data set builder that can be given DICOM P10 tokens to be
/// materialized into an in-memory DICOM data set.
///
pub fn new() -> DataSetBuilder {
  DataSetBuilder(
    file_preamble: None,
    file_meta_information: None,
    location: [RootDataSet(data_set.new())],
    pending_data_element: None,
    is_complete: False,
  )
}

/// Returns whether the data set builder is complete, i.e. whether it has
/// received the final `p10_token.End` token signalling the end of the incoming
/// DICOM P10 tokens.
///
pub fn is_complete(builder: DataSetBuilder) -> Bool {
  builder.is_complete
}

/// Returns the File Preamble read by a data set builder, or an error if it has
/// not yet been read. The File Preamble is always 128 bytes in size.
///
/// The content of these bytes are application-defined, and are often unused and
/// set to zero.
///
pub fn file_preamble(builder: DataSetBuilder) -> Result(BitArray, Nil) {
  builder.file_preamble
  |> option.to_result(Nil)
}

/// Returns the final data set constructed by a data set builder from the DICOM
/// P10 tokens it has been fed, or an error if it has not yet been fully read.
///
pub fn final_data_set(builder: DataSetBuilder) -> Result(DataSet, Nil) {
  let root_data_set = case builder.is_complete, builder.location {
    True, [RootDataSet(data_set)] -> Ok(data_set)
    _, _ -> Error(Nil)
  }
  use root_data_set <- result.map(root_data_set)

  let file_meta_information =
    builder.file_meta_information
    |> option.unwrap(data_set.new())

  data_set.merge(root_data_set, file_meta_information)
}

/// Takes a data set builder that isn't yet complete, e.g. because an error was
/// encountered reading the source of the P10 tokens it was being built from,
/// and adds the necessary delimiter and end tokens so that it is considered
/// complete and can have its final data set read out.
///
/// This allows a partially built data set to be retrieved in its current state.
/// This should never be needed when reading or constructing valid and complete
/// DICOM P10 data.
///
pub fn force_end(builder: DataSetBuilder) -> DataSetBuilder {
  use <- bool.guard(builder.is_complete, builder)

  let builder = DataSetBuilder(..builder, pending_data_element: None)

  let token = case builder.location {
    [Sequence(tag:, ..), ..] -> p10_token.SequenceDelimiter(tag)

    [EncapsulatedPixelDataSequence(..), ..] ->
      p10_token.SequenceDelimiter(dictionary.pixel_data.tag)

    [SequenceItem(..), ..] -> p10_token.SequenceItemDelimiter

    _ -> p10_token.End
  }

  let assert Ok(builder) = builder |> add_token(token)

  force_end(builder)
}

/// Adds new DICOM P10 token to a data set builder. This function is responsible
/// for progressively constructing a data set from the tokens received, and also
/// checks that the tokens being received are in a valid order.
///
pub fn add_token(
  builder: DataSetBuilder,
  token: P10Token,
) -> Result(DataSetBuilder, P10Error) {
  use <- bool.guard(
    builder.is_complete,
    Error(p10_error.TokenStreamInvalid(
      "Building data set",
      "Token received after the token stream has ended",
      token,
    )),
  )

  // If there's a pending data element then it needs to be dealt with first as
  // the incoming token must be a DataElementValueBytes
  use <- bool.lazy_guard(builder.pending_data_element != None, fn() {
    add_token_to_pending_data_element(builder, token)
  })

  case token, builder.location {
    // Handle File Preamble token
    p10_token.FilePreambleAndDICMPrefix(preamble), _ ->
      Ok(DataSetBuilder(..builder, file_preamble: Some(preamble)))

    // Handle File Meta Information token
    p10_token.FileMetaInformation(data_set), _ ->
      Ok(DataSetBuilder(..builder, file_meta_information: Some(data_set)))

    // If a sequence is being read then add this token to it
    _, [Sequence(..), ..] -> add_token_to_sequence(builder, token)

    // If an encapsulated pixel data sequence is being read then add this token
    // to it
    _, [EncapsulatedPixelDataSequence(..), ..] ->
      add_token_to_encapsulated_pixel_data_sequence(builder, token)

    // Add this token to the current data set, which will be either the root
    // data set or an item in a sequence
    _, _ -> add_token_to_data_set(builder, token)
  }
}

/// Ingests the next token when the data set builder's current location
/// specifies a sequence.
///
fn add_token_to_sequence(
  builder: DataSetBuilder,
  token: P10Token,
) -> Result(DataSetBuilder, P10Error) {
  case token, builder.location {
    p10_token.SequenceItemStart(..), [RootDataSet(_)]
    | p10_token.SequenceItemStart(..), [Sequence(..), ..]
    ->
      Ok(
        DataSetBuilder(..builder, location: [
          SequenceItem(data_set.new()),
          ..builder.location
        ]),
      )

    p10_token.SequenceDelimiter(..), [Sequence(tag, items), ..sequence_location]
    -> {
      let sequence =
        items
        |> list.reverse
        |> data_element_value.new_sequence

      let new_location =
        insert_data_element_at_current_location(
          sequence_location,
          tag,
          sequence,
        )

      Ok(DataSetBuilder(..builder, location: new_location))
    }

    token, _ -> unexpected_token_error(token, builder)
  }
}

/// Ingests the next token when the data set builder's current location
/// specifies an encapsulated pixel data sequence.
///
fn add_token_to_encapsulated_pixel_data_sequence(
  builder: DataSetBuilder,
  token: P10Token,
) -> Result(DataSetBuilder, P10Error) {
  case token, builder.location {
    p10_token.PixelDataItem(..), _ ->
      DataSetBuilder(
        ..builder,
        pending_data_element: Some(
          PendingDataElement(
            dictionary.item.tag,
            value_representation.OtherByteString,
            [],
          ),
        ),
      )
      |> Ok

    p10_token.SequenceDelimiter(..),
      [EncapsulatedPixelDataSequence(vr, items), ..sequence_location]
    -> {
      let assert Ok(value) =
        items
        |> list.reverse
        |> data_element_value.new_encapsulated_pixel_data(vr, _)

      let new_location =
        insert_data_element_at_current_location(
          sequence_location,
          dictionary.pixel_data.tag,
          value,
        )

      Ok(DataSetBuilder(..builder, location: new_location))
    }

    _, _ -> unexpected_token_error(token, builder)
  }
}

/// Ingests the next token when the data set builder's current location is in
/// either the root data set or in an item that's part of a sequence.
///
fn add_token_to_data_set(
  builder: DataSetBuilder,
  token: P10Token,
) -> Result(DataSetBuilder, P10Error) {
  case token {
    // If this token is the start of a new data element then create a new
    // pending data element that will have its data filled in by subsequent
    // DataElementValueBytes tokens
    p10_token.DataElementHeader(tag, vr, _length, ..) ->
      DataSetBuilder(
        ..builder,
        pending_data_element: Some(PendingDataElement(tag, vr, [])),
      )
      |> Ok

    // If this token indicates the start of a new sequence then update the
    // current location accordingly
    p10_token.SequenceStart(tag, vr, ..) -> {
      let new_location = case vr {
        value_representation.OtherByteString
        | value_representation.OtherWordString ->
          EncapsulatedPixelDataSequence(vr, [])

        _ -> Sequence(tag, [])
      }

      DataSetBuilder(..builder, location: [new_location, ..builder.location])
      |> Ok
    }

    // If this token indicates the end of the current item then check that the
    // current location is in fact an item
    p10_token.SequenceItemDelimiter ->
      case builder.location {
        [SequenceItem(item_data_set), Sequence(tag, items), ..rest] -> {
          let new_location = [Sequence(tag, [item_data_set, ..items]), ..rest]

          Ok(DataSetBuilder(..builder, location: new_location))
        }

        _ ->
          Error(p10_error.TokenStreamInvalid(
            "Building data set",
            "Received sequence item delimiter token outside of an item",
            token:,
          ))
      }

    // If this token indicates the end of the DICOM P10 tokens then mark the
    // builder as complete, so long as it's currently located in the root
    // data set
    p10_token.End ->
      case builder.location {
        [RootDataSet(..)] -> Ok(DataSetBuilder(..builder, is_complete: True))

        _ ->
          Error(p10_error.TokenStreamInvalid(
            "Building data set",
            "Received end token outside of the root data set",
            token:,
          ))
      }

    token -> unexpected_token_error(token, builder)
  }
}

/// Ingests the next token when the data set builder has a pending data element
/// that is expecting value bytes tokens containing its data.
///
fn add_token_to_pending_data_element(
  builder: DataSetBuilder,
  token: P10Token,
) -> Result(DataSetBuilder, P10Error) {
  case token, builder.pending_data_element {
    p10_token.DataElementValueBytes(data:, bytes_remaining:, ..),
      Some(pending_data_element)
    -> {
      let tag = pending_data_element.tag
      let vr = pending_data_element.vr
      let data = [data, ..pending_data_element.data]

      case bytes_remaining {
        0 -> {
          let value = build_final_data_element_value(tag, vr, data)

          let new_location =
            insert_data_element_at_current_location(
              builder.location,
              tag,
              value,
            )

          DataSetBuilder(
            ..builder,
            location: new_location,
            pending_data_element: None,
          )
          |> Ok
        }

        _ ->
          DataSetBuilder(
            ..builder,
            pending_data_element: Some(PendingDataElement(tag, vr, data)),
          )
          |> Ok
      }
    }

    token, _ -> unexpected_token_error(token, builder)
  }
}

/// Inserts a new data element into the head of the given data set builder
/// location and returns an updated location.
///
fn insert_data_element_at_current_location(
  location: List(BuilderLocation),
  tag: DataElementTag,
  value: DataElementValue,
) -> List(BuilderLocation) {
  case location, data_element_value.bytes(value) {
    // Insert new data element into the root data set
    [RootDataSet(data_set)], _ -> [
      data_set
      |> data_set.insert(tag, value)
      |> RootDataSet,
    ]

    // Insert new data element into the current sequence item
    [SequenceItem(item_data_set), ..rest], _ -> [
      item_data_set
        |> data_set.insert(tag, value)
        |> SequenceItem,
      ..rest
    ]

    // Insert new data element into the current encapsulated pixel data sequence
    [EncapsulatedPixelDataSequence(vr, items), ..rest], Ok(bytes) -> [
      EncapsulatedPixelDataSequence(vr, [bytes, ..items]),
      ..rest
    ]

    // Other locations aren't valid for insertion of a data element. This case
    // is not expected to be logically possible.
    _, _ -> panic as "Internal error: unable to insert at current location"
  }
}

/// The error returned when an unexpected DICOM P10 token is received.
///
fn unexpected_token_error(
  token: P10Token,
  builder: DataSetBuilder,
) -> Result(DataSetBuilder, P10Error) {
  Error(p10_error.TokenStreamInvalid(
    "Building data set",
    "Received unexpected P10 token at location: "
      <> location_to_string(builder.location, []),
    token:,
  ))
}

/// Takes the tag, VR, and final bytes for a new data element and returns the
/// `DataElementValue` for it to insert into the active data set.
///
fn build_final_data_element_value(
  tag: DataElementTag,
  vr: ValueRepresentation,
  value_bytes: List(BitArray),
) -> DataElementValue {
  // Concatenate all received bytes to get the bytes that are the final bytes
  // for the data element value
  let final_bytes =
    value_bytes
    |> list.reverse
    |> bit_array.concat

  // Lookup table descriptors are a special case due to the non-standard way
  // their VR applies to their underlying bytes
  case dictionary.is_lut_descriptor_tag(tag) {
    True ->
      data_element_value.new_lookup_table_descriptor_unchecked(vr, final_bytes)
    False -> data_element_value.new_binary_unchecked(vr, final_bytes)
  }
}

/// Converts a data set location to a human-readable string for error reporting
/// and debugging purposes.
///
fn location_to_string(
  location: List(BuilderLocation),
  acc: List(String),
) -> String {
  case location {
    [] -> string.join(acc, ".")

    [item, ..rest] -> {
      let s = case item {
        RootDataSet(..) -> "RootDataSet"
        Sequence(tag, ..) -> "Sequence" <> data_element_tag.to_string(tag)
        SequenceItem(..) -> "SequenceItem"
        EncapsulatedPixelDataSequence(..) -> "EncapsulatedPixelDataSequence"
      }

      location_to_string(rest, [s, ..acc])
    }
  }
}
