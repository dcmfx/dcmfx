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
//!    SpecificCharacterSet'* tag, most commonly in the root data set, but can be
//!    overridden in a sequence item.
//!
//! 3. The value of data elements that have been read and which are needed in
//!    order to determine the correct VR of subsequent data elements when the
//!    transfer syntax is 'Implicit VR Little Endian'.
//!
//!    E.g. the *'(0028,0106) Smallest Image Pixel Value'* data element uses
//!    either the `UnsignedShort` or `SignedShort` VR, and determining which
//!    requires the *'(0028,0103) Pixel Representation'* data element's value.

use std::collections::HashMap;

use dcmfx_character_set::{self, SpecificCharacterSet, StringType};
use dcmfx_core::{dictionary, utils, DataElementTag, ValueRepresentation};

use crate::{internal::value_length::ValueLength, P10Error, P10Token};

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
    last_data_element_tag: DataElementTag,
  },
  Sequence {
    is_implicit_vr: bool,
    ends_at: Option<u64>,
    item_count: usize,
  },
  Item {
    clarifying_data_elements: ClarifyingDataElements,
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
  waveform_bits_stored: Option<u16>,
  waveform_bits_allocated: Option<u16>,
  private_creators: HashMap<DataElementTag, String>,
}

/// Returns whether a data element tag is for a clarifying data element that
/// needs to be materialized by the read process and added to the location.
///
pub fn is_clarifying_data_element(tag: DataElementTag) -> bool {
  tag == dictionary::SPECIFIC_CHARACTER_SET.tag
    || tag == dictionary::BITS_ALLOCATED.tag
    || tag == dictionary::PIXEL_REPRESENTATION.tag
    || tag == dictionary::WAVEFORM_BITS_STORED.tag
    || tag == dictionary::WAVEFORM_BITS_ALLOCATED.tag
    || tag.is_private_creator()
}

impl ClarifyingDataElements {
  fn private_creator_for_tag(&self, tag: DataElementTag) -> Option<&String> {
    if !tag.is_private() {
      return None;
    }

    let private_creator_tag = DataElementTag::new(tag.group, tag.element >> 8);

    self.private_creators.get(&private_creator_tag)
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
      waveform_bits_stored: None,
      waveform_bits_allocated: None,
      private_creators: HashMap::new(),
    }
  }
}

impl P10Location {
  /// Creates a new P10 location with an initial entry for the root data set.
  ///
  pub fn new() -> Self {
    Self {
      entries: vec![LocationEntry::RootDataSet {
        clarifying_data_elements: ClarifyingDataElements::default(),
        last_data_element_tag: DataElementTag::ZERO,
      }],
    }
  }

