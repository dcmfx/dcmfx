#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use bytes::Bytes;
use dcmfx_core::{
  DataElementTag, DataElementValue, DataSet, StructuredAge, StructuredDate,
  StructuredDateTime, StructuredPersonName, StructuredTime,
  ValueRepresentation,
  data_element_value::{
    age_string::AgeUnit, person_name::PersonNameComponents,
  },
  dictionary,
};
use sha2::{Digest, Sha256};

use crate::{
  Action, DeidentificationMethodCode, LongitudinalTemporalInformation,
  ProfileOptions, date_shift, utils::trim_trailing_padding,
};

/// The value used when replacing a text value with a dummy value.
///
pub const DUMMY_TEXT: &str = "DEIDENTIFIED";

/// The code that replaces the code of a CODE content item whose value is
/// replaced under the Clean Structured Content Option: the SNOMED CT code for
/// Unknown, as a code value, coding scheme designator, and code meaning. A
/// real code is used so that the replacement is a valid code that conveys
/// that the original isn't known, rather than text that isn't a code.
///
pub const DUMMY_CODE: (&str, &str, &str) = ("261665006", "SCT", "Unknown");

/// Applies the Basic Application Level Confidentiality Profile defined in Annex
/// E of DICOM PS3.15, and any active [`ProfileOptions`], to data sets. See the
/// crate documentation for details of what the profile and its options do.
///
/// A de-identifier holds the secret used to generate replacement UIDs. The
/// same original UID always maps to the same replacement UID for a given
/// secret, so UIDs stay consistent, and references between instances remain
/// intact, across every data set that is de-identified with the same secret.
/// UIDs produced with different secrets are unrelated. The secret is also used
/// to derive the number of days to shift dates by when the Retain Longitudinal
/// Temporal Information With Modified Dates Option is active without an
/// explicit shift.
///
#[derive(Clone, PartialEq, Eq)]
pub struct Deidentifier {
  uid_secret: Vec<u8>,
  options: ProfileOptions,

  /// The number of days to shift dates by under the Retain Longitudinal
  /// Temporal Information With Modified Dates Option. Derived from the secret
  /// when the option doesn't specify a shift.
  date_shift_days: Option<i32>,
}

/// The secret is redacted so that it can't leak into logs or error messages.
///
impl core::fmt::Debug for Deidentifier {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("Deidentifier")
      .field("uid_secret", &"<redacted>")
      .field("options", &self.options)
      .field("date_shift_days", &self.date_shift_days)
      .finish()
  }
}

#[cfg(all(
  feature = "std",
  not(all(target_family = "wasm", target_os = "unknown"))
))]
impl Default for Deidentifier {
  fn default() -> Self {
    Self::new()
  }
}

