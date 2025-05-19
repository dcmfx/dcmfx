use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::PathBuf;

use clap::{Args, ValueEnum};
use rand::Rng;

use dcmfx::core::*;
use dcmfx::p10::*;
use dcmfx::pixel_data::{
  iods::image_pixel_module::{ImagePixelModule, PhotometricInterpretation},
  *,
};

use crate::utils::prompt_to_overwrite_if_exists;
use crate::{InputSource, transfer_syntax_arg::TransferSyntaxArg, utils};

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
      in lossless compression.",
    default_value_t = 85,
    value_parser = clap::value_parser!(u8).range(1..=100),
  )]
  quality: u8,

  #[clap(
    long,
    help_heading = "Transcoding",
    help = "When transcoding pixel data using --transfer-syntax, this \
      specifies the color space conversion to perform on decoded color images \
      prior to them being encoded for the output transfer syntax.\n\
      \n\
      This option has no effect on the transcoding of monochrome pixel data.\n\
      \n\
      When the output transfer syntax is 'JPEG Baseline 8-bit' the color space \
      conversion defaults to 'ybr'. This is because the 'RGB' photometric \
      interpretation in JPEG isn't always well-supported by older systems, and \
      YBR generally gives more favorable compression ratios.\n\
      \n\
      When the output transfer syntax is a JPEG 2000 transfer syntax the color \
      space conversion defaults to 'rgb'. This is because RGB input allows the \
      encoder to target the 'YBR_ICT' and 'YBR_RCT' photometric \
      interpretations, which are generally preferred in JPEG 2000.\n\
      \n\
      For all other output transfer syntaxes no default color space conversion \
      is active.\n\
      \n\
      Note that this option does not directly control the output photometric \
      interpretation in all cases, however it does map through for many output \
      transfer syntaxes. The notable exception is JPEG 2000, where specifying \
      'rgb' will result in either the 'YBR_ICT' or 'YBR_RCT' photometric \
      interpretations being used by the output DICOM P10 file.\n"
  )]
  convert_color_space: Option<ConvertColorSpaceArg>,

  #[arg(
    long,
    help = "\
      The zlib compression level to use when outputting to the 'Deflated \
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
      element in output DICOM P10 files.",
    default_value_t = uids::DCMFX_IMPLEMENTATION_VERSION_NAME.to_string(),
  )]
  implementation_version_name: String,
}

