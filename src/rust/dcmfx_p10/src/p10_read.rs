//! Converts incoming chunks of binary DICOM P10 data into DICOM P10 tokens.
//!
//! This conversion is done in a streaming fashion, where chunks of incoming
//! raw binary data are added to a read context, and DICOM P10 tokens are then
//! progressively made available as their data comes in. See the [`P10Token`]
//! type for details on the different tokens that are emitted.
//!
//! If DICOM P10 data already exists fully in memory it can be added to a new
//! read context as one complete and final chunk, and then have its DICOM tokens
//! read out, i.e. there is no requirement to use a read context in a streaming
//! fashion, and in either scenario a series of DICOM P10 tokens will be made
//! available by the read context.
//!
//! Additional configuration for controlling memory usage when reading DICOM
//! P10 data is available via [`P10ReadConfig`].

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, format, string::ToString, vec, vec::Vec};

use byteorder::ByteOrder;

use dcmfx_core::{
  DataElementTag, DataElementValue, DataError, DataSet, DataSetPath,
  RcByteSlice, TransferSyntax, ValueRepresentation, dictionary,
  transfer_syntax,
};

use crate::internal::byte_stream::{ByteStream, ByteStreamError};
use crate::internal::data_element_header::{
  DataElementHeader, ValueLengthSize,
};
use crate::internal::p10_location::{self, P10Location};
use crate::{
  P10Error, P10ReadConfig, P10Token, internal::value_length::ValueLength,
};

/// A read context holds the current state of an in-progress DICOM P10 read. Raw
/// DICOM P10 data is added to a read context with [`Self::write_bytes`], and
/// DICOM P10 tokens are then read out with [`Self::read_tokens`].
///
/// An updated read context is returned whenever data is added or tokens are
/// read out, and the updated read context must be used for subsequent calls.
///
#[derive(Debug)]
pub struct P10ReadContext {
  config: P10ReadConfig,
  stream: ByteStream,
  next_action: NextAction,
  transfer_syntax: &'static TransferSyntax,
  path: DataSetPath,
  location: P10Location,
  has_emitted_specific_character_set_data_element: bool,
}

/// The next action specifies what will be attempted to be read next from a read
/// context by `read_tokens`.
///
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
enum NextAction {
  ReadFilePreambleAndDICMPrefix,
  ReadFileMetaInformation {
    starts_at: u64,
    ends_at: Option<u64>,
    data_set: DataSet,
  },
  ReadDataElementHeader,
  ReadDataElementValueBytes {
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: u32,
    bytes_remaining: u32,
    emit_tokens: bool,
  },
  ReadPixelDataItem {
    vr: ValueRepresentation,
  },
}

impl P10ReadContext {
  /// Creates a new read context for reading DICOM P10 data.
  ///
  pub fn new(config: Option<P10ReadConfig>) -> P10ReadContext {
    P10ReadContext {
      config: config.unwrap_or_default(),
      stream: ByteStream::new(),
      next_action: NextAction::ReadFilePreambleAndDICMPrefix,
      transfer_syntax: &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN,
      path: DataSetPath::new(),
      location: P10Location::new(),
      has_emitted_specific_character_set_data_element: false,
    }
  }

  /// Sets the transfer syntax to use when reading DICOM P10 data that doesn't
  /// specify a transfer syntax in its File Meta Information, or doesn't have
  /// any File Meta Information.
  ///
  /// The default is 'Implicit VR Little Endian'.
  ///
  /// The fallback transfer syntax should be set prior to reading any DICOM P10
  /// tokens from the read context.
  ///
  pub fn set_fallback_transfer_syntax(
    &mut self,
    transfer_syntax: &'static TransferSyntax,
  ) {
    self.transfer_syntax = transfer_syntax;
  }

  /// Returns the transfer syntax for a P10 read context. This defaults to
  /// 'Implicit VR Little Endian' and is updated when a transfer syntax is read
  /// from the File Meta Information.
  ///
  /// The default transfer syntax can be set using
  /// [`Self::set_fallback_transfer_syntax()`].
  ///
  pub fn transfer_syntax(&self) -> &TransferSyntax {
    self.transfer_syntax
  }

  /// Writes raw DICOM P10 bytes to a read context that will be parsed into
  /// DICOM P10 tokens by subsequent calls to [`Self::read_tokens()`]. If `done`
  /// is true this indicates the end of the incoming DICOM P10 data to be
  /// parsed, after which any further calls to this function will error.
  ///
  pub fn write_bytes(
    &mut self,
    bytes: RcByteSlice,
    done: bool,
  ) -> Result<(), P10Error> {
    match self.stream.write(bytes, done) {
      Ok(_) => Ok(()),

      Err(e) => Err(
        self.map_byte_stream_error(e, "Writing data to DICOM P10 read context"),
      ),
    }
  }

