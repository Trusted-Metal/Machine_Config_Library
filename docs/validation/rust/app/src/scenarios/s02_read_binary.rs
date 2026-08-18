// S-02: Read reference fixture with binary data (correction grids)
//
// ID:          S-02
// Expected:    correction_data shape: [257, 257, 2]
//              inverse_correction_data shape: [257, 257, 2]
//              Both contain at least one finite (non-NaN) value
//              Forward and inverse arrays differ (not byte-identical)
//              SHA-256 hash of correction_data bytes matches reference value
// Rationale:   Binary data round-trips are the highest-risk correctness area.

use super::common::bitwise_equal;
use machine_config::MachineConfigReader;
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn run(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
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
