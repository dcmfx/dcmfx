//! Lookup of the action the Basic Application Level Confidentiality Profile
//! and its options take for a data element or content item.

mod content_item_table;
mod table;

use dcmfx_core::{DataElementTag, ValueRepresentation, dictionary};

use crate::{
  Action, LongitudinalTemporalInformation, ProfileOptions, utils::trim_padding,
};

/// An entry in Table E.1-1 of DICOM PS3.15.
///
pub(crate) struct Entry {
  pub tag: DataElementTag,
  pub basic_profile: Action,
  pub options: u8,
}

/// An entry in Table E.3.4-1 of DICOM PS3.15.
///
pub(crate) struct ContentItemEntry {
  pub coding_scheme_designator: &'static str,
  pub code_value: &'static str,
  pub value_type: &'static str,
  pub basic_profile: Action,
  pub options: u8,
}

// Short names for the actions that mirror the action codes in the standard,
// used by the generated tables
pub(crate) const X: Action = Action::Remove;
pub(crate) const Z: Action = Action::ReplaceWithZeroLength;
pub(crate) const D: Action = Action::ReplaceWithDummy;
pub(crate) const U: Action = Action::ReplaceUid;
pub(crate) const K: Action = Action::Keep;

// Flags for the option columns in the tables. The `_CLEANED` flags mark data
// elements that an option specifies are to be cleaned rather than kept, which
// isn't supported, so they are not retained under that option.
pub(crate) const RETAIN_UIDS: u8 = 1 << 0;
pub(crate) const RETAIN_DEVICE_IDENTITY: u8 = 1 << 1;
pub(crate) const RETAIN_DEVICE_IDENTITY_CLEANED: u8 = 1 << 2;
pub(crate) const RETAIN_INSTITUTION_IDENTITY: u8 = 1 << 3;
pub(crate) const RETAIN_PATIENT_CHARACTERISTICS: u8 = 1 << 4;
pub(crate) const RETAIN_PATIENT_CHARACTERISTICS_CLEANED: u8 = 1 << 5;
pub(crate) const RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION: u8 = 1 << 6;
pub(crate) const CLEAN_STRUCTURED_CONTENT: u8 = 1 << 7;

/// Returns the action that the Basic Application Level Confidentiality Profile
/// and the given options take for the given data element. The lookup is the
/// same regardless of whether the data element is in the root data set or
/// inside a sequence item.
///
/// The action is determined as follows:
///
/// 1. Private data elements and group length data elements are removed. The
///    latter would be invalidated by the changes to the lengths of other
///    values. The retired Source Image IDs (0020,31xx), which identify
///    images, are removed as well. Data elements that aren't in the data
///    dictionary are kept, like other unlisted data elements, so the
///    dictionary should be kept current with the standard.
/// 2. The File Meta Information is reduced to its structural data elements:
///    File Meta Information Version, Media Storage SOP Class UID, Transfer
///    Syntax UID, Implementation Class UID, and Implementation Version Name
///    are kept, Media Storage SOP Instance UID is handled by Table E.1-1, and
///    everything else, including the Application Entity Titles, Presentation
///    Addresses, and private information, is removed.
/// 3. Data elements in group 0004 are removed. This removes the directory
///    records of a DICOMDIR, which should instead be regenerated from the
///    de-identified files it references.
/// 4. Curve data in groups 5000-50FF and overlays in groups 6000-60FF are
///    removed in their entirety. Table E.1-1 lists only Overlay Data
///    (60xx,3000) and Overlay Comments (60xx,4000), but removing them leaves
///    an overlay plane without its required data, and its description,
///    subtype, and label are free text that often describes what it
///    annotates, so the whole optional Overlay Plane Module is removed.
/// 5. Encapsulated Document (0042,0011) is removed rather than replaced with
///    a dummy value, and MIME Type of Encapsulated Document (0042,0012) and
///    Encapsulated Document Length (0042,0015), which describe it, are
///    removed with it.
/// 6. A few data elements that aren't listed in Table E.1-1 are handled like
///    the listed data elements they resemble: the UIDs of source instances
///    and other references are replaced like the listed UIDs, and the
///    unlisted labels, names, and identifiers that modules require are
///    replaced like the listed ones.
/// 7. Data elements listed in Table E.1-1 of DICOM PS3.15 take the action
///    specified there for the Basic Profile, unless an active option lists the
///    data element as retained, in which case it is kept, or as cleaned, in
///    which case it is cleaned. Where the Basic Profile's action is
///    conditional on the type of the data element in the IOD, the action that
///    maintains IOD conformance for every type is used: Z/D becomes D, X/Z
///    becomes Z, X/D and X/Z/D become D, and X/Z/U* retains the sequence and
///    replaces the UIDs it contains.
/// 8. Any remaining data elements in the patient group 0010, or with a PN or
///    AE VR, are removed.
/// 9. Any remaining data elements with a DA, DT, or TM VR are handled in the
///    same way as the dates and times listed in Table E.1-1: they are removed,
///    or kept or cleaned under a Retain Longitudinal Temporal Information
///    Option.
/// 10. Everything else is kept.
///
/// The returned action is also adjusted to suit the VR: replacing a UID with a
/// dummy value is reported as [`Action::ReplaceUid`] so that the dummy UID is
/// consistent across instances, and replacing the UIDs in a sequence is
/// reported as [`Action::Keep`] so that the sequence's items are recursed into.
/// A UID replacement on a binary VR is reported as [`Action::Remove`].
///
/// A data element whose value has the UN VR is removed regardless of the
/// above, as its value can't be inspected: it may be a sequence that holds the
/// data elements the profile removes, as a defined-length sequence that isn't
/// in the data dictionary is read as such a value from an implicit VR data
/// set.
///
pub fn action_for_data_element(
  tag: DataElementTag,
  vr: ValueRepresentation,
  options: &ProfileOptions,
) -> Action {
  if vr == ValueRepresentation::Unknown {
    return Action::Remove;
  }

  action_for_vr(lookup_action(tag, vr, options), vr)
}

/// Adjusts an action to suit the VR of the data element it applies to. See
/// [`action_for_data_element()`].
///
fn action_for_vr(action: Action, vr: ValueRepresentation) -> Action {
  match (action, vr) {
    (Action::ReplaceWithDummy, ValueRepresentation::UniqueIdentifier) => {
      Action::ReplaceUid
    }

    (Action::ReplaceUid, ValueRepresentation::Sequence) => Action::Keep,

    (Action::ReplaceUid, vr) if vr.is_binary() => Action::Remove,

    _ => action,
  }
}

