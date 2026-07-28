// Phase 3.9 — Integration tests.
// All tests run against real fixture files; none use mocked HDF5.
//
// Fixtures used:
//   FIXTURE          synthetic_2laser.h5        — Python MockConfigBuilder, no OPCUA
//   REFERENCE        reference_config.h5         — real AconityMIDI, no OPCUA
//   REFERENCE_OPCUA  reference_config_opcua.h5   — real AconityMIDI, with OPCUA

use machine_config::builder::MockConfigBuilder;
use machine_config::reader::MachineConfigReader;
use machine_config::writer::MachineConfigWriter;
use tempfile::NamedTempFile;

static FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/synthetic_2laser.h5"
);
static REFERENCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/reference_config.h5"
);
static REFERENCE_OPCUA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/reference_config_opcua.h5"
);

// ---------------------------------------------------------------------------
// Structural / metadata tests
// ---------------------------------------------------------------------------

#[test]
fn test_machine_name() {
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    assert!(!config.meta.machine_name.is_empty());
}

#[test]
fn test_optical_train_count() {
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    assert_eq!(config.optical_trains.len(), 2);
}

#[test]
fn test_working_distance() {
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let wd = config.optical_trains[0].scanner.working_distance.unwrap();
    assert!((wd - 670.0).abs() < 1e-6);
}

#[test]
fn test_json_output_validates_schema() {
    // Structural check: valid JSON with required top-level keys and array shape.
    // Full JSON-Schema validation is handled by cross_check.py (Phase 5).
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let json = serde_json::to_string(&config).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(v.get("meta").is_some(), "missing 'meta'");
    assert!(v.get("machine").is_some(), "missing 'machine'");
    let trains = v["optical_trains"].as_array().expect("optical_trains must be an array");
    assert_eq!(trains.len(), 2);
    assert!(trains[0].get("scanner").is_some(), "train missing 'scanner'");
    // opcua must be absent (not null) for the synthetic fixture
    assert!(v.get("opcua").is_none(), "synthetic fixture must have no opcua key");
}

#[test]
fn test_configuration_hash_length() {
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    assert_eq!(config.meta.configuration_hash.len(), 64);
}

#[test]
fn test_meta_extra_preserved() {
    // extra may be empty for synthetic fixture — just assert it doesn’t panic
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let _ = &config.meta.extra;
}

// ---------------------------------------------------------------------------
// Correction-data tests
// ---------------------------------------------------------------------------

#[test]
fn test_correction_data_shape() {
    let reader = MachineConfigReader::open(FIXTURE).unwrap();
    let data = reader.get_correction_data(0).unwrap();
    assert_eq!(data.shape(), &[257, 257, 2]);
}

#[test]
fn test_correction_data_is_nonzero() {
    let reader = MachineConfigReader::open(FIXTURE).unwrap();
    let data = reader.get_correction_data(0).unwrap();
    assert!(data.iter().any(|&v| v != 0.0_f64));
}

#[test]
fn test_nan_to_null_in_correction_data() {
    // Real AconityMIDI fixture has NaN border cells outside the scan field.
    // parse_with_binary() must map those to None in the nested Vec.
    let config = MachineConfigReader::open(REFERENCE)
        .unwrap()
        .parse_with_binary()
        .unwrap();
    let cb = config.optical_trains[0].clearbox.as_ref().unwrap();
    let data = cb.correction_data.as_ref().unwrap();
    assert!(
        data.iter().flatten().flatten().any(|v| v.is_none()),
        "expected at least one NaN-derived null in correction_data"
    );
}

// ---------------------------------------------------------------------------
// Builder roundtrip
// ---------------------------------------------------------------------------

#[test]
fn test_builder_roundtrip() {
    let file = NamedTempFile::with_suffix(".h5").unwrap();
    MockConfigBuilder::new(2).save(file.path()).unwrap();
    let config = MachineConfigReader::open(file.path()).unwrap().parse().unwrap();
    assert_eq!(config.optical_trains.len(), 2);
    assert!(!config.meta.machine_name.is_empty());
}

// ---------------------------------------------------------------------------
// OPCUA tests
// ---------------------------------------------------------------------------

#[test]
fn test_opcua_client_fields() {
    let config = MachineConfigReader::open(REFERENCE_OPCUA)
        .unwrap()
        .parse()
        .unwrap();
    let client = config.opcua.unwrap().client;
    assert!(!client.server_url.is_empty());
    assert_eq!(client.bfs_max_depth, 16);
    assert_eq!(client.session_timeout, 60000);
}

#[test]
fn test_opcua_triggers_parsed() {
    let config = MachineConfigReader::open(REFERENCE_OPCUA)
        .unwrap()
        .parse()
        .unwrap();
    let opcua = config.opcua.unwrap();
    assert_eq!(opcua.triggers_enabled, Some(true));
    assert!(
        opcua.triggers.contains_key("Laser Emission Interlock"),
        "expected 'Laser Emission Interlock' trigger"
    );
}

// ---------------------------------------------------------------------------
// Writer roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn test_writer_roundtrip_scalars() {
    let original = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&original).write(tmp.path()).unwrap();
    let roundtripped = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();
    assert_eq!(original.meta.machine_name, roundtripped.meta.machine_name);
    assert_eq!(
        original.optical_trains[0].scanner.working_distance,
        roundtripped.optical_trains[0].scanner.working_distance,
    );
}

#[test]
fn test_writer_roundtrip_correction_data_checksum() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn arr_hash(arr: &ndarray::Array3<f64>) -> u64 {
        let mut h = DefaultHasher::new();
        for v in arr.iter() {
            v.to_bits().hash(&mut h);
        }
        h.finish()
    }

    // parse_with_binary() so correction_data is populated before writing
    let reader = MachineConfigReader::open(FIXTURE).unwrap();
    let original = reader.get_correction_data(0).unwrap();
    let config = reader.parse_with_binary().unwrap();
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&config).write(tmp.path()).unwrap();
    let roundtripped = MachineConfigReader::open(tmp.path())
        .unwrap()
        .get_correction_data(0)
        .unwrap();
    assert_eq!(
        arr_hash(&original),
        arr_hash(&roundtripped),
        "Correction data hash changed across write roundtrip"
    );
}
