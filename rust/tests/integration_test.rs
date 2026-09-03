// Phase 3.9 — Integration tests.
// All tests run against real fixture files; none use mocked HDF5.
//
// Fixtures used:
//   FIXTURE          synthetic_2laser.h5        — Python MockConfigBuilder, no OPCUA
//   REFERENCE        reference_config.h5         — real AconityMIDI, no OPCUA
//   REFERENCE_OPCUA  reference_config_opcua.h5   — real AconityMIDI, with OPCUA

use machine_config::builder::MockConfigBuilder;
use machine_config::capabilities::open_machine_config;
use machine_config::capabilities::errors::CapabilityError;
use machine_config::error::MachineConfigError;
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
    let cd = reader.get_correction_data(0).unwrap();
    assert_eq!(cd.shape, [257, 257, 2]);
}

#[test]
fn test_correction_data_is_nonzero() {
    let reader = MachineConfigReader::open(FIXTURE).unwrap();
    let cd = reader.get_correction_data(0).unwrap();
    assert!(cd.data.iter().any(|&v| v != 0.0_f64));
}

#[test]
fn test_nan_to_null_in_correction_data() {
    // Real AconityMIDI fixture has NaN border cells outside the scan field.
    // parse_with_binary() must map those to None in the nested Vec.
    let config = MachineConfigReader::open(REFERENCE)
        .unwrap()
        .parse_with_binary()
        .unwrap();
    let cb = config.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
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
    use machine_config::models::CorrectionData;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn cd_hash(cd: &CorrectionData) -> u64 {
        let mut h = DefaultHasher::new();
        for v in &cd.data {
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
        cd_hash(&original),
        cd_hash(&roundtripped),
        "Correction data hash changed across write roundtrip"
    );
}

#[test]
fn test_writer_roundtrip_opcua() {
    let original = MachineConfigReader::open(REFERENCE_OPCUA)
        .unwrap()
        .parse()
        .unwrap();
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&original).write(tmp.path()).unwrap();
    let rt = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();

    let orig_opcua = original.opcua.as_ref().unwrap();
    let rt_opcua = rt.opcua.as_ref().unwrap();

    assert_eq!(rt_opcua.client.server_url, orig_opcua.client.server_url);
    assert_eq!(rt_opcua.client.session_timeout, orig_opcua.client.session_timeout);
    assert_eq!(rt_opcua.client.bfs_max_depth, orig_opcua.client.bfs_max_depth);
    assert_eq!(rt_opcua.pipe.buffer_size, orig_opcua.pipe.buffer_size);
    assert_eq!(rt_opcua.triggers_enabled, orig_opcua.triggers_enabled);
    assert_eq!(rt_opcua.triggers.len(), orig_opcua.triggers.len());

    // Verify individual trigger field values survive the write→read cycle.
    let orig_t = orig_opcua.triggers.get("Chamber Oxygen Level").unwrap();
    let rt_t   = rt_opcua.triggers.get("Chamber Oxygen Level").unwrap();
    assert_eq!(rt_t.signal,       orig_t.signal);
    assert_eq!(rt_t.subsystem,    orig_t.subsystem);
    assert_eq!(rt_t.rule_enabled, orig_t.rule_enabled);
    assert_eq!(rt_t.start_value,  orig_t.start_value);
    assert_eq!(rt_t.stop_value,   orig_t.stop_value);
}

fn write_future_file_version(path: &std::path::Path) {
    let f = hdf5::File::create(path).expect("create future.h5");
    let v: hdf5::types::VarLenUnicode = "2.0".parse().unwrap();
    f.new_attr::<hdf5::types::VarLenUnicode>()
        .create("File_Version")
        .unwrap()
        .write_scalar(&v)
        .unwrap();
}

#[test]
fn test_unknown_file_version_does_not_use_v1_layout() {
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    write_future_file_version(tmp.path());

    match MachineConfigReader::open(tmp.path()) {
        Err(err) => assert!(
            matches!(err, MachineConfigError::UnsupportedVersion(ref v) if v == "2.0"),
            "reader error: {err}"
        ),
        Ok(_) => panic!("expected UnsupportedVersion from reader"),
    }

    match open_machine_config(tmp.path()) {
        Err(cap) => assert!(
            matches!(cap, CapabilityError::UnsupportedVersion(ref m) if m.contains("2.0")),
            "facade error: {cap}"
        ),
        Ok(_) => panic!("expected UnsupportedVersion from facade"),
    }
}