  /// Checks that the specified data element tag is greater than the previous
  /// one at the current P10 location. In DICOM P10 data, data elements in a
  /// data set and sequence item must appear in ascending order.
  ///
  /// This is important to enforce when reading DICOM P10 data in a streaming
  /// fashion because lower numbered data elements are sometimes used in the
  /// interpretation of higher numbered data elements.
  ///
  pub fn check_data_element_ordering(
    &mut self,
    tag: DataElementTag,
  ) -> Result<(), ()> {
    match self.entries.last_mut() {
      Some(LocationEntry::RootDataSet {
        last_data_element_tag,
        ..
      })
      | Some(LocationEntry::Item {
        last_data_element_tag,
        ..
      }) if tag > *last_data_element_tag => {
        *last_data_element_tag = tag;
        Ok(())
      }

      _ => Err(()),
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
        ends_at: Some(ends_at),
        ..
      }) if *ends_at <= bytes_read => {
        self.entries.pop();
        Ok(P10Token::SequenceDelimiter)
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
        LocationEntry::Sequence { .. } => P10Token::SequenceDelimiter,
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
  pub fn end_sequence(&mut self) -> Result<(), String> {
    match self.entries.last() {
      Some(LocationEntry::Sequence { .. }) => {
        self.entries.pop();
        Ok(())
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

  /// Adds a new item to a P10 location.
  ///
  pub fn add_item(
    &mut self,
    ends_at: Option<u64>,
    length: ValueLength,
  ) -> Result<(), String> {
    match self.entries.last_mut() {
      // Carry across the current clarifying data elements as the initial state
      // for the new item
      Some(LocationEntry::Sequence { item_count, .. }) => {
        *item_count += 1;

        self.entries.push(LocationEntry::Item {
          clarifying_data_elements: self
            .active_clarifying_data_elements()
            .clone(),
          last_data_element_tag: DataElementTag::ZERO,
          ends_at,
        });

        Ok(())
      }

      _ => Err(format!(
        "Item encountered outside of a sequence, length: {} bytes",
        length
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
    value_bytes: &mut Vec<u8>,
  ) -> Result<(), P10Error> {
    if tag == dictionary::SPECIFIC_CHARACTER_SET.tag {
      self
        .update_specific_character_set_clarifying_data_element(value_bytes)?;
    } else if vr == ValueRepresentation::UnsignedShort {
      if let Ok(u) = TryInto::<[u8; 2]>::try_into(value_bytes.as_slice()) {
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
    value_bytes: &mut Vec<u8>,
  ) -> Result<(), P10Error> {
    let specific_character_set =
      std::str::from_utf8(value_bytes).map_err(|_| {
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
    .map_err(|error| P10Error::SpecificCharacterSetInvalid {
      specific_character_set: specific_character_set.to_string(),
      details: error,
    })?;

    value_bytes.clear();
    value_bytes.extend_from_slice(b"ISO_IR 192");

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
    } else if tag == dictionary::WAVEFORM_BITS_STORED.tag {
      clarifying_data_elements.waveform_bits_stored = Some(value);
    } else if tag == dictionary::WAVEFORM_BITS_ALLOCATED.tag {
      clarifying_data_elements.waveform_bits_allocated = Some(value);
    }
  }

  fn update_private_creator_clarifying_data_element(
    &mut self,
    value_bytes: &[u8],
    tag: DataElementTag,
  ) {
    let private_creator = match std::str::from_utf8(value_bytes) {
      Ok(value) => value.trim_end_matches(' ').to_string(),
      Err(_) => return,
    };

    let clarifying_data_elements = self.active_clarifying_data_elements_mut();

    clarifying_data_elements
      .private_creators
      .insert(tag, private_creator);
  }

  /// Returns whether the current specific character set is byte compatible with
  /// UTF-8.
  ///
  pub fn is_specific_character_set_utf8_compatible(&self) -> bool {
    self
      .active_clarifying_data_elements()
      .specific_character_set
      .is_utf8_compatible()
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
      [ValueRepresentation::OtherByteString, ValueRepresentation::OtherWordString]
        if tag == dictionary::PIXEL_DATA.tag =>
      {
        Ok(ValueRepresentation::OtherWordString)
      }

      // Use '(0028,0103) PixelRepresentation' to determine a US/SS VR on
      // relevant values
      [ValueRepresentation::UnsignedShort, ValueRepresentation::SignedShort]
        if tag == dictionary::ZERO_VELOCITY_PIXEL_VALUE.tag
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
          || tag
            == dictionary::RED_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag
          || tag
            == dictionary::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag
          || tag
            == dictionary::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR.tag
          || tag == dictionary::LUT_DESCRIPTOR.tag
          || tag == dictionary::REAL_WORLD_VALUE_LAST_VALUE_MAPPED.tag
          || tag == dictionary::REAL_WORLD_VALUE_FIRST_VALUE_MAPPED.tag
          || tag == dictionary::HISTOGRAM_FIRST_BIN_VALUE.tag
          || tag == dictionary::HISTOGRAM_LAST_BIN_VALUE.tag =>
      {
        match clarifying_data_elements.pixel_representation {
          Some(0) => Ok(ValueRepresentation::UnsignedShort),
          Some(1) => Ok(ValueRepresentation::SignedShort),
          _ => Err(dictionary::PIXEL_REPRESENTATION.tag),
        }
      }

      // Use '(003A,021A) WaveformBitsStored' to determine an OB/OW VR on
      // relevant values
      [ValueRepresentation::OtherByteString, ValueRepresentation::OtherWordString]
        if tag == dictionary::CHANNEL_MINIMUM_VALUE.tag
          || tag == dictionary::CHANNEL_MAXIMUM_VALUE.tag =>
      {
        match clarifying_data_elements.waveform_bits_stored {
          Some(8) => Ok(ValueRepresentation::OtherByteString),
          Some(16) => Ok(ValueRepresentation::OtherWordString),
          _ => Err(dictionary::WAVEFORM_BITS_STORED.tag),
        }
      }

      // Use '(5400,1004) WaveformBitsAllocated' to determine an OB/OW VR on
      // relevant values
      [ValueRepresentation::OtherByteString, ValueRepresentation::OtherWordString]
        if tag == dictionary::WAVEFORM_PADDING_VALUE.tag
          || tag == dictionary::WAVEFORM_DATA.tag =>
      {
        match clarifying_data_elements.waveform_bits_allocated {
          Some(8) => Ok(ValueRepresentation::OtherByteString),
          Some(16) => Ok(ValueRepresentation::OtherWordString),
          _ => Err(dictionary::WAVEFORM_BITS_ALLOCATED.tag),
        }
      }

      // The VR for '(0028,3006) LUTData' doesn't need to be determined because
      // the raw binary representation of both VRs is the same.
      // `OtherWordString` is chosen because it's closer to being correct in the
      // case of the LUT containing tightly packed 8-bit data, which is allowed
      // by the spec (Ref: PS3.3 C.11.1.1.1), even though there is no VR that
      // correctly expresses this, i.e. OB is not a valid VR for LUTData.
      [ValueRepresentation::UnsignedShort, ValueRepresentation::OtherWordString]
        if tag == dictionary::LUT_DATA.tag =>
      {
        Ok(ValueRepresentation::OtherWordString)
      }

      // The VR for '(60xx,3000) Overlay Data' doesn't need to be determined as
      // when the transfer syntax is 'Implicit VR Little Endian' it is always
      // OW. Ref: PS3.5 8.1.2.
      [ValueRepresentation::OtherByteString, ValueRepresentation::OtherWordString]
        if tag.group >= 0x6000
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
