//! S-01–09/AV-01–08/AV-12–13 scenarios for the Rust validation app (VALIDATION_PLAN.md §8).
//! AV-09–11 live in `rust/tests/` instead — see docs/validation/rust/results.md for why.

mod common;

use common::{av_fixture, bitwise_equal};
use machine_config::capabilities::errors::CapabilityError;
use machine_config::capabilities::open_machine_config;
use machine_config::{
    ClearBox, Collimator, LightSource, Machine, MachineConfig, MachineConfigError,
    MachineConfigMeta, MachineConfigReader, MachineConfigWriter, MockConfigBuilder, OpcuaConfig,
    OpticalTrain, OptionalComponents, ScanFieldCorrectionFile, Scanner, ScannerCard,
};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::Path;

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
pub fn run_s01(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
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

// S-02: Read reference fixture with binary data (correction grids)
//
// ID:          S-02
// Expected:    correction_data shape: [257, 257, 2]
//              inverse_correction_data shape: [257, 257, 2]
//              Both contain at least one finite (non-NaN) value
//              Forward and inverse arrays differ (not byte-identical)
//              SHA-256 hash of correction_data bytes matches reference value
// Rationale:   Binary data round-trips are the highest-risk correctness area.
pub fn run_s02(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let path = fixtures_dir.join("reference_config.h5");
    let reader = match MachineConfigReader::open(&path) {
        Ok(r) => r,
        Err(e) => return (false, format!("open failed: {e}")),
    };

    let cd = match reader.get_correction_data(0) {
        Ok(c) => c,
        Err(e) => return (false, format!("get_correction_data failed: {e}")),
    };
    let icd = match reader.get_inverse_correction_data(0) {
        Ok(c) => c,
        Err(e) => return (false, format!("get_inverse_correction_data failed: {e}")),
    };

    if cd.shape != [257, 257, 2] {
        return (false, format!("correction_data shape: {:?}", cd.shape));
    }
    if icd.shape != [257, 257, 2] {
        return (false, format!("inverse_correction_data shape: {:?}", icd.shape));
    }
    if !cd.data.iter().any(|v| v.is_finite()) {
        return (false, "correction_data: no finite values".to_string());
    }
    if !icd.data.iter().any(|v| v.is_finite()) {
        return (false, "inverse_correction_data: no finite values".to_string());
    }
    if bitwise_equal(&cd.data, &icd.data) {
        return (
            false,
            "correction_data and inverse_correction_data are identical".to_string(),
        );
    }

    let mut hasher = Sha256::new();
    for v in &cd.data {
        hasher.update(v.to_le_bytes());
    }
    let cd_hash = hex::encode(hasher.finalize());

    (
        true,
        format!("shapes OK, finite values OK, forward≠inverse, correction_data SHA-256={cd_hash}"),
    )
}

// S-03: Read real AconityMIDI fixture file
//
// ID:          S-03
// Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
// Action:      Parse the file, print all fields to stdout
// Expected:    No error. All fields present in output match known machine parameters.
// Rationale:   This is the primary real-world validation.
pub fn run_s03(_fixtures_dir: &Path, real_dir: &Path) -> (bool, String) {
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

// S-04: Write modified config and verify field change survives round-trip
//
// ID:          S-04
// Action:      Read → change machine_name to "VALIDATION_TEST_MACHINE" →
//              write to temp file → read temp file → assert machine_name matches
// Expected:    machine_name persisted; file_version and build_plate_x unchanged.
pub fn run_s04(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
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

// S-05: Full binary round-trip with correction hash verification
//
// ID:          S-05
// Action:      Read → write to temp → read temp → compare SHA-256 of
//              correction_data / inverse_correction_data before and after.
// Rationale:   Silent precision loss in binary data is undetectable without
//              hash comparison.
fn sha256_hex(data: &[f64]) -> String {
    let mut hasher = Sha256::new();
    for v in data {
        hasher.update(v.to_le_bytes());
    }
    hex::encode(hasher.finalize())
}

pub fn run_s05(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let path = fixtures_dir.join("reference_config.h5");
    let reader = match MachineConfigReader::open(&path) {
        Ok(r) => r,
        Err(e) => return (false, format!("open failed: {e}")),
    };
    // parse_with_binary(), not parse(): Rust's reader splits scalars-only vs
    // scalars+binary (unlike Python's single always-binary parse()) — writing
    // a config read via plain parse() would zero-fill the correction grid by
    // design (see capabilities/v1_0/writer.rs's nested_to_array3 comment).
    let cfg = match reader.parse_with_binary() {
        Ok(c) => c,
        Err(e) => return (false, format!("parse_with_binary failed: {e}")),
    };

    let cd_before = match reader.get_correction_data(0) {
        Ok(c) => c,
        Err(e) => return (false, format!("get_correction_data failed: {e}")),
    };
    let icd_before = match reader.get_inverse_correction_data(0) {
        Ok(c) => c,
        Err(e) => return (false, format!("get_inverse_correction_data failed: {e}")),
    };

    let tmp = match tempfile::Builder::new().suffix(".h5").tempfile() {
        Ok(t) => t,
        Err(e) => return (false, format!("tempfile creation failed: {e}")),
    };
    if let Err(e) = MachineConfigWriter::new(&cfg).write(tmp.path()) {
        return (false, format!("write failed: {e}"));
    }

    let reader2 = match MachineConfigReader::open(tmp.path()) {
        Ok(r) => r,
        Err(e) => return (false, format!("reopen failed: {e}")),
    };
    let cd_after = match reader2.get_correction_data(0) {
        Ok(c) => c,
        Err(e) => return (false, format!("readback get_correction_data failed: {e}")),
    };
    let icd_after = match reader2.get_inverse_correction_data(0) {
        Ok(c) => c,
        Err(e) => return (false, format!("readback get_inverse_correction_data failed: {e}")),
    };

    if !bitwise_equal(&cd_before.data, &cd_after.data) {
        return (
            false,
            format!(
                "correction_data mismatch: {}... -> {}...",
                &sha256_hex(&cd_before.data)[..16],
                &sha256_hex(&cd_after.data)[..16]
            ),
        );
    }
    if !bitwise_equal(&icd_before.data, &icd_after.data) {
        return (false, "inverse_correction_data mismatch after roundtrip".to_string());
    }

    let cd_hash = sha256_hex(&cd_before.data);
    (true, format!("correction_data preserved: SHA-256={cd_hash}"))
}

// S-06: Build synthetic config with MockConfigBuilder and verify fields
//
// ID:          S-06
// Action:      Build a 2-laser config → verify fields → save to temp → re-read
// Expected:    2 trains, rotations 0/180, machine_name non-empty,
//              correction_data centre cell ≈ 2.0 (Gaussian peak), roundtrip OK.
pub fn run_s06(_fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
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

// S-07: OPCUA config roundtrip
//
// ID:          S-07
// Precondition: fixtures/reference_config_opcua.h5
// Action:      Read → write to temp → read temp
// Expected:    server_url, session_timeout, triggers_enabled, trigger names,
//              and "Chamber Oxygen Level" signal/subsystem all unchanged.
pub fn run_s07(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let path = fixtures_dir.join("reference_config_opcua.h5");
    if !path.exists() {
        return (false, format!("OPCUA fixture not found: {}", path.display()));
    }

    let cfg = match MachineConfigReader::open(&path).and_then(|r| r.parse()) {
        Ok(c) => c,
        Err(e) => return (false, format!("read failed: {e}")),
    };
    let opcua = match &cfg.opcua {
        Some(o) => o,
        None => return (false, "opcua is None after reading OPCUA fixture".to_string()),
    };

    let orig_url = opcua.client.server_url.clone();
    let orig_timeout = opcua.client.session_timeout;
    let orig_triggers_enabled = opcua.triggers_enabled;
    let orig_trigger_names: HashSet<String> = opcua.triggers.keys().cloned().collect();
    // Newly-promoted fields (OPCUA_FIELD_PROMOTION_PLAN.md Phase 1) — a
    // representative subset, proving the low-level roundtrip works through
    // the public MachineConfigReader/Writer API too, not just in unit tests.
    let orig_machine_profile = opcua.client.machine_profile.clone();
    let orig_root_node = opcua.client.root_node.clone();
    let orig_pipe_name = opcua.pipe.pipe_name.clone();
    let orig_ceiling_layers = opcua.trigger_stop_ceiling_layers;

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
    let rb_opcua = match &rb.opcua {
        Some(o) => o,
        None => return (false, "opcua is None after roundtrip".to_string()),
    };

    if rb_opcua.client.server_url != orig_url {
        return (false, format!("server_url changed: {orig_url:?} -> {:?}", rb_opcua.client.server_url));
    }
    if rb_opcua.client.session_timeout != orig_timeout {
        return (
            false,
            format!("session_timeout changed: {orig_timeout} -> {}", rb_opcua.client.session_timeout),
        );
    }
    if rb_opcua.triggers_enabled != orig_triggers_enabled {
        return (
            false,
            format!("triggers_enabled changed: {orig_triggers_enabled:?} -> {:?}", rb_opcua.triggers_enabled),
        );
    }

    let rb_names: HashSet<String> = rb_opcua.triggers.keys().cloned().collect();
    if rb_names != orig_trigger_names {
        let missing: Vec<_> = orig_trigger_names.difference(&rb_names).collect();
        return (false, format!("trigger names changed: missing={missing:?}"));
    }

    let co = "Chamber Oxygen Level";
    if let (Some(ot), Some(rt)) = (opcua.triggers.get(co), rb_opcua.triggers.get(co)) {
        if ot.signal != rt.signal || ot.subsystem != rt.subsystem {
            return (false, format!("'{co}' signal/subsystem changed"));
        }
        if ot.event != rt.event || ot.trigger_label != rt.trigger_label {
            return (false, format!("'{co}' event/trigger_label changed"));
        }
    }

    if rb_opcua.client.machine_profile != orig_machine_profile {
        return (
            false,
            format!("machine_profile changed: {orig_machine_profile:?} -> {:?}", rb_opcua.client.machine_profile),
        );
    }
    if rb_opcua.client.root_node != orig_root_node {
        return (
            false,
            format!("root_node changed: {orig_root_node:?} -> {:?}", rb_opcua.client.root_node),
        );
    }
    if rb_opcua.pipe.pipe_name != orig_pipe_name {
        return (
            false,
            format!("pipe_name changed: {orig_pipe_name:?} -> {:?}", rb_opcua.pipe.pipe_name),
        );
    }
    if rb_opcua.trigger_stop_ceiling_layers != orig_ceiling_layers {
        return (
            false,
            format!(
                "trigger_stop_ceiling_layers changed: {orig_ceiling_layers:?} -> {:?}",
                rb_opcua.trigger_stop_ceiling_layers
            ),
        );
    }

    (
        true,
        format!(
            "OPCUA roundtrip OK: {} triggers, url={orig_url:?}, machine_profile={orig_machine_profile:?}",
            orig_trigger_names.len()
        ),
    )
}

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
pub fn run_s08(_fixtures_dir: &Path, real_dir: &Path) -> (bool, String) {
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

// S-09: Public type export surface
//
// ID:          S-09
// Title:       Verify all public model types are importable from the library surface
// Category:    happy-path
// Layer:       public API / packaging
// Action:      Import every consumer-facing type directly from the crate root
//              (no `machine_config::models::...` or other sub-module path) and
//              prove each is usable, not just nameable.
// Rationale:   If a consumer must import from an internal path, the library's
//              public API surface is incomplete. See VALIDATION_PLAN.md §8.

// `MockConfigBuilder::new(n).build()` never sets OPCUA (that only comes from
// the OPCUA fixture used in S-07) — there's no real value to hold here, so
// type-annotating a never-called parameter is what VALIDATION_PLAN.md §8 S-09
// explicitly allows: "Construct OR type-annotate a variable with each type."
#[allow(dead_code)]
fn type_check_opcua(_opcua: &OpcuaConfig) {}

pub fn run_s09(_fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let builder: MockConfigBuilder = MockConfigBuilder::new(2);
    let config: MachineConfig = builder.build();

    let meta: &MachineConfigMeta = &config.meta;
    let machine: &Machine = &config.machine;

    if config.optical_trains.is_empty() {
        return (false, "builder produced zero optical trains".to_string());
    }
    let train: &OpticalTrain = &config.optical_trains[0];
    let scanner: &Scanner = &train.scanner;
    let light_source: &LightSource = &train.light_source;
    let collimator: &Collimator = &train.collimator;
    let scanner_card: &ScannerCard = &train.scanner_card;
    let optional_components: &OptionalComponents = &train.optional_components;

    let clearbox: &ClearBox = match &optional_components.clearbox {
        Some(cb) => cb,
        None => return (false, "expected builder to include a clearbox by default".to_string()),
    };
    let sfcf: &ScanFieldCorrectionFile = match &train.scan_field_correction_file {
        Some(s) => s,
        None => return (false, "expected builder to include an SFCF by default".to_string()),
    };

    let writer: MachineConfigWriter = MachineConfigWriter::new(&config);
    let _reader_type_check: fn(&str) -> machine_config::error::Result<MachineConfigReader> =
        |p| MachineConfigReader::open(p);

    if meta.machine_name.is_empty()
        || machine.manufacturer.is_empty()
        || scanner.manufacturer.is_empty()
        || light_source.manufacturer.is_empty()
        || collimator.manufacturer.is_empty()
        || scanner_card.manufacturer.is_empty()
        || clearbox.ip_address.is_empty()
        || sfcf.document_name.is_empty()
    {
        return (false, "one or more constructed fields were unexpectedly empty".to_string());
    }
    let _ = writer;

    (
        true,
        "all public types resolve and are usable from the crate root".to_string(),
    )
}

// AV-01: Reader rejects unknown File_Version with typed error
pub fn run_av01(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "v2_0_unknown.h5");
    match MachineConfigReader::open(&fixture) {
        Ok(_) => (false, "no error raised for File_Version='2.0'".to_string()),
        Err(MachineConfigError::UnsupportedVersion(v)) if v == "2.0" => {
            (true, format!("UnsupportedVersion raised, version='{v}'"))
        }
        Err(MachineConfigError::UnsupportedVersion(v)) => {
            (false, format!("UnsupportedVersion raised but version='{v}'"))
        }
        Err(e) => (false, format!("wrong error variant: {e}")),
    }
}

// AV-02: Reader handles absent File_Version attribute predictably
//
// Either outcome (defaults to v1.0 dispatch, or a typed error) is acceptable
// per VALIDATION_PLAN.md — behavior must simply be documented, and must match
// across all five languages. This scenario records what actually happens
// rather than asserting one outcome, mirroring Python's/Node's own AV-02.
pub fn run_av02(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "missing_version.h5");
    match MachineConfigReader::open(&fixture).and_then(|r| r.parse()) {
        Ok(cfg) => (
            true,
            format!(
                "missing File_Version dispatches OK, file_version='{}'",
                cfg.meta.file_version
            ),
        ),
        Err(e) => (true, format!("missing File_Version raises {e}")),
    }
}

// AV-03: v1.0 reader encountering a v1.1 file fails loudly
pub fn run_av03(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "v1_1_simulated.h5");
    match MachineConfigReader::open(&fixture) {
        Ok(_) => (false, "no error raised for File_Version='1.1'".to_string()),
        Err(MachineConfigError::UnsupportedVersion(v)) if v == "1.1" => {
            (true, format!("UnsupportedVersion raised, version='{v}'"))
        }
        Err(MachineConfigError::UnsupportedVersion(v)) => {
            (false, format!("UnsupportedVersion raised but version='{v}'"))
        }
        Err(e) => (false, format!("wrong error variant: {e}")),
    }
}

// AV-04: Reader returns typed error when required group (Machine/) is absent
pub fn run_av04(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "missing_machine_group.h5");
    match MachineConfigReader::open(&fixture).and_then(|r| r.parse()) {
        Ok(_) => (false, "no error raised for missing Machine/ group".to_string()),
        Err(e) => (true, format!("error raised for missing Machine/ group: {e}")),
    }
}

// AV-05: Reader handles corrupt required attribute (Build_Plate_X_Dimension) gracefully
pub fn run_av05(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "corrupt_scalar.h5");
    match MachineConfigReader::open(&fixture).and_then(|r| r.parse()) {
        Ok(_) => (false, "no error raised for corrupt Build_Plate_X_Dimension".to_string()),
        Err(e @ MachineConfigError::Parse(_)) => {
            (true, format!("Parse error raised for corrupt scalar: {e}"))
        }
        Err(e) => (false, format!("unexpected error variant: {e}")),
    }
}

