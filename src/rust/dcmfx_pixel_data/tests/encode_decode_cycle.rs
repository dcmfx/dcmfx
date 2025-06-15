use std::rc::Rc;

#[cfg(not(feature = "std"))]
use alloc::rc::Rc;

use dcmfx_pixel_data::PixelDataEncodeConfig;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use dcmfx_core::{
  DataElementValue, DataSet, TransferSyntax, ValueRepresentation, dictionary,
  transfer_syntax,
};

use dcmfx_pixel_data::{
  ColorImage, ColorSpace, LookupTable, MonochromeImage, decode, encode,
  iods::{
    PaletteColorLookupTableModule,
    image_pixel_module::{
      BitsAllocated, ImagePixelModule, PhotometricInterpretation,
      PixelRepresentation, PlanarConfiguration, SamplesPerPixel,
    },
  },
};

const RNG_SEED: u64 = 1023;

#[test]
fn test_native_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules(),
    &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN,
    0.0,
    0.0,
  );
}

#[test]
fn test_rle_lossless_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules()
      .into_iter()
      .filter(|m| !m.photometric_interpretation().is_ybr_full_422())
      .collect(),
    &transfer_syntax::RLE_LOSSLESS,
    0.0,
    0.0,
  );
}

#[test]
fn test_jpeg_baseline_8bit_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules()
      .into_iter()
      .filter(|m| {
        !m.photometric_interpretation().is_palette_color()
          && m.bits_allocated() == BitsAllocated::Eight
          && m.pixel_representation().is_unsigned()
      })
      .collect(),
    &transfer_syntax::JPEG_BASELINE_8BIT,
    0.01,
    0.25,
  );
}

#[test]
fn test_jpeg_extended_12bit_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules()
      .into_iter()
      .filter(|m| {
        !m.photometric_interpretation().is_palette_color()
          && m.bits_allocated() == BitsAllocated::Sixteen
          && m.bits_stored() <= 12
          && m.pixel_representation().is_unsigned()
      })
      .collect(),
    &transfer_syntax::JPEG_EXTENDED_12BIT,
    0.01,
    0.01,
  );
}

#[test]
fn test_jpeg_ls_lossless_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules()
      .into_iter()
      .filter(|m| {
        !m.photometric_interpretation().is_palette_color()
          && !m.photometric_interpretation().is_ybr_full_422()
          && (m.bits_allocated() == BitsAllocated::Eight
            || m.bits_allocated() == BitsAllocated::Sixteen)
          && m.pixel_representation().is_unsigned()
      })
      .collect(),
    &transfer_syntax::JPEG_LS_LOSSLESS,
    0.0,
    0.0,
  );
}

#[test]
fn test_jpeg_ls_near_lossless_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules()
      .into_iter()
      .filter(|m| {
        !m.photometric_interpretation().is_palette_color()
          && !m.photometric_interpretation().is_ybr_full_422()
          && (m.bits_allocated() == BitsAllocated::Eight
            || m.bits_allocated() == BitsAllocated::Sixteen)
          && m.pixel_representation().is_unsigned()
      })
      .collect(),
    &transfer_syntax::JPEG_LS_LOSSY_NEAR_LOSSLESS,
    0.01,
    0.02,
  );
}

#[test]
fn test_jpeg_2k_lossless_only_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules()
      .into_iter()
      .filter(|m| {
        !m.photometric_interpretation().is_ybr_full_422()
          && (2..=30).contains(&m.bits_stored())
      })
      .collect(),
    &transfer_syntax::JPEG_2K_LOSSLESS_ONLY,
    0.0,
    0.0,
  );
}

#[test]
fn test_jpeg_2k_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules()
      .into_iter()
      .filter(|m| {
        (m.photometric_interpretation().is_monochrome()
          || m.photometric_interpretation().is_rgb())
          && (2..=30).contains(&m.bits_stored())
      })
      .collect(),
    &transfer_syntax::JPEG_2K,
    0.15,
    0.1,
  );
}

#[test]
fn test_high_throughput_jpeg_2k_lossless_only_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules()
      .into_iter()
      .filter(|m| {
        !m.photometric_interpretation().is_ybr_full_422()
          && (2..=30).contains(&m.bits_stored())
      })
      .collect(),
    &transfer_syntax::HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY,
    0.0,
    0.0,
  );
}

