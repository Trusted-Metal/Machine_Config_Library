// S-01: Read reference fixture, all scalar fields
//
// ID:          S-01
// Title:       Read reference fixture and verify all scalar fields
// Category:    happy-path
// Layer:       reader
// Precondition: fixtures/reference_config.h5
// Action:      Parse the file, print key fields to stdout
// Expected:    machine_name = "TM-LPBF-02: AconityMIDI+_OG"
//              build_plate_x ≈ 250.0
//              build_plate_y ≈ 250.0
//              len(optical_trains) = 2
//              train[0].scanner.working_distance ≈ 670.0
//              train[0].scanner.scan_head_rotation ≈ 0.0
//              train[1].scanner.scan_head_rotation ≈ 180.0
//              configuration_hash: 64 hex characters
//              file_version: "1.0"
// Rationale:   Establishes baseline read correctness against a known-good fixture.
//
// Note on flat model: unlike Python's nested `machine.build_plate.x`, Rust's
// `Machine` inlines build-plate fields directly (`build_plate_x`, etc.) — see
// the doc comment on `Machine` in rust/src/models.rs.

use machine_config::MachineConfigReader;
use std::path::Path;

pub fn run(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let path = fixtures_dir.join("reference_config.h5");

    let reader = match MachineConfigReader::open(&path) {
        Ok(r) => r,
        Err(e) => return (false, format!("open failed: {e}")),
    };
    let cfg = match reader.parse() {
        Ok(c) => c,
        Err(e) => return (false, format!("parse failed: {e}")),
    };

    if cfg.meta.machine_name != "TM-LPBF-02: AconityMIDI+_OG" {
        return (false, format!("machine_name: got '{}'", cfg.meta.machine_name));
    }

    match cfg.machine.build_plate_x {
        Some(x) if (x - 250.0).abs() <= 0.001 => {}
        other => return (false, format!("build_plate_x: got {other:?}")),
    }
    match cfg.machine.build_plate_y {
        Some(y) if (y - 250.0).abs() <= 0.001 => {}
        other => return (false, format!("build_plate_y: got {other:?}")),
    }

    if cfg.optical_trains.len() != 2 {
        return (
            false,
            format!("optical_trains count: got {}", cfg.optical_trains.len()),
        );
    }

    match cfg.optical_trains[0].scanner.working_distance {
        Some(wd) if (wd - 670.0).abs() <= 0.1 => {}
        other => return (false, format!("train[0].working_distance: got {other:?}")),
    }
    match cfg.optical_trains[0].scanner.scan_head_rotation {
        Some(r) if r.abs() <= 0.001 => {}
        other => return (false, format!("train[0].scan_head_rotation: got {other:?}")),
    }
    match cfg.optical_trains[1].scanner.scan_head_rotation {
        Some(r) if (r - 180.0).abs() <= 0.001 => {}
        other => return (false, format!("train[1].scan_head_rotation: got {other:?}")),
    }

    let h = &cfg.meta.configuration_hash;
    if h.len() != 64 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return (false, format!("configuration_hash invalid: '{h}'"));
    }
    if cfg.meta.file_version.trim() != "1.0" {
        return (false, format!("file_version: got '{}'", cfg.meta.file_version));
    }

    (true, "all scalar fields match expected values".to_string())
}
