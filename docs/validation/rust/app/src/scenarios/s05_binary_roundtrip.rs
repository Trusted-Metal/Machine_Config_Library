// S-05: Full binary round-trip with correction hash verification
//
// ID:          S-05
// Action:      Read → write to temp → read temp → compare SHA-256 of
//              correction_data / inverse_correction_data before and after.
// Rationale:   Silent precision loss in binary data is undetectable without
//              hash comparison.

use super::common::bitwise_equal;
use machine_config::{MachineConfigReader, MachineConfigWriter};
use sha2::{Digest, Sha256};
use std::path::Path;

fn sha256_hex(data: &[f64]) -> String {
    let mut hasher = Sha256::new();
    for v in data {
        hasher.update(v.to_le_bytes());
    }
    hex::encode(hasher.finalize())
}

pub fn run(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
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
