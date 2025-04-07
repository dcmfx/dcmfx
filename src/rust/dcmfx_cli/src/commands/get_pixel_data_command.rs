use std::time::Duration;
use std::{fs::File, io::Write, path::Path, path::PathBuf};

use clap::{Args, ValueEnum};

use dcmfx::core::*;
use dcmfx::p10::*;
use dcmfx::pixel_data::{
  iods::{CineModule, MultiFrameModule},
  *,
};

use crate::InputSource;
use crate::mp4_encoder::{
  LogLevel, Mp4Codec, Mp4CompressionPreset, Mp4Encoder, Mp4EncoderConfig,
  Mp4PixelFormat,
};

pub const ABOUT: &str = "Extracts pixel data from DICOM P10 files, writing it \
  to image and video files";

#[derive(Args)]
pub struct GetPixelDataArgs {
  #[clap(
    required = true,
    help = "The names of the DICOM P10 files to extract pixel data from. \
      Specify '-' to read from stdin."
  )]
  input_filenames: Vec<PathBuf>,

  #[arg(
    long,
    short,
    help = "The prefix for output files. When writing individual frames this is
      suffixed with a 4-digit frame number, and an appropriate file extension. \
      This option is only valid when a single input filename is specified. By \
      default, the output prefix is the input filename."
  )]
  output_prefix: Option<PathBuf>,

  #[arg(
    long,
    short,
    value_enum,
    help = "The output format for the pixel data.",
    default_value_t = OutputFormat::Raw
  )]
  format: OutputFormat,

  #[arg(
    long,
    help = "When the output format is 'jpg', specifies the quality level in \
      the range 0-100.",
    default_value_t = 85
  )]
  jpg_quality: u8,

  #[arg(
    long,
    help_heading = "MP4 Encoding",
    help = "When the output format is 'mp4', specifies the codec to use for \
      encoding the video stream.",
    default_value_t = Mp4Codec::Libx264
  )]
  mp4_codec: Mp4Codec,

  #[arg(
    long,
    help_heading = "MP4 Encoding",
    help = "When the output format is 'mp4', specifies the constant rate \
      factor in the range 0-51. Smaller values give higher quality and larger \
      file sizes. Larger values give lower quality and smaller file sizes.\n\
      \n\
      The default CRF for libx264 is 18.\n\
      The default CRF for libx265 is 6.",
    value_parser = clap::value_parser!(u32).range(0..=51),
  )]
  mp4_crf: Option<u32>,

  #[arg(
    long,
    help_heading = "MP4 Encoding",
    help = "When the output format is 'mp4', specifies the preset to use that \
      controls encoding speed vs compression efficiency.",
    default_value_t = Mp4CompressionPreset::Medium
  )]
  mp4_preset: Mp4CompressionPreset,

  #[arg(
    long,
    help_heading = "MP4 Encoding",
    help = "When the output format is 'mp4', specifies how video data is \
      stored.",
    default_value_t = Mp4PixelFormat::Yuv420p
  )]
  mp4_pixel_format: Mp4PixelFormat,

  #[arg(
    long,
    help_heading = "MP4 Encoding",
    help = "When the output format is 'mp4', specifies the frame rate of the \
      MP4 file. This overrides any frame rate or frame duration information \
      contained in the Cine Module IOD in the input DICOM P10 file. The \
      fallback frame rate is 1 frame per second."
  )]
  mp4_frame_rate: Option<f64>,

  #[arg(
    long,
    help_heading = "MP4 Encoding",
    help = "When the output format is 'mp4', specifies the output log level \
      for FFmpeg.",
    default_value_t = LogLevel::Error
  )]
  mp4_log_level: LogLevel,

  #[arg(
    long,
    short = 'w',
    num_args=2..=2,
    value_parser = clap::value_parser!(f32),
    value_names = ["WINDOW_CENTER", "WINDOW_WIDTH"],
    help = "For grayscale DICOM P10 files, when the output format is 'jpg' or \
      'png', specifies a VOI LUT's window center and width to use instead of \
      the VOI LUT specified in the input DICOM file."
  )]
  voi_window: Option<Vec<f32>>,

  #[arg(
    long,
    value_enum,
    help = "For grayscale DICOM P10 files, when the output format is 'jpg' or \
      'png', specifies the well-known color palette to apply to visualize the \
      grayscale image in color."
  )]
  color_palette: Option<StandardColorPaletteArg>,

  #[arg(
    long = "overlays",
    help = "Whether to render overlays present in the DICOM. Overlays are \
      rendered on top of the pixel data. Each overlay is rendered using a \
      different color",
    default_value_t = false
  )]
  render_overlays: bool,

  #[clap(
    long = "force",
    help = "Overwrite files without prompting",
    default_value_t = false
  )]
  force_overwrite: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