  /// Reads the next DICOM P10 tokens from a read context. On success, zero or
  /// more tokens are returned and the function can be called again to read
  /// further tokens.
  ///
  /// On error, a value of [`P10Error::DataRequired`] means the read context
  /// does not have enough data to return the next token, i.e. further calls to
  /// [`Self::write_bytes`] are required before the next token is able to be
  /// read.
  ///
  pub fn read_tokens(&mut self) -> Result<Vec<P10Token>, P10Error> {
    match self.next_action {
      NextAction::ReadFilePreambleAndDICMPrefix => {
        self.read_file_preamble_and_dicm_prefix_token()
      }

      NextAction::ReadFileMetaInformation { .. } => {
        self.read_file_meta_information_token()
      }

      NextAction::ReadDataElementHeader => {
        // If there is a delimiter token for a defined-length sequence or item
        // that needs to be emitted then return that as the next token
        let delimiter_token = self.next_delimiter_token();
        if !delimiter_token.is_empty() {
          return Ok(delimiter_token);
        }

        // Detect the end of the DICOM data
        if self.stream.is_fully_consumed() {
          // Return the tokens required to end any active sequences and items.
          //
          // This means there is no check that all items and sequences have been
          // ended as should occur in well-formed P10 data, i.e. P10 data can be
          // truncated on a data element boundary and no error will be thrown.
          //
          // If there's a desire to error on truncated data then add a check
          // that context.location has exactly one entry.

          let tokens = self.location.pending_delimiter_tokens();

          Ok(tokens)
        } else {
          let is_at_root = self.path.entries().is_empty();

          // There is more data so start reading the next data element
          let (mut tokens, tag) = self.read_data_element_header_token()?;

          // Ensure that a Specific Character Set data element is emitted even
          // if the input P10 data doesn't specify one. In this situation, a new
          // data element is inserted into the token stream
          if !self.has_emitted_specific_character_set_data_element
            && is_at_root
            && tag >= dictionary::SPECIFIC_CHARACTER_SET.tag
          {
            if tag > dictionary::SPECIFIC_CHARACTER_SET.tag {
              tokens.splice(0..0, Self::specific_character_set_utf8_tokens());
            }

            self.has_emitted_specific_character_set_data_element = true;
          }

          Ok(tokens)
        }
      }

      NextAction::ReadDataElementValueBytes {
        tag,
        vr,
        length,
        bytes_remaining,
        emit_tokens,
      } => self.read_data_element_value_bytes_token(
        tag,
        vr,
        length,
        bytes_remaining,
        emit_tokens,
      ),

      NextAction::ReadPixelDataItem { vr } => {
        self.read_pixel_data_item_token(vr)
      }
    }
  }

  /// Checks whether there is a delimiter token that needs to be emitted, and if
  /// so then returns it.
  ///
  fn next_delimiter_token(&mut self) -> Vec<P10Token> {
    let bytes_read = self.stream.bytes_read();

    match self.location.next_delimiter_token(bytes_read) {
      Ok(token) => {
        // Update current path
        if matches!(token, P10Token::SequenceDelimiter { .. })
          || token == P10Token::SequenceItemDelimiter
        {
          self.path.pop().unwrap();
        }

        vec![token]
      }

      Err(()) => vec![],
    }
  }

  /// Reads the 128-byte File Preamble and the 4-byte `DICM` prefix following
  /// it. If the `DICM` bytes aren't present at the expected offset then it is
  /// assumed that the File Preamble is not present in the input, and a File
  /// Preamble containing all zero bytes is returned.
  ///
  fn read_file_preamble_and_dicm_prefix_token(
    &mut self,
  ) -> Result<Vec<P10Token>, P10Error> {
    let preamble = match self.stream.peek(132) {
      Ok(data) => {
        if &data[128..132] == b"DICM" {
          self.stream.read(132).map_err(|error| {
            self.map_byte_stream_error(error, "Reading file header")
          })?;

          let mut preamble = [0u8; 128];
          preamble.copy_from_slice(&data[0..128]);

          Ok(Box::new(preamble))
        } else if self.config.require_dicm_prefix {
          Err(P10Error::DicmPrefixNotPresent)
        } else {
          // The 'DICM' prefix is absent but is not configured as required, so
          // return empty preamble bytes
          Ok(Box::new([0u8; 128]))
        }
      }

      // If the end of the data is encountered when trying to read the first 132
      // bytes then there is no File Preamble so return empty preamble bytes
      // unless the 'DICM' prefix is configured as required
      Err(ByteStreamError::DataEnd) if !self.config.require_dicm_prefix => {
        Ok(Box::new([0; 128]))
      }

      Err(e) => Err(self.map_byte_stream_error(e, "Reading file header")),
    }?;

    // The next action after reading or skipping the File Preamble is to read
    // the File Meta Information
    self.next_action = NextAction::ReadFileMetaInformation {
      starts_at: self.stream.bytes_read(),
      ends_at: None,
      data_set: DataSet::new(),
    };

    Ok(vec![P10Token::FilePreambleAndDICMPrefix { preamble }])
  }

