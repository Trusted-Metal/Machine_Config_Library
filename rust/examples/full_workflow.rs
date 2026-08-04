// Machine Config Library — Rust Full Workflow: Calibration Adjustment
//
// Run from the repo root:
//
//   cargo run --example full_workflow --manifest-path rust/Cargo.toml
//
// Scenario: a field calibration measured new scanner-head positions for both
// optical trains.  Load the current machine config, apply the updated offsets,
// write the modified config to a new file, and verify the changes persisted
// alongside the binary correction data.

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
    // 1. Print pre-calibration summary
    // -------------------------------------------------------------------------
    let reader = MachineConfigReader::open(&fixture)?;
    let mut config = reader.parse()?;

    println!("=== Full Workflow: Calibration Adjustment ===\n");
    println!("Machine : {}", config.meta.machine_name);
    println!("Trains  : {}", config.optical_trains.len());
    println!();
    println!("Before calibration:");
    for (i, train) in config.optical_trains.iter().enumerate() {
        let s = &train.scanner;
        println!(
            "  Train {}  offset x={:?}, y={:?}",
            i + 1,
            s.scan_head_offset_x,
            s.scan_head_offset_y,
        );
        if let Ok(cd) = reader.get_correction_data(i) {
            println!("           correction grid {:?}", cd.shape);
        }
    }
    println!();

    // -------------------------------------------------------------------------
    // 2. Apply new scanner offsets (post-calibration values)
    // -------------------------------------------------------------------------
    let new_offsets: [(f64, f64); 2] = [(-91.5, 24.0), (91.5, -24.0)];
    for (i, (x, y)) in new_offsets.iter().enumerate() {
        config.optical_trains[i].scanner.scan_head_offset_x = Some(*x);
        config.optical_trains[i].scanner.scan_head_offset_y = Some(*y);
    }

    // -------------------------------------------------------------------------
    // 3. Write updated config
    // -------------------------------------------------------------------------
    let tmp_file = tempfile::NamedTempFile::new()?;
    let out_path = tmp_file.path().with_extension("h5");
    drop(tmp_file);

    MachineConfigWriter::new(&config).write(&out_path)?;
    println!(
        "Written to : {}\n",
        out_path.file_name().unwrap_or_default().to_string_lossy()
    );

    // -------------------------------------------------------------------------
    // 4. Read back and verify
    // -------------------------------------------------------------------------
    let reader2 = MachineConfigReader::open(&out_path)?;
    let updated = reader2.parse()?;
    let mut failures: Vec<String> = Vec::new();

    for (i, (ex, ey)) in new_offsets.iter().enumerate() {
        let got_x = updated.optical_trains[i].scanner.scan_head_offset_x;
        let got_y = updated.optical_trains[i].scanner.scan_head_offset_y;
        if got_x != Some(*ex) {
            failures.push(format!(
                "  train{} offset_x: expected {:?}, got {:?}",
                i + 1, ex, got_x
            ));
        }
        if got_y != Some(*ey) {
            failures.push(format!(
                "  train{} offset_y: expected {:?}, got {:?}",
                i + 1, ey, got_y
            ));
        }
        match reader2.get_correction_data(i) {
            Ok(cd) if cd.shape != [257, 257, 2] => failures.push(format!(
                "  train{} correction shape: expected [257, 257, 2], got {:?}",
                i + 1, cd.shape
            )),
            Err(e) => failures.push(format!("  train{} correction read error: {e}", i + 1)),
            _ => {}
        }
    }

    let _ = std::fs::remove_file(&out_path);

    println!("After calibration:");
    for (i, train) in updated.optical_trains.iter().enumerate() {
        let s = &train.scanner;
        println!(
            "  Train {}  offset x={:?}, y={:?}",
            i + 1,
            s.scan_head_offset_x,
            s.scan_head_offset_y,
        );
    }
    println!();

    if failures.is_empty() {
        println!("PASS");
        Ok(())
    } else {
        println!("FAIL");
        for msg in &failures {
            println!("{msg}");
        }
        process::exit(1);
    }
}
