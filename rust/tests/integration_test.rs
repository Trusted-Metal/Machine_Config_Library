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
use machine_config::{extension_contract::DatasetData, MachineConfig};
use tempfile::NamedTempFile;

/// A float64 ClearBox grid (`Correction_Data` / `Inverse_Correction_Data`) of train `index`.
fn clearbox_grid(config: &MachineConfig, index: usize, name: &str) -> (Vec<usize>, Vec<f64>) {
    let ds = &config.extensions["ClearBox"].children[&format!("Optical_Train_{:02}", index + 1)]
        .datasets[name];
    match &ds.data {
        DatasetData::F64(v) => (ds.shape.clone(), v.clone()),
        _ => panic!("{name} is not float64"),
    }
}

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|v| v.to_bits()).collect()
}

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
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let (shape, _) = clearbox_grid(&config, 0, "Correction_Data");
    assert_eq!(shape, [257, 257, 2]);
}

#[test]
fn test_correction_data_is_nonzero() {
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let (_, data) = clearbox_grid(&config, 0, "Correction_Data");
    assert!(data.iter().any(|&v| v != 0.0_f64));
}

#[test]
fn test_nan_cells_survive_in_correction_data() {
    // Real AconityMIDI fixture has NaN border cells outside the scan field; the extension tree
    // carries them as real NaNs, not nulls.
    let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    let (_, data) = clearbox_grid(&config, 0, "Correction_Data");
    assert!(data.iter().any(|v| v.is_nan()), "expected NaN border cells in correction_data");
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
fn test_opcua_group_is_an_extension_tree() {
    let config = MachineConfigReader::open(REFERENCE_OPCUA).unwrap().parse().unwrap();
    let tree = &config.extensions["TM_OPCUA"];
    let client = &tree.children["Client"];
    assert!(!client.attrs["Server_URL"].as_str().unwrap().is_empty());
    assert_eq!(client.attrs["BFS_Max_Depth"].as_i64(), Some(16));
    assert_eq!(client.attrs["Session_Timeout"].as_i64(), Some(60000));
}

#[test]
fn test_opcua_triggers_parsed() {
    let config = MachineConfigReader::open(REFERENCE_OPCUA).unwrap().parse().unwrap();
    let triggers = &config.extensions["TM_OPCUA"].children["Triggers"];
    assert_eq!(triggers.attrs["Triggers_Enabled"].as_i64(), Some(1));
    assert!(
        triggers.children.contains_key("Laser Emission Interlock"),
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
    let original = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&original).write(tmp.path()).unwrap();
    let roundtripped = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();
    assert_eq!(
        bits(&clearbox_grid(&original, 0, "Correction_Data").1),
        bits(&clearbox_grid(&roundtripped, 0, "Correction_Data").1),
        "Correction data changed across write roundtrip"
    );
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
    let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    let (_, fwd) = clearbox_grid(&config, 0, "Correction_Data");
    let (_, inv) = clearbox_grid(&config, 0, "Inverse_Correction_Data");
    let first_fwd = fwd.iter().find(|v| v.is_finite()).copied().unwrap();
    let first_inv = inv.iter().find(|v| v.is_finite()).copied().unwrap();
    assert!(
        (first_fwd - first_inv).abs() > 1e-10,
        "forward and inverse correction grids must differ: fwd={first_fwd} inv={first_inv}"
    );
}

#[test]
fn test_writer_roundtrip_inverse_correction_data_checksum() {
    let original = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&original).write(tmp.path()).unwrap();
    let roundtripped = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();
    assert_eq!(
        bits(&clearbox_grid(&original, 0, "Inverse_Correction_Data").1),
        bits(&clearbox_grid(&roundtripped, 0, "Inverse_Correction_Data").1),
        "Inverse correction data changed across write roundtrip"
    );
}

#[test]
fn test_writer_roundtrip_opcua() {
    let original = MachineConfigReader::open(REFERENCE_OPCUA).unwrap().parse().unwrap();
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&original).write(tmp.path()).unwrap();
    let rt = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();

    let (a, b) = (&original.extensions["TM_OPCUA"], &rt.extensions["TM_OPCUA"]);
    assert_eq!(
        machine_config::canonical_digest(a).unwrap(),
        machine_config::canonical_digest(b).unwrap()
    );
    let oxygen = |t: &machine_config::ExtensionTree| t.children["Triggers"].children["Chamber Oxygen Level"].clone();
    assert_eq!(oxygen(a), oxygen(b));
}

