use std::{
  path::{Path, PathBuf},
  time::Duration,
};

use clap::ValueEnum;
use ffmpeg_next::{self as ffmpeg};

const TIME_BASE: (i32, i32) = (100, 90000);

/// Writes a stream of RGB24 frames to an MP4 video file using FFmpeg.
///
pub struct Mp4Encoder {
  path: PathBuf,
  output: ffmpeg::format::context::Output,
  video_encoder: ffmpeg::codec::encoder::video::Encoder,
  duration: Duration,

  raw_frame: ffmpeg::frame::Video,
  yuv_frame: ffmpeg::frame::Video,
  scaling_context: ffmpeg::software::scaling::Context,
}

impl Mp4Encoder {
  /// Initializes MP4 encoding to the specified output file.
  ///
  pub fn new(
    filename: &PathBuf,
    first_frame: &image::DynamicImage,
    mut output_width: u32,
    mut output_height: u32,
    encoder_config: Mp4EncoderConfig,
  ) -> Result<Self, ffmpeg::Error> {
    ffmpeg::init()?;
    ffmpeg::util::log::set_level(encoder_config.log_level.ffmpeg_log_level());

    // Ensure output dimensions are divisible by two. This is required by
    // libx264 and libx265.
    output_width &= !1;
    output_height &= !1;

    // Configure output to put the 'moov' atom at the start of the file. This
    // requires a second pass so is a little slower, but is recommended for any
    // streaming usage.
    let mut options = ffmpeg::Dictionary::new();
    options.set("movflags", "faststart");

    // Create MP4 output
    let mut output = ffmpeg::format::output_as_with(filename, "mp4", options)?;

    // Look up the codec
    let codec = ffmpeg::codec::encoder::find(encoder_config.codec.ffmpeg_id())
      .ok_or(ffmpeg::Error::EncoderNotFound)?;

    // Create video encoder
    let context = ffmpeg::codec::Context::new_with_codec(codec);
    let mut video_encoder = context.encoder().video()?;

    // Configure video encoder
    video_encoder.set_width(output_width);
    video_encoder.set_height(output_height);
    video_encoder.set_max_b_frames(2);
    video_encoder.set_time_base(TIME_BASE);
    video_encoder.set_format(encoder_config.pixel_format.ffmpeg_id());

    // Open the encoder
    let encoder = video_encoder
      .open_as_with(codec, encoder_config.ffmpeg_encoder_options())?;
    let mut parameters = ffmpeg::codec::Parameters::from(&encoder);

    // For H.265 output, set an 'HVC1' codec tag to improve compatibility on
    // Apple devices
    if encoder_config.codec.is_h265() {
      let hvc1_fourcc = u32::from_le_bytes([b'h', b'v', b'c', b'1']);
      unsafe {
        (*parameters.as_mut_ptr()).codec_tag = hvc1_fourcc;
      }
    }

    // Add an output stream for the video codec/encoder
    let mut output_stream = output.add_stream(codec)?;
    output_stream.set_parameters(parameters);

    // Write the MP4 header
    output.write_header()?;

    // Create a scaling context for converting incoming raw frame data to the
    // pixel format expected by the video encoder. The scaling context will also
    // resize the incoming frame if needed.

    let input_format = match first_frame.color() {
      image::ColorType::L8 => ffmpeg::format::Pixel::GRAY8,
      image::ColorType::L16 => ffmpeg::format::Pixel::GRAY16,
      image::ColorType::Rgb8 => ffmpeg::format::Pixel::RGB24,
      image::ColorType::Rgb16 => ffmpeg::format::Pixel::RGB48,
      _ => unreachable!(),
    };

    let raw_frame = ffmpeg::frame::Video::new(
      input_format,
      first_frame.width(),
      first_frame.height(),
    );

    let yuv_frame = ffmpeg::frame::Video::new(
      encoder_config.pixel_format.ffmpeg_id(),
      output_width,
      output_height,
    );

    let is_resizing = first_frame.width() != output_width
      || first_frame.height() != output_height;
    let filter = if is_resizing {
      encoder_config.resize_filter.ffmpeg_flag()
    } else {
      ffmpeg::software::scaling::Flags::POINT
    };

    let scaling_context = ffmpeg::software::scaling::Context::get(
      raw_frame.format(),
      raw_frame.width(),
      raw_frame.height(),
      yuv_frame.format(),
      yuv_frame.width(),
      yuv_frame.height(),
      filter,
    )?;

    Ok(Self {
      path: filename.clone(),
      output,
      video_encoder: encoder,
      duration: Duration::ZERO,

      raw_frame,
      yuv_frame,
      scaling_context,
    })
  }

