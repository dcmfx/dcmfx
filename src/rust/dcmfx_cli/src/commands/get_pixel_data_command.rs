use std::{fs::File, io::Write, path::Path, path::PathBuf};

use clap::{Args, ValueEnum};
use rayon::prelude::*;

use dcmfx::core::*;
use dcmfx::p10::*;
use dcmfx::pixel_data::{
  PixelDataDecodeError, PixelDataFrame, PixelDataRenderer,
  iods::{
    CineModule, MultiFrameModule, OverlayPlaneModule,
    voi_lut_module::{VoiLutFunction, VoiWindow},
  },
  transforms::{
    CropRect, P10PixelDataFrameTransform, P10PixelDataFrameTransformError,
  },
};

use crate::{
  args::{
    frame_selection_arg::FrameSelection, input_args::InputSource,
    standard_color_palette_arg::StandardColorPaletteArg,
    transform_arg::TransformArg,
  },
  mp4_encoder::{
    LogLevel, Mp4Codec, Mp4CompressionPreset, Mp4Encoder, Mp4EncoderConfig,
    Mp4PixelFormat, ResizeFilter,
  },
  utils,
};

pub const ABOUT: &str = "Extracts pixel data from DICOM P10 files, writing it \
  to image and video files";

#[derive(Args)]
pub struct GetPixelDataArgs {
  #[arg(
    long,
    help = "The number of threads to use to perform work.",
    default_value_t = rayon::current_num_threads()
  )]
  threads: usize,

  #[command(flatten)]
  input: crate::args::input_args::P10InputArgs,

  #[arg(
    long,
    short = 'd',
    help_heading = "Output",
    help = "The directory to write output files into. The names of the output \
      files will be the name of the input file suffixed with a 4-digit frame \
      number, and an appropriate file extension."
  )]
  output_directory: Option<PathBuf>,

  #[arg(
    long,
    help = "Overwrite files without prompting.",
    default_value_t = false
  )]
  overwrite: bool,

  #[arg(
    long,
    short,
    value_enum,
    help_heading = "Output",
    help = "The output format for the pixel data.",
    default_value_t = OutputFormat::Raw
  )]
  format: OutputFormat,

  #[arg(
    long,
    help_heading = "Output",
    help = "Selects specific frames to extract, instead of extracting every \
      frame from the input which is the default behavior. Frame selection can \
      be specified as:\n\
      \n\
      1. Individual frame indices: '0', '1,5,7', '-2,-1'\n\
      2. A range of frames: '2..10', '-10..-5'\n\
      3. An open range of frames: '10..', '-5..'\n\
      \n\
      Negative values are interpreted as offsets from the end of the set of \
      frames."
  )]
  select_frames: Option<FrameSelection>,

  #[arg(
    long,
    help_heading = "Output",
    help = "When the output format is not 'raw', specifies a crop to \
      apply to the frames of image data. The crop is specified as \
      'x,y[,(width_or_right)[,(height_or_bottom)]]'. The last two values are \
      optional, and if positive they specify the width and height of the crop \
      rectangle, however if they are zero or negative then they specify an \
      offset from the right and bottom edges of the pixel data respectively.\n\
      \n\
      The order of image data operations is: crop, transform, resize."
  )]
  crop: Option<CropRect>,

  #[arg(
    long,
    help_heading = "Output",
    help = "When the output format is not 'raw', specifies a transform to \
      apply to the frames of image data.\n\
      \n\
      The order of image data operations is: crop, transform, resize."
  )]
  transform: Option<TransformArg>,

  #[arg(
    long,
    num_args=2..=2,
    value_parser = clap::value_parser!(u32),
    value_names = ["WIDTH", "HEIGHT"],
    help_heading = "Output",
    help = "When the output format is not 'raw', specifies the resolution of \
      output images and videos. If either width or height is zero then it is \
      calculated automatically such that the input aspect ratio is preserved.\n\
      \n\
      The order of image data operations is: crop, transform, resize."
  )]
  resize: Option<Vec<u32>>,

  #[arg(
    long,
    help_heading = "Output",
    help = "The filter to use when resizing images.",
    default_value_t = ResizeFilter::Lanczos3
  )]
  resize_filter: ResizeFilter,

  #[arg(
    long,
    help_heading = "Output",
    help = "When the output format is 'jpg', specifies the quality level in \
      the range 1-100.",
    default_value_t = 85,
    value_parser = clap::value_parser!(u8).range(1..=100),
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
    help = "Custom parameters to pass to the codec that allow fine control \
      over its operation. Refer to the documentation for the active codec for \
      further details."
  )]
  mp4_codec_params: Option<String>,

  #[arg(
    long,
    help_heading = "MP4 Encoding",
    help = "When the output format is 'mp4', specifies the constant rate \
      factor in the range 0-51. Smaller values give higher quality and larger \
      file sizes. Larger values give lower quality and smaller file sizes.",
    value_parser = clap::value_parser!(u32).range(0..=51),
    default_value_t = 18
  )]
  mp4_crf: u32,

  #[arg(
    long,
    help_heading = "MP4 Encoding",
    help = "When the output format is 'mp4', specifies the preset to use that \
      controls encoding speed vs compression efficiency.",
    default_value_t = Mp4CompressionPreset::Slow
  )]
  mp4_preset: Mp4CompressionPreset,

  #[arg(
    long,
    help_heading = "MP4 Encoding",
    help = "When the output format is 'mp4', specifies the sampling rate of \
      chroma information and whether to encode in 10-bit or 12-bit. 12-bit is \
      only supported by libx265. Some pixel formats may have more limited \
      playback support depending on the player and hardware.\n\
      \n\
      The default pixel format for libx264 is yuv420p.\n\
      The default pixel format for libx265 is yuv420p10."
  )]
  mp4_pixel_format: Option<Mp4PixelFormat>,

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
    help_heading = "Output",
    help = "For grayscale DICOM P10 files, when the output format is 'jpg' or \
      'png', specifies a VOI LUT's window center and width to use instead of \
      the VOI LUT specified in the input DICOM file."
  )]
  voi_window: Option<Vec<f32>>,

  #[arg(
    long,
    value_enum,
    help_heading = "Output",
    help = "For grayscale DICOM P10 files, when the output format is 'jpg' or \
      'png', specifies the well-known color palette to apply to visualize the \
      grayscale image in color."
  )]
  color_palette: Option<StandardColorPaletteArg>,

  #[arg(
    long = "overlays",
    help_heading = "Output",
    help = "Whether to render overlays present in the DICOM. Overlays are \
      rendered on top of the pixel data. Each overlay is rendered using a \
      different color",
    default_value_t = false
  )]
  render_overlays: bool,

  #[command(flatten)]
  decoder: crate::args::decoder_args::DecoderArgs,
}