fn lookup_action(
  tag: DataElementTag,
  vr: ValueRepresentation,
  options: &ProfileOptions,
) -> Action {
  if is_always_removed(tag) {
    return Action::Remove;
  }

  // The File Meta Information is entirely replaced when a file is written, so
  // only its structural data elements are kept
  if tag.group == 0x0002 {
    return match tag.element {
      0x0000 | 0x0001 | 0x0002 | 0x0010 | 0x0012 | 0x0013 => Action::Keep,
      0x0003 => table_action(tag, vr, options).unwrap_or(Action::ReplaceUid),
      _ => Action::Remove,
    };
  }

  // Encapsulated Document (0042,0011) is removed rather than replaced with a
  // dummy value, which the table specifies, as no dummy document carries
  // anything of use, and the data elements that describe it go with it
  if tag == dictionary::ENCAPSULATED_DOCUMENT.tag
    || tag == dictionary::MIME_TYPE_OF_ENCAPSULATED_DOCUMENT.tag
    || tag == dictionary::ENCAPSULATED_DOCUMENT_LENGTH.tag
  {
    return Action::Remove;
  }

  if options.clean_structured_content && vr == ValueRepresentation::Sequence {
    // The content items of Specimen Preparation Sequence (0040,0610) aren't
    // its items but are in the Specimen Preparation Step Content Item Sequence
    // (0040,0612) of each of them, so the former is recursed into and the
    // latter has its content items cleaned
    if tag == dictionary::SPECIMEN_PREPARATION_SEQUENCE.tag {
      return Action::Keep;
    }

    // The items of Content Item Modifier Sequence (0040,0441) are content
    // items that modify the content item containing them
    if tag == dictionary::SPECIMEN_PREPARATION_STEP_CONTENT_ITEM_SEQUENCE.tag
      || tag == dictionary::CONTENT_ITEM_MODIFIER_SEQUENCE.tag
    {
      return Action::Clean;
    }
  }

  if let Some(action) = table_action(tag, vr, options) {
    return action;
  }

  if tag.group == 0x0010 {
    return Action::Remove;
  }

  match vr {
    ValueRepresentation::PersonName
    | ValueRepresentation::ApplicationEntity => Action::Remove,

    ValueRepresentation::Date
    | ValueRepresentation::DateTime
    | ValueRepresentation::Time => {
      option_action(RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION, Some(vr), options)
        .unwrap_or(Action::Remove)
    }

    _ => Action::Keep,
  }
}

/// Returns whether a data element is removed regardless of the options and
/// of where it appears: private data elements, group lengths, group 0004,
/// the retired Source Image IDs (0020,31xx), curve data, and overlays.
///
fn is_always_removed(tag: DataElementTag) -> bool {
  tag.is_private()
    || (tag.element == 0x0000 && tag.group != 0x0002)
    || tag.group == 0x0004
    || (tag.group == 0x0020 && (0x3100..=0x31FF).contains(&tag.element))
    || (0x5000..=0x50FF).contains(&tag.group)
    || (0x6000..=0x60FF).contains(&tag.group)
}

