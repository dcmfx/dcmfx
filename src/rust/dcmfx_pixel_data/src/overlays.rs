#[cfg(feature = "std")]
use std::rc::Rc;

#[cfg(not(feature = "std"))]
use alloc::{
  boxed::Box, format, rc::Rc, string::String, string::ToString, vec, vec::Vec,
};

use dcmfx_core::{
  DataElementTag, DataError, DataSet, DataSetPath, ValueRepresentation,
  dictionary,
};
use dcmfx_p10::PredicateFunction;
use image::{Rgb, RgbImage};

/// Defines a set of overlays where each overlay is a bitmap that can be
/// rendered on top of pixel data. Overlays are used to defined ROIs and other
/// graphics. They are often able to be toggled on and off when viewing.
///
/// The maximum number of overlays allowed is 16.
///
/// Ref: PS3.3 C.9.
///
#[derive(Clone, Debug, PartialEq)]
pub struct Overlays(Vec<Overlay>);

impl Overlays {
  /// Returns a filter predicate that returns true for data elements that are
  /// relevant to overlays.
  ///
  pub fn filter_predicate() -> Box<PredicateFunction> {
    Box::new(|tag, _vr, _length, _location| {
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
    })
  }

  /// The highest data element tag value that will successfully pass though the
  /// predicate returned by [`Self::filter_predicate()`].
  ///
  pub const HIGHEST_TAG: DataElementTag = DataElementTag {
    group: 0x601E,
    element: 0x3000,
  };

  /// Creates a new [`Overlays`] instance from the relevant data elements in the
  /// given data set.
  ///
  pub fn from_data_set(data_set: &DataSet) -> Result<Overlays, DataError> {
    let mut overlays = vec![];

    for i in 0..16 {
      let tag_group = 0x6000 + i * 2;

      if !data_set.has(DataElementTag::new(
        tag_group,
        dictionary::OVERLAY_DATA.tag.element,
      )) {
        continue;
      }

      overlays.push(Overlay::from_data_set(data_set, tag_group)?);
    }

    Ok(Overlays(overlays))
  }

  /// Renders all overlays onto an [`RgbImage`] using the default overlay
  /// colors specified by [`Self::DEFAULT_COLORS`].
  ///
  pub fn render_to_rgb_image(
    &self,
    rgb_image: &mut RgbImage,
    frame_index: usize,
  ) {
    self.render_to_rgb_image_using_colors(
      rgb_image,
      frame_index,
      &Self::DEFAULT_COLORS,
    );
  }

  /// Renders all overlays onto an [`RgbImage`] using the specified colors.
  ///
  pub fn render_to_rgb_image_using_colors(
    &self,
    rgb_image: &mut RgbImage,
    frame_index: usize,
    colors: &[Rgb<u8>; 16],
  ) {
    for (i, overlay) in self.0.iter().enumerate() {
      overlay.render_to_rgb_image(rgb_image, frame_index, colors[i]);
    }
  }

  /// The default set of colors used to render overlays. The maximum number of
  /// overlays allowed is 16.
  ///
  pub const DEFAULT_COLORS: [Rgb<u8>; 16] = [
    Rgb([255, 255, 255]), // White
    Rgb([0, 191, 255]),   // Electric blue
    Rgb([50, 205, 50]),   // Lime green
    Rgb([255, 215, 0]),   // Sunflower yellow
    Rgb([178, 34, 34]),   // Crimson red
    Rgb([255, 105, 180]), // Hot pink
    Rgb([255, 140, 0]),   // Tangerine orange
    Rgb([0, 128, 0]),     // Emerald green
    Rgb([102, 0, 204]),   // Royal purple
    Rgb([64, 224, 208]),  // Turquoise
    Rgb([255, 0, 255]),   // Magenta
    Rgb([0, 71, 171]),    // Cobalt blue
    Rgb([255, 255, 102]), // Canary yellow
    Rgb([148, 0, 211]),   // Violet
    Rgb([0, 128, 128]),   // Teal
    Rgb([255, 127, 80]),  // Coral
  ];
}

/// Definition for a single DICOM overlay.
///
/// Ref: PS3.3 C.9.
///
#[derive(Clone, Debug, PartialEq)]
pub struct Overlay {
  tag_group: u16,

  rows: u16,
  columns: u16,
  overlay_type: OverlayType,
  origin: [i32; 2],
  data: Rc<Vec<u8>>,
  description: Option<String>,
  subtype: Option<OverlaySubtype>,
  label: Option<String>,
  roi_area: Option<u64>,
  roi_mean: Option<f64>,
  roi_standard_deviation: Option<f64>,
  number_of_frames_in_overlay: usize,
  image_frame_origin: usize,
}

impl Overlay {
  /// Creates a new `Overlay` from the relevant data elements in the given data
  /// set.
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

    let data_tag =
      DataElementTag::new(tag_group, dictionary::OVERLAY_DATA.tag.element);
    let data = data_set
      .get_value_vr_bytes(
        data_tag,
        &[
          ValueRepresentation::OtherByteString,
          ValueRepresentation::OtherWordString,
        ],
      )?
      .clone();

    let description_tag = DataElementTag::new(
      tag_group,
      dictionary::OVERLAY_DESCRIPTION.tag.element,
    );
    let description = if data_set.has(description_tag) {
      Some(data_set.get_string(description_tag)?.to_string())
    } else {
      None
    };

