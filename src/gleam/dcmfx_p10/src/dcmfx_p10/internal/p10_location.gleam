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

import dcmfx_character_set.{type SpecificCharacterSet}
import dcmfx_character_set/string_type
import dcmfx_core/data_element_tag.{type DataElementTag, DataElementTag}
import dcmfx_core/internal/utils
import dcmfx_core/registry
import dcmfx_core/value_representation.{type ValueRepresentation}
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_part.{type P10Part}
import gleam/bit_array
import gleam/bool
import gleam/dict.{type Dict}
import gleam/int
import gleam/option.{type Option, None, Some}
import gleam/result

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
  RootDataSet(clarifying_data_elements: ClarifyingDataElements)
  Sequence(is_implicit_vr: Bool, ends_at: Option(Int), item_count: Int)
  Item(clarifying_data_elements: ClarifyingDataElements, ends_at: Option(Int))
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
    waveform_bits_stored: Option(Int),
    waveform_bits_allocated: Option(Int),
    private_creators: Dict(DataElementTag, String),
  )
}

/// Returns whether a data element tag is for a clarifying data element that
/// needs to be materialized by the read process and added to the location.
///
pub fn is_clarifying_data_element(tag: DataElementTag) -> Bool {
  tag == registry.specific_character_set.tag
  || tag == registry.bits_allocated.tag
  || tag == registry.pixel_representation.tag
  || tag == registry.waveform_bits_stored.tag
  || tag == registry.waveform_bits_allocated.tag
  || data_element_tag.is_private_creator(tag)
}

fn private_creator_for_tag(
  clarifying_data_elements: ClarifyingDataElements,
  tag: DataElementTag,
) -> Option(String) {
  use <- bool.guard(!data_element_tag.is_private(tag), None)

  let private_creator_tag =
    DataElementTag(tag.group, int.bitwise_shift_right(tag.element, 8))

  clarifying_data_elements.private_creators
  |> dict.get(private_creator_tag)
  |> result.map(Some)
  |> result.unwrap(None)
}

/// Returns the default/initial value for the clarifying data elements.
///
/// Strictly speaking the default value for the specific character set should be
/// the default character set: ISO 646. However, some DICOM P10 data has been
/// seen in the wild that does not select a SpecificCharacterSet and assumes
/// that ISO IR 100 (ISO 8859-1) is available by default. Given that this
/// character set is compatible with ISO 646 it is safe to default to it instead
/// in order to improve compatibility.
///
/// Note that if the DICOM P10 data explicitly selects "ISO_IR 6" then ISO 646
/// will be active and this default fallback to ISO IR 100 will no longer be in
/// effect.
///
fn default_clarifying_data_elements() -> ClarifyingDataElements {
  let assert Ok(charset) = dcmfx_character_set.from_string("ISO_IR 100")

  ClarifyingDataElements(charset, None, None, None, None, dict.new())
}