/// Data elements that aren't listed in Table E.1-1 but are handled in the
/// same way as the listed data elements they resemble, either because they
/// are UIDs that reference other objects, or because they are required by
/// their module and would otherwise be removed by the rules for their group
/// or VR, or because they are the text labels and names that modules require
/// and that the listed labels and names sit alongside:
///
/// - UIDs that reference source instances, frames of reference, color
///   palettes, findings, assertions, models, fiducials, and other objects,
///   which are replaced like the listed UIDs so that they neither link back
///   to the original instances nor dangle, and are retained under the Retain
///   UIDs Option. HL7 Instance Identifier (0040,E001) is replaced like a UID
///   so that the replacement is a valid identifier.
/// - The locations that references to other instances are retrieved from,
///   which identify the storing institution: Retrieve Location UID
///   (0040,E011), Repository Unique ID (0040,E030), and Home Community ID
///   (0040,E031) are replaced like the listed UIDs, and are also retained
///   under the Retain Institution Identity Option.
/// - Retrieve URL (0008,1190), Retrieve URI (0040,E010), Storage URL
///   (0040,4073), and Contact URI (0074,100A), which embed the UIDs of the
///   instances they locate or identify a person, and are required where they
///   appear, so are replaced with a dummy URL. Contact Display Name
///   (0074,100C), which names the person, is replaced with a dummy value.
/// - The required labels and names that can name the patient or the people
///   involved like the listed labels and descriptions they sit alongside:
///   RT Image Label (3002,0002), Beam Name (300A,00C2), Content Label
///   (0070,0080), Graphic Group Label (0070,0207), Worklist Label
///   (0074,1202), Procedure Step Label (0074,1204), Document Title
///   (0042,0010), Acquisition Protocol Name (0018,9423), Patient Additional
///   Position (300A,0184), Tracking ID (0062,0020), Selector UC Value
///   (0072,006F), and the name, description, and creator of a hanging
///   protocol are replaced with dummy values, and Text Value (0040,A160)
///   outside a content item, where it isn't subject to the handling of
///   content items, is replaced with a dummy value, with the text of a kept
///   TEXT content item exempt. Content Description (0070,0081), Hanging
///   Protocol User Identification Code Sequence (0072,000E), Fixation Device
///   Label (300A,0196), Shielding Device Label (300A,01A4), Setup Device
///   Label (300A,01B2), Override Reason (3008,0066), Treatment Termination
///   Description (300A,0730), Inventory Purpose (0008,0401), and Storage
///   Media File-set ID (0088,0130) are Type 2, so are replaced with
///   zero-length values.
/// - Device ID (0018,1003) and Substance Administration Device ID
///   (0044,0012), which are required and identify the device like the listed
///   Device Serial Number (0018,1000), are replaced with dummy values, and
///   Radiation Machine Name (3002,0020) with a zero-length value, all
///   retained under the Retain Device Identity Option.
/// - The dates of an inventory, which describe the studies it lists: Item
///   Inventory DateTime (0008,0404) is replaced with a dummy value and Study
///   Update DateTime (0008,041F) with a zero-length value, with the dates
///   retained or modified under the Retain Longitudinal Temporal Information
///   Options.
/// - Optional sequences whose items would be left without a required data
///   element by the table's actions: the RT Physician Intent Input Instance
///   (3010,005F) and Planning Input Information (3010,0076) Sequences, which
///   require Referenced Study Sequence (0008,1110) to have an item, are
///   replaced with zero-length values.
/// - The characteristics of an animal patient, which the removal of the
///   patient group would otherwise take with it: Anatomical Orientation Type
///   (0010,2210) and Responsible Person Role (0010,2298) are kept, like
///   Quality Control Subject (0010,0200) and its type code sequence; Patient
///   Species Description (0010,2201) and Patient Species Code Sequence
///   (0010,2202), one of which is required, are replaced with dummy values,
///   and Patient Breed Description (0010,2292) and Patient Breed Code
///   Sequence (0010,2293) with zero-length values, all retained under the
///   Retain Patient Characteristics Option like Patient's Sex Neutered
///   (0010,2203); and Breed Registration Sequence (0010,2294) is replaced
///   with a zero-length value, like the listed Responsible Person
///   (0010,2297) and Responsible Organization (0010,2299).
/// - Type of Patient ID (0010,0022), which only says what kind of identifier
///   the replaced Patient ID (0010,0020) held, and Subject Relative Position
///   in Image (0010,0028), which positions a subject in a multi-subject image,
///   which the removal of the patient group would otherwise take with it.
///
const SUPPLEMENTARY_TABLE: &[Entry] = &[
  Entry {
    tag: dictionary::RT_IMAGE_LABEL.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::MULTI_FRAME_SOURCE_SOP_INSTANCE_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::RETRIEVE_URL.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::RADIOPHARMACEUTICAL_ADMINISTRATION_EVENT_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::TARGET_FRAME_OF_REFERENCE_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::SOP_INSTANCE_UID_OF_CONCATENATION_SOURCE.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::VOLUME_FRAME_OF_REFERENCE_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::TABLE_FRAME_OF_REFERENCE_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::FINDINGS_GROUP_UID_TRIAL.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::REFERENCED_FINDINGS_GROUP_UID_TRIAL.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::ASSERTION_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::REFERENCED_ASSERTION_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::MODEL_GROUP_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::CONTENT_DESCRIPTION.tag,
    basic_profile: Z,
    options: 0,
  },
  Entry {
    tag: dictionary::GRAPHIC_GROUP_LABEL.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::REFERENCED_FIDUCIAL_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::RETRIEVE_URI.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::VOLUMETRIC_PRESENTATION_INPUT_SET_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::REFERENCED_CONTENT_ITEM.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::HANGING_PROTOCOL_NAME.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::HANGING_PROTOCOL_DESCRIPTION.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::HANGING_PROTOCOL_CREATOR.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::HANGING_PROTOCOL_USER_IDENTIFICATION_CODE_SEQUENCE.tag,
    basic_profile: Z,
    options: 0,
  },
  Entry {
    tag: dictionary::WORKLIST_LABEL.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::PROCEDURE_STEP_LABEL.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::RETRIEVE_LOCATION_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS | RETAIN_INSTITUTION_IDENTITY,
  },
  Entry {
    tag: dictionary::REPOSITORY_UNIQUE_ID.tag,
    basic_profile: U,
    options: RETAIN_UIDS | RETAIN_INSTITUTION_IDENTITY,
  },
  Entry {
    tag: dictionary::HOME_COMMUNITY_ID.tag,
    basic_profile: U,
    options: RETAIN_UIDS | RETAIN_INSTITUTION_IDENTITY,
  },
  Entry {
    tag: dictionary::INVENTORY_PURPOSE.tag,
    basic_profile: Z,
    options: 0,
  },
  Entry {
    tag: dictionary::ITEM_INVENTORY_DATE_TIME.tag,
    basic_profile: D,
    options: RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  },
  Entry {
    tag: dictionary::STUDY_UPDATE_DATE_TIME.tag,
    basic_profile: Z,
    options: RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  },
  Entry {
    tag: dictionary::OVERRIDE_REASON.tag,
    basic_profile: Z,
    options: 0,
  },
  Entry {
    tag: dictionary::PATIENT_ADDITIONAL_POSITION.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::TREATMENT_TERMINATION_DESCRIPTION.tag,
    basic_profile: Z,
    options: 0,
  },
  Entry {
    tag: dictionary::ANATOMICAL_ORIENTATION_TYPE.tag,
    basic_profile: K,
    options: 0,
  },
  Entry {
    tag: dictionary::QUALITY_CONTROL_SUBJECT.tag,
    basic_profile: K,
    options: 0,
  },
  Entry {
    tag: dictionary::QUALITY_CONTROL_SUBJECT_TYPE_CODE_SEQUENCE.tag,
    basic_profile: K,
    options: 0,
  },
  Entry {
    tag: dictionary::RESPONSIBLE_PERSON_ROLE.tag,
    basic_profile: K,
    options: 0,
  },
  Entry {
    tag: dictionary::TYPE_OF_PATIENT_ID.tag,
    basic_profile: K,
    options: 0,
  },
  Entry {
    tag: dictionary::SUBJECT_RELATIVE_POSITION_IN_IMAGE.tag,
    basic_profile: K,
    options: 0,
  },
  Entry {
    tag: dictionary::TEXT_VALUE.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::CONTENT_LABEL.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::BEAM_NAME.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::FIXATION_DEVICE_LABEL.tag,
    basic_profile: Z,
    options: 0,
  },
  Entry {
    tag: dictionary::SHIELDING_DEVICE_LABEL.tag,
    basic_profile: Z,
    options: 0,
  },
  Entry {
    tag: dictionary::SETUP_DEVICE_LABEL.tag,
    basic_profile: Z,
    options: 0,
  },
  Entry {
    tag: dictionary::PATIENT_SPECIES_DESCRIPTION.tag,
    basic_profile: D,
    options: RETAIN_PATIENT_CHARACTERISTICS,
  },
  Entry {
    tag: dictionary::PATIENT_SPECIES_CODE_SEQUENCE.tag,
    basic_profile: D,
    options: RETAIN_PATIENT_CHARACTERISTICS,
  },
  Entry {
    tag: dictionary::PATIENT_BREED_DESCRIPTION.tag,
    basic_profile: Z,
    options: RETAIN_PATIENT_CHARACTERISTICS,
  },
  Entry {
    tag: dictionary::PATIENT_BREED_CODE_SEQUENCE.tag,
    basic_profile: Z,
    options: RETAIN_PATIENT_CHARACTERISTICS,
  },
  Entry {
    tag: dictionary::BREED_REGISTRATION_SEQUENCE.tag,
    basic_profile: Z,
    options: 0,
  },
  Entry {
    tag: dictionary::TRACKING_ID.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::RT_PHYSICIAN_INTENT_INPUT_INSTANCE_SEQUENCE.tag,
    basic_profile: Z,
    options: 0,
  },
  Entry {
    tag: dictionary::PLANNING_INPUT_INFORMATION_SEQUENCE.tag,
    basic_profile: Z,
    options: 0,
  },
  Entry {
    tag: dictionary::SELECTOR_UC_VALUE.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::REFERENCED_COLOR_PALETTE_INSTANCE_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::DOCUMENT_TITLE.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::HL7_INSTANCE_IDENTIFIER.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::ACQUISITION_PROTOCOL_NAME.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::STORAGE_MEDIA_FILE_SET_ID.tag,
    basic_profile: Z,
    options: 0,
  },
  Entry {
    tag: dictionary::CONTACT_URI.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::CONTACT_DISPLAY_NAME.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::EQUIPMENT_FRAME_OF_REFERENCE_UID.tag,
    basic_profile: U,
    options: RETAIN_UIDS,
  },
  Entry {
    tag: dictionary::STORAGE_URL.tag,
    basic_profile: D,
    options: 0,
  },
  Entry {
    tag: dictionary::DEVICE_ID.tag,
    basic_profile: D,
    options: RETAIN_DEVICE_IDENTITY,
  },
  Entry {
    tag: dictionary::SUBSTANCE_ADMINISTRATION_DEVICE_ID.tag,
    basic_profile: D,
    options: RETAIN_DEVICE_IDENTITY,
  },
  Entry {
    tag: dictionary::RADIATION_MACHINE_NAME.tag,
    basic_profile: Z,
    options: RETAIN_DEVICE_IDENTITY,
  },
];

