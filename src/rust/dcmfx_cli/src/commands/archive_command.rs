use async_zip::tokio::write::ZipFileWriter;
use bytes::Bytes;
use clap::Args;
use futures::{AsyncWriteExt, StreamExt};
use std::{collections::BTreeMap, path::PathBuf};
use tokio::io::AsyncReadExt;

use dcmfx::{
  core::{
    DataElementTag, DataSet, DcmfxError, data_element_value::unique_identifier,
    dictionary,
  },
  p10::{
    IoAsyncRead, P10Error, P10PartialDataSetReader, dicom_dir,
    dicom_dir::DicomDirError, uids::DCMFX_ROOT_UID_PREFIX,
  },
};
use dcmfx_cli::utils::OutputTarget;

pub const ABOUT: &str = "Archives one or more DICOM P10 files into a directory \
  or ZIP file alongside a DICOMDIR file";

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
    help_heading = "Output",
    help = "The path structure to use for the DICOM files in the output ZIP \
      file. Directories are numbered in the order that their first input DICOM \
      is archived.",
    default_value_t = args::PathStructure::Flat
  )]
  path_structure: args::PathStructure,

  #[arg(
    long,
    help_heading = "Output",
    help = "The file extension to append to the names of the DICOM files in \
      the ZIP file, e.g. 'dcm'. A leading period is optional. By default no \
      file extension is appended. Note that file names containing a period \
      aren't permitted by the DICOM standard's definition of a File ID."
  )]
  extension: Option<String>,

  #[arg(
    long,
    help_heading = "Output",
    help = "The File-set ID to put in the DICOMDIR file. This is a \
      human-readable identifier for the file-set, and must be no more than 16 \
      characters from the DICOM default character repertoire.",
    default_value_t = String::new()
  )]
  file_set_id: String,

  #[arg(
    long,
    help_heading = "ZIP Compression",
    help = "The compression method to use when outputting a ZIP file.",
    default_value_t = args::ZipCompressionMethod::Store
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
  match create_archive(args).await {
    Ok(()) => Ok(()),

    Err((error, task_description)) => {
      error.print(&task_description);
      Err(())
    }
  }
}

async fn create_archive(
  args: ArchiveArgs,
) -> Result<(), (ArchiveError, String)> {
  let task_description =
    format!("creating archive \"{}\"", args.output_filename.display());

  let mut input_sources = args.input.base.input_sources().await;

  OutputTarget::set_overwrite(args.overwrite);

  let output_target = OutputTarget::new(&args.output_filename).await;

  let output_stream = output_target
    .open_write_stream(!output_target.is_stdout())
    .await
    .map_err(|e| (e.into(), task_description.clone()))?;

  let mut output_stream = output_stream.lock().await;
  let mut zip_file_writer = ZipFileWriter::with_tokio(&mut *output_stream);

  let mut file_name_generator = FileNameGenerator::new(&args);
  let mut dicom_files = vec![];

  while let Some(input_source) = input_sources.next().await {
    let input_task_description = format!("archiving \"{input_source}\"");

    let mut input_stream = input_source
      .open_read_stream()
      .await
      .map_err(|e| (e.into(), input_task_description.clone()))?;

    // Read the start of the input, which provides the data elements needed for
    // the DICOMDIR as well as the file's name in the ZIP archive. The result is
    // absent when the input doesn't contain DICOM P10 data and such inputs are
    // being ignored.
    let Some((leading_bytes, data_set)) =
      read_partial_data_set(&mut input_stream, &args)
        .await
        .map_err(|e| (e, input_task_description.clone()))?
    else {
      continue;
    };

    let file_name = file_name_generator.next_file_name(&data_set);

    add_entry_to_zip_archive(
      &mut zip_file_writer,
      &mut input_stream,
      &file_name,
      &leading_bytes,
      &args,
    )
    .await
    .map_err(|e| (e, input_task_description.clone()))?;

    dicom_files.push((file_name, data_set));
  }

  add_dicomdir_to_zip_archive(&mut zip_file_writer, &dicom_files, &args)
    .await
    .map_err(|e| (e, "creating DICOMDIR".into()))?;

  zip_file_writer
    .close()
    .await
    .map_err(|e| (e.into(), task_description.clone()))?;

  output_target
    .commit(&mut output_stream)
    .await
    .map_err(|e| (e.into(), task_description))?;

  Ok(())
}

