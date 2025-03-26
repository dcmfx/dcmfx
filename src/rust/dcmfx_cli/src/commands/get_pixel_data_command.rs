use std::{ffi::OsStr, fs::File, io::Write, path::PathBuf};

use clap::{Args, ValueEnum};
use image::codecs::jpeg::JpegEncoder;
use image::{ImageError, ImageFormat};

use dcmfx::core::*;
use dcmfx::p10::*;
use dcmfx::pixel_data::*;

use crate::InputSource;

pub const ABOUT: &str = "Extracts pixel data from DICOM P10 files, writing \
  each frame to an image file";

#[derive(Args)]
pub struct ExtractPixelDataArgs {
  #[clap(
    required = true,
    help = "The names of the DICOM P10 files to extract pixel data from. \
      Specify '-' to read from stdin."
  )]
  input_filenames: Vec<PathBuf>,

  #[arg(
    long,
    short,
    help = "The prefix for output image files. It is suffixed with a 4-digit \
      frame number and an appropriate file extension. This option is only \
      valid when a single input filename is specified. By default, the output \
      prefix is the input filename."
  )]
  output_prefix: Option<PathBuf>,

  #[arg(
    long,
    short,
    value_enum,
    help = "The output image format. 'raw' causes the pixel data for each \
      frame to be written exactly as it is stored in the DICOM P10 file. 'png' \
      and 'jpg' cause the pixel data to be decoded, passed through any active \
      LUTs such as a Modality LUT and VOI Window LUT, then written out as a \
      PNG or JPG image.",
    default_value_t = OutputFormat::Raw
  )]
  format: OutputFormat,

  #[arg(
    long,
    short,
    help = "When the output image format is 'jpg', specifies the quality level \
      in the range 0-100.",
    default_value_t = 85
  )]
  quality: u8,

  #[arg(
    long,
    short = 'w',
    num_args=2..=2,
    value_parser = clap::value_parser!(f32),
    value_names = ["WINDOW_CENTER", "WINDOW_WIDTH"],
    help = "For grayscale DICOM P10 files, when the output image format is \
      'jpg' or 'png', specifies a VOI LUT's window center and width to use \
      instead of the VOI LUT specified in the input DICOM file."
  )]
  voi_window: Option<Vec<f32>>,

  #[arg(
    long,
    value_enum,
    help = "For grayscale DICOM P10 files, when the output image format is \
      'jpg' or 'png', specifies the well-known color palette to apply to \
      visualize the grayscale image in color."
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
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
enum OutputFormat {
  Raw,
  Png,
  Jpg,
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
enum ExtractPixelDataError {
  P10Error(P10Error),
  DataError(DataError),
  ImageError(ImageError),
}

pub fn run(args: &ExtractPixelDataArgs) -> Result<(), ()> {
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
          ExtractPixelDataError::DataError(e) => e.print(&task_description),
          ExtractPixelDataError::P10Error(e) => e.print(&task_description),
          ExtractPixelDataError::ImageError(e) => {
            let lines = vec![
              format!("DICOM image error {}", task_description),
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
  args: &ExtractPixelDataArgs,
) -> Result<(), ExtractPixelDataError> {
  let mut stream = input_source
    .open_read_stream()
    .map_err(ExtractPixelDataError::P10Error)?;

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

  let mut output_extension = match args.format {
    OutputFormat::Raw => "",
    OutputFormat::Png => ".png",
    OutputFormat::Jpg => ".jpg",
  };

  loop {
    // Read the next tokens from the input stream
    let tokens =
      dcmfx::p10::read_tokens_from_stream(&mut stream, &mut read_context)
        .map_err(ExtractPixelDataError::P10Error)?;

    for token in tokens.iter() {
      if args.format == OutputFormat::Raw {
        // Update output extension when the File Meta Information token is
        // received
        if let P10Token::FileMetaInformation { data_set } = token {
          output_extension = file_extension_for_transfer_syntax(
            data_set
              .get_transfer_syntax()
              .unwrap_or(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN),
          );
        }
      }

      // Pass token through the pixel data renderer transform
      if let Some(pixel_data_renderer_transform) =
        pixel_data_renderer_transform.as_mut()
      {
        match pixel_data_renderer_transform.add_token(token) {
          Ok(()) => (),
          Err(P10CustomTypeTransformError::DataError(e)) => {
            return Err(ExtractPixelDataError::DataError(e));
          }
          Err(P10CustomTypeTransformError::P10Error(e)) => {
            return Err(ExtractPixelDataError::P10Error(e));
          }
        };
      }

      // Pass token through the overlays transform filter
      if let Some(overlays_transform) = overlays_transform.as_mut() {
        match overlays_transform.add_token(token) {
          Ok(()) => (),
          Err(P10CustomTypeTransformError::DataError(e)) => {
            return Err(ExtractPixelDataError::DataError(e));
          }
          Err(P10CustomTypeTransformError::P10Error(e)) => {
            return Err(ExtractPixelDataError::P10Error(e));
          }
        };
      }

      let pixel_data_renderer = if let Some(pixel_data_renderer_transform) =
        pixel_data_renderer_transform.as_mut()
      {
        pixel_data_renderer_transform.get_output_mut()
      } else {
        &mut None
      };

      let overlays =
        if let Some(overlays_transform) = overlays_transform.as_mut() {
          overlays_transform.get_output_mut()
        } else {
          &None
        };

      // Pass token through the pixel data frame filter
      let mut frames =
        p10_pixel_data_frame_filter
          .add_token(token)
          .map_err(|e| match e {
            P10PixelDataFrameFilterError::DataError(e) => {
              ExtractPixelDataError::DataError(e)
            }
            P10PixelDataFrameFilterError::P10Error(e) => {
              ExtractPixelDataError::P10Error(e)
            }
          })?;

      // Write frames
      for frame in frames.iter_mut() {
        let mut filename = output_prefix.clone();
        filename.set_file_name(format!(
          "{}.{:04}{}",
          output_prefix
            .file_name()
            .unwrap_or(OsStr::new(""))
            .to_string_lossy(),
          frame.index(),
          output_extension
        ));

        write_frame(
          &filename,
          frame,
          args.format,
          args.quality,
          pixel_data_renderer,
          overlays,
          &args.voi_window,
          args.color_palette.map(|c| c.color_palette()),
        )?;
      }

      if *token == P10Token::End {
        return Ok(());
      }
    }
  }
}

/// Writes the data for a single frame of pixel data to a file.
///
#[allow(clippy::too_many_arguments)]
fn write_frame(
  filename: &PathBuf,
  frame: &mut PixelDataFrame,
  format: OutputFormat,
  quality: u8,
  pixel_data_renderer: &mut Option<PixelDataRenderer>,
  overlays: &Option<Overlays>,
  voi_window_override: &Option<Vec<f32>>,
  color_palette: Option<&ColorPalette>,
) -> Result<(), ExtractPixelDataError> {
  println!("Writing \"{}\" …", filename.display());

  if format == OutputFormat::Raw {
    write_fragments(filename, frame).map_err(|e| {
      ExtractPixelDataError::P10Error(P10Error::FileError {
        when: "Writing pixel data frame".to_string(),
        details: e.to_string(),
      })
    })?;
  } else {
    let pixel_data_renderer = pixel_data_renderer.as_mut().unwrap();

    // Apply the VOI override if it's set
    if let Some(voi_window_override) = voi_window_override {
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

    let mut img = pixel_data_renderer
      .render_frame(frame, color_palette)
      .map_err(ExtractPixelDataError::DataError)?;

    if let Some(overlays) = overlays {
      overlays.render_to_rgb_image(&mut img, frame.index());
    }

    let output_file =
      File::create(filename).expect("Failed to create output file");
    let mut output_writer = std::io::BufWriter::new(output_file);

    if format == OutputFormat::Png {
      img
        .write_to(&mut output_writer, ImageFormat::Png)
        .map_err(ExtractPixelDataError::ImageError)?;
    } else {
      JpegEncoder::new_with_quality(&mut output_writer, quality)
        .encode_image(&img)
        .map_err(ExtractPixelDataError::ImageError)?;
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
