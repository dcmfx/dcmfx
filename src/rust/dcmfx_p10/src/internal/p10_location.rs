//! A location used by a DICOM P10 read context to track where in the hierarchy
//! of sequences and items the DICOM P10 read is up to, along with associated
//! data required to correctly interpret incoming data elements at the current
//! location.
//!
//! The following are tracked in the location during a DICOM P10 read:
//!
//! 1. The end offset of defined-length sequences and items that need to have a
//!    delimiter emitted. This allows defined lengths to be changed to undefined
//!    lengths.
//!
//! 2. The active specific character set that should be used to decode string
//!    values that aren't in UTF-8. This is set/updated by the *'(0008,0005)
//!    SpecificCharacterSet'* tag, most commonly in the root data set, but can
//!    be overridden in a sequence item.
//!
//! 3. The value of data elements that have been read and which are needed in
//!    order to determine the correct VR of subsequent data elements when the
//!    transfer syntax is 'Implicit VR Little Endian'.
//!
//!    E.g. the *'(0028,0106) Smallest Image Pixel Value'* data element uses
//!    either the `UnsignedShort` or `SignedShort` VR, and determining which
//!    requires the *'(0028,0103) Pixel Representation'* data element's value.
//!
//! 4. Which clarifying data elements described in (3) have been used in the
//!    interpretation of data element values, and where their values were
//!    defined. This allows detection of a clarifying data element appearing
//!    after data elements that it applies to, which isn't compatible with
//!    stream-based reading of DICOM P10 data.

