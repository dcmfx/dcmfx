#[cfg(not(feature = "std"))]
use alloc::{format, string::String, string::ToString, vec, vec::Vec};

use dcmfx_core::{
  DataElementTag, DataError, DataSet, DataSetPath, IodModule, RcByteSlice,
  ValueRepresentation, dictionary,
};

/// The attributes of the Overlay Plane Module, which describe a set of overlays
/// where each overlay is a bitmap that can be rendered on top of pixel data.
/// Overlays are used to defined ROIs and other graphics. They are often able
/// to be toggled on and off when viewing pixel data.
///
/// The maximum number of overlays is 16.
///
/// Ref: PS3.3 C.9.
///
#[derive(Clone, Debug, PartialEq)]
pub struct OverlayPlaneModule {
  overlays: Vec<OverlayPlane>,
}

impl IodModule for OverlayPlaneModule {
  fn is_iod_module_data_element(
    tag: DataElementTag,
    _vr: ValueRepresentation,
    _length: Option<u32>,
    path: &DataSetPath,
  ) -> bool {
    if !path.is_root() {
      return false;
    }

    if tag.group < 0x6000 || tag.group > 0x601E || tag.group % 2 != 0 {
      return false;
    }

    tag.element == 0x0010
      || tag.element == 0x0011
      || tag.element == 0x0040
      || tag.element == 0x0050
      || tag.element == 0x0100
      || tag.element == 0x3000
      || tag.element == 0x0022
      || tag.element == 0x0045
      || tag.element == 0x1500
      || tag.element == 0x1301
      || tag.element == 0x1302
      || tag.element == 0x1303
      || tag.element == 0x0015
      || tag.element == 0x0051
  }

  fn iod_module_highest_tag() -> DataElementTag {
    DataElementTag::new(0x601E, 0x3000)
  }

  fn from_data_set(
    data_set: &DataSet,
  ) -> Result<OverlayPlaneModule, DataError> {
    let mut overlays = vec![];

    for i in 0..16 {
      let tag_group = 0x6000 + i * 2;

      if !data_set.has(dictionary::OVERLAY_DATA.tag.with_group(tag_group)) {
        continue;
      }

      overlays.push(OverlayPlane::from_data_set(data_set, tag_group)?);
    }

    Ok(OverlayPlaneModule { overlays })
  }
}

impl OverlayPlaneModule {
  /// Returns whether the internal list of overlays is empty.
  ///
  pub fn is_empty(&self) -> bool {
    self.overlays.is_empty()
  }