impl GetPixelDataArgs {
  /// Given an input image's width and height, returns the dimensions it should
  /// be resized to. Returns `None` if no resize is active.
  ///
  fn new_dimensions(&self, width: u32, height: u32) -> Option<(u32, u32)> {
    let resize = self.resize.as_ref()?;

    let mut new_width = resize[0];
    let mut new_height = resize[1];

    let aspect_ratio = f64::from(width) / f64::from(height);

    // If the requested width or height is zero then calculate the correct value
    // based on the aspect ratio of the input
    if new_width == 0 {
      new_width = (f64::from(new_height) * aspect_ratio) as u32;
    } else if new_height == 0 {
      new_height = (f64::from(new_width) / aspect_ratio) as u32;
    }

    Some((new_width, new_height))
  }

  /// Returns whether the output format is HDR, i.e. supports more than 8 bits
  /// per color/grayscale component.
  ///
  fn is_output_hdr(&self) -> bool {
    self.format == OutputFormat::Png16
      || self.format == OutputFormat::Mp4
        && self.mp4_pixel_format_to_use().is_hdr()
  }

  /// Returns the MP4 pixel format to use, which has a different default
  /// depending on the codec.
  ///
  fn mp4_pixel_format_to_use(&self) -> Mp4PixelFormat {
    if let Some(pixel_format) = self.mp4_pixel_format {
      pixel_format
    } else {
      match self.mp4_codec {
        Mp4Codec::Libx264 => Mp4PixelFormat::Yuv420p,
        Mp4Codec::Libx265 => Mp4PixelFormat::Yuv420p10,
      }
    }
  }
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
  /// data bit depth is greater than 8-bit then the PNG will be 16-bit,
  /// otherwise it will be 8-bit.
  Png16,