  /// Reads the File Meta Information into a data set and returns the relevant
  /// P10 token once complete. If there is a *'(0002,0000) File Meta Information
  /// Group Length'* data element present then it is used to specify where the
  /// File Meta Information ends. If it is not present then data elements are
  /// read until one with a group other than 0x0002 is encountered.
  ///
  fn read_file_meta_information_token(
    &mut self,
  ) -> Result<Vec<P10Token>, P10Error> {
    let NextAction::ReadFileMetaInformation {
      starts_at,
      ends_at,
      data_set: fmi_data_set,
    } = &mut self.next_action
    else {
      unreachable!();
    };

    loop {
      // Check if the end of the File Meta Information has been reached
      if let Some(ends_at) = ends_at {
        if self.stream.bytes_read() >= *ends_at {
          break;
        }
      }

      // Peek the next 8 bytes that contain the group, element, VR, and two
      // bytes that contain the value length if the VR has a 16-bit length
      // field
      let data = self.stream.peek(8).map_err(|e| {
        map_byte_stream_error(
          e,
          "Reading File Meta Information",
          &self.stream,
          &self.path,
        )
      })?;

      let group = byteorder::LittleEndian::read_u16(&data[0..2]);
      let element = byteorder::LittleEndian::read_u16(&data[2..4]);
      let tag = DataElementTag::new(group, element);

      // If the FMI length isn't known and the group isn't 0x0002 then assume
      // that this is the end of the File Meta Information
      if tag.group != 0x0002 && ends_at.is_none() {
        break;
      }

      // If a data element is encountered in the File Meta Information that
      // doesn't have a group of 0x0002 then the File Meta Information is
      // invalid
      if tag.group != 0x0002 && ends_at.is_some() {
        return Err(P10Error::DataInvalid {
          when: "Reading File Meta Information".to_string(),
          details: "Data element in File Meta Information does not have the \
              group 0x0002"
            .to_string(),
          path: DataSetPath::new_with_data_element(tag),
          offset: self.stream.bytes_read(),
        });
      }

      // Get the VR for the data element
      let vr = ValueRepresentation::from_bytes(&data[4..6]).map_err(|_| {
        P10Error::DataInvalid {
          when: "Reading File Meta Information".to_string(),
          details: "Data element has invalid VR".to_string(),
          path: DataSetPath::new_with_data_element(tag),
          offset: self.stream.bytes_read(),
        }
      })?;

      // Check the VR isn't a sequence as these aren't allowed in the File
      // Meta Information
      if vr == ValueRepresentation::Sequence {
        return Err(P10Error::DataInvalid {
          when: "Reading File Meta Information".to_string(),
          details: "Data element in File Meta Information is a sequence"
            .to_string(),
          path: DataSetPath::new_with_data_element(tag),
          offset: self.stream.bytes_read(),
        });
      }

      // Read the value length based on whether the VR has a 16-bit or 32-bit
      // length stored
      let (value_offset, value_length) =
        match DataElementHeader::value_length_size(vr) {
          // 16-bit lengths are read out of the 8 bytes already read
          ValueLengthSize::U16 => Ok((
            8,
            usize::from(byteorder::LittleEndian::read_u16(&data[6..8])),
          )),

          // 32-bit lengths require another 4 bytes to be read
          ValueLengthSize::U32 => match self.stream.peek(12) {
            Ok(data) => {
              Ok((12, byteorder::LittleEndian::read_u32(&data[8..12]) as usize))
            }
            Err(e) => Err(map_byte_stream_error(
              e,
              "Reading File Meta Information",
              &self.stream,
              &self.path,
            )),
          },
        }?;

      let data_element_size = value_offset + value_length;

      // Check that the File Meta Information remains under the max token size
      if fmi_data_set.total_byte_size() + data_element_size as u64
        > u64::from(self.config.max_token_size)
      {
        return Err(P10Error::MaximumExceeded {
          details: format!(
            "File Meta Information exceeds the max token size of {} bytes",
            self.config.max_token_size
          ),
          path: DataSetPath::new_with_data_element(tag),
          offset: self.stream.bytes_read(),
        });
      }

      // Read the value bytes for the data element
      let data = self.stream.read(data_element_size).map_err(|e| {
        map_byte_stream_error(
          e,
          "Reading File Meta Information data element value",
          &self.stream,
          &self.path,
        )
      })?;

      // Construct new data element value
      let value =
        DataElementValue::new_binary_unchecked(vr, data.drop(value_offset));

      // If this data element specifies the File Meta Information group's
      // length then use it to calculate its end offset
      if tag == dictionary::FILE_META_INFORMATION_GROUP_LENGTH.tag {
        if ends_at.is_none() && fmi_data_set.is_empty() {
          match value.get_int::<u32>() {
            Ok(i) => *ends_at = Some(*starts_at + 12 + u64::from(i)),
            Err(_) => {
              return Err(P10Error::DataInvalid {
                when: "Reading File Meta Information".to_string(),
                details: format!(
                  "Group length is invalid: {:?}",
                  value.to_string(DataElementTag::ZERO, 80)
                ),
                path: DataSetPath::new_with_data_element(tag),
                offset: self.stream.bytes_read(),
              });
            }
          }
        }

        continue;
      }

      // If this data element specifies the transfer syntax to use then set it
      // in the read context
      if tag == dictionary::TRANSFER_SYNTAX_UID.tag {
        self.transfer_syntax = match value.get_string() {
          Ok(uid) => TransferSyntax::from_uid(uid).map_err(|_| {
            P10Error::TransferSyntaxNotSupported {
              transfer_syntax_uid: uid.to_string(),
            }
          }),

          Err(e) => {
            if let DataError::TagNotPresent { .. } = e {
              Ok(self.transfer_syntax)
            } else {
              Err(P10Error::DataInvalid {
                when: "Reading File Meta Information".to_string(),
                details: e.to_string(),
                path: DataSetPath::new_with_data_element(
                  dictionary::TRANSFER_SYNTAX_UID.tag,
                ),
                offset: self.stream.bytes_read(),
              })
            }
          }
        }?;
      }

      fmi_data_set.insert(tag, value);
    }

    // If the transfer syntax is deflated then all data following the File
    // Meta Information needs to passed through zlib inflate before reading
    if self.transfer_syntax.is_deflated {
      match self.stream.start_zlib_inflate() {
        Ok(_) => (),
        Err(_) => {
          return Err(P10Error::DataInvalid {
            when: "Starting zlib decompression for deflated transfer syntax"
              .to_string(),
            details: "Zlib data is invalid".to_string(),
            path: DataSetPath::new(),
            offset: self.stream.bytes_read(),
          });
        }
      }
    }

    // Set the final transfer syntax in the File Meta Information token
    if self.transfer_syntax != &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN {
      fmi_data_set
        .insert_string_value(
          &dictionary::TRANSFER_SYNTAX_UID,
          &[self.transfer_syntax.uid],
        )
        .unwrap();
    }

    let token = P10Token::FileMetaInformation {
      data_set: core::mem::take(fmi_data_set),
    };

    self.next_action = NextAction::ReadDataElementHeader;

    Ok(vec![token])
  }

