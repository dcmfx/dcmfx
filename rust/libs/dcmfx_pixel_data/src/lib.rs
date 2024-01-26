use byteorder::ByteOrder;

use dcmfx_core::{
  registry, transfer_syntax, DataElementValue, DataError, DataSet,
  TransferSyntax, ValueRepresentation,
};

type Frame<'a> = Vec<&'a [u8]>;

/// Adds functions to [`DataSet`] for getting its raw pixel data.
///
pub trait DataSetPixelDataExtensions
where
  Self: Sized,
{
  fn get_pixel_data(
    &self,
  ) -> Result<(ValueRepresentation, Vec<Frame>), DataError>;
}

impl DataSetPixelDataExtensions for DataSet {
  /// Returns all frames of image data present in a data set. Each returned
  /// frame is made up of one or more fragments of binary data. This function
  /// handles both encapsulated and non-encapsulated pixel data, and requires
  /// that the '(7FE0,0010) Pixel Data' data element is present in the data set.
  ///
  /// The '(0028,0008) Number of Frames', '(7FE0,0001) Extended Offset Table',
  /// and '(7FE0,0002) Extended Offset Table Lengths' data elements are used
  /// when present and relevant.
  ///
  fn get_pixel_data(
    &self,
  ) -> Result<(ValueRepresentation, Vec<Frame>), DataError> {
    // Get the pixel data value
    let pixel_data = self.get_value(registry::PIXEL_DATA.tag)?;

    // Get the extended offset table value, if present
    let extended_offset_table = match parse_extended_offset_table(self) {
      Ok(table) => Ok(Some(table)),
      Err(data_error) => {
        if data_error.is_tag_not_present() {
          Ok(None)
        } else {
          Err(data_error)
        }
      }
    }?;

    // Get the number of frames value, if present
    let number_of_frames = self.get_int(registry::NUMBER_OF_FRAMES.tag).ok();

    if let Some(n) = number_of_frames {
      if n < 0 {
        return Err(DataError::new_value_invalid(format!(
          "Number of frames is invalid: {n}"
        )));
      }
    }

    let frames = do_get_pixel_data(
      pixel_data,
      number_of_frames.map(|n| n as usize),
      extended_offset_table,
    )?;

    Ok((pixel_data.value_representation(), frames))
  }
}

