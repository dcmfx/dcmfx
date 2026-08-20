#[cfg(not(feature = "std"))]
use alloc::format;

use dcmfx_core::{
  DataElementTag, DataError, DataSet, DataSetPath, IodModule,
  ValueRepresentation, dictionary,
};

/// The attributes of the Multi-frame Module, which describe a Multi-frame pixel
/// data image.
///
/// Ref: PS3.3 C.7.6.6.
///
#[derive(Clone, Debug, PartialEq)]
pub struct MultiFrameModule {
  pub number_of_frames: Option<usize>,
  pub frame_increment_pointer: Option<DataElementTag>,
  pub stereo_pairs_present: Option<bool>,
  pub encapsulated_pixel_data_value_total_length: Option<usize>,
}

impl IodModule for MultiFrameModule {
  fn is_iod_module_data_element(
    tag: DataElementTag,
    _vr: ValueRepresentation,
    _length: Option<u32>,
    path: &DataSetPath,
  ) -> bool {
    if !path.is_root() {
      return false;
    }

    tag == dictionary::NUMBER_OF_FRAMES.tag
      || tag == dictionary::FRAME_INCREMENT_POINTER.tag
      || tag == dictionary::STEREO_PAIRS_PRESENT.tag
      || tag == dictionary::ENCAPSULATED_PIXEL_DATA_VALUE_TOTAL_LENGTH.tag
  }

  fn iod_module_highest_tag() -> DataElementTag {
    dictionary::ENCAPSULATED_PIXEL_DATA_VALUE_TOTAL_LENGTH.tag
  }

  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let number_of_frames =
      data_set.get_optional_int::<usize>(dictionary::NUMBER_OF_FRAMES.tag)?;

    let frame_increment_pointer = data_set
      .get_optional_attribute_tag(dictionary::FRAME_INCREMENT_POINTER.tag)?;

    let tag = dictionary::STEREO_PAIRS_PRESENT.tag;
    let stereo_pairs_present = match data_set.get_optional_string(tag)? {
      None => None,
      Some("YES") => Some(true),
      Some("NO") => Some(false),
      Some(value) => {
        return Err(
          DataError::new_value_invalid(format!("Invalid enum value '{value}'"))
            .with_path(&DataSetPath::new_with_data_element(tag)),
        );
      }
    };

    let encapsulated_pixel_data_value_total_length = data_set
      .get_optional_int::<usize>(
        dictionary::ENCAPSULATED_PIXEL_DATA_VALUE_TOTAL_LENGTH.tag,
      )?;

    Ok(Self {
      number_of_frames,
      frame_increment_pointer,
      stereo_pairs_present,
      encapsulated_pixel_data_value_total_length,
    })
  }
}