  fn read_data_element_header_token(
    &mut self,
  ) -> Result<(Vec<P10Token>, DataElementTag), P10Error> {
    // Read a data element header if bytes for one are available
    let header = self.read_data_element_header()?;

    // If the VR is UN (Unknown) then attempt to infer it
    let vr = match header.vr {
      Some(ValueRepresentation::Unknown) => {
        Some(self.location.infer_vr_for_tag(header.tag).map_err(
          |missing_tag| P10Error::DataInvalid {
            when: format!(
              "Inferring VR for data element '{}'",
              dictionary::tag_with_name(header.tag, None)
            ),
            details: format!(
              "The value for the '{}' data element is missing or invalid",
              dictionary::tag_with_name(missing_tag, None)
            ),
            path: self.path.clone(),
            offset: self.stream.bytes_read(),
          },
        )?)
      }
      vr => vr,
    };

    match (header.tag, vr, header.length) {
      // If this is the start of a new sequence then add it to the location
      (tag, Some(ValueRepresentation::Sequence), _)
      | (tag, Some(ValueRepresentation::Unknown), ValueLength::Undefined) => {
        self.check_data_element_ordering(&header)?;

        let ends_at = match header.length {
          ValueLength::Defined { length } => {
            Some(self.stream.bytes_read() + u64::from(length))
          }
          ValueLength::Undefined => None,
        };

        // When the original VR was unknown and the length is undefined, as per
        // DICOM Correction Proposal CP-246 the 'Implicit VR Little Endian'
        // transfer syntax must be used to read the sequence's data.
        // Ref: https://dicom.nema.org/dicom/cp/cp246_01.pdf.
        let is_implicit_vr = header.vr == Some(ValueRepresentation::Unknown);

        self
          .location
          .add_sequence(tag, is_implicit_vr, ends_at)
          .map_err(|details| P10Error::DataInvalid {
            when: "Reading data element header".to_string(),
            details,
            path: self.path.clone(),
            offset: self.stream.bytes_read(),
          })?;

        // Check that the maximum sequence depth hasn't been reached
        if self.path.len() / 2 >= self.config.max_sequence_depth {
          return Err(P10Error::MaximumExceeded {
            details: "Maximum allowed sequence depth reached".to_string(),
            path: self.path.clone(),
            offset: self.stream.bytes_read(),
          });
        }

        // Add sequence to the path
        self.path.add_data_element(tag).unwrap();

        Ok((
          vec![P10Token::SequenceStart {
            tag,
            vr: ValueRepresentation::Sequence,
            path: self.path.clone(),
          }],
          header.tag,
        ))
      }

      // If this is the start of a new sequence item then add it to the location
      (tag, None, _) if tag == dictionary::ITEM.tag => {
        let ends_at = match header.length {
          ValueLength::Defined { length } => {
            Some(self.stream.bytes_read() + u64::from(length))
          }
          ValueLength::Undefined => None,
        };

        let index = self.location.add_item(ends_at, header.length).map_err(
          |details| P10Error::DataInvalid {
            when: "Reading data element header".to_string(),
            details,
            path: self.path.clone(),
            offset: self.stream.bytes_read(),
          },
        )?;

        // Add item to the path
        self.path.add_sequence_item(index).unwrap();

        Ok((vec![P10Token::SequenceItemStart { index }], header.tag))
      }

      // If this is an encapsulated pixel data sequence then add it to the
      // current location and update the next action to read its items
      (tag, Some(vr), ValueLength::Undefined)
        if tag == dictionary::PIXEL_DATA.tag
          && (vr == ValueRepresentation::OtherByteString
            || vr == ValueRepresentation::OtherWordString) =>
      {
        self.check_data_element_ordering(&header)?;

        self
          .location
          .add_sequence(tag, false, None)
          .map_err(|details| P10Error::DataInvalid {
            when: "Reading data element header".to_string(),
            details,
            path: self.path.clone(),
            offset: self.stream.bytes_read(),
          })?;

        self.path.add_data_element(tag).unwrap();

        self.next_action = NextAction::ReadPixelDataItem { vr };

        Ok((
          vec![P10Token::SequenceStart {
            tag,
            vr,
            path: self.path.clone(),
          }],
          header.tag,
        ))
      }

      // If this is a sequence delimitation item then remove the current
      // sequence from the current location
      (tag, None, ValueLength::ZERO)
        if tag == dictionary::SEQUENCE_DELIMITATION_ITEM.tag =>
      {
        let tokens = if let Ok(tag) = self.location.end_sequence() {
          self.path.pop().unwrap();

          vec![P10Token::SequenceDelimiter { tag }]
        } else {
          // If a sequence delimiter occurs outside of a sequence then no error
          // is returned and P10 parsing continues. This is done because rogue
          // sequence delimiters have been observed in some DICOM P10 data, and
          // not propagating an error right here doesn't do any harm and allows
          // such data to be read.

          vec![]
        };

        Ok((tokens, header.tag))
      }

      // If this is an item delimitation item then remove the latest item from
      // the location
      (tag, None, ValueLength::ZERO)
        if tag == dictionary::ITEM_DELIMITATION_ITEM.tag =>
      {
        self
          .location
          .end_item()
          .map_err(|details| P10Error::DataInvalid {
            when: "Reading data element header".to_string(),
            details,
            path: self.path.clone(),
            offset: self.stream.bytes_read(),
          })?;

        self.path.pop().unwrap();

        Ok((vec![P10Token::SequenceItemDelimiter], header.tag))
      }

      // For all other cases this is a standard data element that needs to have
      // its value bytes read
      (tag, Some(vr), ValueLength::Defined { length }) => {
        self.check_data_element_ordering(&header)?;

        let materialized_value_required =
          self.is_materialized_value_required(header.tag, vr);

        // If this data element needs to be fully materialized then check it
        // doesn't exceed the max string size
        if materialized_value_required && length > self.config.max_string_size {
          return Err(P10Error::MaximumExceeded {
            details: format!(
              "Value for '{}' with VR {} and length {} bytes exceeds the \
              maximum allowed string size of {} bytes",
              dictionary::tag_with_name(header.tag, None),
              vr,
              length,
              self.config.max_string_size
            ),
            path: self.path.clone(),
            offset: self.stream.bytes_read(),
          });
        }

        // Add data element to the path
        self
          .path
          .add_data_element(tag)
          .map_err(|_| P10Error::DataInvalid {
            when: "Reading data element header".to_string(),
            details: format!(
              "Data element '{}' is not valid for the current path",
              header
            ),
            path: self.path.clone(),
            offset: self.stream.bytes_read(),
          })?;

        // Swallow the '(FFFC,FFFC) Data Set Trailing Padding' data element. No
        // tokens for it are emitted. Ref: PS3.10 7.2.
        //
        // Also swallow group length tags that have an element of 0x0000.
        // Ref: PS3.5 7.2.
        let emit_tokens = header.tag
          != dictionary::DATA_SET_TRAILING_PADDING.tag
          && header.tag.element != 0x0000;

        // If the whole value is being materialized then the DataElementHeader
        // token is only emitted once all the data is available. This is
        // necessary because in the case of string values that are being
        // converted to UTF-8 the length of the final string value following
        // UTF-8 conversion is not yet known.
        let tokens = if emit_tokens && !materialized_value_required {
          vec![P10Token::DataElementHeader {
            tag: header.tag,
            vr,
            length,
            path: self.path.clone(),
          }]
        } else {
          vec![]
        };

        self.next_action = NextAction::ReadDataElementValueBytes {
          tag: header.tag,
          vr,
          length,
          bytes_remaining: length,
          emit_tokens,
        };

        Ok((tokens, header.tag))
      }

      (_, _, _) => Err(P10Error::DataInvalid {
        when: "Reading data element header".to_string(),
        details: format!("Invalid data element '{}'", header),
        path: self.path.clone(),
        offset: self.stream.bytes_read(),
      }),
    }
  }