  /// Returns the output path this MP4 encoder is writing to.
  ///
  pub fn path(&self) -> &Path {
    &self.path
  }

  /// Writes the next frame to be encoded to MP4. The duration that the frame is
  /// to be displayed must be specified.
  ///
  pub fn add_frame(
    &mut self,
    frame_image: &image::DynamicImage,
    frame_duration: Duration,
  ) -> Result<(), ffmpeg::Error> {
    // Copy frame data into the FFmpeg frame
    match frame_image {
      image::DynamicImage::ImageLuma8(image) => {
        self.update_raw_frame(image.width() as usize, image.as_raw())
      }

      image::DynamicImage::ImageRgb8(image) => {
        self.update_raw_frame(image.width() as usize * 3, image.as_raw())
      }

      image::DynamicImage::ImageLuma16(image) => {
        self.update_raw_frame(image.width() as usize * 2, image.as_raw())
      }

      image::DynamicImage::ImageRgb16(image) => {
        self.update_raw_frame(image.width() as usize * 6, image.as_raw())
      }

      _ => unreachable!(),
    }

    // Convert the RGB24 frame to the video encoder's expected pixel format and
    // dimensions
    self
      .scaling_context
      .run(&self.raw_frame, &mut self.yuv_frame)?;

    // Set presentation time stamp on the input frame
    self.yuv_frame.set_pts(Some(
      self.duration.as_micros() as i64 * i64::from(TIME_BASE.1) / 1000000,
    ));

    // Send the frame to the video encoder
    self.video_encoder.send_frame(&self.yuv_frame)?;
    self.flush_packets_to_output()?;

    // Update total video duration
    self.duration += frame_duration;

    Ok(())
  }

  /// Copies data for an incoming frame into the raw frame frame ready for
  /// pixel format conversion and encoding.
  ///
  /// This copy respects the `linesize`` of the target frame.
  ///
  fn update_raw_frame<T: bytemuck::Pod>(
    &mut self,
    row_size: usize,
    data: &[T],
  ) {
    let linesize = unsafe { (*self.raw_frame.as_ptr()).linesize }[0] as usize;

    let mut dst = self.raw_frame.data_mut(0);

    if row_size == linesize {
      dst.copy_from_slice(bytemuck::cast_slice(data));
    } else {
      for src_row in bytemuck::cast_slice(data).chunks_exact(row_size) {
        dst[..row_size].copy_from_slice(src_row);
        dst = &mut dst[linesize..];
      }
    }
  }

  /// Completes encoding once all frames have been written.
  ///
  pub fn finish(&mut self) -> Result<(), ffmpeg::Error> {
    self.video_encoder.send_eof()?;
    self.flush_packets_to_output()?;
    self.output.write_trailer()
  }

  fn flush_packets_to_output(&mut self) -> Result<(), ffmpeg::Error> {
    let mut packet = ffmpeg::Packet::empty();

    while let Ok(()) = self.video_encoder.receive_packet(&mut packet) {
      packet.write_interleaved(&mut self.output)?;
    }

    Ok(())
  }
}

/// Video encoder configuration that specifies the codec and encoding options to
/// use.
///
#[derive(Clone, Debug, PartialEq)]
pub struct Mp4EncoderConfig {
  pub codec: Mp4Codec,
  pub codec_params: String,
  pub crf: u32,
  pub preset: Mp4CompressionPreset,
  pub pixel_format: Mp4PixelFormat,
  pub resize_filter: ResizeFilter,
  pub log_level: LogLevel,
}