#[test]
fn test_high_throughput_jpeg_2k_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules()
      .into_iter()
      .filter(|m| {
        !m.photometric_interpretation().is_palette_color()
          && !m.photometric_interpretation().is_ybr_full_422()
          && (2..=30).contains(&m.bits_stored())
      })
      .collect(),
    &transfer_syntax::HIGH_THROUGHPUT_JPEG_2K,
    0.02,
    0.02,
  );
}

#[test]
fn test_jpeg_xl_lossless_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules()
      .into_iter()
      .filter(|m| {
        (m.photometric_interpretation().is_monochrome()
          || m.photometric_interpretation().is_rgb())
          && (m.bits_allocated() == BitsAllocated::Eight
            || m.bits_allocated() == BitsAllocated::Sixteen)
          && m.pixel_representation().is_unsigned()
      })
      .collect(),
    &transfer_syntax::JPEG_XL_LOSSLESS,
    0.0,
    0.0,
  );
}

#[test]
fn test_jpeg_xl_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules()
      .into_iter()
      .filter(|m| {
        (m.photometric_interpretation().is_monochrome()
          || m.photometric_interpretation().is_rgb())
          && (m.bits_allocated() == BitsAllocated::Eight
            || m.bits_allocated() == BitsAllocated::Sixteen)
          && m.pixel_representation().is_unsigned()
      })
      .collect(),
    &transfer_syntax::JPEG_XL,
    0.05,
    0.05,
  );
}

#[test]
fn test_deflated_image_frame_encode_decode_cycle() {
  test_encode_decode_cycle(
    all_image_pixel_modules(),
    &transfer_syntax::DEFLATED_IMAGE_FRAME_COMPRESSION,
    0.0,
    0.0,
  );
}

fn test_encode_decode_cycle(
  image_pixel_modules: Vec<ImagePixelModule>,
  transfer_syntax: &'static TransferSyntax,
  monochrome_image_max_reencode_delta: f64,
  color_image_max_reencode_delta: f64,
) {
  for image_pixel_module in image_pixel_modules {
    if image_pixel_module.is_monochrome() {
      test_monochrome_image_encode_decode_cycle(
        &image_pixel_module,
        transfer_syntax,
        monochrome_image_max_reencode_delta,
      );
    } else {
      test_color_image_encode_decode_cycle(
        &image_pixel_module,
        transfer_syntax,
        color_image_max_reencode_delta,
      );
    }
  }
}

fn test_monochrome_image_encode_decode_cycle(
  image_pixel_module: &ImagePixelModule,
  transfer_syntax: &'static TransferSyntax,
  max_reencode_delta: f64,
) {
  let original_image = create_monochrome_image(&image_pixel_module);

  // Encode into the target transfer syntax
  let mut encoded_frame = encode::encode_monochrome(
    &original_image,
    &image_pixel_module,
    transfer_syntax,
    &encode_config(),
  )
  .unwrap();

  // Decode out of the target transfer syntax
  let decoded_image = decode::decode_monochrome(
    &mut encoded_frame,
    transfer_syntax,
    &image_pixel_module,
  )
  .unwrap();

  // Check dimensions are unchanged
  assert_eq!(original_image.width(), decoded_image.width());
  assert_eq!(original_image.height(), decoded_image.height());

  // Convert images to stored values so that they can be compared
  let original_image = original_image.to_stored_values();
  let decoded_image = decoded_image.to_stored_values();

  // Convert the max re-encode delta to an integer value
  let max_reencode_delta = (max_reencode_delta
    * (1i64 << image_pixel_module.bits_stored()) as f64)
    .ceil() as i64;

  let mut incorrect_pixel_count = 0;

  // Compare all pixels
  for i in 0..image_pixel_module.pixel_count() {
    if (original_image[i] - decoded_image[i]).abs() > max_reencode_delta {
      incorrect_pixel_count += 1;
    }
  }

  assert!(
    incorrect_pixel_count <= original_image.len().div_ceil(10),
    "More than 10% of monochrome pixels ({}/{}) exceed the allowed error margin for {}",
    incorrect_pixel_count,
    original_image.len(),
    image_pixel_module,
  );
}