/// Reads the data elements needed for the DICOMDIR and for determining a DICOM
/// file's name in the ZIP archive out of the start of a DICOM P10 stream. The
/// leading bytes that were read out of the stream are returned along with them
/// so that they're able to be written into the ZIP archive.
///
/// If the stream doesn't contain DICOM P10 data then an error is returned,
/// unless such inputs are being ignored, in which case nothing is returned.
///
async fn read_partial_data_set<S: IoAsyncRead>(
  input_stream: &mut S,
  args: &ArchiveArgs,
) -> Result<Option<(Vec<u8>, DataSet)>, ArchiveError> {
  // Requiring the 'DICM' prefix means an error is raised for inputs that aren't
  // DICOM P10 data. This is checked prior to creating the ZIP entry so that
  // nothing is added to the ZIP archive for such inputs.
  let read_config = args.input.p10_read_config().require_dicm_prefix(true);

  let mut partial_data_set_reader = P10PartialDataSetReader::new(
    &dicom_dir::DICOMDIR_DATA_ELEMENT_TAGS,
    Some(read_config),
  );

  let mut leading_bytes = vec![];
  let mut buffer = vec![0u8; 256 * 1024];

  // Read until all of the needed data elements have been read. They all appear
  // prior to the pixel data, so the whole input isn't read.
  while !partial_data_set_reader.is_complete() {
    let bytes_read = input_stream.read(&mut buffer).await?;
    leading_bytes.extend_from_slice(&buffer[..bytes_read]);

    let result = partial_data_set_reader.write_bytes(
      Bytes::copy_from_slice(&buffer[..bytes_read]),
      bytes_read == 0,
    );

    match result {
      Ok(()) => (),

      Err(P10Error::DicmPrefixNotPresent)
        if args.input.ignore_non_dicom_inputs =>
      {
        return Ok(None);
      }

      Err(e) => return Err(e.into()),
    }

    if bytes_read == 0 {
      break;
    }
  }

  Ok(Some((
    leading_bytes,
    partial_data_set_reader.into_data_set(),
  )))
}

/// Adds a single DICOM P10 stream to the in-progress ZIP archive. The already
/// read bytes from the start of the stream are written first, followed by the
/// remainder of the stream.
///
async fn add_entry_to_zip_archive<
  W: tokio::io::AsyncWrite + Unpin,
  S: IoAsyncRead,
>(
  zip_file_writer: &mut async_zip::tokio::write::ZipFileWriter<W>,
  input_stream: &mut S,
  file_name: &str,
  leading_bytes: &[u8],
  args: &ArchiveArgs,
) -> Result<(), ArchiveError> {
  let builder = async_zip::ZipEntryBuilder::new(
    file_name.into(),
    args.zip_compression_method.to_async_zip_compression(),
  )
  .deflate_option(args.deflate_compression_level.to_async_zip_deflate_option());

  let mut entry = zip_file_writer.write_entry_stream(builder).await?;

  entry.write_all(leading_bytes).await?;

  let mut buffer = vec![0u8; 256 * 1024];

  loop {
    let bytes_read = input_stream.read(&mut buffer).await?;
    if bytes_read == 0 {
      break;
    }

    entry.write_all(&buffer[..bytes_read]).await?;
  }

  entry.close().await?;

  Ok(())
}

/// Generates the names of archived DICOM files. Patients, studies, and series
/// are numbered in the order they're first seen, and the files in each
/// directory are numbered from zero.
///
struct FileNameGenerator {
  file_structure: args::PathStructure,
  extension: String,
  patient_indices: BTreeMap<String, usize>,
  study_indices: BTreeMap<String, usize>,
  series_indices: BTreeMap<String, usize>,
  file_counts: BTreeMap<String, usize>,
}

impl FileNameGenerator {
  fn new(args: &ArchiveArgs) -> Self {
    // A leading period on the file extension is optional
    let extension = args
      .extension
      .as_deref()
      .unwrap_or_default()
      .trim_start_matches('.');

    let extension = if extension.is_empty() {
      String::new()
    } else {
      format!(".{extension}")
    };

    Self {
      file_structure: args.path_structure,
      extension,
      patient_indices: BTreeMap::new(),
      study_indices: BTreeMap::new(),
      series_indices: BTreeMap::new(),
      file_counts: BTreeMap::new(),
    }
  }

