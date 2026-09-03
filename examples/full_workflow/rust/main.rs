// Machine Config Library — Rust Full Workflow: Calibration Adjustment
//
// Browseable copy next to the other language examples. Run it with:
//
//   cargo run --example full_workflow --manifest-path rust/Cargo.toml
//
// (Cargo compiles rust/examples/full_workflow.rs — keep that file in sync.)
//
// Load examples/dummy_2train.h5, apply new scanner offsets via set_scanner(Merge).

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

    println!("=== Full Workflow: Calibration Adjustment ===\n");
    println!("Machine : {}", file.get_meta().map_err(|e| e.to_string())?.machine_name);
    let n = file.optical_train_count().map_err(|e| e.to_string())?;
    println!("Trains  : {n}");
    println!();
    println!("Before calibration:");
    let reader = MachineConfigReader::open(&dummy)?;
    for i in 0..n {
        let s = file.get_scanner(i).map_err(|e| e.to_string())?;
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

    let new_offsets: [(f64, f64); 2] = [(-91.5, 24.0), (91.5, -24.0)];
    for (i, (x, y)) in new_offsets.iter().enumerate() {
        let mut scanner = file.get_scanner(i).map_err(|e| e.to_string())?;
        scanner.scan_head_offset_x = Some(*x);
        scanner.scan_head_offset_y = Some(*y);
        file.set_scanner(i, scanner, SetMode::Merge)
            .map_err(|e| e.to_string())?;
    }

    let tmp_file = tempfile::NamedTempFile::new()?;
    let out_path = tmp_file.path().with_extension("h5");
    drop(tmp_file);
    file.save(Some(&out_path)).map_err(|e| e.to_string())?;
    println!(
        "Written to : {}\n",
        out_path.file_name().unwrap_or_default().to_string_lossy()
    );

    let again = open_machine_config(&out_path).map_err(|e| e.to_string())?;
    let reader2 = MachineConfigReader::open(&out_path)?;
    let mut failures: Vec<String> = Vec::new();

    for (i, (ex, ey)) in new_offsets.iter().enumerate() {
        let s = again.get_scanner(i).map_err(|e| e.to_string())?;
        if s.scan_head_offset_x != Some(*ex) {
            failures.push(format!(
                "  train{} offset_x: expected {:?}, got {:?}",
                i + 1,
                ex,
                s.scan_head_offset_x
            ));
        }
        if s.scan_head_offset_y != Some(*ey) {
            failures.push(format!(
                "  train{} offset_y: expected {:?}, got {:?}",
                i + 1,
                ey,
                s.scan_head_offset_y
            ));
        }
        match reader2.get_correction_data(i) {
            Ok(cd) if cd.shape != [257, 257, 2] => failures.push(format!(
                "  train{} correction shape: expected [257, 257, 2], got {:?}",
                i + 1,
                cd.shape
            )),
            Err(e) => failures.push(format!("  train{} correction read error: {e}", i + 1)),
            _ => {}
        }
    }

    println!("After calibration:");
    let n2 = again.optical_train_count().map_err(|e| e.to_string())?;
    for i in 0..n2 {
        let s = again.get_scanner(i).map_err(|e| e.to_string())?;
        println!(
            "  Train {}  offset x={:?}, y={:?}",
            i + 1,
            s.scan_head_offset_x,
            s.scan_head_offset_y,
        );
    }
    println!();

    let _ = std::fs::remove_file(&out_path);

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