  /// Decodes the pixel data and writes each frame to a JPG image. The JPG
  /// quality can be controlled with the --jpg-quality argument.
  Jpg,

  /// Decodes the pixel data and writes the frames to an MP4 file. The MP4
  /// codec, quality, preset, and other settings can be controlled with the
  /// --mp4-* arguments.
  Mp4,
}

#[allow(clippy::enum_variant_names)]
enum GetPixelDataError {
  P10Error(P10Error),
  DataError(DataError),
  PixelDataDecodeError(PixelDataDecodeError),
  ImageError(image::ImageError),
  FFmpegError(String),
  OtherError(String),
}

pub fn run(args: &mut GetPixelDataArgs) -> Result<(), ()> {
  crate::validate_output_args(&None, &args.output_directory);

  let input_sources = args.input.base.create_iterator();

  let result = utils::create_thread_pool(args.threads).install(move || {
    input_sources.par_bridge().try_for_each(|input_source| {
      if args.input.ignore_invalid && !input_source.is_dicom_p10() {
        return Ok(());
      }

      let output_prefix = input_source.output_path("", &args.output_directory);

      match get_pixel_data_from_input_source(&input_source, output_prefix, args)
      {
        Ok(()) => Ok(()),

        Err(e) => {
          let task_description =
            format!("extracting pixel data from \"{input_source}\"");

          Err(match e {
            GetPixelDataError::DataError(e) => e.to_lines(&task_description),
            GetPixelDataError::P10Error(e) => e.to_lines(&task_description),
            GetPixelDataError::PixelDataDecodeError(e) => {
              e.to_lines(&task_description)
            }
            GetPixelDataError::ImageError(e) => vec![
              format!("Image error {}", task_description),
              "".to_string(),
              format!("  Error: {}", e),
            ],

            GetPixelDataError::FFmpegError(e) => vec![
              format!("FFmpeg encoding error {}", task_description),
              "".to_string(),
              format!("  Error: {}", e),
            ],
            GetPixelDataError::OtherError(s) => vec![
              format!("Error {}", task_description),
              "".to_string(),
              format!("  Error: {}", s),
            ],
          })
        }
      }
    })
  });

  match result {
    Ok(()) => Ok(()),

    Err(lines) => {
      error::print_error_lines(&lines);
      Err(())
    }
  }
}

