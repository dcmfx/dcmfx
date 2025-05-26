use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::PathBuf;

use clap::Args;
use rand::Rng;

use dcmfx::core::*;
use dcmfx::p10::*;
use dcmfx::pixel_data::{
  iods::image_pixel_module::{ImagePixelModule, PhotometricInterpretation},
  *,
};

use crate::utils::prompt_to_overwrite_if_exists;
use crate::{
  InputSource,
  args::{
    photometric_interpretation_arg::{
      PhotometricInterpretationColorArg, PhotometricInterpretationMonochromeArg,
    },
    planar_configuration_arg::PlanarConfigurationArg,
    transfer_syntax_arg::{self, TransferSyntaxArg},
  },
  utils::{self, TempFileRenamer},
};

pub const ABOUT: &str = "Modifies the content of DICOM P10 files";

#[derive(Args)]
pub struct ModifyArgs {
  #[clap(
    required = true,
    help = "The names of the DICOM P10 files to modify. Specify '-' to read \
      from stdin."
  )]
  input_filenames: Vec<PathBuf>,

  #[clap(
    long,
    short,
    help = "The name of the DICOM P10 output file. Specify '-' to write to \
      stdout.\n\
      \n\
      This argument is not permitted when multiple input files are specified."
  )]
  output_filename: Option<PathBuf>,

  #[clap(
    long,
    short = 'd',
    help = "The directory to write output files into. The names of the output \
      DICOM P10 files will be the same as the input files."
  )]
  output_directory: Option<PathBuf>,

  #[arg(
    long,
    help = "Whether to modify the input files in place, i.e. overwrite them \
      with the newly modified version rather than write it to a new file.\n\
      \n\
      If there is an error during in-place modification of a file then it will \
      not be altered.\n\
      \n\
      WARNING: modification in-place is a potentially irreversible operation.",
    default_value_t = false
  )]
  in_place: bool,

  #[arg(
    long,
    short,
    help_heading = "Transcoding",
    help = "The transfer syntax for the output DICOM P10 file. Pixel data will \
      be automatically transcoded. For some transfer syntaxes additional \
      arguments are available to control image compression."
  )]
  transfer_syntax: Option<TransferSyntaxArg>,

  #[arg(
    long,
    short,
    help_heading = "Transcoding",
    help = "When transcoding to a lossy transfer syntax, specifies the \
      compression quality in the range 1-100. A quality of 100 does not result \
      in lossless compression.\n\
      \n\
      Default value: 85",
    value_parser = clap::value_parser!(u8).range(1..=100),
  )]
  quality: Option<u8>,

  #[clap(
    long,
    help_heading = "Transcoding",
    help = "When transcoding monochrome pixel data using --transfer-syntax, \
      this specifies the photometric interpretation to be used by the output \
      DICOM P10 files. This option has no effect on color pixel data."
  )]
  photometric_interpretation_monochrome:
    Option<PhotometricInterpretationMonochromeArg>,

  #[clap(
    long,
    help_heading = "Transcoding",
    help = "When transcoding color pixel data using --transfer-syntax, this \
      specifies the photometric interpretation to be used by the transcoded \
      pixel data. This option has no effect on monochrome pixel data.\n\
      \n\
      When the output transfer syntax is 'JPEG Baseline 8-bit' the output \
      photometric interpretation defaults to 'YBR_FULL' if the color data \
      is not YBR following decoding. This is because the 'RGB' photometric \
      interpretation in JPEG is uncommon, and a YBR color space usually yields \
      better compression ratios.\n\
      \n\
      When the output transfer syntax is JPEG 2000 the output photometric \
      interpretation defaults to 'YBR_RCT' for lossless encoding, and \
      'YBR_ICT' for lossy encoding. These two photometric interpretations are \
      generally preferred in JPEG 2000, however others may be used if there is \
      a need to compress with no risk of loss from color space conversions.\n\
      \n\
      For all other output transfer syntaxes there is no default output \
      photometric interpretation, however the output photometric \
      interpretation may differ from the input for the following reasons:\n\
      \n\
      1. If the output transfer syntax doesn't support 'PALETTE_COLOR' then \
         the color image's data will be automatically expanded to 'RGB'.\n\
      \n\
      2. If the output transfer syntax doesn't support 'YBR_FULL_422' then the \
         color image's data will be automatically expanded to 'YBR_FULL'."
  )]
  photometric_interpretation_color: Option<PhotometricInterpretationColorArg>,

  #[arg(
    long,
    help_heading = "Transcoding",
    help = "When transcoding color pixel data using --transfer-syntax, this \
      specifies the planar configuration to be used by the transcoded pixel \
      data. This option has no effect on monochrome pixel data.\n\
      \n\
      The planar configuration can only be specified for the following \
      transfer syntaxes:\n\
      \n\
      - Implicit VR Little Endian\n\
      - Explicit VR Little Endian\n\
      - Encapsulated Uncompressed Explicit VR Little Endian\n\
      - Deflated Explicit VR Little Endian\n\
      - Explicit VR Big Endian\n\
      - Deflated Image Frame Compression\n\
      \n\
      The planar configuration is ignored when transcoding into other transfer \
      syntaxes."
  )]
  planar_configuration: Option<PlanarConfigurationArg>,

  #[arg(
    long,
    help = "The zlib compression level to use when outputting to the 'Deflated \
      Explicit VR Little Endian' and 'Deflated Image Frame Compression' \
      transfer syntaxes. The level ranges from 0, meaning no compression, \
      through to 9, which gives the best compression at the cost of speed.",
    default_value_t = 6,
    value_parser = clap::value_parser!(u32).range(0..=9),
  )]
  zlib_compression_level: u32,

  #[arg(
    long,
    help = "Whether to anonymize the output DICOM P10 file by removing all \
      patient data elements, other identifying data elements, as well as \
      private data elements. Note that this option does not remove any \
      identifying information that may be baked into the pixel data.",
    default_value_t = false
  )]
  anonymize: bool,

  #[arg(
    long,
    help = "A data element tag to delete and not include in the output DICOM \
      P10 file. This argument can be specified multiple times to delete \
      multiple tags.",
    value_parser = validate_data_element_tag,
  )]
  delete_tag: Vec<DataElementTag>,

  #[clap(
    long,
    help = "Overwrite files without prompting",
    default_value_t = false
  )]
  overwrite: bool,

  #[clap(
    long,
    help = "Specifies the value of the Implementation Version Name data \
      element in output DICOM P10 files. The value must conform to the \
      specification of the SS (Short String) value representation.",
    default_value_t = uids::DCMFX_IMPLEMENTATION_VERSION_NAME.to_string(),
  )]
  implementation_version_name: String,
}