#[test]
fn test_writer_rejects_unknown_file_version() {
    let mut cfg = MachineConfigReader::open(REFERENCE)
        .unwrap()
        .parse()
        .unwrap();
    cfg.meta.file_version = "2.0".into();
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    let err = MachineConfigWriter::new(&cfg).write(tmp.path()).unwrap_err();
    assert!(
        matches!(err, MachineConfigError::UnsupportedVersion(ref v) if v == "2.0"),
        "writer error: {err}"
    );
}

// ---------------------------------------------------------------------------
// T-Rust — 12 reader field tests + 1 inverse correction writer test
// ---------------------------------------------------------------------------

#[test]
fn test_machine_serial_number() {
    let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    assert!(!config.machine.serial_number.is_empty());
}

#[test]
fn test_machine_gas_flow_direction() {
    let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    assert!(config.machine.gas_flow_direction.is_some());
}

#[test]
fn test_build_plate_y() {
    let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    let y = config.machine.build_plate_y.unwrap();
    assert!((y - 250.0).abs() < 1.0, "build_plate_y = {y}");
}

#[test]
fn test_build_plate_z() {
    let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    assert!(config.machine.build_plate_z.is_some());
}

#[test]
fn test_train_ids() {
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    assert_eq!(config.optical_trains[0].train_id, "Optical_Train_01");
    assert_eq!(config.optical_trains[1].train_id, "Optical_Train_02");
}

#[test]
fn test_scan_head_rotation_two_lasers() {
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let r0 = config.optical_trains[0].scanner.scan_head_rotation.unwrap();
    let r1 = config.optical_trains[1].scanner.scan_head_rotation.unwrap();
    assert!((r0 - 0.0).abs() < 0.01, "train 0 rotation = {r0}");
    assert!((r1 - 180.0).abs() < 0.01, "train 1 rotation = {r1}");
}

#[test]
fn test_scan_head_offset_y_two_lasers() {
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let y0 = config.optical_trains[0].scanner.scan_head_offset_y.expect("train 0 offset_y");
    let y1 = config.optical_trains[1].scanner.scan_head_offset_y.expect("train 1 offset_y");
    assert!((y0 - y1).abs() > 0.001, "trains should have different offset_y: {y0} vs {y1}");
}

#[test]
fn test_light_source_wavelength() {
    let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    let wl = config.optical_trains[0].light_source.wavelength.unwrap();
    assert!(wl > 0.0, "wavelength = {wl}");
}

#[test]
fn test_scanner_card_sample_period() {
    let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    let sp = config.optical_trains[0].scanner_card.sample_period.unwrap();
    assert!(sp > 0.0, "sample_period = {sp}");
}

#[test]
fn test_thermal_lensing_both_trains() {
    let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    assert_eq!(config.optical_trains[0].thermal_lensing_passed, Some(false));
    assert_eq!(config.optical_trains[1].thermal_lensing_passed, Some(true));
}

#[test]
fn test_sfcf_file_size() {
    let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    let sfcf = config.optical_trains[0].scan_field_correction_file.as_ref().unwrap();
    assert!(sfcf.file_size > 0);
}

#[test]
fn test_inverse_correction_differs_from_forward() {
    let reader = MachineConfigReader::open(REFERENCE).unwrap();
    let fwd = reader.get_correction_data(0).unwrap();
    let inv = reader.get_inverse_correction_data(0).unwrap();
    let first_fwd = fwd.data.iter().find(|v| v.is_finite()).copied().unwrap();
    let first_inv = inv.data.iter().find(|v| v.is_finite()).copied().unwrap();
    assert!(
        (first_fwd - first_inv).abs() > 1e-10,
        "forward and inverse correction grids must differ: fwd={first_fwd} inv={first_inv}"
    );
}

#[test]
fn test_writer_roundtrip_inverse_correction_data_checksum() {
    use machine_config::models::CorrectionData;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn cd_hash(cd: &CorrectionData) -> u64 {
        let mut h = DefaultHasher::new();
        for v in &cd.data {
            v.to_bits().hash(&mut h);
        }
        h.finish()
    }

    let reader = MachineConfigReader::open(FIXTURE).unwrap();
    let original_inv = reader.get_inverse_correction_data(0).unwrap();
    let config = reader.parse_with_binary().unwrap();
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&config).write(tmp.path()).unwrap();
    let roundtripped_inv = MachineConfigReader::open(tmp.path())
        .unwrap()
        .get_inverse_correction_data(0)
        .unwrap();
    assert_eq!(
        cd_hash(&original_inv),
        cd_hash(&roundtripped_inv),
        "Inverse correction data hash changed across write roundtrip"
    );
}
