//! Entry point for DCMfx's CLI tool.

mod commands;
mod input_source;
mod mp4_encoder;
mod photometric_interpretation_arg;
mod transfer_syntax_arg;
mod utils;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use commands::{
  dcm_to_json_command, get_pixel_data_command, json_to_dcm_command,
  modify_command, print_command,
};
use input_source::{InputSource, get_input_sources};

#[derive(Parser)]
#[command(
  name = "dcmfx",
  bin_name = "dcmfx",
  version = env!("CARGO_PKG_VERSION"),
  about = "DCMfx is a CLI tool for working with DICOM and DICOM JSON",
  max_term_width = 80
)]
struct Cli {
  #[command(subcommand)]
  command: Commands,

  #[arg(
    long,
    default_value_t = false,
    help = "Write timing and memory stats to stderr on exit"
  )]
  print_stats: bool,
}

#[derive(Subcommand)]
enum Commands {
  #[command(about = get_pixel_data_command::ABOUT)]
  GetPixelData(get_pixel_data_command::GetPixelDataArgs),

  #[command(about = modify_command::ABOUT)]
  Modify(modify_command::ModifyArgs),

  #[command(about = print_command::ABOUT)]
  Print(print_command::PrintArgs),

  #[command(about = json_to_dcm_command::ABOUT)]
  JsonToDcm(json_to_dcm_command::ToDcmArgs),

  #[command(about = dcm_to_json_command::ABOUT)]
  DcmToJson(dcm_to_json_command::ToJsonArgs),
}

fn main() -> Result<(), ()> {
  let cli = Cli::parse();

  let started_at = std::time::Instant::now();

  let r = match &cli.command {
    Commands::GetPixelData(args) => get_pixel_data_command::run(args),
    Commands::Modify(args) => modify_command::run(args),
    Commands::Print(args) => print_command::run(args),
    Commands::JsonToDcm(args) => json_to_dcm_command::run(args),
    Commands::DcmToJson(args) => dcm_to_json_command::run(args),
  };

  if cli.print_stats {
    #[cfg(not(windows))]
    let peak_memory_mb = get_peak_memory_usage() as f64 / (1024.0 * 1024.0);

    eprintln!();
    eprintln!("-----");
    eprintln!(
      "Time elapsed:      {:.2} seconds",
      started_at.elapsed().as_secs_f64()
    );

    #[cfg(not(windows))]
    eprintln!("Peak memory usage: {:.0} MiB", peak_memory_mb);
  }

  r
}

#[cfg(not(windows))]
fn get_peak_memory_usage() -> i64 {
  let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
  unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };

  let mut max = usage.ru_maxrss;

  // On Linux, ru_maxrss is in KiB
  if std::env::consts::OS == "linux" {
    max *= 1024;
  }

  max
}

/// Validates the --output-filename and --output-directory arguments for the
/// given input sources.
///
pub fn validate_output_args(
  input_sources: &[InputSource],
  output_filename: &Option<PathBuf>,
  output_directory: &Option<PathBuf>,
) {
  // Check that --output-directory is a valid directory
  if let Some(output_directory) = output_directory {
    if !output_directory.is_dir() {
      eprintln!(
        "Error: '{}' is not a valid directory",
        output_directory.display()
      );
      std::process::exit(1);
    }
  }

  // Check that --output-filename and --output-directory aren't both specified
  if output_filename.is_some() && output_directory.is_some() {
    eprintln!(
      "Error: --output-filename and --output-directory can't be specified \
       together"
    );
    std::process::exit(1);
  }

  // Check that --output-filename isn't specified when there's more than one
  // input source
  if input_sources.len() > 1 && output_filename.is_some() {
    eprintln!(
      "Error: --output-filename is not valid when there are multiple input \
       files"
    );
    std::process::exit(1);
  }
}