/// Data elements that Table E.1-1 removes but that are Type 1 or Type 2 in a
/// module or macro of PS3.3, so that removing them leaves an invalid
/// instance. They are instead replaced with a dummy value (Type 1) or a
/// zero-length value (Type 2), which removes the same information:
///
/// - Retrieve AE Title (0008,0054), Retrieve URL (0008,1190), and Retrieve
///   URI (0040,E010) are Type 1 in the retrieval sequences of the Referenced
///   Instances and Access Macro, at least one of which is required, and
///   Storage URL (0040,4073) is Type 1 in STOW-RS Storage Sequence
///   (0040,4072). Storage Media File-set ID (0088,0130) is Type 2 in DICOM
///   Media Retrieval Sequence (0040,E022).
/// - Presentation Creation Date (0070,0082) and Time (0070,0083) are Type 1
///   in presentation states and structured displays.
/// - Radiopharmaceutical Start DateTime (0018,1078) is Type 1 in the
///   Enhanced PET Isotope Module.
/// - Requested Procedure ID (0040,1001) and Reason for Requested Procedure
///   Code Sequence (0040,100A) are Type 2 in requests and hanging protocol
///   definitions.
/// - Position Acquisition Template Name (3002,0121) and Description
///   (3002,0123), Source Manufacturer (300A,0216), Patient Treatment
///   Preparation Procedure Parameter Description (300A,078E), Patient Setup
///   Photo Description (300A,0794), and Prior Treatment Dose Description
///   (3010,0061) are Type 1 or 2 in radiotherapy objects, and RT Accessory
///   Holder Slot ID (300A,0611) and RT Accessory Device Slot ID (300A,0615)
///   are Type 1 where the table gives them a zero-length value.
/// - Responsible Person (0010,2297) and Responsible Organization (0010,2299)
///   are Type 2 for an animal patient.
/// - Study Description (0008,1030) is Type 2 in an inventory's studies.
/// - Timezone Offset From UTC (0008,0201) is Type 1 in the Timezone Module,
///   and its dummy value is the offset `+0000`.
/// - Sex Parameters for Clinical Use Category Comment (0010,0042) is Type 2
///   when the category is retained under the Retain Patient Characteristics
///   Option, which cleans rather than retains the comment, so the comment is
///   emptied.
///
/// The override only applies when the table's action is removal or a
/// zero-length value, so an option that retains or cleans the data element
/// still does so.
///
const REQUIRED_DATA_ELEMENTS: &[(DataElementTag, Action)] = &[
  (dictionary::RETRIEVE_AE_TITLE.tag, D),
  (dictionary::RETRIEVE_URL.tag, D),
  (dictionary::RETRIEVE_URI.tag, D),
  (dictionary::STORAGE_URL.tag, D),
  (dictionary::STORAGE_MEDIA_FILE_SET_ID.tag, Z),
  (dictionary::PRESENTATION_CREATION_DATE.tag, D),
  (dictionary::PRESENTATION_CREATION_TIME.tag, D),
  (dictionary::RADIOPHARMACEUTICAL_START_DATE_TIME.tag, D),
  (dictionary::REQUESTED_PROCEDURE_ID.tag, Z),
  (
    dictionary::REASON_FOR_REQUESTED_PROCEDURE_CODE_SEQUENCE.tag,
    Z,
  ),
  (dictionary::POSITION_ACQUISITION_TEMPLATE_NAME.tag, D),
  (dictionary::POSITION_ACQUISITION_TEMPLATE_DESCRIPTION.tag, Z),
  (dictionary::SOURCE_MANUFACTURER.tag, Z),
  (
    dictionary::PATIENT_TREATMENT_PREPARATION_PROCEDURE_PARAMETER_DESCRIPTION
      .tag,
    Z,
  ),
  (dictionary::PATIENT_SETUP_PHOTO_DESCRIPTION.tag, Z),
  (dictionary::PRIOR_TREATMENT_DOSE_DESCRIPTION.tag, Z),
  (dictionary::RESPONSIBLE_PERSON.tag, Z),
  (dictionary::RESPONSIBLE_ORGANIZATION.tag, Z),
  (dictionary::STUDY_DESCRIPTION.tag, Z),
  (dictionary::TIMEZONE_OFFSET_FROM_UTC.tag, D),
  (dictionary::RT_ACCESSORY_HOLDER_SLOT_ID.tag, D),
  (dictionary::RT_ACCESSORY_DEVICE_SLOT_ID.tag, D),
  (
    dictionary::SEX_PARAMETERS_FOR_CLINICAL_USE_CATEGORY_COMMENT.tag,
    Z,
  ),
];

/// Returns the action for a data element listed in Table E.1-1 or
/// [`SUPPLEMENTARY_TABLE`], taking the active options into account, or `None`
/// if the data element isn't listed. Removal of a data element in
/// [`REQUIRED_DATA_ELEMENTS`] becomes the replacement given there.
///
fn table_action(
  tag: DataElementTag,
  vr: ValueRepresentation,
  options: &ProfileOptions,
) -> Option<Action> {
  let entry = table::TABLE
    .binary_search_by_key(&tag, |entry| entry.tag)
    .ok()
    .map(|index| &table::TABLE[index])
    .or_else(|| SUPPLEMENTARY_TABLE.iter().find(|entry| entry.tag == tag))?;

  // The Retain Longitudinal Temporal Information With Modified Dates Option
  // specifies that Timezone Offset From UTC (0008,0201) is cleaned. Dates are
  // shifted by whole days and times aren't modified, so the offset remains
  // correct and is kept.
  if tag == dictionary::TIMEZONE_OFFSET_FROM_UTC.tag
    && matches!(
      options.retain_longitudinal_temporal_information,
      Some(LongitudinalTemporalInformation::ModifiedDates { .. })
    )
  {
    return Some(Action::Keep);
  }

  // The option also specifies that Frame Origin Timestamp (0034,0007) is
  // cleaned. It is a binary PTP timestamp rather than a date, and is shifted
  // like one.
  if tag == dictionary::FRAME_ORIGIN_TIMESTAMP.tag
    && matches!(
      options.retain_longitudinal_temporal_information,
      Some(LongitudinalTemporalInformation::ModifiedDates { .. })
    )
  {
    return Some(Action::Clean);
  }

  let action = option_action(entry.options, Some(vr), options)
    .unwrap_or(entry.basic_profile);

  if matches!(action, Action::Remove | Action::ReplaceWithZeroLength)
    && let Some((_, replacement)) =
      REQUIRED_DATA_ELEMENTS.iter().find(|(t, _)| *t == tag)
  {
    return Some(*replacement);
  }

  Some(action)
}