fn validate_data_element_tag(s: &str) -> Result<DataElementTag, String> {
  DataElementTag::from_hex_string(s)
    .map_err(|_| "Invalid data element tag".to_string())
}

enum ModifyCommandError {
  P10Error(P10Error),
  P10PixelDataTranscodeTransformError(P10PixelDataTranscodeTransformError),
}

pub fn run(args: &ModifyArgs) -> Result<(), ()> {
  if (args.output_filename.is_some() as u8
    + args.output_directory.is_some() as u8
    + args.in_place as u8)
    != 1
  {
    eprintln!(
      "Exactly one of --output-filename, --output-directory, or --in-place \
       must be specified"
    );
    return Err(());
  }

  if args.transfer_syntax.is_none() {
    if args.photometric_interpretation_monochrome.is_some() {
      eprintln!(
        "The --photometric-interpretation-monochrome option is only valid when \
         --transfer-syntax is specified"
      );
      return Err(());
    }

    if args.photometric_interpretation_color.is_some() {
      eprintln!(
        "The --photometric-interpretation-color option is only valid when \
         --transfer-syntax is specified"
      );
      return Err(());
    }

    if args.quality.is_some() {
      eprintln!(
        "The --quality option is only valid when --transfer-syntax is specified"
      );
      return Err(());
    }
  }

  let input_sources = crate::get_input_sources(&args.input_filenames);

  crate::validate_output_args(
    &input_sources,
    &args.output_filename,
    &args.output_directory,
  );

  if args.in_place && input_sources.contains(&InputSource::Stdin) {
    eprintln!("Error: --in-place is not valid when reading from stdin");
    return Err(());
  }

  for input_source in input_sources {
    let output_filename: PathBuf = if args.in_place {
      input_source.path()
    } else if let Some(output_filename) = &args.output_filename {
      output_filename.clone()
    } else {
      input_source.output_path("", &args.output_directory)
    };

    match modify_input_source(&input_source, output_filename, args) {
      Ok(()) => (),

      Err(e) => {
        let task_description = format!("modifying \"{}\"", input_source);

        match e {
          ModifyCommandError::P10Error(e) => e.print(&task_description),
          ModifyCommandError::P10PixelDataTranscodeTransformError(e) => {
            e.print(&task_description);
          }
        }

        return Err(());
      }
    }
  }

  Ok(())
}

