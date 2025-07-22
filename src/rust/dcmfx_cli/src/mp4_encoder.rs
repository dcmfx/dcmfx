use std::{io::Write, path::Path};

use clap::ValueEnum;

/// Writes a stream of RGB or Luma frames to an MP4 video file using FFmpeg.
///
pub struct Mp4Encoder {
  ffmpeg_child_process: std::process::Child,
}

impl Mp4Encoder {
  /// Initializes MP4 encoding to the specified output file.
  ///
  pub fn new(
    filename: &Path,
    first_frame: &image::DynamicImage,
    frame_rate: f64,
    mut output_width: u32,
    mut output_height: u32,
    encoder_config: Mp4EncoderConfig,
  ) -> Result<Self, String> {
    Self::check_ffmpeg_is_available()?;

    // Ensure output dimensions are divisible by two. This is required by
    // libx264 and libx265.
    output_width &= !1;
    output_height &= !1;

    // Start building FFmpeg command line arguments
    let mut ffmpeg_args = vec![
      "-loglevel".to_string(),
      encoder_config.log_level.ffmpeg_log_level().to_string(),
    ];

    // Specify how input will be sent as raw data on stdin
    ffmpeg_args.push("-f".to_string());
    ffmpeg_args.push("rawvideo".to_string());
    ffmpeg_args.push("-pix_fmt".to_string());
    match first_frame.color() {
      image::ColorType::L8 => ffmpeg_args.push("gray".to_string()),
      image::ColorType::L16 => ffmpeg_args.push("gray16le".to_string()),
      image::ColorType::Rgb8 => ffmpeg_args.push("rgb24".to_string()),
      image::ColorType::Rgb16 => ffmpeg_args.push("rgb48le".to_string()),
      _ => unreachable!(),
    };
    ffmpeg_args.push("-s".to_string());
    ffmpeg_args.push(format!(
      "{}x{}",
      first_frame.width(),
      first_frame.height()
    ));
    ffmpeg_args.push("-framerate".to_string());
    ffmpeg_args.push(frame_rate.to_string());
    ffmpeg_args.push("-i".to_string());
    ffmpeg_args.push("-".to_string());

    // Output an MP4 video
    ffmpeg_args.push("-f".to_string());
    ffmpeg_args.push("mp4".to_string());

    // Specify the codec
    ffmpeg_args.push("-c:v".to_string());
    ffmpeg_args.push(encoder_config.codec.ffmpeg_id().to_string());

    // Configure output to put the 'moov' atom at the start of the file. This
    // requires a second pass so is a little slower, but is recommended for any
    // streaming usage.
    ffmpeg_args.push("-movflags".to_string());
    ffmpeg_args.push("+faststart".to_string());

    // Add rescale filter if the output size doesn't match the input
    let is_resizing = first_frame.width() != output_width
      || first_frame.height() != output_height;
    if is_resizing {
      ffmpeg_args.push("-vf".to_string());
      ffmpeg_args.push(format!(
        "scale={}:{}:{}",
        output_width,
        output_height,
        encoder_config.resize_filter.ffmpeg_flag()
      ));
    };

    // Add extra encoder options
    ffmpeg_args.extend(encoder_config.ffmpeg_encoder_options());

    // For H.265 output, set an 'HVC1' codec tag to improve compatibility on
    // Apple devices
    if encoder_config.codec == Mp4Codec::Libx265 {
      ffmpeg_args.push("-tag:v".to_string());
      ffmpeg_args.push("hvc1".to_string());
    }

    // Specify output filename
    ffmpeg_args.push(filename.to_string_lossy().to_string());
    ffmpeg_args.push("-y".to_string());

    // Spawn the ffmpeg process
    let ffmpeg_child_process =
      std::process::Command::new(Self::ffmpeg_binary())
        .args(ffmpeg_args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(Self {
      ffmpeg_child_process,
    })
  }

  /// Returns the name of the FFmpeg binary to use.
  ///
  fn ffmpeg_binary() -> &'static str {
    #[cfg(not(windows))]
    return "ffmpeg";

    #[cfg(windows)]
    return "ffmpeg.exe";
  }

