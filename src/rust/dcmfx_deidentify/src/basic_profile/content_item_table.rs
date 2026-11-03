//! The content item concept names listed in Table E.3.4-1 of DICOM PS3.15,
//! the action the Basic Application Level Confidentiality Profile specifies
//! for content items with each of them, and the options that retain them.
//!
//! This file is generated from the DocBook source of DICOM PS3.15 2026d.
//! Retired SNOMED codes for the SNOMED CT concepts in the table are included
//! using the mapping in Annex O of DICOM PS3.16, as the standard requires
//! de-identifiers to continue to recognize them.
//!
//! Conditional actions in the table are resolved in the same way as for
//! Table E.1-1. The original action code is noted alongside each such entry.

use super::{
  ContentItemEntry, D, RETAIN_DEVICE_IDENTITY, RETAIN_INSTITUTION_IDENTITY,
  RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION, RETAIN_PATIENT_CHARACTERISTICS,
  RETAIN_UIDS, X,
};

const fn entry(
  coding_scheme_designator: &'static str,
  code_value: &'static str,
  value_type: &'static str,
  basic_profile: crate::Action,
  options: u8,
) -> ContentItemEntry {
  ContentItemEntry {
    coding_scheme_designator,
    code_value,
    value_type,
    basic_profile,
    options,
  }
}