  /// Returns the two tokens for the '(0008,0005) Specific Character Set' data
  /// element that specifies UTF-8 (ISO_IR 192).
  ///
  fn specific_character_set_utf8_tokens() -> [P10Token; 2] {
    let tag = dictionary::SPECIFIC_CHARACTER_SET.tag;
    let vr = ValueRepresentation::CodeString;
    let data = b"ISO_IR 192";

    [
      P10Token::DataElementHeader {
        tag,
        vr,
        length: data.len() as u32,
        path: DataSetPath::new(),
      },
      P10Token::DataElementValueBytes {
        tag,
        vr,
        data: data.to_vec().into(),
        bytes_remaining: 0,
      },
    ]
  }

  /// Reads a data element header. Depending on the transfer syntax and the
  /// specific VR (for explicit VR transfer syntaxes), this reads either 8 or 12
  /// bytes in total.
  ///
  fn read_data_element_header(
    &mut self,
  ) -> Result<DataElementHeader, P10Error> {
    let transfer_syntax = self.active_transfer_syntax();

    // Peek the 4 bytes containing the tag
    let tag = match self.stream.peek(4) {
      Ok(data) => {
        let (group, element) = match transfer_syntax.endianness {
          transfer_syntax::Endianness::LittleEndian => (
            byteorder::LittleEndian::read_u16(&data[0..2]),
            byteorder::LittleEndian::read_u16(&data[2..4]),
          ),

          transfer_syntax::Endianness::BigEndian => (
            byteorder::BigEndian::read_u16(&data[0..2]),
            byteorder::BigEndian::read_u16(&data[2..4]),
          ),
        };

        Ok(DataElementTag::new(group, element))
      }

      Err(e) => {
        Err(self.map_byte_stream_error(e, "Reading data element header"))
      }
    }?;

    // The item and delimitation tags always use implicit VRs
    let vr_serialization = if tag == dictionary::ITEM.tag
      || tag == dictionary::ITEM_DELIMITATION_ITEM.tag
      || tag == dictionary::SEQUENCE_DELIMITATION_ITEM.tag
    {
      transfer_syntax::VrSerialization::VrImplicit
    } else {
      transfer_syntax.vr_serialization
    };

    // File Meta Information data elements aren't allowed in the root of the
    // main data set. They are allowed in sequence items only because this has
    // been observed in the wild (specifically TransferSyntaxUID as the first
    // data element in an item), however this is not valid according to the
    // spec.
    if tag.group == 0x0002
      && self.path.is_root()
      && !matches!(self.next_action, NextAction::ReadFileMetaInformation { .. })
    {
      return Err(P10Error::DataInvalid {
        when: "Reading data element header".to_string(),
        details: format!(
          "File Meta Information data element '{}' found in the main data set",
          tag
        ),
        path: DataSetPath::new_with_data_element(tag),
        offset: self.stream.bytes_read(),
      });
    }

    match vr_serialization {
      transfer_syntax::VrSerialization::VrExplicit => {
        self.read_explicit_vr_and_length(tag)
      }
      transfer_syntax::VrSerialization::VrImplicit => {
        self.read_implicit_vr_and_length(tag)
      }
    }
  }