  /// Returns an iterator over the individual overlays.
  ///
  pub fn iter(&self) -> core::slice::Iter<'_, OverlayPlane> {
    self.overlays.iter()
  }

  /// Renders all overlays onto an RGB image using the default overlay colors
  /// specified by [`Self::DEFAULT_COLORS`]. The RGB image must be 8-bit or
  /// 16-bit.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn render_to_rgb_image(
    &self,
    rgb_image: &mut image::DynamicImage,
    frame_index: usize,
  ) -> Result<(), ()> {
    self.render_to_rgb_image_with_colors(
      rgb_image,
      frame_index,
      &Self::DEFAULT_COLORS,
    )
  }

  /// Renders all overlays onto an RGB image using the specified colors. The RGB
  /// image must be 8-bit or 16-bit.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn render_to_rgb_image_with_colors(
    &self,
    rgb_image: &mut image::DynamicImage,
    frame_index: usize,
    colors: &[image::Rgb<u8>; 16],
  ) -> Result<(), ()> {
    for (i, overlay) in self.iter().enumerate() {
      match rgb_image {
        image::DynamicImage::ImageRgb8(rgb_image) => overlay
          .render_to_rgb_image(
            rgb_image.width(),
            rgb_image.height(),
            rgb_image.as_mut(),
            frame_index,
            colors[i],
          ),

        image::DynamicImage::ImageRgb16(rgb_image) => overlay
          .render_to_rgb_image(
            rgb_image.width(),
            rgb_image.height(),
            rgb_image.as_mut(),
            frame_index,
            image::Rgb::<u16>([
              colors[i].0[0] as u16 * 257,
              colors[i].0[1] as u16 * 257,
              colors[i].0[2] as u16 * 257,
            ]),
          ),

        _ => return Err(()),
      }
    }

    Ok(())
  }

  /// The default set of colors used to render overlays. The maximum number of
  /// overlays allowed is 16.
  ///
  pub const DEFAULT_COLORS: [image::Rgb<u8>; 16] = [
    image::Rgb([255, 255, 255]), // White
    image::Rgb([0, 191, 255]),   // Electric blue
    image::Rgb([50, 205, 50]),   // Lime green
    image::Rgb([255, 215, 0]),   // Sunflower yellow
    image::Rgb([178, 34, 34]),   // Crimson red
    image::Rgb([255, 105, 180]), // Hot pink
    image::Rgb([255, 140, 0]),   // Tangerine orange
    image::Rgb([0, 128, 0]),     // Emerald green
    image::Rgb([102, 0, 204]),   // Royal purple
    image::Rgb([64, 224, 208]),  // Turquoise
    image::Rgb([255, 0, 255]),   // Magenta
    image::Rgb([0, 71, 171]),    // Cobalt blue
    image::Rgb([255, 255, 102]), // Canary yellow
    image::Rgb([148, 0, 211]),   // Violet
    image::Rgb([0, 128, 128]),   // Teal
    image::Rgb([255, 127, 80]),  // Coral
  ];
}

/// Definition for a single DICOM overlay plane.
///
/// Ref: PS3.3 C.9.
///
#[derive(Clone, Debug, PartialEq)]
pub struct OverlayPlane {
  tag_group: u16,

  overlay_rows: u16,
  overlay_columns: u16,
  overlay_type: OverlayType,
  overlay_origin: [i32; 2],
  overlay_data: RcByteSlice,
  overlay_description: Option<String>,
  overlay_subtype: Option<OverlaySubtype>,
  overlay_label: Option<String>,
  roi_area: Option<u64>,
  roi_mean: Option<f64>,
  roi_standard_deviation: Option<f64>,
  number_of_frames_in_overlay: usize,
  image_frame_origin: usize,
}