fn do_get_pixel_data(
  value: &DataElementValue,
  number_of_frames: Option<usize>,
  extended_offset_table: Option<ExtendedOffsetTable>,
) -> Result<Vec<Frame>, DataError> {
  let vr = value.value_representation();

  // Non-encapsulated OB or OW pixel data
  if let Ok(bytes) = value.bytes() {
    if vr != ValueRepresentation::OtherByteString
      && vr != ValueRepresentation::OtherWordString
    {
      return Err(DataError::new_value_not_present());
    }

    return match number_of_frames {
      None | Some(0) | Some(1) => Ok(vec![vec![bytes.as_slice()]]),

      Some(number_of_frames) => {
        let bytes_size = bytes.len();
        let frame_size = bytes_size / number_of_frames;

        // Check that the pixel data divides exactly into the number of frames.
        // If it doesn't then it's either due to an inconsistency in the pixel
        // data, or the pixel data for a single frame is not aligned on byte
        // boundaries, which is not supported by this library. The latter is
        // possible when the bits allocated value isn't a multiple of 8.
        if number_of_frames * frame_size == bytes_size {
          Ok(
            (0..number_of_frames)
              .map(|i| {
                vec![&bytes.as_slice()[i * frame_size..((i + 1) * frame_size)]]
              })
              .collect(),
          )
        } else {
          Err(DataError::new_value_invalid(format!(
            "Multi-frame pixel data of length {} does not divide evenly into \
             {} frames",
            bytes_size, number_of_frames
          )))
        }
      }
    };
  }

  if let Ok(items) = value.encapsulated_pixel_data() {
    if items.is_empty() {
      return Err(DataError::new_value_not_present());
    }

    // Encapsulated pixel data with an extended offset table present in the data
    // set. There should be no basic offset table, and the extended offset table
    // is used to define the frames.
    if let Some(extended_offset_table) = extended_offset_table {
      // The basic offset table must be empty when an extended offset table is
      // present
      if !items[0].is_empty() {
        return Err(DataError::new_value_invalid(
          "Encapsulated pixel data has both a basic offset table and an \
            extended offset table, but only one of these is allowed"
            .to_string(),
        ));
      }

      let frames = fragments_to_frames_using_extended_offset_table(
        items[1..].iter().map(|f| f.as_ref().as_slice()).collect(),
        &extended_offset_table,
      )?;

      return Ok(frames.iter().map(|i| vec![*i]).collect());
    }

    // Encapsulated pixel data with an empty basic offset table and a single
    // fragment. The sole fragment is treated as a single frame of pixel data.
    if items.len() == 2 && items[0].is_empty() {
      return Ok(vec![vec![&items[1]]]);
    }

    // Encapsulated pixel data with an empty basic offset table and multiple
    // fragments. Use the number of frames to decide what to do.
    if !items.is_empty() && items[0].is_empty() {
      let fragments = &items[1..];

      return match number_of_frames {
        // Exactly one frame, so all fragments must belong to it
        None | Some(1) => {
          Ok(vec![fragments.iter().map(|f| f.as_slice()).collect()])
        }

        // The same number of fragments as frames, so each fragment is its own
        // frame
        Some(number_of_frames) if number_of_frames == fragments.len() => {
          Ok(fragments.iter().map(|f| vec![f.as_slice()]).collect())
        }

        // There is a different number of fragments and frames. Given there is
        // no basic offset table, this means there's no way to allocate
        // fragments to frames.
        _ => Err(DataError::new_value_invalid(
          "Encapsulated pixel data structure can't be determined".to_string(),
        )),
      };
    }

    // Encapsulated pixel data with a basic offset table. A single frame can be
    // spread over one or more fragments.
    if items.len() > 1 {
      let basic_offset_table = &items[0];

      // Decode the 32-bit integers in the basic offset table data
      if basic_offset_table.len() % 4 != 0 {
        return Err(DataError::new_value_invalid(
          "Encapsulated pixel data basic offset table is invalid".to_string(),
        ));
      }
      let mut basic_offset_table_values = vec![0; basic_offset_table.len() / 4];
      byteorder::LittleEndian::read_u32_into(
        basic_offset_table.as_slice(),
        basic_offset_table_values.as_mut_slice(),
      );

      // Check the basic offset table is sorted
      if !basic_offset_table_values.windows(2).all(|w| w[0] <= w[1]) {
        return Err(DataError::new_value_invalid(
          "Encapsulated pixel data basic offset table is not sorted"
            .to_string(),
        ));
      }

      // The first item in the basic offset table should always be zero
      if basic_offset_table_values[0] != 0 {
        return Err(DataError::new_value_invalid(
          "Encapsulated pixel data basic offset table does not start at zero"
            .to_string(),
        ));
      }

      // Turn the flat list of fragments into a list of frames
      let frames = fragments_to_frames_using_basic_offset_table(
        items[1..].iter().map(|f| f.as_ref().as_slice()).collect(),
        &basic_offset_table_values[1..],
      )?;

      return Ok(frames);
    }
  }

  Err(DataError::new_value_not_present())
}