#[cfg(feature = "std")]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(not(feature = "std"))]
use alloc::{
  collections::{BTreeMap, BTreeSet},
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use bytes::Bytes;

use dcmfx_character_set::{self, SpecificCharacterSet, StringType};
use dcmfx_core::{DataElementTag, ValueRepresentation, dictionary, utils};

use crate::{P10Error, P10Token, internal::value_length::ValueLength};

/// A P10 location is a list of location entries, with the current/most recently
/// added one at the end of the vector.
///
#[derive(Debug)]
pub struct P10Location {
  entries: Vec<LocationEntry>,
}

/// An entry in a P10 location. A root data set entry always appears exactly
/// once at the start, and can then be followed by sequences, each containing
/// nested lists of items that can themselves contain sequences.
///
#[derive(Debug)]
enum LocationEntry {
  RootDataSet {
    clarifying_data_elements: ClarifyingDataElements,
    locally_defined_clarifying_data_elements: ClarifyingDataElementSet,
    used_clarifying_data_elements: ClarifyingDataElementSet,
    last_data_element_tag: DataElementTag,
  },
  Sequence {
    tag: DataElementTag,
    is_implicit_vr: bool,
    ends_at: Option<u64>,
    item_count: usize,
  },
  Item {
    clarifying_data_elements: ClarifyingDataElements,
    locally_defined_clarifying_data_elements: ClarifyingDataElementSet,
    used_clarifying_data_elements: ClarifyingDataElementSet,
    last_data_element_tag: DataElementTag,
    ends_at: Option<u64>,
  },
}

/// The data elements needed to determine VRs of some data elements when the
/// transfer syntax is 'Implicit VR Little Endian', and to decode non-UTF-8
/// string data.
///
#[derive(Clone, Debug)]
struct ClarifyingDataElements {
  specific_character_set: SpecificCharacterSet,
  bits_allocated: Option<u16>,
  pixel_representation: Option<u16>,
  waveform_bits_allocated: Option<u16>,
  private_creators: BTreeMap<DataElementTag, String>,
}

/// Returns whether a data element tag is for a clarifying data element that
/// needs to be materialized by the read process and added to the location.
///
pub fn is_clarifying_data_element(tag: DataElementTag) -> bool {
  tag == dictionary::SPECIFIC_CHARACTER_SET.tag
    || tag == dictionary::BITS_ALLOCATED.tag
    || tag == dictionary::PIXEL_REPRESENTATION.tag
    || tag == dictionary::WAVEFORM_BITS_ALLOCATED.tag
    || tag.is_private_creator()
}

impl ClarifyingDataElements {
  fn private_creator_for_tag(&self, tag: DataElementTag) -> Option<&String> {
    if !tag.is_private() {
      return None;
    }

    self.private_creators.get(&private_creator_tag_for_tag(tag))
  }
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
#[derive(Debug, Default)]
struct ClarifyingDataElementSet {
  specific_character_set: bool,
  bits_allocated: bool,
  pixel_representation: bool,
  waveform_bits_allocated: bool,
  private_creators: BTreeSet<DataElementTag>,
}

impl ClarifyingDataElementSet {
  /// Adds a clarifying data element to this set.
  ///
  fn insert(&mut self, tag: DataElementTag) {
    if tag == dictionary::SPECIFIC_CHARACTER_SET.tag {
      self.specific_character_set = true;
    } else if tag == dictionary::BITS_ALLOCATED.tag {
      self.bits_allocated = true;
    } else if tag == dictionary::PIXEL_REPRESENTATION.tag {
      self.pixel_representation = true;
    } else if tag == dictionary::WAVEFORM_BITS_ALLOCATED.tag {
      self.waveform_bits_allocated = true;
    } else if tag.is_private_creator() {
      self.private_creators.insert(tag);
    }
  }

  /// Returns whether this set contains the specified clarifying data element.
  ///
  fn contains(&self, tag: DataElementTag) -> bool {
    if tag == dictionary::SPECIFIC_CHARACTER_SET.tag {
      self.specific_character_set
    } else if tag == dictionary::BITS_ALLOCATED.tag {
      self.bits_allocated
    } else if tag == dictionary::PIXEL_REPRESENTATION.tag {
      self.pixel_representation
    } else if tag == dictionary::WAVEFORM_BITS_ALLOCATED.tag {
      self.waveform_bits_allocated
    } else {
      self.private_creators.contains(&tag)
    }
  }
}

impl Default for ClarifyingDataElements {
  /// Returns the default/initial value for the clarifying data elements.
  ///
  fn default() -> Self {
    Self {
      specific_character_set: SpecificCharacterSet::from_string("ISO_IR 6")
        .unwrap(),
      bits_allocated: None,
      pixel_representation: None,
      waveform_bits_allocated: None,
      private_creators: BTreeMap::new(),
    }
  }
}

/// Returns the tag of the *'(gggg,00xx) Private Creator'* data element that
/// defines the private block containing the specified private tag.
///
fn private_creator_tag_for_tag(tag: DataElementTag) -> DataElementTag {
  DataElementTag::new(tag.group, tag.element >> 8)
}

/// Returns whether the VR of a data element is determined by the value of the
/// *'(0028,0103) PixelRepresentation'* data element, i.e. whether it uses either
/// the `UnsignedShort` or `SignedShort` VR.
///
fn is_pixel_representation_dependent(tag: DataElementTag) -> bool {
  tag == dictionary::ZERO_VELOCITY_PIXEL_VALUE.tag
    || tag == dictionary::MAPPED_PIXEL_VALUE.tag
    || tag == dictionary::SMALLEST_VALID_PIXEL_VALUE.tag
    || tag == dictionary::LARGEST_VALID_PIXEL_VALUE.tag
    || tag == dictionary::SMALLEST_IMAGE_PIXEL_VALUE.tag
    || tag == dictionary::LARGEST_IMAGE_PIXEL_VALUE.tag
    || tag == dictionary::SMALLEST_PIXEL_VALUE_IN_SERIES.tag
    || tag == dictionary::LARGEST_PIXEL_VALUE_IN_SERIES.tag
    || tag == dictionary::SMALLEST_IMAGE_PIXEL_VALUE_IN_PLANE.tag
    || tag == dictionary::LARGEST_IMAGE_PIXEL_VALUE_IN_PLANE.tag
    || tag == dictionary::PIXEL_PADDING_VALUE.tag
    || tag == dictionary::PIXEL_PADDING_RANGE_LIMIT.tag
    || tag == dictionary::RED_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag
    || tag == dictionary::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag
    || tag == dictionary::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag
    || tag == dictionary::LUT_DESCRIPTOR.tag
    || tag == dictionary::REAL_WORLD_VALUE_LAST_VALUE_MAPPED.tag
    || tag == dictionary::REAL_WORLD_VALUE_FIRST_VALUE_MAPPED.tag
    || tag == dictionary::HISTOGRAM_FIRST_BIN_VALUE.tag
    || tag == dictionary::HISTOGRAM_LAST_BIN_VALUE.tag
}

impl P10Location {
  /// Creates a new P10 location with an initial entry for the root data set.
  ///
  pub fn new() -> Self {
    Self {
      entries: vec![LocationEntry::RootDataSet {
        clarifying_data_elements: ClarifyingDataElements::default(),
        locally_defined_clarifying_data_elements:
          ClarifyingDataElementSet::default(),
        used_clarifying_data_elements: ClarifyingDataElementSet::default(),
        last_data_element_tag: DataElementTag::ZERO,
      }],
    }
  }

  /// Records that a data element with the specified tag and VR is being read at
  /// the current P10 location, and checks that its ordering relative to the data
  /// elements that precede it doesn't affect their interpretation.
  ///
  /// In DICOM P10 data, data elements in a data set and sequence item must
  /// appear in ascending order. This is relevant when reading DICOM P10 data in
  /// a streaming fashion because lower numbered data elements are sometimes used
  /// in the interpretation of higher numbered data elements.
  ///
  /// However, the only data elements able to alter the interpretation of others
  /// are the clarifying data elements, so an error is returned only when a
  /// clarifying data element appears after a data element that it applies to.
  /// Other out-of-order data elements are read without error because doing so
  /// can't affect the result.
  ///
  /// `is_vr_inferred` specifies whether the data element's VR was inferred by
  /// [`Self::infer_vr_for_tag()`], and `is_big_endian` specifies whether its
  /// value bytes have their endianness swapped by [`Self::swap_endianness()`].
  ///
  pub fn check_data_element_ordering(
    &mut self,
    tag: DataElementTag,
    vr: ValueRepresentation,
    is_vr_inferred: bool,
    is_big_endian: bool,
  ) -> Result<(), ()> {
    match self.entries.last_mut() {
      Some(LocationEntry::RootDataSet {
        used_clarifying_data_elements,
        last_data_element_tag,
        ..
      })
      | Some(LocationEntry::Item {
        used_clarifying_data_elements,
        last_data_element_tag,
        ..
      }) => {
        if tag > *last_data_element_tag {
          *last_data_element_tag = tag;
        } else if is_clarifying_data_element(tag)
          && used_clarifying_data_elements.contains(tag)
        {
          return Err(());
        }
      }

      Some(LocationEntry::Sequence { .. }) => return Ok(()),

      None => return Err(()),
    }

    self.record_clarifying_data_element_uses(
      tag,
      vr,
      is_vr_inferred,
      is_big_endian,
    );

    Ok(())
  }

  /// Records the clarifying data elements used in the interpretation of a data
  /// element with the specified tag and VR.
  ///
  /// `is_vr_inferred` specifies whether the data element's VR was inferred by
  /// [`Self::infer_vr_for_tag()`], and `is_big_endian` specifies whether its
  /// value bytes have their endianness swapped by [`Self::swap_endianness()`].
  ///
  fn record_clarifying_data_element_uses(
    &mut self,
    tag: DataElementTag,
    vr: ValueRepresentation,
    is_vr_inferred: bool,
    is_big_endian: bool,
  ) {
    // Encoded string values, with the exception of private creators, are
    // decoded using the active specific character set
    if vr.is_encoded_string() && !tag.is_private_creator() {
      self.record_clarifying_data_element_use(
        dictionary::SPECIFIC_CHARACTER_SET.tag,
      );
    }

    // Inferring a VR uses the pixel representation for the data elements that
    // are either US or SS, and uses the private creator for private data
    // elements
    if is_vr_inferred {
      if is_pixel_representation_dependent(tag) {
        self.record_clarifying_data_element_use(
          dictionary::PIXEL_REPRESENTATION.tag,
        );
      }

      if tag.is_private() && !tag.is_private_creator() {
        self
          .record_clarifying_data_element_use(private_creator_tag_for_tag(tag));
      }
    }

    // Swapping the endianness of pixel data and waveform data uses their bits
    // allocated value to determine the word size
    if is_big_endian && vr == ValueRepresentation::OtherWordString {
      if tag == dictionary::PIXEL_DATA.tag {
        self.record_clarifying_data_element_use(dictionary::BITS_ALLOCATED.tag);
      } else if tag == dictionary::WAVEFORM_DATA.tag {
        self.record_clarifying_data_element_use(
          dictionary::WAVEFORM_BITS_ALLOCATED.tag,
        );
      }
    }
  }

  /// Records that the specified clarifying data element has been used in the
  /// interpretation of a data element at the current P10 location.
  ///
  /// The use is recorded on the current data set or item, as well as on each
  /// enclosing data set or item up to and including the one that defines the
  /// clarifying data element's value. This is because clarifying data elements
  /// are inherited by nested locations but don't propagate back out of them,
  /// e.g. a clarifying data element defined in an item applies only inside that
  /// item, so its use there is unaffected by the same clarifying data element
  /// subsequently appearing out of order in an enclosing data set or item.
  ///
  fn record_clarifying_data_element_use(&mut self, tag: DataElementTag) {
    for entry in self.entries.iter_mut().rev() {
      let (
        locally_defined_clarifying_data_elements,
        used_clarifying_data_elements,
      ) = match entry {
        LocationEntry::RootDataSet {
          locally_defined_clarifying_data_elements,
          used_clarifying_data_elements,
          ..
        }
        | LocationEntry::Item {
          locally_defined_clarifying_data_elements,
          used_clarifying_data_elements,
          ..
        } => (
          locally_defined_clarifying_data_elements,
          used_clarifying_data_elements,
        ),

        LocationEntry::Sequence { .. } => continue,
      };

      used_clarifying_data_elements.insert(tag);

      if locally_defined_clarifying_data_elements.contains(tag) {
        return;
      }
    }
  }

  /// Records that the specified clarifying data element is defined at the
  /// current P10 location, i.e. that its value overrides the value inherited
  /// from any enclosing data set or item.
  ///
  fn record_clarifying_data_element_definition(&mut self, tag: DataElementTag) {
    for entry in self.entries.iter_mut().rev() {
      match entry {
        LocationEntry::RootDataSet {
          locally_defined_clarifying_data_elements,
          ..
        }
        | LocationEntry::Item {
          locally_defined_clarifying_data_elements,
          ..
        } => {
          locally_defined_clarifying_data_elements.insert(tag);
          return;
        }

        LocationEntry::Sequence { .. } => (),
      }
    }
  }

  /// Returns whether there is a sequence in the location that has forced the
  /// use of the 'Implicit VR Little Endian' transfer syntax. This occurs when
  /// there is an explicit VR of `UN` (Unknown) that has an undefined length.
  ///
  /// Ref: DICOM Correction Proposal CP-246.
  ///
  pub fn is_implicit_vr_forced(&self) -> bool {
    self.entries.iter().any(|l| {
      matches!(
        l,
        LocationEntry::Sequence {
          is_implicit_vr: true,
          ..
        }
      )
    })
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
    &self,
    tag: DataElementTag,
    vr: ValueRepresentation,
    data: &mut [u8],
  ) {
    let vr = if vr == ValueRepresentation::OtherWordString {
      let bits_allocated = if tag == dictionary::PIXEL_DATA.tag {
        self.active_clarifying_data_elements().bits_allocated
      } else if tag == dictionary::WAVEFORM_DATA.tag {
        self
          .active_clarifying_data_elements()
          .waveform_bits_allocated
      } else if tag == dictionary::CHANNEL_MINIMUM_VALUE.tag
        || tag == dictionary::CHANNEL_MAXIMUM_VALUE.tag
        || tag == dictionary::WAVEFORM_PADDING_VALUE.tag
      {
        // These data elements hold a single value in the waveform data's
        // sample encoding, so their length gives the word size. Ref: PS3.5
        // 8.3.
        match data.len() {
          4 => Some(32),
          8 => Some(64),
          _ => None,
        }
      } else {
        None
      };

      if bits_allocated == Some(32) {
        ValueRepresentation::UnsignedLong
      } else if bits_allocated == Some(64) {
        ValueRepresentation::UnsignedVeryLong
      } else {
        vr
      }
    } else {
      vr
    };

    vr.swap_endianness(data);
  }

  /// Returns the next delimiter token for a location. This checks the `ends_at`
  /// value of the entry at the head of the location to see if the bytes read
  /// has met or exceeded it, and if it has then the relevant delimiter token is
  /// returned.
  ///
  /// This is token of the conversion of defined-length sequences and items to
  /// use undefined lengths.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn next_delimiter_token(
    &mut self,
    bytes_read: u64,
  ) -> Result<P10Token, ()> {
    match self.entries.last() {
      Some(LocationEntry::Sequence {
        tag,
        ends_at: Some(ends_at),
        ..
      }) if *ends_at <= bytes_read => {
        let tag = *tag;
        self.entries.pop();
        Ok(P10Token::SequenceDelimiter { tag })
      }

      Some(LocationEntry::Item {
        ends_at: Some(ends_at),
        ..
      }) if *ends_at <= bytes_read => {
        self.entries.pop();
        Ok(P10Token::SequenceItemDelimiter)
      }

      _ => Err(()),
    }
  }

  /// Returns all pending delimiter tokens for a location, regardless of whether
  /// their `ends_at` offset has been reached.
  ///
  pub fn pending_delimiter_tokens(&self) -> Vec<P10Token> {
    self
      .entries
      .iter()
      .rev()
      .map(|entry| match entry {
        LocationEntry::Sequence { tag, .. } => {
          P10Token::SequenceDelimiter { tag: *tag }
        }
        LocationEntry::Item { .. } => P10Token::SequenceItemDelimiter,
        LocationEntry::RootDataSet { .. } => P10Token::End,
      })
      .collect()
  }

  /// Adds a new sequence to a P10 location.
  ///
  pub fn add_sequence(
    &mut self,
    tag: DataElementTag,
    is_implicit_vr: bool,
    ends_at: Option<u64>,
  ) -> Result<(), String> {
    match self.entries.last() {
      Some(LocationEntry::RootDataSet { .. })
      | Some(LocationEntry::Item { .. }) => {
        self.entries.push(LocationEntry::Sequence {
          tag,
          is_implicit_vr,
          ends_at,
          item_count: 0,
        });

        Ok(())
      }

      _ => {
        let private_creator = self
          .active_clarifying_data_elements()
          .private_creator_for_tag(tag);

        Err(format!(
          "Sequence data element '{}' encountered outside of the root data set \
            or an item",
          dictionary::tag_with_name(tag, private_creator.map(|x| x.as_str()))
        ))
      }
    }
  }

  /// Ends the current sequence for a P10 location.
  ///
  pub fn end_sequence(&mut self) -> Result<DataElementTag, String> {
    match self.entries.last() {
      Some(LocationEntry::Sequence { tag, .. }) => {
        let tag = *tag;
        self.entries.pop();
        Ok(tag)
      }

      _ => {
        Err("Sequence delimiter encountered outside of a sequence".to_string())
      }
    }
  }

  /// Returns the number of items that have been added to the current sequence.
  ///
  pub fn sequence_item_count(&self) -> Result<usize, ()> {
    match self.entries.as_slice() {
      [LocationEntry::Sequence { item_count, .. }, ..] => Ok(*item_count),
      _ => Err(()),
    }
  }

  /// Adds a new item to a P10 location. The index of the new item is returned.
  ///
  pub fn add_item(
    &mut self,
    ends_at: Option<u64>,
    length: ValueLength,
  ) -> Result<usize, String> {
    match self.entries.last_mut() {
      // Carry across the current clarifying data elements as the initial state
      // for the new item
      Some(LocationEntry::Sequence { item_count, .. }) => {
        let index = *item_count;

        *item_count += 1;

        self.entries.push(LocationEntry::Item {
          clarifying_data_elements: self
            .active_clarifying_data_elements()
            .clone(),
          locally_defined_clarifying_data_elements:
            ClarifyingDataElementSet::default(),
          used_clarifying_data_elements: ClarifyingDataElementSet::default(),
          last_data_element_tag: DataElementTag::ZERO,
          ends_at,
        });

        Ok(index)
      }

      _ => Err(format!(
        "Item encountered outside of a sequence, length: {length}",
      )),
    }
  }

  /// Ends the current item for a P10 location.
  ///
  pub fn end_item(&mut self) -> Result<(), String> {
    match self.entries.last() {
      Some(LocationEntry::Item { .. }) => {
        self.entries.pop();
        Ok(())
      }

      _ => Err("Item delimiter encountered outside of an item".to_string()),
    }
  }

  /// Returns the clarifying data elements that currently apply to any new data
  /// elements.
  ///
  fn active_clarifying_data_elements(&self) -> &ClarifyingDataElements {
    for entry in self.entries.iter().rev() {
      match entry {
        LocationEntry::RootDataSet {
          clarifying_data_elements,
          ..
        }
        | LocationEntry::Item {
          clarifying_data_elements,
          ..
        } => return clarifying_data_elements,

        _ => (),
      }
    }

    unreachable!();
  }

  /// Returns the clarifying data elements that currently apply to any new data
  /// elements.
  ///
  fn active_clarifying_data_elements_mut(
    &mut self,
  ) -> &mut ClarifyingDataElements {
    for entry in self.entries.iter_mut().rev() {
      match entry {
        LocationEntry::RootDataSet {
          clarifying_data_elements,
          ..
        }
        | LocationEntry::Item {
          clarifying_data_elements,
          ..
        } => return clarifying_data_elements,

        _ => (),
      }
    }

    unreachable!();
  }

  /// Adds a clarifying data element to a location.
  ///
  /// The only time that the value bytes are altered is the *'(0008,0005)
  /// SpecificCharacterSet'* data element.
  ///
  pub fn add_clarifying_data_element(
    &mut self,
    tag: DataElementTag,
    vr: ValueRepresentation,
    value_bytes: &mut Bytes,
  ) -> Result<(), P10Error> {
    if tag == dictionary::SPECIFIC_CHARACTER_SET.tag {
      self
        .update_specific_character_set_clarifying_data_element(value_bytes)?;
    } else if vr == ValueRepresentation::UnsignedShort {
      let value_bytes: &[u8] = value_bytes;
      if let Ok(u) = TryInto::<[u8; 2]>::try_into(value_bytes) {
        self.update_unsigned_short_clarifying_data_element(
          tag,
          u16::from_le_bytes(u),
        );
      }
    } else if vr == ValueRepresentation::LongString && tag.is_private_creator()
    {
      self.update_private_creator_clarifying_data_element(value_bytes, tag);
    }

    Ok(())
  }

  fn update_specific_character_set_clarifying_data_element(
    &mut self,
    value_bytes: &mut Bytes,
  ) -> Result<(), P10Error> {
    let specific_character_set =
      core::str::from_utf8(value_bytes).map_err(|_| {
        P10Error::SpecificCharacterSetInvalid {
          specific_character_set: utils::inspect_u8_slice(value_bytes, 64),
          details: "Invalid UTF-8".to_string(),
        }
      })?;

    // Set specific character set in current location
    self
      .active_clarifying_data_elements_mut()
      .specific_character_set = SpecificCharacterSet::from_string(
      specific_character_set,
    )
    .map_err(|details| P10Error::SpecificCharacterSetInvalid {
      specific_character_set: specific_character_set.chars().take(64).collect(),
      details,
    })?;

    self.record_clarifying_data_element_definition(
      dictionary::SPECIFIC_CHARACTER_SET.tag,
    );

    *value_bytes = b"ISO_IR 192".to_vec().into();

    Ok(())
  }

  fn update_unsigned_short_clarifying_data_element(
    &mut self,
    tag: DataElementTag,
    value: u16,
  ) {
    let clarifying_data_elements = self.active_clarifying_data_elements_mut();

    if tag == dictionary::BITS_ALLOCATED.tag {
      clarifying_data_elements.bits_allocated = Some(value);
    } else if tag == dictionary::PIXEL_REPRESENTATION.tag {
      clarifying_data_elements.pixel_representation = Some(value);
    } else if tag == dictionary::WAVEFORM_BITS_ALLOCATED.tag {
      clarifying_data_elements.waveform_bits_allocated = Some(value);
    } else {
      return;
    }

    self.record_clarifying_data_element_definition(tag);
  }

  fn update_private_creator_clarifying_data_element(
    &mut self,
    value_bytes: &[u8],
    tag: DataElementTag,
  ) {
    let private_creator = match core::str::from_utf8(value_bytes) {
      Ok(value) => value.trim_end_matches(' ').to_string(),
      Err(_) => return,
    };

    let clarifying_data_elements = self.active_clarifying_data_elements_mut();

    clarifying_data_elements
      .private_creators
      .insert(tag, private_creator);

    self.record_clarifying_data_element_definition(tag);
  }

  /// Returns whether the current specific character set is UTF-8.
  ///
  pub fn is_specific_character_set_utf8(&self) -> bool {
    self
      .active_clarifying_data_elements()
      .specific_character_set
      .is_utf8()
  }

  /// Decodes encoded string bytes using the currently active specific character
  /// set and returns their UTF-8 bytes.
  ///
  pub fn decode_string_bytes(
    &self,
    vr: ValueRepresentation,
    value_bytes: &[u8],
  ) -> Vec<u8> {
    let charset = &self
      .active_clarifying_data_elements()
      .specific_character_set;

    // Determine the type of the string to be decoded based on the VR. See the
    // `StringType` enum for further details.
    let string_type = match vr {
      ValueRepresentation::PersonName => StringType::PersonName,

      ValueRepresentation::LongString
      | ValueRepresentation::ShortString
      | ValueRepresentation::UnlimitedCharacters => StringType::MultiValue,

      _ => StringType::SingleValue,
    };

    let mut bytes = charset.decode_bytes(value_bytes, string_type).into_bytes();

    vr.pad_bytes_to_even_length(&mut bytes);

    bytes
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
    &self,
    tag: DataElementTag,
  ) -> Result<ValueRepresentation, DataElementTag> {
    let clarifying_data_elements = self.active_clarifying_data_elements();

    let private_creator = clarifying_data_elements.private_creator_for_tag(tag);

    let allowed_vrs =
      match dictionary::find(tag, private_creator.map(|x| x.as_str())) {
        Ok(item) => item.vrs,
        Err(_) => &[],
      };

    match allowed_vrs {
      [vr] => Ok(*vr),

      // For '(7FE0,0010) Pixel Data', OB is not usable when in an implicit VR
      // transfer syntax. Ref: PS3.5 8.2.
      [
        ValueRepresentation::OtherByteString,
        ValueRepresentation::OtherWordString,
      ] if tag == dictionary::PIXEL_DATA.tag => {
        Ok(ValueRepresentation::OtherWordString)
      }

      // Use '(0028,0103) PixelRepresentation' to determine a US/SS VR on
      // relevant values
      [
        ValueRepresentation::UnsignedShort,
        ValueRepresentation::SignedShort,
      ] if is_pixel_representation_dependent(tag) => {
        match clarifying_data_elements.pixel_representation {
          Some(0) => Ok(ValueRepresentation::UnsignedShort),
          Some(1) => Ok(ValueRepresentation::SignedShort),
          _ => Err(dictionary::PIXEL_REPRESENTATION.tag),
        }
      }

      // For '(5400,1010) Waveform Data' and the other waveform data elements
      // that store values in its sample encoding, OB is not usable when in an
      // implicit VR transfer syntax. Ref: PS3.5 8.3.
      [
        ValueRepresentation::OtherByteString,
        ValueRepresentation::OtherWordString,
      ] if tag == dictionary::CHANNEL_MINIMUM_VALUE.tag
        || tag == dictionary::CHANNEL_MAXIMUM_VALUE.tag
        || tag == dictionary::WAVEFORM_PADDING_VALUE.tag
        || tag == dictionary::WAVEFORM_DATA.tag =>
      {
        Ok(ValueRepresentation::OtherWordString)
      }

      // The VR for '(0028,3006) LUTData' doesn't need to be determined because
      // the raw binary representation of both VRs is the same.
      // `OtherWordString` is chosen because it's closer to being correct in the
      // case of the LUT containing tightly packed 8-bit data, which is allowed
      // by the spec (Ref: PS3.3 C.11.1.1.1), even though there is no VR that
      // correctly expresses this, i.e. OB is not a valid VR for LUTData.
      [
        ValueRepresentation::UnsignedShort,
        ValueRepresentation::OtherWordString,
      ] if tag == dictionary::LUT_DATA.tag => {
        Ok(ValueRepresentation::OtherWordString)
      }

      // The VR for '(60xx,3000) Overlay Data' doesn't need to be determined as
      // when the transfer syntax is 'Implicit VR Little Endian' it is always
      // OW. Ref: PS3.5 8.1.2.
      [
        ValueRepresentation::OtherByteString,
        ValueRepresentation::OtherWordString,
      ] if tag.group >= 0x6000
        && tag.group <= 0x60FF
        && tag.element == 0x3000 =>
      {
        Ok(ValueRepresentation::OtherWordString)
      }

      // The VR couldn't be determined
      _ => Ok(ValueRepresentation::Unknown),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Reads a data element at the current location, with its VR not inferred and
  /// the transfer syntax being little endian.
  ///
  fn read_data_element(
    location: &mut P10Location,
    item: &dictionary::Item,
  ) -> Result<(), ()> {
    location.check_data_element_ordering(item.tag, item.vrs[0], false, false)
  }

  /// Reads a clarifying data element at the current location, defining its value
  /// there.
  ///
  fn read_clarifying_data_element(
    location: &mut P10Location,
    item: &dictionary::Item,
    value_bytes: &[u8],
  ) -> Result<(), ()> {
    read_data_element(location, item)?;

    let mut value_bytes: Bytes = value_bytes.to_vec().into();
    location
      .add_clarifying_data_element(item.tag, item.vrs[0], &mut value_bytes)
      .unwrap();

    Ok(())
  }

  /// Reads a sequence containing a single item at the current location, calling
  /// the passed function to read the item's content.
  ///
  fn read_sequence_item(
    location: &mut P10Location,
    item: &dictionary::Item,
    read_item_content: impl FnOnce(&mut P10Location),
  ) {
    assert_eq!(read_data_element(location, item), Ok(()));
    location.add_sequence(item.tag, false, None).unwrap();
    location.add_item(None, ValueLength::Undefined).unwrap();

    read_item_content(location);

    location.end_item().unwrap();
    location.end_sequence().unwrap();
  }

  #[test]
  fn ordered_data_elements_are_allowed() {
    let mut location = P10Location::new();

    assert_eq!(
      read_data_element(&mut location, &dictionary::SPECIFIC_CHARACTER_SET),
      Ok(())
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::PATIENT_NAME),
      Ok(())
    );
  }

  #[test]
  fn out_of_order_data_elements_that_arent_clarifying_are_allowed() {
    let mut location = P10Location::new();

    assert_eq!(
      read_data_element(&mut location, &dictionary::PATIENT_NAME),
      Ok(())
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::SERIES_DESCRIPTION),
      Ok(())
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::PATIENT_NAME),
      Ok(())
    );
  }

  #[test]
  fn out_of_order_specific_character_set_is_only_allowed_when_unused() {
    // An out-of-order specific character set following a data element that
    // doesn't use an encoded string VR is allowed
    let mut location = P10Location::new();
    assert_eq!(
      read_data_element(&mut location, &dictionary::SOP_INSTANCE_UID),
      Ok(())
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::SPECIFIC_CHARACTER_SET),
      Ok(())
    );

    // An out-of-order specific character set following a data element that uses
    // an encoded string VR is an error
    let mut location = P10Location::new();
    assert_eq!(
      read_data_element(&mut location, &dictionary::PATIENT_NAME),
      Ok(())
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::SPECIFIC_CHARACTER_SET),
      Err(())
    );
  }

  #[test]
  fn out_of_order_specific_character_set_used_in_an_item_is_an_error() {
    let mut location = P10Location::new();

    // Read a sequence containing an item that has a data element using an
    // encoded string VR
    read_sequence_item(
      &mut location,
      &dictionary::REFERENCED_STUDY_SEQUENCE,
      |location| {
        assert_eq!(
          read_data_element(location, &dictionary::PATIENT_NAME),
          Ok(())
        );
      },
    );

    // The specific character set used by the item's data element is inherited
    // from the root data set, so it appearing out of order in the root data set
    // is an error
    assert_eq!(
      read_data_element(&mut location, &dictionary::SPECIFIC_CHARACTER_SET),
      Err(())
    );
  }

  #[test]
  fn clarifying_data_element_defined_in_an_item_doesnt_apply_outside_it() {
    // Read a sequence containing an item that uses the bits allocated value
    // inherited from the root data set. Bits allocated then appearing out of
    // order in the root data set is an error because it alters how the item's
    // pixel data should have been read.
    let mut location = P10Location::new();
    read_sequence_item(
      &mut location,
      &dictionary::SHARED_FUNCTIONAL_GROUPS_SEQUENCE,
      |location| {
        assert_eq!(
          location.check_data_element_ordering(
            dictionary::PIXEL_DATA.tag,
            ValueRepresentation::OtherWordString,
            false,
            true
          ),
          Ok(())
        );
      },
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::BITS_ALLOCATED),
      Err(())
    );

    // Read a sequence containing an item that defines its own bits allocated
    // value before using it. Bits allocated then appearing out of order in the
    // root data set is allowed because the item's pixel data didn't depend on
    // the root data set's value.
    let mut location = P10Location::new();
    read_sequence_item(
      &mut location,
      &dictionary::SHARED_FUNCTIONAL_GROUPS_SEQUENCE,
      |location| {
        assert_eq!(
          read_clarifying_data_element(
            location,
            &dictionary::BITS_ALLOCATED,
            &32u16.to_le_bytes()
          ),
          Ok(())
        );
        assert_eq!(
          location.check_data_element_ordering(
            dictionary::PIXEL_DATA.tag,
            ValueRepresentation::OtherWordString,
            false,
            true
          ),
          Ok(())
        );
      },
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::BITS_ALLOCATED),
      Ok(())
    );
  }

  #[test]
  fn clarifying_data_element_redefined_in_an_item_doesnt_apply_outside_it() {
    // Read a root data set that defines bits allocated, followed by a sequence
    // containing an item that uses the inherited value. Bits allocated then
    // appearing out of order in the root data set is an error because it alters
    // the value that the item's pixel data was read with.
    let mut location = P10Location::new();
    assert_eq!(
      read_clarifying_data_element(
        &mut location,
        &dictionary::BITS_ALLOCATED,
        &16u16.to_le_bytes()
      ),
      Ok(())
    );
    read_sequence_item(
      &mut location,
      &dictionary::SHARED_FUNCTIONAL_GROUPS_SEQUENCE,
      |location| {
        assert_eq!(
          location.check_data_element_ordering(
            dictionary::PIXEL_DATA.tag,
            ValueRepresentation::OtherWordString,
            false,
            true
          ),
          Ok(())
        );
      },
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::BITS_ALLOCATED),
      Err(())
    );

    // Read the same, but with the item redefining bits allocated before using
    // it. Bits allocated then appearing out of order in the root data set is
    // allowed because the item's pixel data used the item's own value.
    let mut location = P10Location::new();
    assert_eq!(
      read_clarifying_data_element(
        &mut location,
        &dictionary::BITS_ALLOCATED,
        &16u16.to_le_bytes()
      ),
      Ok(())
    );
    read_sequence_item(
      &mut location,
      &dictionary::SHARED_FUNCTIONAL_GROUPS_SEQUENCE,
      |location| {
        assert_eq!(
          read_clarifying_data_element(
            location,
            &dictionary::BITS_ALLOCATED,
            &32u16.to_le_bytes()
          ),
          Ok(())
        );
        assert_eq!(
          location.check_data_element_ordering(
            dictionary::PIXEL_DATA.tag,
            ValueRepresentation::OtherWordString,
            false,
            true
          ),
          Ok(())
        );
      },
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::BITS_ALLOCATED),
      Ok(())
    );
  }

  #[test]
  fn out_of_order_clarifying_data_element_redefined_at_a_location_is_an_error()
  {
    let mut location = P10Location::new();

    // Define bits allocated and then use it when reading big endian pixel data
    assert_eq!(
      read_clarifying_data_element(
        &mut location,
        &dictionary::BITS_ALLOCATED,
        &32u16.to_le_bytes()
      ),
      Ok(())
    );
    assert_eq!(
      location.check_data_element_ordering(
        dictionary::PIXEL_DATA.tag,
        ValueRepresentation::OtherWordString,
        false,
        true
      ),
      Ok(())
    );

    // A second out-of-order bits allocated is an error because the pixel data
    // was read using the first one's value
    assert_eq!(
      read_data_element(&mut location, &dictionary::BITS_ALLOCATED),
      Err(())
    );
  }

  #[test]
  fn out_of_order_private_creator_is_only_allowed_when_unused() {
    let private_creator = DataElementTag::new(0x0009, 0x0010);
    let private_tag = DataElementTag::new(0x0009, 0x1001);

    // An out-of-order private creator following a private data element in its
    // block that had its VR inferred is an error
    let mut location = P10Location::new();
    assert_eq!(
      location.check_data_element_ordering(
        private_tag,
        ValueRepresentation::Unknown,
        true,
        false
      ),
      Ok(())
    );
    assert_eq!(
      location.check_data_element_ordering(
        private_creator,
        ValueRepresentation::LongString,
        false,
        false
      ),
      Err(())
    );

    // An out-of-order private creator for a different private block is allowed
    let mut location = P10Location::new();
    assert_eq!(
      location.check_data_element_ordering(
        private_tag,
        ValueRepresentation::Unknown,
        true,
        false
      ),
      Ok(())
    );
    assert_eq!(
      location.check_data_element_ordering(
        DataElementTag::new(0x0009, 0x0011),
        ValueRepresentation::LongString,
        false,
        false
      ),
      Ok(())
    );

    // An out-of-order private creator is allowed when the private data element's
    // VR wasn't inferred, i.e. the transfer syntax is explicit VR
    let mut location = P10Location::new();
    assert_eq!(
      location.check_data_element_ordering(
        private_tag,
        ValueRepresentation::ShortText,
        false,
        false
      ),
      Ok(())
    );
    assert_eq!(
      location.check_data_element_ordering(
        private_creator,
        ValueRepresentation::LongString,
        false,
        false
      ),
      Ok(())
    );
  }

  #[test]
  fn out_of_order_pixel_representation_is_only_allowed_when_unused() {
    // An out-of-order pixel representation following a data element that had a
    // US/SS VR inferred for it is an error
    let mut location = P10Location::new();
    assert_eq!(
      location.check_data_element_ordering(
        dictionary::LARGEST_IMAGE_PIXEL_VALUE.tag,
        ValueRepresentation::UnsignedShort,
        true,
        false
      ),
      Ok(())
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::PIXEL_REPRESENTATION),
      Err(())
    );

    // An out-of-order pixel representation is allowed when the VR of the
    // preceding data element wasn't inferred
    let mut location = P10Location::new();
    assert_eq!(
      location.check_data_element_ordering(
        dictionary::LARGEST_IMAGE_PIXEL_VALUE.tag,
        ValueRepresentation::UnsignedShort,
        false,
        false
      ),
      Ok(())
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::PIXEL_REPRESENTATION),
      Ok(())
    );
  }

  #[test]
  fn out_of_order_bits_allocated_is_only_allowed_when_unused() {
    // An out-of-order bits allocated following pixel data in a big endian
    // transfer syntax is an error because it determines the word size used when
    // swapping endianness
    let mut location = P10Location::new();
    assert_eq!(
      location.check_data_element_ordering(
        dictionary::PIXEL_DATA.tag,
        ValueRepresentation::OtherWordString,
        false,
        true
      ),
      Ok(())
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::BITS_ALLOCATED),
      Err(())
    );

    // An out-of-order bits allocated following pixel data in a little endian
    // transfer syntax is allowed
    let mut location = P10Location::new();
    assert_eq!(
      location.check_data_element_ordering(
        dictionary::PIXEL_DATA.tag,
        ValueRepresentation::OtherWordString,
        false,
        false
      ),
      Ok(())
    );
    assert_eq!(
      read_data_element(&mut location, &dictionary::BITS_ALLOCATED),
      Ok(())
    );
  }

  #[test]
  fn swap_endianness_of_waveform_single_value_data_elements() {
    let location = P10Location::new();

    // Data elements that hold a single value in the waveform data's sample
    // encoding are swapped using the word size given by their length
    for tag in [
      dictionary::CHANNEL_MINIMUM_VALUE.tag,
      dictionary::CHANNEL_MAXIMUM_VALUE.tag,
      dictionary::WAVEFORM_PADDING_VALUE.tag,
    ] {
      let mut data = [0, 1];
      location.swap_endianness(
        tag,
        ValueRepresentation::OtherWordString,
        &mut data,
      );
      assert_eq!(data, [1, 0]);

      let mut data = [0, 1, 2, 3];
      location.swap_endianness(
        tag,
        ValueRepresentation::OtherWordString,
        &mut data,
      );
      assert_eq!(data, [3, 2, 1, 0]);

      let mut data = [0, 1, 2, 3, 4, 5, 6, 7];
      location.swap_endianness(
        tag,
        ValueRepresentation::OtherWordString,
        &mut data,
      );
      assert_eq!(data, [7, 6, 5, 4, 3, 2, 1, 0]);
    }

    // Other OW data elements are swapped as 16-bit words regardless of length
    let mut data = [0, 1, 2, 3];
    location.swap_endianness(
      DataElementTag::new(0x0008, 0x0000),
      ValueRepresentation::OtherWordString,
      &mut data,
    );
    assert_eq!(data, [1, 0, 3, 2]);
  }
}
