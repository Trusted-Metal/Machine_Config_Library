// Phase 3.8 — CLI entry point (`machine-config-cli`)
// Single subcommand: export-json <path> [--include-binary]
// Outputs JSON to stdout; exits 0 on success, 1 on any error.
// This is the interface called by cross_check.py (Phase 5).

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use machine_config::reader::MachineConfigReader;

#[derive(Parser)]
#[command(
    name = "machine-config-cli",
    about = "Inspect and convert machine config HDF5 files.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Export a machine config HDF5 file as JSON (stdout).
    ExportJson {
        /// Path to the .h5 file.
        path: PathBuf,
        /// Include binary fields (correction_data, raw_bytes) in the output.
        #[arg(long)]
        include_binary: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::ExportJson { path, include_binary } => {
            let reader = MachineConfigReader::open(&path)?;
            let json = reader.to_json(true, include_binary)?;
            println!("{json}");
        }
    }
    Ok(())
}

