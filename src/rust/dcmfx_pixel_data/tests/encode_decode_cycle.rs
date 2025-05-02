use std::rc::Rc;

#[cfg(not(feature = "std"))]
use alloc::rc::Rc;

use dcmfx_pixel_data::PixelDataEncodeConfig;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use dcmfx_core::{DataError, TransferSyntax, transfer_syntax};

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
  let transfer_syntax = &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN;

  for image_pixel_module in all_image_pixel_modules() {
    if image_pixel_module.is_grayscale() {
      test_monochrome_image_encode_decode_cycle(
        &image_pixel_module,
        transfer_syntax,
      );
    } else {
      test_color_image_encode_decode_cycle(
        &image_pixel_module,
        transfer_syntax,
      );
    }
  }
}

#[test]
fn test_rle_lossless_encode_decode_cycle() {
  let transfer_syntax = &transfer_syntax::RLE_LOSSLESS;

  for image_pixel_module in all_image_pixel_modules() {
    if image_pixel_module.is_grayscale() {
      test_monochrome_image_encode_decode_cycle(
        &image_pixel_module,
        transfer_syntax,
      );
    } else {
      if image_pixel_module
        .photometric_interpretation()
        .is_palette_color()
        || image_pixel_module.photometric_interpretation().is_ybr_422()
      {
        continue;
      }

      test_color_image_encode_decode_cycle(
        &image_pixel_module,
        transfer_syntax,
      );
    }
  }
}

#[test]
fn test_deflated_image_frame_encode_decode_cycle() {
  let transfer_syntax = &transfer_syntax::DEFLATED_IMAGE_FRAME_COMPRESSION;

  for image_pixel_module in all_image_pixel_modules() {
    if image_pixel_module.is_grayscale() {
      test_monochrome_image_encode_decode_cycle(
        &image_pixel_module,
        transfer_syntax,
      );
    } else {
      test_color_image_encode_decode_cycle(
        &image_pixel_module,
        transfer_syntax,
      );
    }
  }
}

fn test_monochrome_image_encode_decode_cycle(
  image_pixel_module: &ImagePixelModule,
  transfer_syntax: &'static TransferSyntax,
) {
  let image = create_monochrome_image(&image_pixel_module);

  let mut encoded_frame = encode::encode_monochrome(
    &image,
    transfer_syntax,
    &PixelDataEncodeConfig::new(),
  )
  .unwrap();

  let decoded_pixel_data = decode::decode_monochrome(
    &mut encoded_frame,
    transfer_syntax,
    &image_pixel_module,
  )
  .unwrap();

  assert_eq!(image, decoded_pixel_data);
}

fn test_color_image_encode_decode_cycle(
  image_pixel_module: &ImagePixelModule,
  transfer_syntax: &'static TransferSyntax,
) {
  let image = create_color_image(&image_pixel_module);

  let mut encoded_frame = encode::encode_color(
    &image,
    transfer_syntax,
    &image_pixel_module,
    &PixelDataEncodeConfig::new(),
  )
  .unwrap();

  let decoded_image = decode::decode_color(
    &mut encoded_frame,
    transfer_syntax,
    &image_pixel_module,
  )
  .unwrap();

  assert_eq!(image, decoded_image);
}

/// Enumerates a large number of different configurations of
/// [`ImagePixelModule`] that covers every possible combination of setups, each
/// at a variety of different resolutions.
///
fn all_image_pixel_modules() -> Vec<ImagePixelModule> {
  let photometric_interpretations = &[
    PhotometricInterpretation::Monochrome1,
    PhotometricInterpretation::Monochrome2,
    PhotometricInterpretation::PaletteColor {
      palette: create_palette_color_lookup_table_module(),
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
            if photometric_interpretation.is_grayscale() {
              image_pixel_modules.push(
                ImagePixelModule::new_basic(
                  SamplesPerPixel::One,
                  photometric_interpretation.clone(),
                  *rows,
                  *columns,
                  *bits_allocated,
                  bits_stored.into(),
                  PixelRepresentation::Unsigned,
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
              if photometric_interpretation.is_ybr_422() && columns % 2 == 1 {
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
                  PixelRepresentation::Unsigned,
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
    ) -> Result<MonochromeImage, DataError>,
  ) -> MonochromeImage
  where
    T: TryFrom<i64> + Copy + Default,
    <T as TryFrom<i64>>::Error: std::fmt::Debug,
  {
    let mut rng = SmallRng::seed_from_u64(RNG_SEED);

    let range = if image_pixel_module.pixel_representation().is_signed() {
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

  match image_pixel_module.bits_allocated() {
    BitsAllocated::One => create_image(
      &image_pixel_module,
      |width, height, data, _bits_stored, is_monochrome1| {
        MonochromeImage::new_bitmap(width, height, data, false, is_monochrome1)
      },
    ),

    BitsAllocated::Eight => {
      create_image(&image_pixel_module, MonochromeImage::new_u8)
    }

    BitsAllocated::Sixteen => {
      create_image(&image_pixel_module, MonochromeImage::new_u16)
    }

    BitsAllocated::ThirtyTwo => {
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
    ) -> Result<ColorImage, DataError>,
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
    if image_pixel_module.photometric_interpretation().is_ybr_422() {
      for i in data.chunks_exact_mut(6) {
        i[4] = i[1];
        i[5] = i[2];
      }
    }

    let color_space = match image_pixel_module.photometric_interpretation() {
      PhotometricInterpretation::PaletteColor { .. }
      | PhotometricInterpretation::Rgb => ColorSpace::RGB,
      PhotometricInterpretation::YbrFull => ColorSpace::YBR,
      PhotometricInterpretation::YbrFull422 => ColorSpace::YBR422,
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
  let lut = LookupTable::new(0, None, vec![], 255);

  Rc::new(PaletteColorLookupTableModule::new(
    lut.clone(),
    lut.clone(),
    lut.clone(),
  ))
}
