//! Entry point for DCMfx's CLI tool.

mod commands;

use clap::{Parser, Subcommand};

use commands::{
  extract_pixel_data_command, modify_command, print_command, to_dcm_command,
  to_json_command,
};

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
  #[command(about = extract_pixel_data_command::ABOUT)]
  ExtractPixelData(extract_pixel_data_command::ExtractPixelDataArgs),

  #[command(about = modify_command::ABOUT)]
  Modify(modify_command::ModifyArgs),

  #[command(about = print_command::ABOUT)]
  Print(print_command::PrintArgs),

  #[command(about = to_dcm_command::ABOUT)]
  ToDcm(to_dcm_command::ToDcmArgs),

  #[command(about = to_json_command::ABOUT)]
  ToJson(to_json_command::ToJsonArgs),
}

fn main() -> Result<(), ()> {
  let cli = Cli::parse();

  let started_at = std::time::Instant::now();

  let r = match &cli.command {
    Commands::ExtractPixelData(args) => extract_pixel_data_command::run(args),
    Commands::Modify(args) => modify_command::run(args),
    Commands::Print(args) => print_command::run(args),
    Commands::ToDcm(args) => to_dcm_command::run(args),
    Commands::ToJson(args) => to_json_command::run(args),
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