fn modify_input_source(
  input_source: &InputSource,
  output_filename: PathBuf,
  args: &ModifyArgs,
) -> Result<(), ModifyCommandError> {
  if output_filename != PathBuf::from("-") {
    if args.in_place {
      println!("Modifying \"{}\" in place …", input_source,);
    } else {
      println!(
        "Modifying \"{}\" => \"{}\" …",
        input_source,
        output_filename.display()
      );
    }

    if !args.in_place && !args.overwrite {
      prompt_to_overwrite_if_exists(&output_filename);
    }
  }

  // Append a random suffix to get a unique name for a temporary output file.
  // This isn't needed when outputting to stdout.
  let (tmp_output_filename, mut temp_file_guard) =
    if output_filename == PathBuf::from("-") {
      (None, None)
    } else {
      let mut rng = rand::rng();
      let random_suffix: String = (0..16)
        .map(|_| char::from(rng.sample(rand::distr::Alphanumeric)))
        .collect();

      let file_name = output_filename.file_name().unwrap_or(OsStr::new(""));
      let file_name =
        format!("{}.{}.tmp", file_name.to_string_lossy(), random_suffix);

      let mut new_path = output_filename.clone();
      new_path.set_file_name(file_name);

      (
        Some(new_path.clone()),
        Some(TempFileRenamer::new(new_path, output_filename.clone())),
      )
    };

  // Create a filter transform for anonymization and tag deletion if needed
  let tags_to_delete = args.delete_tag.clone();
  let anonymize = args.anonymize;
  let filter_transform = if anonymize || !tags_to_delete.is_empty() {
    Some(P10FilterTransform::new(Box::new(
      move |tag, vr, _length, _location| {
        (!anonymize || dcmfx::anonymize::filter_tag(tag, vr))
          && !tags_to_delete.contains(&tag)
      },
    )))
  } else {
    None
  };

  // Create an insert transform that sets '(0028,2110) Lossy Image Compression'
  // if a lossy transfer syntax is being transcoded into
  let insert_transform = if let Some(transfer_syntax) = args.transfer_syntax {
    if transfer_syntax == TransferSyntaxArg::JpegBaseline8Bit
      || transfer_syntax == TransferSyntaxArg::JpegLsLossyNearLossless
      || transfer_syntax == TransferSyntaxArg::Jpeg2k
    {
      let mut lossy_image_compression = DataSet::new();
      lossy_image_compression.insert(
        dictionary::LOSSY_IMAGE_COMPRESSION.tag,
        DataElementValue::new_code_string(&["01"]).unwrap(),
      );

      Some(P10InsertTransform::new(lossy_image_compression))
    } else {
      None
    }
  } else {
    None
  };

  // Setup write config
  let write_config = P10WriteConfig {
    implementation_version_name: args.implementation_version_name.clone(),
    zlib_compression_level: args.zlib_compression_level,
    ..P10WriteConfig::default()
  };

  let input_stream = input_source
    .open_read_stream()
    .map_err(ModifyCommandError::P10Error)?;

  streaming_rewrite(
    input_stream,
    tmp_output_filename.as_ref().unwrap_or(&output_filename),
    write_config,
    filter_transform,
    insert_transform,
    args,
  )?;

  // Rename the temporary file to the desired output filename
  if let Some(temp_file_guard) = &mut temp_file_guard {
    temp_file_guard.commit().map_err(|(when, details)| {
      ModifyCommandError::P10Error(P10Error::FileError { when, details })
    })?;
  }

  Ok(())
}