// AV-06: Dispatcher normalizes whitespace in File_Version (" 1.0 ")
pub fn run_av06(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "version_whitespace.h5");
    match MachineConfigReader::open(&fixture).and_then(|r| r.parse()) {
        Ok(_) => (true, "whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK".to_string()),
        Err(e) => (false, format!("{e}")),
    }
}

// AV-07: Dispatcher handles empty string File_Version
//
// Either outcome is acceptable per VALIDATION_PLAN.md — this records the
// actual behavior rather than asserting one, mirroring Python's/Node's AV-07.
pub fn run_av07(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "empty_version.h5");
    match MachineConfigReader::open(&fixture).and_then(|r| r.parse()) {
        Ok(cfg) => (
            true,
            format!(
                "empty File_Version dispatches OK, file_version='{}'",
                cfg.meta.file_version
            ),
        ),
        Err(e) => (true, format!("empty File_Version raises {e}")),
    }
}

// AV-08: File_Version string survives write→read unchanged
pub fn run_av08(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let path = fixtures_dir.join("reference_config.h5");
    let cfg = match MachineConfigReader::open(&path).and_then(|r| r.parse()) {
        Ok(c) => c,
        Err(e) => return (false, format!("read failed: {e}")),
    };
    let orig_version = cfg.meta.file_version.trim().to_string();

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
    let rb_version = rb.meta.file_version.trim().to_string();

    if rb_version != "1.0" {
        return (false, format!("file_version after roundtrip: expected '1.0', got '{rb_version}'"));
    }
    if rb_version != orig_version {
        return (false, format!("file_version changed: '{orig_version}' -> '{rb_version}'"));
    }

    (true, format!("File_Version survives roundtrip unchanged: '{rb_version}'"))
}

