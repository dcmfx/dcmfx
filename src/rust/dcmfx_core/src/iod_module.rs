use crate::{
  DataElementTag, DataError, DataSet, DataSetPath, ValueRepresentation,
};

/// Defines an IOD module, which is a collection of typed data elements that can
/// be created from a [`DataSet`] using [`IodModule::from_data_set()`].
///
/// IOD modules should specify the data elements needed in their construction
/// using [`IodModule::is_iod_module_data_element()`] and
/// [`IodModule::iod_module_highest_tag()`], which allows them to be created
/// from a stream of monotonically increasing DICOM data elements.
///
pub trait IodModule {
  /// Returns whether the specified data element is needed for the construction
  /// of this IOD.
  ///
  fn is_iod_module_data_element(
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: Option<u32>,
    path: &DataSetPath,
  ) -> bool;

  /// Returns the highest data element tag that can return true from
  /// [`IodModule::is_iod_module_data_element()`]
  ///
  fn iod_module_highest_tag() -> DataElementTag;

  /// Creates the IOD module from a data set. The tags of the data elements used
  /// by this function must be consistent with the behavior of
  /// [`IodModule::is_iod_module_data_element()`] and
  /// [`IodModule::iod_module_highest_tag()`].
  ///
  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError>
  where
    Self: Sized;
}
