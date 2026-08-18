// S-03: Read real AconityMIDI fixture file
//
// ID:          S-03
// Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
// Action:      Parse the file, print all fields to stdout
// Expected:    No error. All fields present in output match known machine parameters.
// Rationale:   This is the primary real-world validation.

use machine_config::MachineConfigReader;
use std::path::Path;

pub fn run(_fixtures_dir: &Path, real_dir: &Path) -> (bool, String) {
    let entries = match std::fs::read_dir(real_dir) {
        Ok(e) => e,
        Err(e) => return (false, format!("read_dir({}) failed: {e}", real_dir.display())),
    };

    let matched = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.ends_with(".h5") && name.contains("AconityMIDI") && name.contains("OG_178")
        });

    let path = match matched {
        Some(p) => p,
        None => return (false, format!("real AconityMIDI file not found in {}", real_dir.display())),
    };

    let cfg = match MachineConfigReader::open(&path).and_then(|r| r.parse()) {
        Ok(c) => c,
        Err(e) => return (false, format!("read failed: {e}")),
    };

    let hash_prefix: String = cfg.meta.configuration_hash.chars().take(16).collect();
    let detail = format!(
        "machine_name={:?} | file_version={:?} | trains={} | build_plate_x={:?} | build_plate_y={:?} | wd={:?} | rotation[0]={:?} | hash={hash_prefix}...",
        cfg.meta.machine_name,
        cfg.meta.file_version,
        cfg.optical_trains.len(),
        cfg.machine.build_plate_x,
        cfg.machine.build_plate_y,
        cfg.optical_trains[0].scanner.working_distance,
        cfg.optical_trains[0].scanner.scan_head_rotation,
    );
    (true, detail)
}