  /// Checks that the specified data element tag is greater than the previous
  /// one at the current P10 location.
  ///
  fn check_data_element_ordering(
    &mut self,
    header: &DataElementHeader,
  ) -> Result<(), P10Error> {
    if !self.config.require_ordered_data_elements {
      return Ok(());
    }

    self
      .location
      .check_data_element_ordering(header.tag)
      .map_err(|_| P10Error::DataInvalid {
        when: "Reading data element header".to_string(),
        details: format!("Data element '{}' is not in ascending order", header),
        path: self.path.clone(),
        offset: self.stream.bytes_read(),
      })
  }

  /// Returns the transfer syntax that should be used to decode the current
  /// data. This will always be the transfer syntax specified in the File Meta
  /// Information, except in the case of 'Implicit VR Little Endian' being
  /// forced by an explicit VR of `UN` (Unknown) that has an undefined length.
  ///
  /// Ref: DICOM Correction Proposal CP-246.
  ///
  fn active_transfer_syntax(&self) -> &'static TransferSyntax {
    if self.location.is_implicit_vr_forced() {
      &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN
    } else {
      self.transfer_syntax
    }
  }

  /// Reads the (implicit) VR and value length following a data element tag when
  /// the transfer syntax is 'Implicit VR Little Endian'.
  ///
  fn read_implicit_vr_and_length(
    &mut self,
    tag: DataElementTag,
  ) -> Result<DataElementHeader, P10Error> {
    match self.stream.read(8) {
      Ok(data) => {
        let value_length = match self.active_transfer_syntax().endianness {
          transfer_syntax::Endianness::LittleEndian => {
            byteorder::LittleEndian::read_u32(&data[4..8])
          }
          transfer_syntax::Endianness::BigEndian => {
            byteorder::BigEndian::read_u32(&data[4..8])
          }
        };

        // Return the VR as `None` for those tags that don't support one. All
        // other tags are returned as UN (Unknown) and will have their VR
        // inferred in due course.
        let vr = if tag == dictionary::ITEM.tag
          || tag == dictionary::ITEM_DELIMITATION_ITEM.tag
          || tag == dictionary::SEQUENCE_DELIMITATION_ITEM.tag
        {
          None
        } else {
          Some(ValueRepresentation::Unknown)
        };

        let header = DataElementHeader {
          tag,
          vr,
          length: ValueLength::new(value_length),
        };

        Ok(header)
      }

      Err(e) => {
        Err(self.map_byte_stream_error(e, "Reading data element header"))
      }
    }
  }

  /// Reads the explicit VR and value length following a data element tag when
  /// the transfer syntax is not 'Implicit VR Little Endian'.
  ///
  fn read_explicit_vr_and_length(
    &mut self,
    tag: DataElementTag,
  ) -> Result<DataElementHeader, P10Error> {
    // Peek and validate the explicit VR
    let vr = match self.stream.peek(6) {
      Ok(data) => {
        let vr_bytes = &data[4..6];

        match ValueRepresentation::from_bytes(vr_bytes) {
          Ok(vr) => Ok(vr),

          // If the VR is two spaces or two NULL characters then treat it as UN,
          // and there will be an attempt to infer it in due course. This is not
          // part of the DICOM P10 spec, but such data has been observed in the
          // wild.
          _ => match vr_bytes {
            [0x00, 0x00] | [0x20, 0x20] => Ok(ValueRepresentation::Unknown),

            _ => Err(P10Error::DataInvalid {
              when: "Reading data element VR".to_string(),
              details: format!(
                "Unrecognized VR {:?} for tag '{}'",
                vr_bytes,
                dictionary::tag_with_name(tag, None)
              ),
              path: self.path.clone(),
              offset: self.stream.bytes_read(),
            }),
          },
        }
      }

      Err(e) => Err(
        self
          .map_byte_stream_error(e, "Reading explicit VR data element header"),
      ),
    }?;

    // If reading the VR succeeded continue by reading the value length that
    // follows it. The total size of the header in bytes varies by VR.
    let header_size = match DataElementHeader::value_length_size(vr) {
      ValueLengthSize::U32 => 12,
      ValueLengthSize::U16 => 8,
    };

    // Read the full header, including the tag, VR, and value length
    match self.stream.read(header_size) {
      Ok(data) => {
        // Parse value length
        let length = match header_size {
          12 => match self.active_transfer_syntax().endianness {
            transfer_syntax::Endianness::LittleEndian => {
              byteorder::LittleEndian::read_u32(&data[8..12])
            }
            transfer_syntax::Endianness::BigEndian => {
              byteorder::BigEndian::read_u32(&data[8..12])
            }
          },
          _ => match self.active_transfer_syntax().endianness {
            transfer_syntax::Endianness::LittleEndian => {
              byteorder::LittleEndian::read_u16(&data[6..8]).into()
            }
            transfer_syntax::Endianness::BigEndian => {
              byteorder::BigEndian::read_u16(&data[6..8]).into()
            }
          },
        };

        let header = DataElementHeader {
          tag,
          vr: Some(vr),
          length: ValueLength::new(length),
        };

        Ok(header)
      }

      Err(e) => Err(
        self
          .map_byte_stream_error(e, "Reading explicit VR data element header"),
      ),
    }
  }

  fn read_data_element_value_bytes_token(
    &mut self,
    tag: DataElementTag,
    vr: ValueRepresentation,
    value_length: u32,
    bytes_remaining: u32,
    emit_tokens: bool,
  ) -> Result<Vec<P10Token>, P10Error> {
    let materialized_value_required =
      self.is_materialized_value_required(tag, vr);

    // If this data element value is being fully materialized then it needs to
    // be read as a whole, so use its full length as the number of bytes to
    // read. Otherwise, read up to the max token size.
    let bytes_to_read = if materialized_value_required {
      value_length
    } else {
      core::cmp::min(bytes_remaining, self.config.max_token_size)
    };

    match self.stream.read(bytes_to_read as usize) {
      Ok(mut data) => {
        // Data element values are always returned in little endian, so if this
        // is a big endian transfer syntax then convert to little endian
        if self.active_transfer_syntax().endianness.is_big() {
          let mut raw_data = data.into_vec();
          self.location.swap_endianness(tag, vr, &mut raw_data);
          data = raw_data.into();
        }

        let bytes_remaining = bytes_remaining - bytes_to_read;

        let data = if materialized_value_required {
          self.process_materialized_data_element(tag, vr, data)?
        } else {
          data
        };

        let mut tokens = Vec::with_capacity(2);

        if emit_tokens {
          // If this is a materialized value then the data element header for it
          // is emitted now. It was not emitted when it was read due to the
          // possibility of the Value and Value Length being altered above.
          if materialized_value_required {
            let max_length =
              DataElementHeader::value_length_size(vr).max_length();

            if data.len() <= max_length {
              tokens.push(P10Token::DataElementHeader {
                tag,
                vr,
                length: data.len() as u32,
                path: self.path.clone(),
              });
            } else {
              return Err(P10Error::DataInvalid {
                when: "Reading data element value bytes".to_string(),
                details: format!(
                  "Length of {} bytes exceeds the maximum of {} bytes after \
                    conversion to UTF-8",
                  data.len(),
                  max_length
                ),
                path: self.path.clone(),
                offset: self.stream.bytes_read(),
              });
            }
          }

          tokens.push(P10Token::DataElementValueBytes {
            tag,
            vr,
            data,
            bytes_remaining,
          });
        }

        let next_action = if bytes_remaining == 0 {
          // This data element is complete, so the next action is either to read
          // the next pixel data item if currently reading pixel data items, or
          // to read the header for the next data element
          if tag == dictionary::ITEM.tag {
            NextAction::ReadPixelDataItem { vr }
          } else {
            NextAction::ReadDataElementHeader
          }
        } else {
          // Continue reading value bytes for this data element
          NextAction::ReadDataElementValueBytes {
            tag,
            vr,
            length: value_length,
            bytes_remaining,
            emit_tokens,
          }
        };

        if bytes_remaining == 0 {
          self.path.pop().unwrap();
        }

        self.next_action = next_action;

        Ok(tokens)
      }

      Err(e) => {
        let when = format!(
          "Reading {} data element value bytes, VR: {}",
          bytes_to_read, vr
        );

        Err(self.map_byte_stream_error(e, &when))
      }
    }
  }

  fn is_materialized_value_required(
    &self,
    tag: DataElementTag,
    vr: ValueRepresentation,
  ) -> bool {
    // If this is a clarifying data element then its data needs to be
    // materialized
    if p10_location::is_clarifying_data_element(tag) {
      return true;
    }

    // If the value is an encoded string, and it isn't UTF-8 compatible data
    // that can be passed straight through, then materialize it so that it can
    // be converted to UTF-8.
    if vr.is_encoded_string() {
      return !self.location.is_specific_character_set_utf8_compatible();
    }

    // Convert strings that are defined to use ISO-646/US-ASCII. In theory this
    // shouldn't be necessary as they should already be valid UTF-8, but DICOM
    // P10 data has been observed that contains invalid ISO-646 data, hence
    // these string values are sanitized by replacing invalid characters with a
    // question mark.
    if vr.is_string() {
      return true;
    }

    false
  }

  fn process_materialized_data_element(
    &mut self,
    tag: DataElementTag,
    vr: ValueRepresentation,
    mut value_bytes: RcByteSlice,
  ) -> Result<RcByteSlice, P10Error> {
    // Decode string values using the relevant character set
    if vr.is_string() {
      // Private Creator values must only contain characters from the Default
      // Character Repertoire and so are sanitized against that character set.
      // Ref: PS3.5 7.8.1.
      if vr.is_encoded_string() && !tag.is_private_creator() {
        value_bytes =
          self.location.decode_string_bytes(vr, &value_bytes).into();
      } else {
        let mut data = value_bytes.into_vec();
        dcmfx_character_set::sanitize_default_charset_bytes(&mut data);
        value_bytes = data.into();
      }
    }

    // Update the P10 location with the materialized value, this will only do
    // something when this is a clarifying data element
    self
      .location
      .add_clarifying_data_element(tag, vr, &mut value_bytes)?;

    Ok(value_bytes)
  }

  fn read_pixel_data_item_token(
    &mut self,
    vr: ValueRepresentation,
  ) -> Result<Vec<P10Token>, P10Error> {
    match self.read_data_element_header() {
      Ok(header) => match header {
        // Pixel data items must have no VR and a defined length
        DataElementHeader {
          tag,
          vr: None,
          length: ValueLength::Defined { length },
        } if tag == dictionary::ITEM.tag => {
          self.next_action = NextAction::ReadDataElementValueBytes {
            tag: dictionary::ITEM.tag,
            vr,
            length,
            bytes_remaining: length,
            emit_tokens: true,
          };

          // Add item to the path
          let item_count = self.location.sequence_item_count().unwrap_or(1);
          let index = item_count - 1;
          self.path.add_sequence_item(index).unwrap();

          let token = P10Token::PixelDataItem { index, length };

          Ok(vec![token])
        }

        DataElementHeader {
          tag,
          vr: None,
          length: ValueLength::ZERO,
        } if tag == dictionary::SEQUENCE_DELIMITATION_ITEM.tag => {
          let token = P10Token::SequenceDelimiter {
            tag: dictionary::PIXEL_DATA.tag,
          };

          self.location.end_sequence().map_err(|details| {
            P10Error::DataInvalid {
              when: "Reading encapsulated pixel data item".to_string(),
              details,
              path: self.path.clone(),
              offset: self.stream.bytes_read(),
            }
          })?;

          self.path.pop().unwrap();

          self.next_action = NextAction::ReadDataElementHeader;

          Ok(vec![token])
        }

        _ => Err(P10Error::DataInvalid {
          when: "Reading encapsulated pixel data item".to_string(),
          details: format!("Invalid data element '{}'", header),
          path: self.path.clone(),
          offset: self.stream.bytes_read(),
        }),
      },

      Err(e) => Err(e),
    }
  }

  /// Takes an error from the byte stream and maps it through to a P10 error.
  ///
  fn map_byte_stream_error(
    &self,
    error: ByteStreamError,
    when: &str,
  ) -> P10Error {
    map_byte_stream_error(error, when, &self.stream, &self.path)
  }
}

/// Takes an error from the byte stream and maps it through to a P10 error.
///
fn map_byte_stream_error(
  error: ByteStreamError,
  when: &str,
  stream: &ByteStream,
  path: &DataSetPath,
) -> P10Error {
  let offset = stream.bytes_read();

  match error {
    ByteStreamError::DataRequired => P10Error::DataRequired {
      when: when.to_string(),
    },

    ByteStreamError::DataEnd => P10Error::DataEndedUnexpectedly {
      when: when.to_string(),
      path: path.clone(),
      offset,
    },

    ByteStreamError::ZlibDataError => P10Error::DataInvalid {
      when: when.to_string(),
      details: "Zlib data is invalid".to_string(),
      path: path.clone(),
      offset,
    },

    ByteStreamError::WriteAfterCompletion => P10Error::WriteAfterCompletion,
  }
}

impl Default for P10ReadContext {
  fn default() -> Self {
    Self::new(None)
  }
}
