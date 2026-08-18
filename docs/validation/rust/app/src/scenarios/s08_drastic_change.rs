// S-08: Drastic field change to real file, verify adapter pipeline integrity
//
// ID:          S-08
// Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
// Action:      1. Add a third optical train (clone train 1, change train_id)
//              2. Change build_plate_x from 250.0 to 350.0
//              3. Set scan_head_rotation on new train to 90.0
//              4. Clear all correction data on the new train (clearbox = None)
//              5. Change machine_name to "MODIFIED_ACONITY_VALIDATION"
// Expected:    All five changes persist after write → read; file_version
//              unchanged at "1.0".

use machine_config::{MachineConfigReader, MachineConfigWriter};
use std::path::Path;

pub fn run(_fixtures_dir: &Path, real_dir: &Path) -> (bool, String) {
    let entries = match std::fs::read_dir(real_dir) {
        Ok(e) => e,
        Err(e) => return (false, format!("read_dir({}) failed: {e}", real_dir.display())),
    };
    let matched = entries.filter_map(|e| e.ok()).map(|e| e.path()).find(|p| {
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

    let mut modified = cfg.clone();
    modified.meta.machine_name = "MODIFIED_ACONITY_VALIDATION".to_string();
    modified.machine.machine_name = "MODIFIED_ACONITY_VALIDATION".to_string();
    modified.machine.build_plate_x = Some(350.0);

    let mut new_train = cfg.optical_trains[1].clone();
    new_train.train_id = "Optical_Train_03".to_string();
    new_train.scanner.scan_head_rotation = Some(90.0);
    new_train.optional_components.clearbox = None;
    modified.optical_trains.push(new_train);

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

    if rb.optical_trains.len() != 3 {
        return (false, format!("optical_trains: expected 3, got {}", rb.optical_trains.len()));
    }
    match rb.machine.build_plate_x {
        Some(x) if (x - 350.0).abs() <= 0.001 => {}
        other => return (false, format!("build_plate_x: got {other:?}")),
    }
    match rb.optical_trains[2].scanner.scan_head_rotation {
        Some(r) if (r - 90.0).abs() <= 0.001 => {}
        other => return (false, format!("train[2].scan_head_rotation: got {other:?}")),
    }
    if rb.optical_trains[2].optional_components.clearbox.is_some() {
        return (false, "train[2].clearbox should be None".to_string());
    }
    if rb.meta.machine_name != "MODIFIED_ACONITY_VALIDATION" {
        return (false, format!("machine_name: got '{}'", rb.meta.machine_name));
    }
    if rb.meta.file_version.trim() != "1.0" {
        return (false, format!("file_version changed: '{}'", rb.meta.file_version));
    }

    (true, "3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK".to_string())
}