fn get_pixel_data_from_input_source(
  input_source: &InputSource,
  output_prefix: PathBuf,
  args: &GetPixelDataArgs,
) -> Result<(), GetPixelDataError> {
  let mut stream = input_source
    .open_read_stream()
    .map_err(GetPixelDataError::P10Error)?;

  // Create read context with a small max token size to keep memory usage low
  let read_config = args.input.p10_read_config().max_token_size(1024 * 1024);
  let mut read_context = P10ReadContext::new(Some(read_config));

  let mut p10_pixel_data_frame_transform = P10PixelDataFrameTransform::new();

  let mut pixel_data_renderer_transform = if args.format == OutputFormat::Raw {
    None
  } else {
    Some(P10CustomTypeTransform::<PixelDataRenderer>::new_for_iod_module())
  };

  let mut overlay_plane_module_transform = if args.render_overlays {
    Some(P10CustomTypeTransform::<OverlayPlaneModule>::new_for_iod_module())
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
  };

  let mut mp4_encoder: Option<Mp4Encoder> = None;

  loop {
    // Read the next tokens from the input stream
    let tokens =
      dcmfx::p10::read_tokens_from_stream(&mut stream, &mut read_context, None)
        .map_err(GetPixelDataError::P10Error)?;

    for token in tokens.iter() {
      // For raw output, determine the output extension from the transfer syntax
      if args.format == OutputFormat::Raw
        && let P10Token::FileMetaInformation { data_set } = token
      {
        let ts = data_set
          .get_transfer_syntax()
          .unwrap_or(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN);

        output_extension =
          dcmfx::pixel_data::file_extension_for_transfer_syntax(ts);
      }

      // Pass token through the transforms to extract relevant data
      add_token_to_p10_transform(&mut pixel_data_renderer_transform, token)?;
      add_token_to_p10_transform(&mut overlay_plane_module_transform, token)?;
      add_token_to_p10_transform(&mut cine_module_transform, token)?;
      add_token_to_p10_transform(&mut multiframe_module_transform, token)?;

      let pixel_data_renderer: &mut Option<PixelDataRenderer> =
        if let Some(pixel_data_renderer_transform) =
          pixel_data_renderer_transform.as_mut()
        {
          let pixel_data_renderer =
            pixel_data_renderer_transform.get_output_mut();

          if let Some(pixel_data_renderer) = pixel_data_renderer {
            pixel_data_renderer.decode_config =
              args.decoder.pixel_data_decode_config();
          }

          pixel_data_renderer
        } else {
          &mut None
        };

      let overlay_plane_module = if let Some(overlay_plane_module) =
        overlay_plane_module_transform.as_mut()
      {
        overlay_plane_module.get_output()
      } else {
        None
      };

      // Pass token through the pixel data frame transform, receiving any frames
      // that are now available
      let mut frames = p10_pixel_data_frame_transform
        .add_token(token)
        .map_err(|e| match e {
          P10PixelDataFrameTransformError::DataError(e) => {
            GetPixelDataError::DataError(e)
          }
          P10PixelDataFrameTransformError::P10Error(e) => {
            GetPixelDataError::P10Error(e)
          }
        })?;

      let number_of_frames =
        p10_pixel_data_frame_transform.get_number_of_frames();

      // Process available frames
      for frame in frames.iter_mut() {
        let frame_index = frame.index().unwrap();

        // If selecting a subset of frames, only export this frame if is
        // selected
        let mut is_frame_selected = true;
        if let Some(frame_selection) = args.select_frames.as_ref() {
          is_frame_selected =
            frame_selection.contains(frame_index, number_of_frames);
        }

        if is_frame_selected {
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
              overlay_plane_module,
              args,
            )?;
          } else {
            let filename = crate::utils::path_append(
              output_prefix.clone(),
              &format!(".{:04}{}", frame.index().unwrap(), output_extension),
            );

            if !args.overwrite {
              crate::utils::error_if_exists(&filename);
            }

            write_frame_to_image_file(
              &filename,
              frame,
              pixel_data_renderer,
              overlay_plane_module,
              args,
            )?;
          }
        }

        // If selecting a subset of frames, stop once they're all done
        if let Some(frame_selection) = args.select_frames.as_ref()
          && frame_selection.is_complete(frame_index, number_of_frames)
        {
          break;
        }
      }

      if *token == P10Token::End {
        if let Some(mp4_encoder) = mp4_encoder.as_mut() {
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
  overlay_plane_module: Option<&OverlayPlaneModule>,
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

    let image = frame_to_final_image(
      frame,
      pixel_data_renderer,
      overlay_plane_module,
      args,
    )?;

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
    for fragment in frame.chunks() {
      stream.write_all(fragment)?;
    }
  } else {
    stream.write_all(&frame.to_bytes())?;
  }

  stream.flush()
}

/// Turns a raw frame of pixel data into an [`image::DynamicImage`] with
/// all alterations performed, including overlay rendering and any active
/// transform or resize.
///
fn frame_to_final_image(
  frame: &mut PixelDataFrame,
  pixel_data_renderer: &mut PixelDataRenderer,
  overlay_plane_module: Option<&OverlayPlaneModule>,
  args: &GetPixelDataArgs,
) -> Result<image::DynamicImage, GetPixelDataError> {
  let mut image = frame_to_dynamic_image(frame, pixel_data_renderer, args)?;

  // Render the overlays, if present
  if let Some(overlay_plane_module) = overlay_plane_module {
    // Expand Luma images to RGB because overlays can only be rendered on RGB
    if image.color() == image::ColorType::L8 {
      image = image.to_rgb8().into();
    } else if image.color() == image::ColorType::L16 {
      image = image.to_rgb16().into();
    }

    overlay_plane_module
      .render_to_rgb_image(&mut image, frame.index().unwrap())
      .unwrap();
  }

  // Apply the image crop, if specified
  if let Some(crop) = args.crop {
    let (cropped_height, cropped_width) =
      crop.apply(image.height() as u16, image.width() as u16);

    image = image.crop(
      crop.left.into(),
      crop.top.into(),
      cropped_width.into(),
      cropped_height.into(),
    );
  }

  // Apply the image transform, if specified
  if let Some(transform) = args.transform {
    image.apply_orientation(transform.orientation());
  }

  // Apply the image resize, if specified. Note that no resize is performed here
  // when outputting to an MP4 because in that case FFmpeg is used to do the
  // resize, which is faster.
  if args.format != OutputFormat::Mp4
    && let Some((new_width, new_height)) =
      args.new_dimensions(image.width(), image.height())
  {
    image = image.resize_exact(
      new_width,
      new_height,
      args.resize_filter.filter_type(),
    );
  }

  Ok(image)
}

/// Turns a raw frame of pixel data into an [`image::DynamicImage`], applying
/// the grayscale pipeline to monochrome images to reach a final display value.
/// The most optimal storage format will be used, e.g. 8-bit/16-bit, and
/// grayscale will be returned when possible for monochrome input frames.
///
fn frame_to_dynamic_image(
  frame: &mut PixelDataFrame,
  pixel_data_renderer: &mut PixelDataRenderer,
  args: &GetPixelDataArgs,
) -> Result<image::DynamicImage, GetPixelDataError> {
  if pixel_data_renderer.image_pixel_module.is_monochrome() {
    let monochrome_image = pixel_data_renderer
      .decode_monochrome_frame(frame)
      .map_err(GetPixelDataError::PixelDataDecodeError)?;

    // Apply the VOI override if it's set
    if let Some(voi_window_override) = &args.voi_window {
      pixel_data_renderer
        .grayscale_pipeline
        .set_voi_window(VoiWindow::new(
          voi_window_override[0],
          voi_window_override[1],
          "".to_string(),
          VoiLutFunction::LinearExact,
        ));
    }
    // If there's no VOI LUT in the DICOM or specified on the command line
    // then calculate a VOI Window from the content of the first frame and use
    // it for all subsequent frames.
    else if pixel_data_renderer.grayscale_pipeline.voi_lut().is_empty() {
      let image = pixel_data_renderer
        .decode_monochrome_frame(frame)
        .map_err(GetPixelDataError::PixelDataDecodeError)?;

      if let Some(window) = image.default_voi_window() {
        pixel_data_renderer
          .grayscale_pipeline
          .set_voi_window(window);
      }
    }

    // For HDR outputs, emit a Luma16 buffer. A color palette implies 8-bit
    // output because looking up a color palette always returns 8-bit values.
    if args.is_output_hdr() && args.color_palette.is_none() {
      let image = monochrome_image
        .to_gray_u16_image(&pixel_data_renderer.grayscale_pipeline);

      Ok(image.into())
    }
    // If there is an active color palette then use it and output the
    // resulting RGB8
    else if let Some(color_palette) = args.color_palette {
      let image = pixel_data_renderer.render_monochrome_image(
        &monochrome_image,
        Some(color_palette.color_palette()),
      );

      Ok(image.into())
    }
    // Otherwise, emit a Luma8 image
    else {
      let image = monochrome_image
        .to_gray_u8_image(&pixel_data_renderer.grayscale_pipeline);

      Ok(image.into())
    }
  } else {
    let image = pixel_data_renderer
      .decode_color_frame(frame)
      .map_err(GetPixelDataError::PixelDataDecodeError)?;

    // Emit a 16-bit color image if the output format supports HDR and there are
    // more than 8 bits per pixel
    if args.is_output_hdr()
      && pixel_data_renderer.image_pixel_module.bits_stored() > 8
    {
      Ok(image.into_rgb_u16_image().into())
    } else {
      Ok(image.into_rgb_u8_image().into())
    }
  }
}

/// Creates an [`Mp4Encoder`] based on the first frame to be encoded.
///
fn create_mp4_encoder(
  output_prefix: &Path,
  first_frame: &image::DynamicImage,
  cine_module: &CineModule,
  multiframe_module: &MultiFrameModule,
  args: &GetPixelDataArgs,
) -> Result<Mp4Encoder, GetPixelDataError> {
  let mp4_path = crate::utils::path_append(output_prefix.to_path_buf(), ".mp4");

  if !args.overwrite {
    crate::utils::error_if_exists(&mp4_path);
  }

  // Construct MP4 encoder config
  let encoder_config = Mp4EncoderConfig {
    codec: args.mp4_codec,
    codec_params: args.mp4_codec_params.clone().unwrap_or_default(),
    crf: args.mp4_crf,
    preset: args.mp4_preset,
    pixel_format: args.mp4_pixel_format_to_use(),
    resize_filter: args.resize_filter,
    log_level: args.mp4_log_level,
  };

  // Validate the encoder config
  if let Err(message) = encoder_config.validate() {
    return Err(GetPixelDataError::OtherError(message));
  }

  // Determine output dimensions. If there is a resize active then it will be
  // performed by the MP4 encoder.
  let (output_width, output_height) = args
    .new_dimensions(first_frame.width(), first_frame.height())
    .unwrap_or((first_frame.width(), first_frame.height()));

  // Use the Cine Module to determine the frame rate. This can be overridden by
  // a CLI argument if desired. The fallback value is one frame per second.
  let frame_rate = if let Some(frame_rate) = args.mp4_frame_rate {
    frame_rate
  } else {
    cine_module.frame_rate(multiframe_module).unwrap_or(1.0)
  };

  Mp4Encoder::new(
    &mp4_path,
    first_frame,
    frame_rate,
    output_width,
    output_height,
    encoder_config,
  )
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
  overlay_plane_module: Option<&OverlayPlaneModule>,
  args: &GetPixelDataArgs,
) -> Result<(), GetPixelDataError> {
  // Respect frame trimming
  if cine_module.is_frame_trimmed(frame.index().unwrap()) {
    return Ok(());
  }

  // Convert the raw frame into an image ready for MP4 encoding
  let image = frame_to_final_image(
    frame,
    pixel_data_renderer,
    overlay_plane_module,
    args,
  )?;

  // If this is the first frame then the MP4 encoder won't have been created,
  // so create it now
  if mp4_encoder.is_none() {
    println!("Writing \"{}.mp4\" …", output_prefix.display());

    *mp4_encoder = Some(create_mp4_encoder(
      output_prefix,
      &image,
      cine_module,
      multiframe_module,
      args,
    )?);
  }

  // Add the frame to the MP4 encoder
  mp4_encoder
    .as_mut()
    .unwrap()
    .add_frame(&image)
    .map_err(GetPixelDataError::FFmpegError)
}

fn add_token_to_p10_transform<T>(
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
