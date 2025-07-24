#[cfg(not(feature = "std"))]
use alloc::string::{String, ToString};

use crate::{
  DataElementTag, DataError, DataSet, DataSetPath, IodModule,
  ValueRepresentation, dictionary,
};

/// The attributes of the SOP Common Module, which describe a DICOM SOP
/// instance.
///
/// Note: not all SOP Common Module attributes are included in this struct.
///
/// Ref: PS3.3 C.12-1.
///
#[derive(Clone, Debug, PartialEq)]
pub struct SopCommonModule {
  pub sop_class_uid: String,
  pub sop_instance_uid: String,
  pub instance_number: Option<i32>,
}

impl IodModule for SopCommonModule {
  fn is_iod_module_data_element(
    tag: DataElementTag,
    _vr: ValueRepresentation,
    _length: Option<u32>,
    path: &DataSetPath,
  ) -> bool {
    if !path.is_root() {
      return false;
    }

    tag == dictionary::SOP_CLASS_UID.tag
      || tag == dictionary::SOP_INSTANCE_UID.tag
      || tag == dictionary::INSTANCE_NUMBER.tag
  }

  fn iod_module_highest_tag() -> DataElementTag {
    dictionary::INSTANCE_NUMBER.tag
  }

  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let sop_class_uid = data_set
      .get_string(dictionary::SOP_CLASS_UID.tag)?
      .to_string();

    let sop_instance_uid = data_set
      .get_string(dictionary::SOP_INSTANCE_UID.tag)?
      .to_string();

    let instance_number = if data_set.has(dictionary::INSTANCE_NUMBER.tag) {
      Some(data_set.get_int::<i32>(dictionary::INSTANCE_NUMBER.tag)?)
    } else {
      None
    };

    Ok(Self {
      sop_class_uid,
      sop_instance_uid,
      instance_number,
    })
  }
}