  /// Returns the name of the next DICOM file to be added to the ZIP archive.
  ///
  fn next_file_name(&mut self, data_set: &DataSet) -> String {
    use args::PathStructure;

    let mut directory = "DICOM".to_string();

    if self.file_structure == PathStructure::PatientStudySeries {
      let index = Self::index_of(
        &mut self.patient_indices,
        data_set,
        dictionary::PATIENT_ID.tag,
      );
      directory.push_str(&format!("/PA{index:06}"));
    }

    if self.file_structure != PathStructure::Flat {
      let index = Self::index_of(
        &mut self.study_indices,
        data_set,
        dictionary::STUDY_INSTANCE_UID.tag,
      );
      directory.push_str(&format!("/ST{index:06}"));
    }

    if self.file_structure == PathStructure::StudySeries
      || self.file_structure == PathStructure::PatientStudySeries
    {
      let index = Self::index_of(
        &mut self.series_indices,
        data_set,
        dictionary::SERIES_INSTANCE_UID.tag,
      );
      directory.push_str(&format!("/SE{index:06}"));
    }

    let file_count = self.file_counts.entry(directory.clone()).or_default();
    let index = *file_count;
    *file_count += 1;

    format!("{directory}/IM{index:06}{}", self.extension)
  }

  /// Returns the index for the specified data element value, assigning it the
  /// next available index if it hasn't been seen before.
  ///
  fn index_of(
    indices: &mut BTreeMap<String, usize>,
    data_set: &DataSet,
    tag: DataElementTag,
  ) -> usize {
    let value = data_set.get_string(tag).unwrap_or_default().to_string();
    let next_index = indices.len();

    *indices.entry(value).or_insert(next_index)
  }
}

/// Creates the DICOMDIR file and adds it to the ZIP file.
///
async fn add_dicomdir_to_zip_archive<W: tokio::io::AsyncWrite + Unpin>(
  zip_file_writer: &mut async_zip::tokio::write::ZipFileWriter<W>,
  dicom_files: &[(String, DataSet)],
  args: &ArchiveArgs,
) -> Result<(), ArchiveError> {
  let entry_builder = async_zip::ZipEntryBuilder::new(
    "DICOMDIR".into(),
    args.zip_compression_method.to_async_zip_compression(),
  )
  .deflate_option(args.deflate_compression_level.to_async_zip_deflate_option());

  // Generate a new Media Storage SOP Instance UID to identify the DICOMDIR
  let media_storage_sop_instance_uid =
    unique_identifier::new(DCMFX_ROOT_UID_PREFIX).unwrap();

  let dicomdir_buffer = dicom_dir::create(
    dicom_files,
    &media_storage_sop_instance_uid,
    &args.file_set_id,
    None,
  )?;

  zip_file_writer
    .write_entry_whole(entry_builder, &dicomdir_buffer)
    .await?;

  Ok(())
}

mod args {
  use clap::ValueEnum;

  #[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
  pub enum PathStructure {
    Flat,
    Study,
    StudySeries,
    PatientStudySeries,
  }

  impl core::fmt::Display for PathStructure {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
      match self {
        Self::Flat => write!(f, "flat"),
        Self::Study => write!(f, "study"),
        Self::StudySeries => write!(f, "study-series"),
        Self::PatientStudySeries => write!(f, "patient-study-series"),
      }
    }
  }

  #[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
  pub enum ZipCompressionMethod {
    Store,
    Deflate,
  }

  impl ZipCompressionMethod {
    pub fn to_async_zip_compression(self) -> async_zip::Compression {
      match self {
        Self::Store => async_zip::Compression::Stored,
        Self::Deflate => async_zip::Compression::Deflate,
      }
    }
  }

  impl core::fmt::Display for ZipCompressionMethod {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
      match self {
        Self::Store => write!(f, "store"),
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
    pub fn to_async_zip_deflate_option(self) -> async_zip::DeflateOption {
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

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
enum ArchiveError {
  P10Error(P10Error),
  DicomDirError(DicomDirError),
  ZipError(async_zip::error::ZipError),
}

impl From<P10Error> for ArchiveError {
  fn from(error: P10Error) -> Self {
    Self::P10Error(error)
  }
}

impl From<DicomDirError> for ArchiveError {
  fn from(error: DicomDirError) -> Self {
    Self::DicomDirError(error)
  }
}

impl From<async_zip::error::ZipError> for ArchiveError {
  fn from(error: async_zip::error::ZipError) -> Self {
    Self::ZipError(error)
  }
}

impl From<std::io::Error> for ArchiveError {
  fn from(error: std::io::Error) -> Self {
    Self::ZipError(error.into())
  }
}

impl DcmfxError for ArchiveError {
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    match self {
      Self::P10Error(e) => e.to_lines(task_description),
      Self::DicomDirError(e) => e.to_lines(task_description),

      Self::ZipError(e) => vec![
        format!("ZIP error {task_description}"),
        "".to_string(),
        format!("  Error: {e}"),
      ],
    }
  }
}
