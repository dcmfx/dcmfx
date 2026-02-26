use dcmfx_core::{
  DataElementTag, DataError, DataSet, DataSetPath, IodModule,
  ValueRepresentation, dictionary,
};

/// The attributes of the Image Plane Module, which define the transmitted pixel
/// array of a two dimensional image plane in a three dimensional space.
///
/// Ref: PS3.3 C.7.6.2.
///
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePlaneModule {
  pub pixel_spacing: [f32; 2],
  pub image_orientation_patient: [f32; 6],
  pub image_position_patient: [f32; 3],
  pub slice_thickness: Option<f32>,
  pub spacing_between_slices: Option<f32>,
  pub slice_location: Option<f32>,
}

impl IodModule for ImagePlaneModule {
  fn is_iod_module_data_element(
    tag: DataElementTag,
    _vr: ValueRepresentation,
    _length: Option<u32>,
    path: &DataSetPath,
  ) -> bool {
    if !path.is_root() {
      return false;
    }

    Self::TAGS.contains(&tag)
  }

  fn iod_module_highest_tag() -> DataElementTag {
    dictionary::PIXEL_SPACING.tag
  }

  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let tag = dictionary::PIXEL_SPACING.tag;
    let pixel_spacing = match data_set.get_floats(tag)?.as_slice() {
      [a, b] => [*a as f32, *b as f32],
      _ => {
        return Err(
          DataError::new_value_invalid(
            "Pixel spacing must have exactly two values".into(),
          )
          .with_path(&DataSetPath::new_with_data_element(tag)),
        );
      }
    };

    let tag = dictionary::IMAGE_ORIENTATION_PATIENT.tag;
    let image_orientation_patient = match data_set.get_floats(tag)?.as_slice() {
      [a, b, c, d, e, f] => [
        *a as f32, *b as f32, *c as f32, *d as f32, *e as f32, *f as f32,
      ],
      _ => {
        return Err(
          DataError::new_value_invalid(
            "Image orientation must have exactly six values".into(),
          )
          .with_path(&DataSetPath::new_with_data_element(tag)),
        );
      }
    };

    let tag = dictionary::IMAGE_POSITION_PATIENT.tag;
    let image_position_patient = match data_set.get_floats(tag)?.as_slice() {
      [a, b, c] => [*a as f32, *b as f32, *c as f32],
      _ => {
        return Err(
          DataError::new_value_invalid(
            "Image position must have exactly three values".into(),
          )
          .with_path(&DataSetPath::new_with_data_element(tag)),
        );
      }
    };

    let tag = dictionary::SLICE_THICKNESS.tag;
    let slice_thickness = if data_set.has(tag) {
      Some(data_set.get_float(tag)? as f32)
    } else {
      None
    };

    let tag = dictionary::SPACING_BETWEEN_SLICES.tag;
    let spacing_between_slices = if data_set.has(tag) {
      Some(data_set.get_float(tag)? as f32)
    } else {
      None
    };

    let tag = dictionary::SLICE_LOCATION.tag;
    let slice_location = if data_set.has(tag) {
      Some(data_set.get_float(tag)? as f32)
    } else {
      None
    };

    Ok(Self {
      pixel_spacing,
      image_orientation_patient,
      image_position_patient,
      slice_thickness,
      spacing_between_slices,
      slice_location,
    })
  }
}

impl ImagePlaneModule {
  /// The data element tags used when reading [`ImagePlaneModule`].
  ///
  pub const TAGS: [DataElementTag; 6] = [
    dictionary::PIXEL_SPACING.tag,
    dictionary::IMAGE_ORIENTATION_PATIENT.tag,
    dictionary::IMAGE_POSITION_PATIENT.tag,
    dictionary::SLICE_THICKNESS.tag,
    dictionary::SPACING_BETWEEN_SLICES.tag,
    dictionary::SLICE_LOCATION.tag,
  ];
}
