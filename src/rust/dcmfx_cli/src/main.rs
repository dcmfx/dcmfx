//! Entry point for DCMfx's CLI tool.

mod args;
mod commands;
mod input_source;
mod mp4_encoder;
mod utils;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use commands::{
  dcm_to_json_command, get_pixel_data_command, json_to_dcm_command,
  list_command, modify_command, print_command,
};
use input_source::InputSource;

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

  #[command(about = list_command::ABOUT)]
  List(list_command::ListArgs),
}

fn main() -> Result<(), ()> {
  let cli = Cli::parse();

  let started_at = std::time::Instant::now();

  let r = match cli.command {
    Commands::GetPixelData(mut args) => get_pixel_data_command::run(&mut args),
    Commands::Modify(mut args) => modify_command::run(&mut args),
    Commands::Print(mut args) => print_command::run(&mut args),
    Commands::JsonToDcm(mut args) => json_to_dcm_command::run(&mut args),
    Commands::DcmToJson(mut args) => dcm_to_json_command::run(&mut args),
    Commands::List(args) => list_command::run(&args),
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
}