/// Returns the action that the active options specify for a table entry with
/// the given option flags, or `None` if no active option applies to it and
/// the Basic Profile's action should be used.
///
/// If any applicable option cleans the value then the result is
/// [`Action::Clean`], otherwise if any applicable option retains the value the
/// result is [`Action::Keep`].
///
/// `vr` is `None` for content items, where the Retain Longitudinal Temporal
/// Information With Modified Dates Option results in [`Action::Clean`] so that
/// the content item is kept and the date in its value is cleaned by the rules
/// for the value's data element.
///
fn option_action(
  flags: u8,
  vr: Option<ValueRepresentation>,
  options: &ProfileOptions,
) -> Option<Action> {
  let mut retain = false;
  let mut clean = false;

  if options.retain_uids && flags & RETAIN_UIDS != 0 {
    retain = true;
  }

  if options.retain_device_identity && flags & RETAIN_DEVICE_IDENTITY != 0 {
    retain = true;
  }

  if options.retain_institution_identity
    && flags & RETAIN_INSTITUTION_IDENTITY != 0
  {
    retain = true;
  }

  if options.retain_patient_characteristics
    && flags & RETAIN_PATIENT_CHARACTERISTICS != 0
  {
    retain = true;
  }

  if flags & RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION != 0 {
    match (options.retain_longitudinal_temporal_information, vr) {
      (Some(LongitudinalTemporalInformation::FullDates), _) => retain = true,

      // Content items holding dates are kept so that the date in their value
      // is shifted by the rules for the value's data element
      (Some(LongitudinalTemporalInformation::ModifiedDates { .. }), None) => {
        return Some(Action::Clean);
      }

      (
        Some(LongitudinalTemporalInformation::ModifiedDates { .. }),
        Some(vr),
      ) => {
        // Only dates and times can be shifted here. The other data elements
        // the option lists, Timezone Offset From UTC (0008,0201) and Frame
        // Origin Timestamp (0034,0007), are handled by [`table_action()`].
        if is_date_or_time_vr(vr) {
          clean = true;
        }
      }

      (None, _) => (),
    }
  }

  if options.clean_structured_content
    && flags & CLEAN_STRUCTURED_CONTENT != 0
    && vr == Some(ValueRepresentation::Sequence)
  {
    clean = true;
  }

  if clean {
    Some(Action::Clean)
  } else if retain {
    Some(Action::Keep)
  } else {
    None
  }
}

fn is_date_or_time_vr(vr: ValueRepresentation) -> bool {
  matches!(
    vr,
    ValueRepresentation::Date
      | ValueRepresentation::DateTime
      | ValueRepresentation::Time
  )
}

/// The action to take on a content item under the Clean Structured Content
/// Option.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContentItemAction {
  /// The content item is kept and its data elements, including those holding
  /// its value, are subject to the profile in the usual way. This is the
  /// action for content items whose concept name isn't listed in Table
  /// E.3.4-1.
  ///
  Keep,

  /// The content item is retained by an active option. Its value is kept
  /// unchanged, and its other data elements are subject to the profile in the
  /// usual way.
  ///
  Retain,

  /// The content item's value is replaced with a dummy value, and its other
  /// data elements are subject to the profile in the usual way. This
  /// corresponds to the `D` action code.
  ///
  ReplaceValue,

  /// The content item is removed, along with any content items nested inside
  /// it. This corresponds to the `X` action code. Content items that follow
  /// it in the same sequence move up, so a by-reference relationship that
  /// identifies one of them by its position no longer resolves.
  ///
  Remove,
}

impl ContentItemAction {
  /// Returns how strict this action is, where a stricter action leaves less
  /// of the original content item in place. Keeping a content item that isn't
  /// listed is the least strict, as it is the absence of a listed action, so
  /// an explicit retention outranks it.
  ///
  pub(crate) fn strictness(self) -> u8 {
    match self {
      Self::Remove => 3,
      Self::ReplaceValue => 2,
      Self::Retain => 1,
      Self::Keep => 0,
    }
  }
}

/// Returns the action that the Clean Structured Content Option takes for a
/// content item with the given concept name and value type, taking into
/// account the other active options. The concept name is given as the Coding
/// Scheme Designator (0008,0102) and Code Value (0008,0100) of the first item
/// of the content item's Concept Name Code Sequence (0040,A043), and the value
/// type is the content item's Value Type (0040,A040).
///
/// Content items whose concept name is listed in Table E.3.4-1 of DICOM PS3.15
/// are removed or have their value replaced with a dummy value, unless an
/// active option retains them. All other content items are kept.
///
/// If the table lists the concept name but not with the content item's value
/// type, the most restrictive action across the table's entries for the
/// concept name is used.
///
pub fn action_for_content_item(
  coding_scheme_designator: Option<&str>,
  code_value: Option<&str>,
  value_type: Option<&str>,
  options: &ProfileOptions,
) -> ContentItemAction {
  let (Some(coding_scheme_designator), Some(code_value)) =
    (coding_scheme_designator, code_value)
  else {
    return ContentItemAction::Keep;
  };

  let key = (
    trim_padding(coding_scheme_designator),
    trim_padding(code_value),
  );
  let entry_key = |entry: &ContentItemEntry| {
    (entry.coding_scheme_designator, entry.code_value)
  };

  let start =
    content_item_table::TABLE.partition_point(|entry| entry_key(entry) < key);
  let end =
    content_item_table::TABLE.partition_point(|entry| entry_key(entry) <= key);
  let entries = &content_item_table::TABLE[start..end];

  let resolve = |entry: &ContentItemEntry| match option_action(
    entry.options,
    None,
    options,
  ) {
    Some(Action::Keep) => ContentItemAction::Retain,
    Some(_) => ContentItemAction::Keep,
    None => match entry.basic_profile {
      Action::Remove => ContentItemAction::Remove,
      _ => ContentItemAction::ReplaceValue,
    },
  };

  let value_type = value_type.map(trim_padding);
  if let Some(entry) = entries
    .iter()
    .find(|entry| Some(entry.value_type) == value_type)
  {
    return resolve(entry);
  }

  entries
    .iter()
    .map(resolve)
    .max_by_key(|action| action.strictness())
    .unwrap_or(ContentItemAction::Keep)
}

