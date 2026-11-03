#[cfg(not(feature = "std"))]
use alloc::{
  boxed::Box,
  string::{String, ToString},
  vec,
  vec::Vec,
};
use dcmfx_core::{
  DataElementTag, DataElementValue, DataSet, DataSetPath, ValueRepresentation,
  dictionary,
};
use dcmfx_p10::{DataSetBuilder, P10Error, P10Token, p10_token};

use crate::{
  Action, ContentItemAction, Deidentifier, DeidentifyError,
  basic_profile::{action_for_content_item, action_for_data_element},
  deidentifier::{
    deidentification_method_code_sequence_value, deidentification_method_value,
    longitudinal_temporal_information_modified_value,
    patient_identity_removed_value,
  },
  utils::trim_padding,
};

/// De-identifies a data set by running its tokens through a
/// [`P10DeidentifyTransform`] and rebuilding it from the result, so that the
/// result is exactly what de-identifying a stream of the data set's tokens
/// produces. The File Meta Information, if any, is passed as its own token.
/// `record` is whether the data elements that record the de-identification
/// are set in the result, which they aren't for the File Meta Information
/// that the transform de-identifies in memory.
///
pub(crate) fn deidentify_data_set(
  deidentifier: &Deidentifier,
  data_set: &mut DataSet,
  record: bool,
) -> Result<(), DeidentifyError> {
  if deidentifier.date_shift_days() == Some(0) {
    return Err(DeidentifyError::ZeroDateShift);
  }

  // The File Meta Information, if any, is passed as its own token, except
  // when it is the File Meta Information itself that is being de-identified,
  // whose data elements are then passed as data elements
  let (file_meta_information, main_data_set) = if record {
    (
      data_set.filter(|tag, _| tag.is_file_meta_information()),
      data_set.filter(|tag, _| !tag.is_file_meta_information()),
    )
  } else {
    (DataSet::new(), data_set.clone())
  };

  let mut transform = if record {
    P10DeidentifyTransform::new(deidentifier.clone())
  } else {
    P10DeidentifyTransform::new_without_recording(deidentifier.clone())
  };
  let mut builder = DataSetBuilder::new();

  // The tokens of a data set are always well formed, so the only errors the
  // transform reports for them are its own
  let mut add_token = |token: P10Token| -> Result<(), DeidentifyError> {
    let output_tokens = transform.add_token(&token).map_err(|_| {
      transform
        .deidentify_error()
        .cloned()
        .expect("De-identifying data set tokens failed")
    })?;

    builder
      .add_tokens(&output_tokens)
      .expect("Rebuilding de-identified data set failed");

    Ok(())
  };

  if !file_meta_information.is_empty() {
    add_token(P10Token::FileMetaInformation {
      data_set: file_meta_information,
    })?;
  }

  p10_token::data_elements_to_tokens(
    &main_data_set,
    &DataSetPath::new(),
    &mut add_token,
  )?;

  add_token(P10Token::End)?;

  *data_set = builder
    .final_data_set()
    .expect("De-identified data set is incomplete");

  Ok(())
}

/// The longest value that is held in memory so that it can be replaced with
/// a value that depends on it: 64 KiB.
///
const MAX_REPLACED_VALUE_LENGTH: u32 = 64 * 1024;

/// The data elements that hold a content item's value and identify by
/// themselves. These are kept unchanged in content items that an option
/// retains, as they are the only parts of a value that the profile's usual
/// rules would alter.
///
const CONTENT_ITEM_LEAF_VALUE_TAGS: [DataElementTag; 7] = [
  dictionary::DATE_TIME.tag,
  dictionary::DATE.tag,
  dictionary::TIME.tag,
  dictionary::PERSON_NAME.tag,
  dictionary::UID.tag,
  dictionary::TEXT_VALUE.tag,
  dictionary::UNFORMATTED_TEXT_VALUE.tag,
];

/// The data elements that hold the numeric or text representations of a
/// content item's value and aren't subject to Table E.1-1. These are the data
/// elements replaced when a content item's value is replaced, both directly in
/// the content item and in the items of its Measured Value Sequence
/// (0040,A300).
///
const CONTENT_ITEM_REPLACED_VALUE_TAGS: [DataElementTag; 5] = [
  dictionary::TEXT_VALUE.tag,
  dictionary::NUMERIC_VALUE.tag,
  dictionary::FLOATING_POINT_VALUE.tag,
  dictionary::RATIONAL_NUMERATOR_VALUE.tag,
  dictionary::RATIONAL_DENOMINATOR_VALUE.tag,
];

/// Transform that applies the Basic Application Level Confidentiality Profile,
/// and the de-identifier's options, to a stream of DICOM P10 tokens without
/// needing to hold the whole data set in memory. Values whose replacement
/// depends on the original, i.e. UIDs and dates, are accumulated before being
/// replaced, with a value longer than 64 KiB an error, see
/// [`DeidentifyError::OriginalValueTooLong`], and under the Clean Structured
/// Content Option the tokens of each content item are held back until its
/// Concept Name Code Sequence (0040,A043) arrives, as the action for the
/// content item depends on it and a few of the content item's data elements
/// precede it. The memory held is bounded by 64 KiB for one data element's
/// value, or by the start of one content item at each level of nesting. [`DataSetDeidentifyExtensions::deidentify()`] is
/// implemented by running the data set's tokens through this transform, as is
/// the File Meta Information that this transform de-identifies in memory.
///
/// [`DataSetDeidentifyExtensions::deidentify()`]: crate::DataSetDeidentifyExtensions::deidentify
///
/// The File Preamble is zeroed, the File Meta Information is de-identified, each
/// data element has its action applied as it streams through, and the data
/// elements that record the de-identification are inserted into the root data
/// set at the correct position, replacing any existing ones.
///
pub struct P10DeidentifyTransform {
  deidentifier: Deidentifier,

  /// One entry for each sequence, sequence item, data element, and pixel data
  /// item that is currently open in the token stream.
  frames: Vec<Frame>,

  /// The data element whose value bytes are currently being accumulated so
  /// that a replacement value can be emitted once they're complete.
  pending_replacement: Option<PendingReplacement>,

  /// The content item whose tokens are currently being held back until its
  /// action is decided. The content items nested inside it are held back as
  /// part of it, so there is at most one at a time.
  pending_content_item: Option<PendingContentItem>,

  /// The data elements that record the de-identification and have not yet
  /// been emitted into the root data set. Sorted by descending tag so that the
  /// lowest tag is at the end.
  pending_inserts: Vec<(DataElementTag, DataElementValue)>,