impl OverlayPlane {
  /// Creates a new [`OverlayPlane`] from the relevant data elements in the
  /// given data set.
  ///
  pub fn from_data_set(
    data_set: &DataSet,
    tag_group: u16,
  ) -> Result<Self, DataError> {
    let rows = data_set.get_int::<u16>(DataElementTag::new(
      tag_group,
      dictionary::OVERLAY_ROWS.tag.element,
    ))?;

    let columns = data_set.get_int::<u16>(DataElementTag::new(
      tag_group,
      dictionary::OVERLAY_COLUMNS.tag.element,
    ))?;

    let overlay_type = OverlayType::from_data_set(data_set, tag_group)?;

    let origin_tag =
      DataElementTag::new(tag_group, dictionary::OVERLAY_ORIGIN.tag.element);
    let origin_value = data_set.get_ints::<i16>(origin_tag)?;
    if origin_value.len() != 2 {
      return Err(
        DataError::new_value_length_invalid(
          ValueRepresentation::SignedShort,
          2,
          "Overlay Origin does not have exactly two values".to_string(),
        )
        .with_path(&DataSetPath::new_with_data_element(origin_tag)),
      );
    }

    let data_tag = dictionary::OVERLAY_DATA.tag.with_group(tag_group);
    let data = data_set
      .get_value_vr_bytes(
        data_tag,
        &[
          ValueRepresentation::OtherByteString,
          ValueRepresentation::OtherWordString,
        ],
      )?
      .clone();

    let description_tag =
      dictionary::OVERLAY_DESCRIPTION.tag.with_group(tag_group);
    let description = if data_set.has(description_tag) {
      Some(data_set.get_string(description_tag)?.to_string())
    } else {
      None
    };

    let subtype_tag = dictionary::OVERLAY_SUBTYPE.tag.with_group(tag_group);
    let subtype = if data_set.has(subtype_tag) {
      Some(OverlaySubtype::from_data_set(data_set, tag_group)?)
    } else {
      None
    };

    let label_tag = dictionary::OVERLAY_LABEL.tag.with_group(tag_group);
    let label = if data_set.has(label_tag) {
      Some(data_set.get_string(label_tag)?.to_string())
    } else {
      None
    };

    let roi_area_tag = dictionary::ROI_AREA.tag.with_group(tag_group);
    let roi_area = if data_set.has(roi_area_tag) {
      Some(data_set.get_int::<u64>(roi_area_tag)?)
    } else {
      None
    };

    let roi_mean_tag = dictionary::ROI_MEAN.tag.with_group(tag_group);
    let roi_mean = if data_set.has(roi_mean_tag) {
      Some(data_set.get_float(roi_mean_tag)?)
    } else {
      None
    };

    let roi_standard_deviation_tag =
      dictionary::ROI_STANDARD_DEVIATION.tag.with_group(tag_group);
    let roi_standard_deviation = if data_set.has(roi_standard_deviation_tag) {
      Some(data_set.get_float(roi_standard_deviation_tag)?)
    } else {
      None
    };

    let number_of_frames_in_overlay = data_set.get_int_with_default::<usize>(
      dictionary::NUMBER_OF_FRAMES_IN_OVERLAY
        .tag
        .with_group(tag_group),
      1,
    )?;

    let image_frame_origin = data_set.get_int_with_default::<usize>(
      dictionary::IMAGE_FRAME_ORIGIN.tag.with_group(tag_group),
      1,
    )?;

    let expected_data_length =
      (usize::from(rows) * usize::from(columns) * number_of_frames_in_overlay)
        .div_ceil(8);
    if data.len() != expected_data_length {
      return Err(
        DataError::new_value_length_invalid(
          ValueRepresentation::SignedShort,
          data.len() as u64,
          format!("Overlay Data should have length {}", expected_data_length,),
        )
        .with_path(&DataSetPath::new_with_data_element(data_tag)),
      );
    }

    Ok(Self {
      tag_group,

      overlay_rows: rows,
      overlay_columns: columns,
      overlay_type,
      overlay_origin: [i32::from(origin_value[0]), i32::from(origin_value[1])],
      overlay_data: data,
      overlay_description: description,
      overlay_subtype: subtype,
      overlay_label: label,
      roi_area,
      roi_mean,
      roi_standard_deviation,
      number_of_frames_in_overlay,
      image_frame_origin,
    })
  }