/// Takes a list of pixel data fragments and turns them into a list of frames
/// using a basic offset table. A single frame can be made up of one or more
/// fragments, and the basic offset table specifies where the frame boundaries
/// lie.
///
fn fragments_to_frames_using_basic_offset_table<'a>(
  fragments: Vec<&'a [u8]>,
  mut basic_offset_table: &[u32],
) -> Result<Vec<Vec<&'a [u8]>>, DataError> {
  let mut offset = 0usize;
  let mut fragments = fragments.as_slice();

  let mut current_frame: Vec<&'a [u8]> = vec![];
  let mut frames: Vec<Vec<&'a [u8]>> = vec![];

  loop {
    // When the basic offset table has no more entries, all remaining fragments
    // constitute the final frame
    if basic_offset_table.is_empty() {
      if !fragments.is_empty() {
        frames.push(fragments.to_vec());
      }

      break;
    }

    if fragments.is_empty() {
      return Err(DataError::new_value_invalid(
        "Encapsulated pixel data basic offset table is malformed".to_string(),
      ));
    }

    let next_frame_offset = basic_offset_table[0] as usize;

    // Add the next fragment to the current frame
    let fragment = &fragments[0];
    current_frame.push(fragment);
    fragments = &fragments[1..];

    // Increment the offset, with an extra 8 bytes for the item header
    offset += fragment.len() + 8;

    // If the offset now exceeds the offset to the next frame, then the values
    // in the basic offset table are invalid
    if offset > next_frame_offset {
      return Err(DataError::new_value_invalid(
        "Encapsulated pixel data basic offset table is malformed".to_string(),
      ));
    }

    // If the next offset in the basic offset table has been reached then
    // this frame is now complete, so add it to the list and start gathering
    // fragments for the next frame
    if offset == next_frame_offset {
      frames.push(current_frame);
      current_frame = Vec::new();

      basic_offset_table = &basic_offset_table[1..];
    }
  }

  Ok(frames)
}

struct ExtendedOffsetTableEntry {
  offset: u64,
  length: u64,
}

type ExtendedOffsetTable = Vec<ExtendedOffsetTableEntry>;

/// Returns the extended offset table present in the '(7FE0,0001) Extended
/// Offset Table', and '(7FE0,0001) Extended Offset Table Lengths' data
/// elements, if present in the data set.
///
fn parse_extended_offset_table(
  data_set: &DataSet,
) -> Result<ExtendedOffsetTable, DataError> {
  // Get the value of the '(0x7FE0,0001) Extended Offset Table' data
  // element
  let extended_offset_table_bytes = data_set.get_value_bytes(
    registry::EXTENDED_OFFSET_TABLE.tag,
    ValueRepresentation::OtherVeryLongString,
  )?;

  if extended_offset_table_bytes.len() % 8 != 0 {
    return Err(DataError::new_value_invalid(
      "Extended offset table has invalid size".to_string(),
    ));
  }

  let mut extended_offset_table =
    vec![0u64; extended_offset_table_bytes.len() / 8];
  byteorder::LittleEndian::read_u64_into(
    extended_offset_table_bytes.as_slice(),
    extended_offset_table.as_mut_slice(),
  );

  // Get the value of the '(0x7FE0,0002) Extended Offset Table Lengths' data
  // element
  let extended_offset_table_lengths_bytes = data_set.get_value_bytes(
    registry::EXTENDED_OFFSET_TABLE_LENGTHS.tag,
    ValueRepresentation::OtherVeryLongString,
  )?;

  if extended_offset_table_lengths_bytes.len() % 8 != 0 {
    return Err(DataError::new_value_invalid(
      "Extended offset table lengths has invalid size".to_string(),
    ));
  }

  let mut extended_offset_table_lengths =
    vec![0u64; extended_offset_table_lengths_bytes.len() / 8];
  byteorder::LittleEndian::read_u64_into(
    extended_offset_table_lengths_bytes.as_slice(),
    extended_offset_table_lengths.as_mut_slice(),
  );

  // Check the two lists are of the same length
  if extended_offset_table.len() != extended_offset_table_lengths.len() {
    return Err(DataError::new_value_invalid(
      "Extended offset table and lengths are of different size".to_string(),
    ));
  }

  // Return the extended offset table
  let mut entries = Vec::with_capacity(extended_offset_table.len());
  for i in 0..extended_offset_table.len() {
    entries.push(ExtendedOffsetTableEntry {
      offset: extended_offset_table[i],
      length: extended_offset_table_lengths[i],
    });
  }

  Ok(entries)
}