#[cfg(test)]
mod tests {
  use super::*;

  use dcmfx_core::dictionary;

  fn full_dates() -> ProfileOptions {
    ProfileOptions {
      retain_longitudinal_temporal_information: Some(
        LongitudinalTemporalInformation::FullDates,
      ),
      ..Default::default()
    }
  }

  fn modified_dates() -> ProfileOptions {
    ProfileOptions {
      retain_longitudinal_temporal_information: Some(
        LongitudinalTemporalInformation::ModifiedDates {
          date_shift_days: Some(-10),
        },
      ),
      ..Default::default()
    }
  }

  fn action(item: &dictionary::Item, options: &ProfileOptions) -> Action {
    action_for_data_element(item.tag, item.vrs[0], options)
  }

  fn action_with_vr(
    tag: DataElementTag,
    vr: ValueRepresentation,
    options: &ProfileOptions,
  ) -> Action {
    action_for_data_element(tag, vr, options)
  }

  #[test]
  fn tables_are_sorted_and_unique() {
    assert!(
      table::TABLE
        .windows(2)
        .all(|pair| pair[0].tag < pair[1].tag)
    );

    assert!(content_item_table::TABLE.windows(2).all(|pair| {
      let key = |entry: &ContentItemEntry| {
        (
          entry.coding_scheme_designator,
          entry.code_value,
          entry.value_type,
        )
      };
      key(&pair[0]) < key(&pair[1])
    }));
  }

  #[test]
  fn table_entries() {
    let options = ProfileOptions::default();

    for (item, expected) in [
      (&dictionary::ACCESSION_NUMBER, Action::ReplaceWithZeroLength),
      (&dictionary::PATIENT_NAME, Action::ReplaceWithZeroLength),
      (&dictionary::PATIENT_ID, Action::ReplaceWithDummy),
      (
        &dictionary::PATIENT_BIRTH_DATE,
        Action::ReplaceWithZeroLength,
      ),
      (&dictionary::SOP_INSTANCE_UID, Action::ReplaceUid),
      (
        &dictionary::MEDIA_STORAGE_SOP_INSTANCE_UID,
        Action::ReplaceUid,
      ),
      (&dictionary::STUDY_INSTANCE_UID, Action::ReplaceUid),
      (&dictionary::REFERENCED_SOP_INSTANCE_UID, Action::ReplaceUid),
      (&dictionary::STUDY_DATE, Action::ReplaceWithZeroLength),
      (&dictionary::CONTENT_DATE, Action::ReplaceWithDummy),
      (&dictionary::ACQUISITION_DATE, Action::ReplaceWithZeroLength),
      (&dictionary::SERIES_DATE, Action::ReplaceWithDummy),
      (&dictionary::OPERATORS_NAME, Action::ReplaceWithDummy),
      (&dictionary::INSTITUTION_NAME, Action::ReplaceWithDummy),
      (&dictionary::REFERENCED_IMAGE_SEQUENCE, Action::Keep),
      (&dictionary::SOURCE_IMAGE_SEQUENCE, Action::Keep),
      (&dictionary::PATIENT_COMMENTS, Action::Remove),
      (&dictionary::IMAGE_COMMENTS, Action::Remove),
      (&dictionary::ICON_IMAGE_SEQUENCE, Action::Remove),
      (&dictionary::CONTENT_SEQUENCE, Action::ReplaceWithDummy),
      (&dictionary::ANNOTATION_GROUP_UID, Action::ReplaceUid),
    ] {
      assert_eq!(action(item, &options), expected, "{}", item.name);
    }
  }

  #[test]
  fn rules_outside_table() {
    let options = ProfileOptions::default();

    // Private data elements, private File Meta Information, group 0004,
    // curve data, overlays, unlisted patient group data elements, and the
    // retired Source Image IDs are removed
    for (tag, vr) in [
      (
        DataElementTag::new(0x0009, 0x0010),
        ValueRepresentation::LongString,
      ),
      (
        DataElementTag::new(0x0002, 0x0102),
        ValueRepresentation::OtherByteString,
      ),
      (
        dictionary::DIRECTORY_RECORD_SEQUENCE.tag,
        ValueRepresentation::Sequence,
      ),
      (
        DataElementTag::new(0x5002, 0x0010),
        ValueRepresentation::UnsignedShort,
      ),
      (
        DataElementTag::new(0x6002, 0x0010),
        ValueRepresentation::UnsignedShort,
      ),
      (
        DataElementTag::new(0x6002, 0x3000),
        ValueRepresentation::OtherWordString,
      ),
      (
        DataElementTag::new(0x0010, 0xABCD),
        ValueRepresentation::LongString,
      ),
      (
        DataElementTag::new(0x0020, 0x3110),
        ValueRepresentation::LongString,
      ),
      (
        DataElementTag::new(0x0008, 0x0000),
        ValueRepresentation::UnsignedLong,
      ),
    ] {
      assert_eq!(action_with_vr(tag, vr, &options), Action::Remove, "{tag}");
    }

    // The File Meta Information keeps only its structural data elements
    for item in [
      &dictionary::FILE_META_INFORMATION_VERSION,
      &dictionary::MEDIA_STORAGE_SOP_CLASS_UID,
      &dictionary::TRANSFER_SYNTAX_UID,
      &dictionary::IMPLEMENTATION_CLASS_UID,
      &dictionary::IMPLEMENTATION_VERSION_NAME,
    ] {
      assert_eq!(action(item, &options), Action::Keep, "{}", item.name);
    }
    assert_eq!(
      action(&dictionary::MEDIA_STORAGE_SOP_INSTANCE_UID, &options),
      Action::ReplaceUid
    );
    for item in [
      &dictionary::SOURCE_APPLICATION_ENTITY_TITLE,
      &dictionary::SENDING_PRESENTATION_ADDRESS,
      &dictionary::PRIVATE_INFORMATION_CREATOR_UID,
      &dictionary::PRIVATE_INFORMATION,
    ] {
      assert_eq!(action(item, &options), Action::Remove, "{}", item.name);
    }

    // An encapsulated document and the data elements describing it are
    // removed
    for item in [
      &dictionary::ENCAPSULATED_DOCUMENT,
      &dictionary::MIME_TYPE_OF_ENCAPSULATED_DOCUMENT,
      &dictionary::ENCAPSULATED_DOCUMENT_LENGTH,
    ] {
      assert_eq!(action(item, &options), Action::Remove, "{}", item.name);
    }

    // Unlisted names, AE titles, dates and times are removed
    for vr in [
      ValueRepresentation::PersonName,
      ValueRepresentation::ApplicationEntity,
      ValueRepresentation::Date,
      ValueRepresentation::DateTime,
      ValueRepresentation::Time,
    ] {
      assert_eq!(
        action_with_vr(DataElementTag::new(0x0008, 0xABCD), vr, &options),
        Action::Remove
      );
    }

    // Everything else is kept, including unlisted free text
    for (tag, vr) in [
      (dictionary::ROWS.tag, ValueRepresentation::UnsignedShort),
      (
        dictionary::PIXEL_DATA.tag,
        ValueRepresentation::OtherWordString,
      ),
      (
        dictionary::SOP_CLASS_UID.tag,
        ValueRepresentation::UniqueIdentifier,
      ),
      (
        dictionary::COMPLETION_FLAG_DESCRIPTION.tag,
        ValueRepresentation::LongString,
      ),
    ] {
      assert_eq!(action_with_vr(tag, vr, &options), Action::Keep, "{tag}");
    }

    // A UN value is always removed, and a UID replacement on a binary VR is
    // a removal
    assert_eq!(
      action_with_vr(
        dictionary::SOP_INSTANCE_UID.tag,
        ValueRepresentation::Unknown,
        &options,
      ),
      Action::Remove
    );
    assert_eq!(
      action_with_vr(
        dictionary::SOP_INSTANCE_UID.tag,
        ValueRepresentation::OtherByteString,
        &options,
      ),
      Action::Remove
    );
  }

