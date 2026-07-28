//// A location used by a DICOM P10 read context to track where in the hierarchy
//// of sequences and items the DICOM P10 read is up to, along with associated
//// data required to correctly interpret incoming data elements at the current
//// location.
////
//// The following are tracked in the location during a DICOM P10 read:
////
//// 1. The end offset of defined-length sequences and items that need to have a
////    delimiter emitted. This allows defined lengths to be changed to
////    undefined lengths.
////
//// 2. The active specific character set that should be used to decode string
////    values that aren't in UTF-8. This is set/updated by the *'(0008,0005)
////    SpecificCharacterSet'* tag, most commonly in the root data set, but can
////    be overridden in a sequence item.
////
//// 3. The value of data elements that have been read and which are needed in
////    order to determine the correct VR of subsequent data elements when the
////    transfer syntax is 'Implicit VR Little Endian'.
////
////    E.g. the *'(0028,0106) Smallest Image Pixel Value'* data element uses
////    either the `UnsignedShort` or `SignedShort` VR, and determining which
////    requires the *'(0028,0103) Pixel Representation'* data element's value.
////
//// 4. Which clarifying data elements described in (3) have been used in the
////    interpretation of data element values, and where their values were
////    defined. This allows detection of a clarifying data element appearing
////    after data elements that it applies to, which isn't compatible with
////    stream-based reading of DICOM P10 data.

import dcmfx_character_set.{type SpecificCharacterSet}
import dcmfx_character_set/string_type
import dcmfx_core/data_element_tag.{type DataElementTag, DataElementTag}
import dcmfx_core/dictionary
import dcmfx_core/internal/utils
import dcmfx_core/value_representation.{type ValueRepresentation}
import dcmfx_p10/internal/value_length.{type ValueLength}
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_token.{type P10Token}
import gleam/bit_array
import gleam/bool
import gleam/dict.{type Dict}
import gleam/int
import gleam/option.{type Option, None, Some}
import gleam/result
import gleam/set.{type Set}
import gleam/string

/// A P10 location is a list of location entries, with the current/most recently
/// added one at the head of the list.
///
pub type P10Location =
  List(LocationEntry)

/// An entry in a P10 location. A root data set entry always appears exactly
/// once at the start, and can then be followed by sequences, each containing
/// nested lists of items that can themselves contain sequences.
///
pub opaque type LocationEntry {
  RootDataSet(
    clarifying_data_elements: ClarifyingDataElements,
    locally_defined_clarifying_data_elements: ClarifyingDataElementSet,
    used_clarifying_data_elements: ClarifyingDataElementSet,
    last_data_element_tag: DataElementTag,
  )
  Sequence(
    tag: DataElementTag,
    is_implicit_vr: Bool,
    ends_at: Option(Int),
    item_count: Int,
  )
  Item(
    clarifying_data_elements: ClarifyingDataElements,
    locally_defined_clarifying_data_elements: ClarifyingDataElementSet,
    used_clarifying_data_elements: ClarifyingDataElementSet,
    last_data_element_tag: DataElementTag,
    ends_at: Option(Int),
  )
}

/// The data elements needed to determine VRs of some data elements when the
/// transfer syntax is 'Implicit VR Little Endian', and to decode non-UTF-8
/// string data.
///
type ClarifyingDataElements {
  ClarifyingDataElements(
    specific_character_set: SpecificCharacterSet,
    bits_allocated: Option(Int),
    pixel_representation: Option(Int),
    waveform_bits_allocated: Option(Int),
    private_creators: Dict(DataElementTag, String),
  )
}

/// Returns whether a data element tag is for a clarifying data element that
/// needs to be materialized by the read process and added to the location.
///
pub fn is_clarifying_data_element(tag: DataElementTag) -> Bool {
  tag == dictionary.specific_character_set.tag
  || tag == dictionary.bits_allocated.tag
  || tag == dictionary.pixel_representation.tag
  || tag == dictionary.waveform_bits_allocated.tag
  || data_element_tag.is_private_creator(tag)
}

fn private_creator_for_tag(
  clarifying_data_elements: ClarifyingDataElements,
  tag: DataElementTag,
) -> Option(String) {
  use <- bool.guard(!data_element_tag.is_private(tag), None)

  clarifying_data_elements.private_creators
  |> dict.get(private_creator_tag_for_tag(tag))
  |> result.map(Some)
  |> result.unwrap(None)
}

