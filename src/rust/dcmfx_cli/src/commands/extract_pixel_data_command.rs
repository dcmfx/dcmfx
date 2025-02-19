use std::fs::File;
use std::io::{Read, Write};

use clap::{Args, ValueEnum};
use image::codecs::jpeg::JpegEncoder;
use image::{ImageError, ImageFormat};

use dcmfx::core::*;
use dcmfx::p10::*;
use dcmfx::pixel_data::*;

pub const ABOUT: &str = "Extracts the pixel data from a DICOM P10 file and \
  writes each frame to a separate image file";

#[derive(Args)]
pub struct ExtractPixelDataArgs {
  #[clap(
    help = "The name of the file to read DICOM P10 content from. Specify '-' \
      to read from stdin."
  )]
  input_filename: String,

  #[arg(
    long,
    short,
    help = "The prefix for output image files. It is suffixed with a 4-digit \
      frame number and an appropriate file extension. By default, the output \
      prefix is the input filename."
  )]
  output_prefix: Option<String>,

  #[arg(
    long,
    short,
    value_enum,
    help = "The output image format. 'raw' causes the pixel data for each \
      frame to be written without alteration. For native pixel data, 'png' or \
      'jpg' causes the pixel data to be converted to a PNG or JPG image prior \
      to being written out.",
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
    short,
    num_args=2..=2,
    value_parser = clap::value_parser!(f64),
    value_names = ["WINDOW_CENTER", "WINDOW_WIDTH"],
    help = "When the input DICOM is grayscale and the output image format is \
      'jpg' or 'png', specifies the VOI window center and width to use instead \
      of the VOI LUT defined in the input DICOM."
  )]
  voi_window: Option<Vec<f64>>,

  #[arg(
    long,
    short,
    value_enum,
    help = "When the output image format is 'jpg' or 'png' and the input DICOM \
      is grayscale, specifies the well-known color palette to apply."
  )]
  color_palette: Option<StandardColorPaletteArg>,
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