  #[test]
  fn retain_options() {
    // For each option: the data elements it retains, and ones it doesn't
    // because the table lists them as cleaned rather than kept, or doesn't
    // list them under the option at all
    let cases: [(ProfileOptions, &[&dictionary::Item], &[&dictionary::Item]);
      4] = [
      (
        ProfileOptions {
          retain_uids: true,
          ..Default::default()
        },
        &[
          &dictionary::SOP_INSTANCE_UID,
          &dictionary::MEDIA_STORAGE_SOP_INSTANCE_UID,
          &dictionary::REFERENCED_SOP_INSTANCE_UID,
          &dictionary::ANNOTATION_GROUP_UID,
          &dictionary::REFERENCED_PATIENT_SEQUENCE,
          &dictionary::MULTI_FRAME_SOURCE_SOP_INSTANCE_UID,
        ],
        &[&dictionary::UID, &dictionary::PATIENT_NAME],
      ),
      (
        ProfileOptions {
          retain_device_identity: true,
          ..Default::default()
        },
        &[
          &dictionary::DEVICE_SERIAL_NUMBER,
          &dictionary::STATION_NAME,
          &dictionary::DEVICE_UID,
          &dictionary::DATE_OF_LAST_CALIBRATION,
          &dictionary::DEVICE_ID,
        ],
        &[
          &dictionary::STATION_AE_TITLE,
          &dictionary::DESTINATION_AE,
          &dictionary::INSTITUTION_NAME,
        ],
      ),
      (
        ProfileOptions {
          retain_institution_identity: true,
          ..Default::default()
        },
        &[
          &dictionary::INSTITUTION_NAME,
          &dictionary::INSTITUTION_ADDRESS,
          &dictionary::INSTITUTIONAL_DEPARTMENT_NAME,
          &dictionary::CLINICAL_TRIAL_SITE_NAME,
          &dictionary::RETRIEVE_LOCATION_UID,
        ],
        &[&dictionary::STATION_NAME],
      ),
      (
        ProfileOptions {
          retain_patient_characteristics: true,
          ..Default::default()
        },
        &[
          &dictionary::PATIENT_AGE,
          &dictionary::PATIENT_SEX,
          &dictionary::PATIENT_SIZE,
          &dictionary::PATIENT_WEIGHT,
          &dictionary::ETHNIC_GROUP,
          &dictionary::SMOKING_STATUS,
          &dictionary::PREGNANCY_STATUS,
          &dictionary::PATIENT_SPECIES_DESCRIPTION,
        ],
        &[
          &dictionary::ALLERGIES,
          &dictionary::PATIENT_STATE,
          &dictionary::PATIENT_BIRTH_DATE,
          &dictionary::PATIENT_NAME,
        ],
      ),
    ];

    for (options, retained, not_retained) in cases {
      for item in retained {
        assert_eq!(action(item, &options), Action::Keep, "{}", item.name);
        assert_ne!(
          action(item, &ProfileOptions::default()),
          Action::Keep,
          "{}",
          item.name
        );
      }
      for item in not_retained {
        assert_ne!(action(item, &options), Action::Keep, "{}", item.name);
      }
    }
  }

  #[test]
  fn retain_longitudinal_temporal_information_options() {
    let full_dates = full_dates();
    let modified_dates = modified_dates();

    for item in [
      &dictionary::STUDY_DATE,
      &dictionary::STUDY_TIME,
      &dictionary::ACQUISITION_DATE_TIME,
      &dictionary::CONTENT_DATE,
      &dictionary::DATE_OF_LAST_CALIBRATION,
      &dictionary::DATE,
      &dictionary::TIME,
      &dictionary::DATE_TIME,
      &dictionary::ITEM_INVENTORY_DATE_TIME,
    ] {
      assert_eq!(action(item, &full_dates), Action::Keep, "{}", item.name);
      assert_eq!(
        action(item, &modified_dates),
        Action::Clean,
        "{}",
        item.name
      );
    }

    // Unlisted dates and times are handled the same way
    for vr in [
      ValueRepresentation::Date,
      ValueRepresentation::DateTime,
      ValueRepresentation::Time,
    ] {
      let tag = dictionary::EXPIRATION_DATE_TIME.tag;

      assert_eq!(action_with_vr(tag, vr, &full_dates), Action::Keep);
      assert_eq!(action_with_vr(tag, vr, &modified_dates), Action::Clean);
    }

    // The timezone offset remains correct after a whole-day shift, and the
    // PTP timestamp is shifted like a date
    for (item, default, shifted) in [
      (
        &dictionary::TIMEZONE_OFFSET_FROM_UTC,
        Action::ReplaceWithDummy,
        Action::Keep,
      ),
      (
        &dictionary::FRAME_ORIGIN_TIMESTAMP,
        Action::ReplaceWithDummy,
        Action::Clean,
      ),
    ] {
      assert_eq!(action(item, &ProfileOptions::default()), default);
      assert_eq!(action(item, &full_dates), Action::Keep);
      assert_eq!(action(item, &modified_dates), shifted);
    }

    // The patient's birth date is not covered by the option
    assert_eq!(
      action(&dictionary::PATIENT_BIRTH_DATE, &full_dates),
      Action::ReplaceWithZeroLength
    );

    // Cleaning takes precedence over retaining when both apply
    let options = ProfileOptions {
      retain_device_identity: true,
      ..modified_dates
    };
    assert_eq!(
      action(&dictionary::DATE_OF_LAST_CALIBRATION, &options),
      Action::Clean
    );
  }