    let subtype_tag =
      DataElementTag::new(tag_group, dictionary::OVERLAY_SUBTYPE.tag.element);
    let subtype = if data_set.has(subtype_tag) {
      Some(OverlaySubtype::from_data_set(data_set, tag_group)?)
    } else {
      None
    };

    let label_tag =
      DataElementTag::new(tag_group, dictionary::OVERLAY_LABEL.tag.element);
    let label = if data_set.has(label_tag) {
      Some(data_set.get_string(label_tag)?.to_string())
    } else {
      None
    };

    let roi_area_tag =
      DataElementTag::new(tag_group, dictionary::ROI_AREA.tag.element);
    let roi_area = if data_set.has(roi_area_tag) {
      Some(data_set.get_int::<u64>(roi_area_tag)?)
    } else {
      None
    };

    let roi_mean_tag =
      DataElementTag::new(tag_group, dictionary::ROI_MEAN.tag.element);
    let roi_mean = if data_set.has(roi_mean_tag) {
      Some(data_set.get_float(roi_mean_tag)?)
    } else {
      None
    };

    let roi_standard_deviation_tag = DataElementTag::new(
      tag_group,
      dictionary::ROI_STANDARD_DEVIATION.tag.element,
    );
    let roi_standard_deviation = if data_set.has(roi_standard_deviation_tag) {
      Some(data_set.get_float(roi_standard_deviation_tag)?)
    } else {
      None
    };

    let number_of_frames_in_overlay_tag = DataElementTag::new(
      tag_group,
      dictionary::NUMBER_OF_FRAMES_IN_OVERLAY.tag.element,
    );
    let number_of_frames_in_overlay =
      if data_set.has(number_of_frames_in_overlay_tag) {
        data_set.get_int::<usize>(number_of_frames_in_overlay_tag)?
      } else {
        1
      };

    let image_frame_origin_tag = DataElementTag::new(
      tag_group,
      dictionary::IMAGE_FRAME_ORIGIN.tag.element,
    );
    let image_frame_origin = if data_set.has(image_frame_origin_tag) {
      data_set.get_int::<usize>(image_frame_origin_tag)?
    } else {
      1
    };

    let expected_data_length =
      (rows as usize * columns as usize * number_of_frames_in_overlay + 7) / 8;
    if data.len() != expected_data_length {
      return Err(
        DataError::new_value_length_invalid(
          ValueRepresentation::SignedShort,
          data.len(),
          format!("Overlay Data should have length {}", expected_data_length,),
        )
        .with_path(&DataSetPath::new_with_data_element(data_tag)),
      );
    }

    Ok(Self {
      tag_group,

      rows,
      columns,
      overlay_type,
      origin: [origin_value[0] as i32, origin_value[1] as i32],
      data,
      description,
      subtype,
      label,
      roi_area,
      roi_mean,
      roi_standard_deviation,
      number_of_frames_in_overlay,
      image_frame_origin,
    })
  }

  /// Renders this overlay onto an [`RgbImage`] using the specified color.
  ///
  pub fn render_to_rgb_image(
    &self,
    rgb_image: &mut RgbImage,
    frame_index: usize,
    color: Rgb<u8>,
  ) {
    // Check whether there is overlay data for this frame based on its index
    if (frame_index + 1) < self.image_frame_origin
      || (frame_index + 1)
        >= self.image_frame_origin + self.number_of_frames_in_overlay
    {
      return;
    }

    // Get the data for this frame
    let overlay_data_offset = self.rows as usize
      * self.columns as usize
      * ((frame_index + 1) - self.image_frame_origin);

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

    for y in 0..self.rows {
      let pt_y = self.origin[1] + y as i32 - 1;
      if pt_y < 0 || pt_y as u32 >= rgb_image.height() {
        continue;
      }

      for x in 0..self.columns {
        let pt_x = self.origin[0] + x as i32 - 1;
        if pt_x < 0 || pt_x as u32 >= rgb_image.width() {
          continue;
        }

        // Check whether this pixel in the overlay bitmap is set
        let data_offset =
          overlay_data_offset + y as usize * self.columns as usize + x as usize;
        let byte = self.data[data_offset / 8];
        if (byte >> (data_offset % 8)) & 1 == 0 {
          continue;
        }

        // This pixel is set in the overlay so draw it into the RGB image. Use
        // a 3x3 kernel to achieve a smoothed result.
        for (i, alpha) in alphas.iter().enumerate() {
          let pixel_x = pt_x + i as i32 % 3 - 1;
          let pixel_y = pt_y + i as i32 / 3 - 1;

          if pixel_x > 0
            && pixel_x < rgb_image.width() as i32
            && pixel_y > 0
            && pixel_y < rgb_image.height() as i32
          {
            let rgb = rgb_image.get_pixel(pixel_x as u32, pixel_y as u32);

            rgb_image.put_pixel(
              pixel_x as u32,
              pixel_y as u32,
              Rgb([
                (color.0[0] as f64 * alpha + rgb.0[0] as f64 * (1.0 - alpha))
                  as u8,
                (color.0[1] as f64 * alpha + rgb.0[1] as f64 * (1.0 - alpha))
                  as u8,
                (color.0[2] as f64 * alpha + rgb.0[2] as f64 * (1.0 - alpha))
                  as u8,
              ]),
            );
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