impl Mp4EncoderConfig {
  /// Validates that the encoder config is supported by FFmpeg.
  ///
  pub fn validate(&self) -> Result<(), String> {
    if self.codec == Mp4Codec::Libx264 && self.pixel_format.is_12bit() {
      return Err("libx264 does not support 12-bit pixel formats".to_string());
    }

    Ok(())
  }

  /// Converts the encoder configuration to an FFmpeg dictionary of encoder
  /// options.
  ///
  fn ffmpeg_encoder_options(&self) -> ffmpeg::Dictionary {
    let mut opts = ffmpeg::Dictionary::new();

    opts.set("preset", &self.preset.to_string());
    opts.set("crf", &self.crf.to_string());

    let codec_params = if self.codec == Mp4Codec::Libx265 {
      // Pass log level through to libx265
      format!(
        "log-level={}:{}",
        self.log_level.x265_log_level(),
        self.codec_params
      )
    } else {
      self.codec_params.clone()
    };

    match self.codec {
      Mp4Codec::Libx264 => opts.set("x264-params", &codec_params),
      Mp4Codec::Libx265 => opts.set("x265-params", &codec_params),
    };

    opts
  }
}

/// The supported codecs for MP4 encoding.
///
#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum Mp4Codec {
  /// The libx264 encoder which encodes H.264/AVC video.
  Libx264,

  /// The libx265 encoder which encodes H.265/HEVC video.
  Libx265,
}

impl Mp4Codec {
  /// Returns whether this codec produces H.265 video.
  ///
  pub fn is_h265(&self) -> bool {
    *self == Self::Libx265
  }

  /// Converts to an FFmpeg codec ID.
  ///
  pub fn ffmpeg_id(&self) -> ffmpeg::codec::Id {
    match self {
      Self::Libx264 => ffmpeg::codec::Id::H264,
      Self::Libx265 => ffmpeg::codec::Id::H265,
    }
  }
}

impl core::fmt::Display for Mp4Codec {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::Libx264 => write!(f, "libx264"),
      Self::Libx265 => write!(f, "libx265"),
    }
  }
}

/// MP4 compression presets that control encoding speed vs compression
/// efficiency.
///
#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum Mp4CompressionPreset {
  Ultrafast,
  Superfast,
  Veryfast,
  Faster,
  Fast,
  Medium,
  Slow,
  Slower,
  Veryslow,
}

impl core::fmt::Display for Mp4CompressionPreset {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::Ultrafast => write!(f, "ultrafast"),
      Self::Superfast => write!(f, "superfast"),
      Self::Veryfast => write!(f, "veryfast"),
      Self::Faster => write!(f, "faster"),
      Self::Fast => write!(f, "fast"),
      Self::Medium => write!(f, "medium"),
      Self::Slow => write!(f, "slow"),
      Self::Slower => write!(f, "slower"),
      Self::Veryslow => write!(f, "veryslow"),
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum Mp4PixelFormat {
  Yuv420p,
  Yuv422p,
  Yuv444p,
  Yuv420p10,
  Yuv422p10,
  Yuv444p10,
  Yuv420p12,
  Yuv422p12,
  Yuv444p12,
}

impl Mp4PixelFormat {
  pub fn is_hdr(&self) -> bool {
    *self == Self::Yuv420p10
      || *self == Self::Yuv422p10
      || *self == Self::Yuv444p10
      || *self == Self::Yuv420p12
      || *self == Self::Yuv422p12
      || *self == Self::Yuv444p12
  }

  fn is_12bit(&self) -> bool {
    *self == Self::Yuv420p12
      || *self == Self::Yuv422p12
      || *self == Self::Yuv444p12
  }