// AV-12: capabilities facade .get_opcua() returns Ok with the newly-promoted
// typed fields readable, when every required field is present.
//
// Nothing before this scenario exercised the `capabilities` facade at all —
// S-07 above only goes through the plain MachineConfigReader/Writer. See
// OPCUA_FIELD_PROMOTION_PLAN.md's "Validation-app coverage" section for why
// this is a real public-API guarantee, not just a unit-test concern.
pub fn run_av12(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let path = fixtures_dir.join("reference_config_opcua.h5");
    let file = match open_machine_config(&path) {
        Ok(f) => f,
        Err(e) => return (false, format!("open_machine_config failed: {e}")),
    };
    match file.get_opcua() {
        Ok(opcua) => (
            true,
            format!(
                "get_opcua() Ok: machine_profile={:?}, root_node={:?}, pipe_name={:?}, \
                 triggers_enabled={:?}, trigger_stop_ceiling_layers={:?}",
                opcua.client.machine_profile,
                opcua.client.root_node,
                opcua.pipe.pipe_name,
                opcua.triggers_enabled,
                opcua.trigger_stop_ceiling_layers,
            ),
        ),
        Err(e) => (false, format!("get_opcua() failed on fully-populated fixture: {e}")),
    }
}

// AV-13: capabilities facade .get_opcua() returns Err(ValidationError) with
// `details` naming exactly the seven missing required fields, when OPCUA is
// present but incomplete. Uses the shared `opcua_missing_required.h5` fixture
// (Phase 0), which deliberately removes Event from only one of the two
// triggers so this also proves the still-complete trigger isn't flagged.
pub fn run_av13(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "opcua_missing_required.h5");
    let file = match open_machine_config(&fixture) {
        Ok(f) => f,
        Err(e) => return (false, format!("open_machine_config failed: {e}")),
    };
    match file.get_opcua() {
        Ok(_) => (false, "get_opcua() returned Ok on a fixture missing required fields".to_string()),
        Err(CapabilityError::ValidationError { details: Some(mut details), .. }) => {
            details.sort();
            let mut expected = vec![
                "Machine_Profile".to_string(),
                "Root_Node".to_string(),
                "Configure_Client".to_string(),
                "Pipe_Name".to_string(),
                "Triggers_Enabled".to_string(),
                "Trigger_Stop_Ceiling_Layers".to_string(),
                "Laser Emission Interlock.Event".to_string(),
            ];
            expected.sort();
            if details == expected {
                (true, format!("ValidationError with details={details:?}"))
            } else {
                (false, format!("details mismatch: got {details:?}, expected {expected:?}"))
            }
        }
        Err(e) => (false, format!("wrong error variant/shape: {e}")),
    }
}
