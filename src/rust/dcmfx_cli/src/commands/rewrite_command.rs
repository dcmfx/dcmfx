use std::path::PathBuf;

use clap::Args;

use dcmfx::{core::*, p10::*};

use crate::utils::{self, InputSource, OutputTarget};

pub const ABOUT: &str = "Rewrites DICOM P10 files to correct and recover their \
  data";

pub const LONG_ABOUT: &str = "Rewrites DICOM P10 files in order to standardize \
  their data, correct certain non-conformances where possible, and optionally \
  partially recover invalid or corrupted DICOM P10 files.\n\
  \n\
  Rewriting converts input DICOM P10 files to UTF-8, changes all sequences to \
  undefined length, reorders data elements that aren't in ascending order, \
  corrects missing File Meta Information Group Length, and optionally recovers \
  invalid or corrupted files by rewriting them up to the point that the data \
  remained valid and readable.\n\
  \n\
  Each DICOM file must be fully read into memory to be rewritten. DICOM \
  streaming is not supported for rewrites due to reordering corrections that \
  may be needed.";

#[derive(Args)]
pub struct RewriteArgs {
  #[arg(
    long,
    help = "The number of concurrent tasks to use. Defaults to the number of \
      CPU cores.",
    default_value_t = {num_cpus::get()}
  )]
  concurrency: usize,

  #[command(flatten)]
  input: crate::args::input_args::P10InputArgs,

  #[arg(
    long,
    help_heading = "Input",
    help = "Whether to rewrite the input files in place, i.e. overwrite them \
      with the rewritten version rather than write it to a new file.\n\
      \n\
      If there is an error during in-place modification of a file then it will \
      not be altered.\n\
      \n\
      WARNING: rewriting in-place is a potentially irreversible operation.",
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
    help = "The value of the Implementation Version Name data element in \
      output DICOM P10 files. The value must conform to the specification of \
      the SS (Short String) value representation.",
    default_value_t = uids::DCMFX_IMPLEMENTATION_VERSION_NAME.to_string(),
  )]
  implementation_version_name: String,

  #[arg(
    long,
    help_heading = "Output",
    help = "Instead of reporting an error on invalid DICOM data, rewrite all \
      data that was successfully read, and ignore the invalid data. This can \
      be used to partially recover corrupted DICOM data, however all \
      unreadable data is lost in the rewrite."
  )]
  partial_rewrite_on_invalid_data: bool,
}

pub async fn run(args: RewriteArgs) -> Result<(), ()> {
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
        crate::utils::exit_with_error(
          "--in-place can't be used with stdin as an input",
          "",
        );
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

      match rewrite_input_source(&input_source, output_target, &args).await {
        Ok(()) => Ok(()),

        Err(P10Error::DicmPrefixNotPresent) if args.input.ignore_invalid => {
          Ok(())
        }

        Err(e) => {
          let task_description = format!("rewriting \"{input_source}\"");
          Err(e.to_lines(&task_description))
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

async fn rewrite_input_source(
  input_source: &InputSource,
  output_target: OutputTarget,
  args: &RewriteArgs,
) -> Result<(), P10Error> {
  if args.in_place {
    println!("Rewriting \"{input_source}\" in place …");
  } else if let Some(output_filename) = &args.output_filename {
    if !output_target.is_stdout() {
      println!(
        "Rewriting \"{input_source}\" => \"{}\" …",
        output_filename.display()
      );
    }
  } else {
    println!(
      "Rewriting \"{input_source}\" => \"{}\" …",
      output_target.specified_path().display()
    );
  }

  // Construct read config
  let read_config = args
    .input
    .p10_read_config()
    .require_dicm_prefix(args.input.ignore_invalid)
    .require_ordered_data_elements(false);

  // Open input stream
  let mut input_stream = input_source.open_read_stream().await?;

  // Read into in-memory data set
  let ds =
    match dcmfx::p10::read_stream_async(&mut input_stream, Some(read_config))
      .await
    {
      Ok(ds) => ds,

      Err((
        P10Error::DataInvalid { .. } | P10Error::DataEndedUnexpectedly { .. },
        mut data_set_builder,
      )) if args.partial_rewrite_on_invalid_data => {
        data_set_builder.force_end();
        data_set_builder.final_data_set().unwrap()
      }

      Err((e, _)) => return Err(e),
    };

  // Open output write stream
  let output_stream_handle = output_target.open_write_stream(false).await?;

  // Get exclusive access to the output stream
  let mut output_stream = output_stream_handle.lock().await;

  // Setup write config
  let write_config = P10WriteConfig::default()
    .implementation_version_name(args.implementation_version_name.clone());

  // Write P10 output
  ds.write_p10_stream_async(&mut *output_stream, Some(write_config))
    .await?;

  output_target.commit(&mut output_stream).await
}