fn test_color_image_encode_decode_cycle(
  image_pixel_module: &ImagePixelModule,
  transfer_syntax: &'static TransferSyntax,
  mut max_reencode_delta: f64,
) {
  // If the Image Pixel Module isn't supported for encoding then there's
  // nothing to do
  let encoded_image_pixel_module = encode::encode_image_pixel_module(
    image_pixel_module.clone(),
    transfer_syntax,
    &encode_config(),
  )
  .unwrap();

  // Create a random color image to test with
  let original_image = create_color_image(&image_pixel_module);

  // Encode into the target transfer syntax
  let mut encoded_frame = encode::encode_color(
    &original_image,
    &encoded_image_pixel_module,
    transfer_syntax,
    &encode_config(),
  )
  .unwrap();

  // Decode out of the target transfer syntax
  let decoded_image = decode::decode_color(
    &mut encoded_frame,
    transfer_syntax,
    &encoded_image_pixel_module,
  )
  .unwrap();

  // Check dimensions are unchanged
  assert_eq!(original_image.width(), decoded_image.width());
  assert_eq!(original_image.height(), decoded_image.height());

  // Convert images to RGB f64 so their pixels can each be compared
  let original_image = original_image.to_rgb_f64_image();
  let decoded_image = decoded_image.to_rgb_f64_image();

  // The allowed error doubles with every unused bit because the comparison is
  // done following expansion to the 0-1 range, meaning the same error in a raw
  // stored value ends up as double the error in the normalized representation.
  let bits_stored = image_pixel_module.bits_stored();
  let bits_allocated = u16::from(u8::from(image_pixel_module.bits_allocated()));
  for _ in bits_stored..bits_allocated {
    max_reencode_delta *= 2.0;
  }

  let mut incorrect_pixel_count = 0;

  // Compare all pixels
  for y in 0..original_image.height() {
    for x in 0..original_image.width() {
      let original_pixel = original_image.get_pixel(x, y);
      let decoded_pixel = decoded_image.get_pixel(x, y);

      for i in 0..3 {
        if (original_pixel.0[i] - decoded_pixel.0[i]).abs() > max_reencode_delta
        {
          incorrect_pixel_count += 1;
        }
      }
    }
  }

  assert!(
    incorrect_pixel_count <= original_image.len().div_ceil(10),
    "More than 10% of color pixels ({}/{}) exceed the allowed error margin for {}",
    incorrect_pixel_count,
    original_image.len(),
    image_pixel_module
  );
}

/// Returns an pixel data encode config that uses maximum quality for lossy
/// compression so that any changes following encode and decode are minimized.
///
fn encode_config() -> PixelDataEncodeConfig {
  let mut encode_config = PixelDataEncodeConfig::new();
  encode_config.set_effort(1);
  encode_config.set_quality(100);
  encode_config
}

/// Enumerates a large number of different configurations of
/// [`ImagePixelModule`] that covers every possible combination of setups, each
/// at a variety of different resolutions.
///
fn all_image_pixel_modules() -> Vec<ImagePixelModule> {
  let photometric_interpretations = &[
    PhotometricInterpretation::Monochrome1 {
      pixel_representation: PixelRepresentation::Signed,
    },
    PhotometricInterpretation::Monochrome1 {
      pixel_representation: PixelRepresentation::Unsigned,
    },
    PhotometricInterpretation::Monochrome2 {
      pixel_representation: PixelRepresentation::Signed,
    },
    PhotometricInterpretation::Monochrome2 {
      pixel_representation: PixelRepresentation::Unsigned,
    },
    PhotometricInterpretation::PaletteColor {
      palette: create_palette_color_lookup_table_module(),
    },
    PhotometricInterpretation::Rgb,
    PhotometricInterpretation::YbrFull,
    PhotometricInterpretation::YbrFull422,
  ];

  let planar_configurations = &[
    PlanarConfiguration::Interleaved,
    PlanarConfiguration::Separate,
  ];

  let bits_allocated = &[
    BitsAllocated::One,
    BitsAllocated::Eight,
    BitsAllocated::Sixteen,
    BitsAllocated::ThirtyTwo,
  ];

  let dimensions = &[
    (1, 1),
    (1, 5),
    (5, 1),
    (2, 2),
    (5, 5),
    (10, 5),
    (5, 10),
    (10, 10),
    (32, 16),
    (16, 32),
    (32, 32),
    (64, 64),
  ];

  let mut image_pixel_modules = vec![];

  for photometric_interpretation in photometric_interpretations {
    for planar_configuration in planar_configurations {
      for bits_allocated in bits_allocated {
        let mut bits_stored = vec![u8::from(*bits_allocated)];

        // Test variations where the number of bits stored is less than the
        // bits allocated
        if bits_allocated != &BitsAllocated::One {
          bits_stored.push(1);
          bits_stored.push(u8::from(*bits_allocated) - 1);
          bits_stored.push(u8::from(*bits_allocated) / 2);
        }

        for bits_stored in bits_stored {
          for (rows, columns) in dimensions {
            if photometric_interpretation.is_monochrome() {
              image_pixel_modules.push(
                ImagePixelModule::new_basic(
                  SamplesPerPixel::One,
                  photometric_interpretation.clone(),
                  *rows,
                  *columns,
                  *bits_allocated,
                  bits_stored.into(),
                )
                .unwrap(),
              );
            } else {
              // A bits allocated of one is not supported by color images
              if *bits_allocated == BitsAllocated::One {
                continue;
              }

              // A bits allocated of 32 is not supported by palette color images
              if *bits_allocated == BitsAllocated::ThirtyTwo
                && photometric_interpretation.is_palette_color()
              {
                continue;
              }

              // The YBR 422 photometric interpretation requires an even width
              if photometric_interpretation.is_ybr_full_422()
                && columns % 2 == 1
              {
                continue;
              }

              let samples_per_pixel =
                if photometric_interpretation.is_palette_color() {
                  SamplesPerPixel::One
                } else {
                  SamplesPerPixel::Three {
                    planar_configuration: *planar_configuration,
                  }
                };

              image_pixel_modules.push(
                ImagePixelModule::new_basic(
                  samples_per_pixel,
                  photometric_interpretation.clone(),
                  *rows,
                  *columns,
                  *bits_allocated,
                  bits_stored.into(),
                )
                .unwrap(),
              );
            }
          }
        }
      }
    }
  }

  image_pixel_modules
}