fn validate_data_element_tag(s: &str) -> Result<DataElementTag, String> {
  DataElementTag::from_hex_string(s)
    .map_err(|_| "Invalid data element tag".to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
enum ConvertColorSpaceArg {
  False,
  Rgb,
  Ybr,
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
  let tmp_output_filename = if output_filename == PathBuf::from("-") {
    None
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

    Some(new_path)
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

  // Setup a pixel data transcode transform if an output transfer syntax is
  // specified
  let pixel_data_transcode_transform =
    if let Some(output_transfer_syntax) = args.transfer_syntax {
      let mut pixel_data_encode_config = PixelDataEncodeConfig::new();
      pixel_data_encode_config.set_quality(args.quality);
      pixel_data_encode_config
        .set_zlib_compression_level(args.zlib_compression_level);

      Some(P10PixelDataTranscodeTransform::new(
        output_transfer_syntax.as_transfer_syntax(),
        pixel_data_encode_config,
        get_transcode_image_data_functions(args),
      ))
    } else {
      None
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
    pixel_data_transcode_transform,
  )?;

  // Rename the temporary file to the desired output filename
  if output_filename != PathBuf::from("-") {
    if let Some(tmp_output_filename) = tmp_output_filename {
      std::fs::rename(&tmp_output_filename, &output_filename).map_err(|e| {
        ModifyCommandError::P10Error(P10Error::FileError {
          when: format!(
            "Renaming '{}' to '{}'",
            tmp_output_filename.display(),
            output_filename.display()
          ),
          details: e.to_string(),
        })
      })?;
    }
  }

  Ok(())
}

/// Returns the image data functions to use when transcoding pixel data.
///
/// Currently the only supported modification to pixel data during transcoding
/// is color space conversion and sampling of palette color data when the output
/// transfer syntax doesn't natively support palette color.
///
fn get_transcode_image_data_functions(
  args: &ModifyArgs,
) -> Option<TranscodeImageDataFunctions> {
  // Convert out of color palette data if the target transfer syntax doesn't
  // support color palettes
  let convert_palette_color_to_rgb = match args.transfer_syntax {
    Some(transfer_syntax) => !transfer_syntax.supports_palette_color(),
    None => true,
  };

  // Determine if transcoding to a JPEG Baseline 8-bit transfer syntax
  let is_jpeg_baseline_8bit =
    args.transfer_syntax == Some(TransferSyntaxArg::JpegBaseline8Bit);

  // Determine if transcoding to a JPEG 2000 transfer syntax
  let is_jpeg_2k = match args.transfer_syntax {
    Some(transfer_syntax) => transfer_syntax.as_transfer_syntax().is_jpeg_2k(),
    None => false,
  };

  // Determine what the target color space for conversion is, if any
  let target_color_space = if let Some(color_space) = args.convert_color_space {
    color_space
  } else if is_jpeg_baseline_8bit {
    ConvertColorSpaceArg::Ybr
  } else if is_jpeg_2k {
    ConvertColorSpaceArg::Rgb
  } else {
    ConvertColorSpaceArg::False
  };

  // If a color space conversion is active then setup image data functions to do
  // so. These are called during pixel data transcoding.
  if target_color_space != ConvertColorSpaceArg::False
    || convert_palette_color_to_rgb
  {
    Some(TranscodeImageDataFunctions {
      process_image_pixel_module: Box::new(
        move |image_pixel_module: &mut ImagePixelModule| {
          // Handle palette color conversion to RGB if requested
          if image_pixel_module
            .photometric_interpretation()
            .is_palette_color()
            && convert_palette_color_to_rgb
          {
            image_pixel_module
              .set_photometric_interpretation(PhotometricInterpretation::Rgb);
          }

          let photometric_interpretation =
            image_pixel_module.photometric_interpretation().clone();

          // Handle color space conversion if requested
          if target_color_space == ConvertColorSpaceArg::Rgb {
            if photometric_interpretation.is_ybr_full()
              || photometric_interpretation.is_ybr_full_422()
            {
              image_pixel_module
                .set_photometric_interpretation(PhotometricInterpretation::Rgb);
            }
          } else if target_color_space == ConvertColorSpaceArg::Ybr
            && photometric_interpretation.is_rgb()
          {
            image_pixel_module.set_photometric_interpretation(
              PhotometricInterpretation::YbrFull,
            );
          }
        },
      ),

      process_monochrome_image: Box::new(|_image| {}),

      process_color_image: Box::new(move |image| {
        // Handle palette color conversion to RGB if requested
        if convert_palette_color_to_rgb {
          image.convert_palette_color_to_rgb();
        }

        // Handle color space conversion if requested
        if target_color_space == ConvertColorSpaceArg::Rgb {
          image.convert_to_rgb_color_space()
        } else if target_color_space == ConvertColorSpaceArg::Ybr {
          image.convert_palette_color_to_rgb();
          image.convert_to_ybr_color_space()
        }
      }),
    })
  } else {
    None
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
  mut pixel_data_transcode_transform: Option<P10PixelDataTranscodeTransform>,
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

  // Stream P10 tokens from the input stream to the output stream
  loop {
    // Read the next P10 tokens from the input stream
    let mut tokens = dcmfx::p10::read_tokens_from_stream(
      &mut input_stream,
      &mut p10_read_context,
    )
    .map_err(ModifyCommandError::P10Error)?;

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
