use std::{
  path::{Path, PathBuf},
  time::Duration,
};

use clap::ValueEnum;
use ffmpeg_next::{self as ffmpeg};
use image::RgbImage;

const TIME_BASE: (i32, i32) = (100, 90000);

/// Writes a stream of RGB24 pixel data to an MP4 video file using FFmpeg.
///
pub struct Mp4Encoder {
  path: PathBuf,
  output: ffmpeg::format::context::Output,
  video_encoder: ffmpeg::codec::encoder::video::Encoder,
  duration: Duration,

  rgb24_frame: ffmpeg::frame::Video,
  input_frame: ffmpeg::frame::Video,
  scaling_context: ffmpeg::software::scaling::Context,
}

impl Mp4Encoder {
  /// Initializes video encoding to the specified output MP4 file.
  ///
  pub fn new(
    filename: &PathBuf,
    width: u16,
    height: u16,
    encoder_config: VideoEncoderConfig,
  ) -> Result<Self, ffmpeg::Error> {
    ffmpeg::init()?;
    ffmpeg::util::log::set_level(encoder_config.log_level.ffmpeg_log_level());

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
    video_encoder.set_width(width.into());
    video_encoder.set_height(height.into());
    video_encoder.set_max_b_frames(2);
    video_encoder.set_time_base(TIME_BASE);

    // Frames are provided to the encoder as YUV420P
    let video_encoder_format = ffmpeg::format::Pixel::YUV420P;
    video_encoder.set_format(video_encoder_format);

    // Open the encoder
    let encoder = video_encoder
      .open_as_with(codec, encoder_config.opts(encoder_config.log_level))?;
    let parameters = ffmpeg::codec::Parameters::from(&encoder);

    // Add an output stream for the video codec/encoder
    let mut output_stream = output.add_stream(codec)?;
    output_stream.set_parameters(parameters);

    // Write the MP4 header
    output.write_header()?;

    // Create a scaling context for converting incoming RGB24 frame data to the
    // pixel format expected by the video encoder
    let rgb24_frame = ffmpeg::frame::Video::new(
      ffmpeg::format::Pixel::RGB24,
      width.into(),
      height.into(),
    );
    let input_frame = ffmpeg::frame::Video::new(
      video_encoder_format,
      width.into(),
      height.into(),
    );
    let scaling_context = ffmpeg::software::scaling::Context::get(
      rgb24_frame.format(),
      width.into(),
      height.into(),
      video_encoder_format,
      width.into(),
      height.into(),
      ffmpeg::software::scaling::Flags::BILINEAR,
    )?;

    Ok(Self {
      path: filename.clone(),
      output,
      video_encoder: encoder,
      duration: Duration::ZERO,

      rgb24_frame,
      input_frame,
      scaling_context,
    })
  }

  /// Returns the output path this MP4 encoder is writing to.
  ///
  pub fn path(&self) -> &Path {
    &self.path
  }

  /// Writes the next frame of RGB24 data to be encoded. The duration that the
  /// frame is to be displayed must be specified.
  ///
  pub fn add_frame(
    &mut self,
    rgb_image: &RgbImage,
    frame_duration: Duration,
  ) -> Result<(), ffmpeg::Error> {
    let width = rgb_image.width() as usize;

    // Copy RGB24 data into the FFmpeg frame, ensuring that rows are 32-byte
    // aligned
    if width % 32 == 0 {
      self
        .rgb24_frame
        .data_mut(0)
        .copy_from_slice(rgb_image.as_raw());
    } else {
      let row_size = width * 3;

      let dst_row_size = row_size + (32 - row_size % 32);
      let mut dst = self.rgb24_frame.data_mut(0);

      for src_row in rgb_image.as_raw().chunks_exact(row_size) {
        dst[..row_size].copy_from_slice(src_row);
        dst = &mut dst[dst_row_size..];
      }
    }

    // Convert the RGB24 frame to the video encoder's pixel format
    self
      .scaling_context
      .run(&self.rgb24_frame, &mut self.input_frame)?;

    // Set presentation time stamp on the input frame
    self.input_frame.set_pts(Some(
      self.duration.as_micros() as i64 * i64::from(TIME_BASE.1) / 1000000,
    ));

    // Send the frame to the video encoder
    self.video_encoder.send_frame(&self.input_frame)?;
    self.flush_packets_to_output()?;

    // Update total video duration
    self.duration += frame_duration;

    Ok(())
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
pub struct VideoEncoderConfig {
  pub codec: VideoCodec,
  pub h264_profile: H264Profile,
  pub preset: VideoCompressionPreset,
  pub crf: u32,
  pub log_level: LogLevel,
}

impl VideoEncoderConfig {
  /// Converts the video encoder configuration to an FFmpeg dictionary of
  /// encoder options.
  ///
  pub fn opts(&self, log_level: LogLevel) -> ffmpeg::Dictionary {
    let mut opts = ffmpeg::Dictionary::new();

    opts.set("preset", &self.preset.to_string());
    opts.set("crf", &self.crf.to_string());

    if self.codec == VideoCodec::H264 {
      opts.set("profile", &self.h264_profile.to_string());
    } else {
      opts.set(
        "x265-params",
        &format!("log-level={}", log_level.x265_log_level()),
      );
    }

    opts
  }
}

/// The supported codecs for video encoding.
///
#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum VideoCodec {
  /// H.264, also known as Advanced Video Coding (AVC).
  H264,
}

impl VideoCodec {
  /// Converts to an FFmpeg codec ID.
  ///
  pub fn ffmpeg_id(&self) -> ffmpeg::codec::Id {
    match self {
      Self::H264 => ffmpeg::codec::Id::H264,
    }
  }

  /// Returns the default CRF (constant rate factor) for the codec.
  ///
  pub fn default_crf(&self) -> u32 {
    match self {
      Self::H264 => 18,
    }
  }
}

impl core::fmt::Display for VideoCodec {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::H264 => write!(f, "h264"),
    }
  }
}

/// For H.264 video output, the available profiles that control the features and
/// capability of the video stream.
///
#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum H264Profile {
  /// Basic H.264 profile for low-complexity devices and real-time use.
  Baseline,

  /// Mid-tier H.264 profile adding B-frames and interlacing for better
  /// compression.
  Main,

  /// Advanced H.264 profile for high-definition video and optimal efficiency.
  High,
}

impl core::fmt::Display for H264Profile {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::Baseline => write!(f, "baseline"),
      Self::Main => write!(f, "main"),
      Self::High => write!(f, "high"),
    }
  }
}

/// Video compression presets that control encoding speed vs compression
/// efficiency.
///
#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum VideoCompressionPreset {
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

impl core::fmt::Display for VideoCompressionPreset {
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
  pub fn ffmpeg_log_level(&self) -> ffmpeg::util::log::Level {
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

  pub fn x265_log_level(&self) -> &str {
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