/// Returns the tag of the *'(gggg,00xx) Private Creator'* data element that
/// defines the private block containing the specified private tag.
///
fn private_creator_tag_for_tag(tag: DataElementTag) -> DataElementTag {
  DataElementTag(tag.group, int.bitwise_shift_right(tag.element, 8))
}

/// Returns whether the VR of a data element is determined by the value of the
/// *'(0028,0103) PixelRepresentation'* data element, i.e. whether it uses either
/// the `UnsignedShort` or `SignedShort` VR.
///
fn is_pixel_representation_dependent(tag: DataElementTag) -> Bool {
  tag == dictionary.zero_velocity_pixel_value.tag
  || tag == dictionary.mapped_pixel_value.tag
  || tag == dictionary.smallest_valid_pixel_value.tag
  || tag == dictionary.largest_valid_pixel_value.tag
  || tag == dictionary.smallest_image_pixel_value.tag
  || tag == dictionary.largest_image_pixel_value.tag
  || tag == dictionary.smallest_pixel_value_in_series.tag
  || tag == dictionary.largest_pixel_value_in_series.tag
  || tag == dictionary.smallest_image_pixel_value_in_plane.tag
  || tag == dictionary.largest_image_pixel_value_in_plane.tag
  || tag == dictionary.pixel_padding_value.tag
  || tag == dictionary.pixel_padding_range_limit.tag
  || tag == dictionary.red_palette_color_lookup_table_descriptor.tag
  || tag == dictionary.green_palette_color_lookup_table_descriptor.tag
  || tag == dictionary.blue_palette_color_lookup_table_descriptor.tag
  || tag == dictionary.lut_descriptor.tag
  || tag == dictionary.real_world_value_last_value_mapped.tag
  || tag == dictionary.real_world_value_first_value_mapped.tag
  || tag == dictionary.histogram_first_bin_value.tag
  || tag == dictionary.histogram_last_bin_value.tag
}

/// A set of clarifying data elements. Each location tracks two of these: the
/// clarifying data elements locally defined at it, and those that have been used
/// in the interpretation of the data elements read at it.
///
/// Together these detect the case of a clarifying data element appearing *after*
/// data elements that it applies to. Such out-of-order data elements should not
/// occur in well-formed DICOM P10 data, and aren't compatible with stream-based
/// DICOM P10 parsing.
///
type ClarifyingDataElementSet {
  ClarifyingDataElementSet(
    specific_character_set: Bool,
    bits_allocated: Bool,
    pixel_representation: Bool,
    waveform_bits_allocated: Bool,
    private_creators: Set(DataElementTag),
  )
}

/// Returns a new empty set of clarifying data elements.
///
fn new_clarifying_data_element_set() -> ClarifyingDataElementSet {
  ClarifyingDataElementSet(False, False, False, False, set.new())
}

/// Adds a clarifying data element to a set of clarifying data elements.
///
fn clarifying_data_element_set_insert(
  clarifying_data_element_set: ClarifyingDataElementSet,
  tag: DataElementTag,
) -> ClarifyingDataElementSet {
  case tag {
    tag if tag == dictionary.specific_character_set.tag ->
      ClarifyingDataElementSet(
        ..clarifying_data_element_set,
        specific_character_set: True,
      )

    tag if tag == dictionary.bits_allocated.tag ->
      ClarifyingDataElementSet(
        ..clarifying_data_element_set,
        bits_allocated: True,
      )

    tag if tag == dictionary.pixel_representation.tag ->
      ClarifyingDataElementSet(
        ..clarifying_data_element_set,
        pixel_representation: True,
      )

    tag if tag == dictionary.waveform_bits_allocated.tag ->
      ClarifyingDataElementSet(
        ..clarifying_data_element_set,
        waveform_bits_allocated: True,
      )

    _ ->
      case data_element_tag.is_private_creator(tag) {
        True ->
          ClarifyingDataElementSet(
            ..clarifying_data_element_set,
            private_creators: set.insert(
              clarifying_data_element_set.private_creators,
              tag,
            ),
          )

        False -> clarifying_data_element_set
      }
  }
}

/// Returns whether a set of clarifying data elements contains the specified
/// clarifying data element.
///
fn clarifying_data_element_set_contains(
  clarifying_data_element_set: ClarifyingDataElementSet,
  tag: DataElementTag,
) -> Bool {
  case tag {
    tag if tag == dictionary.specific_character_set.tag ->
      clarifying_data_element_set.specific_character_set

    tag if tag == dictionary.bits_allocated.tag ->
      clarifying_data_element_set.bits_allocated

    tag if tag == dictionary.pixel_representation.tag ->
      clarifying_data_element_set.pixel_representation

    tag if tag == dictionary.waveform_bits_allocated.tag ->
      clarifying_data_element_set.waveform_bits_allocated

    _ -> set.contains(clarifying_data_element_set.private_creators, tag)
  }
}

