use std::path::PathBuf;

use clap::Args;

use dcmfx::{
  core::*,
  p10::*,
  pixel_data::{transforms::*, *},
};

use crate::{
  args::{
    photometric_interpretation_arg::{
      PhotometricInterpretationColorArg, PhotometricInterpretationMonochromeArg,
    },
    planar_configuration_arg::PlanarConfigurationArg,
    transfer_syntax_arg::TransferSyntaxArg,
  },
  utils::{self, InputSource, OutputTarget},
};

pub const ABOUT: &str = "Modifies the content of DICOM P10 files";

#[derive(Args)]
pub struct ModifyArgs {
  #[arg(
    long,
    help = "The number of concurrent tasks to use. Defaults to the number of CPU
      cores.",
    default_value_t = {num_cpus::get()}
  )]
  concurrency: usize,

  #[command(flatten)]
  input: crate::args::input_args::P10InputArgs,

  #[arg(
    long,
    help_heading = "Input",
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
    help_heading = "Output",
    help = "The name of the DICOM P10 output file. Specify '-' to write to \
      stdout."
  )]
  output_filename: Option<PathBuf>,

  #[arg(
    long,
    short = 'd',
    help_heading = "Output",
    help = "The directory to write output files into. The names of the output \
      DICOM P10 files will be the same as the input files."
  )]
  output_directory: Option<PathBuf>,

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
    help = "The zlib compression level to use when outputting to the 'Deflated \
      Explicit VR Little Endian' and 'Deflated Image Frame Compression' \
      transfer syntaxes. The level ranges from 0, meaning no compression, \
      through to 9, which gives the best compression at the cost of speed.",
    default_value_t = 6,
    value_parser = clap::value_parser!(u32).range(0..=9),
  )]
  zlib_compression_level: u32,

  #[arg(
    long,
    help_heading = "Data Set Content",
    help = "A DICOM JSON data set to merge into the output DICOM P10 data \
      sets. If this specifies data elements already present in the input data \
      set then the data elements specified by this argument will replace those \
      existing values.",
    value_parser = crate::args::parse_dicom_json_data_set,
  )]
  merge_dicom_json: Option<DataSet>,

  #[arg(
    long = "delete",
    value_name = "DATA_ELEMENT_TAG",
    help_heading = "Data Set Content",
    help = "A data element tag to delete and not include in the output DICOM \
      P10 file. This argument can be specified multiple times to delete \
      multiple tags.",
    value_parser = crate::args::parse_data_element_tag,
  )]
  deletions: Vec<DataElementTag>,

  #[arg(
    long,
    help_heading = "Data Set Content",
    help = "Delete private data elements, which are those with a tag group \
      that's an odd number.",
    default_value_t = false
  )]
  delete_private: bool,

  #[arg(
    long,
    help_heading = "Data Set Content",
    help = "Anonymize the data set file by removing all patient data elements, \
      other potentially identifying data elements, as well as private data \
      elements. Note that this option does not remove identifying information \
      baked into the pixel data, however such data may be able to be cropped \
      out using --crop",
    default_value_t = false
  )]
  anonymize: bool,

  #[arg(
    long,
    help_heading = "Data Set Content",
    help = "The value of the Implementation Version Name data element in \
      output DICOM P10 files. The value must conform to the specification of \
      the SS (Short String) value representation.",
    default_value_t = uids::DCMFX_IMPLEMENTATION_VERSION_NAME.to_string(),
  )]
  implementation_version_name: String,

  #[arg(
    long,
    short,
    help_heading = "Transcoding",
    help = "The transfer syntax for the output DICOM P10 file. Pixel data will \
      be automatically transcoded. For some transfer syntaxes additional \
      arguments are available to control pixel data compression."
  )]
  transfer_syntax: Option<TransferSyntaxArg>,

  #[arg(
    long,
    short,
    help_heading = "Transcoding",
    help = "When transcoding pixel data to a lossy transfer syntax, specifies \
      the compression quality in the range 1-100. A quality of 100 does not \
      result in lossless compression.\n\
      \n\
      The quality value only applies when encoding into the following transfer \
      syntaxes:\n\
      \n\
      - JPEG Baseline 8-bit\n\
      - JPEG Extended 12-bit\n\
      - JPEG-LS Lossy (Near-Lossless)\n\
      - JPEG 2000 (Lossy)\n\
      - High-Throughput JPEG 2000 (Lossy)\n\
      - JPEG XL\n\
      - JPEG XL JPEG Recompression\n\
      \n\
      Default value: 90",
    value_parser = clap::value_parser!(u8).range(1..=100),
  )]
  quality: Option<u8>,

  #[arg(
    long,
    short,
    help_heading = "Transcoding",
    help = "When transcoding pixel data to a compressed transfer syntax, \
      specifies the effort to put into the compression process, in the range \
      1-10. Higher values allow the compressor to take more processing time in \
      order to try and achieve a better compression ratio at the same \
      quality.\n\
      \n\
      The effort value only applies when encoding into the following transfer \
      syntaxes:\n\
      \n\
      - JPEG XL Lossless\n\
      - JPEG XL\n\
      \n\
      Default value: 7",
    value_parser = clap::value_parser!(u8).range(1..=10),
  )]
  effort: Option<u8>,

  #[arg(
    long,
    help_heading = "Transcoding",
    help = "When transcoding monochrome pixel data using --transfer-syntax, \
      specifies the photometric interpretation to be used by the output DICOM \
      P10 files. This option has no effect on color pixel data.\n\
      \n\
      This option is ignored when transcoding between the 'JPEG XL JPEG \
      Recompression' and 'JPEG Baseline 8-bit' transfer syntaxes."
  )]
  photometric_interpretation_monochrome:
    Option<PhotometricInterpretationMonochromeArg>,

  #[arg(
    long,
    help_heading = "Transcoding",
    help = "When transcoding color pixel data using --transfer-syntax, \
      specifies the photometric interpretation to be used by the transcoded \
      pixel data. This option has no effect on monochrome pixel data.\n\
      \n\
      When the output transfer syntax is 'JPEG Baseline 8-bit' or 'JPEG \
      Extended 12-bit' the output photometric interpretation defaults to \
      'YBR_FULL' if the color data is not YBR following decoding. This is \
      because the 'RGB' photometric interpretation in JPEG is uncommon, and a \
      YBR color space usually yields better compression ratios.\n\
      \n\
      When the output transfer syntax is JPEG 2000 the output photometric \
      interpretation defaults to 'YBR_RCT' for lossless encoding, and \
      'YBR_ICT' for lossy encoding. These two photometric interpretations are \
      generally preferred in JPEG 2000, however others may be used if there is \
      a need to compress with no risk of loss from color space conversions.\n\
      \n\
      When the output transfer syntax is 'JPEG XL' or 'JPEG XL Lossless Only' \
      the output photometric interpretation defaults to 'RGB' for lossless \
      encoding, and 'XYB' for lossy encoding.\n\
      \n\
      For all other output transfer syntaxes there is no default output \
      photometric interpretation, however the output photometric \
      interpretation may differ from the input for the following reasons:\n\
      \n\
      1. If the output transfer syntax doesn't support 'PALETTE_COLOR' then \
         palette color image data will be automatically expanded to 'RGB'.\n\
      \n\
      2. If the output transfer syntax doesn't support 'YBR_FULL_422' then the \
         color image's data will be automatically expanded to 'YBR_FULL'.\n\
      \n\
      This option is ignored when transcoding between the 'JPEG XL JPEG \
      Recompression' and 'JPEG Baseline 8-bit' transfer syntaxes."
  )]
  photometric_interpretation_color: Option<PhotometricInterpretationColorArg>,

  #[arg(
    long,
    help_heading = "Transcoding",
    help = "When transcoding color pixel data using --transfer-syntax, \
      specifies the planar configuration to be used by the transcoded pixel \
      data. This is only used when encoding color pixel data into the \
      following transfer syntaxes:\n\
      \n\
      - Implicit VR Little Endian\n\
      - Explicit VR Little Endian\n\
      - Encapsulated Uncompressed Explicit VR Little Endian\n\
      - Deflated Explicit VR Little Endian\n\
      - Explicit VR Big Endian\n\
      - Deflated Image Frame Compression"
  )]
  planar_configuration: Option<PlanarConfigurationArg>,

  #[arg(
    long,
    help_heading = "Transcoding",
    help = "When transcoding pixel data using --transfer-syntax, specifies a \
      crop to apply to the pixel data. The crop is specified as \
      'x,y[,(width_or_right)[,(height_or_bottom)]]'. The last two values are \
      optional, and if positive they specify the width and height of the crop \
      rectangle, however if they are zero or negative then they specify an \
      offset from the right and bottom edges of the pixel data respectively.\n\
      \n\
      This option is ignored when transcoding between the 'JPEG XL JPEG \
      Recompression' and 'JPEG Baseline 8-bit' transfer syntaxes."
  )]
  crop: Option<CropRect>,

  #[command(flatten)]
  decoder: crate::args::decoder_args::DecoderArgs,
}

