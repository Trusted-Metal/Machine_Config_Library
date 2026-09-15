//! SHA-256 configuration-integrity hash for [`MachineConfig`].
//!
//! Computed over the canonical, `include_binary = false` JSON shape of the
//! whole config (the same shape `to_json()`/`export-json` produce), minus
//! `meta.configuration_hash` (can't hash itself) and `meta.export_date`
//! (changes on every re-export even when nothing configuration-wise
//! changed). Object keys are recursively sorted (free in Rust:
//! `serde_json::Value`'s map is `BTreeMap`-backed here — `preserve_order` is
//! not enabled anywhere in this workspace) so the result doesn't depend on
//! this language's particular field-declaration order.
//!
//! This hash is computed for MCF's own self-consistency across MCF's own
//! five language implementations — it is NOT designed to match any other
//! producer's own hashing scheme (e.g. an external system that may hash data
//! which never survives to the on-disk file at all). A file MCF's own
//! writer produced will read back valid; a file authored by anything else
//! will very likely read back invalid — that's expected, not a bug: it
//! answers "was this MCF-touched file tampered with since MCF itself last
//! touched it," not "does this match some other system's proprietary
//! algorithm."
//!
//! Binary correction-grid data (`correction_data`/`inverse_correction_data`/
//! `raw_bytes`) is always excluded regardless of whether the `MachineConfig`
//! passed in happens to carry it (e.g. a config obtained via
//! `parse_with_binary()`) — a separate, dedicated mechanism
//! (`correction-hash`) already exists for binary-grid integrity, and
//! duplicating that here would be slow and redundant.

use sha2::{Digest, Sha256};

use crate::models::MachineConfig;

/// Returns the SHA-256 hex digest of `config`'s canonical, hash-relevant
/// content. See the module docs for exactly what is included/excluded.
pub fn compute_configuration_hash(config: &MachineConfig) -> String {
    let mut config = config.clone();
    for train in &mut config.optical_trains {
        if let Some(cb) = train.optional_components.clearbox.as_mut() {
            cb.correction_data = None;
            cb.inverse_correction_data = None;
        }
        if let Some(sfcf) = train.scan_field_correction_file.as_mut() {
            sfcf.raw_bytes = None;
        }
    }

    let mut value =
        serde_json::to_value(&config).expect("MachineConfig always serializes to JSON");
    if let Some(meta) = value.get_mut("meta").and_then(|m| m.as_object_mut()) {
        meta.remove("configuration_hash");
        meta.remove("export_date");
    }

    // `serde_json::to_string` on a `Value` already yields sorted-keys,
    // whitespace-free output, and (verified empirically) already renders a
    // whole-valued float like 250.0 as "250.0" — not "250" — so no separate
    // float-rendering fixup is needed in this language, unlike Node/Go.
    let canonical = serde_json::to_string(&value).expect("Value always serializes to JSON");
    let digest = Sha256::digest(canonical.as_bytes());
    format!("{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::MachineConfigReader;

    const REFERENCE: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config.h5");

    #[test]
    fn same_content_same_hash() {
        let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let a = compute_configuration_hash(&config);
        let b = compute_configuration_hash(&config);
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn hash_ignores_its_own_stored_value() {
        let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let a = compute_configuration_hash(&config);
        config.meta.configuration_hash = "f".repeat(64);
        let b = compute_configuration_hash(&config);
        assert_eq!(a, b, "changing only the stored hash must not change the recomputed hash");
    }

    #[test]
    fn hash_ignores_export_date() {
        let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let a = compute_configuration_hash(&config);
        config.meta.export_date = "2099-01-01T00:00:00.000Z".into();
        let b = compute_configuration_hash(&config);
        assert_eq!(a, b);
    }

    #[test]
    fn hash_changes_when_a_scalar_field_changes() {
        let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let a = compute_configuration_hash(&config);
        config.machine.build_plate_x = config.machine.build_plate_x.map(|v| v + 1.0);
        let b = compute_configuration_hash(&config);
        assert_ne!(a, b);
    }

    #[test]
    fn hash_ignores_binary_grid_data() {
        let with_binary = MachineConfigReader::open(REFERENCE).unwrap().parse_with_binary().unwrap();
        let without_binary = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        assert_eq!(
            compute_configuration_hash(&with_binary),
            compute_configuration_hash(&without_binary),
        );
    }
}
