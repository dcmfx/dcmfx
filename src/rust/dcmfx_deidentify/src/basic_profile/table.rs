//! The data elements listed in Table E.1-1 of DICOM PS3.15, the action the
//! Basic Application Level Confidentiality Profile specifies for each of
//! them, and the options that retain or clean them.
//!
//! This file is generated from the DocBook source of DICOM PS3.15 2026d. The
//! wildcard entries in the table for curve data, overlay data, overlay comments,
//! and private data elements are not included here and are instead handled
//! directly by the lookup code.
//!
//! Conditional actions in the table are resolved to the action that preserves
//! IOD conformance regardless of the type of the data element in the IOD:
//! Z/D becomes D, X/Z becomes Z, X/D and X/Z/D become D, and X/Z/U* becomes K
//! so that the sequence is retained with the UIDs it contains replaced. The
//! original action code is noted alongside each such entry.

use dcmfx_core::DataElementTag;

use super::{
  CLEAN_STRUCTURED_CONTENT, D, Entry, K, RETAIN_DEVICE_IDENTITY,
  RETAIN_DEVICE_IDENTITY_CLEANED, RETAIN_INSTITUTION_IDENTITY,
  RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION, RETAIN_PATIENT_CHARACTERISTICS,
  RETAIN_PATIENT_CHARACTERISTICS_CLEANED, RETAIN_UIDS, U, X, Z,
};

const fn entry(
  group: u16,
  element: u16,
  basic_profile: crate::Action,
  options: u8,
) -> Entry {
  Entry {
    tag: DataElementTag { group, element },
    basic_profile,
    options,
  }
}

