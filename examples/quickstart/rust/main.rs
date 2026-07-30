// Machine Config Library — Rust Quickstart
//
// Source lives in rust/examples/quickstart.rs (Cargo example).
// Run from the repo root:
//
//   cargo run --example quickstart --manifest-path rust/Cargo.toml
//
// Demonstrates the six essential operations:
//   1. Open an HDF5 machine config file
//   2. Read scalar fields (machine name, optical train count, working distance)
//   3. Inspect binary data shape (ClearBox correction grid)
//   4. Write the config to a temporary HDF5 file
//   5. Read the temporary file back
//   6. Assert round-trip fidelity and print PASS / FAIL

use std::path::PathBuf;
use std::process;

use machine_config::reader::MachineConfigReader;
use machine_config::writer::MachineConfigWriter;

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // -------------------------------------------------------------------------
    // Resolve the fixture path relative to the repo root (CARGO_MANIFEST_DIR
    // is rust/, so we go one level up to reach the repo root).
    // -------------------------------------------------------------------------
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has no parent")
        .to_path_buf();
    let fixture = repo_root.join("fixtures").join("reference_config.h5");

    if !fixture.exists() {
        eprintln!("Fixture not found: {}", fixture.display());
        eprintln!("Run from the repo root and ensure fixtures/ is present.");
        process::exit(1);
    }

    // -------------------------------------------------------------------------
    // Step 1 & 2 — Open the file and read scalar fields
    // -------------------------------------------------------------------------
    let reader = MachineConfigReader::open(&fixture)?;
    let config = reader.parse()?;

    println!("=== Machine Config Quickstart ===\n");
    println!("Machine name   : {}", config.meta.machine_name);
    println!("Optical trains : {}", config.optical_trains.len());

    let train0 = &config.optical_trains[0];
    let wd = train0.scanner.working_distance;
    let wd_unit = train0.scanner.working_distance_unit.as_deref().unwrap_or("");
    println!("Working dist   : {} {}   (train 0)", wd.unwrap_or(0.0), wd_unit);

    // -------------------------------------------------------------------------
    // Step 3 — Binary data shape (correction grid)
    // -------------------------------------------------------------------------
    let correction = reader.get_correction_data(0)?;
    println!(
        "Correction grid: {:?}   (train 0)",
        correction.shape
    );

    // -------------------------------------------------------------------------
    // Step 4 — Write to a temporary file
    // -------------------------------------------------------------------------
    println!();
    let tmp_file = tempfile::NamedTempFile::new()?;
    let tmp_path = tmp_file.path().with_extension("h5");
    // Release the handle so the writer can create the file at that path.
    drop(tmp_file);

    MachineConfigWriter::new(&config).write(&tmp_path)?;
    println!(
        "Written to     : {}",
        tmp_path.file_name().unwrap_or_default().to_string_lossy()
    );

    // -------------------------------------------------------------------------
    // Step 5 — Read the temporary file back
    // -------------------------------------------------------------------------
    let config2 = MachineConfigReader::open(&tmp_path)?.parse()?;

    // -------------------------------------------------------------------------
    // Step 6 — Assert round-trip fidelity
    // -------------------------------------------------------------------------
    let mut failures: Vec<String> = Vec::new();

    if config2.meta.machine_name != config.meta.machine_name {
        failures.push(format!(
            "  machine_name: expected {:?}, got {:?}",
            config.meta.machine_name, config2.meta.machine_name
        ));
    }

    if config2.optical_trains.len() != config.optical_trains.len() {
        failures.push(format!(
            "  train_count: expected {}, got {}",
            config.optical_trains.len(),
            config2.optical_trains.len()
        ));
    }

    let wd2 = config2.optical_trains[0].scanner.working_distance;
    if wd2 != wd {
        failures.push(format!(
            "  working_distance: expected {:?}, got {:?}",
            wd, wd2
        ));
    }

    // Clean up the temp file regardless of outcome.
    let _ = std::fs::remove_file(&tmp_path);

    println!();
    if failures.is_empty() {
        println!("PASS");
    } else {
        println!("FAIL");
        for msg in &failures {
            println!("{msg}");
        }
        process::exit(1);
    }

    Ok(())
}