  /// The de-identification error behind the most recent error returned by
  /// [`Self::add_token()`], if that error was specific to de-identification.
  deidentify_error: Option<DeidentifyError>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Frame {
  /// Tokens pass through unchanged.
  PassThrough,

  /// Tokens are discarded.
  Discard,

  /// The value bytes of a data element are accumulated and then replaced.
  ReplaceValue,

  /// A sequence of content items that is cleaned under the Clean Structured
  /// Content Option. Each of its items is a content item whose action is
  /// decided when its concept name arrives.
  ContentItems,

  /// A content item whose tokens are held in `pending_content_item` until its
  /// action is decided.
  PendingContentItem,

  /// A content item whose action has been decided, or an item of the Measured
  /// Value Sequence (0040,A300) of a content item whose value is replaced.
  /// `is_text` is whether the content item's Value Type (0040,A040) is TEXT,
  /// in which case its Text Value (0040,A160) is its value.
  ContentItem {
    action: ContentItemAction,
    is_text: bool,
  },

  /// The Measured Value Sequence (0040,A300) of a content item whose value is
  /// replaced, whose items have their numeric values replaced.
  ReplacedMeasuredValues,
}

struct PendingReplacement {
  tag: DataElementTag,
  vr: ValueRepresentation,
  path: DataSetPath,

  /// The action that replaces the value, which is either
  /// [`Action::ReplaceUid`] or [`Action::Clean`].
  action: Action,

  bytes: Vec<u8>,
}

/// A content item whose tokens are held back until its action is decided,
/// which is when its Concept Name Code Sequence (0040,A043) has arrived, or
/// when it ends without one.
///
struct PendingContentItem {
  /// The token that started the content item.
  item_start: P10Token,

  /// The tokens held back for the content item, after `item_start`.
  tokens: Vec<P10Token>,

  /// The nesting depth of the held back tokens relative to the content item,
  /// where zero is the content item's own level.
  relative_depth: usize,

  /// The data element at the content item's own level whose value bytes are
  /// arriving.
  element_tag: Option<DataElementTag>,

  /// The value bytes of the content item's Value Type (0040,A040).
  value_type: Vec<u8>,

  /// The index in `tokens` of the start of the Concept Name Code Sequence.
  concept_name_start: Option<usize>,
}

impl P10DeidentifyTransform {
  /// Creates a new transform that de-identifies a stream of DICOM P10 tokens
  /// using the given de-identifier.
  ///
  pub fn new(deidentifier: Deidentifier) -> Self {
    let mut transform = Self::new_without_recording(deidentifier);
    let options = transform.deidentifier.options();

    transform.pending_inserts = vec![
      (
        dictionary::LONGITUDINAL_TEMPORAL_INFORMATION_MODIFIED.tag,
        longitudinal_temporal_information_modified_value(options),
      ),
      (
        dictionary::DEIDENTIFICATION_METHOD_CODE_SEQUENCE.tag,
        deidentification_method_code_sequence_value(options),
      ),
      (
        dictionary::DEIDENTIFICATION_METHOD.tag,
        deidentification_method_value(options),
      ),
      (
        dictionary::PATIENT_IDENTITY_REMOVED.tag,
        patient_identity_removed_value(),
      ),
    ];

    transform
  }

  /// Creates a new transform that doesn't set the data elements that record
  /// the de-identification, for the File Meta Information that the transform
  /// de-identifies in memory.
  ///
  fn new_without_recording(deidentifier: Deidentifier) -> Self {
    Self {
      deidentifier,
      frames: vec![],
      pending_replacement: None,
      pending_content_item: None,
      pending_inserts: vec![],
      deidentify_error: None,
    }
  }

  /// Returns the de-identification error behind the most recent error returned
  /// by [`Self::add_token()`], if that error was specific to de-identification
  /// rather than to the token stream.
  ///
  pub fn deidentify_error(&self) -> Option<&DeidentifyError> {
    self.deidentify_error.as_ref()
  }

  /// Records a de-identification error and returns it as a [`P10Error`].
  ///
  fn fail(&mut self, error: DeidentifyError) -> P10Error {
    let error_type = match error {
      DeidentifyError::ZeroDateShift => "De-identifying with a zero date shift",
      DeidentifyError::ValueTooLong { .. } => "De-identified value too long",
      DeidentifyError::OriginalValueTooLong { .. } => "Value too long",
    };

    let p10_error = P10Error::OtherError {
      error_type: error_type.to_string(),
      details: error.details(),
    };

    self.deidentify_error = Some(error);

    p10_error
  }

  /// Returns the de-identifier used by this transform.
  ///
  pub fn deidentifier(&self) -> &Deidentifier {
    &self.deidentifier
  }