/// The data elements in Table E.1-1, sorted by tag so they can be binary
/// searched.
///
pub(crate) const TABLE: &[Entry] = &[
  // Affected SOP Instance UID
  entry(0x0000, 0x1000, X, RETAIN_UIDS),
  // Requested SOP Instance UID
  entry(0x0000, 0x1001, U, RETAIN_UIDS),
  // Media Storage SOP Instance UID
  entry(0x0002, 0x0003, U, RETAIN_UIDS),
  // Referenced SOP Instance UID in File
  entry(0x0004, 0x1511, U, RETAIN_UIDS),
  // Instance Creation Date (X/D)
  entry(0x0008, 0x0012, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Instance Creation Time (X/Z/D)
  entry(0x0008, 0x0013, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Instance Creator UID
  entry(0x0008, 0x0014, U, RETAIN_UIDS),
  // Instance Coercion DateTime
  entry(0x0008, 0x0015, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Acquisition UID
  entry(0x0008, 0x0017, U, RETAIN_UIDS),
  // SOP Instance UID
  entry(0x0008, 0x0018, U, RETAIN_UIDS),
  // Pyramid UID
  entry(0x0008, 0x0019, U, RETAIN_UIDS),
  // Study Date
  entry(0x0008, 0x0020, Z, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Series Date (X/D)
  entry(0x0008, 0x0021, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Acquisition Date (X/Z)
  entry(0x0008, 0x0022, Z, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Content Date (Z/D)
  entry(0x0008, 0x0023, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Overlay Date
  entry(0x0008, 0x0024, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Curve Date
  entry(0x0008, 0x0025, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Acquisition DateTime (X/Z/D)
  entry(0x0008, 0x002A, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Study Time
  entry(0x0008, 0x0030, Z, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Series Time (X/D)
  entry(0x0008, 0x0031, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Acquisition Time (X/Z)
  entry(0x0008, 0x0032, Z, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Content Time (Z/D)
  entry(0x0008, 0x0033, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Overlay Time
  entry(0x0008, 0x0034, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Curve Time
  entry(0x0008, 0x0035, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Accession Number
  entry(0x0008, 0x0050, Z, 0),
  // Retrieve AE Title
  entry(0x0008, 0x0054, X, RETAIN_DEVICE_IDENTITY_CLEANED),
  // Station AE Title
  entry(0x0008, 0x0055, X, RETAIN_DEVICE_IDENTITY_CLEANED),
  // Failed SOP Instance UID List
  entry(0x0008, 0x0058, U, RETAIN_UIDS),
  // Institution Name (X/Z/D)
  entry(0x0008, 0x0080, D, RETAIN_INSTITUTION_IDENTITY),
  // Institution Address
  entry(0x0008, 0x0081, X, RETAIN_INSTITUTION_IDENTITY),
  // Institution Code Sequence (X/Z/D)
  entry(0x0008, 0x0082, D, RETAIN_INSTITUTION_IDENTITY),
  // Referring Physician's Name
  entry(0x0008, 0x0090, Z, 0),
  // Referring Physician's Address
  entry(0x0008, 0x0092, X, 0),
  // Referring Physician's Telephone Numbers
  entry(0x0008, 0x0094, X, 0),
  // Referring Physician Identification Sequence
  entry(0x0008, 0x0096, X, 0),
  // Consulting Physician's Name
  entry(0x0008, 0x009C, Z, 0),
  // Consulting Physician Identification Sequence
  entry(0x0008, 0x009D, X, 0),
  // Context Group Version
  entry(0x0008, 0x0106, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Context Group Local Version
  entry(0x0008, 0x0107, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Timezone Offset From UTC
  entry(0x0008, 0x0201, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Network ID
  entry(0x0008, 0x1000, X, RETAIN_DEVICE_IDENTITY_CLEANED),
  // Station Name (X/Z/D)
  entry(0x0008, 0x1010, D, RETAIN_DEVICE_IDENTITY),
  // Study Description
  entry(0x0008, 0x1030, X, 0),
  // Series Description
  entry(0x0008, 0x103E, X, 0),
  // Institutional Department Name
  entry(0x0008, 0x1040, X, RETAIN_INSTITUTION_IDENTITY),
  // Institutional Department Type Code Sequence
  entry(0x0008, 0x1041, X, RETAIN_INSTITUTION_IDENTITY),
  // Physician(s) of Record
  entry(0x0008, 0x1048, X, 0),
  // Physician(s) of Record Identification Sequence
  entry(0x0008, 0x1049, X, 0),
  // Performing Physician's Name
  entry(0x0008, 0x1050, X, 0),
  // Performing Physician Identification Sequence
  entry(0x0008, 0x1052, X, 0),
  // Name of Physician(s) Reading Study
  entry(0x0008, 0x1060, X, 0),
  // Physician(s) Reading Study Identification Sequence
  entry(0x0008, 0x1062, X, 0),
  // Operators' Name (X/Z/D)
  entry(0x0008, 0x1070, D, 0),
  // Operator Identification Sequence (X/D)
  entry(0x0008, 0x1072, D, 0),
  // Admitting Diagnoses Description
  entry(0x0008, 0x1080, X, 0),
  // Admitting Diagnoses Code Sequence
  entry(0x0008, 0x1084, X, 0),
  // Pyramid Description
  entry(0x0008, 0x1088, X, 0),
  // Referenced Study Sequence (X/Z)
  entry(0x0008, 0x1110, Z, RETAIN_UIDS),
  // Referenced Performed Procedure Step Sequence (X/Z/D)
  entry(0x0008, 0x1111, D, RETAIN_UIDS),
  // Referenced Patient Sequence
  entry(0x0008, 0x1120, X, RETAIN_UIDS),
  // Referenced Image Sequence (X/Z/U*)
  entry(0x0008, 0x1140, K, RETAIN_UIDS),
  // Referenced SOP Instance UID
  entry(0x0008, 0x1155, U, RETAIN_UIDS),
  // Transaction UID
  entry(0x0008, 0x1195, U, RETAIN_UIDS),
  // Principal Diagnosis Code Sequence
  entry(0x0008, 0x1301, X, 0),
  // Primary Diagnosis Code Sequence
  entry(0x0008, 0x1302, X, 0),
  // Secondary Diagnoses Code Sequence
  entry(0x0008, 0x1303, X, 0),
  // Histological Diagnoses Code Sequence
  entry(0x0008, 0x1304, X, 0),
  // Derivation Description
  entry(0x0008, 0x2111, X, 0),
  // Source Image Sequence (X/Z/U*)
  entry(0x0008, 0x2112, K, RETAIN_UIDS),
  // Irradiation Event UID
  entry(0x0008, 0x3010, U, RETAIN_UIDS),
  // Identifying Comments
  entry(0x0008, 0x4000, X, 0),
  // Patient's Name
  entry(0x0010, 0x0010, Z, 0),
  // Person Names to Use Sequence
  entry(0x0010, 0x0011, X, 0),
  // Name to Use
  entry(0x0010, 0x0012, X, 0),
  // Name to Use Comment
  entry(0x0010, 0x0013, X, 0),
  // Third Person Pronouns Sequence
  entry(0x0010, 0x0014, X, 0),
  // Pronoun Code Sequence
  entry(0x0010, 0x0015, X, 0),
  // Pronoun Comment
  entry(0x0010, 0x0016, X, 0),
  // Patient ID (Z/D)
  entry(0x0010, 0x0020, D, 0),
  // Issuer of Patient ID
  entry(0x0010, 0x0021, X, 0),
  // Patient's Birth Date
  entry(0x0010, 0x0030, Z, 0),
  // Patient's Birth Time
  entry(0x0010, 0x0032, X, 0),
  // Patient's Sex
  entry(0x0010, 0x0040, Z, RETAIN_PATIENT_CHARACTERISTICS),
  // Gender Identity Sequence
  entry(0x0010, 0x0041, X, 0),
  // Sex Parameters for Clinical Use Category Comment
  entry(0x0010, 0x0042, X, RETAIN_PATIENT_CHARACTERISTICS_CLEANED),
  // Sex Parameters for Clinical Use Category Sequence
  entry(0x0010, 0x0043, X, RETAIN_PATIENT_CHARACTERISTICS),
  // Gender Identity Code Sequence
  entry(0x0010, 0x0044, X, 0),
  // Gender Identity Comment
  entry(0x0010, 0x0045, X, 0),
  // Sex Parameters for Clinical Use Category Code Sequence
  entry(0x0010, 0x0046, X, RETAIN_PATIENT_CHARACTERISTICS),
  // Sex Parameters for Clinical Use Category Reference
  entry(0x0010, 0x0047, X, RETAIN_PATIENT_CHARACTERISTICS),
  // Patient's Insurance Plan Code Sequence
  entry(0x0010, 0x0050, X, 0),
  // Patient's Primary Language Code Sequence
  entry(0x0010, 0x0101, X, 0),
  // Patient's Primary Language Modifier Code Sequence
  entry(0x0010, 0x0102, X, 0),
  // Other Patient IDs
  entry(0x0010, 0x1000, X, 0),
  // Other Patient Names
  entry(0x0010, 0x1001, X, 0),
  // Other Patient IDs Sequence
  entry(0x0010, 0x1002, X, 0),
  // Patient's Birth Name
  entry(0x0010, 0x1005, X, 0),
  // Patient's Age
  entry(0x0010, 0x1010, X, RETAIN_PATIENT_CHARACTERISTICS),
  // Patient's Size
  entry(0x0010, 0x1020, X, RETAIN_PATIENT_CHARACTERISTICS),
  // Patient's Weight
  entry(0x0010, 0x1030, X, RETAIN_PATIENT_CHARACTERISTICS),
  // Patient's Address
  entry(0x0010, 0x1040, X, 0),
  // Insurance Plan Identification
  entry(0x0010, 0x1050, X, 0),
  // Patient's Mother's Birth Name
  entry(0x0010, 0x1060, X, 0),
  // Military Rank
  entry(0x0010, 0x1080, X, 0),
  // Branch of Service
  entry(0x0010, 0x1081, X, 0),
  // Medical Record Locator
  entry(0x0010, 0x1090, X, 0),
  // Referenced Patient Photo Sequence
  entry(0x0010, 0x1100, X, 0),
  // Medical Alerts
  entry(0x0010, 0x2000, X, 0),
  // Allergies
  entry(0x0010, 0x2110, X, RETAIN_PATIENT_CHARACTERISTICS_CLEANED),
  // Country of Residence
  entry(0x0010, 0x2150, X, 0),
  // Region of Residence
  entry(0x0010, 0x2152, X, 0),
  // Patient's Telephone Numbers
  entry(0x0010, 0x2154, X, 0),
  // Patient's Telecom Information
  entry(0x0010, 0x2155, X, 0),
  // Ethnic Group
  entry(0x0010, 0x2160, X, RETAIN_PATIENT_CHARACTERISTICS),
  // Ethnic Group Code Sequence
  entry(0x0010, 0x2161, X, RETAIN_PATIENT_CHARACTERISTICS),
  // Ethnic Groups
  entry(0x0010, 0x2162, X, RETAIN_PATIENT_CHARACTERISTICS),
  // Occupation
  entry(0x0010, 0x2180, X, 0),
  // Smoking Status
  entry(0x0010, 0x21A0, X, RETAIN_PATIENT_CHARACTERISTICS),
  // Additional Patient History
  entry(0x0010, 0x21B0, X, 0),
  // Pregnancy Status
  entry(0x0010, 0x21C0, X, RETAIN_PATIENT_CHARACTERISTICS),
  // Last Menstrual Date
  entry(0x0010, 0x21D0, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Patient's Religious Preference
  entry(0x0010, 0x21F0, X, 0),
  // Patient's Sex Neutered (X/Z)
  entry(0x0010, 0x2203, Z, RETAIN_PATIENT_CHARACTERISTICS),
  // Responsible Person
  entry(0x0010, 0x2297, X, 0),
  // Responsible Organization
  entry(0x0010, 0x2299, X, 0),
  // Patient Comments
  entry(0x0010, 0x4000, X, 0),
  // Clinical Trial Sponsor Name
  entry(0x0012, 0x0010, D, 0),
  // Clinical Trial Protocol ID
  entry(0x0012, 0x0020, D, 0),
  // Clinical Trial Protocol Name
  entry(0x0012, 0x0021, Z, 0),
  // Issuer of Clinical Trial Protocol ID
  entry(0x0012, 0x0022, X, 0),
  // Other Clinical Trial Protocol IDs Sequence
  entry(0x0012, 0x0023, X, 0),
  // Clinical Trial Site ID
  entry(0x0012, 0x0030, Z, RETAIN_INSTITUTION_IDENTITY),
  // Clinical Trial Site Name
  entry(0x0012, 0x0031, Z, RETAIN_INSTITUTION_IDENTITY),
  // Issuer of Clinical Trial Site ID
  entry(0x0012, 0x0032, X, 0),
  // Clinical Trial Subject ID
  entry(0x0012, 0x0040, D, 0),
  // Issuer of Clinical Trial Subject ID
  entry(0x0012, 0x0041, X, 0),
  // Clinical Trial Subject Reading ID
  entry(0x0012, 0x0042, D, 0),
  // Issuer of Clinical Trial Subject Reading ID
  entry(0x0012, 0x0043, X, 0),
  // Clinical Trial Time Point ID
  entry(0x0012, 0x0050, Z, 0),
  // Clinical Trial Time Point Description
  entry(0x0012, 0x0051, X, 0),
  // Issuer of Clinical Trial Time Point ID
  entry(0x0012, 0x0055, X, 0),
  // Clinical Trial Coordinating Center Name
  entry(0x0012, 0x0060, Z, RETAIN_INSTITUTION_IDENTITY),
  // Clinical Trial Series ID
  entry(0x0012, 0x0071, X, 0),
  // Clinical Trial Series Description
  entry(0x0012, 0x0072, X, 0),
  // Issuer of Clinical Trial Series ID
  entry(0x0012, 0x0073, X, 0),
  // Clinical Trial Protocol Ethics Committee Name
  entry(0x0012, 0x0081, D, RETAIN_INSTITUTION_IDENTITY),
  // Clinical Trial Protocol Ethics Committee Approval Number
  entry(0x0012, 0x0082, X, 0),
  // Ethics Committee Approval Effectiveness Start Date
  entry(0x0012, 0x0086, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Ethics Committee Approval Effectiveness End Date
  entry(0x0012, 0x0087, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Calibration Time
  entry(
    0x0014,
    0x407C,
    X,
    RETAIN_DEVICE_IDENTITY | RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Calibration Date
  entry(
    0x0014,
    0x407E,
    X,
    RETAIN_DEVICE_IDENTITY | RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Maker Note
  entry(0x0016, 0x002B, X, 0),
  // Device Setting Description
  entry(0x0016, 0x004B, X, 0),
  // Camera Owner Name
  entry(0x0016, 0x004D, X, 0),
  // Lens Specification
  entry(0x0016, 0x004E, X, RETAIN_DEVICE_IDENTITY),
  // Lens Make
  entry(0x0016, 0x004F, X, RETAIN_DEVICE_IDENTITY),
  // Lens Model
  entry(0x0016, 0x0050, X, RETAIN_DEVICE_IDENTITY),
  // Lens Serial Number
  entry(0x0016, 0x0051, X, RETAIN_DEVICE_IDENTITY),
  // GPS Version ID
  entry(0x0016, 0x0070, X, 0),
  // GPS Latitude​ Ref
  entry(0x0016, 0x0071, X, 0),
  // GPS Latitude​
  entry(0x0016, 0x0072, X, 0),
  // GPS Longitude Ref
  entry(0x0016, 0x0073, X, 0),
  // GPS Longitude
  entry(0x0016, 0x0074, X, 0),
  // GPS Altitude​ Ref
  entry(0x0016, 0x0075, X, 0),
  // GPS Altitude​
  entry(0x0016, 0x0076, X, 0),
  // GPS Time​ Stamp
  entry(0x0016, 0x0077, X, 0),
  // GPS Satellites
  entry(0x0016, 0x0078, X, 0),
  // GPS Status
  entry(0x0016, 0x0079, X, 0),
  // GPS Measure ​Mode
  entry(0x0016, 0x007A, X, 0),
  // GPS DOP
  entry(0x0016, 0x007B, X, 0),
  // GPS Speed​ Ref
  entry(0x0016, 0x007C, X, 0),
  // GPS Speed​
  entry(0x0016, 0x007D, X, 0),
  // GPS Track ​Ref
  entry(0x0016, 0x007E, X, 0),
  // GPS Track
  entry(0x0016, 0x007F, X, 0),
  // GPS Img​ Direction Ref
  entry(0x0016, 0x0080, X, 0),
  // GPS Img ​Direction
  entry(0x0016, 0x0081, X, 0),
  // GPS Map​ Datum
  entry(0x0016, 0x0082, X, 0),
  // GPS Dest​ Latitude Ref
  entry(0x0016, 0x0083, X, 0),
  // GPS Dest​ Latitude
  entry(0x0016, 0x0084, X, 0),
  // GPS Dest ​Longitude Ref
  entry(0x0016, 0x0085, X, 0),
  // GPS Dest ​Longitude
  entry(0x0016, 0x0086, X, 0),
  // GPS Dest​ Bearing Ref
  entry(0x0016, 0x0087, X, 0),
  // GPS Dest ​Bearing
  entry(0x0016, 0x0088, X, 0),
  // GPS Dest ​Distance Ref
  entry(0x0016, 0x0089, X, 0),
  // GPS Dest ​Distance
  entry(0x0016, 0x008A, X, 0),
  // GPS Processing​ Method
  entry(0x0016, 0x008B, X, 0),
  // GPS Area ​Information
  entry(0x0016, 0x008C, X, 0),
  // GPS Date​ Stamp
  entry(0x0016, 0x008D, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // GPS Differential
  entry(0x0016, 0x008E, X, 0),
  // Contrast/Bolus Agent (Z/D)
  entry(0x0018, 0x0010, D, 0),
  // Intervention Drug Stop Time
  entry(0x0018, 0x0027, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Intervention Drug Start Time
  entry(0x0018, 0x0035, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Device Serial Number (X/Z/D)
  entry(0x0018, 0x1000, D, RETAIN_DEVICE_IDENTITY),
  // Device UID
  entry(0x0018, 0x1002, U, RETAIN_UIDS | RETAIN_DEVICE_IDENTITY),
  // Plate ID
  entry(0x0018, 0x1004, X, RETAIN_DEVICE_IDENTITY),
  // Generator ID
  entry(0x0018, 0x1005, X, RETAIN_DEVICE_IDENTITY),
  // Cassette ID
  entry(0x0018, 0x1007, X, RETAIN_DEVICE_IDENTITY),
  // Gantry ID
  entry(0x0018, 0x1008, X, RETAIN_DEVICE_IDENTITY),
  // Unique Device Identifier
  entry(0x0018, 0x1009, X, RETAIN_DEVICE_IDENTITY),
  // UDI Sequence
  entry(0x0018, 0x100A, X, RETAIN_DEVICE_IDENTITY),
  // Manufacturer's Device Class UID
  entry(0x0018, 0x100B, U, RETAIN_UIDS | RETAIN_DEVICE_IDENTITY),
  // Secondary Capture Device ID
  entry(0x0018, 0x1010, X, RETAIN_DEVICE_IDENTITY),
  // Hardcopy Creation Device ID
  entry(0x0018, 0x1011, X, RETAIN_DEVICE_IDENTITY),
  // Date of Secondary Capture
  entry(0x0018, 0x1012, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Time of Secondary Capture
  entry(0x0018, 0x1014, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Protocol Name (X/D)
  entry(0x0018, 0x1030, D, 0),
  // Contrast/Bolus Start Time
  entry(0x0018, 0x1042, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Contrast/Bolus Stop Time
  entry(0x0018, 0x1043, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Radiopharmaceutical Start Time
  entry(0x0018, 0x1072, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Radiopharmaceutical Stop Time
  entry(0x0018, 0x1073, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Radiopharmaceutical Start DateTime
  entry(0x0018, 0x1078, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Radiopharmaceutical Stop DateTime
  entry(0x0018, 0x1079, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Acquisition Field of View Label
  entry(0x0018, 0x11BB, D, 0),
  // Date of Last Calibration
  entry(
    0x0018,
    0x1200,
    X,
    RETAIN_DEVICE_IDENTITY | RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Time of Last Calibration
  entry(
    0x0018,
    0x1201,
    X,
    RETAIN_DEVICE_IDENTITY | RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // DateTime of Last Calibration
  entry(
    0x0018,
    0x1202,
    X,
    RETAIN_DEVICE_IDENTITY | RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Calibration DateTime
  entry(
    0x0018,
    0x1203,
    Z,
    RETAIN_DEVICE_IDENTITY | RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Date of Manufacture
  entry(
    0x0018,
    0x1204,
    X,
    RETAIN_DEVICE_IDENTITY | RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Date of Installation
  entry(
    0x0018,
    0x1205,
    X,
    RETAIN_DEVICE_IDENTITY | RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Acquisition Device Processing Description (X/D)
  entry(0x0018, 0x1400, D, 0),
  // Target UID
  entry(0x0018, 0x2042, U, RETAIN_UIDS),
  // Acquisition Comments
  entry(0x0018, 0x4000, X, 0),
  // Transducer Identification Sequence
  entry(0x0018, 0x5011, X, RETAIN_DEVICE_IDENTITY),
  // Detector ID (X/D)
  entry(0x0018, 0x700A, D, RETAIN_DEVICE_IDENTITY),
  // Date of Last Detector Calibration (X/D)
  entry(
    0x0018,
    0x700C,
    D,
    RETAIN_DEVICE_IDENTITY | RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Time of Last Detector Calibration (X/D)
  entry(
    0x0018,
    0x700E,
    D,
    RETAIN_DEVICE_IDENTITY | RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Frame Acquisition DateTime
  entry(0x0018, 0x9074, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Frame Reference DateTime
  entry(0x0018, 0x9151, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Respiratory Motion Compensation Technique Description
  entry(0x0018, 0x9185, X, 0),
  // X-Ray Source ID
  entry(0x0018, 0x9367, D, RETAIN_DEVICE_IDENTITY),
  // Source Start DateTime
  entry(0x0018, 0x9369, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Source End DateTime
  entry(0x0018, 0x936A, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // X-Ray Detector ID
  entry(0x0018, 0x9371, D, RETAIN_DEVICE_IDENTITY),
  // X-Ray Detector Label
  entry(0x0018, 0x9373, X, RETAIN_DEVICE_IDENTITY),
  // Multi-energy Acquisition Description
  entry(0x0018, 0x937B, X, 0),
  // Decomposition Description
  entry(0x0018, 0x937F, X, 0),
  // Acquisition Protocol Description
  entry(0x0018, 0x9424, X, 0),
  // Start Acquisition DateTime (X/D)
  entry(0x0018, 0x9516, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // End Acquisition DateTime (X/D)
  entry(0x0018, 0x9517, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Functional Sync Pulse
  entry(0x0018, 0x9623, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Decay Correction DateTime
  entry(0x0018, 0x9701, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Exclusion Start DateTime
  entry(0x0018, 0x9804, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Instruction Performed DateTime (Z/D)
  entry(0x0018, 0x9919, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Requested Series Description
  entry(0x0018, 0x9937, X, 0),
  // Contribution DateTime
  entry(0x0018, 0xA002, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Contribution Description
  entry(0x0018, 0xA003, X, 0),
  // Study Instance UID
  entry(0x0020, 0x000D, U, RETAIN_UIDS),
  // Series Instance UID
  entry(0x0020, 0x000E, U, RETAIN_UIDS),
  // Study ID
  entry(0x0020, 0x0010, Z, 0),
  // Pyramid Label
  entry(0x0020, 0x0027, X, 0),
  // Frame of Reference UID
  entry(0x0020, 0x0052, U, RETAIN_UIDS),
  // Synchronization Frame of Reference UID
  entry(0x0020, 0x0200, U, RETAIN_UIDS),
  // Modifying Device ID
  entry(0x0020, 0x3401, X, RETAIN_DEVICE_IDENTITY),
  // Modified Image Date
  entry(0x0020, 0x3403, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Modified Image Time
  entry(0x0020, 0x3405, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Modified Image Description
  entry(0x0020, 0x3406, X, 0),
  // Image Comments
  entry(0x0020, 0x4000, X, 0),
  // Frame Comments
  entry(0x0020, 0x9158, X, 0),
  // Concatenation UID
  entry(0x0020, 0x9161, U, RETAIN_UIDS),
  // Dimension Organization UID
  entry(0x0020, 0x9164, U, RETAIN_UIDS),
  // Palette Color Lookup Table UID
  entry(0x0028, 0x1199, U, RETAIN_UIDS),
  // Large Palette Color Lookup Table UID
  entry(0x0028, 0x1214, U, RETAIN_UIDS),
  // Image Presentation Comments
  entry(0x0028, 0x4000, X, 0),
  // Study ID Issuer
  entry(0x0032, 0x0012, X, 0),
  // Study Verified Date
  entry(0x0032, 0x0032, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Study Verified Time
  entry(0x0032, 0x0033, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Study Read Date
  entry(0x0032, 0x0034, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Study Read Time
  entry(0x0032, 0x0035, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Study Start Date
  entry(0x0032, 0x1000, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Study Start Time
  entry(0x0032, 0x1001, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Study Stop Date
  entry(0x0032, 0x1010, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Study Stop Time
  entry(0x0032, 0x1011, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Study Location
  entry(0x0032, 0x1020, X, RETAIN_DEVICE_IDENTITY),
  // Scheduled Study Location AE Title
  entry(0x0032, 0x1021, X, RETAIN_DEVICE_IDENTITY_CLEANED),
  // Reason for Study
  entry(0x0032, 0x1030, X, 0),
  // Requesting Physician
  entry(0x0032, 0x1032, X, 0),
  // Requesting Service
  entry(0x0032, 0x1033, X, 0),
  // Study Arrival Date
  entry(0x0032, 0x1040, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Study Arrival Time
  entry(0x0032, 0x1041, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Study Completion Date
  entry(0x0032, 0x1050, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Study Completion Time
  entry(0x0032, 0x1051, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Requested Procedure Description (X/Z)
  entry(0x0032, 0x1060, Z, 0),
  // Reason for Visit
  entry(0x0032, 0x1066, X, 0),
  // Reason for Visit Code Sequence
  entry(0x0032, 0x1067, X, 0),
  // Requested Contrast Agent
  entry(0x0032, 0x1070, X, 0),
  // Study Comments
  entry(0x0032, 0x4000, X, 0),
  // Flow Identifier Sequence
  entry(0x0034, 0x0001, D, 0),
  // Flow Identifier
  entry(0x0034, 0x0002, D, 0),
  // Source Identifier
  entry(0x0034, 0x0005, D, 0),
  // Frame Origin Timestamp
  entry(0x0034, 0x0007, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Referenced Patient Alias Sequence
  entry(0x0038, 0x0004, X, 0),
  // Admission ID
  entry(0x0038, 0x0010, X, 0),
  // Issuer of Admission ID
  entry(0x0038, 0x0011, X, 0),
  // Issuer of Admission ID Sequence
  entry(0x0038, 0x0014, X, 0),
  // Scheduled Admission Date
  entry(0x0038, 0x001A, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Admission Time
  entry(0x0038, 0x001B, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Discharge Date
  entry(0x0038, 0x001C, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Discharge Time
  entry(0x0038, 0x001D, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Patient Institution Residence
  entry(0x0038, 0x001E, X, 0),
  // Admitting Date
  entry(0x0038, 0x0020, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Admitting Time
  entry(0x0038, 0x0021, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Discharge Date
  entry(0x0038, 0x0030, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Discharge Time
  entry(0x0038, 0x0032, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Discharge Diagnosis Description
  entry(0x0038, 0x0040, X, 0),
  // Special Needs
  entry(0x0038, 0x0050, X, RETAIN_PATIENT_CHARACTERISTICS_CLEANED),
  // Service Episode ID
  entry(0x0038, 0x0060, X, 0),
  // Issuer of Service Episode ID
  entry(0x0038, 0x0061, X, 0),
  // Service Episode Description
  entry(0x0038, 0x0062, X, 0),
  // Issuer of Service Episode ID Sequence
  entry(0x0038, 0x0064, X, 0),
  // Current Patient Location
  entry(0x0038, 0x0300, X, 0),
  // Patient's Institution Residence
  entry(0x0038, 0x0400, X, 0),
  // Patient State
  entry(0x0038, 0x0500, X, RETAIN_PATIENT_CHARACTERISTICS_CLEANED),
  // Visit Comments
  entry(0x0038, 0x4000, X, 0),
  // Multiplex Group Label
  entry(0x003A, 0x0020, X, 0),
  // Channel Label
  entry(0x003A, 0x0203, X, 0),
  // Channel Derivation Description
  entry(0x003A, 0x020C, X, 0),
  // Multiplex Group UID
  entry(0x003A, 0x0310, U, RETAIN_UIDS),
  // Impedance Measurement DateTime
  entry(0x003A, 0x0314, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // ​Waveform Filter Description
  entry(0x003A, 0x0329, X, 0),
  // Filter Lookup Table Description
  entry(0x003A, 0x032B, X, 0),
  // Scheduled Station AE Title
  entry(0x0040, 0x0001, X, RETAIN_DEVICE_IDENTITY_CLEANED),
  // Scheduled Procedure Step Start Date
  entry(0x0040, 0x0002, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Procedure Step Start Time
  entry(0x0040, 0x0003, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Procedure Step End Date
  entry(0x0040, 0x0004, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Procedure Step End Time
  entry(0x0040, 0x0005, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Performing Physician's Name
  entry(0x0040, 0x0006, X, 0),
  // Scheduled Procedure Step Description
  entry(0x0040, 0x0007, X, 0),
  // Scheduled Procedure Step ID
  entry(0x0040, 0x0009, X, 0),
  // Scheduled Performing Physician Identification Sequence
  entry(0x0040, 0x000B, X, 0),
  // Scheduled Station Name
  entry(0x0040, 0x0010, X, RETAIN_DEVICE_IDENTITY),
  // Scheduled Procedure Step Location
  entry(0x0040, 0x0011, X, RETAIN_DEVICE_IDENTITY),
  // Pre-Medication
  entry(0x0040, 0x0012, X, RETAIN_PATIENT_CHARACTERISTICS_CLEANED),
  // Performed Station AE Title
  entry(0x0040, 0x0241, X, RETAIN_DEVICE_IDENTITY_CLEANED),
  // Performed Station Name
  entry(0x0040, 0x0242, X, RETAIN_DEVICE_IDENTITY),
  // Performed Location
  entry(0x0040, 0x0243, X, 0),
  // Performed Procedure Step Start Date
  entry(0x0040, 0x0244, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Performed Procedure Step Start Time
  entry(0x0040, 0x0245, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Performed Procedure Step End Date
  entry(0x0040, 0x0250, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Performed Procedure Step End Time
  entry(0x0040, 0x0251, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Performed Procedure Step ID
  entry(0x0040, 0x0253, X, 0),
  // Performed Procedure Step Description
  entry(0x0040, 0x0254, X, 0),
  // Request Attributes Sequence
  entry(0x0040, 0x0275, X, 0),
  // Comments on the Performed Procedure Step
  entry(0x0040, 0x0280, X, 0),
  // Comments on Radiation Dose
  entry(0x0040, 0x0310, X, 0),
  // Specimen Accession Number
  entry(0x0040, 0x050A, X, 0),
  // Container Identifier
  entry(0x0040, 0x0512, D, 0),
  // Issuer of the Container Identifier Sequence
  entry(0x0040, 0x0513, Z, 0),
  // Container Description
  entry(0x0040, 0x051A, X, 0),
  // Specimen Identifier
  entry(0x0040, 0x0551, D, 0),
  // Specimen UID
  entry(0x0040, 0x0554, U, RETAIN_UIDS),
  // Acquisition Context Sequence (X/Z)
  entry(0x0040, 0x0555, Z, CLEAN_STRUCTURED_CONTENT),
  // Acquisition Context Description
  entry(0x0040, 0x0556, X, 0),
  // Issuer of the Specimen Identifier Sequence
  entry(0x0040, 0x0562, Z, 0),
  // Specimen Short Description
  entry(0x0040, 0x0600, X, 0),
  // Specimen Detailed Description
  entry(0x0040, 0x0602, X, 0),
  // Specimen Preparation Sequence
  entry(0x0040, 0x0610, Z, CLEAN_STRUCTURED_CONTENT),
  // Slide Identifier
  entry(0x0040, 0x06FA, X, 0),
  // Requested Procedure ID
  entry(0x0040, 0x1001, X, 0),
  // Reason for the Requested Procedure
  entry(0x0040, 0x1002, X, 0),
  // Patient Transport Arrangements
  entry(0x0040, 0x1004, X, 0),
  // Requested Procedure Location
  entry(0x0040, 0x1005, X, 0),
  // Reason for Requested Procedure Code Sequence
  entry(0x0040, 0x100A, X, 0),
  // Names of Intended Recipients of Results
  entry(0x0040, 0x1010, X, 0),
  // Intended Recipients of Results Identification Sequence
  entry(0x0040, 0x1011, X, 0),
  // Person Identification Code Sequence
  entry(0x0040, 0x1101, D, 0),
  // Person's Address
  entry(0x0040, 0x1102, X, 0),
  // Person's Telephone Numbers
  entry(0x0040, 0x1103, X, 0),
  // Person's Telecom Information
  entry(0x0040, 0x1104, X, 0),
  // Requested Procedure Comments
  entry(0x0040, 0x1400, X, 0),
  // Reason for the Imaging Service Request
  entry(0x0040, 0x2001, X, 0),
  // Issue Date of Imaging Service Request
  entry(0x0040, 0x2004, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Issue Time of Imaging Service Request
  entry(0x0040, 0x2005, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Order Entered By
  entry(0x0040, 0x2008, X, 0),
  // Order Enterer's Location
  entry(0x0040, 0x2009, X, 0),
  // Order Callback Phone Number
  entry(0x0040, 0x2010, X, 0),
  // Order Callback Telecom Information
  entry(0x0040, 0x2011, X, 0),
  // Placer Order Number / Imaging Service Request
  entry(0x0040, 0x2016, Z, 0),
  // Filler Order Number / Imaging Service Request
  entry(0x0040, 0x2017, Z, 0),
  // Imaging Service Request Comments
  entry(0x0040, 0x2400, X, 0),
  // Confidentiality Constraint on Patient Data Description
  entry(0x0040, 0x3001, X, 0),
  // Scheduled Procedure Step Start DateTime
  entry(0x0040, 0x4005, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Procedure Step Expiration DateTime
  entry(0x0040, 0x4008, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Scheduled Procedure Step Modification DateTime
  entry(0x0040, 0x4010, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Expected Completion DateTime
  entry(0x0040, 0x4011, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Referenced General Purpose Scheduled Procedure Step Transaction UID
  entry(0x0040, 0x4023, U, RETAIN_UIDS),
  // Scheduled Station Name Code Sequence
  entry(0x0040, 0x4025, X, RETAIN_DEVICE_IDENTITY),
  // Scheduled Station Geographic Location Code Sequence
  entry(0x0040, 0x4027, X, RETAIN_DEVICE_IDENTITY),
  // Performed Station Name Code Sequence
  entry(0x0040, 0x4028, X, RETAIN_DEVICE_IDENTITY),
  // Performed Station Geographic Location Code Sequence
  entry(0x0040, 0x4030, X, RETAIN_DEVICE_IDENTITY),
  // Scheduled Human Performers Sequence
  entry(0x0040, 0x4034, X, 0),
  // Actual Human Performers Sequence
  entry(0x0040, 0x4035, X, 0),
  // Human Performer's Organization
  entry(0x0040, 0x4036, X, 0),
  // Human Performer's Name
  entry(0x0040, 0x4037, X, 0),
  // Performed Procedure Step Start DateTime
  entry(0x0040, 0x4050, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Performed Procedure Step End DateTime
  entry(0x0040, 0x4051, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Procedure Step Cancellation DateTime
  entry(0x0040, 0x4052, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Findings Group Recording Date (Trial)
  entry(0x0040, 0xA023, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Findings Group Recording Time (Trial)
  entry(0x0040, 0xA024, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Verifying Organization
  entry(0x0040, 0xA027, D, 0),
  // Verification DateTime
  entry(0x0040, 0xA030, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Observation DateTime (X/D)
  entry(0x0040, 0xA032, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Observation Start DateTime
  entry(0x0040, 0xA033, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Effective Start DateTime
  entry(0x0040, 0xA034, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Effective Stop DateTime
  entry(0x0040, 0xA035, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Verifying Observer Sequence
  entry(0x0040, 0xA073, D, 0),
  // Verifying Observer Name
  entry(0x0040, 0xA075, D, 0),
  // Author Observer Sequence
  entry(0x0040, 0xA078, X, 0),
  // Participant Sequence
  entry(0x0040, 0xA07A, X, 0),
  // Custodial Organization Sequence
  entry(0x0040, 0xA07C, X, 0),
  // Participation DateTime
  entry(0x0040, 0xA082, Z, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Verifying Observer Identification Code Sequence
  entry(0x0040, 0xA088, Z, 0),
  // Date of Document or Verbal Transaction (Trial)
  entry(0x0040, 0xA110, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Time of Document Creation or Verbal Transaction (Trial)
  entry(0x0040, 0xA112, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // DateTime
  entry(0x0040, 0xA120, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Date
  entry(0x0040, 0xA121, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Time
  entry(0x0040, 0xA122, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Person Name
  entry(0x0040, 0xA123, D, 0),
  // UID
  entry(0x0040, 0xA124, U, 0),
  // Referenced DateTime
  entry(0x0040, 0xA13A, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Observation UID
  entry(0x0040, 0xA171, U, RETAIN_UIDS),
  // Referenced Observation UID (Trial)
  entry(0x0040, 0xA172, U, RETAIN_UIDS),
  // Observation Date (Trial)
  entry(0x0040, 0xA192, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Observation Time (Trial)
  entry(0x0040, 0xA193, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Current Observer (Trial)
  entry(0x0040, 0xA307, X, 0),
  // Verbal Source (Trial)
  entry(0x0040, 0xA352, X, 0),
  // Address (Trial)
  entry(0x0040, 0xA353, X, 0),
  // Telephone Number (Trial)
  entry(0x0040, 0xA354, X, 0),
  // Verbal Source Identifier Code Sequence (Trial)
  entry(0x0040, 0xA358, X, 0),
  // Observation Subject UID (Trial)
  entry(0x0040, 0xA402, U, RETAIN_UIDS),
  // Content Sequence
  entry(0x0040, 0xA730, D, CLEAN_STRUCTURED_CONTENT),
  // Waveform Annotation Sequence (X/D)
  entry(0x0040, 0xB020, D, CLEAN_STRUCTURED_CONTENT),
  // Annotation DateTime
  entry(0x0040, 0xB034, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Segment Definition DateTime
  entry(0x0040, 0xB036, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Montage Name
  entry(0x0040, 0xB03B, X, 0),
  // Montage Channel Label
  entry(0x0040, 0xB03F, X, 0),
  // Template Version
  entry(0x0040, 0xDB06, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Template Local Version
  entry(0x0040, 0xDB07, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Template Extension Organization UID
  entry(0x0040, 0xDB0C, U, RETAIN_UIDS),
  // Template Extension Creator UID
  entry(0x0040, 0xDB0D, U, RETAIN_UIDS),
  // HL7 Document Effective Time
  entry(0x0040, 0xE004, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Display URI
  entry(0x0040, 0xE012, X, 0),
  // Encapsulated Document
  entry(0x0042, 0x0011, D, 0),
  // Approval Status DateTime
  entry(0x0044, 0x0004, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Product Expiration DateTime
  entry(0x0044, 0x000B, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Substance Administration DateTime
  entry(0x0044, 0x0010, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Assertion DateTime
  entry(0x0044, 0x0104, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Assertion Expiration DateTime
  entry(0x0044, 0x0105, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Container Component ID
  entry(0x0050, 0x001B, X, 0),
  // Device Description
  entry(0x0050, 0x0020, X, RETAIN_DEVICE_IDENTITY),
  // Long Device Description
  entry(0x0050, 0x0021, X, 0),
  // Tracking UID
  entry(0x0062, 0x0021, U, RETAIN_UIDS),
  // Source Frame of Reference UID
  entry(0x0064, 0x0003, U, RETAIN_UIDS),
  // Effective DateTime
  entry(0x0068, 0x6226, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Information Issue DateTime
  entry(0x0068, 0x6270, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Annotation Group UID
  entry(0x006A, 0x0003, D, RETAIN_UIDS),
  // Annotation Group Label
  entry(0x006A, 0x0005, D, 0),
  // Annotation Group Description
  entry(0x006A, 0x0006, X, 0),
  // Graphic Annotation Sequence
  entry(0x0070, 0x0001, D, 0),
  // Unformatted Text Value
  entry(0x0070, 0x0006, D, 0),
  // Presentation Creation Date
  entry(0x0070, 0x0082, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Presentation Creation Time
  entry(0x0070, 0x0083, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Content Creator's Name (Z/D)
  entry(0x0070, 0x0084, D, 0),
  // Content Creator's Identification Code Sequence
  entry(0x0070, 0x0086, X, 0),
  // Fiducial UID
  entry(0x0070, 0x031A, U, RETAIN_UIDS),
  // Presentation Display Collection UID
  entry(0x0070, 0x1101, U, RETAIN_UIDS),
  // Presentation Sequence Collection UID
  entry(0x0070, 0x1102, U, RETAIN_UIDS),
  // Hanging Protocol Creation DateTime
  entry(0x0072, 0x000A, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Selector AE Value
  entry(0x0072, 0x005E, D, RETAIN_DEVICE_IDENTITY_CLEANED),
  // Selector AS Value
  entry(0x0072, 0x005F, D, RETAIN_PATIENT_CHARACTERISTICS),
  // Selector DA Value
  entry(0x0072, 0x0061, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Selector DT Value
  entry(0x0072, 0x0063, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Selector OB Value
  entry(0x0072, 0x0065, D, 0),
  // Selector LO Value
  entry(0x0072, 0x0066, D, 0),
  // Selector LT Value
  entry(0x0072, 0x0068, D, 0),
  // Selector PN Value
  entry(0x0072, 0x006A, D, 0),
  // Selector TM Value
  entry(0x0072, 0x006B, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Selector SH Value
  entry(0x0072, 0x006C, D, 0),
  // Selector UN Value
  entry(0x0072, 0x006D, D, 0),
  // Selector ST Value
  entry(0x0072, 0x006E, D, 0),
  // Selector UT Value
  entry(0x0072, 0x0070, D, 0),
  // Selector UR Value
  entry(0x0072, 0x0071, D, 0),
  // Receiving AE
  entry(0x0074, 0x1234, X, RETAIN_DEVICE_IDENTITY_CLEANED),
  // Requesting AE
  entry(0x0074, 0x1236, X, RETAIN_DEVICE_IDENTITY_CLEANED),
  // Storage Media File-set UID
  entry(0x0088, 0x0140, U, RETAIN_UIDS),
  // Icon Image Sequence (see Note 11)
  entry(0x0088, 0x0200, X, 0),
  // Topic Title
  entry(0x0088, 0x0904, X, 0),
  // Topic Subject
  entry(0x0088, 0x0906, X, 0),
  // Topic Author
  entry(0x0088, 0x0910, X, 0),
  // Topic Keywords
  entry(0x0088, 0x0912, X, 0),
  // SOP Authorization DateTime
  entry(0x0100, 0x0420, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Digital Signature UID
  entry(0x0400, 0x0100, U, 0),
  // Digital Signature DateTime
  entry(0x0400, 0x0105, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Certificate of Signer
  entry(0x0400, 0x0115, D, 0),
  // Certified Timestamp
  entry(0x0400, 0x0310, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Referenced Digital Signature Sequence
  entry(0x0400, 0x0402, X, 0),
  // Referenced SOP Instance MAC Sequence
  entry(0x0400, 0x0403, X, 0),
  // MAC
  entry(0x0400, 0x0404, X, 0),
  // Modified Attributes Sequence
  entry(0x0400, 0x0550, X, 0),
  // Nonconforming Modified Attributes Sequence
  entry(0x0400, 0x0551, X, 0),
  // Nonconforming Data Element Value
  entry(0x0400, 0x0552, X, 0),
  // Original Attributes Sequence
  entry(0x0400, 0x0561, X, 0),
  // Attribute Modification DateTime
  entry(0x0400, 0x0562, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Modifying System
  entry(0x0400, 0x0563, D, RETAIN_DEVICE_IDENTITY),
  // Source of Previous Values
  entry(0x0400, 0x0564, Z, RETAIN_INSTITUTION_IDENTITY),
  // Reason for the Attribute Modification
  entry(0x0400, 0x0565, D, 0),
  // Instance Origin Status
  entry(0x0400, 0x0600, X, 0),
  // Text String
  entry(0x2030, 0x0020, X, 0),
  // Creation Date
  entry(0x2100, 0x0040, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Creation Time
  entry(0x2100, 0x0050, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Originator
  entry(0x2100, 0x0070, X, RETAIN_DEVICE_IDENTITY_CLEANED),
  // Destination AE
  entry(0x2100, 0x0140, D, RETAIN_DEVICE_IDENTITY_CLEANED),
  // Label Text (X/Z)
  entry(0x2200, 0x0002, Z, 0),
  // Barcode Value (X/Z)
  entry(0x2200, 0x0005, Z, 0),
  // Position Acquisition Template Name
  entry(0x3002, 0x0121, X, 0),
  // Position Acquisition Template Description
  entry(0x3002, 0x0123, X, 0),
  // Dose Calculation Model Name
  entry(0x3004, 0x007F, X, 0),
  // Structure Set Label
  entry(0x3006, 0x0002, D, 0),
  // Structure Set Name
  entry(0x3006, 0x0004, X, 0),
  // Structure Set Description
  entry(0x3006, 0x0006, X, 0),
  // Structure Set Date
  entry(0x3006, 0x0008, Z, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Structure Set Time
  entry(0x3006, 0x0009, Z, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Referenced Frame of Reference UID
  entry(0x3006, 0x0024, U, RETAIN_UIDS),
  // ROI Name
  entry(0x3006, 0x0026, Z, 0),
  // ROI Description
  entry(0x3006, 0x0028, X, 0),
  // ROI DateTime
  entry(0x3006, 0x002D, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // ROI Observation DateTime
  entry(0x3006, 0x002E, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // ROI Generation Description
  entry(0x3006, 0x0038, X, 0),
  // ROI Creator Sequence
  entry(0x3006, 0x004D, X, 0),
  // ROI Interpreter Sequence
  entry(0x3006, 0x004E, X, 0),
  // ROI Observation Label
  entry(0x3006, 0x0085, X, 0),
  // ROI Observation Description
  entry(0x3006, 0x0088, X, 0),
  // ROI Interpreter
  entry(0x3006, 0x00A6, Z, 0),
  // Related Frame of Reference UID
  entry(0x3006, 0x00C2, U, RETAIN_UIDS),
  // Treatment Control Point Date
  entry(0x3008, 0x0024, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Treatment Control Point Time
  entry(0x3008, 0x0025, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // First Treatment Date (X/D)
  entry(0x3008, 0x0054, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Most Recent Treatment Date (X/D)
  entry(0x3008, 0x0056, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Source Serial Number (X/Z)
  entry(0x3008, 0x0105, Z, RETAIN_DEVICE_IDENTITY),
  // Safe Position Exit Date
  entry(0x3008, 0x0162, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Safe Position Exit Time
  entry(0x3008, 0x0164, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Safe Position Return Date
  entry(0x3008, 0x0166, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Safe Position Return Time
  entry(0x3008, 0x0168, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Treatment Date (X/D)
  entry(0x3008, 0x0250, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Treatment Time (X/D)
  entry(0x3008, 0x0251, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // RT Plan Label
  entry(0x300A, 0x0002, D, 0),
  // RT Plan Name
  entry(0x300A, 0x0003, X, 0),
  // RT Plan Description
  entry(0x300A, 0x0004, X, 0),
  // RT Plan Date (X/D)
  entry(0x300A, 0x0006, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // RT Plan Time (X/D)
  entry(0x300A, 0x0007, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Treatment Sites
  entry(0x300A, 0x000B, X, 0),
  // Prescription Description
  entry(0x300A, 0x000E, X, 0),
  // Dose Reference UID
  entry(0x300A, 0x0013, U, RETAIN_UIDS),
  // Dose Reference Description
  entry(0x300A, 0x0016, X, 0),
  // Table Top Position Alignment UID
  entry(0x300A, 0x0054, U, RETAIN_UIDS),
  // Fraction Group Description
  entry(0x300A, 0x0072, X, 0),
  // Referenced Dose Reference UID
  entry(0x300A, 0x0083, U, RETAIN_UIDS),
  // Treatment Machine Name (X/Z)
  entry(0x300A, 0x00B2, Z, RETAIN_DEVICE_IDENTITY),
  // Beam Description
  entry(0x300A, 0x00C3, X, 0),
  // Bolus Description
  entry(0x300A, 0x00DD, X, 0),
  // Fixation Device Description
  entry(0x300A, 0x0196, X, 0),
  // Shielding Device Description
  entry(0x300A, 0x01A6, X, 0),
  // Setup Technique Description
  entry(0x300A, 0x01B2, X, 0),
  // Source Manufacturer
  entry(0x300A, 0x0216, X, RETAIN_DEVICE_IDENTITY),
  // Source Strength Reference Date
  entry(0x300A, 0x022C, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Source Strength Reference Time
  entry(0x300A, 0x022E, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Compensator Description
  entry(0x300A, 0x02EB, X, 0),
  // Treatment Position Group Label
  entry(0x300A, 0x0608, D, 0),
  // Treatment Position Group UID
  entry(0x300A, 0x0609, U, RETAIN_UIDS),
  // RT Accessory Holder Slot ID
  entry(0x300A, 0x0611, Z, 0),
  // RT Accessory Device Slot ID
  entry(0x300A, 0x0615, Z, 0),
  // Radiation Dose Identification Label
  entry(0x300A, 0x0619, D, 0),
  // Radiation Dose In-Vivo Measurement Label
  entry(0x300A, 0x0623, D, 0),
  // RT Tolerance Set Label
  entry(0x300A, 0x062A, D, 0),
  // Patient Setup UID
  entry(0x300A, 0x0650, U, RETAIN_UIDS),
  // Equipment Frame of Reference Description
  entry(0x300A, 0x0676, X, 0),
  // Radiation Generation Mode Label
  entry(0x300A, 0x067C, D, 0),
  // Radiation Generation Mode Description
  entry(0x300A, 0x067D, Z, 0),
  // Treatment Session UID
  entry(0x300A, 0x0700, U, RETAIN_UIDS),
  // Treatment Tolerance Violation Description
  entry(0x300A, 0x0734, D, 0),
  // Treatment Tolerance Violation DateTime
  entry(0x300A, 0x0736, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Recorded RT Control Point DateTime
  entry(0x300A, 0x073A, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Interlock DateTime
  entry(0x300A, 0x0741, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Interlock Description
  entry(0x300A, 0x0742, D, 0),
  // Override DateTime
  entry(0x300A, 0x0760, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Interlock Origin Description
  entry(0x300A, 0x0783, D, 0),
  // Referenced Treatment Position Group UID
  entry(0x300A, 0x0785, U, RETAIN_UIDS),
  // Patient Treatment Preparation Procedure Parameter Description
  entry(0x300A, 0x078E, X, 0),
  // Patient Treatment Preparation Method Description
  entry(0x300A, 0x0792, X, 0),
  // Patient Setup Photo Description
  entry(0x300A, 0x0794, X, 0),
  // Displacement Reference Label
  entry(0x300A, 0x079A, X, 0),
  // Reason for Omission Description
  entry(0x300C, 0x0113, X, 0),
  // Beam Hold Transition DateTime
  entry(
    0x300C,
    0x0127,
    D,
    RETAIN_DEVICE_IDENTITY | RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Review Date
  entry(0x300E, 0x0004, Z, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Review Time
  entry(0x300E, 0x0005, Z, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Reviewer Name (X/Z)
  entry(0x300E, 0x0008, Z, 0),
  // Conceptual Volume UID
  entry(0x3010, 0x0006, U, RETAIN_UIDS),
  // Referenced Conceptual Volume UID
  entry(0x3010, 0x000B, U, RETAIN_UIDS),
  // Conceptual Volume Combination Description
  entry(0x3010, 0x000F, Z, 0),
  // Constituent Conceptual Volume UID
  entry(0x3010, 0x0013, U, RETAIN_UIDS),
  // Source Conceptual Volume UID
  entry(0x3010, 0x0015, U, RETAIN_UIDS),
  // Conceptual Volume Description
  entry(0x3010, 0x0017, Z, 0),
  // Device Alternate Identifier
  entry(0x3010, 0x001B, Z, 0),
  // Device Label
  entry(0x3010, 0x002D, D, RETAIN_DEVICE_IDENTITY),
  // Referenced Fiducials UID
  entry(0x3010, 0x0031, U, RETAIN_UIDS),
  // User Content Label
  entry(0x3010, 0x0033, D, 0),
  // User Content Long Label
  entry(0x3010, 0x0034, D, 0),
  // Entity Label
  entry(0x3010, 0x0035, D, 0),
  // Entity Name
  entry(0x3010, 0x0036, X, 0),
  // Entity Description
  entry(0x3010, 0x0037, X, 0),
  // Entity Long Label
  entry(0x3010, 0x0038, D, 0),
  // RT Treatment Phase UID
  entry(0x3010, 0x003B, U, RETAIN_UIDS),
  // Manufacturer's Device Identifier
  entry(0x3010, 0x0043, Z, RETAIN_DEVICE_IDENTITY),
  // Intended Phase Start Date (X/D)
  entry(0x3010, 0x004C, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Intended Phase End Date (X/D)
  entry(0x3010, 0x004D, D, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // RT Prescription Label
  entry(0x3010, 0x0054, D, 0),
  // RT Treatment Approach Label (X/D)
  entry(0x3010, 0x0056, D, 0),
  // RT Physician Intent Narrative
  entry(0x3010, 0x005A, Z, 0),
  // Reason for Superseding
  entry(0x3010, 0x005C, Z, 0),
  // Prior Treatment Dose Description
  entry(0x3010, 0x0061, X, 0),
  // Dosimetric Objective UID
  entry(0x3010, 0x006E, U, RETAIN_UIDS),
  // Referenced Dosimetric Objective UID
  entry(0x3010, 0x006F, U, RETAIN_UIDS),
  // Treatment Site (X/D)
  entry(0x3010, 0x0077, D, 0),
  // Treatment Technique Notes
  entry(0x3010, 0x007A, Z, 0),
  // Prescription Notes
  entry(0x3010, 0x007B, Z, 0),
  // Fractionation Notes
  entry(0x3010, 0x007F, Z, 0),
  // Prescription Notes Sequence
  entry(0x3010, 0x0081, Z, 0),
  // Intended Fraction Start Time
  entry(0x3010, 0x0085, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Arbitrary
  entry(0x4000, 0x0010, X, 0),
  // Text Comments
  entry(0x4000, 0x4000, X, 0),
  // Results ID
  entry(0x4008, 0x0040, X, 0),
  // Results ID Issuer
  entry(0x4008, 0x0042, X, 0),
  // Interpretation Recorded Date
  entry(0x4008, 0x0100, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Interpretation Recorded Time
  entry(0x4008, 0x0101, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Interpretation Recorder
  entry(0x4008, 0x0102, X, 0),
  // Interpretation Transcription Date
  entry(0x4008, 0x0108, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Interpretation Transcription Time
  entry(0x4008, 0x0109, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Interpretation Transcriber
  entry(0x4008, 0x010A, X, 0),
  // Interpretation Text
  entry(0x4008, 0x010B, X, 0),
  // Interpretation Author
  entry(0x4008, 0x010C, X, 0),
  // Interpretation Approver Sequence
  entry(0x4008, 0x0111, X, 0),
  // Interpretation Approval Date
  entry(0x4008, 0x0112, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Interpretation Approval Time
  entry(0x4008, 0x0113, X, RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION),
  // Physician Approving Interpretation
  entry(0x4008, 0x0114, X, 0),
  // Interpretation Diagnosis Description
  entry(0x4008, 0x0115, X, 0),
  // Results Distribution List Sequence
  entry(0x4008, 0x0118, X, 0),
  // Distribution Name
  entry(0x4008, 0x0119, X, 0),
  // Distribution Address
  entry(0x4008, 0x011A, X, 0),
  // Interpretation ID
  entry(0x4008, 0x0200, X, 0),
  // Interpretation ID Issuer
  entry(0x4008, 0x0202, X, 0),
  // Impressions
  entry(0x4008, 0x0300, X, 0),
  // Results Comments
  entry(0x4008, 0x4000, X, 0),
  // Digital Signatures Sequence
  entry(0xFFFA, 0xFFFA, X, 0),
  // Data Set Trailing Padding
  entry(0xFFFC, 0xFFFC, X, 0),
];
