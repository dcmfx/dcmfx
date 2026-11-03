/// The action to take on a data element when applying the Basic Application
/// Level Confidentiality Profile. These correspond to the action codes defined
/// in Table E.1-1a of DICOM PS3.15.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
  /// The data element is kept unchanged. For sequences, the profile is applied
  /// recursively to the data set of each item. This corresponds to the `K`
  /// action code.
  ///
  Keep,

  /// The data element is removed. This corresponds to the `X` action code.
  ///
  Remove,

  /// The data element's value is replaced with a zero length value of the same
  /// VR. This corresponds to the `Z` action code.
  ///
  ReplaceWithZeroLength,

  /// The data element's value is replaced with a non-zero length dummy value
  /// that is consistent with its VR. This corresponds to the `D` action code.
  ///
  ReplaceWithDummy,

  /// The data element's UID value is replaced with a new UID. The replacement
  /// is consistent, i.e. the same original UID always produces the same
  /// replacement UID, so that references between instances are maintained.
  /// This corresponds to the `U` action code.
  ///
  ReplaceUid,

  /// The data element's value is replaced with a value of similar meaning that
  /// is known not to contain identifying information. This corresponds to the
  /// `C` action code, and is only used when an option that cleans rather than
  /// removes is active:
  ///
  /// - Under the Retain Longitudinal Temporal Information With Modified Dates
  ///   Option, dates and times have their dates shifted by a consistent number
  ///   of days.
  /// - Under the Clean Structured Content Option, sequences of content items
  ///   are retained and have their content items cleaned, i.e. removed or
  ///   replaced with dummy values according to their concept name.
  ///
  Clean,
}