impl Deidentifier {
  /// Creates a new de-identifier with a randomly generated UID secret and no
  /// options active. UIDs are consistent across all data sets de-identified by
  /// this de-identifier and its clones, but are unrelated to those produced by
  /// any other de-identifier.
  ///
  /// Use [`Deidentifier::with_uid_secret()`] when UIDs need to be consistent
  /// across separate runs, e.g. when related instances are de-identified at
  /// different times.
  ///
  /// The 32-byte secret is sourced from the operating system's random number
  /// generator. This function isn't available on `wasm32-unknown-unknown`,
  /// which has no entropy source, so a secret must be supplied there using
  /// [`Deidentifier::with_uid_secret()`].
  ///
  /// # Panics
  ///
  /// Panics if the operating system's random number generator fails.
  ///
  #[cfg(all(
    feature = "std",
    not(all(target_family = "wasm", target_os = "unknown"))
  ))]
  pub fn new() -> Self {
    let mut uid_secret = vec![0u8; 32];

    getrandom::fill(&mut uid_secret)
      .expect("Failed generating a random UID secret");

    Self::with_uid_secret(uid_secret)
  }

  /// Creates a new de-identifier that uses the given secret to generate
  /// replacement UIDs, with no options active. Replacement UIDs are derived
  /// from the original UID and the secret, so de-identifiers created with the
  /// same secret produce the same replacement UIDs.
  ///
  /// The secret should be kept private. Anyone who knows it, and has the
  /// original UIDs, is able to match de-identified instances back to their
  /// originals.
  ///
  pub fn with_uid_secret(uid_secret: impl Into<Vec<u8>>) -> Self {
    Self {
      uid_secret: uid_secret.into(),
      options: ProfileOptions::default(),
      date_shift_days: None,
    }
  }

  /// Sets the options to the Basic Profile that this de-identifier applies.
  ///
  pub fn with_options(mut self, options: ProfileOptions) -> Self {
    self.options = options;
    self.date_shift_days = self.derive_date_shift_days();
    self
  }

  /// Returns the secret used by this de-identifier to generate replacement UIDs.
  ///
  pub fn uid_secret(&self) -> &[u8] {
    &self.uid_secret
  }

  /// Returns the options to the Basic Profile that this de-identifier applies.
  ///
  pub fn options(&self) -> &ProfileOptions {
    &self.options
  }

  /// Returns the number of days that dates are shifted by, or `None` if the
  /// Retain Longitudinal Temporal Information With Modified Dates Option isn't
  /// active. When that option doesn't specify a shift, the shift is derived
  /// from a SHA-256 hash of this de-identifier's secret and lies between 1 and
  /// 3650 days into the past.
  ///
  pub fn date_shift_days(&self) -> Option<i32> {
    self.date_shift_days
  }

  fn derive_date_shift_days(&self) -> Option<i32> {
    match self.options.retain_longitudinal_temporal_information {
      Some(LongitudinalTemporalInformation::ModifiedDates {
        date_shift_days,
      }) => Some(date_shift_days.unwrap_or_else(|| {
        let mut hasher = Sha256::new();
        hasher.update(&self.uid_secret);
        hasher.update([1u8]);

        let hash = hasher.finalize();
        let n = u32::from_be_bytes(hash[0..4].try_into().unwrap());

        -((n % 3650) as i32 + 1)
      })),

      _ => None,
    }
  }

  /// Returns the replacement UID for the given original UID. The replacement is
  /// a UUID-derived UID of the form `2.25.<integer>`, where the integer is
  /// derived from a SHA-256 hash of this de-identifier's secret and the original
  /// UID. The same original UID always produces the same replacement UID.
  ///
  pub fn map_uid(&self, uid: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(&self.uid_secret);
    hasher.update([0u8]);
    hasher.update(uid.as_bytes());

    let hash = hasher.finalize();
    let n = u128::from_be_bytes(uuid_from_hash(&hash));

    format!("2.25.{n}")
  }

  /// Returns the value that replaces a data element's value under the given
  /// action. `bytes` is the original value's bytes, which is only used when
  /// replacing UIDs or cleaning dates. Returns `None` for actions that don't
  /// replace the value.
  ///
  /// The dummy value for a sequence is an empty sequence. The dummy values for
  /// Context Group Version (0008,0106) and Context Group Local Version
  /// (0008,0107) are dates with day precision, the dummy value for Timezone
  /// Offset From UTC (0008,0201) has the syntax of an offset, and the dummy
  /// value for Rational Denominator Value (0040,A163) is one, as a denominator
  /// must be nonzero. See [`Self::dummy_value()`] for the others.
  ///
  pub(crate) fn replacement_value(
    &self,
    tag: DataElementTag,
    action: Action,
    vr: ValueRepresentation,
    bytes: &[u8],
  ) -> Option<DataElementValue> {
    match action {
      Action::Keep | Action::Remove => None,

      Action::ReplaceWithZeroLength => {
        if vr == ValueRepresentation::Sequence {
          Some(DataElementValue::new_sequence(vec![]))
        } else {
          Some(DataElementValue::new_binary_unchecked(vr, Bytes::new()))
        }
      }

      Action::ReplaceWithDummy
        if tag == dictionary::CONTEXT_GROUP_VERSION.tag
          || tag == dictionary::CONTEXT_GROUP_LOCAL_VERSION.tag =>
      {
        Some(DataElementValue::new_binary_unchecked(
          vr,
          Bytes::from_static(b"19000101"),
        ))
      }

      Action::ReplaceWithDummy
        if tag == dictionary::TIMEZONE_OFFSET_FROM_UTC.tag =>
      {
        Some(DataElementValue::new_short_string(&["+0000"]).unwrap())
      }

      Action::ReplaceWithDummy
        if tag == dictionary::RATIONAL_DENOMINATOR_VALUE.tag
          && vr == ValueRepresentation::UnsignedLong =>
      {
        Some(DataElementValue::new_unsigned_long(&[1]).unwrap())
      }

      Action::ReplaceWithDummy => Some(self.dummy_value(vr)),

      Action::ReplaceUid => Some(self.replace_uids(vr, bytes)),

      // Cleaning of a non-sequence value is the shifting of dates, or of the
      // PTP timestamp in Frame Origin Timestamp (0034,0007). If the value
      // can't be shifted it is replaced with a dummy value. Cleaning of a
      // sequence is the cleaning of its content items, which is handled
      // before this is reached.
      Action::Clean => Some(
        self
          .date_shift_days
          .and_then(|days| {
            if tag == dictionary::FRAME_ORIGIN_TIMESTAMP.tag {
              date_shift::shift_ptp_timestamp(bytes, days).map(|shifted| {
                DataElementValue::new_binary_unchecked(vr, shifted.into())
              })
            } else {
              date_shift::shift_value(vr, bytes, days)
            }
          })
          .unwrap_or_else(|| self.dummy_value(vr)),
      ),
    }
  }

  /// Replaces each of the UIDs in the given value bytes using
  /// [`Self::map_uid()`] and returns a new value of the given VR. Empty UIDs
  /// are left empty, as they carry no information and giving them a value
  /// would invent a reference that isn't in the original. UIDs defined by the
  /// DICOM standard, which start with `1.2.840.10008.`, such as the well-known
  /// color palettes and SOP instances, are left unchanged as they identify
  /// nothing specific to the original.
  ///
  fn replace_uids(
    &self,
    vr: ValueRepresentation,
    bytes: &[u8],
  ) -> DataElementValue {
    // A backslash separates the values of a UI value, but is text in a
    // single-valued VR such as the ST of HL7 Instance Identifier (0040,E001),
    // whose whole value is one identifier
    let values: Vec<&[u8]> = if vr == ValueRepresentation::UniqueIdentifier {
      bytes.split(|b| *b == b'\\').collect()
    } else {
      vec![bytes]
    };

    let uids: Vec<String> = values
      .into_iter()
      .map(|uid| {
        let uid = trim_trailing_padding(uid);
        let uid = &uid[uid
          .iter()
          .position(|b| *b != 0x00 && *b != 0x20)
          .unwrap_or(uid.len())..];

        if uid.is_empty() {
          String::new()
        } else if vr == ValueRepresentation::UniqueIdentifier
          && uid.starts_with(b"1.2.840.10008.")
        {
          String::from_utf8_lossy(uid).into_owned()
        } else {
          self.map_uid(&String::from_utf8_lossy(uid))
        }
      })
      .collect();

    let mut bytes = uids.join("\\").into_bytes();

    if bytes.len() % 2 == 1 {
      if vr.is_string() {
        vr.pad_bytes_to_even_length(&mut bytes);
      } else {
        bytes.push(0);
      }
    }

    DataElementValue::new_binary_unchecked(vr, bytes.into())
  }

  /// Returns the item of a code sequence that holds [`DUMMY_CODE`], which
  /// replaces the code of a CODE content item whose value is replaced.
  ///
  pub fn dummy_code_item() -> DataSet {
    let (code_value, coding_scheme_designator, code_meaning) = DUMMY_CODE;

    Self::code_item(code_value, coding_scheme_designator, code_meaning)
  }

  /// Returns a code sequence item with the given code value, coding scheme
  /// designator, and code meaning.
  ///
  pub(crate) fn code_item(
    code_value: &str,
    coding_scheme_designator: &str,
    code_meaning: &str,
  ) -> DataSet {
    let mut item = DataSet::new();
    item
      .insert_string_value(&dictionary::CODE_VALUE, &[code_value])
      .unwrap();
    item
      .insert_string_value(
        &dictionary::CODING_SCHEME_DESIGNATOR,
        &[coding_scheme_designator],
      )
      .unwrap();
    item
      .insert_string_value(&dictionary::CODE_MEANING, &[code_meaning])
      .unwrap();

    item
  }

  /// Returns a non-zero length dummy value for the given VR that doesn't
  /// contain any identifying information.
  ///
  /// Text values are [`DUMMY_TEXT`], dates are 1900-01-01, times are midnight,
  /// numbers are zero, binary values are a single zero, sequences are empty,
  /// and UIDs are the replacement for an empty UID.
  ///
  pub fn dummy_value(&self, vr: ValueRepresentation) -> DataElementValue {
    match vr {
      ValueRepresentation::AgeString => {
        DataElementValue::new_age_string(&StructuredAge {
          number: 0,
          unit: AgeUnit::Years,
        })
      }

      ValueRepresentation::ApplicationEntity => {
        DataElementValue::new_application_entity(DUMMY_TEXT)
      }

      ValueRepresentation::AttributeTag => {
        DataElementValue::new_attribute_tag(&[DataElementTag::new(0, 0)])
      }

      ValueRepresentation::CodeString => {
        DataElementValue::new_code_string(&[DUMMY_TEXT])
      }

      ValueRepresentation::Date => {
        DataElementValue::new_date(&StructuredDate {
          year: 1900,
          month: 1,
          day: 1,
        })
      }

      ValueRepresentation::DateTime => {
        DataElementValue::new_date_time(&StructuredDateTime {
          year: 1900,
          month: Some(1),
          day: Some(1),
          hour: Some(0),
          minute: Some(0),
          second: None,
          time_zone_offset: None,
        })
      }

      ValueRepresentation::DecimalString => {
        DataElementValue::new_decimal_string(&[0.0])
      }

      ValueRepresentation::FloatingPointDouble => {
        DataElementValue::new_floating_point_double(&[0.0])
      }

      ValueRepresentation::FloatingPointSingle => {
        DataElementValue::new_floating_point_single(&[0.0])
      }

      ValueRepresentation::IntegerString => {
        DataElementValue::new_integer_string(&[0])
      }

      ValueRepresentation::LongString => {
        DataElementValue::new_long_string(&[DUMMY_TEXT])
      }

      ValueRepresentation::LongText => {
        DataElementValue::new_long_text(DUMMY_TEXT)
      }

      ValueRepresentation::OtherByteString => {
        DataElementValue::new_other_byte_string(vec![0, 0])
      }

      ValueRepresentation::OtherDoubleString => {
        DataElementValue::new_other_double_string(&[0.0])
      }

      ValueRepresentation::OtherFloatString => {
        DataElementValue::new_other_float_string(&[0.0])
      }

      ValueRepresentation::OtherLongString => {
        DataElementValue::new_other_long_string(vec![0; 4])
      }

      ValueRepresentation::OtherVeryLongString => {
        DataElementValue::new_other_very_long_string(vec![0; 8])
      }

      ValueRepresentation::OtherWordString => {
        DataElementValue::new_other_word_string(vec![0, 0])
      }

      ValueRepresentation::PersonName => {
        DataElementValue::new_person_name(&[StructuredPersonName {
          alphabetic: Some(PersonNameComponents {
            last_name: DUMMY_TEXT.to_string(),
            first_name: "".to_string(),
            middle_name: "".to_string(),
            prefix: "".to_string(),
            suffix: "".to_string(),
          }),
          ideographic: None,
          phonetic: None,
        }])
      }

      ValueRepresentation::Sequence => {
        Ok(DataElementValue::new_sequence(vec![]))
      }

      ValueRepresentation::ShortString => {
        DataElementValue::new_short_string(&[DUMMY_TEXT])
      }

      ValueRepresentation::ShortText => {
        DataElementValue::new_short_text(DUMMY_TEXT)
      }

      ValueRepresentation::SignedLong => {
        DataElementValue::new_signed_long(&[0])
      }

      ValueRepresentation::SignedShort => {
        DataElementValue::new_signed_short(&[0])
      }

      ValueRepresentation::SignedVeryLong => {
        DataElementValue::new_signed_very_long(&[0])
      }

      ValueRepresentation::Time => {
        DataElementValue::new_time(&StructuredTime {
          hour: 0,
          minute: Some(0),
          second: None,
        })
      }

      ValueRepresentation::UniqueIdentifier => {
        DataElementValue::new_unique_identifier(&[&self.map_uid("")])
      }

      ValueRepresentation::UniversalResourceIdentifier => {
        DataElementValue::new_universal_resource_identifier(
          "https://de-identified.invalid",
        )
      }

      ValueRepresentation::Unknown => DataElementValue::new_unknown(vec![0, 0]),

      ValueRepresentation::UnlimitedCharacters => {
        DataElementValue::new_unlimited_characters(&[DUMMY_TEXT])
      }

      ValueRepresentation::UnlimitedText => {
        DataElementValue::new_unlimited_text(DUMMY_TEXT)
      }

      ValueRepresentation::UnsignedLong => {
        DataElementValue::new_unsigned_long(&[0])
      }

      ValueRepresentation::UnsignedShort => {
        DataElementValue::new_unsigned_short(&[0])
      }

      ValueRepresentation::UnsignedVeryLong => {
        DataElementValue::new_unsigned_very_long(&[0])
      }
    }
    .unwrap()
  }
}

