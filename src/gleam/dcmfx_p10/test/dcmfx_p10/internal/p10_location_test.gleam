import dcmfx_core/data_element_tag.{type DataElementTag, DataElementTag}
import dcmfx_core/dictionary
import dcmfx_core/value_representation.{type ValueRepresentation}
import dcmfx_p10/internal/p10_location.{type P10Location}
import dcmfx_p10/internal/value_length
import gleam/option.{None}
import gleam/result

/// Reads a data element at the current location, with its VR not inferred and
/// the transfer syntax being little endian.
///
fn read_data_element(
  location: P10Location,
  item: dictionary.Item,
) -> Result(P10Location, Nil) {
  let assert [vr, ..] = item.vrs

  p10_location.check_data_element_ordering(location, item.tag, vr, False, False)
}

/// Reads a data element at the current location with the specified VR and
/// transfer syntax characteristics.
///
fn read_data_element_with_vr(
  location: P10Location,
  tag: DataElementTag,
  vr: ValueRepresentation,
  is_vr_inferred: Bool,
  is_big_endian: Bool,
) -> Result(P10Location, Nil) {
  p10_location.check_data_element_ordering(
    location,
    tag,
    vr,
    is_vr_inferred,
    is_big_endian,
  )
}

/// Reads a clarifying data element at the current location, defining its value
/// there.
///
fn read_clarifying_data_element(
  location: P10Location,
  item: dictionary.Item,
  value_bytes: BitArray,
) -> Result(P10Location, Nil) {
  let assert [vr, ..] = item.vrs

  use location <- result.try(read_data_element(location, item))

  let assert Ok(#(_, location)) =
    p10_location.add_clarifying_data_element(
      location,
      item.tag,
      vr,
      value_bytes,
    )

  Ok(location)
}

/// Reads big endian pixel data at the current location, which uses the bits
/// allocated value to determine the word size when swapping endianness.
///
fn read_big_endian_pixel_data(
  location: P10Location,
) -> Result(P10Location, Nil) {
  read_data_element_with_vr(
    location,
    dictionary.pixel_data.tag,
    value_representation.OtherWordString,
    False,
    True,
  )
}

/// Reads a sequence containing a single item at the current location, calling
/// the passed function to read the item's content.
///
fn read_sequence_item(
  location: P10Location,
  item: dictionary.Item,
  read_item_content: fn(P10Location) -> P10Location,
) -> P10Location {
  let assert Ok(location) = read_data_element(location, item)
  let assert Ok(location) =
    p10_location.add_sequence(location, item.tag, False, None)
  let assert Ok(#(_, location)) =
    p10_location.add_item(location, None, value_length.Undefined)

  let location = read_item_content(location)

  let assert Ok(location) = p10_location.end_item(location)
  let assert Ok(#(_, location)) = p10_location.end_sequence(location)

  location
}

pub fn ordered_data_elements_are_allowed_test() {
  let location = p10_location.new()

  let assert Ok(location) =
    read_data_element(location, dictionary.specific_character_set)

  let assert Ok(_) = read_data_element(location, dictionary.patient_name)
}

pub fn out_of_order_data_elements_that_arent_clarifying_are_allowed_test() {
  let location = p10_location.new()

  let assert Ok(location) = read_data_element(location, dictionary.patient_name)
  let assert Ok(location) =
    read_data_element(location, dictionary.series_description)

  let assert Ok(_) = read_data_element(location, dictionary.patient_name)
}

pub fn out_of_order_specific_character_set_is_only_allowed_when_unused_test() {
  // An out-of-order specific character set following a data element that
  // doesn't use an encoded string VR is allowed
  let assert Ok(location) =
    read_data_element(p10_location.new(), dictionary.sop_instance_uid)
  let assert Ok(_) =
    read_data_element(location, dictionary.specific_character_set)

  // An out-of-order specific character set following a data element that uses
  // an encoded string VR is an error
  let assert Ok(location) =
    read_data_element(p10_location.new(), dictionary.patient_name)
  let assert Error(Nil) =
    read_data_element(location, dictionary.specific_character_set)
}

pub fn out_of_order_specific_character_set_used_in_an_item_is_an_error_test() {
  // Read a sequence containing an item that has a data element using an encoded
  // string VR
  let location =
    read_sequence_item(
      p10_location.new(),
      dictionary.referenced_study_sequence,
      fn(location) {
        let assert Ok(location) =
          read_data_element(location, dictionary.patient_name)
        location
      },
    )

  // The specific character set used by the item's data element is inherited
  // from the root data set, so it appearing out of order in the root data set
  // is an error
  let assert Error(Nil) =
    read_data_element(location, dictionary.specific_character_set)
}

pub fn clarifying_data_element_defined_in_an_item_doesnt_apply_outside_it_test() {
  // Read a sequence containing an item that uses the bits allocated value
  // inherited from the root data set. Bits allocated then appearing out of
  // order in the root data set is an error because it alters how the item's
  // pixel data should have been read.
  let location =
    read_sequence_item(
      p10_location.new(),
      dictionary.shared_functional_groups_sequence,
      fn(location) {
        let assert Ok(location) = read_big_endian_pixel_data(location)
        location
      },
    )
  let assert Error(Nil) = read_data_element(location, dictionary.bits_allocated)

  // Read a sequence containing an item that defines its own bits allocated
  // value before using it. Bits allocated then appearing out of order in the
  // root data set is allowed because the item's pixel data didn't depend on the
  // root data set's value.
  let location =
    read_sequence_item(
      p10_location.new(),
      dictionary.shared_functional_groups_sequence,
      fn(location) {
        let assert Ok(location) =
          read_clarifying_data_element(location, dictionary.bits_allocated, <<
            32:16-little,
          >>)
        let assert Ok(location) = read_big_endian_pixel_data(location)
        location
      },
    )
  let assert Ok(_) = read_data_element(location, dictionary.bits_allocated)
}

pub fn clarifying_data_element_redefined_in_an_item_doesnt_apply_outside_it_test() {
  // Read a root data set that defines bits allocated, followed by a sequence
  // containing an item that uses the inherited value. Bits allocated then
  // appearing out of order in the root data set is an error because it alters
  // the value that the item's pixel data was read with.
  let assert Ok(location) =
    read_clarifying_data_element(p10_location.new(), dictionary.bits_allocated, <<
      16:16-little,
    >>)
  let location =
    read_sequence_item(
      location,
      dictionary.shared_functional_groups_sequence,
      fn(location) {
        let assert Ok(location) = read_big_endian_pixel_data(location)
        location
      },
    )
  let assert Error(Nil) = read_data_element(location, dictionary.bits_allocated)

  // Read the same, but with the item redefining bits allocated before using it.
  // Bits allocated then appearing out of order in the root data set is allowed
  // because the item's pixel data used the item's own value.
  let assert Ok(location) =
    read_clarifying_data_element(p10_location.new(), dictionary.bits_allocated, <<
      16:16-little,
    >>)
  let location =
    read_sequence_item(
      location,
      dictionary.shared_functional_groups_sequence,
      fn(location) {
        let assert Ok(location) =
          read_clarifying_data_element(location, dictionary.bits_allocated, <<
            32:16-little,
          >>)
        let assert Ok(location) = read_big_endian_pixel_data(location)
        location
      },
    )
  let assert Ok(_) = read_data_element(location, dictionary.bits_allocated)
}

pub fn out_of_order_clarifying_data_element_redefined_at_a_location_is_an_error_test() {
  // Define bits allocated and then use it when reading big endian pixel data
  let assert Ok(location) =
    read_clarifying_data_element(p10_location.new(), dictionary.bits_allocated, <<
      32:16-little,
    >>)
  let assert Ok(location) = read_big_endian_pixel_data(location)

  // A second out-of-order bits allocated is an error because the pixel data was
  // read using the first one's value
  let assert Error(Nil) = read_data_element(location, dictionary.bits_allocated)
}

pub fn out_of_order_private_creator_is_only_allowed_when_unused_test() {
  let private_creator = DataElementTag(0x0009, 0x0010)
  let private_tag = DataElementTag(0x0009, 0x1001)

  // An out-of-order private creator following a private data element in its
  // block that had its VR inferred is an error
  let assert Ok(location) =
    read_data_element_with_vr(
      p10_location.new(),
      private_tag,
      value_representation.Unknown,
      True,
      False,
    )
  let assert Error(Nil) =
    read_data_element_with_vr(
      location,
      private_creator,
      value_representation.LongString,
      False,
      False,
    )

  // An out-of-order private creator for a different private block is allowed
  let assert Ok(location) =
    read_data_element_with_vr(
      p10_location.new(),
      private_tag,
      value_representation.Unknown,
      True,
      False,
    )
  let assert Ok(_) =
    read_data_element_with_vr(
      location,
      DataElementTag(0x0009, 0x0011),
      value_representation.LongString,
      False,
      False,
    )

  // An out-of-order private creator is allowed when the private data element's
  // VR wasn't inferred, i.e. the transfer syntax is explicit VR
  let assert Ok(location) =
    read_data_element_with_vr(
      p10_location.new(),
      private_tag,
      value_representation.ShortText,
      False,
      False,
    )
  let assert Ok(_) =
    read_data_element_with_vr(
      location,
      private_creator,
      value_representation.LongString,
      False,
      False,
    )
}

pub fn out_of_order_pixel_representation_is_only_allowed_when_unused_test() {
  // An out-of-order pixel representation following a data element that had a
  // US/SS VR inferred for it is an error
  let assert Ok(location) =
    read_data_element_with_vr(
      p10_location.new(),
      dictionary.largest_image_pixel_value.tag,
      value_representation.UnsignedShort,
      True,
      False,
    )
  let assert Error(Nil) =
    read_data_element(location, dictionary.pixel_representation)

  // An out-of-order pixel representation is allowed when the VR of the
  // preceding data element wasn't inferred
  let assert Ok(location) =
    read_data_element_with_vr(
      p10_location.new(),
      dictionary.largest_image_pixel_value.tag,
      value_representation.UnsignedShort,
      False,
      False,
    )
  let assert Ok(_) =
    read_data_element(location, dictionary.pixel_representation)
}

pub fn out_of_order_bits_allocated_is_only_allowed_when_unused_test() {
  // An out-of-order bits allocated following pixel data in a big endian
  // transfer syntax is an error because it determines the word size used when
  // swapping endianness
  let assert Ok(location) = read_big_endian_pixel_data(p10_location.new())
  let assert Error(Nil) = read_data_element(location, dictionary.bits_allocated)

  // An out-of-order bits allocated following pixel data in a little endian
  // transfer syntax is allowed
  let assert Ok(location) =
    read_data_element_with_vr(
      p10_location.new(),
      dictionary.pixel_data.tag,
      value_representation.OtherWordString,
      False,
      False,
    )
  let assert Ok(_) = read_data_element(location, dictionary.bits_allocated)
}