/// Returns the image data functions to use when transcoding pixel data.
///
/// These are currently able to perform the following alterations to pixel data
/// as it is transcoded:
///
/// - Setting a desired photometric interpretation for the transcoded output.
/// - Sampling of PALETTE_COLOR data into RGB when the output transfer syntax
///   doesn't support PALETTE_COLOR.
/// - Expansion of YBR_FULL_422 to YBR_FULL when the output transfer syntax
///   doesn't support YBR_FULL_422.
/// - Setting a desired planar configuration for the transcoded output.
///
fn get_transcode_image_data_functions(
  output_transfer_syntax: &'static TransferSyntax,
  photometric_interpretation_monochrome_arg: Option<
    PhotometricInterpretationMonochromeArg,
  >,
  photometric_interpretation_color_arg: Option<
    PhotometricInterpretationColorArg,
  >,
  planar_configuration_arg: Option<PlanarConfigurationArg>,
) -> TranscodeImageDataFunctions {
  let process_image_pixel_module =
    move |image_pixel_module: &mut ImagePixelModule| {
      // For grayscale pixel data, the photometric interpretation, if set, can
      // be either MONOCHROME1 or MONOCHROME2
      if image_pixel_module.is_monochrome() {
        if let Some(photometric_interpretation_monochrome_arg) =
          photometric_interpretation_monochrome_arg
        {
          if let Some(photometric_interpretation) =
            photometric_interpretation_monochrome_arg
              .as_photometric_interpretation()
          {
            image_pixel_module
              .set_photometric_interpretation(photometric_interpretation);
          }
        }
      }

      if image_pixel_module.is_color() {
        // If a photometric interpretation has been explicitly specified
        // then use it for the output
        if let Some(photometric_interpretation_color_arg) =
          photometric_interpretation_color_arg
        {
          if let Some(photometric_interpretation) =
            photometric_interpretation_color_arg.as_photometric_interpretation()
          {
            image_pixel_module
              .set_photometric_interpretation(photometric_interpretation);
          }
        } else {
          // If the input is palette color and the output transfer syntax
          // doesn't support palette color then expand to RGB by default
          if image_pixel_module
            .photometric_interpretation()
            .is_palette_color()
            && !transfer_syntax_arg::supports_palette_color(
              output_transfer_syntax,
            )
          {
            image_pixel_module
              .set_photometric_interpretation(PhotometricInterpretation::Rgb);
          }

          // If the input is YBR_FULL_422 and the output transfer syntax
          // doesn't support YBR_FULL_422 then expand to YBR_FULL by default
          if image_pixel_module
            .photometric_interpretation()
            .is_ybr_full_422()
            && !transfer_syntax_arg::supports_ybr_full_422(
              output_transfer_syntax,
            )
          {
            image_pixel_module.set_photometric_interpretation(
              PhotometricInterpretation::YbrFull,
            );
          }

          match *output_transfer_syntax {
            // When transcoding to JPEG Baseline 8-bit default to YBR if the
            // incoming data is RGB
            transfer_syntax::JPEG_BASELINE_8BIT => {
              if image_pixel_module.photometric_interpretation().is_rgb() {
                image_pixel_module.set_photometric_interpretation(
                  PhotometricInterpretation::YbrFull,
                );
              }
            }

            // When transcoding to JPEG 2000 Lossless Only default to YBR_RCT
            // unless the incoming data is PALETTE_COLOR
            transfer_syntax::JPEG_2K_LOSSLESS_ONLY
              if !image_pixel_module
                .photometric_interpretation()
                .is_palette_color() =>
            {
              image_pixel_module.set_photometric_interpretation(
                PhotometricInterpretation::YbrRct,
              )
            }

            // When transcoding to JPEG 2000 Lossy default to YBR_ICT
            transfer_syntax::JPEG_2K => image_pixel_module
              .set_photometric_interpretation(
                PhotometricInterpretation::YbrIct,
              ),

            _ => (),
          }
        }

        // If a planar configuration has been explicitly specified then use it
        // for the output
        if let Some(planar_configuration_arg) = planar_configuration_arg {
          if transfer_syntax_arg::supports_planar_configuration(
            output_transfer_syntax,
          ) {
            image_pixel_module
              .set_planar_configuration(planar_configuration_arg.into());
          }
        }
      }

      Ok(())
    };

  let process_monochrome_image =
    move |image: &mut MonochromeImage,
          image_pixel_module: &ImagePixelModule| {
      // Convert to MONOCHROME1/MONOCHROME1 based on the output photometric
      // interpretation
      match image_pixel_module.photometric_interpretation() {
        PhotometricInterpretation::Monochrome1 => {
          if !image.is_monochrome1() {
            image.change_monochrome_representation();
          }
        }

        PhotometricInterpretation::Monochrome2 => {
          if image.is_monochrome1() {
            image.change_monochrome_representation();
          }
        }

        _ => (),
      }

      Ok(())
    };

  let process_color_image =
    move |image: &mut ColorImage, image_pixel_module: &ImagePixelModule| {
      // Convert palette color to RGB if the output image pixel module isn't in
      // palette color
      if image.is_palette_color()
        && !image_pixel_module
          .photometric_interpretation()
          .is_palette_color()
      {
        image.convert_palette_color_to_rgb();
      }

      let photometric_interpretation =
        image_pixel_module.photometric_interpretation();

      // If the output image pixel module is using RGB, or needs RGB color data
      // as its input, then convert the color image to RGB
      if photometric_interpretation.is_rgb()
        || photometric_interpretation.is_ybr_ict()
        || photometric_interpretation.is_ybr_rct()
      {
        image.convert_to_rgb_color_space()
      }

      // If the output image pixel module is using YBR then convert the color
      // image to YBR full
      if photometric_interpretation.is_ybr_full() {
        image.convert_to_ybr_color_space();
      }

      // If the output image pixel module is using YBR 422 then convert the
      // color image to YBR 422
      if photometric_interpretation.is_ybr_full_422() {
        image.convert_to_ybr_422_color_space().map_err(|_| {
          P10PixelDataTranscodeTransformError::DataError(
            DataError::new_value_invalid(
              "Can't convert to YBR 422 because width is odd".to_string(),
            ),
          )
        })?;
      }

      Ok(())
    };

  TranscodeImageDataFunctions {
    process_image_pixel_module: Box::new(process_image_pixel_module),
    process_monochrome_image: Box::new(process_monochrome_image),
    process_color_image: Box::new(process_color_image),
  }
}

