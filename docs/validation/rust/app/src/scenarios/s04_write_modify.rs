// S-04: Write modified config and verify field change survives round-trip
//
// ID:          S-04
// Action:      Read → change machine_name to "VALIDATION_TEST_MACHINE" →
//              write to temp file → read temp file → assert machine_name matches
// Expected:    machine_name persisted; file_version and build_plate_x unchanged.

use machine_config::{MachineConfigReader, MachineConfigWriter};
use std::path::Path;

pub fn run(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let path = fixtures_dir.join("reference_config.h5");
    let cfg = match MachineConfigReader::open(&path).and_then(|r| r.parse()) {
        Ok(c) => c,
        Err(e) => return (false, format!("read failed: {e}")),
    };
    let orig_x = cfg.machine.build_plate_x;

    let mut modified = cfg.clone();
    modified.meta.machine_name = "VALIDATION_TEST_MACHINE".to_string();
    modified.machine.machine_name = "VALIDATION_TEST_MACHINE".to_string();

    let tmp = match tempfile::Builder::new().suffix(".h5").tempfile() {
        Ok(t) => t,
        Err(e) => return (false, format!("tempfile creation failed: {e}")),
    };
    if let Err(e) = MachineConfigWriter::new(&modified).write(tmp.path()) {
        return (false, format!("write failed: {e}"));
    }

    let rb = match MachineConfigReader::open(tmp.path()).and_then(|r| r.parse()) {
        Ok(c) => c,
        Err(e) => return (false, format!("readback failed: {e}")),
    };

    if rb.meta.machine_name != "VALIDATION_TEST_MACHINE" {
        return (false, format!("machine_name not persisted: '{}'", rb.meta.machine_name));
    }
    if rb.meta.file_version.trim() != "1.0" {
        return (false, format!("file_version changed: '{}'", rb.meta.file_version));
    }
    if rb.machine.build_plate_x != orig_x {
        return (
            false,
            format!("build_plate_x changed: {orig_x:?} -> {:?}", rb.machine.build_plate_x),
        );
    }

    (true, "machine_name persisted, file_version and other fields unchanged".to_string())
}