/// Creates a new P10 location with an initial entry for the root data set.
///
pub fn new() -> P10Location {
  [RootDataSet(default_clarifying_data_elements())]
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

/// Returns the next delimiter part for a location. This checks the `ends_at`
/// value of the entry at the head of the location to see if the bytes read has
/// met or exceeded it, and if it has then the relevant delimiter part is
/// returned.
///
/// This is part of the conversion of defined-length sequences and items to use
/// undefined lengths.
///
pub fn next_delimiter_part(
  location: P10Location,
  bytes_read: Int,
) -> Result(#(P10Part, P10Location), Nil) {
  case location {
    [Sequence(ends_at: Some(ends_at), ..), ..rest] if ends_at <= bytes_read ->
      Ok(#(p10_part.SequenceDelimiter, rest))

    [Item(ends_at: Some(ends_at), ..), ..rest] if ends_at <= bytes_read ->
      Ok(#(p10_part.SequenceItemDelimiter, rest))

    _ -> Error(Nil)
  }
}

/// Returns all pending delimiter parts for a location, regardless of whether
/// their `ends_at` offset has been reached.
///
pub fn pending_delimiter_parts(location: P10Location) -> List(P10Part) {
  case location {
    [Sequence(..), ..rest] -> [
      p10_part.SequenceDelimiter,
      ..pending_delimiter_parts(rest)
    ]

    [Item(..), ..rest] -> [
      p10_part.SequenceItemDelimiter,
      ..pending_delimiter_parts(rest)
    ]

    _ -> [p10_part.End]
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
      Ok([Sequence(is_implicit_vr, ends_at, 0), ..location])

    _ -> {
      let private_creator =
        private_creator_for_tag(active_clarifying_data_elements(location), tag)

      Error(
        "Sequence data element '"
        <> registry.tag_with_name(tag, private_creator)
        <> "' encountered outside of the root data set or an item",
      )
    }
  }
}

/// Ends the current sequence for a P10 location.
///
pub fn end_sequence(location: P10Location) -> Result(P10Location, String) {
  case location {
    [Sequence(..), ..rest] -> Ok(rest)

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
  length: Int,
) -> Result(P10Location, String) {
  case location {
    // Carry across the current clarifying data elements as the initial state
    // for the new item
    [Sequence(is_implicit_vr, ends_at: sequence_ends_at, item_count:), ..rest] ->
      Ok([
        Item(active_clarifying_data_elements(location), ends_at),
        Sequence(is_implicit_vr, sequence_ends_at, item_count + 1),
        ..rest
      ])

    _ ->
      Error(
        "Item encountered outside of a sequence, length: "
        <> int.to_string(length)
        <> " bytes",
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
    [RootDataSet(clarifying_data_elements), ..]
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
    tag, _, _ if tag == registry.specific_character_set.tag ->
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
      p10_error.SpecificCharacterSetInvalid("", "Invalid UTF-8")
    })
  use specific_character_set <- result.try(specific_character_set)

  let charset =
    specific_character_set
    |> dcmfx_character_set.from_string
    |> result.map_error(fn(error) {
      p10_error.SpecificCharacterSetInvalid(specific_character_set, error)
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

  Ok(#(<<"ISO_IR 192">>, new_location))
}

fn update_unsigned_short_clarifying_data_element(
  location: P10Location,
  tag: DataElementTag,
  value: Int,
) -> P10Location {
  case tag {
    tag if tag == registry.bits_allocated.tag ->
      location
      |> map_clarifying_data_elements(fn(clarifying_data_elements) {
        ClarifyingDataElements(
          ..clarifying_data_elements,
          bits_allocated: Some(value),
        )
      })

    tag if tag == registry.pixel_representation.tag ->
      location
      |> map_clarifying_data_elements(fn(clarifying_data_elements) {
        ClarifyingDataElements(
          ..clarifying_data_elements,
          pixel_representation: Some(value),
        )
      })

    tag if tag == registry.waveform_bits_stored.tag ->
      location
      |> map_clarifying_data_elements(fn(clarifying_data_elements) {
        ClarifyingDataElements(
          ..clarifying_data_elements,
          waveform_bits_stored: Some(value),
        )
      })

    tag if tag == registry.waveform_bits_allocated.tag ->
      location
      |> map_clarifying_data_elements(fn(clarifying_data_elements) {
        ClarifyingDataElements(
          ..clarifying_data_elements,
          waveform_bits_allocated: Some(value),
        )
      })

    _ -> location
  }
}

fn update_private_creator_clarifying_data_element(
  location: P10Location,
  value_bytes: BitArray,
  tag: DataElementTag,
) -> #(BitArray, P10Location) {
  let assert Ok(private_creator) =
    bit_array.to_string(value_bytes)
    |> result.map(utils.trim_right_whitespace)

  let location =
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

  #(value_bytes, location)
}

fn map_clarifying_data_elements(
  location: P10Location,
  map_fn: fn(ClarifyingDataElements) -> ClarifyingDataElements,
) -> P10Location {
  case location {
    [RootDataSet(clarifying_data_elements), ..rest] -> [
      RootDataSet(map_fn(clarifying_data_elements)),
      ..rest
    ]

    [Item(clarifying_data_elements, ends_at), ..rest] -> [
      Item(map_fn(clarifying_data_elements), ends_at),
      ..rest
    ]

    _ -> location
  }
}

/// Returns whether the current specific character set is byte compatible with
/// UTF-8.
///
pub fn is_specific_character_set_utf8_compatible(location: P10Location) -> Bool {
  dcmfx_character_set.is_utf8_compatible(
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

/// When reading a DICOM P10 that uses the 'Implicit VR Little Endian' transfer
/// syntax, returns the VR for the data element, or `Unknown` if it can't be
/// determined.
///
/// The vast majority of VRs can be determined by looking in the registry as
/// they only have one valid VR. Data elements that can use different VRs
/// depending on the context require additional logic, which isn't guaranteed
/// to succeed.
///
/// If no VR can be determined then the `Unknown` VR is returned.
///
pub fn infer_vr_for_tag(
  location: P10Location,
  tag: DataElementTag,
) -> ValueRepresentation {
  let clarifying_data_elements = active_clarifying_data_elements(location)

  let private_creator = private_creator_for_tag(clarifying_data_elements, tag)

  let allowed_vrs = case registry.find(tag, private_creator) {
    Ok(registry.Item(vrs: vrs, ..)) -> vrs
    Error(Nil) -> []
  }

  case allowed_vrs {
    [vr] -> vr

    // For '(7FE0, 0010) Pixel Data', OB is not usable when in an implicit VR
    // transfer syntax. Ref: PS3.5 8.2.
    [value_representation.OtherByteString, value_representation.OtherWordString]
      if tag == registry.pixel_data.tag
    -> value_representation.OtherWordString

    // Use '(0028,0103) PixelRepresentation' to determine a US/SS VR on relevant
    // values
    [value_representation.UnsignedShort, value_representation.SignedShort]
      if tag == registry.zero_velocity_pixel_value.tag
      || tag == registry.mapped_pixel_value.tag
      || tag == registry.smallest_valid_pixel_value.tag
      || tag == registry.largest_valid_pixel_value.tag
      || tag == registry.smallest_image_pixel_value.tag
      || tag == registry.largest_image_pixel_value.tag
      || tag == registry.smallest_pixel_value_in_series.tag
      || tag == registry.largest_pixel_value_in_series.tag
      || tag == registry.smallest_image_pixel_value_in_plane.tag
      || tag == registry.largest_image_pixel_value_in_plane.tag
      || tag == registry.pixel_padding_value.tag
      || tag == registry.pixel_padding_range_limit.tag
      || tag == registry.red_palette_color_lookup_table_descriptor.tag
      || tag == registry.green_palette_color_lookup_table_descriptor.tag
      || tag == registry.blue_palette_color_lookup_table_descriptor.tag
      || tag == registry.lut_descriptor.tag
      || tag == registry.real_world_value_last_value_mapped.tag
      || tag == registry.real_world_value_first_value_mapped.tag
      || tag == registry.histogram_first_bin_value.tag
      || tag == registry.histogram_last_bin_value.tag
    ->
      case clarifying_data_elements.pixel_representation {
        Some(0) -> value_representation.UnsignedShort
        _ -> value_representation.SignedShort
      }

    // Use '(003A,021A) WaveformBitsStored' to determine an OB/OW VR on relevant
    // values
    [value_representation.OtherByteString, value_representation.OtherWordString]
      if tag == registry.channel_minimum_value.tag
      || tag == registry.channel_maximum_value.tag
    ->
      case clarifying_data_elements.waveform_bits_stored {
        Some(16) -> value_representation.OtherWordString
        Some(_) -> value_representation.OtherByteString
        None -> value_representation.Unknown
      }

    // Use '(5400,1004) WaveformBitsAllocated' to determine an OB/OW VR on
    // relevant values
    [value_representation.OtherByteString, value_representation.OtherWordString]
      if tag == registry.waveform_padding_value.tag
      || tag == registry.waveform_data.tag
    ->
      case clarifying_data_elements.waveform_bits_allocated {
        Some(16) -> value_representation.OtherWordString
        Some(_) -> value_representation.OtherByteString
        None -> value_representation.Unknown
      }

    // The VR for '(0028,3006) LUTData' doesn't need to be determined because
    // the raw binary representation of both VRs is the same. `OtherWordString`
    // is chosen because it's closer to being correct in the case of the LUT
    // containing tightly packed 8-bit data, which is allowed by the spec
    // (Ref: PS3.3 C.11.1.1.1), even though there is no VR that correctly
    // expresses this, i.e. OB is not a valid VR for LUTData.
    [value_representation.UnsignedShort, value_representation.OtherWordString]
      if tag == registry.lut_data.tag
    -> value_representation.OtherWordString

    // The VR for '(60xx,3000) Overlay Data' doesn't need to be determined as
    // when the transfer syntax is 'Implicit VR Little Endian' it is always OW.
    // Ref: PS3.5 8.1.2.
    [value_representation.OtherByteString, value_representation.OtherWordString]
      if tag.group >= 0x6000 && tag.group <= 0x60FF && tag.element == 0x3000
    -> value_representation.OtherWordString

    // The VR couldn't be determined, so fall back to UN
    _ -> value_representation.Unknown
  }
}