impl ModifyArgs {
  fn pixel_data_encode_config(&self) -> PixelDataEncodeConfig {
    let mut config = PixelDataEncodeConfig::default();

    config.set_quality(self.quality.unwrap_or(90));
    config.set_effort(self.effort.unwrap_or(7));
    config.set_zlib_compression_level(self.zlib_compression_level);

    config
  }
}

enum ModifyCommandError {
  P10Error(P10Error),
  P10PixelDataTranscodeTransformError(P10PixelDataTranscodeTransformError),
}

pub async fn run(args: ModifyArgs) -> Result<(), ()> {
  if (args.output_filename.is_some() as u8
    + args.output_directory.is_some() as u8
    + args.in_place as u8)
    != 1
  {
    eprintln!(
      "Error: Exactly one of --output-filename, --output-directory, or \
       --in-place must be specified"
    );
    return Err(());
  }

  if args.transfer_syntax.is_none() {
    if args.photometric_interpretation_monochrome.is_some() {
      eprintln!(
        "Error: The --photometric-interpretation-monochrome option is only \
         valid when --transfer-syntax is specified"
      );
      return Err(());
    }

    if args.photometric_interpretation_color.is_some() {
      eprintln!(
        "Error: The --photometric-interpretation-color option is only valid \
         when --transfer-syntax is specified"
      );
      return Err(());
    }

    if args.quality.is_some() {
      eprintln!(
        "Error: The --quality option is only valid when --transfer-syntax is \
         specified"
      );
      return Err(());
    }

    if args.effort.is_some() {
      eprintln!(
        "Error: The --effort option is only valid when --transfer-syntax is \
         specified"
      );
      return Err(());
    }

    if args.crop.is_some() {
      eprintln!(
        "Error: The --crop option is only valid when --transfer-syntax is \
         specified"
      );
      return Err(());
    }
  }

  crate::validate_output_args(
    args.output_filename.as_ref(),
    args.output_directory.as_ref(),
  )
  .await;

  OutputTarget::set_overwrite(args.overwrite || args.in_place);

  let input_sources = args.input.base.input_sources().await;

  let result = utils::run_tasks(
    args.concurrency,
    input_sources,
    async |input_source: InputSource| {
      if args.in_place
        && let InputSource::Stdin = input_source
      {
        eprintln!("Error: --in-place can't be used with stdin as an input");
        std::process::exit(1);
      }

      let output_target = if args.in_place {
        OutputTarget::new(input_source.specified_path()).await
      } else if let Some(output_filename) = &args.output_filename {
        OutputTarget::new(output_filename).await
      } else {
        OutputTarget::from_input_source(
          &input_source,
          "",
          &args.output_directory,
        )
        .await
      };

      match modify_input_source(&input_source, output_target, &args).await {
        Ok(()) => Ok(()),

        Err(ModifyCommandError::P10Error(P10Error::DicmPrefixNotPresent))
          if args.input.ignore_invalid =>
        {
          Ok(())
        }

        Err(e) => {
          let task_description = format!("modifying \"{input_source}\"");

          Err(match e {
            ModifyCommandError::P10Error(e) => e.to_lines(&task_description),
            ModifyCommandError::P10PixelDataTranscodeTransformError(e) => {
              e.to_lines(&task_description)
            }
          })
        }
      }
    },
  )
  .await;

  match result {
    Ok(()) => Ok(()),

    Err(lines) => {
      error::print_error_lines(&lines);
      Err(())
    }
  }
}