  /// Adds the next token to this transform and returns the resulting tokens.
  ///
  pub fn add_token(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<P10Token>, P10Error> {
    if self.deidentifier.date_shift_days() == Some(0) {
      return Err(self.fail(DeidentifyError::ZeroDateShift));
    }

    self.route_token(token)
  }

  /// Holds back the token if a content item's action is pending, and
  /// processes it otherwise.
  ///
  fn route_token(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<P10Token>, P10Error> {
    if self.pending_content_item.is_some() {
      return self.hold_content_item_token(token);
    }

    self.process_token(token)
  }

  /// Holds back a token for the pending content item, tracking the nesting
  /// depth, the content item's Value Type (0040,A040), and its Concept Name
  /// Code Sequence (0040,A043). When the concept name has arrived, or the
  /// content item ends without one, the content item's action is decided.
  ///
  fn hold_content_item_token(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<P10Token>, P10Error> {
    let pending = self.pending_content_item.as_mut().unwrap();

    match token {
      P10Token::SequenceStart { tag, .. } => {
        if pending.relative_depth == 0
          && *tag == dictionary::CONCEPT_NAME_CODE_SEQUENCE.tag
        {
          pending.concept_name_start = Some(pending.tokens.len());
        }

        pending.relative_depth += 1;
      }

      P10Token::SequenceItemStart { .. } | P10Token::PixelDataItem { .. } => {
        pending.relative_depth += 1
      }

      P10Token::DataElementHeader { tag, .. } => {
        if pending.relative_depth == 0 {
          pending.element_tag = Some(*tag);
        }

        pending.relative_depth += 1;
      }

      P10Token::DataElementValueBytes {
        data,
        bytes_remaining,
        ..
      } => {
        if pending.relative_depth == 1
          && pending.element_tag == Some(dictionary::VALUE_TYPE.tag)
        {
          pending.value_type.extend_from_slice(data);
        }

        if *bytes_remaining == 0 {
          pending.relative_depth -= 1;
        }
      }

      P10Token::SequenceDelimiter { .. } => {
        pending.relative_depth -= 1;

        // The concept name has arrived
        if pending.relative_depth == 0 && pending.concept_name_start.is_some() {
          pending.tokens.push(token.clone());

          return self.decide_content_item(true);
        }
      }

      P10Token::SequenceItemDelimiter => {
        // The content item has ended without a concept name
        if pending.relative_depth == 0 {
          let mut output_tokens = self.decide_content_item(false)?;
          output_tokens.extend(self.route_token(token)?);

          return Ok(output_tokens);
        }

        pending.relative_depth -= 1;
      }

      P10Token::FilePreambleAndDICMPrefix { .. }
      | P10Token::FileMetaInformation { .. }
      | P10Token::End => {
        return Err(token_stream_invalid(
          "Token received inside a held back content item",
          token,
        ));
      }
    }

    pending.tokens.push(token.clone());

    Ok(vec![])
  }

  /// Decides the action for the pending content item from its concept name,
  /// if it has one, and its value type. A removed content item is discarded,
  /// and any other content item is emitted, with its held back tokens
  /// processed under its action.
  ///
  fn decide_content_item(
    &mut self,
    has_concept_name: bool,
  ) -> Result<Vec<P10Token>, P10Error> {
    let pending = self.pending_content_item.take().unwrap();

    let concept_name = if has_concept_name {
      // The held back tokens end with the concept name sequence's delimiter
      let start = pending.concept_name_start.unwrap();
      let items = sequence_items_from_tokens(
        dictionary::CONCEPT_NAME_CODE_SEQUENCE.tag,
        &pending.tokens[start + 1..pending.tokens.len() - 1],
      )?;

      items.first().cloned()
    } else {
      None
    };

    let code_string = |item: &DataSet, tag: DataElementTag| {
      item
        .get_string(tag)
        .ok()
        .map(|s| trim_padding(s).to_string())
    };
    let coding_scheme_designator = concept_name.as_ref().and_then(|code| {
      code_string(code, dictionary::CODING_SCHEME_DESIGNATOR.tag)
    });
    let code_value = concept_name
      .as_ref()
      .and_then(|code| code_string(code, dictionary::CODE_VALUE.tag));
    let value_type =
      trim_padding(&String::from_utf8_lossy(&pending.value_type)).to_string();

    let action = action_for_content_item(
      coding_scheme_designator.as_deref(),
      code_value.as_deref(),
      Some(&value_type),
      self.deidentifier.options(),
    );

    self.frames.pop();

    if action == ContentItemAction::Remove {
      self.frames.push(Frame::Discard);
      return Ok(vec![]);
    }

    self.frames.push(Frame::ContentItem {
      action,
      is_text: value_type == "TEXT",
    });

    let mut output_tokens = vec![pending.item_start];
    for token in &pending.tokens {
      output_tokens.extend(self.route_token(token)?);
    }

    Ok(output_tokens)
  }

  fn process_token(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<P10Token>, P10Error> {
    match token {
      P10Token::FilePreambleAndDICMPrefix { .. } => {
        Ok(vec![P10Token::FilePreambleAndDICMPrefix {
          preamble: Box::new([0; 128]),
        }])
      }

      P10Token::FileMetaInformation { data_set } => {
        let mut data_set = data_set.clone();

        if let Err(error) =
          deidentify_data_set(&self.deidentifier, &mut data_set, false)
        {
          return Err(self.fail(error));
        }

        Ok(vec![P10Token::FileMetaInformation { data_set }])
      }

      P10Token::SequenceStart { tag, vr, path } => {
        self.add_sequence_start_token(token, *tag, *vr, path)
      }

      P10Token::SequenceDelimiter { .. } => {
        let frame = self.pop_frame(token)?;

        match frame {
          Frame::PassThrough
          | Frame::ContentItems
          | Frame::ReplacedMeasuredValues => Ok(vec![token.clone()]),

          Frame::Discard | Frame::ReplaceValue => Ok(vec![]),

          Frame::PendingContentItem | Frame::ContentItem { .. } => {
            Err(token_stream_invalid(
              "Sequence delimiter received for a content item",
              token,
            ))
          }
        }
      }

      P10Token::SequenceItemStart { .. } | P10Token::PixelDataItem { .. } => {
        Ok(self.push_child_frame(token))
      }

      P10Token::SequenceItemDelimiter => {
        let frame = self.pop_frame(token)?;

        match frame {
          Frame::Discard => Ok(vec![]),
          _ => Ok(vec![token.clone()]),
        }
      }

      P10Token::DataElementHeader {
        tag,
        vr,
        length,
        path,
      } => self.add_data_element_header_token(token, *tag, *vr, *length, path),

      P10Token::DataElementValueBytes {
        data,
        bytes_remaining,
        ..
      } => {
        let frame = self.current_frame();

        let mut output_tokens = vec![];

        match frame {
          Frame::Discard => (),

          Frame::ReplaceValue => {
            let pending = self.pending_replacement.as_mut().ok_or_else(|| {
              token_stream_invalid(
                "Data element value bytes received with no pending replacement",
                token,
              )
            })?;

            pending.bytes.extend_from_slice(data);

            if *bytes_remaining == 0 {
              let pending = self.pending_replacement.take().unwrap();

              if let Some(value) = self.deidentifier.replacement_value(
                pending.tag,
                pending.action,
                pending.vr,
                &pending.bytes,
              ) {
                append_data_element_tokens(
                  pending.tag,
                  &value,
                  &pending.path,
                  &mut output_tokens,
                )
                .map_err(|error| self.fail(error))?;
              }
            }
          }

          _ => output_tokens.push(token.clone()),
        }

        if *bytes_remaining == 0 {
          self.pop_frame(token)?;
        }

        Ok(output_tokens)
      }

      P10Token::End => {
        let mut output_tokens = vec![];

        self.flush_pending_inserts(None, &mut output_tokens);
        output_tokens.push(P10Token::End);

        Ok(output_tokens)
      }
    }
  }

  fn add_sequence_start_token(
    &mut self,
    token: &P10Token,
    tag: DataElementTag,
    vr: ValueRepresentation,
    path: &DataSetPath,
  ) -> Result<Vec<P10Token>, P10Error> {
    let parent = self.current_frame();

    if parent == Frame::Discard {
      self.frames.push(Frame::Discard);
      return Ok(vec![]);
    }

    let mut output_tokens = vec![];

    if self.frames.is_empty() {
      self.flush_pending_inserts(Some(tag), &mut output_tokens);

      // An existing data element that records the de-identification is
      // replaced with this profile's value
      if is_recording_data_element(tag) {
        self.frames.push(Frame::Discard);
        return Ok(output_tokens);
      }
    }

    let action = action_for_data_element(tag, vr, self.deidentifier.options());

    // Encapsulated pixel data is a sequence of fragments rather than of items,
    // so it takes the action of a binary data element: it is passed through
    // when kept, and is otherwise replaced as a whole or discarded
    if vr != ValueRepresentation::Sequence {
      if action == Action::Keep {
        self.frames.push(Frame::PassThrough);
        output_tokens.push(token.clone());
      } else {
        self.frames.push(Frame::Discard);

        if let Some(value) =
          self.deidentifier.replacement_value(tag, action, vr, &[])
        {
          append_data_element_tokens(tag, &value, path, &mut output_tokens)
            .map_err(|error| self.fail(error))?;
        }
      }

      return Ok(output_tokens);
    }

    // The value sequences of a content item whose value is replaced: the code
    // of a CODE content item becomes the dummy code, the tables of a TABLE
    // content item are emptied, and the measured values of a NUM content item
    // have their numeric values replaced
    if let Frame::ContentItem {
      action: ContentItemAction::ReplaceValue,
      ..
    } = parent
    {
      if tag == dictionary::CONCEPT_CODE_SEQUENCE.tag
        || tag == dictionary::TABULATED_VALUES_SEQUENCE.tag
      {
        let items = if tag == dictionary::CONCEPT_CODE_SEQUENCE.tag {
          vec![Deidentifier::dummy_code_item()]
        } else {
          vec![]
        };

        self.frames.push(Frame::Discard);
        append_data_element_tokens(
          tag,
          &DataElementValue::new_sequence(items),
          path,
          &mut output_tokens,
        )
        .map_err(|error| self.fail(error))?;

        return Ok(output_tokens);
      }

      if tag == dictionary::MEASURED_VALUE_SEQUENCE.tag {
        self.frames.push(Frame::ReplacedMeasuredValues);
        output_tokens.push(token.clone());

        return Ok(output_tokens);
      }
    }

    match action {
      Action::Keep | Action::ReplaceUid => {
        self.frames.push(Frame::PassThrough);
        output_tokens.push(token.clone());
      }

      // The items of a cleaned sequence of content items are each held back
      // until their action is decided
      Action::Clean => {
        self.frames.push(Frame::ContentItems);
        output_tokens.push(token.clone());
      }

      Action::Remove => self.frames.push(Frame::Discard),

      Action::ReplaceWithZeroLength | Action::ReplaceWithDummy => {
        self.frames.push(Frame::Discard);
        append_data_element_tokens(
          tag,
          &DataElementValue::new_sequence(vec![]),
          path,
          &mut output_tokens,
        )
        .map_err(|error| self.fail(error))?;
      }
    }

    Ok(output_tokens)
  }

  fn add_data_element_header_token(
    &mut self,
    token: &P10Token,
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: u32,
    path: &DataSetPath,
  ) -> Result<Vec<P10Token>, P10Error> {
    let parent = self.current_frame();

    if parent == Frame::Discard {
      self.frames.push(Frame::Discard);
      return Ok(vec![]);
    }

    let mut output_tokens = vec![];

    if self.frames.is_empty() {
      self.flush_pending_inserts(Some(tag), &mut output_tokens);

      // An existing data element that records the de-identification is
      // replaced with this profile's value
      if is_recording_data_element(tag) {
        self.frames.push(Frame::Discard);
        return Ok(output_tokens);
      }
    }

    let mut action =
      action_for_data_element(tag, vr, self.deidentifier.options());

    // In a content item, the data elements that hold its value take the
    // content item's action: a kept TEXT content item keeps its text, a
    // retained content item keeps its value, and a content item whose value
    // is replaced has the value's representations replaced
    if let Frame::ContentItem {
      action: item_action,
      is_text,
    } = parent
    {
      action = match item_action {
        ContentItemAction::Keep
          if is_text && tag == dictionary::TEXT_VALUE.tag =>
        {
          Action::Keep
        }

        ContentItemAction::Retain
          if CONTENT_ITEM_LEAF_VALUE_TAGS.contains(&tag) =>
        {
          Action::Keep
        }

        ContentItemAction::ReplaceValue
          if CONTENT_ITEM_REPLACED_VALUE_TAGS.contains(&tag) =>
        {
          Action::ReplaceWithDummy
        }

        _ => action,
      };
    }

    match action {
      Action::Keep => {
        self.frames.push(Frame::PassThrough);
        output_tokens.push(token.clone());
      }

      Action::Remove => self.frames.push(Frame::Discard),

      Action::ReplaceWithZeroLength | Action::ReplaceWithDummy => {
        let value = self
          .deidentifier
          .replacement_value(tag, action, vr, &[])
          .unwrap();

        self.frames.push(Frame::Discard);
        append_data_element_tokens(tag, &value, path, &mut output_tokens)
          .map_err(|error| self.fail(error))?;
      }

      // Replacements that depend on the original value are emitted once the
      // original's bytes have been accumulated, which is capped so that the
      // memory held is bounded
      Action::ReplaceUid | Action::Clean => {
        if length > MAX_REPLACED_VALUE_LENGTH {
          return Err(self.fail(DeidentifyError::OriginalValueTooLong { tag }));
        }

        self.frames.push(Frame::ReplaceValue);
        self.pending_replacement = Some(PendingReplacement {
          tag,
          vr,
          path: path.clone(),
          action,
          bytes: vec![],
        });
      }
    }

    Ok(output_tokens)
  }

  /// Returns the frame for the current position in the token stream.
  ///
  fn current_frame(&self) -> Frame {
    self.frames.last().copied().unwrap_or(Frame::PassThrough)
  }

  /// Pushes a frame for a new sequence item or pixel data item, and returns
  /// the tokens to emit. Items in a discarded sequence are discarded, an item
  /// of a cleaned sequence of content items is held back until its action is
  /// decided, and an item of a replaced Measured Value Sequence has its
  /// numeric values replaced.
  ///
  fn push_child_frame(&mut self, token: &P10Token) -> Vec<P10Token> {
    match self.current_frame() {
      Frame::Discard => {
        self.frames.push(Frame::Discard);
        vec![]
      }

      Frame::ContentItems => {
        self.frames.push(Frame::PendingContentItem);
        self.pending_content_item = Some(PendingContentItem {
          item_start: token.clone(),
          tokens: vec![],
          relative_depth: 0,
          element_tag: None,
          value_type: vec![],
          concept_name_start: None,
        });
        vec![]
      }

      Frame::ReplacedMeasuredValues => {
        self.frames.push(Frame::ContentItem {
          action: ContentItemAction::ReplaceValue,
          is_text: false,
        });
        vec![token.clone()]
      }

      _ => {
        self.frames.push(Frame::PassThrough);
        vec![token.clone()]
      }
    }
  }

  fn pop_frame(&mut self, token: &P10Token) -> Result<Frame, P10Error> {
    self.frames.pop().ok_or_else(|| {
      token_stream_invalid("Token received when there is no open frame", token)
    })
  }

  /// Emits all pending inserts with a tag lower than the given tag, or all of
  /// them if no tag is given.
  ///
  fn flush_pending_inserts(
    &mut self,
    next_tag: Option<DataElementTag>,
    output_tokens: &mut Vec<P10Token>,
  ) {
    while let Some((tag, value)) = self.pending_inserts.pop() {
      if let Some(next_tag) = next_tag
        && tag >= next_tag
      {
        self.pending_inserts.push((tag, value));
        break;
      }

      append_data_element_tokens(
        tag,
        &value,
        &DataSetPath::new_with_data_element(tag),
        output_tokens,
      )
      .expect("Replacement value is encodable");
    }
  }
}

/// Returns whether the given data element is one of those that record the
/// de-identification in the root data set, which are set to this profile's
/// values.
///
fn is_recording_data_element(tag: DataElementTag) -> bool {
  tag == dictionary::PATIENT_IDENTITY_REMOVED.tag
    || tag == dictionary::DEIDENTIFICATION_METHOD.tag
    || tag == dictionary::DEIDENTIFICATION_METHOD_CODE_SEQUENCE.tag
    || tag == dictionary::LONGITUDINAL_TEMPORAL_INFORMATION_MODIFIED.tag
}

/// Appends the tokens for the given data element, or returns an error if its
/// value, or a value in the items of a sequence, is too long to be encoded in
/// its VR.
///
fn append_data_element_tokens(
  tag: DataElementTag,
  value: &DataElementValue,
  path: &DataSetPath,
  output_tokens: &mut Vec<P10Token>,
) -> Result<(), DeidentifyError> {
  check_encodable(tag, value)?;

  p10_token::data_element_to_tokens::<()>(tag, value, path, &mut |token| {
    output_tokens.push(token);
    Ok(())
  })
  .unwrap();

  Ok(())
}

/// Returns an error if the given value, or a value in the items of a
/// sequence, is too long to be encoded in its VR.
///
fn check_encodable(
  tag: DataElementTag,
  value: &DataElementValue,
) -> Result<(), DeidentifyError> {
  if let Ok(items) = value.sequence_items() {
    for item in items {
      for (tag, value) in item.iter() {
        check_encodable(*tag, value)?;
      }
    }

    return Ok(());
  }

  // VRs other than these have a 16-bit length in explicit VR transfer
  // syntaxes, which caps their values at 65534 bytes
  let has_32_bit_length = matches!(
    value.value_representation(),
    ValueRepresentation::OtherByteString
      | ValueRepresentation::OtherDoubleString
      | ValueRepresentation::OtherFloatString
      | ValueRepresentation::OtherLongString
      | ValueRepresentation::OtherVeryLongString
      | ValueRepresentation::OtherWordString
      | ValueRepresentation::Sequence
      | ValueRepresentation::SignedVeryLong
      | ValueRepresentation::UnlimitedCharacters
      | ValueRepresentation::Unknown
      | ValueRepresentation::UniversalResourceIdentifier
      | ValueRepresentation::UnlimitedText
      | ValueRepresentation::UnsignedVeryLong
  );

  if !has_32_bit_length && value.bytes().is_ok_and(|bytes| bytes.len() > 0xFFFE)
  {
    return Err(DeidentifyError::ValueTooLong { tag });
  }

  Ok(())
}

/// Builds the items of a sequence from the tokens that were held back for it,
/// which don't include its start and delimiter tokens.
///
fn sequence_items_from_tokens(
  tag: DataElementTag,
  tokens: &[P10Token],
) -> Result<Vec<DataSet>, P10Error> {
  let mut builder = DataSetBuilder::new();
  builder.add_token(&P10Token::SequenceStart {
    tag,
    vr: ValueRepresentation::Sequence,
    path: DataSetPath::new_with_data_element(tag),
  })?;
  builder.add_tokens(tokens)?;
  builder.add_token(&P10Token::SequenceDelimiter { tag })?;
  builder.add_token(&P10Token::End)?;

  let invalid = || P10Error::TokenStreamInvalid {
    when: "Rebuilding held back sequence".to_string(),
    details: "The held back tokens don't form a complete sequence".to_string(),
    token: P10Token::End,
  };

  let data_set = builder.final_data_set().map_err(|_| invalid())?;

  data_set
    .get_sequence_items(tag)
    .map(|items| items.to_vec())
    .map_err(|_| invalid())
}

fn token_stream_invalid(details: &str, token: &P10Token) -> P10Error {
  P10Error::TokenStreamInvalid {
    when: "Adding token to de-identify transform".to_string(),
    details: details.to_string(),
    token: token.clone(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use crate::{
    DataSetDeidentifyExtensions, ProfileOptions,
    deidentifier::{
      self,
      tests::{
        assert_data_sets_equal, content_item, modified_dates, raw_value,
      },
    },
  };

  /// Converts a data set into the stream of tokens the P10 reader would
  /// produce for it.
  ///
  fn data_set_to_tokens(data_set: &DataSet) -> Vec<P10Token> {
    let mut tokens = vec![
      P10Token::FilePreambleAndDICMPrefix {
        preamble: Box::new([1; 128]),
      },
      P10Token::FileMetaInformation {
        data_set: data_set.file_meta_information(),
      },
    ];

    let main_data_set =
      data_set.filter(|tag, _| !tag.is_file_meta_information());

    p10_token::data_elements_to_tokens::<()>(
      &main_data_set,
      &DataSetPath::new(),
      &mut |token| {
        tokens.push(token);
        Ok(())
      },
    )
    .unwrap();

    tokens.push(P10Token::End);

    tokens
  }

  fn run_transform(
    deidentifier: &Deidentifier,
    tokens: &[P10Token],
  ) -> Vec<P10Token> {
    let mut transform = P10DeidentifyTransform::new(deidentifier.clone());

    let mut output_tokens = vec![];
    for token in tokens {
      output_tokens.extend(transform.add_token(token).unwrap());
    }

    output_tokens
  }

  fn tokens_to_data_set(tokens: &[P10Token]) -> DataSet {
    let mut builder = DataSetBuilder::new();
    builder.add_tokens(tokens).unwrap();
    builder.final_data_set().unwrap()
  }

  /// Returns the data set produced by running the transform over the tokens
  /// of the given data set.
  ///
  fn transformed(deidentifier: &Deidentifier, data_set: &DataSet) -> DataSet {
    tokens_to_data_set(&run_transform(
      deidentifier,
      &data_set_to_tokens(data_set),
    ))
  }

  /// Returns a de-identifier with the Clean Structured Content Option active.
  ///
  fn clean_structured_content_deidentifier() -> Deidentifier {
    Deidentifier::with_uid_secret(b"secret".to_vec()).with_options(
      ProfileOptions {
        clean_structured_content: true,
        ..Default::default()
      },
    )
  }

  /// Returns the content items of the root Content Sequence of the data set
  /// produced by running the transform over the given data set.
  ///
  fn transformed_content_items(
    deidentifier: &Deidentifier,
    data_set: &DataSet,
  ) -> Vec<DataSet> {
    transformed(deidentifier, data_set)
      .get_sequence_items(dictionary::CONTENT_SEQUENCE.tag)
      .unwrap()
      .to_vec()
  }

  #[test]
  fn reports_errors() {
    // A replacement value that can't be encoded
    let deidentifier = Deidentifier::with_uid_secret(b"secret".to_vec());
    let mut transform = P10DeidentifyTransform::new(deidentifier);

    let uids = (0..2000).map(|i| format!("1.2.{i}")).collect::<Vec<_>>();
    let mut data_set = DataSet::new();
    data_set.insert(
      dictionary::STUDY_INSTANCE_UID.tag,
      raw_value(&dictionary::STUDY_INSTANCE_UID, &uids.join("\\")),
    );

    let mut result = Ok(vec![]);
    for token in data_set_to_tokens(&data_set) {
      result = transform.add_token(&token);
      if result.is_err() {
        break;
      }
    }

    assert!(matches!(result, Err(P10Error::OtherError { .. })));
    assert_eq!(
      transform.deidentify_error(),
      Some(&DeidentifyError::ValueTooLong {
        tag: dictionary::STUDY_INSTANCE_UID.tag
      })
    );

    // A UID longer than 64 KiB, which is rejected from its header before
    // any of its bytes are held
    let deidentifier = Deidentifier::with_uid_secret(b"secret".to_vec());
    let mut transform = P10DeidentifyTransform::new(deidentifier);
    let tag = dictionary::STUDY_INSTANCE_UID.tag;
    let result = transform.add_token(&P10Token::DataElementHeader {
      tag,
      vr: ValueRepresentation::UniqueIdentifier,
      length: 64 * 1024 + 2,
      path: DataSetPath::new_with_data_element(tag),
    });
    assert!(matches!(result, Err(P10Error::OtherError { .. })));
    assert_eq!(
      transform.deidentify_error(),
      Some(&DeidentifyError::OriginalValueTooLong { tag })
    );

    // A zero date shift
    let deidentifier = Deidentifier::with_uid_secret(b"secret".to_vec())
      .with_options(modified_dates(Some(0)));
    let mut transform = P10DeidentifyTransform::new(deidentifier);

    let result = transform.add_token(&P10Token::FilePreambleAndDICMPrefix {
      preamble: Box::new([0; 128]),
    });

    assert!(matches!(result, Err(P10Error::OtherError { .. })));
    assert_eq!(
      transform.deidentify_error(),
      Some(&DeidentifyError::ZeroDateShift)
    );
  }

  #[test]
  fn encapsulated_pixel_data_is_kept() {
    let deidentifier = Deidentifier::with_uid_secret(b"secret".to_vec());

    let pixel_data = DataElementValue::new_encapsulated_pixel_data(
      ValueRepresentation::OtherByteString,
      vec![bytes::Bytes::from_static(b"FRAGMENT")],
    )
    .unwrap();

    // Encapsulated pixel data in the root data set is kept, and goes with
    // the items of a sequence that is replaced with a dummy value
    let mut item = DataSet::new();
    item.insert(dictionary::PIXEL_DATA.tag, pixel_data.clone());

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(
        &dictionary::VERIFYING_OBSERVER_SEQUENCE,
        vec![item],
      )
      .unwrap();
    data_set.insert(dictionary::PIXEL_DATA.tag, pixel_data.clone());

    let output = transformed(&deidentifier, &data_set);
    assert_eq!(
      output.get_value(dictionary::PIXEL_DATA.tag),
      Ok(&pixel_data)
    );
    assert_eq!(
      output
        .get_sequence_items(dictionary::VERIFYING_OBSERVER_SEQUENCE.tag)
        .unwrap(),
      &[]
    );
  }

  #[test]
  fn recording_data_elements_are_set() {
    let deidentifier = Deidentifier::with_uid_secret(b"secret".to_vec())
      .with_options(ProfileOptions {
        retain_uids: true,
        ..Default::default()
      });

    // Existing values, including ones that arrive after a higher tag by
    // which point this profile's values have been emitted, are replaced
    // rather than duplicated
    let mut code = DataSet::new();
    code
      .insert_string_value(&dictionary::CODE_VALUE, &["113101"])
      .unwrap();
    code
      .insert_string_value(&dictionary::CODING_SCHEME_DESIGNATOR, &["DCM"])
      .unwrap();

    let mut first = DataSet::new();
    first
      .insert_string_value(&dictionary::STUDY_INSTANCE_UID, &["1.2.3"])
      .unwrap();

    let mut late = DataSet::new();
    late
      .insert_string_value(&dictionary::PATIENT_IDENTITY_REMOVED, &["NO"])
      .unwrap();
    late
      .insert_string_value(&dictionary::DEIDENTIFICATION_METHOD, &["Prior"])
      .unwrap();
    late
      .insert_sequence_value(
        &dictionary::DEIDENTIFICATION_METHOD_CODE_SEQUENCE,
        vec![code],
      )
      .unwrap();
    late
      .insert_string_value(
        &dictionary::LONGITUDINAL_TEMPORAL_INFORMATION_MODIFIED,
        &["UNMODIFIED"],
      )
      .unwrap();

    for data_sets in [vec![&late], vec![&first, &late]] {
      let mut tokens = vec![];
      for data_set in data_sets {
        p10_token::data_elements_to_tokens::<()>(
          data_set,
          &DataSetPath::new(),
          &mut |token| {
            tokens.push(token);
            Ok(())
          },
        )
        .unwrap();
      }
      tokens.push(P10Token::End);

      let output_tokens = run_transform(&deidentifier, &tokens);

      let count = |expected_tag: DataElementTag| {
        output_tokens
          .iter()
          .filter(|token| match token {
            P10Token::DataElementHeader { tag, .. }
            | P10Token::SequenceStart { tag, .. } => *tag == expected_tag,
            _ => false,
          })
          .count()
      };
      for tag in [
        dictionary::PATIENT_IDENTITY_REMOVED.tag,
        dictionary::DEIDENTIFICATION_METHOD.tag,
        dictionary::DEIDENTIFICATION_METHOD_CODE_SEQUENCE.tag,
        dictionary::LONGITUDINAL_TEMPORAL_INFORMATION_MODIFIED.tag,
      ] {
        assert_eq!(count(tag), 1, "{tag}");
      }

      let result = tokens_to_data_set(&output_tokens);
      assert_eq!(
        result.get_string(dictionary::PATIENT_IDENTITY_REMOVED.tag),
        Ok("YES")
      );
      assert_eq!(
        result
          .get_strings(dictionary::DEIDENTIFICATION_METHOD.tag)
          .unwrap(),
        vec![
          "Basic Application Confidentiality Profile",
          "Retain UIDs Option"
        ]
      );
      assert_eq!(
        result
          .get_sequence_items(
            dictionary::DEIDENTIFICATION_METHOD_CODE_SEQUENCE.tag
          )
          .unwrap()
          .len(),
        2
      );
      assert_eq!(
        result.get_string(
          dictionary::LONGITUDINAL_TEMPORAL_INFORMATION_MODIFIED.tag
        ),
        Ok("REMOVED")
      );
    }
  }

  #[test]
  fn content_item_with_out_of_order_data_elements_is_decided_at_its_end() {
    let deidentifier = clean_structured_content_deidentifier();

    let mut item = content_item("DCM", "121022", "TEXT");
    item.insert(
      dictionary::TEXT_VALUE.tag,
      raw_value(&dictionary::TEXT_VALUE, "SECRET-ACCESSION"),
    );

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(&dictionary::CONTENT_SEQUENCE, vec![item])
      .unwrap();

    // Move the Text Value tokens to before the Concept Name Code Sequence
    let mut tokens = data_set_to_tokens(&data_set);
    let text_value_start = tokens
      .iter()
      .position(|token| {
        matches!(token, P10Token::DataElementHeader { tag, .. } if *tag == dictionary::TEXT_VALUE.tag)
      })
      .unwrap();
    let text_value_tokens: Vec<P10Token> = tokens
      .drain(text_value_start..text_value_start + 2)
      .collect();
    let concept_name_start = tokens
      .iter()
      .position(|token| {
        matches!(token, P10Token::SequenceStart { tag, .. } if *tag == dictionary::CONCEPT_NAME_CODE_SEQUENCE.tag)
      })
      .unwrap();
    tokens.splice(concept_name_start..concept_name_start, text_value_tokens);

    let mut expected = data_set.clone();
    expected.deidentify(&deidentifier).unwrap();

    let output_tokens = run_transform(&deidentifier, &tokens);
    assert_data_sets_equal(&tokens_to_data_set(&output_tokens), &expected);
    assert!(!output_tokens.iter().any(|token| {
      matches!(token, P10Token::DataElementValueBytes { data, .. } if data.as_ref() == b"SECRET-ACCESSION")
    }));
  }

  #[test]
  fn nested_content_items_held_back_with_their_parent() {
    let deidentifier = clean_structured_content_deidentifier();

    // An SR content item that also carries an Acquisition Context Sequence,
    // which precedes its concept name in tag order and so is held back with
    // it, and a container with no concept name whose nested content is held
    // back until the container ends
    let mut accession_number = content_item("DCM", "121022", "TEXT");
    accession_number.insert(
      dictionary::TEXT_VALUE.tag,
      raw_value(&dictionary::TEXT_VALUE, "ACC123"),
    );

    let mut finding = content_item("99TEST", "1234", "TEXT");
    finding.insert(
      dictionary::TEXT_VALUE.tag,
      raw_value(&dictionary::TEXT_VALUE, "A mass"),
    );
    finding
      .insert_sequence_value(
        &dictionary::ACQUISITION_CONTEXT_SEQUENCE,
        vec![accession_number.clone()],
      )
      .unwrap();

    let mut container = DataSet::new();
    container
      .insert_string_value(&dictionary::RELATIONSHIP_TYPE, &["CONTAINS"])
      .unwrap();
    container
      .insert_string_value(&dictionary::VALUE_TYPE, &["CONTAINER"])
      .unwrap();
    container
      .insert_sequence_value(
        &dictionary::CONTENT_SEQUENCE,
        vec![accession_number, finding.clone()],
      )
      .unwrap();

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(
        &dictionary::CONTENT_SEQUENCE,
        vec![finding, container],
      )
      .unwrap();

    // The accession numbers are removed wherever they are nested
    let items = transformed_content_items(&deidentifier, &data_set);
    assert_eq!(
      items[0]
        .get_sequence_items(dictionary::ACQUISITION_CONTEXT_SEQUENCE.tag)
        .unwrap(),
      &[]
    );
    let container_items = items[1]
      .get_sequence_items(dictionary::CONTENT_SEQUENCE.tag)
      .unwrap();
    assert_eq!(container_items.len(), 1);
    assert_eq!(
      container_items[0]
        .get_string(dictionary::TEXT_VALUE.tag)
        .unwrap(),
      "A mass"
    );
  }

  #[test]
  fn numeric_modifier_of_replaced_content_item_is_kept() {
    let deidentifier = clean_structured_content_deidentifier();

    // An unlisted NUMERIC modifier sits three frames below its parent, like a
    // Numeric Value in a Measured Value Sequence does, but must not be treated
    // as the parent's value
    let mut units = DataSet::new();
    units
      .insert_string_value(&dictionary::CODE_VALUE, &["mm"])
      .unwrap();
    units
      .insert_string_value(&dictionary::CODING_SCHEME_DESIGNATOR, &["UCUM"])
      .unwrap();
    units
      .insert_string_value(&dictionary::CODE_MEANING, &["millimeter"])
      .unwrap();

    let mut modifier = content_item("99TEST", "7777", "NUMERIC");
    modifier.insert(
      dictionary::NUMERIC_VALUE.tag,
      raw_value(&dictionary::NUMERIC_VALUE, "3"),
    );
    modifier
      .insert_sequence_value(
        &dictionary::MEASUREMENT_UNITS_CODE_SEQUENCE,
        vec![units],
      )
      .unwrap();

    // Component ID has its value replaced
    let mut component_id = content_item("DCM", "112347", "TEXT");
    component_id.insert(
      dictionary::TEXT_VALUE.tag,
      raw_value(&dictionary::TEXT_VALUE, "Component 7"),
    );
    component_id
      .insert_sequence_value(
        &dictionary::CONTENT_ITEM_MODIFIER_SEQUENCE,
        vec![modifier.clone()],
      )
      .unwrap();

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(
        &dictionary::ACQUISITION_CONTEXT_SEQUENCE,
        vec![component_id],
      )
      .unwrap();

    let result = transformed(&deidentifier, &data_set);
    let item = &result
      .get_sequence_items(dictionary::ACQUISITION_CONTEXT_SEQUENCE.tag)
      .unwrap()[0];
    assert_eq!(
      item.get_string(dictionary::TEXT_VALUE.tag).unwrap(),
      deidentifier::DUMMY_TEXT
    );
    assert_eq!(
      item
        .get_sequence_items(dictionary::CONTENT_ITEM_MODIFIER_SEQUENCE.tag)
        .unwrap(),
      &[modifier]
    );
  }

  #[test]
  fn only_first_concept_name_item_is_consulted() {
    let deidentifier = clean_structured_content_deidentifier();

    // The first item is a concept name that is removed, the second is unlisted
    let mut item = content_item("DCM", "121022", "TEXT");
    let mut concept_name = item
      .get_sequence_items(dictionary::CONCEPT_NAME_CODE_SEQUENCE.tag)
      .unwrap()
      .to_vec();
    concept_name.extend(
      content_item("99TEST", "1234", "TEXT")
        .get_sequence_items(dictionary::CONCEPT_NAME_CODE_SEQUENCE.tag)
        .unwrap()
        .iter()
        .cloned(),
    );
    item.insert(
      dictionary::CONCEPT_NAME_CODE_SEQUENCE.tag,
      DataElementValue::new_sequence(concept_name),
    );
    item.insert(
      dictionary::TEXT_VALUE.tag,
      raw_value(&dictionary::TEXT_VALUE, "ACC123"),
    );

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(&dictionary::CONTENT_SEQUENCE, vec![item])
      .unwrap();

    // The content item is removed as its first concept name is listed
    assert!(transformed_content_items(&deidentifier, &data_set).is_empty());

    // A content item with an empty concept name is kept
    let mut item = DataSet::new();
    item
      .insert_string_value(&dictionary::VALUE_TYPE, &["TEXT"])
      .unwrap();
    item
      .insert_sequence_value(&dictionary::CONCEPT_NAME_CODE_SEQUENCE, vec![])
      .unwrap();
    item.insert(
      dictionary::TEXT_VALUE.tag,
      raw_value(&dictionary::TEXT_VALUE, "A mass"),
    );

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(&dictionary::CONTENT_SEQUENCE, vec![item.clone()])
      .unwrap();
    assert_eq!(
      transformed_content_items(&deidentifier, &data_set),
      vec![item]
    );
  }

  #[test]
  fn values_split_across_value_bytes_tokens() {
    // A Code Value split in two is still matched against the table
    let deidentifier = clean_structured_content_deidentifier();

    let mut item = content_item("DCM", "121022", "TEXT");
    item.insert(
      dictionary::TEXT_VALUE.tag,
      raw_value(&dictionary::TEXT_VALUE, "ACC123"),
    );

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(&dictionary::CONTENT_SEQUENCE, vec![item])
      .unwrap();

    let tokens: Vec<P10Token> = data_set_to_tokens(&data_set)
      .into_iter()
      .flat_map(|token| match &token {
        P10Token::DataElementValueBytes {
          tag,
          vr,
          data,
          bytes_remaining: 0,
        } if *tag == dictionary::CODE_VALUE.tag && data.len() > 1 => {
          let (first, second) = data.split_at(data.len() / 2);
          vec![
            P10Token::DataElementValueBytes {
              tag: *tag,
              vr: *vr,
              data: first.to_vec().into(),
              bytes_remaining: second.len() as u32,
            },
            P10Token::DataElementValueBytes {
              tag: *tag,
              vr: *vr,
              data: second.to_vec().into(),
              bytes_remaining: 0,
            },
          ]
        }
        _ => vec![token],
      })
      .collect();

    let mut expected = data_set.clone();
    expected.deidentify(&deidentifier).unwrap();

    let output_tokens = run_transform(&deidentifier, &tokens);
    assert_data_sets_equal(&tokens_to_data_set(&output_tokens), &expected);
    assert_eq!(
      expected
        .get_sequence_items(dictionary::CONTENT_SEQUENCE.tag)
        .unwrap(),
      &[]
    );

    // A UID split in two is replaced as a whole
    let deidentifier = Deidentifier::with_uid_secret(b"secret".to_vec());

    let tag = dictionary::SOP_INSTANCE_UID.tag;
    let vr = ValueRepresentation::UniqueIdentifier;
    let path = DataSetPath::new_with_data_element(tag);

    let tokens = vec![
      P10Token::DataElementHeader {
        tag,
        vr,
        length: 8,
        path: path.clone(),
      },
      P10Token::DataElementValueBytes {
        tag,
        vr,
        data: b"1.2.".to_vec().into(),
        bytes_remaining: 4,
      },
      P10Token::DataElementValueBytes {
        tag,
        vr,
        data: b"3\\1.".to_vec().into(),
        bytes_remaining: 0,
      },
    ];

    let output_tokens = run_transform(&deidentifier, &tokens);

    let expected_value = DataElementValue::new_unique_identifier(&[
      &deidentifier.map_uid("1.2.3"),
      &deidentifier.map_uid("1."),
    ])
    .unwrap();

    let mut expected_tokens = vec![];
    append_data_element_tokens(
      tag,
      &expected_value,
      &path,
      &mut expected_tokens,
    )
    .unwrap();

    assert_eq!(output_tokens, expected_tokens);
  }

  #[test]
  fn content_item_tokens_stream_once_the_concept_name_has_arrived() {
    let deidentifier = clean_structured_content_deidentifier();

    // An unlisted TEXT content item: its tokens up to and including the
    // concept name are held back, and everything after the concept name is
    // emitted as it arrives rather than when the content item ends
    let mut item = content_item("99TEST", "1234", "TEXT");
    item.insert(
      dictionary::TEXT_VALUE.tag,
      raw_value(&dictionary::TEXT_VALUE, "A mass"),
    );
    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(&dictionary::CONTENT_SEQUENCE, vec![item])
      .unwrap();

    let mut transform = P10DeidentifyTransform::new(deidentifier);
    let mut concept_name_seen = false;
    let mut item_ended = false;

    for token in data_set_to_tokens(&data_set) {
      let output_tokens = transform.add_token(&token).unwrap();

      if let P10Token::SequenceDelimiter { tag } = &token
        && *tag == dictionary::CONCEPT_NAME_CODE_SEQUENCE.tag
      {
        concept_name_seen = true;
      }

      if concept_name_seen
        && !item_ended
        && matches!(
          token,
          P10Token::DataElementHeader { .. }
            | P10Token::DataElementValueBytes { .. }
        )
      {
        assert!(!output_tokens.is_empty(), "{token:?} was held back");
      }

      if matches!(token, P10Token::SequenceItemDelimiter) {
        item_ended = true;
      }
    }
  }

  #[test]
  fn removed_sequence_discards_nested_content() {
    let deidentifier = Deidentifier::with_uid_secret(b"secret".to_vec());

    let mut item = DataSet::new();
    item.insert(
      dictionary::PATIENT_NAME.tag,
      raw_value(&dictionary::PATIENT_NAME, "Doe^Jane"),
    );

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(
        &dictionary::REQUEST_ATTRIBUTES_SEQUENCE,
        vec![item],
      )
      .unwrap();

    let result = transformed(&deidentifier, &data_set);
    assert!(!result.has(dictionary::REQUEST_ATTRIBUTES_SEQUENCE.tag));
    assert!(!result.has(dictionary::PATIENT_NAME.tag));
  }

  #[test]
  fn date_shift_is_consistent_with_deidentifier() {
    let deidentifier = Deidentifier::with_uid_secret(b"secret".to_vec())
      .with_options(modified_dates(Some(-1)));

    let mut data_set = DataSet::new();
    data_set.insert(
      dictionary::STUDY_DATE.tag,
      raw_value(&dictionary::STUDY_DATE, "20240301"),
    );

    assert_eq!(
      transformed(&deidentifier, &data_set)
        .get_string(dictionary::STUDY_DATE.tag)
        .unwrap(),
      "20240229"
    );
  }
}