/// Returns the default/initial value for the clarifying data elements.
///
fn default_clarifying_data_elements() -> ClarifyingDataElements {
  let assert Ok(charset) = dcmfx_character_set.from_string("ISO_IR 6")

  ClarifyingDataElements(charset, None, None, None, dict.new())
}

/// Creates a new P10 location with an initial entry for the root data set.
///
pub fn new() -> P10Location {
  [
    RootDataSet(
      default_clarifying_data_elements(),
      new_clarifying_data_element_set(),
      new_clarifying_data_element_set(),
      data_element_tag.zero,
    ),
  ]
}

/// Records that a data element with the specified tag and VR is being read at
/// the current P10 location, and checks that its ordering relative to the data
/// elements that precede it doesn't affect their interpretation.
///
/// In DICOM P10 data, data elements in a data set and sequence item must appear
/// in ascending order. This is relevant when reading DICOM P10 data in a
/// streaming fashion because lower numbered data elements are sometimes used in
/// the interpretation of higher numbered data elements.
///
/// However, the only data elements able to alter the interpretation of others
/// are the clarifying data elements, so an error is returned only when a
/// clarifying data element appears after a data element that it applies to.
/// Other out-of-order data elements are read without error because doing so
/// can't affect the result.
///
/// `is_vr_inferred` specifies whether the data element's VR was inferred by
/// `infer_vr_for_tag`, and `is_big_endian` specifies whether its value bytes
/// have their endianness swapped by `swap_endianness`.
///
pub fn check_data_element_ordering(
  location: P10Location,
  tag: DataElementTag,
  vr: ValueRepresentation,
  is_vr_inferred: Bool,
  is_big_endian: Bool,
) -> Result(P10Location, Nil) {
  let check = fn(
    used_clarifying_data_elements: ClarifyingDataElementSet,
    last_data_element_tag: DataElementTag,
  ) -> Result(DataElementTag, Nil) {
    let is_in_order =
      data_element_tag.to_int(tag)
      > data_element_tag.to_int(last_data_element_tag)

    // An out-of-order clarifying data element that has already been used in the
    // interpretation of a preceding data element is an error
    use <- bool.guard(
      !is_in_order
        && is_clarifying_data_element(tag)
        && clarifying_data_element_set_contains(
        used_clarifying_data_elements,
        tag,
      ),
      Error(Nil),
    )

    case is_in_order {
      True -> Ok(tag)
      False -> Ok(last_data_element_tag)
    }
  }

  case location {
    [
      RootDataSet(
        clarifying_data_elements:,
        locally_defined_clarifying_data_elements:,
        used_clarifying_data_elements:,
        last_data_element_tag:,
      ),
      ..rest
    ] -> {
      use last_data_element_tag <- result.map(check(
        used_clarifying_data_elements,
        last_data_element_tag,
      ))

      [
        RootDataSet(
          clarifying_data_elements,
          locally_defined_clarifying_data_elements,
          used_clarifying_data_elements,
          last_data_element_tag,
        ),
        ..rest
      ]
      |> record_clarifying_data_element_uses(
        tag,
        vr,
        is_vr_inferred,
        is_big_endian,
      )
    }

    [
      Item(
        clarifying_data_elements:,
        locally_defined_clarifying_data_elements:,
        used_clarifying_data_elements:,
        last_data_element_tag:,
        ends_at:,
      ),
      ..rest
    ] -> {
      use last_data_element_tag <- result.map(check(
        used_clarifying_data_elements,
        last_data_element_tag,
      ))

      [
        Item(
          clarifying_data_elements,
          locally_defined_clarifying_data_elements,
          used_clarifying_data_elements,
          last_data_element_tag,
          ends_at,
        ),
        ..rest
      ]
      |> record_clarifying_data_element_uses(
        tag,
        vr,
        is_vr_inferred,
        is_big_endian,
      )
    }

    [Sequence(..), ..] -> Ok(location)

    [] -> Error(Nil)
  }
}