/// Rewrites by streaming the tokens of the DICOM P10 straight to the output
/// file.
///
fn streaming_rewrite(
  mut input_stream: Box<dyn Read>,
  output_filename: &PathBuf,
  write_config: P10WriteConfig,
  mut filter_transform: Option<P10FilterTransform>,
  mut insert_transform: Option<P10InsertTransform>,
  args: &ModifyArgs,
) -> Result<(), ModifyCommandError> {
  // Open output stream
  let mut output_stream: Box<dyn Write> =
    utils::open_output_stream(output_filename, None, false)
      .map_err(ModifyCommandError::P10Error)?;

  // Create read and write contexts
  let mut p10_read_context = P10ReadContext::new();
  p10_read_context.set_config(&P10ReadConfig {
    max_token_size: 256 * 1024,
    ..P10ReadConfig::default()
  });
  let mut p10_write_context = P10WriteContext::new();
  p10_write_context.set_config(&write_config);

  let mut pixel_data_transcode_transform = None;

  // Stream P10 tokens from the input stream to the output stream
  loop {
    // Read the next P10 tokens from the input stream
    let mut tokens = dcmfx::p10::read_tokens_from_stream(
      &mut input_stream,
      &mut p10_read_context,
    )
    .map_err(ModifyCommandError::P10Error)?;

    // If transcoding is active, setup a transcode transform when the File Meta
    // Information token is received
    if let Some(transfer_syntax_arg) = args.transfer_syntax {
      for token in tokens.iter() {
        if let P10Token::FileMetaInformation { data_set } = token {
          let output_transfer_syntax =
            transfer_syntax_arg.as_transfer_syntax().unwrap_or_else(|| {
              data_set
                .get_transfer_syntax()
                .unwrap_or(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN)
            });

          let mut pixel_data_encode_config = PixelDataEncodeConfig::new();
          pixel_data_encode_config.set_quality(args.quality.unwrap_or(85));
          pixel_data_encode_config
            .set_zlib_compression_level(args.zlib_compression_level);

          pixel_data_transcode_transform =
            Some(P10PixelDataTranscodeTransform::new(
              output_transfer_syntax,
              pixel_data_encode_config,
              Some(get_transcode_image_data_functions(
                output_transfer_syntax,
                args.photometric_interpretation_monochrome,
                args.photometric_interpretation_color,
                args.planar_configuration,
              )),
            ));
        }
      }
    }

    // Pass tokens through the pixel data transcode transform if one is active
    if let Some(pixel_data_transcode_transform) =
      pixel_data_transcode_transform.as_mut()
    {
      let mut new_tokens = vec![];

      for token in tokens.iter() {
        new_tokens.extend(
          pixel_data_transcode_transform
            .add_token(token)
            .map_err(ModifyCommandError::P10PixelDataTranscodeTransformError)?,
        );
      }

      tokens = new_tokens
    }

    // Pass tokens through the filter transform if one is specified
    let tokens = if let Some(filter_transform) = filter_transform.as_mut() {
      tokens.into_iter().try_fold(vec![], |mut acc, token| {
        if filter_transform
          .add_token(&token)
          .map_err(ModifyCommandError::P10Error)?
        {
          acc.push(token);
        }

        Ok(acc)
      })
    } else {
      Ok(tokens)
    }?;

    // Pass tokens through the insert transform if one is specified
    let tokens = if let Some(insert_transform) = insert_transform.as_mut() {
      tokens.into_iter().try_fold(vec![], |mut acc, token| {
        acc.extend(
          insert_transform
            .add_token(&token)
            .map_err(ModifyCommandError::P10Error)?,
        );

        Ok(acc)
      })
    } else {
      Ok(tokens)
    }?;

    // Write tokens to the output stream
    let ended = dcmfx::p10::write_tokens_to_stream(
      &tokens,
      &mut output_stream,
      &mut p10_write_context,
    )
    .map_err(ModifyCommandError::P10Error)?;

    // Stop when the end token is received
    if ended {
      break;
    }
  }

  Ok(())
}