/// Returns the first 16 bytes of a hash as a valid version 4 (random) UUID,
/// i.e. with the version and variant bits of RFC 4122 set, as the 2.25 UIDs
/// of PS3.5 B.2 are UUIDs.
///
fn uuid_from_hash(hash: &[u8]) -> [u8; 16] {
  let mut uuid: [u8; 16] = hash[0..16].try_into().unwrap();
  uuid[6] = (uuid[6] & 0x0F) | 0x40;
  uuid[8] = (uuid[8] & 0x3F) | 0x80;
  uuid
}

/// Returns the value for Patient Identity Removed (0012,0062).
///
pub(crate) fn patient_identity_removed_value() -> DataElementValue {
  DataElementValue::new_code_string(&["YES"]).unwrap()
}

/// Returns the value for Longitudinal Temporal Information Modified
/// (0028,0303).
///
pub(crate) fn longitudinal_temporal_information_modified_value(
  options: &ProfileOptions,
) -> DataElementValue {
  DataElementValue::new_code_string(&[
    options.longitudinal_temporal_information_modified()
  ])
  .unwrap()
}

/// Returns the value for De-identification Method (0012,0063): the meaning of
/// the code for the Basic Profile and each active option.
///
pub(crate) fn deidentification_method_value(
  options: &ProfileOptions,
) -> DataElementValue {
  let codes = options.deidentification_method_codes();
  let meanings: Vec<&str> =
    codes.iter().map(|code| code.code_meaning).collect();

  DataElementValue::new_long_string(&meanings).unwrap()
}

/// Returns the item for De-identification Method Code Sequence (0012,0064)
/// that specifies the given code.
///
pub(crate) fn deidentification_method_code_item(
  code: &DeidentificationMethodCode,
) -> DataSet {
  let mut item = DataSet::new();

  item
    .insert_string_value(&dictionary::CODE_VALUE, &[code.code_value])
    .unwrap();
  item
    .insert_string_value(&dictionary::CODING_SCHEME_DESIGNATOR, &["DCM"])
    .unwrap();
  item
    .insert_string_value(&dictionary::CODE_MEANING, &[code.code_meaning])
    .unwrap();

  item
}

/// Returns the value for De-identification Method Code Sequence (0012,0064):
/// an item for the Basic Profile and each active option.
///
pub(crate) fn deidentification_method_code_sequence_value(
  options: &ProfileOptions,
) -> DataElementValue {
  DataElementValue::new_sequence(
    options
      .deidentification_method_codes()
      .iter()
      .map(deidentification_method_code_item)
      .collect(),
  )
}

#[cfg(test)]
pub(crate) mod tests {
  use super::*;

  use crate::{DataSetDeidentifyExtensions, DeidentifyError};

  use dcmfx_core::data_element_value::unique_identifier;

  fn deidentifier() -> Deidentifier {
    Deidentifier::with_uid_secret(b"secret".to_vec())
  }

  fn clean_structured_content() -> ProfileOptions {
    ProfileOptions {
      clean_structured_content: true,
      ..Default::default()
    }
  }

  fn full_dates() -> ProfileOptions {
    ProfileOptions {
      retain_longitudinal_temporal_information: Some(
        LongitudinalTemporalInformation::FullDates,
      ),
      ..Default::default()
    }
  }

  pub(crate) fn modified_dates(date_shift_days: Option<i32>) -> ProfileOptions {
    ProfileOptions {
      retain_longitudinal_temporal_information: Some(
        LongitudinalTemporalInformation::ModifiedDates { date_shift_days },
      ),
      ..Default::default()
    }
  }

  /// Creates a value for the given dictionary item directly from a string,
  /// for VRs that `DataSet::insert_string_value()` doesn't support.
  ///
  pub(crate) fn raw_value(
    item: &dictionary::Item,
    s: &str,
  ) -> DataElementValue {
    let mut bytes = s.as_bytes().to_vec();
    item.vrs[0].pad_bytes_to_even_length(&mut bytes);

    DataElementValue::new_binary(item.vrs[0], bytes.into()).unwrap()
  }

  /// Builds a content item with the given concept name and value type.
  ///
  pub(crate) fn content_item(
    coding_scheme_designator: &str,
    code_value: &str,
    value_type: &str,
  ) -> DataSet {
    let mut concept_name = DataSet::new();
    concept_name
      .insert_string_value(&dictionary::CODE_VALUE, &[code_value])
      .unwrap();
    concept_name
      .insert_string_value(
        &dictionary::CODING_SCHEME_DESIGNATOR,
        &[coding_scheme_designator],
      )
      .unwrap();
    concept_name
      .insert_string_value(&dictionary::CODE_MEANING, &["Meaning"])
      .unwrap();

    let mut item = DataSet::new();
    item
      .insert_string_value(&dictionary::RELATIONSHIP_TYPE, &["CONTAINS"])
      .unwrap();
    item
      .insert_string_value(&dictionary::VALUE_TYPE, &[value_type])
      .unwrap();
    item
      .insert_sequence_value(
        &dictionary::CONCEPT_NAME_CODE_SEQUENCE,
        vec![concept_name],
      )
      .unwrap();

    item
  }

  /// Builds a TEXT content item with the given concept name and text.
  ///
  fn text_item(
    coding_scheme_designator: &str,
    code_value: &str,
    text: &str,
  ) -> DataSet {
    let mut item = content_item(coding_scheme_designator, code_value, "TEXT");
    item.insert(
      dictionary::TEXT_VALUE.tag,
      raw_value(&dictionary::TEXT_VALUE, text),
    );
    item
  }

  /// Builds a code sequence item.
  ///
  fn code(value: &str, scheme: &str, meaning: &str) -> DataSet {
    Deidentifier::code_item(value, scheme, meaning)
  }

  fn p10_bytes(data_set: &DataSet) -> Vec<u8> {
    let mut bytes = vec![];
    dcmfx_p10::DataSetP10Extensions::to_p10_bytes(
      data_set,
      &mut |chunk| {
        bytes.extend_from_slice(&chunk);
        Ok(())
      },
      None,
    )
    .unwrap();

    bytes
  }

  /// Asserts two data sets are equal, reporting the first differing data
  /// element rather than printing both data sets in full.
  ///
  pub(crate) fn assert_data_sets_equal(actual: &DataSet, expected: &DataSet) {
    fn first_difference(
      actual: &DataSet,
      expected: &DataSet,
      path: &str,
    ) -> Option<String> {
      let mut tags: Vec<DataElementTag> =
        actual.tags().into_iter().chain(expected.tags()).collect();
      tags.sort();
      tags.dedup();

      for tag in tags {
        let location = format!("{path}/{tag}");

        match (actual.get_value(tag), expected.get_value(tag)) {
          (Ok(a), Ok(e)) => {
            if let (Ok(a_items), Ok(e_items)) =
              (a.sequence_items(), e.sequence_items())
            {
              if a_items.len() != e_items.len() {
                return Some(format!(
                  "{location}: {} items, expected {}",
                  a_items.len(),
                  e_items.len()
                ));
              }

              for (i, (a_item, e_item)) in
                a_items.iter().zip(e_items.iter()).enumerate()
              {
                if let Some(difference) =
                  first_difference(a_item, e_item, &format!("{location}[{i}]"))
                {
                  return Some(difference);
                }
              }
            } else if a != e {
              return Some(format!("{location}: {a:?}, expected {e:?}"));
            }
          }
          (Ok(a), Err(_)) => {
            return Some(format!("{location}: unexpected {a:?}"));
          }
          (Err(_), Ok(e)) => return Some(format!("{location}: missing {e:?}")),
          (Err(_), Err(_)) => (),
        }
      }

      None
    }

    if let Some(difference) = first_difference(actual, expected, "") {
      panic!("Data sets differ at {difference}");
    }
  }