pub fn run(args: &ExtractPixelDataArgs) -> Result<(), ()> {
  let output_prefix =
    args.output_prefix.as_ref().unwrap_or(&args.input_filename);

  match perform_extract_pixel_data(
    &args.input_filename,
    output_prefix,
    args.format,
    args.quality,
    &args.voi_window,
    args.color_palette.map(|e| e.color_palette()),
  ) {
    Ok(_) => Ok(()),

    Err(e) => {
      let task_description =
        format!("reading file \"{}\"", args.input_filename);

      match e {
        ExtractPixelDataError::DataError(e) => e.print(&task_description),
        ExtractPixelDataError::P10Error(e) => e.print(&task_description),
        ExtractPixelDataError::ImageError(e) => {
          let lines = vec![
            format!("DICOM image error {}", task_description),
            "".to_string(),
            format!("  Error: {}", e),
          ];

          dcmfx::core::error::print_error_lines(&lines);
        }
      }

      Err(())
    }
  }
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
enum ExtractPixelDataError {
  P10Error(P10Error),
  DataError(DataError),
  ImageError(ImageError),
}

fn perform_extract_pixel_data(
  input_filename: &str,
  output_prefix: &str,
  format: OutputFormat,
  quality: u8,
  voi_window_override: &Option<Vec<f64>>,
  color_palette: Option<&ColorPalette>,
) -> Result<(), ExtractPixelDataError> {
  // Open input stream
  let mut input_stream: Box<dyn Read> = match input_filename {
    "-" => Box::new(std::io::stdin()),
    _ => match File::open(input_filename) {
      Ok(file) => Box::new(file),
      Err(e) => {
        return Err(ExtractPixelDataError::P10Error(P10Error::FileError {
          when: "Opening file".to_string(),
          details: e.to_string(),
        }));
      }
    },
  };

  // Create read context
  let mut read_context = P10ReadContext::new();
  read_context.set_config(&P10ReadConfig {
    max_token_size: 1024 * 1024,
    ..P10ReadConfig::default()
  });

  let mut pixel_data_filter = PixelDataFilter::new();

  let mut pixel_data_reader = P10CustomTypeTransform::<PixelDataReader>::new(
    &PixelDataReader::DATA_ELEMENT_TAGS,
    PixelDataReader::from_data_set,
  );

  let mut output_extension = match format {
    OutputFormat::Raw => "",
    OutputFormat::Png => ".png",
    OutputFormat::Jpg => ".jpg",
  };

  loop {
    // Read the next tokens from the input stream
    let tokens =
      dcmfx::p10::read_tokens_from_stream(&mut input_stream, &mut read_context)
        .map_err(ExtractPixelDataError::P10Error)?;

    for token in tokens.iter() {
      if format == OutputFormat::Raw {
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

      // Pass token through the pixel data definition filter
      match pixel_data_reader.add_token(token) {
        Ok(()) => (),
        Err(P10CustomTypeTransformError::DataError(e)) => {
          return Err(ExtractPixelDataError::DataError(e));
        }
        Err(P10CustomTypeTransformError::P10Error(e)) => {
          return Err(ExtractPixelDataError::P10Error(e));
        }
      };

      // Pass token through the pixel data filter
      let mut frames =
        pixel_data_filter.add_token(token).map_err(|e| match e {
          PixelDataFilterError::DataError(e) => {
            ExtractPixelDataError::DataError(e)
          }
          PixelDataFilterError::P10Error(e) => {
            ExtractPixelDataError::P10Error(e)
          }
        })?;

      // Write frames
      for frame in frames.iter_mut() {
        let filename =
          format!("{}.{:04}{}", output_prefix, frame.index(), output_extension);

        write_frame(
          &filename,
          frame,
          format,
          quality,
          pixel_data_reader.get_output_mut(),
          voi_window_override,
          color_palette,
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
fn write_frame(
  filename: &str,
  frame: &mut PixelDataFrame,
  format: OutputFormat,
  quality: u8,
  pixel_data_reader: &mut Option<PixelDataReader>,
  voi_window_override: &Option<Vec<f64>>,
  color_palette: Option<&ColorPalette>,
) -> Result<(), ExtractPixelDataError> {
  println!("Writing \"{filename}\" …");

  if format == OutputFormat::Raw {
    write_fragments(filename, frame).map_err(|e| {
      ExtractPixelDataError::P10Error(P10Error::FileError {
        when: "Writing pixel data frame".to_string(),
        details: e.to_string(),
      })
    })?;
  } else {
    match pixel_data_reader {
      Some(pixel_data_reader) => {
        // Apply the VOI override if it's set
        if let Some(voi_window_override) = voi_window_override {
          pixel_data_reader.voi_lut = VoiLut {
            luts: vec![],
            windows: vec![VoiWindow::new(
              voi_window_override[0],
              voi_window_override[1],
              "".to_string(),
              VoiLutFunction::LinearExact,
            )],
          };
        }

        let img = pixel_data_reader
          .decode_frame(frame, color_palette)
          .map_err(ExtractPixelDataError::DataError)?;

        let mut output_file =
          File::create(filename).expect("Failed to create output file");

        if format == OutputFormat::Png {
          img
            .write_to(&mut output_file, ImageFormat::Png)
            .map_err(ExtractPixelDataError::ImageError)?;
        } else {
          JpegEncoder::new_with_quality(&mut output_file, quality)
            .encode_image(&img)
            .map_err(ExtractPixelDataError::ImageError)?;
        }
      }

      None => unreachable!(),
    }
  }

  Ok(())
}

/// Writes the data for a single frame of pixel data to a file.
///
fn write_fragments(
  filename: &str,
  frame: &PixelDataFrame,
) -> Result<(), std::io::Error> {
  let mut stream = File::create(filename)?;

  for fragment in frame.fragments() {
    stream.write_all(fragment)?;
  }

  stream.flush()
}
