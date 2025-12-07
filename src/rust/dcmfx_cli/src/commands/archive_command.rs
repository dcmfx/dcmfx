use clap::Args;
use dcmfx::{core::DataSet, p10::IoAsyncRead};
use dcmfx_cli::utils::OutputTarget;
use futures::{AsyncWriteExt, StreamExt};
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

use crate::utils;

pub const ABOUT: &str = "Archives one or more DICOM P10 files into a ZIP file \
  together with a DICOMDIR file.";

#[derive(Args)]
pub struct ArchiveArgs {
  #[command(flatten)]
  input: crate::args::input_args::P10InputArgs,

  #[arg(
    long,
    short,
    help_heading = "Output",
    help = "The name of the ZIP file containing the archived DICOM files. \
      Specify '-' to write to stdout."
  )]
  output_filename: PathBuf,

  #[arg(
    long,
    help_heading = "Output",
    help = "Overwrite any output files that already exist",
    default_value_t = false
  )]
  overwrite: bool,

  #[arg(
    long,
    help_heading = "ZIP Compression",
    help = "The compression method to use when outputting a ZIP file.",
    default_value_t = args::ZipCompressionMethod::Stored
  )]
  zip_compression_method: args::ZipCompressionMethod,

  #[arg(
    long,
    help_heading = "ZIP Compression",
    help = "The compression level to use when outputting a ZIP file using the \
      Deflate compression method.",
    default_value_t = args::DeflateCompressionLevel::Normal
  )]
  deflate_compression_level: args::DeflateCompressionLevel,
}

pub async fn run(args: ArchiveArgs) -> Result<(), ()> {
  let mut input_sources = args.input.base.input_sources().await;

  OutputTarget::set_overwrite(args.overwrite);

  let output_target = OutputTarget::new(&args.output_filename).await;

  let output_stream = match output_target
    .open_write_stream(!output_target.is_stdout())
    .await
  {
    Ok(s) => s,

    Err(e) => {
      utils::exit_with_error(
        &format!(
          "Error opening output file \"{}\"",
          args.output_filename.display()
        ),
        e,
      );
    }
  };

  let mut output_stream = output_stream.lock().await;

  use async_zip::tokio::write::ZipFileWriter;

  let mut zip_file_writer = ZipFileWriter::with_tokio(&mut *output_stream);

  let mut file_index = 0;

  while let Some(input_source) = input_sources.next().await {
    let filename = format!("{file_index:08}");

    let mut input_stream = input_source.open_read_stream().await.unwrap();

    add_entry_to_zip_archive(
      &mut zip_file_writer,
      &mut input_stream,
      &filename,
      &args,
    )
    .await
    .unwrap();

    file_index += 1;
  }

  zip_file_writer.close().await.unwrap();

  output_target.commit(&mut output_stream).await.unwrap();

  Ok(())
}

async fn add_entry_to_zip_archive<
  W: tokio::io::AsyncWrite + Unpin,
  S: IoAsyncRead,
>(
  zip_file_writer: &mut async_zip::tokio::write::ZipFileWriter<W>,
  input_stream: &mut S,
  filename: &str,
  args: &ArchiveArgs,
) -> Result<DataSet, async_zip::error::ZipError> {
  let builder = async_zip::ZipEntryBuilder::new(
    filename.into(),
    args.zip_compression_method.to_async_zip_compression(),
  )
  .deflate_option(args.deflate_compression_level.to_async_zip_deflate_option());

  let mut entry = zip_file_writer.write_entry_stream(builder).await?;

  let mut buffer = vec![0u8; 64 * 1024];

  loop {
    let bytes_read = input_stream.read(&mut buffer).await?;
    if bytes_read == 0 {
      break;
    }

    entry.write_all(&buffer[..bytes_read]).await?;
  }

  entry.close().await
}

mod args {
  use clap::ValueEnum;

  #[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
  pub enum ZipCompressionMethod {
    Stored,
    Deflate,
  }

  impl ZipCompressionMethod {
    pub fn to_async_zip_compression(&self) -> async_zip::Compression {
      match self {
        Self::Stored => async_zip::Compression::Stored,
        Self::Deflate => async_zip::Compression::Deflate,
      }
    }
  }

  impl core::fmt::Display for ZipCompressionMethod {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
      match self {
        Self::Stored => write!(f, "stored"),
        Self::Deflate => write!(f, "deflate"),
      }
    }
  }

  #[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
  pub enum DeflateCompressionLevel {
    Normal,
    Maximum,
    Fast,
    SuperFast,
  }

  impl DeflateCompressionLevel {
    pub fn to_async_zip_deflate_option(&self) -> async_zip::DeflateOption {
      match self {
        Self::Normal => async_zip::DeflateOption::Normal,
        Self::Maximum => async_zip::DeflateOption::Maximum,
        Self::Fast => async_zip::DeflateOption::Fast,
        Self::SuperFast => async_zip::DeflateOption::Super,
      }
    }
  }

  impl core::fmt::Display for DeflateCompressionLevel {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
      match self {
        Self::Normal => write!(f, "normal"),
        Self::Maximum => write!(f, "maximum"),
        Self::Fast => write!(f, "fast"),
        Self::SuperFast => write!(f, "superfast"),
      }
    }
  }
}