  #[test]
  fn uids_are_replaced_consistently() {
    let deidentifier = deidentifier();

    // The replacement is a valid UID derived from a version 4 UUID, and is
    // the same for the same original and secret only
    let uid = deidentifier.map_uid("1.2.3.4");
    assert!(uid.starts_with("2.25."));
    assert!(unique_identifier::is_valid(&uid));
    let uuid = uid[5..].parse::<u128>().unwrap().to_be_bytes();
    assert_eq!(uuid[6] >> 4, 4);
    assert_eq!(uuid[8] >> 6, 0b10);
    assert_eq!(uid, deidentifier.map_uid("1.2.3.4"));
    assert_ne!(uid, deidentifier.map_uid("1.2.3.5"));
    assert_ne!(
      uid,
      Deidentifier::with_uid_secret(b"other".to_vec()).map_uid("1.2.3.4")
    );

    // UIDs are replaced with their padding ignored, empty UIDs stay empty,
    // standard UIDs are kept, a source UID maps to the same replacement as
    // a reference to it, and an HL7 identifier is replaced as a whole as its
    // backslash is text rather than a separator
    let ui = |s: &str| {
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::UniqueIdentifier,
        s.as_bytes().to_vec().into(),
      )
    };
    let mut data_set = DataSet::new();
    data_set.insert(dictionary::SOP_INSTANCE_UID.tag, ui(" 1.2.3.4 "));
    data_set
      .insert(dictionary::REFERENCED_SOP_INSTANCE_UID.tag, ui("1.2.3.4\0"));
    data_set.insert(dictionary::STUDY_INSTANCE_UID.tag, ui("\\1.2 "));
    data_set.insert(dictionary::SERIES_INSTANCE_UID.tag, ui(""));
    data_set.insert(
      dictionary::REFERENCED_COLOR_PALETTE_INSTANCE_UID.tag,
      ui("1.2.840.10008.9.1"),
    );
    data_set.insert(
      dictionary::MULTI_FRAME_SOURCE_SOP_INSTANCE_UID.tag,
      ui("1.2.3.4"),
    );
    data_set.insert(
      dictionary::HL7_INSTANCE_IDENTIFIER.tag,
      raw_value(&dictionary::HL7_INSTANCE_IDENTIFIER, "1.2.3.402^RPT\\7"),
    );
    data_set.deidentify(&deidentifier).unwrap();

    for item in [
      &dictionary::SOP_INSTANCE_UID,
      &dictionary::REFERENCED_SOP_INSTANCE_UID,
      &dictionary::MULTI_FRAME_SOURCE_SOP_INSTANCE_UID,
    ] {
      assert_eq!(
        data_set.get_string(item.tag),
        Ok(uid.as_str()),
        "{}",
        item.name
      );
    }
    assert_eq!(
      data_set
        .get_strings(dictionary::STUDY_INSTANCE_UID.tag)
        .unwrap(),
      vec!["", deidentifier.map_uid("1.2").as_str()]
    );
    assert_eq!(
      data_set.get_string(dictionary::SERIES_INSTANCE_UID.tag),
      Ok("")
    );
    assert_eq!(
      data_set
        .get_string(dictionary::REFERENCED_COLOR_PALETTE_INSTANCE_UID.tag),
      Ok("1.2.840.10008.9.1")
    );
    let hl7 = deidentifier.map_uid("1.2.3.402^RPT\\7");
    assert_eq!(
      data_set.get_string(dictionary::HL7_INSTANCE_IDENTIFIER.tag),
      Ok(hl7.as_str())
    );
    assert!(!hl7.contains('\\'));

