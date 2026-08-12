// Machine Config Library — Rust Quickstart
//
// Run from the repo root:
//
//   cargo run --example quickstart --manifest-path rust/Cargo.toml
//
// Opens examples/dummy_2train.h5 via the stable model facade.

use std::path::PathBuf;
use std::process;

use machine_config::capabilities::{open_machine_config, SetMode};
use machine_config::reader::MachineConfigReader;

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
    let dummy = repo_root.join("examples").join("dummy_2train.h5");

    if !dummy.exists() {
        eprintln!("Dummy file not found: {}", dummy.display());
        eprintln!("Run: python examples/generate_dummy.py");
        process::exit(1);
    }

    let mut file = open_machine_config(&dummy).map_err(|e| e.to_string())?;

    println!("=== Machine Config Quickstart ===\n");
    println!("File version   : {}", file.file_version());
    let meta = file.get_meta().map_err(|e| e.to_string())?;
    println!("Machine name   : {}", meta.machine_name);
    let n = file.optical_train_count().map_err(|e| e.to_string())?;
    println!("Optical trains : {n}");

    for i in 0..n {
        let scanner = file.get_scanner(i).map_err(|e| e.to_string())?;
        println!(
            "  Train {i}  wd={:?} {}  offset x={:?}, y={:?}",
            scanner.working_distance,
            scanner.working_distance_unit.as_deref().unwrap_or(""),
            scanner.scan_head_offset_x,
            scanner.scan_head_offset_y,
        );
        match file.get_clearbox(i) {
            Ok(Some(_)) => println!("           clearbox: present"),
            Ok(None) => println!("           optionalComponents: none"),
            Err(e) => println!("           clearbox: {e}"),
        }
    }

    let scanner0 = file.get_scanner(0).map_err(|e| e.to_string())?;
    let correction = MachineConfigReader::open(&dummy)?.get_correction_data(0)?;
    println!("Correction grid: {:?}   (train 0)", correction.shape);

    println!();
    file.set_scanner(0, scanner0.clone(), SetMode::Merge)
        .map_err(|e| e.to_string())?;
    let tmp_file = tempfile::NamedTempFile::new()?;
    let tmp_path = tmp_file.path().with_extension("h5");
    drop(tmp_file);
    file.save(Some(&tmp_path)).map_err(|e| e.to_string())?;
    println!(
        "Written to     : {}",
        tmp_path.file_name().unwrap_or_default().to_string_lossy()
    );

    let again = open_machine_config(&tmp_path).map_err(|e| e.to_string())?;
    let mut failures: Vec<String> = Vec::new();
    let meta2 = again.get_meta().map_err(|e| e.to_string())?;
    if meta2.machine_name != meta.machine_name {
        failures.push("  machine_name mismatch after round-trip".into());
    }
    let n2 = again.optical_train_count().map_err(|e| e.to_string())?;
    if n2 != n {
        failures.push(format!("  train_count: expected {n}, got {n2}"));
    }
    let wd2 = again.get_scanner(0).map_err(|e| e.to_string())?;
    if wd2.working_distance != scanner0.working_distance {
        failures.push(format!(
            "  working_distance: expected {:?}, got {:?}",
            scanner0.working_distance, wd2.working_distance
        ));
    }

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
