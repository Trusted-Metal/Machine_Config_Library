// Phase 3.8 — CLI entry point (`machine-config-cli`)
// Subcommands:
//   export-json  <path> [--include-binary]   Read HDF5 → print JSON to stdout
//   write-hdf5   <input.json> <output.h5>    Read canonical JSON → write HDF5
// Exits 0 on success, 1 on any error.
// This is the interface called by cross_check.py (Phase 5).

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use machine_config::models::MachineConfig;
use machine_config::reader::MachineConfigReader;
use machine_config::writer::MachineConfigWriter;
use sha2::{Digest, Sha256};

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
    /// Write an HDF5 machine config file from canonical JSON input.
    ///
    /// The JSON must match the schema produced by `export-json`.
    /// Binary fields (correction grids, .fc3 bytes) are not expected in the
    /// input and will be absent in the output.
    WriteHdf5 {
        /// Path to the canonical JSON input file.
        input: PathBuf,
        /// Path to write the .h5 output (created or overwritten).
        output: PathBuf,
    },
    /// Print the SHA-256 hash of a ClearBox correction grid.
    ///
    /// Values are hashed as flat little-endian float64 bytes, making the
    /// digest platform-independent and directly comparable with the Python
    /// `correction-hash` command.
    CorrectionHash {
        /// Path to the .h5 file.
        path: PathBuf,
        /// 0-indexed optical train number.
        #[arg(long, default_value = "0")]
        train: usize,
        /// Hash the inverse correction grid instead of the forward grid.
        #[arg(long)]
        inverse: bool,
    },
    /// Copy an HDF5 machine config file, preserving all binary data
    /// (correction grids, fc3 bytes).  Used by cross_check Phase 3.5.
    CopyHdf5 {
        /// Path to the source .h5 file.
        input: PathBuf,
        /// Path to write the copied .h5 file (created or overwritten).
        output: PathBuf,
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
        Command::WriteHdf5 { input, output } => {
            let json = std::fs::read_to_string(&input)?;
            let config: MachineConfig = serde_json::from_str(&json)?;
            MachineConfigWriter::new(&config).write(&output)?;
        }
        Command::CorrectionHash { path, train, inverse } => {
            let reader = MachineConfigReader::open(&path)?;
            let cd = if inverse {
                reader.get_inverse_correction_data(train)?
            } else {
                reader.get_correction_data(train)?
            };
            // Hash as flat little-endian f64 bytes — explicit endianness for
            // cross-platform and cross-language determinism.
            let bytes: Vec<u8> = cd.data.iter().flat_map(|&v| v.to_le_bytes()).collect();
            let digest = Sha256::digest(&bytes);
            println!("{digest:x}");
        }
        Command::CopyHdf5 { input, output } => {
            let reader = MachineConfigReader::open(&input)?;
            let config = reader.parse_with_binary()?;
            MachineConfigWriter::new(&config).write(&output)?;
        }
    }
    Ok(())
}