    // The Retain UIDs Option keeps them
    let mut data_set = DataSet::new();
    data_set.insert(dictionary::SOP_INSTANCE_UID.tag, ui("1.2.3.4"));
    data_set
      .deidentify(&deidentifier.with_options(ProfileOptions {
        retain_uids: true,
        ..Default::default()
      }))
      .unwrap();
    assert_eq!(
      data_set.get_string(dictionary::SOP_INSTANCE_UID.tag),
      Ok("1.2.3.4")
    );
  }

  #[cfg(all(
    feature = "std",
    not(all(target_family = "wasm", target_os = "unknown"))
  ))]
  #[test]
  fn new_uses_random_secret() {
    let a = Deidentifier::new();
    let b = Deidentifier::new();

    assert_eq!(a.uid_secret().len(), 32);
    assert_ne!(a.uid_secret(), b.uid_secret());
    assert_ne!(a.map_uid("1.2.3"), b.map_uid("1.2.3"));
    assert_eq!(a.map_uid("1.2.3"), a.clone().map_uid("1.2.3"));
  }

  #[test]
  fn date_shift_days() {
    let deidentifier = deidentifier();
    assert_eq!(deidentifier.date_shift_days(), None);
    assert_eq!(
      deidentifier
        .clone()
        .with_options(full_dates())
        .date_shift_days(),
      None
    );
    assert_eq!(
      deidentifier
        .clone()
        .with_options(modified_dates(Some(42)))
        .date_shift_days(),
      Some(42)
    );

    // A derived shift is consistent for the same secret and in range
    let derived = deidentifier.clone().with_options(modified_dates(None));
    let days = derived.date_shift_days().unwrap();
    assert!((-3650..=-1).contains(&days));
    assert_eq!(
      deidentifier
        .clone()
        .with_options(modified_dates(None))
        .date_shift_days(),
      Some(days)
    );
    assert_ne!(
      Deidentifier::with_uid_secret(b"other".to_vec())
        .with_options(modified_dates(None))
        .date_shift_days(),
      Some(days)
    );

    // A zero shift is rejected
    let mut data_set = DataSet::new();
    data_set
      .insert_string_value(&dictionary::MODALITY, &["CT"])
      .unwrap();
    assert_eq!(
      data_set.deidentify(&deidentifier.with_options(modified_dates(Some(0)))),
      Err(DeidentifyError::ZeroDateShift)
    );
    assert_eq!(data_set.deidentify(&derived), Ok(()));
  }

  #[test]
  fn dummy_values_are_valid_and_non_empty() {
    let deidentifier = deidentifier();

    for vr in [
      ValueRepresentation::AgeString,
      ValueRepresentation::ApplicationEntity,
      ValueRepresentation::AttributeTag,
      ValueRepresentation::CodeString,
      ValueRepresentation::Date,
      ValueRepresentation::DateTime,
      ValueRepresentation::DecimalString,
      ValueRepresentation::FloatingPointDouble,
      ValueRepresentation::FloatingPointSingle,
      ValueRepresentation::IntegerString,
      ValueRepresentation::LongString,
      ValueRepresentation::LongText,
      ValueRepresentation::OtherByteString,
      ValueRepresentation::OtherDoubleString,
      ValueRepresentation::OtherFloatString,
      ValueRepresentation::OtherLongString,
      ValueRepresentation::OtherVeryLongString,
      ValueRepresentation::OtherWordString,
      ValueRepresentation::PersonName,
      ValueRepresentation::ShortString,
      ValueRepresentation::ShortText,
      ValueRepresentation::SignedLong,
      ValueRepresentation::SignedShort,
      ValueRepresentation::SignedVeryLong,
      ValueRepresentation::Time,
      ValueRepresentation::UniqueIdentifier,
      ValueRepresentation::UniversalResourceIdentifier,
      ValueRepresentation::Unknown,
      ValueRepresentation::UnlimitedCharacters,
      ValueRepresentation::UnlimitedText,
      ValueRepresentation::UnsignedLong,
      ValueRepresentation::UnsignedShort,
      ValueRepresentation::UnsignedVeryLong,
    ] {
      let value = deidentifier.dummy_value(vr);
      assert_eq!(value.value_representation(), vr);

      let bytes = value.bytes().unwrap();
      assert!(!bytes.is_empty(), "{vr}");
      assert!(bytes.len().is_multiple_of(2), "{vr}");
    }

    let sequence = deidentifier.dummy_value(ValueRepresentation::Sequence);
    assert_eq!(sequence.sequence_items().unwrap().len(), 0);

    assert_eq!(
      deidentifier
        .dummy_value(ValueRepresentation::Date)
        .get_string()
        .unwrap(),
      "19000101"
    );
    assert_eq!(
      deidentifier
        .dummy_value(ValueRepresentation::PersonName)
        .get_string()
        .unwrap(),
      DUMMY_TEXT
    );
  }

  #[test]
  fn deidentify_with_modified_dates() {
    let deidentifier = deidentifier().with_options(modified_dates(Some(-10)));

    let mut data_set = test_data_set();
    data_set.insert(
      dictionary::ACQUISITION_DATE_TIME.tag,
      raw_value(&dictionary::ACQUISITION_DATE_TIME, "20240102123000+1200"),
    );
    data_set.insert(
      dictionary::STUDY_TIME.tag,
      raw_value(&dictionary::STUDY_TIME, "123000"),
    );
    data_set.insert(
      dictionary::ACQUISITION_DATE.tag,
      raw_value(&dictionary::ACQUISITION_DATE, "invalid!"),
    );
    data_set.insert(
      dictionary::REFERENCED_DATE_TIME.tag,
      raw_value(&dictionary::REFERENCED_DATE_TIME, "bad\\worse"),
    );
    data_set.insert(
      dictionary::TIMEZONE_OFFSET_FROM_UTC.tag,
      raw_value(&dictionary::TIMEZONE_OFFSET_FROM_UTC, "+1200"),
    );

    data_set.deidentify(&deidentifier).unwrap();

    let string = |item: &dictionary::Item| {
      data_set.get_string(item.tag).unwrap().to_string()
    };

    assert_eq!(string(&dictionary::STUDY_DATE), "20231223");
    assert_eq!(string(&dictionary::SERIES_DATE), "20231223");
    assert_eq!(string(&dictionary::CONTENT_DATE), "20231223");
    assert_eq!(
      string(&dictionary::ACQUISITION_DATE_TIME),
      "20231223123000+1200"
    );
    assert_eq!(string(&dictionary::STUDY_TIME), "123000");
    assert_eq!(string(&dictionary::ACQUISITION_DATE), "19000101");
    assert_eq!(string(&dictionary::REFERENCED_DATE_TIME), "190001010000");
    assert_eq!(string(&dictionary::PATIENT_BIRTH_DATE), "");
    assert_eq!(string(&dictionary::TIMEZONE_OFFSET_FROM_UTC), "+1200");
    assert_eq!(
      string(&dictionary::LONGITUDINAL_TEMPORAL_INFORMATION_MODIFIED),
      "MODIFIED"
    );
    assert!(
      data_set
        .get_strings(dictionary::DEIDENTIFICATION_METHOD.tag)
        .unwrap()
        .contains(
          &"Retain Longitudinal Temporal Information Modified Dates Option"
        )
    );
  }

  #[test]
  fn frame_origin_timestamps_are_shifted_with_modified_dates() {
    let timestamp = |seconds: u64| {
      let mut bytes = seconds.to_be_bytes()[2..].to_vec();
      bytes.extend_from_slice(&500_000_000u32.to_be_bytes());
      DataElementValue::new_other_byte_string(bytes).unwrap()
    };

    let mut data_set = DataSet::new();
    data_set.insert(
      dictionary::FRAME_ORIGIN_TIMESTAMP.tag,
      timestamp(1_700_000_000),
    );

    // The PTP timestamp is shifted by the same number of days
    let mut result = data_set.clone();
    result
      .deidentify(&deidentifier().with_options(modified_dates(Some(-7))))
      .unwrap();
    assert_eq!(
      result.get_value(dictionary::FRAME_ORIGIN_TIMESTAMP.tag),
      Ok(&timestamp(1_700_000_000 - 7 * 86400))
    );

    // Without the option it becomes a dummy
    data_set.deidentify(&deidentifier()).unwrap();
    assert_eq!(
      data_set
        .get_value_bytes(dictionary::FRAME_ORIGIN_TIMESTAMP.tag)
        .unwrap()
        .as_ref(),
      &[0, 0]
    );
  }

  #[test]
  fn file_meta_information_is_reduced_to_structural_data_elements() {
    let mut data_set = test_data_set();
    data_set
      .insert_string_value(
        &dictionary::SOURCE_PRESENTATION_ADDRESS,
        &["dicom://pacs.hospital.example:104"],
      )
      .unwrap();
    data_set.deidentify(&deidentifier()).unwrap();

    assert_eq!(
      data_set.file_meta_information().tags(),
      vec![
        dictionary::MEDIA_STORAGE_SOP_CLASS_UID.tag,
        dictionary::MEDIA_STORAGE_SOP_INSTANCE_UID.tag,
      ]
    );
  }

  #[test]
  fn removals_outside_the_table() {
    let mut data_set = DataSet::new();

    // Group lengths, the retired Source Image IDs, overlays in full, and UN
    // values are removed; a data element that isn't in the data dictionary
    // is kept
    data_set.insert(
      DataElementTag::new(0x0008, 0x0000),
      DataElementValue::new_unsigned_long(&[1234]).unwrap(),
    );
    data_set.insert(
      DataElementTag::new(0x0020, 0x3110),
      DataElementValue::new_long_string(&["IMG-123"]).unwrap(),
    );
    data_set
      .insert_string_value(&dictionary::OVERLAY_TYPE, &["G"])
      .unwrap();
    data_set.insert(
      DataElementTag::new(0x6002, 0x1500),
      raw_value(&dictionary::OVERLAY_LABEL, "SMITH^JOHN"),
    );
    data_set.insert(
      dictionary::IMAGE_FILTER_DESCRIPTION.tag,
      DataElementValue::new_binary_unchecked(
        ValueRepresentation::Unknown,
        Bytes::from_static(b"Smoothing "),
      ),
    );
    data_set.insert(
      DataElementTag::new(0x0018, 0x7FFF),
      DataElementValue::new_long_string(&["Unknown"]).unwrap(),
    );
    data_set
      .insert_string_value(&dictionary::MODALITY, &["CT"])
      .unwrap();

    data_set.deidentify(&deidentifier()).unwrap();

    assert_eq!(
      data_set.tags(),
      vec![
        DataElementTag::new(0x0008, 0x0060),
        DataElementTag::new(0x0012, 0x0062),
        DataElementTag::new(0x0012, 0x0063),
        DataElementTag::new(0x0012, 0x0064),
        DataElementTag::new(0x0018, 0x7FFF),
        DataElementTag::new(0x0028, 0x0303),
      ]
    );
  }

  #[test]
  fn encapsulated_documents_are_removed() {
    let mut item = DataSet::new();
    item.insert(
      dictionary::ENCAPSULATED_DOCUMENT.tag,
      DataElementValue::new_other_byte_string(b"<xml/>".to_vec()).unwrap(),
    );
    item
      .insert_string_value(
        &dictionary::MIME_TYPE_OF_ENCAPSULATED_DOCUMENT,
        &["text/xml"],
      )
      .unwrap();

    let mut data_set = DataSet::new();
    data_set
      .insert_string_value(
        &dictionary::SOP_CLASS_UID,
        &["1.2.840.10008.5.1.4.1.1.104.1"],
      )
      .unwrap();
    data_set.insert(
      dictionary::ENCAPSULATED_DOCUMENT.tag,
      DataElementValue::new_other_byte_string(b"%PDF-1.7 MRN-54321".to_vec())
        .unwrap(),
    );
    data_set
      .insert_string_value(
        &dictionary::MIME_TYPE_OF_ENCAPSULATED_DOCUMENT,
        &["application/pdf"],
      )
      .unwrap();
    data_set.insert(
      dictionary::ENCAPSULATED_DOCUMENT_LENGTH.tag,
      DataElementValue::new_unsigned_long(&[18]).unwrap(),
    );
    data_set
      .insert_sequence_value(&dictionary::REFERENCED_IMAGE_SEQUENCE, vec![item])
      .unwrap();
    data_set.deidentify(&deidentifier()).unwrap();

    // The document and the data elements describing it are removed in the
    // root data set and in sequence items alike
    let item = &data_set
      .get_sequence_items(dictionary::REFERENCED_IMAGE_SEQUENCE.tag)
      .unwrap()[0];
    for data_set in [&data_set, item] {
      assert!(!data_set.has(dictionary::ENCAPSULATED_DOCUMENT.tag));
      assert!(
        !data_set.has(dictionary::MIME_TYPE_OF_ENCAPSULATED_DOCUMENT.tag)
      );
      assert!(!data_set.has(dictionary::ENCAPSULATED_DOCUMENT_LENGTH.tag));
    }

    let bytes = p10_bytes(&data_set);
    assert!(!bytes.windows(9).any(|window| window == b"MRN-54321"));
  }

  #[test]
  fn replacement_values_that_cannot_be_encoded_are_an_error() {
    // Two thousand short UIDs whose replacements are longer than a UI value
    // can hold
    let uids = (0..2000).map(|i| format!("1.2.{i}")).collect::<Vec<_>>();
    let mut data_set = DataSet::new();
    data_set.insert(
      dictionary::STUDY_INSTANCE_UID.tag,
      raw_value(&dictionary::STUDY_INSTANCE_UID, &uids.join("\\")),
    );

    assert_eq!(
      data_set.deidentify(&deidentifier()),
      Err(DeidentifyError::ValueTooLong {
        tag: dictionary::STUDY_INSTANCE_UID.tag
      })
    );
  }

  /// What a data element's value becomes.
  #[derive(Clone, Copy)]
  enum Expected {
    Dummy,
    Empty,
    Url,
    Str(&'static str),
    Kept,
    Uid,
  }

  /// Asserts that each of the given data elements, given with an input
  /// value, comes out of de-identification with the given options as
  /// expected, both in the root data set and inside a kept sequence item.
  ///
  fn assert_expected(
    options: ProfileOptions,
    cases: &[(&dictionary::Item, &str, Expected)],
  ) {
    let deidentifier = deidentifier().with_options(options);

    let mut data_set = DataSet::new();
    for (item, value, _) in cases {
      data_set.insert(item.tag, raw_value(item, value));
    }
    let item = data_set.clone();
    data_set
      .insert_sequence_value(&dictionary::REFERENCED_IMAGE_SEQUENCE, vec![item])
      .unwrap();
    data_set.deidentify(&deidentifier).unwrap();

    let item = &data_set
      .get_sequence_items(dictionary::REFERENCED_IMAGE_SEQUENCE.tag)
      .unwrap()[0];

    for output in [&data_set, item] {
      for (item, value, expected) in cases {
        let actual = output.get_string(item.tag).map(str::to_string);
        let expected = match expected {
          Expected::Dummy => Ok(DUMMY_TEXT.to_string()),
          Expected::Empty => Ok(String::new()),
          Expected::Url => Ok("https://de-identified.invalid".to_string()),
          Expected::Str(s) => Ok(s.to_string()),
          Expected::Kept => Ok(value.to_string()),
          Expected::Uid => Ok(deidentifier.map_uid(value)),
        };
        assert_eq!(actual, expected, "{}", item.name);
      }
    }
  }

  #[test]
  fn supplementary_table_entries_are_applied() {
    use Expected::*;

    // Required labels, names, and URLs are replaced, Type 2 ones are emptied,
    // retrieval locations are replaced like UIDs, and unlisted descriptions
    // are kept
    assert_expected(
      ProfileOptions::default(),
      &[
        (&dictionary::DOCUMENT_TITLE, "Report for Jane Smith", Dummy),
        (
          &dictionary::ACQUISITION_PROTOCOL_NAME,
          "Jane Smith head CT",
          Dummy,
        ),
        (
          &dictionary::WORKLIST_LABEL,
          "Dr Jane Smith's worklist",
          Dummy,
        ),
        (
          &dictionary::PROCEDURE_STEP_LABEL,
          "CT for Jane Smith",
          Dummy,
        ),
        (
          &dictionary::PATIENT_ADDITIONAL_POSITION,
          "Jane Smith",
          Dummy,
        ),
        (&dictionary::RT_IMAGE_LABEL, "MRN-54321", Dummy),
        (&dictionary::CONTENT_LABEL, "LABEL", Dummy),
        (&dictionary::GRAPHIC_GROUP_LABEL, "Jane Smith", Dummy),
        (&dictionary::CONTACT_DISPLAY_NAME, "Dr Jane Smith", Dummy),
        (&dictionary::DEVICE_ID, "DEV-7", Dummy),
        (
          &dictionary::SUBSTANCE_ADMINISTRATION_DEVICE_ID,
          "PUMP-1",
          Dummy,
        ),
        (&dictionary::TEXT_VALUE, "Doe at root", Dummy),
        (
          &dictionary::INVENTORY_PURPOSE,
          "Studies of Jane Smith",
          Empty,
        ),
        (&dictionary::STORAGE_MEDIA_FILE_SET_ID, "SMITH_JANE", Empty),
        (&dictionary::CONTENT_DESCRIPTION, "Jane Smith", Empty),
        (&dictionary::OVERRIDE_REASON, "Jane Smith", Empty),
        (
          &dictionary::TREATMENT_TERMINATION_DESCRIPTION,
          "Jane Smith",
          Empty,
        ),
        (&dictionary::RADIATION_MACHINE_NAME, "LINAC2", Empty),
        (
          &dictionary::RETRIEVE_URL,
          "https://pacs.example/studies/1.2.3",
          Url,
        ),
        (
          &dictionary::STORAGE_URL,
          "https://pacs.example/store/1.2.3",
          Url,
        ),
        (
          &dictionary::CONTACT_URI,
          "mailto:jane.smith@hospital.example",
          Url,
        ),
        (&dictionary::RETRIEVE_LOCATION_UID, "1.2.3.4.5.66", Uid),
        (&dictionary::REPOSITORY_UNIQUE_ID, "1.2.3.4.5.88", Uid),
        (&dictionary::HOME_COMMUNITY_ID, "1.2.3.4.5.99", Uid),
        (
          &dictionary::ITEM_INVENTORY_DATE_TIME,
          "20240102030405",
          Str("190001010000"),
        ),
        (&dictionary::STUDY_UPDATE_DATE_TIME, "20240102030405", Empty),
        (
          &dictionary::CONTEXT_GROUP_VERSION,
          "20250401",
          Str("19000101"),
        ),
        (
          &dictionary::INVENTORY_INSTANCE_DESCRIPTION,
          "MRN-123456",
          Kept,
        ),
        (&dictionary::RT_IMAGE_NAME, "Jane Smith", Kept),
        (
          &dictionary::GRAPHIC_LAYER_DESCRIPTION,
          "Marked by Dr Jones",
          Kept,
        ),
        (
          &dictionary::COMPLETION_FLAG_DESCRIPTION,
          "Signed by Dr Jones",
          Kept,
        ),
      ],
    );

    // The options retain the device identifiers and retrieval locations and
    // shift the dates
    assert_expected(
      ProfileOptions {
        retain_device_identity: true,
        retain_institution_identity: true,
        ..modified_dates(Some(-10))
      },
      &[
        (&dictionary::DEVICE_ID, "DEV-7", Kept),
        (&dictionary::RADIATION_MACHINE_NAME, "LINAC2", Kept),
        (&dictionary::RETRIEVE_LOCATION_UID, "1.2.3.4.5.66", Kept),
        (&dictionary::HOME_COMMUNITY_ID, "1.2.3.4.5.99", Kept),
        (
          &dictionary::ITEM_INVENTORY_DATE_TIME,
          "20240102030405",
          Str("20231223030405"),
        ),
        (
          &dictionary::STUDY_UPDATE_DATE_TIME,
          "20240102030405",
          Str("20231223030405"),
        ),
        (
          &dictionary::CONTEXT_GROUP_VERSION,
          "20250401",
          Str("20250322"),
        ),
      ],
    );
  }

  #[test]
  fn required_data_elements_are_replaced_rather_than_removed() {
    use Expected::*;

    assert_expected(
      ProfileOptions::default(),
      &[
        (
          &dictionary::PRESENTATION_CREATION_DATE,
          "20240102",
          Str("19000101"),
        ),
        (
          &dictionary::PRESENTATION_CREATION_TIME,
          "030405",
          Str("0000"),
        ),
        (&dictionary::REQUESTED_PROCEDURE_ID, "RP-123", Empty),
        (&dictionary::SOURCE_MANUFACTURER, "Jane Smith", Empty),
        (&dictionary::TIMEZONE_OFFSET_FROM_UTC, "+1200", Str("+0000")),
        (&dictionary::RETRIEVE_AE_TITLE, "HOSPITALPACS", Dummy),
      ],
    );

    // The options still retain and shift them
    assert_expected(
      ProfileOptions {
        retain_device_identity: true,
        ..modified_dates(Some(-1))
      },
      &[
        (
          &dictionary::PRESENTATION_CREATION_DATE,
          "20240102",
          Str("20240101"),
        ),
        (&dictionary::PRESENTATION_CREATION_TIME, "030405", Kept),
        (&dictionary::SOURCE_MANUFACTURER, "Jane Smith", Kept),
        (&dictionary::TIMEZONE_OFFSET_FROM_UTC, "+1200", Kept),
      ],
    );
  }

  #[test]
  fn animal_patient_characteristics_are_handled_explicitly() {
    let labrador = code("L-80700", "SRT", "Labrador retriever");

    let mut data_set = DataSet::new();
    data_set
      .insert_string_value(
        &dictionary::ANATOMICAL_ORIENTATION_TYPE,
        &["QUADRUPED"],
      )
      .unwrap();
    data_set
      .insert_string_value(&dictionary::RESPONSIBLE_PERSON_ROLE, &["OWNER"])
      .unwrap();
    data_set
      .insert_string_value(
        &dictionary::PATIENT_SPECIES_DESCRIPTION,
        &["Canis lupus familiaris"],
      )
      .unwrap();
    data_set
      .insert_sequence_value(
        &dictionary::PATIENT_BREED_CODE_SEQUENCE,
        vec![labrador.clone()],
      )
      .unwrap();
    data_set.insert(
      dictionary::RESPONSIBLE_PERSON.tag,
      raw_value(&dictionary::RESPONSIBLE_PERSON, "Smith^Jane"),
    );
    data_set
      .insert_string_value(&dictionary::STRAIN_DESCRIPTION, &["C57BL/6"])
      .unwrap();

    // The structural attributes are kept, the required species is dummied,
    // the breed is emptied, the required responsible person is emptied, and
    // the strain goes with the rest of the patient group
    let mut result = data_set.clone();
    result.deidentify(&deidentifier()).unwrap();
    assert_eq!(
      result.get_string(dictionary::ANATOMICAL_ORIENTATION_TYPE.tag),
      Ok("QUADRUPED")
    );
    assert_eq!(
      result.get_string(dictionary::RESPONSIBLE_PERSON_ROLE.tag),
      Ok("OWNER")
    );
    assert_eq!(
      result.get_string(dictionary::PATIENT_SPECIES_DESCRIPTION.tag),
      Ok(DUMMY_TEXT)
    );
    assert_eq!(
      result
        .get_sequence_items(dictionary::PATIENT_BREED_CODE_SEQUENCE.tag)
        .unwrap(),
      &[]
    );
    assert_eq!(
      result.get_string(dictionary::RESPONSIBLE_PERSON.tag),
      Ok("")
    );
    assert!(!result.has(dictionary::STRAIN_DESCRIPTION.tag));

    // The characteristics are retained by the option, the identifiers aren't
    data_set
      .deidentify(&deidentifier().with_options(ProfileOptions {
        retain_patient_characteristics: true,
        ..Default::default()
      }))
      .unwrap();
    assert_eq!(
      data_set.get_string(dictionary::PATIENT_SPECIES_DESCRIPTION.tag),
      Ok("Canis lupus familiaris")
    );
    assert_eq!(
      data_set
        .get_sequence_items(dictionary::PATIENT_BREED_CODE_SEQUENCE.tag)
        .unwrap(),
      &[labrador]
    );
    assert_eq!(
      data_set.get_string(dictionary::RESPONSIBLE_PERSON.tag),
      Ok("")
    );
    assert!(!data_set.has(dictionary::STRAIN_DESCRIPTION.tag));
  }

  #[test]
  fn content_items_are_cleaned_by_concept() {
    let deidentifier = deidentifier().with_options(clean_structured_content());

    // A listed item is removed, a listed PNAME's value is replaced, and an
    // unlisted item is kept, in a cleaned sequence and in a nested one
    let mut observer_name = content_item("DCM", "121008", "PNAME");
    observer_name.insert(
      dictionary::PERSON_NAME.tag,
      raw_value(&dictionary::PERSON_NAME, "Smith^John"),
    );
    let finding = text_item("99TEST", "1234", "Stained");

    let mut step = DataSet::new();
    step
      .insert_sequence_value(
        &dictionary::SPECIMEN_PREPARATION_STEP_CONTENT_ITEM_SEQUENCE,
        vec![
          text_item("DCM", "121022", "ACC123"),
          observer_name,
          finding.clone(),
        ],
      )
      .unwrap();

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(
        &dictionary::SPECIMEN_PREPARATION_SEQUENCE,
        vec![step],
      )
      .unwrap();
    data_set.deidentify(&deidentifier).unwrap();

    let mut expected_observer_name = content_item("DCM", "121008", "PNAME");
    expected_observer_name.insert(
      dictionary::PERSON_NAME.tag,
      deidentifier.dummy_value(ValueRepresentation::PersonName),
    );
    assert_eq!(
      data_set
        .get_sequence_items(dictionary::SPECIMEN_PREPARATION_SEQUENCE.tag)
        .unwrap()[0]
        .get_sequence_items(
          dictionary::SPECIMEN_PREPARATION_STEP_CONTENT_ITEM_SEQUENCE.tag
        )
        .unwrap(),
      &[expected_observer_name, finding]
    );

    // A kept item's modifiers are cleaned too
    let mut item = text_item("99TEST", "1234", "Supine");
    item
      .insert_sequence_value(
        &dictionary::CONTENT_ITEM_MODIFIER_SEQUENCE,
        vec![text_item("DCM", "121030", "PATIENT123")],
      )
      .unwrap();
    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(
        &dictionary::ACQUISITION_CONTEXT_SEQUENCE,
        vec![item],
      )
      .unwrap();
    data_set.deidentify(&deidentifier).unwrap();

    let item = &data_set
      .get_sequence_items(dictionary::ACQUISITION_CONTEXT_SEQUENCE.tag)
      .unwrap()[0];
    assert_eq!(item.get_string(dictionary::TEXT_VALUE.tag), Ok("Supine"));
    assert_eq!(
      item
        .get_sequence_items(dictionary::CONTENT_ITEM_MODIFIER_SEQUENCE.tag)
        .unwrap()[0]
        .get_string(dictionary::TEXT_VALUE.tag),
      Ok(DUMMY_TEXT)
    );
  }

  #[test]
  fn content_item_values_are_replaced() {
    let deidentifier = deidentifier().with_options(clean_structured_content());

    // Every numeric representation of a Subject ID is replaced, directly in
    // an Acquisition Context NUMERIC and in the measured values of an SR NUM,
    // and an empty Measured Value Sequence stays empty
    let mut numeric = content_item("DCM", "121030", "NUMERIC");
    numeric.insert(
      dictionary::NUMERIC_VALUE.tag,
      raw_value(&dictionary::NUMERIC_VALUE, "12345678"),
    );
    numeric.insert(
      dictionary::FLOATING_POINT_VALUE.tag,
      DataElementValue::new_floating_point_double(&[12345678.0]).unwrap(),
    );
    numeric.insert(
      dictionary::RATIONAL_NUMERATOR_VALUE.tag,
      DataElementValue::new_signed_long(&[12345678]).unwrap(),
    );
    numeric.insert(
      dictionary::RATIONAL_DENOMINATOR_VALUE.tag,
      DataElementValue::new_unsigned_long(&[7]).unwrap(),
    );

    let mut measured_value = DataSet::new();
    measured_value.insert(
      dictionary::NUMERIC_VALUE.tag,
      raw_value(&dictionary::NUMERIC_VALUE, "12345678"),
    );
    let mut num = content_item("DCM", "121030", "NUM");
    num
      .insert_sequence_value(
        &dictionary::MEASURED_VALUE_SEQUENCE,
        vec![measured_value],
      )
      .unwrap();
    let mut failed_measurement = content_item("DCM", "121030", "NUM");
    failed_measurement
      .insert_sequence_value(&dictionary::MEASURED_VALUE_SEQUENCE, vec![])
      .unwrap();

    // The code of a CODE content item is replaced with the dummy code
    let mut coded = content_item("DCM", "121030", "CODE");
    coded
      .insert_sequence_value(
        &dictionary::CONCEPT_CODE_SEQUENCE,
        vec![code("MRN-12345678", "99HOSP", "MRN-12345678")],
      )
      .unwrap();

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(
        &dictionary::ACQUISITION_CONTEXT_SEQUENCE,
        vec![numeric],
      )
      .unwrap();
    data_set
      .insert_sequence_value(
        &dictionary::CONTENT_SEQUENCE,
        vec![num, failed_measurement, coded],
      )
      .unwrap();
    data_set.deidentify(&deidentifier).unwrap();

    let bytes = p10_bytes(&data_set);
    assert!(!bytes.windows(8).any(|window| window == b"12345678"));

    let numeric = &data_set
      .get_sequence_items(dictionary::ACQUISITION_CONTEXT_SEQUENCE.tag)
      .unwrap()[0];
    assert_eq!(
      numeric.get_value(dictionary::NUMERIC_VALUE.tag).unwrap(),
      &raw_value(&dictionary::NUMERIC_VALUE, "0")
    );
    assert_eq!(
      numeric
        .get_floats(dictionary::FLOATING_POINT_VALUE.tag)
        .unwrap(),
      vec![0.0]
    );
    assert_eq!(
      numeric
        .get_ints::<i32>(dictionary::RATIONAL_NUMERATOR_VALUE.tag)
        .unwrap(),
      vec![0]
    );
    assert_eq!(
      numeric
        .get_ints::<u32>(dictionary::RATIONAL_DENOMINATOR_VALUE.tag)
        .unwrap(),
      vec![1]
    );

    let items = data_set
      .get_sequence_items(dictionary::CONTENT_SEQUENCE.tag)
      .unwrap();
    assert_eq!(
      items[0]
        .get_sequence_items(dictionary::MEASURED_VALUE_SEQUENCE.tag)
        .unwrap()[0]
        .get_value(dictionary::NUMERIC_VALUE.tag)
        .unwrap(),
      &raw_value(&dictionary::NUMERIC_VALUE, "0")
    );
    assert_eq!(
      items[1]
        .get_sequence_items(dictionary::MEASURED_VALUE_SEQUENCE.tag)
        .unwrap(),
      &[]
    );
    assert_eq!(
      items[2]
        .get_sequence_items(dictionary::CONCEPT_CODE_SEQUENCE.tag)
        .unwrap(),
      &[Deidentifier::dummy_code_item()]
    );
  }

  #[test]
  fn retained_content_items_keep_their_values() {
    let deidentifier = deidentifier().with_options(ProfileOptions {
      retain_uids: true,
      retain_patient_characteristics: true,
      ..clean_structured_content()
    });

    let mut referenced_sop = DataSet::new();
    referenced_sop
      .insert_string_value(
        &dictionary::REFERENCED_SOP_CLASS_UID,
        &["1.2.840.10008.5.1.4.1.1.2"],
      )
      .unwrap();
    referenced_sop
      .insert_string_value(&dictionary::REFERENCED_SOP_INSTANCE_UID, &["1.2.9"])
      .unwrap();

    let mut study_uid = content_item("DCM", "110180", "UIDREF");
    study_uid
      .insert_string_value(&dictionary::UID, &["1.2.3"])
      .unwrap();

    let mut image = content_item("DCM", "121080", "IMAGE");
    image
      .insert_sequence_value(
        &dictionary::REFERENCED_SOP_SEQUENCE,
        vec![referenced_sop],
      )
      .unwrap();

    // Not retained by an option, so its name is still replaced
    let mut observer_name = content_item("DCM", "121008", "PNAME");
    observer_name.insert(
      dictionary::PERSON_NAME.tag,
      raw_value(&dictionary::PERSON_NAME, "Smith^John"),
    );

    // Retained by an option, but its value sequence contains a private data
    // element and a patient name that must still be removed
    let mut measured_value = DataSet::new();
    measured_value.insert(
      dictionary::NUMERIC_VALUE.tag,
      raw_value(&dictionary::NUMERIC_VALUE, "44"),
    );
    measured_value.insert(
      DataElementTag::new(0x0009, 0x1001),
      DataElementValue::new_long_string(&["PATIENT123"]).unwrap(),
    );
    measured_value.insert(
      dictionary::PATIENT_NAME.tag,
      raw_value(&dictionary::PATIENT_NAME, "Doe^Jane"),
    );
    let mut subject_age = content_item("DCM", "121033", "NUM");
    subject_age
      .insert_sequence_value(
        &dictionary::MEASURED_VALUE_SEQUENCE,
        vec![measured_value],
      )
      .unwrap();

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(
        &dictionary::CONTENT_SEQUENCE,
        vec![
          study_uid.clone(),
          image.clone(),
          observer_name,
          subject_age.clone(),
        ],
      )
      .unwrap();
    data_set.deidentify(&deidentifier).unwrap();

    let items = data_set
      .get_sequence_items(dictionary::CONTENT_SEQUENCE.tag)
      .unwrap();
    assert_eq!(items[0], study_uid);
    assert_eq!(items[1], image);
    assert_eq!(
      items[2].get_string(dictionary::PERSON_NAME.tag),
      Ok(DUMMY_TEXT)
    );

    let measured_value = &items[3]
      .get_sequence_items(dictionary::MEASURED_VALUE_SEQUENCE.tag)
      .unwrap()[0];
    assert_eq!(
      measured_value.get_value(dictionary::NUMERIC_VALUE.tag),
      Ok(&raw_value(&dictionary::NUMERIC_VALUE, "44"))
    );
    assert!(!measured_value.has(DataElementTag::new(0x0009, 0x1001)));
    assert_eq!(
      measured_value.get_string(dictionary::PATIENT_NAME.tag),
      Ok("")
    );
  }

  #[test]
  fn text_value_is_replaced_except_in_kept_text_content_items() {
    // Text Value in the root data set, in an item of a sequence that doesn't
    // hold content items, in content items of an unlisted sequence, and in a
    // content item of another value type or no value type isn't a kept
    // content item's text and is replaced
    let mut lut = DataSet::new();
    lut.insert(
      dictionary::TEXT_VALUE.tag,
      raw_value(&dictionary::TEXT_VALUE, "Doe in LUT"),
    );

    let mut num = content_item("99TEST", "1", "NUM");
    num.insert(
      dictionary::TEXT_VALUE.tag,
      raw_value(&dictionary::TEXT_VALUE, "Doe in NUM"),
    );
    let mut untyped = text_item("99TEST", "2", "Doe untyped");
    untyped.delete(dictionary::VALUE_TYPE.tag);
    let text = text_item("99TEST", "3", "Stained");

    let mut original = DataSet::new();
    original.insert(
      dictionary::TEXT_VALUE.tag,
      raw_value(&dictionary::TEXT_VALUE, "Doe at root"),
    );
    original
      .insert_sequence_value(&dictionary::VOILUT_SEQUENCE, vec![lut])
      .unwrap();
    original
      .insert_sequence_value(
        &dictionary::SPECIMEN_LOCALIZATION_CONTENT_ITEM_SEQUENCE,
        vec![text.clone()],
      )
      .unwrap();
    original
      .insert_sequence_value(
        &dictionary::CONTENT_SEQUENCE,
        vec![num, untyped, text],
      )
      .unwrap();

    // The text of a TEXT item in the Content Sequence is kept under the
    // option and the sequence is emptied without it
    for options in [ProfileOptions::default(), clean_structured_content()] {
      let cleaned = options.clean_structured_content;
      let mut data_set = original.clone();
      data_set
        .deidentify(&deidentifier().with_options(options))
        .unwrap();

      let bytes = p10_bytes(&data_set);
      assert!(!bytes.windows(3).any(|window| window == b"Doe"));

      for text_value in [
        data_set.get_string(dictionary::TEXT_VALUE.tag),
        data_set
          .get_sequence_items(dictionary::VOILUT_SEQUENCE.tag)
          .unwrap()[0]
          .get_string(dictionary::TEXT_VALUE.tag),
        data_set
          .get_sequence_items(
            dictionary::SPECIMEN_LOCALIZATION_CONTENT_ITEM_SEQUENCE.tag,
          )
          .unwrap()[0]
          .get_string(dictionary::TEXT_VALUE.tag),
      ] {
        assert_eq!(text_value, Ok(DUMMY_TEXT));
      }

      let items = data_set
        .get_sequence_items(dictionary::CONTENT_SEQUENCE.tag)
        .unwrap();
      if cleaned {
        assert_eq!(items.len(), 3);
        assert_eq!(
          items[0].get_string(dictionary::TEXT_VALUE.tag),
          Ok(DUMMY_TEXT)
        );
        assert_eq!(
          items[1].get_string(dictionary::TEXT_VALUE.tag),
          Ok(DUMMY_TEXT)
        );
        assert_eq!(
          items[2].get_string(dictionary::TEXT_VALUE.tag),
          Ok("Stained")
        );
      } else {
        assert!(items.is_empty());
      }
    }
  }

  /// Builds a data set with File Meta Information and a few data elements
  /// of each action.
  ///
  pub(crate) fn test_data_set() -> DataSet {
    let mut data_set = DataSet::new();

    // File Meta Information
    data_set
      .insert_string_value(
        &dictionary::MEDIA_STORAGE_SOP_CLASS_UID,
        &["1.2.840.10008.5.1.4.1.1.2"],
      )
      .unwrap();
    data_set
      .insert_string_value(
        &dictionary::MEDIA_STORAGE_SOP_INSTANCE_UID,
        &["1.2.3.4.5"],
      )
      .unwrap();
    data_set
      .insert_string_value(
        &dictionary::SOURCE_APPLICATION_ENTITY_TITLE,
        &["SENDER"],
      )
      .unwrap();
    data_set
      .insert_string_value(
        &dictionary::PRIVATE_INFORMATION_CREATOR_UID,
        &["1.2.3"],
      )
      .unwrap();

    // Root data set
    data_set
      .insert_string_value(&dictionary::SPECIFIC_CHARACTER_SET, &["ISO_IR 192"])
      .unwrap();
    data_set
      .insert_string_value(
        &dictionary::SOP_CLASS_UID,
        &["1.2.840.10008.5.1.4.1.1.2"],
      )
      .unwrap();
    data_set
      .insert_string_value(&dictionary::SOP_INSTANCE_UID, &["1.2.3.4.5"])
      .unwrap();
    for item in [
      &dictionary::STUDY_DATE,
      &dictionary::SERIES_DATE,
      &dictionary::CONTENT_DATE,
    ] {
      data_set.insert(item.tag, raw_value(item, "20240102"));
    }
    data_set
      .insert_string_value(&dictionary::ACCESSION_NUMBER, &["ACC123"])
      .unwrap();
    data_set
      .insert_string_value(&dictionary::MODALITY, &["CT"])
      .unwrap();
    data_set.insert(
      dictionary::PATIENT_NAME.tag,
      raw_value(&dictionary::PATIENT_NAME, "Doe^Jane"),
    );
    data_set
      .insert_string_value(&dictionary::PATIENT_ID, &["12345"])
      .unwrap();
    data_set.insert(
      dictionary::PATIENT_BIRTH_DATE.tag,
      raw_value(&dictionary::PATIENT_BIRTH_DATE, "19800101"),
    );
    data_set
      .insert_string_value(
        &dictionary::DEIDENTIFICATION_METHOD,
        &["Previous method"],
      )
      .unwrap();
    data_set
      .insert_string_value(&dictionary::STUDY_INSTANCE_UID, &["1.2.3"])
      .unwrap();
    data_set
      .insert_string_value(&dictionary::SERIES_INSTANCE_UID, &["1.2.3.4"])
      .unwrap();

    data_set
  }
}