/// Creates a [`MonochromeImage`] with random data based on the given
/// [`ImagePixelModule`].
///
fn create_monochrome_image(
  image_pixel_module: &ImagePixelModule,
) -> MonochromeImage {
  fn create_image<T>(
    image_pixel_module: &ImagePixelModule,
    create: impl Fn(
      u16,
      u16,
      Vec<T>,
      u16,
      bool,
    ) -> Result<MonochromeImage, &'static str>,
  ) -> MonochromeImage
  where
    T: TryFrom<i64> + Copy + Default,
    <T as TryFrom<i64>>::Error: std::fmt::Debug,
  {
    let mut rng = SmallRng::seed_from_u64(RNG_SEED);

    let range = if image_pixel_module.pixel_representation().is_signed()
      && image_pixel_module.bits_allocated() != BitsAllocated::One
    {
      let m = 1i64 << (image_pixel_module.bits_stored() - 1);
      (-m)..m
    } else {
      0i64..(1i64 << image_pixel_module.bits_stored())
    };

    // Create random data
    let mut data = vec![
      T::default();
      image_pixel_module.frame_size_in_bytes()
        / core::mem::size_of::<T>()
    ];

    for i in 0..data.len() {
      data[i] = T::try_from(rng.random_range(range.clone())).unwrap();
    }

    create(
      image_pixel_module.columns(),
      image_pixel_module.rows(),
      data,
      image_pixel_module.bits_stored(),
      image_pixel_module
        .photometric_interpretation()
        .is_monochrome1(),
    )
    .unwrap()
  }

  match (
    image_pixel_module.bits_allocated(),
    image_pixel_module.pixel_representation(),
  ) {
    (BitsAllocated::One, pixel_representation) => create_image(
      &image_pixel_module,
      |width, height, data, _bits_stored, is_monochrome1| {
        MonochromeImage::new_bitmap(
          width,
          height,
          data,
          pixel_representation.is_signed(),
          is_monochrome1,
        )
      },
    ),

    (BitsAllocated::Eight, PixelRepresentation::Signed) => {
      create_image(&image_pixel_module, MonochromeImage::new_i8)
    }

    (BitsAllocated::Eight, PixelRepresentation::Unsigned) => {
      create_image(&image_pixel_module, MonochromeImage::new_u8)
    }

    (BitsAllocated::Sixteen, PixelRepresentation::Signed) => {
      create_image(&image_pixel_module, MonochromeImage::new_i16)
    }

    (BitsAllocated::Sixteen, PixelRepresentation::Unsigned) => {
      create_image(&image_pixel_module, MonochromeImage::new_u16)
    }

    (BitsAllocated::ThirtyTwo, PixelRepresentation::Signed) => {
      create_image(&image_pixel_module, MonochromeImage::new_i32)
    }

    (BitsAllocated::ThirtyTwo, PixelRepresentation::Unsigned) => {
      create_image(&image_pixel_module, MonochromeImage::new_u32)
    }
  }
}