enum OutputFormat {
  /// Reads the pixel data for each frame and writes it out exactly as it is
  /// stored in the DICOM P10 file without any alteration. A sensible file
  /// extension is selected based on the file's DICOM transfer syntax.
  Raw,

  /// Decodes the pixel data and writes each frame to an 8-bit PNG image.
  Png,

  /// Decodes the pixel data and writes each frame to a PNG image. If the pixel
  /// data bit depth is greater than 8-bit then the PNG will use 16-bit color
  /// depth.
  Png16,

  /// Decodes the pixel data and writes each frame to a JPG image. The JPG
  /// quality can be controlled with the --jpg-quality argument.
  Jpg,

  /// Decodes the pixel data and writes the frames to an MP4 file. The MP4
  /// codec, quality, preset, and other settings can be controlled with the
  /// --mp4-* arguments.
  Mp4,

  /// Decodes the pixel data and writes each frame to a lossless WebP image.
  Webp,
}

impl OutputFormat {
  fn is_16bit(&self) -> bool {
    *self == Self::Png16
  }
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
enum StandardColorPaletteArg {
  HotIron,
  Pet,
  HotMetalBlue,
  Pet20Step,
  Spring,
  Summer,
  Fall,
  Winter,
}

impl StandardColorPaletteArg {
  fn color_palette(&self) -> &'static ColorPalette {
    match self {
      StandardColorPaletteArg::HotIron => StandardColorPalette::HotIron,
      StandardColorPaletteArg::Pet => StandardColorPalette::Pet,
      StandardColorPaletteArg::HotMetalBlue => {
        StandardColorPalette::HotMetalBlue
      }
      StandardColorPaletteArg::Pet20Step => StandardColorPalette::Pet20Step,
      StandardColorPaletteArg::Spring => StandardColorPalette::Spring,
      StandardColorPaletteArg::Summer => StandardColorPalette::Summer,
      StandardColorPaletteArg::Fall => StandardColorPalette::Fall,
      StandardColorPaletteArg::Winter => StandardColorPalette::Winter,
    }
    .color_palette()
  }
}

#[allow(clippy::enum_variant_names)]
enum GetPixelDataError {
  P10Error(P10Error),
  DataError(DataError),
  ImageError(image::ImageError),
  FFmpegError(ffmpeg_next::Error),
}

pub fn run(args: &GetPixelDataArgs) -> Result<(), ()> {
  let input_sources = crate::get_input_sources(&args.input_filenames);

  if input_sources.contains(&InputSource::Stdin) && args.output_prefix.is_none()
  {
    eprintln!("When reading from stdin --output-prefix must be specified");
    return Err(());
  }

  if input_sources.len() > 1 && args.output_prefix.is_some() {
    eprintln!(
      "When there are multiple input files --output-prefix must not be \
       specified"
    );
    return Err(());
  }

  for input_source in input_sources {
    match get_pixel_data_from_input_source(&input_source, args) {
      Ok(()) => (),

      Err(e) => {
        let task_description =
          format!("extracting pixel data from \"{}\"", input_source);

        match e {
          GetPixelDataError::DataError(e) => e.print(&task_description),
          GetPixelDataError::P10Error(e) => e.print(&task_description),
          GetPixelDataError::ImageError(e) => {
            let lines = vec![
              format!("DICOM image error {}", task_description),
              "".to_string(),
              format!("  Error: {}", e),
            ];

            error::print_error_lines(&lines);
          }
          GetPixelDataError::FFmpegError(e) => {
            let lines = vec![
              format!("FFmpeg encoding error {}", task_description),
              "".to_string(),
              format!("  Error: {}", e),
            ];

            error::print_error_lines(&lines);
          }
        }

        return Err(());
      }
    }
  }

  Ok(())
}