/// The content item concept names in Table E.3.4-1, sorted by coding scheme
/// designator, code value, and value type so they can be binary searched.
///
pub(crate) const TABLE: &[ContentItemEntry] = &[
  // Complication of Procedure (retired code for SCT 116224001)
  entry("99SDM", "DD-60002", "TEXT", X, 0),
  // Anesthesia Start Time (retired code for SCT 398325003)
  entry(
    "99SDM",
    "DF-0068E",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Anesthesia Finish Time (retired code for SCT 398164008)
  entry(
    "99SDM",
    "DF-0070B",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // ECG Finding (retired code for SCT 271921002) (X/D)
  entry("99SDM", "F-00033", "TEXT", D, 0),
  // Social History (retired code for SCT 160476009)
  entry("99SDM", "F-02455", "TEXT", X, 0),
  // Finding Site (retired code for SCT 363698007)
  entry("99SDM", "G-C0E3", "TEXT", D, 0),
  // Stop DateTime (retired code for SCT 397898000)
  entry(
    "99SDM",
    "G-D320",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Start DateTime (retired code for SCT 398201009)
  entry(
    "99SDM",
    "G-D321",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Administration of radiopharmaceutical (retired code for SCT 440252007)
  entry("99SDM", "PA-30029", "TEXT", D, 0),
  // Clinical Report (retired code for SCT 371524004)
  entry("99SDM", "R-42B89", "COMPOSITE", X, RETAIN_UIDS),
  // Clinical Report (retired code for SCT 371524004)
  entry("99SDM", "R-42B89", "TEXT", X, 0),
  // Patient State
  entry("DCM", "109054", "TEXT", X, 0),
  // Stress Protocol
  entry("DCM", "109056", "TEXT", X, 0),
  // Station AE Title
  entry("DCM", "110119", "TEXT", X, RETAIN_DEVICE_IDENTITY),
  // Study Instance UID
  entry("DCM", "110180", "UIDREF", D, RETAIN_UIDS),
  // Issuer of Identifier
  entry("DCM", "110190", "TEXT", X, 0),
  // Content Date (X/D)
  entry(
    "DCM",
    "111018",
    "DATE",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Content Time (X/D)
  entry(
    "DCM",
    "111019",
    "TIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Description of Change
  entry("DCM", "111021", "TEXT", X, 0),
  // Impression Description
  entry("DCM", "111033", "TEXT", D, 0),
  // Original Source
  entry("DCM", "111040", "COMPOSITE", D, RETAIN_UIDS),
  // Recommended Follow-up Date (X/D)
  entry(
    "DCM",
    "111054",
    "DATE",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Selected Region Description
  entry("DCM", "111058", "TEXT", D, 0),
  // Study Date (X/D)
  entry(
    "DCM",
    "111060",
    "DATE",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Study Time (X/D)
  entry(
    "DCM",
    "111061",
    "TIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Sampling DateTime
  entry(
    "DCM",
    "111469",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Medication Type
  entry("DCM", "111516", "TEXT", X, 0),
  // DateTime Started (X/D)
  entry(
    "DCM",
    "111526",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // DateTime Ended
  entry(
    "DCM",
    "111527",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // DateTime problem observed
  entry(
    "DCM",
    "111535",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // DateTime of last evaluation
  entry(
    "DCM",
    "111536",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Specimen Container Identifier
  entry("DCM", "111700", "TEXT", X, 0),
  // DateTime of processing
  entry(
    "DCM",
    "111702",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Processing step description
  entry("DCM", "111703", "TEXT", X, 0),
  // Parent Specimen Identifier
  entry("DCM", "111705", "TEXT", D, 0),
  // Issuer of Parent Specimen Identifier
  entry("DCM", "111706", "TEXT", X, 0),
  // Issuer of Specimen Identifier
  entry("DCM", "111724", "TEXT", X, 0),
  // Series Instance UID
  entry("DCM", "112002", "UIDREF", D, RETAIN_UIDS),
  // Tracking Unique Identifier
  entry("DCM", "112040", "UIDREF", D, RETAIN_UIDS),
  // Anatomic Identifier
  entry("DCM", "112050", "TEXT", X, 0),
  // Frame of Reference UID (X/D)
  entry("DCM", "112227", "UIDREF", D, RETAIN_UIDS),
  // Identifying Segment
  entry("DCM", "112229", "IMAGE", D, RETAIN_UIDS),
  // Component ID
  entry("DCM", "112347", "TEXT", D, 0),
  // Mating Feature Set ID
  entry("DCM", "112351", "TEXT", D, 0),
  // Mating Feature ID
  entry("DCM", "112352", "TEXT", D, 0),
  // Spatial Registration
  entry("DCM", "112353", "COMPOSITE", X, RETAIN_UIDS),
  // Patient Image
  entry("DCM", "112354", "IMAGE", X, RETAIN_UIDS),
  // User Selected Fiducial
  entry("DCM", "112356", "UIDREF", D, RETAIN_UIDS),
  // Derived Fiducial
  entry("DCM", "112357", "UIDREF", D, RETAIN_UIDS),
  // Supporting Information
  entry("DCM", "112359", "COMPOSITE", X, RETAIN_UIDS),
  // Patient Data Used During Planning
  entry("DCM", "112361", "COMPOSITE", X, RETAIN_UIDS),
  // Degree of Freedom ID
  entry("DCM", "112363", "TEXT", D, 0),
  // Related Patient Data Not Used During Planning
  entry("DCM", "112364", "COMPOSITE", X, RETAIN_UIDS),
  // Implant Assembly Template
  entry("DCM", "112366", "COMPOSITE", X, RETAIN_UIDS),
  // Manufacturer Implant Template
  entry("DCM", "112371", "COMPOSITE", D, RETAIN_UIDS),
  // Derived Planning Images
  entry("DCM", "112372", "COMPOSITE", X, RETAIN_UIDS),
  // Derived Planning Data
  entry("DCM", "112373", "COMPOSITE", X, RETAIN_UIDS),
  // Key Object Description
  entry("DCM", "113012", "TEXT", X, 0),
  // Radiopharmaceutical Administration Event UID
  entry("DCM", "113503", "UIDREF", D, RETAIN_UIDS),
  // Radiopharmaceutical Dispense Unit Identifier
  entry("DCM", "113511", "TEXT", X, 0),
  // Radiopharmaceutical Lot Identifier
  entry("DCM", "113512", "TEXT", X, 0),
  // Reagent Vial Identifier
  entry("DCM", "113513", "TEXT", X, 0),
  // Radionuclide Identifier
  entry("DCM", "113514", "TEXT", X, 0),
  // Prescription Identifier
  entry("DCM", "113516", "TEXT", X, 0),
  // Recent Physical Activity
  entry("DCM", "113552", "TEXT", X, 0),
  // Irradiation Event Label
  entry("DCM", "113605", "TEXT", X, 0),
  // X-Ray Radiation Dose Report
  entry("DCM", "113701", "COMPOSITE", X, RETAIN_UIDS),
  // Calibration Protocol
  entry("DCM", "113720", "TEXT", X, 0),
  // Calibration DateTime
  entry(
    "DCM",
    "113723",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Calibration Responsible Party
  entry("DCM", "113724", "TEXT", D, 0),
  // Irradiation Event UID
  entry("DCM", "113769", "UIDREF", D, RETAIN_UIDS),
  // Acquired Image
  entry("DCM", "113795", "IMAGE", D, RETAIN_UIDS),
  // Start of X-Ray Irradiation
  entry(
    "DCM",
    "113809",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // End of X-Ray Irradiation
  entry(
    "DCM",
    "113810",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Patient Model
  entry("DCM", "113815", "TEXT", D, 0),
  // Identification of the X-Ray Source
  entry("DCM", "113832", "TEXT", D, RETAIN_DEVICE_IDENTITY),
  // Irradiation Authorizing
  entry("DCM", "113850", "PNAME", D, 0),
  // Person Name
  entry("DCM", "113870", "PNAME", D, 0),
  // Person ID
  entry("DCM", "113871", "TEXT", X, 0),
  // Person ID Issuer
  entry("DCM", "113872", "TEXT", X, 0),
  // Organization Name
  entry("DCM", "113873", "TEXT", X, RETAIN_INSTITUTION_IDENTITY),
  // Device Name
  entry("DCM", "113877", "TEXT", X, RETAIN_DEVICE_IDENTITY),
  // Device Serial Number (X/D)
  entry("DCM", "113880", "TEXT", D, RETAIN_DEVICE_IDENTITY),
  // Reason for Proceeding
  entry("DCM", "113907", "TEXT", X, 0),
  // Series or Instance used for Water Equivalent Diameter estimation
  entry("DCM", "113985", "UIDREF", D, RETAIN_UIDS),
  // Device Description
  entry("DCM", "120999", "TEXT", X, RETAIN_DEVICE_IDENTITY),
  // Unique Device Identifiers
  entry("DCM", "121000", "CONTAINER", X, RETAIN_DEVICE_IDENTITY),
  // Quoted Source
  entry("DCM", "121002", "COMPOSITE", D, RETAIN_UIDS),
  // Person Observer Name
  entry("DCM", "121008", "PNAME", D, 0),
  // Person Observer's Organization Name
  entry("DCM", "121009", "TEXT", X, RETAIN_INSTITUTION_IDENTITY),
  // Device Observer UID (X/D)
  entry(
    "DCM",
    "121012",
    "UIDREF",
    D,
    RETAIN_UIDS | RETAIN_DEVICE_IDENTITY,
  ),
  // Device Observer Name
  entry("DCM", "121013", "TEXT", X, RETAIN_DEVICE_IDENTITY),
  // Device Observer Serial Number
  entry("DCM", "121016", "TEXT", X, RETAIN_DEVICE_IDENTITY),
  // Device Observer Physical Location During Observation
  entry("DCM", "121017", "TEXT", X, RETAIN_INSTITUTION_IDENTITY),
  // Procedure Study Instance UID
  entry("DCM", "121018", "UIDREF", X, RETAIN_UIDS),
  // Procedure Study Component UID
  entry("DCM", "121019", "UIDREF", X, RETAIN_UIDS),
  // Placer Number
  entry("DCM", "121020", "TEXT", X, 0),
  // Filler Number
  entry("DCM", "121021", "TEXT", X, 0),
  // Accession Number
  entry("DCM", "121022", "TEXT", X, 0),
  // Subject UID
  entry("DCM", "121028", "UIDREF", X, RETAIN_UIDS),
  // Subject Name
  entry("DCM", "121029", "PNAME", D, 0),
  // Subject ID (X/D)
  entry("DCM", "121030", "TEXT", D, 0),
  // Subject Birth Date
  entry("DCM", "121031", "DATE", X, 0),
  // Subject Sex
  entry("DCM", "121032", "CODE", X, RETAIN_PATIENT_CHARACTERISTICS),
  // Subject Age
  entry("DCM", "121033", "NUM", X, RETAIN_PATIENT_CHARACTERISTICS),
  // Mother of fetus
  entry("DCM", "121036", "PNAME", X, 0),
  // Specimen UID
  entry("DCM", "121039", "UIDREF", X, RETAIN_UIDS),
  // Specimen Identifier
  entry("DCM", "121041", "TEXT", X, 0),
  // Procedure Description (X/D)
  entry("DCM", "121065", "TEXT", D, 0),
  // Finding (X/D)
  entry("DCM", "121071", "TEXT", D, 0),
  // Recommendation
  entry("DCM", "121075", "TEXT", X, 0),
  // Conclusion
  entry("DCM", "121077", "TEXT", X, 0),
  // Best illustration of finding
  entry("DCM", "121080", "IMAGE", X, RETAIN_UIDS),
  // Best illustration of finding
  entry("DCM", "121080", "WAVEFORM", X, RETAIN_UIDS),
  // Fellow
  entry("DCM", "121088", "PNAME", X, 0),
  // Comment
  entry("DCM", "121106", "TEXT", X, 0),
  // Patient Presentation
  entry("DCM", "121110", "TEXT", X, 0),
  // Summary
  entry("DCM", "121111", "TEXT", X, 0),
  // Source of Measurement
  entry("DCM", "121112", "IMAGE", D, RETAIN_UIDS),
  // Source of Measurement
  entry("DCM", "121112", "WAVEFORM", X, RETAIN_UIDS),
  // Performing Physician
  entry("DCM", "121114", "PNAME", D, 0),
  // Cath Lab Procedure Log
  entry("DCM", "121120", "COMPOSITE", X, RETAIN_UIDS),
  // Room Identification
  entry("DCM", "121121", "TEXT", X, 0),
  // Equipment Identification
  entry("DCM", "121122", "TEXT", X, RETAIN_DEVICE_IDENTITY),
  // Procedure Action ID (X/D)
  entry("DCM", "121124", "TEXT", D, 0),
  // DateTime of Recording of Log Entry
  entry(
    "DCM",
    "121125",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Performed Procedure Step SOP Instance UID
  entry("DCM", "121126", "UIDREF", D, RETAIN_UIDS),
  // Image Acquired
  entry("DCM", "121138", "IMAGE", D, RETAIN_UIDS),
  // Waveform Acquired
  entry("DCM", "121143", "WAVEFORM", D, RETAIN_UIDS),
  // Description of Material
  entry("DCM", "121145", "TEXT", X, 0),
  // Person administering drug/contrast
  entry("DCM", "121152", "PNAME", X, 0),
  // Intervention attempt identifier
  entry("DCM", "121154", "TEXT", D, 0),
  // Physician Note
  entry("DCM", "121173", "TEXT", X, 0),
  // Referenced Segment (X/D)
  entry("DCM", "121191", "IMAGE", D, RETAIN_UIDS),
  // Device Subject Name
  entry("DCM", "121193", "TEXT", D, RETAIN_DEVICE_IDENTITY),
  // Device Subject Serial Number
  entry("DCM", "121196", "TEXT", X, RETAIN_DEVICE_IDENTITY),
  // Device Subject Physical Location during observation
  entry("DCM", "121197", "TEXT", X, RETAIN_INSTITUTION_IDENTITY),
  // Device Subject UID
  entry(
    "DCM",
    "121198",
    "UIDREF",
    X,
    RETAIN_UIDS | RETAIN_DEVICE_IDENTITY,
  ),
  // Illustration of ROI
  entry("DCM", "121200", "IMAGE", X, RETAIN_UIDS),
  // Referenced Segmentation Frame
  entry("DCM", "121214", "IMAGE", D, RETAIN_UIDS),
  // Source series for segmentation
  entry("DCM", "121232", "UIDREF", D, RETAIN_UIDS),
  // Source image for segmentation
  entry("DCM", "121233", "IMAGE", D, RETAIN_UIDS),
  // Dose Image
  entry("DCM", "121342", "IMAGE", X, RETAIN_UIDS),
  // DateTime Concern Noted
  entry(
    "DCM",
    "121431",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // DateTime Concern Resolved
  entry(
    "DCM",
    "121432",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // DateTime Problem Resolved
  entry(
    "DCM",
    "121433",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Service Delivery Location
  entry("DCM", "121434", "TEXT", X, RETAIN_INSTITUTION_IDENTITY),
  // Service Performer
  entry("DCM", "121435", "PNAME", X, 0),
  // Service Performer
  entry("DCM", "121435", "TEXT", X, 0),
  // Current procedure evidence
  entry("DCM", "122073", "COMPOSITE", X, RETAIN_UIDS),
  // Prior report for current patient
  entry("DCM", "122075", "COMPOSITE", X, RETAIN_UIDS),
  // Drug start
  entry(
    "DCM",
    "122081",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Drug end
  entry(
    "DCM",
    "122082",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Drug administered (X/D)
  entry("DCM", "122083", "TEXT", D, 0),
  // DateTime of Intervention
  entry(
    "DCM",
    "122105",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Patient Transferred From
  entry("DCM", "122128", "TEXT", X, RETAIN_INSTITUTION_IDENTITY),
  // Procedure DateTime (X/D)
  entry(
    "DCM",
    "122146",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Discharge DateTime
  entry(
    "DCM",
    "122163",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // DateTime of Death
  entry(
    "DCM",
    "122165",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // ST Elevation Onset DateTime
  entry(
    "DCM",
    "122173",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Procedure Result
  entry("DCM", "122177", "TEXT", X, 0),
  // Procedure Time Base
  entry(
    "DCM",
    "122701",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Imaging Start DateTime
  entry(
    "DCM",
    "122712",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Radiopharmaceutical Start DateTime (X/D)
  entry(
    "DCM",
    "123003",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Radiopharmaceutical Stop DateTime
  entry(
    "DCM",
    "123004",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Identifier
  entry("DCM", "125010", "TEXT", X, 0),
  // Illustration of Finding
  entry("DCM", "125201", "IMAGE", X, RETAIN_UIDS),
  // Acquisition Protocol
  entry("DCM", "125203", "TEXT", X, 0),
  // Subject Time Point Identifier
  entry("DCM", "126070", "TEXT", X, 0),
  // Protocol Time Point Identifier
  entry("DCM", "126071", "TEXT", X, 0),
  // Real World Value Map used for measurement
  entry("DCM", "126100", "COMPOSITE", X, RETAIN_UIDS),
  // Acquisition Date
  entry(
    "DCM",
    "126201",
    "DATE",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Acquisition Time
  entry(
    "DCM",
    "126202",
    "TIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Glucose Measurement Date
  entry(
    "DCM",
    "127857",
    "DATE",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Glucose Measurement Time
  entry(
    "DCM",
    "127858",
    "TIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Pulse Sequence Name
  entry("DCM", "128230", "TEXT", X, 0),
  // Radiation Dose Estimate Name
  entry("DCM", "128403", "TEXT", D, 0),
  // Radiation Dose Representation Data
  entry("DCM", "128414", "COMPOSITE", D, RETAIN_UIDS),
  // Radiation Dose Representation Data
  entry("DCM", "128414", "IMAGE", D, RETAIN_UIDS),
  // SR Instance Used
  entry("DCM", "128416", "COMPOSITE", D, RETAIN_UIDS),
  // Patient Radiation Dose Model Data
  entry("DCM", "128425", "COMPOSITE", X, RETAIN_UIDS),
  // Patient Radiation Dose Model Data
  entry("DCM", "128425", "IMAGE", X, RETAIN_UIDS),
  // Patient Radiation Dose Model Data
  entry("DCM", "128425", "UIDREF", X, RETAIN_UIDS),
  // Patient Radiation Dose Model Reference
  entry("DCM", "128426", "TEXT", X, 0),
  // Event UID Used
  entry("DCM", "128429", "UIDREF", D, RETAIN_UIDS),
  // Radiation Dose Composite Parameters
  entry("DCM", "128436", "COMPOSITE", D, RETAIN_UIDS),
  // Spatial Registration Reference
  entry("DCM", "128444", "COMPOSITE", D, RETAIN_UIDS),
  // Spatial Fiducials
  entry("DCM", "128447", "COMPOSITE", X, RETAIN_UIDS),
  // X-Ray Attenuator Model Data
  entry("DCM", "128470", "COMPOSITE", X, RETAIN_UIDS),
  // X-Ray Attenuator Model Data
  entry("DCM", "128470", "IMAGE", X, RETAIN_UIDS),
  // X-Ray Attenuator Model Data
  entry("DCM", "128470", "UIDREF", X, RETAIN_UIDS),
  // Person Observer's Login Name
  entry("DCM", "128774", "TEXT", X, 0),
  // Identifier within Person Observer's Role
  entry("DCM", "128775", "TEXT", X, 0),
  // RDSR Frame of Reference Description
  entry("DCM", "130507", "TEXT", X, 0),
  // Identification of the Attenuator
  entry("DCM", "130527", "TEXT", D, RETAIN_INSTITUTION_IDENTITY),
  // Acquisition DateTime
  entry(
    "DCM",
    "130884",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Synchronization Frame of Reference UID
  entry("DCM", "130885", "UIDREF", X, RETAIN_UIDS),
  // Series Date
  entry(
    "DCM",
    "131561",
    "DATE",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Series Time
  entry(
    "DCM",
    "131562",
    "TIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Series Description
  entry("DCM", "131563", "TEXT", X, 0),
  // History
  entry("LN", "11329-0", "TEXT", X, 0),
  // EDD
  entry(
    "LN",
    "11778-8",
    "DATE",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Fetus ID
  entry("LN", "11951-1", "TEXT", D, 0),
  // Date of last menstrual period
  entry(
    "LN",
    "11955-2",
    "DATE",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // LV Wall Motion Segmental Findings
  entry("LN", "18118-0", "TEXT", X, 0),
  // Indications for Procedure
  entry("LN", "18785-6", "TEXT", D, 0),
  // Unique Device Identifier
  entry("LN", "74711-3", "TEXT", X, RETAIN_DEVICE_IDENTITY),
  // Admission DateTime
  entry(
    "NCDR",
    "15",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Procedure DateTime
  entry(
    "NCDR",
    "52",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Procedure Number in this admission
  entry("NCDR", "53", "TEXT", X, 0),
  // Catheterization Operator
  entry("NCDR", "76", "PNAME", D, 0),
  // Experience
  entry("NCIt", "C54627", "NUM", X, 0),
  // Activity Session
  entry("NCIt", "C67447", "TEXT", X, 0),
  // Complication of Procedure
  entry("SCT", "116224001", "TEXT", X, 0),
  // Social History
  entry("SCT", "160476009", "TEXT", X, 0),
  // ECG Finding (X/D)
  entry("SCT", "271921002", "TEXT", D, 0),
  // Finding Site
  entry("SCT", "363698007", "TEXT", D, 0),
  // Clinical Report
  entry("SCT", "371524004", "COMPOSITE", X, RETAIN_UIDS),
  // Clinical Report
  entry("SCT", "371524004", "TEXT", X, 0),
  // Stop DateTime
  entry(
    "SCT",
    "397898000",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Anesthesia Finish Time
  entry(
    "SCT",
    "398164008",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Start DateTime
  entry(
    "SCT",
    "398201009",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Anesthesia Start Time
  entry(
    "SCT",
    "398325003",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Administration of radiopharmaceutical
  entry("SCT", "440252007", "TEXT", D, 0),
  // Complication of Procedure (retired code for SCT 116224001)
  entry("SNM3", "DD-60002", "TEXT", X, 0),
  // Anesthesia Start Time (retired code for SCT 398325003)
  entry(
    "SNM3",
    "DF-0068E",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Anesthesia Finish Time (retired code for SCT 398164008)
  entry(
    "SNM3",
    "DF-0070B",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // ECG Finding (retired code for SCT 271921002) (X/D)
  entry("SNM3", "F-00033", "TEXT", D, 0),
  // Social History (retired code for SCT 160476009)
  entry("SNM3", "F-02455", "TEXT", X, 0),
  // Finding Site (retired code for SCT 363698007)
  entry("SNM3", "G-C0E3", "TEXT", D, 0),
  // Stop DateTime (retired code for SCT 397898000)
  entry(
    "SNM3",
    "G-D320",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Start DateTime (retired code for SCT 398201009)
  entry(
    "SNM3",
    "G-D321",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Administration of radiopharmaceutical (retired code for SCT 440252007)
  entry("SNM3", "PA-30029", "TEXT", D, 0),
  // Clinical Report (retired code for SCT 371524004)
  entry("SNM3", "R-42B89", "COMPOSITE", X, RETAIN_UIDS),
  // Clinical Report (retired code for SCT 371524004)
  entry("SNM3", "R-42B89", "TEXT", X, 0),
  // Complication of Procedure (retired code for SCT 116224001)
  entry("SRT", "DD-60002", "TEXT", X, 0),
  // Anesthesia Start Time (retired code for SCT 398325003)
  entry(
    "SRT",
    "DF-0068E",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Anesthesia Finish Time (retired code for SCT 398164008)
  entry(
    "SRT",
    "DF-0070B",
    "DATETIME",
    X,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // ECG Finding (retired code for SCT 271921002) (X/D)
  entry("SRT", "F-00033", "TEXT", D, 0),
  // Social History (retired code for SCT 160476009)
  entry("SRT", "F-02455", "TEXT", X, 0),
  // Finding Site (retired code for SCT 363698007)
  entry("SRT", "G-C0E3", "TEXT", D, 0),
  // Stop DateTime (retired code for SCT 397898000)
  entry(
    "SRT",
    "G-D320",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Start DateTime (retired code for SCT 398201009)
  entry(
    "SRT",
    "G-D321",
    "DATETIME",
    D,
    RETAIN_LONGITUDINAL_TEMPORAL_INFORMATION,
  ),
  // Administration of radiopharmaceutical (retired code for SCT 440252007)
  entry("SRT", "PA-30029", "TEXT", D, 0),
  // Clinical Report (retired code for SCT 371524004)
  entry("SRT", "R-42B89", "COMPOSITE", X, RETAIN_UIDS),
  // Clinical Report (retired code for SCT 371524004)
  entry("SRT", "R-42B89", "TEXT", X, 0),
  // Time Point (X/D)
  entry("UMLS", "C2348792", "TEXT", D, 0),
];