/// Creates a [`ColorImage`] with random data based on the given
/// [`ImagePixelModule`].
///
fn create_color_image(image_pixel_module: &ImagePixelModule) -> ColorImage {
  fn create_image<T>(
    image_pixel_module: &ImagePixelModule,
    create: impl Fn(
      u16,
      u16,
      Vec<T>,
      ColorSpace,
      u16,
    ) -> Result<ColorImage, &'static str>,
  ) -> ColorImage
  where
    T: TryFrom<u64> + Copy + std::fmt::Debug,
    <T as TryFrom<u64>>::Error: std::fmt::Debug,
  {
    let mut rng = SmallRng::seed_from_u64(RNG_SEED);

    let max_value = (1u64 << image_pixel_module.bits_stored()) - 1;

    // Create random data
    let mut data = vec![];
    for _ in 0..(image_pixel_module.pixel_count()
      * usize::from(u8::from(image_pixel_module.samples_per_pixel())))
    {
      data.push(T::try_from(rng.random_range(0..=max_value)).unwrap());
    }

    // When using a YBR 422 encoding, ensure Cb and Cr values are identical for
    // adjacent pixels that share that value
    if image_pixel_module
      .photometric_interpretation()
      .is_ybr_full_422()
    {
      for i in data.chunks_exact_mut(6) {
        i[4] = i[1];
        i[5] = i[2];
      }
    }

    let color_space = match image_pixel_module.photometric_interpretation() {
      PhotometricInterpretation::PaletteColor { .. }
      | PhotometricInterpretation::Rgb => ColorSpace::Rgb,
      PhotometricInterpretation::YbrFull => ColorSpace::Ybr { is_422: false },
      PhotometricInterpretation::YbrFull422 => ColorSpace::Ybr { is_422: true },
      _ => unreachable!(),
    };

    create(
      image_pixel_module.columns(),
      image_pixel_module.rows(),
      data,
      color_space.clone(),
      image_pixel_module.bits_stored(),
    )
    .unwrap()
  }

  match image_pixel_module.bits_allocated() {
    BitsAllocated::One => unreachable!(),

    BitsAllocated::Eight => {
      if image_pixel_module
        .photometric_interpretation()
        .is_palette_color()
      {
        create_image(
          &image_pixel_module,
          |columns, rows, data, _color_space, bits_stored| {
            let palette = create_palette_color_lookup_table_module();
            ColorImage::new_palette8(columns, rows, data, palette, bits_stored)
          },
        )
      } else {
        create_image(&image_pixel_module, ColorImage::new_u8)
      }
    }

    BitsAllocated::Sixteen => {
      if image_pixel_module
        .photometric_interpretation()
        .is_palette_color()
      {
        create_image(
          &image_pixel_module,
          |columns, rows, data, _color_space, bits_stored| {
            let palette = create_palette_color_lookup_table_module();
            ColorImage::new_palette16(columns, rows, data, palette, bits_stored)
          },
        )
      } else {
        create_image(&image_pixel_module, ColorImage::new_u16)
      }
    }

    BitsAllocated::ThirtyTwo => {
      create_image(&image_pixel_module, ColorImage::new_u32)
    }
  }
}

fn create_palette_color_lookup_table_module()
-> Rc<PaletteColorLookupTableModule> {
  let mut data_set = DataSet::new();

  let lut_descriptor: Vec<u16> = vec![4, 0, 8];

  data_set.insert(
    dictionary::LUT_DESCRIPTOR.tag,
    DataElementValue::new_lookup_table_descriptor_unchecked(
      ValueRepresentation::UnsignedShort,
      bytemuck::cast_slice::<u16, u8>(&lut_descriptor)
        .to_vec()
        .into(),
    ),
  );

  data_set.insert(
    dictionary::LUT_DATA.tag,
    DataElementValue::new_other_byte_string(vec![0, 1, 2, 3]).unwrap(),
  );

  let lut = LookupTable::from_data_set(
    &data_set,
    dictionary::LUT_DESCRIPTOR.tag,
    dictionary::LUT_DATA.tag,
    None,
    None,
  )
  .unwrap();

  Rc::new(PaletteColorLookupTableModule::new(
    lut.clone(),
    lut.clone(),
    lut.clone(),
  ))
}