/// Records the clarifying data elements used in the interpretation of a data
/// element with the specified tag and VR.
///
/// `is_vr_inferred` specifies whether the data element's VR was inferred by
/// `infer_vr_for_tag`, and `is_big_endian` specifies whether its value bytes
/// have their endianness swapped by `swap_endianness`.
///
fn record_clarifying_data_element_uses(
  location: P10Location,
  tag: DataElementTag,
  vr: ValueRepresentation,
  is_vr_inferred: Bool,
  is_big_endian: Bool,
) -> P10Location {
  // Encoded string values, with the exception of private creators, are decoded
  // using the active specific character set
  let location = case
    value_representation.is_encoded_string(vr)
    && !data_element_tag.is_private_creator(tag)
  {
    True ->
      record_clarifying_data_element_use(
        location,
        dictionary.specific_character_set.tag,
      )
    False -> location
  }

  // Inferring a VR uses the pixel representation for the data elements that are
  // either US or SS, and uses the private creator for private data elements
  let location = case is_vr_inferred && is_pixel_representation_dependent(tag) {
    True ->
      record_clarifying_data_element_use(
        location,
        dictionary.pixel_representation.tag,
      )
    False -> location
  }
  let location = case
    is_vr_inferred
    && data_element_tag.is_private(tag)
    && !data_element_tag.is_private_creator(tag)
  {
    True ->
      record_clarifying_data_element_use(
        location,
        private_creator_tag_for_tag(tag),
      )
    False -> location
  }

  // Swapping the endianness of pixel data and waveform data uses their bits
  // allocated value to determine the word size
  case is_big_endian && vr == value_representation.OtherWordString {
    True ->
      case tag {
        tag if tag == dictionary.pixel_data.tag ->
          record_clarifying_data_element_use(
            location,
            dictionary.bits_allocated.tag,
          )

        tag if tag == dictionary.waveform_data.tag ->
          record_clarifying_data_element_use(
            location,
            dictionary.waveform_bits_allocated.tag,
          )

        _ -> location
      }

    False -> location
  }
}

/// Records that the specified clarifying data element has been used in the
/// interpretation of a data element at the current P10 location.
///
/// The use is recorded on the current data set or item, as well as on each
/// enclosing data set or item up to and including the one that defines the
/// clarifying data element's value. This is because clarifying data elements are
/// inherited by nested locations but don't propagate back out of them, e.g. a
/// clarifying data element defined in an item applies only inside that item, so
/// its use there is unaffected by the same clarifying data element subsequently
/// appearing out of order in an enclosing data set or item.
///
fn record_clarifying_data_element_use(
  location: P10Location,
  tag: DataElementTag,
) -> P10Location {
  case location {
    [
      RootDataSet(
        clarifying_data_elements:,
        locally_defined_clarifying_data_elements:,
        used_clarifying_data_elements:,
        last_data_element_tag:,
      ),
      ..rest
    ] -> [
      RootDataSet(
        clarifying_data_elements,
        locally_defined_clarifying_data_elements,
        clarifying_data_element_set_insert(used_clarifying_data_elements, tag),
        last_data_element_tag,
      ),
      ..continue_recording_clarifying_data_element_use(
        rest,
        tag,
        locally_defined_clarifying_data_elements,
      )
    ]

    [
      Item(
        clarifying_data_elements:,
        locally_defined_clarifying_data_elements:,
        used_clarifying_data_elements:,
        last_data_element_tag:,
        ends_at:,
      ),
      ..rest
    ] -> [
      Item(
        clarifying_data_elements,
        locally_defined_clarifying_data_elements,
        clarifying_data_element_set_insert(used_clarifying_data_elements, tag),
        last_data_element_tag,
        ends_at,
      ),
      ..continue_recording_clarifying_data_element_use(
        rest,
        tag,
        locally_defined_clarifying_data_elements,
      )
    ]

    [entry, ..rest] -> [entry, ..record_clarifying_data_element_use(rest, tag)]

    [] -> []
  }
}

/// Continues recording the use of a clarifying data element in the enclosing
/// data sets and items, stopping once the location that defines its value has
/// been reached.
///
fn continue_recording_clarifying_data_element_use(
  location: P10Location,
  tag: DataElementTag,
  locally_defined_clarifying_data_elements: ClarifyingDataElementSet,
) -> P10Location {
  case
    clarifying_data_element_set_contains(
      locally_defined_clarifying_data_elements,
      tag,
    )
  {
    True -> location
    False -> record_clarifying_data_element_use(location, tag)
  }
}