  #[test]
  fn clean_structured_content_option() {
    let options = ProfileOptions {
      clean_structured_content: true,
      ..Default::default()
    };

    for item in [
      &dictionary::CONTENT_SEQUENCE,
      &dictionary::ACQUISITION_CONTEXT_SEQUENCE,
      &dictionary::SPECIMEN_PREPARATION_STEP_CONTENT_ITEM_SEQUENCE,
      &dictionary::WAVEFORM_ANNOTATION_SEQUENCE,
      &dictionary::CONTENT_ITEM_MODIFIER_SEQUENCE,
    ] {
      assert_eq!(action(item, &options), Action::Clean, "{}", item.name);
      assert_ne!(
        action(item, &ProfileOptions::default()),
        Action::Clean,
        "{}",
        item.name
      );
    }

    // The Specimen Preparation Sequence itself holds the sequences of
    // content items rather than content items, so is recursed into
    assert_eq!(
      action(&dictionary::SPECIMEN_PREPARATION_SEQUENCE, &options),
      Action::Keep
    );
    assert_eq!(
      action(
        &dictionary::SPECIMEN_PREPARATION_SEQUENCE,
        &ProfileOptions::default()
      ),
      Action::ReplaceWithZeroLength
    );

    // Data elements inside content items are unaffected by the option
    assert_eq!(
      action(&dictionary::PERSON_NAME, &options),
      Action::ReplaceWithDummy
    );
    assert_eq!(action(&dictionary::UID, &options), Action::ReplaceUid);
    assert_eq!(
      action(&dictionary::TEXT_VALUE, &options),
      Action::ReplaceWithDummy
    );
  }

  #[test]
  fn content_item_actions() {
    let options = ProfileOptions::default();

    let action =
      |scheme: &str, code: &str, value_type: &str, options: &ProfileOptions| {
        action_for_content_item(
          Some(scheme),
          Some(code),
          Some(value_type),
          options,
        )
      };

    for (scheme, code, value_type, expected) in [
      ("DCM", "121022", "TEXT", ContentItemAction::Remove),
      ("DCM", "121008", "PNAME", ContentItemAction::ReplaceValue),
      ("DCM", "110180", "UIDREF", ContentItemAction::ReplaceValue),
      ("DCM", "111060", "DATE", ContentItemAction::ReplaceValue),
      ("NCDR", "76", "PNAME", ContentItemAction::ReplaceValue),
      ("LN", "11329-0", "TEXT", ContentItemAction::Remove),
      // Padding is ignored
      ("DCM ", "121022 ", "TEXT ", ContentItemAction::Remove),
      // Retired SNOMED codes are in the table
      ("SCT", "116224001", "TEXT", ContentItemAction::Remove),
      ("SRT", "DD-60002", "TEXT", ContentItemAction::Remove),
      // The value type is taken into account, falling back to the most
      // restrictive action across the value types listed for the concept
      ("DCM", "121112", "IMAGE", ContentItemAction::ReplaceValue),
      ("DCM", "121112", "WAVEFORM", ContentItemAction::Remove),
      ("DCM", "121112", "COMPOSITE", ContentItemAction::Remove),
      // Unlisted concept names are kept
      ("DCM", "111028", "IMAGE", ContentItemAction::Keep),
      ("99TEST", "1234", "TEXT", ContentItemAction::Keep),
    ] {
      assert_eq!(
        action(scheme, code, value_type, &options),
        expected,
        "{scheme} {code} {value_type}"
      );
    }
    assert_eq!(
      action_for_content_item(Some("DCM"), Some("121112"), None, &options),
      ContentItemAction::Remove
    );
    assert_eq!(
      action_for_content_item(None, None, Some("TEXT"), &options),
      ContentItemAction::Keep
    );

    // Each option retains the content items it lists
    let retain = |options: ProfileOptions| options;
    for (options, scheme, code, value_type) in [
      (
        retain(ProfileOptions {
          retain_uids: true,
          ..Default::default()
        }),
        "DCM",
        "110180",
        "UIDREF",
      ),
      (
        retain(ProfileOptions {
          retain_device_identity: true,
          ..Default::default()
        }),
        "DCM",
        "113880",
        "TEXT",
      ),
      (
        retain(ProfileOptions {
          retain_institution_identity: true,
          ..Default::default()
        }),
        "DCM",
        "113873",
        "TEXT",
      ),
      (
        retain(ProfileOptions {
          retain_patient_characteristics: true,
          ..Default::default()
        }),
        "DCM",
        "121033",
        "NUM",
      ),
      (full_dates(), "DCM", "111060", "DATE"),
    ] {
      assert_eq!(
        action(scheme, code, value_type, &options),
        ContentItemAction::Retain,
        "{code}"
      );
    }
    assert_eq!(
      action(
        "DCM",
        "121008",
        "PNAME",
        &ProfileOptions {
          retain_uids: true,
          ..Default::default()
        }
      ),
      ContentItemAction::ReplaceValue
    );

    // With modified dates a date content item is kept so that the rules for
    // its value's data element shift the date
    assert_eq!(
      action("DCM", "111060", "DATE", &modified_dates()),
      ContentItemAction::Keep
    );
  }

  #[test]
  fn supplementary_table_entries() {
    let options = ProfileOptions::default();

    // Structural attributes in the patient group are kept
    for (tag, vr) in [
      (
        dictionary::TYPE_OF_PATIENT_ID.tag,
        ValueRepresentation::CodeString,
      ),
      (
        dictionary::SUBJECT_RELATIVE_POSITION_IN_IMAGE.tag,
        ValueRepresentation::UnsignedShort,
      ),
    ] {
      assert_eq!(action_with_vr(tag, vr, &options), Action::Keep, "{tag}");
    }

    // Required labels and identifiers are replaced
    for (item, expected) in [
      (
        &dictionary::FIXATION_DEVICE_LABEL,
        Action::ReplaceWithZeroLength,
      ),
      (
        &dictionary::CONTENT_DESCRIPTION,
        Action::ReplaceWithZeroLength,
      ),
      (&dictionary::CONTENT_LABEL, Action::ReplaceWithDummy),
      (&dictionary::BEAM_NAME, Action::ReplaceWithDummy),
      (&dictionary::RT_IMAGE_LABEL, Action::ReplaceWithDummy),
      (&dictionary::DEVICE_ID, Action::ReplaceWithDummy),
      (&dictionary::RETRIEVE_URL, Action::ReplaceWithDummy),
      (&dictionary::HL7_INSTANCE_IDENTIFIER, Action::ReplaceUid),
      (
        &dictionary::MULTI_FRAME_SOURCE_SOP_INSTANCE_UID,
        Action::ReplaceUid,
      ),
    ] {
      assert_eq!(action(item, &options), expected, "{}", item.name);
    }
  }
}