  fn ffmpeg_id(&self) -> ffmpeg::format::Pixel {
    use ffmpeg::format::Pixel;
    match self {
      Self::Yuv420p => Pixel::YUV420P,
      Self::Yuv422p => Pixel::YUV422P,
      Self::Yuv444p => Pixel::YUV444P,
      Self::Yuv420p10 => Pixel::YUV420P10,
      Self::Yuv422p10 => Pixel::YUV422P10,
      Self::Yuv444p10 => Pixel::YUV444P10,
      Self::Yuv420p12 => Pixel::YUV420P12,
      Self::Yuv422p12 => Pixel::YUV422P12,
      Self::Yuv444p12 => Pixel::YUV444P12,
    }
  }
}

/// The output log level for FFmpeg to use when encoding an MP4.
///
#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum LogLevel {
  Quiet,
  Panic,
  Fatal,
  Error,
  Warning,
  Info,
  Verbose,
  Debug,
  Trace,
}

impl LogLevel {
  fn ffmpeg_log_level(&self) -> ffmpeg::util::log::Level {
    match self {
      Self::Quiet => ffmpeg::util::log::Level::Quiet,
      Self::Panic => ffmpeg::util::log::Level::Panic,
      Self::Fatal => ffmpeg::util::log::Level::Fatal,
      Self::Error => ffmpeg::util::log::Level::Error,
      Self::Warning => ffmpeg::util::log::Level::Warning,
      Self::Info => ffmpeg::util::log::Level::Info,
      Self::Verbose => ffmpeg::util::log::Level::Verbose,
      Self::Debug => ffmpeg::util::log::Level::Debug,
      Self::Trace => ffmpeg::util::log::Level::Trace,
    }
  }

  fn x265_log_level(&self) -> &str {
    match self {
      Self::Quiet | Self::Panic | Self::Fatal | Self::Error => "error",
      Self::Warning => "warning",
      Self::Info => "info",
      Self::Verbose => "debug",
      Self::Debug => "debug",
      Self::Trace => "full",
    }
  }
}

impl core::fmt::Display for LogLevel {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::Quiet => write!(f, "quiet"),
      Self::Panic => write!(f, "panic"),
      Self::Fatal => write!(f, "fatal"),
      Self::Error => write!(f, "error"),
      Self::Warning => write!(f, "warning"),
      Self::Info => write!(f, "info"),
      Self::Verbose => write!(f, "verbose"),
      Self::Debug => write!(f, "debug"),
      Self::Trace => write!(f, "trace"),
    }
  }
}

/// The filter to use when a resize of a frame of image data occurs prior to it
/// being encoded.
///
#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum ResizeFilter {
  /// Fast, low-quality filter using linear interpolation for basic resizing.
  Bilinear,

  /// Slower than bilinear, but offers smoother, higher-quality results using
  /// cubic interpolation.
  Bicubic,

  /// Medium speed and quality, applies a soft blur that can reduce aliasing.
  Gaussian,

  /// High-quality but slower filter using a sinc function, ideal for sharp,
  /// detailed resizing.
  Lanczos3,
}

impl ResizeFilter {
  fn ffmpeg_flag(&self) -> ffmpeg::software::scaling::Flags {
    match self {
      Self::Bilinear => ffmpeg::software::scaling::Flags::BILINEAR,
      Self::Bicubic => ffmpeg::software::scaling::Flags::BICUBIC,
      Self::Gaussian => ffmpeg::software::scaling::Flags::GAUSS,
      Self::Lanczos3 => ffmpeg::software::scaling::Flags::LANCZOS,
    }
  }

  pub fn filter_type(&self) -> image::imageops::FilterType {
    match self {
      Self::Bilinear => image::imageops::FilterType::Triangle,
      Self::Bicubic => image::imageops::FilterType::CatmullRom,
      Self::Gaussian => image::imageops::FilterType::Gaussian,
      Self::Lanczos3 => image::imageops::FilterType::Lanczos3,
    }
  }
}

impl core::fmt::Display for ResizeFilter {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::Bilinear => write!(f, "bilinear"),
      Self::Bicubic => write!(f, "bicubic"),
      Self::Gaussian => write!(f, "gaussian"),
      Self::Lanczos3 => write!(f, "lanczos3"),
    }
  }
}