  /// Renders this overlay onto an RGB image using the specified color.
  ///
  pub fn render_to_rgb_image<T>(
    &self,
    width: u32,
    height: u32,
    rgb_data: &mut [T],
    frame_index: usize,
    color: image::Rgb<T>,
  ) where
    T: Copy + Into<f64> + num_traits::FromPrimitive,
  {
    // Check whether there is overlay data for this frame based on its index
    if (frame_index + 1) < self.image_frame_origin
      || (frame_index + 1)
        >= self.image_frame_origin + self.number_of_frames_in_overlay
    {
      return;
    }

    // Get the data for this frame
    let overlay_data_offset = usize::from(self.overlay_rows)
      * usize::from(self.overlay_columns)
      * ((frame_index + 1) - self.image_frame_origin);

    // Alphas that apply some blurring over a 3x3 area where the overlay is
    // present to make it have a less blocky appearance
    let alphas = [
      1.0 / 8.0,
      1.0 / 4.0,
      1.0 / 8.0,
      1.0 / 4.0,
      1.0,
      1.0 / 4.0,
      1.0 / 8.0,
      1.0 / 4.0,
      1.0 / 8.0,
    ];

    for y in 0..self.overlay_rows {
      let pt_y = self.overlay_origin[1] + i32::from(y) - 1;
      if pt_y < 0 || pt_y as u32 >= height {
        continue;
      }

      for x in 0..self.overlay_columns {
        let pt_x = self.overlay_origin[0] + i32::from(x) - 1;
        if pt_x < 0 || pt_x as u32 >= width {
          continue;
        }

        // Check whether this pixel in the overlay bitmap is set
        let data_offset = overlay_data_offset
          + usize::from(y) * usize::from(self.overlay_columns)
          + usize::from(x);
        let byte = self.overlay_data[data_offset / 8];
        if (byte >> (data_offset % 8)) & 1 == 0 {
          continue;
        }

        // This pixel is set in the overlay so draw it into the RGB image. Use
        // a 3x3 kernel to achieve a smoothed result.
        for (i, alpha) in alphas.iter().enumerate() {
          let pixel_x = pt_x + i as i32 % 3 - 1;
          let pixel_y = pt_y + i as i32 / 3 - 1;

          if pixel_x > 0
            && pixel_x < width as i32
            && pixel_y > 0
            && pixel_y < height as i32
          {
            let offset =
              (pixel_y as usize * width as usize + pixel_x as usize) * 3;
            let rgb = &mut rgb_data[offset..(offset + 3)];

            for (i, c) in rgb.iter_mut().enumerate() {
              let a = Into::<f64>::into(color.0[i]);
              let b = Into::<f64>::into(*c);

              *c = T::from_f64(a * alpha + b * (1.0 - alpha)).unwrap();
            }
          }
        }
      }
    }
  }
}

/// Specifies the type of an overlay.
///
/// Ref: PS3.3 C.9.2.1.1
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OverlayType {
  /// The overlay describes graphics.
  Graphics,

  /// The overlay describes a region of interest (ROI).
  Roi,
}

impl OverlayType {
  /// Creates a new [`OverlayType`] from the *'(60gg,0040) Overlay Type'* data
  /// element in the given data set.
  ///
  pub fn from_data_set(
    data_set: &DataSet,
    group: u16,
  ) -> Result<Self, DataError> {
    let tag = DataElementTag::new(group, dictionary::OVERLAY_TYPE.tag.element);

    match data_set.get_string(tag)? {
      "G" => Ok(Self::Graphics),
      "R" => Ok(Self::Roi),
      value => Err(
        DataError::new_value_invalid(format!(
          "Overlay type value of '{}' is invalid",
          value
        ))
        .with_path(&DataSetPath::new_with_data_element(tag)),
      ),
    }
  }
}

/// Specifies the subtype of an overlay.
///
/// Ref: PS3.3 C.9.2.1.3
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OverlaySubtype {
  /// User created graphic annotation (e.g., operator).
  User,

  /// Machine or algorithm generated graphic annotation, such as output of a
  /// Computer Assisted Diagnosis algorithm.
  Automated,
}

impl OverlaySubtype {
  /// Creates a new [`OverlaySubtype`] from the *'(60gg,0045) Overlay Subtype'*
  /// data element in the given data set.
  ///
  pub fn from_data_set(
    data_set: &DataSet,
    group: u16,
  ) -> Result<Self, DataError> {
    let tag =
      DataElementTag::new(group, dictionary::OVERLAY_SUBTYPE.tag.element);

    match data_set.get_string(tag)? {
      "USER" => Ok(Self::User),
      "AUTOMATED" => Ok(Self::Automated),
      value => Err(
        DataError::new_value_invalid(format!(
          "Overlay subtype value of '{}' is invalid",
          value
        ))
        .with_path(&DataSetPath::new_with_data_element(tag)),
      ),
    }
  }
}