fn get_pixel_data_from_input_source(
  input_source: &InputSource,
  args: &GetPixelDataArgs,
) -> Result<(), GetPixelDataError> {
  let mut stream = input_source
    .open_read_stream()
    .map_err(GetPixelDataError::P10Error)?;

  let output_prefix = args
    .output_prefix
    .clone()
    .unwrap_or_else(|| input_source.path().unwrap().clone());

  // Create read context with a small max token size to keep memory usage low
  let mut read_context = P10ReadContext::new();
  read_context.set_config(&P10ReadConfig {
    max_token_size: 1024 * 1024,
    ..P10ReadConfig::default()
  });

  let mut p10_pixel_data_frame_filter = P10PixelDataFrameFilter::new();

  let mut pixel_data_renderer_transform = if args.format == OutputFormat::Raw {
    None
  } else {
    Some(PixelDataRenderer::custom_type_transform())
  };

  let mut overlays_transform = if args.render_overlays {
    Some(Overlays::custom_type_transform())
  } else {
    None
  };

  let (mut cine_module_transform, mut multiframe_module_transform) =
    if args.format == OutputFormat::Mp4 {
      (
        Some(P10CustomTypeTransform::<CineModule>::new_for_iod_module()),
        Some(P10CustomTypeTransform::<MultiFrameModule>::new_for_iod_module()),
      )
    } else {
      (None, None)
    };

  let mut output_extension = match args.format {
    OutputFormat::Raw => "",
    OutputFormat::Png | OutputFormat::Png16 => ".png",
    OutputFormat::Jpg => ".jpg",
    OutputFormat::Mp4 => ".mp4",
    OutputFormat::Webp => ".webp",
  };

  let mut mp4_encoder: Option<Mp4Encoder> = None;

  loop {
    // Read the next tokens from the input stream
    let tokens =
      dcmfx::p10::read_tokens_from_stream(&mut stream, &mut read_context)
        .map_err(GetPixelDataError::P10Error)?;

    for token in tokens.iter() {
      // For raw output, determine the output extension from the transfer syntax
      if args.format == OutputFormat::Raw {
        if let P10Token::FileMetaInformation { data_set } = token {
          output_extension = file_extension_for_transfer_syntax(
            data_set
              .get_transfer_syntax()
              .unwrap_or(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN),
          );
        }
      }

      // Pass token through the transforms to extract relevant data
      add_token_to_transform(&mut pixel_data_renderer_transform, token)?;
      add_token_to_transform(&mut overlays_transform, token)?;
      add_token_to_transform(&mut cine_module_transform, token)?;
      add_token_to_transform(&mut multiframe_module_transform, token)?;

      let pixel_data_renderer: &mut Option<PixelDataRenderer> =
        if let Some(pixel_data_renderer_transform) =
          pixel_data_renderer_transform.as_mut()
        {
          pixel_data_renderer_transform.get_output_mut()
        } else {
          &mut None
        };

      let overlays =
        if let Some(overlays_transform) = overlays_transform.as_mut() {
          overlays_transform.get_output()
        } else {
          None
        };

      // Pass token through the pixel data frame filter, receiving any frames
      // that are now available
      let mut frames =
        p10_pixel_data_frame_filter
          .add_token(token)
          .map_err(|e| match e {
            P10PixelDataFrameFilterError::DataError(e) => {
              GetPixelDataError::DataError(e)
            }
            P10PixelDataFrameFilterError::P10Error(e) => {
              GetPixelDataError::P10Error(e)
            }
          })?;

      // Process available frames
      for frame in frames.iter_mut() {
        if args.format == OutputFormat::Mp4 {
          let pixel_data_renderer = pixel_data_renderer.as_mut().unwrap();

          let cine_module = cine_module_transform
            .as_ref()
            .unwrap()
            .get_output()
            .unwrap();

          let multiframe_module = multiframe_module_transform
            .as_ref()
            .unwrap()
            .get_output()
            .unwrap();

          write_frame_to_mp4_file(
            frame,
            &output_prefix,
            &mut mp4_encoder,
            pixel_data_renderer,
            cine_module,
            multiframe_module,
            overlays,
            args,
          )?;
        } else {
          let filename = crate::utils::path_append(
            output_prefix.clone(),
            &format!(".{:04}{}", frame.index(), output_extension),
          );

          if !args.force_overwrite {
            crate::utils::prompt_to_overwrite_if_exists(&filename);
          }

          write_frame_to_image_file(
            &filename,
            frame,
            pixel_data_renderer,
            overlays,
            args,
          )?;
        }
      }

      if *token == P10Token::End {
        if let Some(mp4_encoder) = mp4_encoder.as_mut() {
          println!();

          mp4_encoder
            .finish()
            .map_err(GetPixelDataError::FFmpegError)?;
        }

        return Ok(());
      }
    }
  }
}