/// Takes a list of pixel data fragments and turns them into a list of frames
/// using an extended offset table. Each frame is made up of exactly one
/// fragment.
///
fn fragments_to_frames_using_extended_offset_table<'a>(
  fragments: Vec<&'a [u8]>,
  extended_offset_table: &ExtendedOffsetTable,
) -> Result<Vec<&'a [u8]>, DataError> {
  if fragments.len() != extended_offset_table.len() {
    return Err(DataError::new_value_invalid(
      "Encapsulated pixel data extended offset table size does not match \
        the number of pixel data fragments"
        .to_string(),
    ));
  }

  let mut current_offset = 0u64;
  let mut frames = vec![];

  for (entry, fragment) in extended_offset_table.iter().zip(fragments.iter()) {
    // Check the extended offset table's offset matches the offset of this
    // fragment
    if current_offset != entry.offset {
      return Err(DataError::new_value_invalid(
        "Encapsulated pixel data extended offset table is malformed"
          .to_string(),
      ));
    }

    // Check if the length value in the extended offset table exceeds the
    // length of the corresponding fragment
    if fragment.len() < entry.length as usize {
      return Err(DataError::new_value_invalid(format!(
        "Encapsulated pixel data extended offset table length of {} bytes \
        exceeds the fragment length of {} bytes",
        entry.length,
        fragment.len(),
      )));
    }

    // Slice the bytes for the frame from the fragment. The frame length is
    // allowed to be less than the size of the fragment, which can be used
    // in cases where the frame's data is of odd length, as fragment length
    // is always even.
    frames.push(&fragment[0..(entry.length as usize)]);

    current_offset += entry.length + 8;
  }

  Ok(frames)
}

/// Returns the file extension to use for raw image data in the given transfer
/// syntax. If there is no sensible file extension to use then `".bin"` is
/// returned.
///
pub fn file_extension_for_transfer_syntax(ts: &TransferSyntax) -> &'static str {
  match ts {
    // JPEG and JPEG Lossless use the .jpg extension
    ts
      if ts == &transfer_syntax::JPEG_BASELINE_8BIT
      || ts == &transfer_syntax::JPEG_EXTENDED_12BIT
      || ts == &transfer_syntax::JPEG_LOSSLESS_NON_HIERARCHICAL
      || ts == &transfer_syntax::JPEG_LOSSLESS_NON_HIERARCHICAL_SV1
    => ".jpg",

    // JPEG-LS uses the .jls extension
    ts
      if ts == &transfer_syntax::JPEG_LS_LOSSLESS
      || ts == &transfer_syntax::JPEG_LS_LOSSY_NEAR_LOSSLESS
    => ".jls",

    // JPEG 2000 uses the .jp2 extension
    ts
      if ts == &transfer_syntax::JPEG_2K_LOSSLESS_ONLY
      || ts == &transfer_syntax::JPEG_2K
      || ts == &transfer_syntax::JPEG_2K_MULTI_COMPONENT_LOSSLESS_ONLY
      || ts == &transfer_syntax::JPEG_2K_MULTI_COMPONENT
    => ".jp2",

    // MPEG-2 uses the .mp2 extension
    ts
      if ts == &transfer_syntax::MPEG2_MAIN_PROFILE_MAIN_LEVEL
      || ts == &transfer_syntax::FRAGMENTABLE_MPEG2_MAIN_PROFILE_MAIN_LEVEL
      || ts == &transfer_syntax::MPEG2_MAIN_PROFILE_HIGH_LEVEL
      || ts == &transfer_syntax::FRAGMENTABLE_MPEG2_MAIN_PROFILE_HIGH_LEVEL
    => ".mp2",

    // MPEG-4 uses the .mp4 extension
    ts
      if ts == &transfer_syntax::MPEG4_AVC_H264_HIGH_PROFILE
      || ts == &transfer_syntax::FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE
      || ts == &transfer_syntax::MPEG4_AVC_H264_BD_COMPATIBLE_HIGH_PROFILE
      || ts
      == &transfer_syntax::FRAGMENTABLE_MPEG4_AVC_H264_BD_COMPATIBLE_HIGH_PROFILE
      || ts == &transfer_syntax::MPEG4_AVC_H264_HIGH_PROFILE_FOR_2D_VIDEO
      || ts
      == &transfer_syntax::FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE_FOR_2D_VIDEO
      || ts == &transfer_syntax::MPEG4_AVC_H264_HIGH_PROFILE_FOR_3D_VIDEO
      || ts
      == &transfer_syntax::FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE_FOR_3D_VIDEO
      || ts == &transfer_syntax::MPEG4_AVC_H264_STEREO_HIGH_PROFILE
      || ts == &transfer_syntax::FRAGMENTABLE_MPEG4_AVC_H264_STEREO_HIGH_PROFILE
    => ".mp4",

    // HEVC/H.265 also uses the .mp4 extension
    ts
      if ts == &transfer_syntax::HEVC_H265_MAIN_PROFILE
      || ts == &transfer_syntax::HEVC_H265_MAIN_10_PROFILE
    => ".mp4",

    // High-Throughput JPEG 2000 uses the .jph extension
    ts
      if ts == &transfer_syntax::HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY
      || ts
      == &transfer_syntax::HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY
      || ts == &transfer_syntax::HIGH_THROUGHPUT_JPEG_2K
    => ".jph",

    // Everything else uses the .bin extension as there isn't a meaningful image
    // extension for them to use
    _ => ".bin",
  }
}