async fn modify_input_source(
  input_source: &InputSource,
  output_target: OutputTarget,
  args: &ModifyArgs,
) -> Result<(), ModifyCommandError> {
  if args.in_place {
    println!("Modifying \"{input_source}\" in place …");
  } else if let Some(output_filename) = &args.output_filename {
    if !output_target.is_stdout() {
      println!(
        "Modifying \"{input_source}\" => \"{}\" …",
        output_filename.display()
      );
    }
  } else {
    println!(
      "Modifying \"{input_source}\" => \"{}\" …",
      output_target.specified_path().display()
    );
  }

  // Create an insert transform for merging in another data set, if needed
  let insert_transform = args
    .merge_dicom_json
    .as_ref()
    .map(|merge_dicom_json| P10InsertTransform::new(merge_dicom_json.clone()));

  // Create a filter transform for anonymization and tag deletion, if needed
  let deletions = args.deletions.clone();
  let delete_private = args.delete_private;
  let anonymize = args.anonymize;
  let filter_transform = if anonymize || !deletions.is_empty() || delete_private
  {
    Some(P10FilterTransform::new(Box::new(
      move |tag, vr, _length, _path| {
        if deletions.contains(&tag) {
          return false;
        }

        if delete_private && tag.is_private() {
          return false;
        }

        if anonymize && !dcmfx::anonymize::filter_tag(tag, vr) {
          return false;
        }

        true
      },
    )))
  } else {
    None
  };

  // Setup write config
  let write_config = P10WriteConfig::default()
    .implementation_version_name(args.implementation_version_name.clone())
    .zlib_compression_level(args.zlib_compression_level);

  let mut input_stream = input_source
    .open_read_stream()
    .await
    .map_err(ModifyCommandError::P10Error)?;

  // Open output write stream
  let output_stream_handle = output_target
    .open_write_stream(false)
    .await
    .map_err(ModifyCommandError::P10Error)?;

  // Get exclusive access to the output stream
  let mut output_stream = output_stream_handle.lock().await;

  streaming_rewrite(
    &mut input_stream,
    &mut *output_stream,
    write_config,
    insert_transform,
    filter_transform,
    args,
  )
  .await?;

  output_target
    .commit(&mut output_stream)
    .await
    .map_err(ModifyCommandError::P10Error)
}