/// Writes the data for a single frame of pixel data to an image file.
///
fn write_frame_to_image_file(
  filename: &PathBuf,
  frame: &mut PixelDataFrame,
  pixel_data_renderer: &mut Option<PixelDataRenderer>,
  overlays: Option<&Overlays>,
  args: &GetPixelDataArgs,
) -> Result<(), GetPixelDataError> {
  println!("Writing \"{}\" …", filename.display());

  if args.format == OutputFormat::Raw {
    write_fragments(filename, frame).map_err(|e| {
      GetPixelDataError::P10Error(P10Error::FileError {
        when: "Writing pixel data frame".to_string(),
        details: e.to_string(),
      })
    })?;
  } else {
    let pixel_data_renderer = pixel_data_renderer.as_mut().unwrap();

    let image = frame_to_image(frame, pixel_data_renderer, overlays, args)?;

    let output_file =
      File::create(filename).expect("Failed to create output file");
    let mut output_writer = std::io::BufWriter::new(output_file);

    match args.format {
      OutputFormat::Png | OutputFormat::Png16 => image
        .write_to(&mut output_writer, image::ImageFormat::Png)
        .map_err(GetPixelDataError::ImageError)?,

      OutputFormat::Jpg => image::codecs::jpeg::JpegEncoder::new_with_quality(
        &mut output_writer,
        args.jpg_quality,
      )
      .encode_image(&image)
      .map_err(GetPixelDataError::ImageError)?,

      OutputFormat::Webp => {
        let image = image.into_rgb8();

        image::codecs::webp::WebPEncoder::new_lossless(&mut output_writer)
          .encode(
            &image,
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgb8,
          )
          .map_err(GetPixelDataError::ImageError)?
      }

      OutputFormat::Raw | OutputFormat::Mp4 => unreachable!(),
    }
  }

  Ok(())
}

/// Writes the data for a single frame of pixel data to a file.
///
fn write_fragments(
  filename: &PathBuf,
  frame: &PixelDataFrame,
) -> Result<(), std::io::Error> {
  let mut stream = File::create(filename)?;

  if frame.bit_offset() == 0 {
    for fragment in frame.fragments() {
      stream.write_all(fragment)?;
    }
  } else {
    stream.write_all(&frame.to_bytes())?;
  }

  stream.flush()
}

fn frame_to_image(
  frame: &mut PixelDataFrame,
  pixel_data_renderer: &mut PixelDataRenderer,
  overlays: Option<&Overlays>,
  args: &GetPixelDataArgs,
) -> Result<image::DynamicImage, GetPixelDataError> {
  let mut image: image::DynamicImage =
    if pixel_data_renderer.definition.is_grayscale() {
      let single_channel_image = pixel_data_renderer
        .decode_single_channel_frame(frame)
        .map_err(GetPixelDataError::DataError)?;

      // Apply the VOI override if it's set
      if let Some(voi_window_override) = &args.voi_window {
        pixel_data_renderer.voi_lut = VoiLut {
          luts: vec![],
          windows: vec![VoiWindow::new(
            voi_window_override[0],
            voi_window_override[1],
            "".to_string(),
            VoiLutFunction::LinearExact,
          )],
        };
      }
      // If there's no VOI LUT in the DICOM or specified on the command line
      // then automatically derive one from the content of the first frame and
      // use it for all subsequent frames.
      else if pixel_data_renderer.voi_lut.is_empty() {
        let image = pixel_data_renderer
          .decode_single_channel_frame(frame)
          .map_err(GetPixelDataError::DataError)?;

        if let Some(fallback) = image.fallback_voi_window() {
          pixel_data_renderer.voi_lut.windows.push(fallback);
        }
      }

      // For 16-bit outputs emit a Luma16 buffer if the pixel data can make use
      // of it. A color palette implies 8-bit output as color palettes always
      // output 8-bit.
      if args.format.is_16bit()
        && pixel_data_renderer.definition.bits_stored() > 8
        && args.color_palette.is_none()
      {
        single_channel_image
          .to_gray_u16_image(
            &pixel_data_renderer.modality_lut,
            &pixel_data_renderer.voi_lut,
          )
          .into()
      }
      // If there is an active color palette then use it and output the
      // resulting RGB8
      else if let Some(color_palette) = args.color_palette {
        pixel_data_renderer
          .render_single_channel_image(
            &single_channel_image,
            Some(color_palette.color_palette()),
          )
          .into()
      }
      // Otherwise, emit a Luma8 image
      else {
        single_channel_image
          .to_gray_u8_image(
            &pixel_data_renderer.modality_lut,
            &pixel_data_renderer.voi_lut,
          )
          .into()
      }
    } else {
      let image = pixel_data_renderer
        .decode_color_frame(frame)
        .map_err(GetPixelDataError::DataError)?;

      if args.format.is_16bit()
        && pixel_data_renderer.definition.bits_stored() > 8
      {
        image
          .into_rgb_u16_image(&pixel_data_renderer.definition)
          .into()
      } else {
        image
          .into_rgb_u8_image(&pixel_data_renderer.definition)
          .into()
      }
    };

  if let Some(overlays) = overlays {
    // Expand Luma images to RGB because overlays can only be rendered to RGB
    if image.color() == image::ColorType::L8 {
      image = image.to_rgb8().into();
    } else if image.color() == image::ColorType::L16 {
      image = image.to_rgb16().into();
    }

    overlays
      .render_to_rgb_image(&mut image, frame.index())
      .unwrap();
  }

  Ok(image)
}