#[cfg(test)]
mod tests {
  use std::rc::Rc;

  use super::*;

  #[test]
  fn get_pixel_data_test() {
    let mut data_set_with_three_fragments = DataSet::new();
    data_set_with_three_fragments.insert(
      registry::PIXEL_DATA.tag,
      DataElementValue::new_encapsulated_pixel_data(
        ValueRepresentation::OtherByteString,
        vec![
          Rc::new(vec![]),
          Rc::new("1".repeat(0x4C6).as_bytes().to_vec()),
          Rc::new("2".repeat(0x24A).as_bytes().to_vec()),
          Rc::new("3".repeat(0x628).as_bytes().to_vec()),
        ],
      )
      .unwrap(),
    );

    // Read a single frame of non-encapsulated OB data
    let pixel_data = DataElementValue::new_binary(
      ValueRepresentation::OtherByteString,
      Rc::new([1, 2, 3, 4].to_vec()),
    )
    .unwrap();

    let mut ds = DataSet::new();
    ds.insert(registry::PIXEL_DATA.tag, pixel_data.clone());

    assert_eq!(
      ds.get_pixel_data(),
      Ok((
        ValueRepresentation::OtherByteString,
        vec![vec![[1, 2, 3, 4].as_slice()]]
      )),
    );

    // Read two frames of non-encapsulated OB data
    let mut ds = DataSet::new();
    ds.insert(registry::PIXEL_DATA.tag, pixel_data.clone());
    ds.insert_int_value(&registry::NUMBER_OF_FRAMES, &[2])
      .unwrap();
    assert_eq!(
      ds.get_pixel_data(),
      Ok((
        ValueRepresentation::OtherByteString,
        vec![vec![[1, 2].as_slice()], vec![[3, 4].as_slice()]]
      )),
    );

    // Read malformed multi-frame non-encapsulated OB data
    let mut ds = DataSet::new();
    ds.insert(registry::PIXEL_DATA.tag, pixel_data.clone());
    ds.insert_int_value(&registry::NUMBER_OF_FRAMES, &[3])
      .unwrap();
    assert_eq!(
      ds.get_pixel_data(),
      Err(DataError::new_value_invalid(
        "Multi-frame pixel data of length 4 does not divide evenly into 3 \
         frames"
          .to_string()
      )),
    );

    // Read frames specified by an extended offset table
    let mut ds = data_set_with_three_fragments.clone();
    ds.insert(
      registry::EXTENDED_OFFSET_TABLE.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherVeryLongString,
        Rc::new(vec![
          0, 0, 0, 0, 0, 0, 0, 0, 206, 4, 0, 0, 0, 0, 0, 0, 32, 7, 0, 0, 0, 0,
          0, 0,
        ]),
      )
      .unwrap(),
    );
    ds.insert(
      registry::EXTENDED_OFFSET_TABLE_LENGTHS.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherVeryLongString,
        Rc::new(vec![
          198, 4, 0, 0, 0, 0, 0, 0, 74, 2, 0, 0, 0, 0, 0, 0, 39, 6, 0, 0, 0, 0,
          0, 0,
        ]),
      )
      .unwrap(),
    );

    assert_eq!(
      ds.get_pixel_data(),
      Ok((
        ValueRepresentation::OtherByteString,
        vec![
          vec!["1".repeat(0x4C6).as_bytes().to_vec().as_slice()],
          vec!["2".repeat(0x24A).as_bytes().to_vec().as_slice()],
          vec!["3".repeat(0x627).as_bytes().to_vec().as_slice()],
        ]
      ))
    );

    // Read three fragments into a single frame
    // Taken from the DICOM standard. Ref: PS3.5 Table A.4-1.
    assert_eq!(
      data_set_with_three_fragments.get_pixel_data(),
      Ok((
        ValueRepresentation::OtherByteString,
        vec![vec![
          "1".repeat(0x4C6).as_bytes().to_vec().as_slice(),
          "2".repeat(0x24A).as_bytes().to_vec().as_slice(),
          "3".repeat(0x628).as_bytes().to_vec().as_slice()
        ],]
      ))
    );

    // Reads three fragments as frames when number of frames is three
    // Similar to the previous test but with a number of frames value present
    // that causes each fragment to be its own frame
    let mut ds = data_set_with_three_fragments.clone();
    ds.insert_int_value(&registry::NUMBER_OF_FRAMES, &[3])
      .unwrap();
    assert_eq!(
      ds.get_pixel_data(),
      Ok((
        ValueRepresentation::OtherByteString,
        vec![
          vec!["1".repeat(0x4C6).as_bytes().to_vec().as_slice()],
          vec!["2".repeat(0x24A).as_bytes().to_vec().as_slice()],
          vec!["3".repeat(0x628).as_bytes().to_vec().as_slice()],
        ]
      ))
    );

    // Read frames specified by a basic offset table
    // Taken from the DICOM standard. Ref: PS3.5 Table A.4-2.
    let mut ds = DataSet::new();
    ds.insert(
      registry::PIXEL_DATA.tag,
      DataElementValue::new_encapsulated_pixel_data(
        ValueRepresentation::OtherByteString,
        vec![
          Rc::new(vec![0, 0, 0, 0, 0x46, 0x06, 0, 0]),
          Rc::new("A".repeat(0x2C8).as_bytes().to_vec()),
          Rc::new("a".repeat(0x36E).as_bytes().to_vec()),
          Rc::new("B".repeat(0xBC8).as_bytes().to_vec()),
        ],
      )
      .unwrap(),
    );

    assert_eq!(
      ds.get_pixel_data(),
      Ok((
        ValueRepresentation::OtherByteString,
        vec![
          vec![
            "A".repeat(0x2C8).as_bytes().to_vec().as_slice(),
            "a".repeat(0x36E).as_bytes().to_vec().as_slice()
          ],
          vec!["B".repeat(0xBC8).as_bytes().to_vec().as_slice()],
        ]
      ))
    );
  }
}
