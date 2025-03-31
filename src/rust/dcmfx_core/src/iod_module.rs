use crate::{DataElementTag, DataError, DataSet};

/// Defines an IOD module, which is a collection of typed data elements that can
/// be created from a [`DataSet`].
///
pub trait IodModule {
  /// The tags of the data elements that are used to construct the IOD module.
  ///
  fn iod_module_data_element_tags() -> &'static [DataElementTag];

  /// Creates the IOD module from a data set. The tags of the data elements used
  /// by this function must be returned by
  /// [`IodModule::iod_module_data_element_tags()`].
  ///
  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError>
  where
    Self: Sized;
}