/// Records that the specified clarifying data element is defined at the current
/// P10 location, i.e. that its value overrides the value inherited from any
/// enclosing data set or item.
///
fn record_clarifying_data_element_definition(
  location: P10Location,
  tag: DataElementTag,
) -> P10Location {
  case location {
    [
      RootDataSet(
        clarifying_data_elements:,
        locally_defined_clarifying_data_elements:,
        used_clarifying_data_elements:,
        last_data_element_tag:,
      ),
      ..rest
    ] -> [
      RootDataSet(
        clarifying_data_elements,
        clarifying_data_element_set_insert(
          locally_defined_clarifying_data_elements,
          tag,
        ),
        used_clarifying_data_elements,
        last_data_element_tag,
      ),
      ..rest
    ]

    [
      Item(
        clarifying_data_elements:,
        locally_defined_clarifying_data_elements:,
        used_clarifying_data_elements:,
        last_data_element_tag:,
        ends_at:,
      ),
      ..rest
    ] -> [
      Item(
        clarifying_data_elements,
        clarifying_data_element_set_insert(
          locally_defined_clarifying_data_elements,
          tag,
        ),
        used_clarifying_data_elements,
        last_data_element_tag,
        ends_at,
      ),
      ..rest
    ]

    [entry, ..rest] -> [
      entry,
      ..record_clarifying_data_element_definition(rest, tag)
    ]

    [] -> []
  }
}

/// Returns whether there is a sequence in the location that has forced the use
/// of the 'Implicit VR Little Endian' transfer syntax. This occurs when there
/// is an explicit VR of `UN` (Unknown) that has an undefined length.
///
/// Ref: DICOM Correction Proposal CP-246.
///
pub fn is_implicit_vr_forced(location: P10Location) -> Bool {
  case location {
    [Sequence(is_implicit_vr: True, ..), ..] -> True
    [_, ..rest] -> is_implicit_vr_forced(rest)
    _ -> False
  }
}

/// Returns the value of *'(0x0028,0x0100) Bits Allocated'* if present.
///
pub fn bits_allocated(location: P10Location) -> Option(Int) {
  active_clarifying_data_elements(location).bits_allocated
}

/// Swaps endianness of the value bytes for a given data element tag and VR.
///
/// This function handles the unusual behavior of pixel data, waveform data,
/// and the data elements storing single values in the waveform data's
/// sample encoding, that have a VR of OW but a word size of 32 or 64 bits.
/// This is a special case for endian swapping because they are actually
/// storing 32/64-bit words, not the 16-bit ones indicated by the VR.
///
pub fn swap_endianness(
  location: P10Location,
  tag: DataElementTag,
  vr: ValueRepresentation,
  data: BitArray,
) -> BitArray {
  let vr = case vr {
    value_representation.OtherWordString -> {
      let bits_allocated = case tag {
        tag if tag == dictionary.pixel_data.tag ->
          active_clarifying_data_elements(location).bits_allocated
        tag if tag == dictionary.waveform_data.tag ->
          active_clarifying_data_elements(location).waveform_bits_allocated

        // These data elements hold a single value in the waveform data's
        // sample encoding, so their length gives the word size. Ref: PS3.5
        // 8.3.
        tag
          if tag == dictionary.channel_minimum_value.tag
          || tag == dictionary.channel_maximum_value.tag
          || tag == dictionary.waveform_padding_value.tag
        ->
          case bit_array.byte_size(data) {
            4 -> Some(32)
            8 -> Some(64)
            _ -> None
          }

        _ -> None
      }

      case bits_allocated {
        Some(32) -> value_representation.UnsignedLong
        Some(64) -> value_representation.UnsignedVeryLong
        _ -> vr
      }
    }
    _ -> vr
  }

  value_representation.swap_endianness(vr, data)
}