/// Rewrites by streaming the tokens of the DICOM P10 straight to the output
/// file.
///
async fn streaming_rewrite<I: IoAsyncRead, O: IoAsyncWrite>(
  input_stream: &mut I,
  output_stream: &mut O,
  write_config: P10WriteConfig,
  mut insert_transform: Option<P10InsertTransform>,
  mut filter_transform: Option<P10FilterTransform>,
  args: &ModifyArgs,
) -> Result<(), ModifyCommandError> {
  // Create read and write contexts
  let read_config = args
    .input
    .p10_read_config()
    .max_token_size(256 * 1024)
    .require_dicm_prefix(args.input.ignore_invalid);

  let mut p10_read_context = P10ReadContext::new(Some(read_config));
  let mut p10_write_context = P10WriteContext::new(Some(write_config));

  let mut pixel_data_transcode_transform = None;

  // Stream P10 tokens from the input stream to the output stream
  loop {
    // Read the next P10 tokens from the input stream
    let mut tokens = dcmfx::p10::read_tokens_from_stream_async(
      input_stream,
      &mut p10_read_context,
      None,
    )
    .await
    .map_err(ModifyCommandError::P10Error)?;

    // If transcoding is active, setup a pixel data transcode transform when the
    // File Meta Information token is received
    if let Some(transfer_syntax_arg) = args.transfer_syntax {
      for token in tokens.iter() {
        let P10Token::FileMetaInformation { data_set } = token else {
          continue;
        };

        let input_transfer_syntax = data_set
          .get_transfer_syntax()
          .unwrap_or(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN);
        let output_transfer_syntax = transfer_syntax_arg
          .as_transfer_syntax()
          .unwrap_or(input_transfer_syntax);

        let photometric_interpretation_monochrome_arg =
          args.photometric_interpretation_monochrome;
        let photometric_interpretation_color_arg =
          args.photometric_interpretation_color;

        let image_data_functions =
          TranscodeImageDataFunctions::standard_behavior(
            output_transfer_syntax,
            Rc::new(move |image_pixel_module| {
              photometric_interpretation_monochrome_arg.and_then(|arg| {
                arg.as_photometric_interpretation(
                  image_pixel_module.pixel_representation(),
                )
              })
            }),
            Rc::new(move |_image_pixel_module| {
              photometric_interpretation_color_arg
                .and_then(|arg| arg.as_photometric_interpretation())
            }),
            args.planar_configuration.map(|a| a.into()),
            args.crop,
            args.quality.is_some(),
          );

        pixel_data_transcode_transform =
          Some(P10PixelDataTranscodeTransform::new(
            output_transfer_syntax,
            args.decoder.pixel_data_decode_config(),
            args.pixel_data_encode_config(),
            Some(image_data_functions),
          ));
      }
    }

    // Pass tokens through the pixel data transcode transform if one is active
    if let Some(transcode_transform) = pixel_data_transcode_transform.as_mut() {
      let mut new_tokens = vec![];

      for token in tokens.iter() {
        new_tokens.extend(
          transcode_transform
            .add_token(token)
            .map_err(ModifyCommandError::P10PixelDataTranscodeTransformError)?,
        );
      }

      // If the pixel data transcode transform is inactive then there is no
      // pixel data in this DICOM to be transcoded. However, some transcodes not
      // involving encapsulated pixel data are still possible, specifically
      // those between any of the four transfer syntaxes listed below. These are
      // done by updating the transfer syntax in the File Meta Information
      // token.
      if !transcode_transform.is_active() {
        let directly_transcodable_transfer_syntaxes = [
          &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN,
          &transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN,
          &transfer_syntax::EXPLICIT_VR_BIG_ENDIAN,
          &transfer_syntax::DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN,
        ];

        let mut output_transfer_syntax =
          transcode_transform.output_transfer_syntax();

        // An output transfer syntax that has encapsulated pixel data is not
        // relevant as this DICOM does not contain pixel data, so automatically
        // drop down to Explicit VR Little Endian
        if output_transfer_syntax.is_encapsulated {
          output_transfer_syntax = &transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN;
        }

        if !directly_transcodable_transfer_syntaxes
          .contains(&transcode_transform.input_transfer_syntax())
          || !directly_transcodable_transfer_syntaxes
            .contains(&output_transfer_syntax)
        {
          return Err(ModifyCommandError::P10PixelDataTranscodeTransformError(
            P10PixelDataTranscodeTransformError::NotSupported {
              details: format!(
                "Transcoding from '{}' to '{}' is not supported",
                transcode_transform.input_transfer_syntax().name,
                output_transfer_syntax.name
              ),
            },
          ));
        }

        // Set the new transfer syntax in the File Meta Information token
        for token in &mut new_tokens {
          token.change_transfer_syntax(output_transfer_syntax)
        }

        // Clear the pixel data transcode transform as it's now inactive
        pixel_data_transcode_transform = None;
      }

      tokens = new_tokens
    }

    // Pass tokens through the insert transform if one is specified
    let tokens = if let Some(insert_transform) = insert_transform.as_mut() {
      let mut new_tokens = vec![];

      for token in tokens.iter() {
        new_tokens.extend(
          insert_transform
            .add_token(token)
            .map_err(ModifyCommandError::P10Error)?,
        );
      }

      Ok(new_tokens)
    } else {
      Ok(tokens)
    }?;

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

    // Write tokens to the output stream
    let ended = dcmfx::p10::write_tokens_to_stream_async(
      &tokens,
      &mut *output_stream,
      &mut p10_write_context,
    )
    .await
    .map_err(ModifyCommandError::P10Error)?;

    // Stop when the end token is received
    if ended {
      break;
    }
  }

  Ok(())
}