  /// Returns an error if the required FFmpeg binary isn't present.
  ///
  fn check_ffmpeg_is_available() -> Result<(), String> {
    std::process::Command::new(Self::ffmpeg_binary())
      .args(["-version"])
      .stdout(std::process::Stdio::null())
      .stderr(std::process::Stdio::null())
      .spawn()
      .map(|_| ())
      .map_err(|_| {
        format!(
          "\"{}\" binary is not available on the path, install it using a \
           package manager or download from https://ffmpeg.org/download.html",
          Self::ffmpeg_binary()
        )
      })
  }

  /// Writes the next frame of video.
  ///
  pub fn add_frame(
    &mut self,
    frame_image: &image::DynamicImage,
  ) -> Result<(), String> {
    let stdin = self
      .ffmpeg_child_process
      .stdin
      .as_mut()
      .expect("Failed to open FFmpeg stdin");

    // Write frame data to the FFmpeg child process' stdin
    stdin
      .write_all(frame_image.as_bytes())
      .map_err(|e| e.to_string())
  }

  /// Completes encoding once all frames have been written.
  ///
  pub fn finish(&mut self) -> Result<(), String> {
    let exit_status = self
      .ffmpeg_child_process
      .wait()
      .map_err(|e| e.to_string())?;

    if !exit_status.success() {
      return Err(format!(
        "FFMpeg process returned exit code {:?}",
        exit_status.code()
      ));
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
  fn ffmpeg_encoder_options(&self) -> Vec<String> {
    let mut args = vec![
      "-preset".to_string(),
      self.preset.to_string(),
      "-crf".to_string(),
      self.crf.to_string(),
    ];

    // Set max B frames
    args.push("-bf".to_string());
    match self.codec {
      Mp4Codec::Libx264 => args.push("2".to_string()),
      Mp4Codec::Libx265 => args.push("4".to_string()),
    }

    // Set output pixel format
    args.push("-pix_fmt".to_string());
    args.push(self.pixel_format.ffmpeg_id().to_string());

    // Pass log level through to libx265
    if self.codec == Mp4Codec::Libx265 {
      args.push("-x265-params".to_string());
      args.push(format!("log-level={}", self.log_level.x265_log_level()));
    }

    args
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
  /// Converts to an FFmpeg codec ID.
  ///
  pub fn ffmpeg_id(&self) -> &'static str {
    match self {
      Self::Libx264 => "libx264",
      Self::Libx265 => "libx265",
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

  fn ffmpeg_id(&self) -> &'static str {
    match self {
      Self::Yuv420p => "yuv420p",
      Self::Yuv422p => "yuv422p",
      Self::Yuv444p => "yuv444p",
      Self::Yuv420p10 => "yuv420p10",
      Self::Yuv422p10 => "yuv422p10",
      Self::Yuv444p10 => "yuv444p10",
      Self::Yuv420p12 => "yuv420p12",
      Self::Yuv422p12 => "yuv422p12",
      Self::Yuv444p12 => "yuv444p12",
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
  fn ffmpeg_log_level(&self) -> &'static str {
    match self {
      Self::Quiet => "quiet",
      Self::Panic => "panic",
      Self::Fatal => "fatal",
      Self::Error => "error",
      Self::Warning => "warning",
      Self::Info => "info",
      Self::Verbose => "verbose",
      Self::Debug => "debug",
      Self::Trace => "trace",
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
  fn ffmpeg_flag(&self) -> &'static str {
    match self {
      Self::Bilinear => "bilinear",
      Self::Bicubic => "bicubic",
      Self::Gaussian => "gauss",
      Self::Lanczos3 => "lanczos",
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