/// Returns the next delimiter token for a location. This checks the `ends_at`
/// value of the entry at the head of the location to see if the bytes read has
/// met or exceeded it, and if it has then the relevant delimiter token is
/// returned.
///
/// This is part of the conversion of defined-length sequences and items to use
/// undefined lengths.
///
pub fn next_delimiter_token(
  location: P10Location,
  bytes_read: Int,
) -> Result(#(P10Token, P10Location), Nil) {
  case location {
    [Sequence(tag, ends_at: Some(ends_at), ..), ..rest]
      if ends_at <= bytes_read
    -> Ok(#(p10_token.SequenceDelimiter(tag), rest))

    [Item(ends_at: Some(ends_at), ..), ..rest] if ends_at <= bytes_read ->
      Ok(#(p10_token.SequenceItemDelimiter, rest))

    _ -> Error(Nil)
  }
}

/// Returns all pending delimiter tokens for a location, regardless of whether
/// their `ends_at` offset has been reached.
///
pub fn pending_delimiter_tokens(location: P10Location) -> List(P10Token) {
  case location {
    [Sequence(tag:, ..), ..rest] -> [
      p10_token.SequenceDelimiter(tag:),
      ..pending_delimiter_tokens(rest)
    ]

    [Item(..), ..rest] -> [
      p10_token.SequenceItemDelimiter,
      ..pending_delimiter_tokens(rest)
    ]

    _ -> [p10_token.End]
  }
}

/// Adds a new sequence to a P10 location.
///
pub fn add_sequence(
  location: P10Location,
  tag: DataElementTag,
  is_implicit_vr: Bool,
  ends_at: Option(Int),
) -> Result(P10Location, String) {
  case location {
    [RootDataSet(..)] | [Item(..), ..] ->
      Ok([Sequence(tag, is_implicit_vr, ends_at, 0), ..location])

    _ -> {
      let private_creator =
        private_creator_for_tag(active_clarifying_data_elements(location), tag)

      Error(
        "Sequence data element '"
        <> dictionary.tag_with_name(tag, private_creator)
        <> "' encountered outside of the root data set or an item",
      )
    }
  }
}

/// Ends the current sequence for a P10 location.
///
pub fn end_sequence(
  location: P10Location,
) -> Result(#(DataElementTag, P10Location), String) {
  case location {
    [Sequence(tag:, ..), ..rest] -> Ok(#(tag, rest))

    _ -> Error("Sequence delimiter encountered outside of a sequence")
  }
}

/// Returns the number of items that have been added to the current sequence.
///
pub fn sequence_item_count(location: P10Location) -> Result(Int, Nil) {
  case location {
    [Sequence(item_count:, ..), ..] -> Ok(item_count)
    _ -> Error(Nil)
  }
}

/// Adds a new item to a P10 location.
///
pub fn add_item(
  location: P10Location,
  ends_at: Option(Int),
  length: ValueLength,
) -> Result(#(Int, P10Location), String) {
  case location {
    // Carry across the current clarifying data elements as the initial state
    // for the new item
    [
      Sequence(tag, is_implicit_vr, ends_at: sequence_ends_at, item_count:),
      ..rest
    ] -> {
      let entries = [
        Item(
          active_clarifying_data_elements(location),
          new_clarifying_data_element_set(),
          new_clarifying_data_element_set(),
          data_element_tag.zero,
          ends_at,
        ),
        Sequence(tag, is_implicit_vr, sequence_ends_at, item_count + 1),
        ..rest
      ]

      Ok(#(item_count, entries))
    }

    _ ->
      Error(
        "Item encountered outside of a sequence, length: "
        <> value_length.to_string(length),
      )
  }
}

/// Ends the current item for a P10 location.
///
pub fn end_item(location: P10Location) -> Result(P10Location, String) {
  case location {
    [Item(..), ..rest] -> Ok(rest)

    _ -> Error("Item delimiter encountered outside of an item")
  }
}

/// Returns the clarifying data elements that apply to new data elements.
///
fn active_clarifying_data_elements(
  location: P10Location,
) -> ClarifyingDataElements {
  case location {
    [RootDataSet(clarifying_data_elements, ..), ..]
    | [Item(clarifying_data_elements, ..), ..] -> clarifying_data_elements

    [_, ..rest] -> active_clarifying_data_elements(rest)

    [] -> panic as "P10 location does not contain the root data set"
  }
}

/// Adds a clarifying data element to a location. The return value includes an
/// updated location and updated value bytes.
///
/// The only time that the value bytes are altered is the *'(0008,0005)
/// SpecificCharacterSet'* data element.
///
pub fn add_clarifying_data_element(
  location: P10Location,
  tag: DataElementTag,
  vr: ValueRepresentation,
  value_bytes: BitArray,
) -> Result(#(BitArray, P10Location), P10Error) {
  case tag, vr, value_bytes {
    tag, _, _ if tag == dictionary.specific_character_set.tag ->
      update_specific_character_set_clarifying_data_element(
        location,
        value_bytes,
      )

    _, value_representation.UnsignedShort, <<value:16-unsigned-little>> -> {
      let location =
        update_unsigned_short_clarifying_data_element(location, tag, value)

      Ok(#(value_bytes, location))
    }

    _, value_representation.LongString, _ -> {
      use <- bool.guard(
        !data_element_tag.is_private_creator(tag),
        Ok(#(value_bytes, location)),
      )

      update_private_creator_clarifying_data_element(location, value_bytes, tag)
      |> Ok
    }

    _, _, _ -> Ok(#(value_bytes, location))
  }
}

fn update_specific_character_set_clarifying_data_element(
  location: P10Location,
  value_bytes: BitArray,
) -> Result(#(BitArray, P10Location), P10Error) {
  let specific_character_set =
    value_bytes
    |> bit_array.to_string
    |> result.map_error(fn(_) {
      p10_error.SpecificCharacterSetInvalid(
        utils.inspect_bit_array(value_bytes, 64),
        "Invalid UTF-8",
      )
    })
  use specific_character_set <- result.try(specific_character_set)

  let charset =
    specific_character_set
    |> dcmfx_character_set.from_string
    |> result.map_error(fn(details) {
      p10_error.SpecificCharacterSetInvalid(
        string.slice(specific_character_set, 0, 64),
        details,
      )
    })
  use charset <- result.try(charset)

  // Set specific character set in current location
  let new_location =
    map_clarifying_data_elements(location, fn(clarifying_data_elements) {
      ClarifyingDataElements(
        ..clarifying_data_elements,
        specific_character_set: charset,
      )
    })
    |> record_clarifying_data_element_definition(
      dictionary.specific_character_set.tag,
    )

  Ok(#(<<"ISO_IR 192">>, new_location))
}

fn update_unsigned_short_clarifying_data_element(
  location: P10Location,
  tag: DataElementTag,
  value: Int,
) -> P10Location {
  case tag {
    tag if tag == dictionary.bits_allocated.tag ->
      location
      |> map_clarifying_data_elements(fn(clarifying_data_elements) {
        ClarifyingDataElements(
          ..clarifying_data_elements,
          bits_allocated: Some(value),
        )
      })
      |> record_clarifying_data_element_definition(tag)

    tag if tag == dictionary.pixel_representation.tag ->
      location
      |> map_clarifying_data_elements(fn(clarifying_data_elements) {
        ClarifyingDataElements(
          ..clarifying_data_elements,
          pixel_representation: Some(value),
        )
      })
      |> record_clarifying_data_element_definition(tag)

    tag if tag == dictionary.waveform_bits_allocated.tag ->
      location
      |> map_clarifying_data_elements(fn(clarifying_data_elements) {
        ClarifyingDataElements(
          ..clarifying_data_elements,
          waveform_bits_allocated: Some(value),
        )
      })
      |> record_clarifying_data_element_definition(tag)

    _ -> location
  }
}

fn update_private_creator_clarifying_data_element(
  location: P10Location,
  value_bytes: BitArray,
  tag: DataElementTag,
) -> #(BitArray, P10Location) {
  let location = case bit_array.to_string(value_bytes) {
    Ok(private_creator) -> {
      let private_creator = private_creator |> utils.trim_ascii_end(0x20)

      location
      |> map_clarifying_data_elements(fn(clarifying_data_elements) {
        ClarifyingDataElements(
          ..clarifying_data_elements,
          private_creators: dict.insert(
            clarifying_data_elements.private_creators,
            tag,
            private_creator,
          ),
        )
      })
      |> record_clarifying_data_element_definition(tag)
    }

    Error(Nil) -> location
  }

  #(value_bytes, location)
}

fn map_clarifying_data_elements(
  location: P10Location,
  map_fn: fn(ClarifyingDataElements) -> ClarifyingDataElements,
) -> P10Location {
  case location {
    [
      RootDataSet(
        clarifying_data_elements,
        locally_defined_clarifying_data_elements,
        used_clarifying_data_elements,
        last_data_element_tag,
      ),
      ..rest
    ] -> [
      RootDataSet(
        map_fn(clarifying_data_elements),
        locally_defined_clarifying_data_elements,
        used_clarifying_data_elements,
        last_data_element_tag,
      ),
      ..rest
    ]

    [
      Item(
        clarifying_data_elements,
        locally_defined_clarifying_data_elements,
        used_clarifying_data_elements,
        last_data_element_tag,
        ends_at,
      ),
      ..rest
    ] -> [
      Item(
        map_fn(clarifying_data_elements),
        locally_defined_clarifying_data_elements,
        used_clarifying_data_elements,
        last_data_element_tag,
        ends_at,
      ),
      ..rest
    ]

    _ -> location
  }
}

/// Returns whether the current specific character set is UTF-8.
///
pub fn is_specific_character_set_utf8(location: P10Location) -> Bool {
  dcmfx_character_set.is_utf8(
    active_clarifying_data_elements(location).specific_character_set,
  )
}

/// Decodes encoded string bytes using the currently active specific character
/// set and returns their UTF-8 bytes.
///
pub fn decode_string_bytes(
  location: P10Location,
  vr: ValueRepresentation,
  value_bytes: BitArray,
) -> BitArray {
  let charset = active_clarifying_data_elements(location).specific_character_set

  // Determine the type of the string to be decoded based on the VR. See the
  // `StringType` type for further details.
  let string_type = case vr {
    value_representation.PersonName -> string_type.PersonName

    value_representation.LongString
    | value_representation.ShortString
    | value_representation.UnlimitedCharacters -> string_type.MultiValue

    _ -> string_type.SingleValue
  }

  charset
  |> dcmfx_character_set.decode_bytes(value_bytes, string_type)
  |> bit_array.from_string
  |> value_representation.pad_bytes_to_even_length(vr, _)
}

/// When reading a DICOM P10 that uses the 'Implicit VR Little Endian'
/// transfer syntax, returns the VR for the data element, or an error if it
/// can't be determined.
///
/// The vast majority of VRs can be determined by looking in the dictionary as
/// the data element has only one valid VR. Data elements that can use more
/// than one VR depending on the context require additional logic.
///
/// On error, the tag of the clarifying data element that was missing or
/// invalid that caused the VR to not be able to be inferred is returned.
///
pub fn infer_vr_for_tag(
  location: P10Location,
  tag: DataElementTag,
) -> Result(ValueRepresentation, DataElementTag) {
  let clarifying_data_elements = active_clarifying_data_elements(location)

  let private_creator = private_creator_for_tag(clarifying_data_elements, tag)

  let allowed_vrs = case dictionary.find(tag, private_creator) {
    Ok(dictionary.Item(vrs: vrs, ..)) -> vrs
    Error(Nil) -> []
  }

  case allowed_vrs {
    [vr] -> Ok(vr)

    // For '(7FE0,0010) Pixel Data', OB is not usable when in an implicit VR
    // transfer syntax. Ref: PS3.5 8.2.
    [value_representation.OtherByteString, value_representation.OtherWordString]
      if tag == dictionary.pixel_data.tag
    -> Ok(value_representation.OtherWordString)

    // Use '(0028,0103) PixelRepresentation' to determine a US/SS VR on relevant
    // values
    [value_representation.UnsignedShort, value_representation.SignedShort] ->
      case is_pixel_representation_dependent(tag) {
        True ->
          case clarifying_data_elements.pixel_representation {
            Some(0) -> Ok(value_representation.UnsignedShort)
            Some(1) -> Ok(value_representation.SignedShort)
            _ -> Error(dictionary.pixel_representation.tag)
          }

        // The VR couldn't be determined, so fall back to UN
        False -> Ok(value_representation.Unknown)
      }

    // For '(5400,1010) Waveform Data' and the other waveform data elements
    // that store values in its sample encoding, OB is not usable when in an
    // implicit VR transfer syntax. Ref: PS3.5 8.3.
    [value_representation.OtherByteString, value_representation.OtherWordString]
      if tag == dictionary.channel_minimum_value.tag
      || tag == dictionary.channel_maximum_value.tag
      || tag == dictionary.waveform_padding_value.tag
      || tag == dictionary.waveform_data.tag
    -> Ok(value_representation.OtherWordString)

    // The VR for '(0028,3006) LUTData' doesn't need to be determined because
    // the raw binary representation of both VRs is the same. `OtherWordString`
    // is chosen because it's closer to being correct in the case of the LUT
    // containing tightly packed 8-bit data, which is allowed by the spec
    // (Ref: PS3.3 C.11.1.1.1), even though there is no VR that correctly
    // expresses this, i.e. OB is not a valid VR for LUTData.
    [value_representation.UnsignedShort, value_representation.OtherWordString]
      if tag == dictionary.lut_data.tag
    -> Ok(value_representation.OtherWordString)

    // The VR for '(60xx,3000) Overlay Data' doesn't need to be determined as
    // when the transfer syntax is 'Implicit VR Little Endian' it is always OW.
    // Ref: PS3.5 8.1.2.
    [value_representation.OtherByteString, value_representation.OtherWordString]
      if tag.group >= 0x6000 && tag.group <= 0x60FF && tag.element == 0x3000
    -> Ok(value_representation.OtherWordString)

    // The VR couldn't be determined, so fall back to UN
    _ -> Ok(value_representation.Unknown)
  }
}
