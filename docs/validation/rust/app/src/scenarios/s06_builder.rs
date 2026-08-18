// S-06: Build synthetic config with MockConfigBuilder and verify fields
//
// ID:          S-06
// Action:      Build a 2-laser config → verify fields → save to temp → re-read
// Expected:    2 trains, rotations 0/180, machine_name non-empty,
//              correction_data centre cell ≈ 2.0 (Gaussian peak), roundtrip OK.

use machine_config::{MachineConfigReader, MachineConfigWriter, MockConfigBuilder};
use std::path::Path;

pub fn run(_fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let cfg = MockConfigBuilder::new(2).build();

    if cfg.optical_trains.len() != 2 {
        return (false, format!("optical_trains count: {}", cfg.optical_trains.len()));
    }

    let r0 = cfg.optical_trains[0].scanner.scan_head_rotation;
    let r1 = cfg.optical_trains[1].scanner.scan_head_rotation;
    match r0 {
        Some(r) if r.abs() <= 0.001 => {}
        other => return (false, format!("train[0].scan_head_rotation: {other:?}")),
    }
    match r1 {
        Some(r) if (r - 180.0).abs() <= 0.001 => {}
        other => return (false, format!("train[1].scan_head_rotation: {other:?}")),
    }
    if cfg.meta.machine_name.is_empty() {
        return (false, "machine_name is empty".to_string());
    }

    let clearbox = match &cfg.optical_trains[0].optional_components.clearbox {
        Some(cb) => cb,
        None => return (false, "clearbox is None".to_string()),
    };
    let grid = match &clearbox.correction_data {
        Some(g) => g,
        None => return (false, "correction_data is None".to_string()),
    };
    let center = match grid[128][128][0] {
        Some(v) => v,
        None => return (false, "correction_data centre cell is None".to_string()),
    };
    if !center.is_finite() || (center - 2.0).abs() > 0.01 {
        return (false, format!("correction_data center: expected ~2.0, got {center}"));
    }

    let tmp = match tempfile::Builder::new().suffix(".h5").tempfile() {
        Ok(t) => t,
        Err(e) => return (false, format!("tempfile creation failed: {e}")),
    };
    if let Err(e) = MachineConfigWriter::new(&cfg).write(tmp.path()) {
        return (false, format!("write failed: {e}"));
    }
    let rb = match MachineConfigReader::open(tmp.path()).and_then(|r| r.parse()) {
        Ok(c) => c,
        Err(e) => return (false, format!("readback failed: {e}")),
    };

    if rb.optical_trains.len() != 2 {
        return (false, format!("readback trains: {}", rb.optical_trains.len()));
    }
    if rb.meta.machine_name != cfg.meta.machine_name {
        return (false, "machine_name changed after roundtrip".to_string());
    }

    (true, "2-laser build OK, center≈2.0, roundtrip OK".to_string())
}