/// Creates an [`Mp4Encoder`] for a given input source
///
fn create_mp4_encoder(
  output_prefix: &Path,
  pixel_data_renderer: &PixelDataRenderer,
  args: &GetPixelDataArgs,
) -> Result<Mp4Encoder, GetPixelDataError> {
  let mp4_path = crate::utils::path_append(output_prefix.to_path_buf(), ".mp4");

  if !args.force_overwrite {
    crate::utils::prompt_to_overwrite_if_exists(&mp4_path);
  }

  let width = pixel_data_renderer.definition.columns();
  let height = pixel_data_renderer.definition.rows();

  let encoder_config = Mp4EncoderConfig {
    codec: args.mp4_codec,
    crf: args.mp4_crf.unwrap_or(args.mp4_codec.default_crf()),
    preset: args.mp4_preset,
    pixel_format: args.mp4_pixel_format,
    log_level: args.mp4_log_level,
  };

  Mp4Encoder::new(&mp4_path, width, height, encoder_config)
    .map_err(GetPixelDataError::FFmpegError)
}

/// Writes the next frame of pixel data to an MP4 file.
///
#[allow(clippy::too_many_arguments)]
fn write_frame_to_mp4_file(
  frame: &mut PixelDataFrame,
  output_prefix: &Path,
  mp4_encoder: &mut Option<Mp4Encoder>,
  pixel_data_renderer: &mut PixelDataRenderer,
  cine_module: &CineModule,
  multiframe_module: &MultiFrameModule,
  overlays: Option<&Overlays>,
  args: &GetPixelDataArgs,
) -> Result<(), GetPixelDataError> {
  // Respect frame trimming
  if cine_module.is_frame_trimmed(frame.index()) {
    return Ok(());
  }

  // If this is the first frame then the MP4 encoder won't have been created,
  // so create it now
  if mp4_encoder.is_none() {
    *mp4_encoder = Some(create_mp4_encoder(
      output_prefix,
      pixel_data_renderer,
      args,
    )?);
  }

  // Update progress readout
  let start_trim = cine_module.start_trim.unwrap_or(0);
  let progress = (frame.index() + 1 - start_trim) as f64
    / (cine_module.number_of_frames(multiframe_module) as f64);
  print!(
    "\rWriting \"{}\" … {:.1}%",
    mp4_encoder.as_ref().unwrap().path().display(),
    100.0 * progress
  );
  let _ = std::io::stdout().flush();

  // Use the Cine Module to Determine the duration of this frame. This can be
  // overridden by a CLI argument if desired. The fallback value is one second
  // per frame.
  let frame_duration = if let Some(frame_rate) = args.mp4_frame_rate {
    Duration::from_secs_f64(1.0 / frame_rate)
  } else {
    cine_module
      .frame_duration(frame.index(), multiframe_module)
      .unwrap_or(Duration::from_secs(1))
  };

  // Convert the raw frame into an image
  let image = frame_to_image(frame, pixel_data_renderer, overlays, args)?;

  // Add the frame to the MP4 encoder
  mp4_encoder
    .as_mut()
    .unwrap()
    .add_frame(&image.into_rgb8(), frame_duration)
    .map_err(GetPixelDataError::FFmpegError)
}

fn add_token_to_transform<T>(
  transform: &mut Option<P10CustomTypeTransform<T>>,
  token: &P10Token,
) -> Result<(), GetPixelDataError> {
  if let Some(transform) = transform.as_mut() {
    match transform.add_token(token) {
      Ok(()) => Ok(()),
      Err(P10CustomTypeTransformError::DataError(e)) => {
        Err(GetPixelDataError::DataError(e))
      }
      Err(P10CustomTypeTransformError::P10Error(e)) => {
        Err(GetPixelDataError::P10Error(e))
      }
    }
  } else {
    Ok(())
  }
}
