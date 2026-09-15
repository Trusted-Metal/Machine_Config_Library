//! Stable model facade tests (File_Version 1.0).

use machine_config::capabilities::{
    create_machine_config, open_machine_config, supported_file_versions, SetMode,
};
use machine_config::capabilities::errors::CapabilityError;
use machine_config::reader::MachineConfigReader;
use machine_config::writer::MachineConfigWriter;
use tempfile::NamedTempFile;

static REFERENCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/reference_config.h5"
);
static REFERENCE_OPCUA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/reference_config_opcua.h5"
);
static OPCUA_MISSING_REQUIRED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/validation/fixtures/opcua_missing_required.h5"
);
static REFERENCE_SENSORS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/reference_config_synchronous_sensors.h5"
);
static REFERENCE_OPCUA_SENSORS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/reference_config_opcua_synchronous_sensors.h5"
);

#[test]
fn supported_versions() {
    assert!(supported_file_versions().contains(&"1.0"));
}

#[test]
fn open_get_scanner_matches_reader() {
    let file = open_machine_config(REFERENCE).unwrap();
    assert_eq!(file.file_version(), "1.0");
    let json = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    let scanner = file.get_scanner(0).unwrap();
    assert_eq!(
        scanner.working_distance,
        json.optical_trains[0].scanner.working_distance
    );
    assert_eq!(
        scanner.manufacturer,
        json.optical_trains[0].scanner.manufacturer
    );
}

#[test]
fn merge_set_scanner_roundtrip() {
    let mut file = open_machine_config(REFERENCE).unwrap();
    let mut scanner = file.get_scanner(0).unwrap();
    let manufacturer = scanner.manufacturer.clone();
    scanner.working_distance = Some(123.5);
    file.set_scanner(0, scanner, SetMode::Merge).unwrap();
    let tmp = NamedTempFile::new().unwrap();
    file.save(Some(tmp.path())).unwrap();

    let again = open_machine_config(tmp.path()).unwrap();
    let after = again.get_scanner(0).unwrap();
    assert_eq!(after.working_distance, Some(123.5));
    assert_eq!(after.manufacturer, manufacturer);
}

#[test]
fn replace_set_scanner() {
    let mut file = open_machine_config(REFERENCE).unwrap();
    let before = file.get_scanner(0).unwrap();
    assert!(before.scan_field_x.is_some());
    let mut replacement = before.clone();
    replacement.manufacturer = "ReplaceCo".into();
    replacement.working_distance = Some(1.0);
    replacement.scan_field_x = None;
    file.set_scanner(0, replacement, SetMode::Replace).unwrap();
    let after = file.get_scanner(0).unwrap();
    assert_eq!(after.manufacturer, "ReplaceCo");
    assert_eq!(after.working_distance, Some(1.0));
    assert!(after.scan_field_x.is_none());
}

#[test]
fn invalid_index() {
    let file = open_machine_config(REFERENCE).unwrap();
    let err = file.get_train(999).unwrap_err();
    assert!(matches!(err, CapabilityError::InvalidIndex(_)));
}

#[test]
fn opcua_not_present_vs_present() {
    let no_opc = open_machine_config(REFERENCE).unwrap();
    assert!(matches!(
        no_opc.get_opcua().unwrap_err(),
        CapabilityError::NotPresent(_)
    ));

    let with_opc = open_machine_config(REFERENCE_OPCUA).unwrap();
    assert!(with_opc.get_opcua().is_ok());
}

#[test]
fn opcua_required_fields_present_on_reference_fixture() {
    let file = open_machine_config(REFERENCE_OPCUA).unwrap();
    let opcua = file.get_opcua().expect("all required fields are present on this fixture");
    // Sanity-check a couple of the fields the check depends on, so a future
    // accidental fixture edit fails loudly here rather than only downstream.
    assert!(opcua.client.machine_profile.is_some());
    assert!(opcua.triggers.values().all(|t| t.event.is_some()));
}

#[test]
fn opcua_missing_required_fields_reports_all_seven_at_once() {
    let file = open_machine_config(OPCUA_MISSING_REQUIRED).unwrap();
    let err = file.get_opcua().unwrap_err();
    let details = match err {
        CapabilityError::ValidationError { details: Some(details), .. } => details,
        other => panic!("expected ValidationError with details, got {other:?}"),
    };

    let mut actual = details.clone();
    actual.sort();
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
    assert_eq!(actual, expected, "details must name exactly the seven missing fields");

    // The other trigger still has Event — must not be reported as missing.
    assert!(!details.iter().any(|d| d.starts_with("Chamber Oxygen Level")));
}

#[test]
fn opcua_optional_field_never_appears_in_missing_details() {
    // opcua_missing_required.h5 only clears the 7 required fields — every
    // optional field is still present there, so absence of an optional field
    // from `details` would be trivially true. To make this a real check
    // (not a tautology), also clear an optional field (keep_alive_count) in
    // memory, re-write to a temp file, and confirm `details` still names
    // exactly the same 7 required fields — not 8.
    let mut config = MachineConfigReader::open(OPCUA_MISSING_REQUIRED)
        .unwrap()
        .parse()
        .unwrap();
    config.opcua.as_mut().unwrap().client.keep_alive_count = None;
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&config).write(tmp.path()).unwrap();

    let file = open_machine_config(tmp.path()).unwrap();
    let err = file.get_opcua().unwrap_err();
    let details = match err {
        CapabilityError::ValidationError { details: Some(details), .. } => details,
        other => panic!("expected ValidationError with details, got {other:?}"),
    };

    assert!(
        !details.iter().any(|d| d.contains("Keep_Alive_Count")),
        "optional field must never appear in details, even when genuinely absent: {details:?}"
    );
    assert_eq!(details.len(), 7, "clearing an optional field must not change the missing count");
}

#[test]
fn clearbox_optional() {
    let file = open_machine_config(REFERENCE).unwrap();
    assert!(file.get_clearbox(0).unwrap().is_some());
}

#[test]
fn clearbox_synchronous_sensors_empty_when_fixture_has_none() {
    // reference_config.h5 deliberately has no Synchronous_Sensors group at
    // all (see SYNCHRONOUS_SENSOR_PLAN.md Phase 0) — the map must read back
    // empty, not missing/absent, since there is no separate "absent" state.
    let file = open_machine_config(REFERENCE).unwrap();
    let cb = file.get_clearbox(0).unwrap().unwrap();
    assert!(cb.synchronous_sensors.is_empty());
}

#[test]
fn clearbox_synchronous_sensors_present_and_named_on_dedicated_fixture() {
    let file = open_machine_config(REFERENCE_SENSORS).unwrap();
    let cb = file.get_clearbox(0).unwrap().unwrap();
    assert_eq!(cb.synchronous_sensors.len(), 1);
    let sensor = cb.synchronous_sensors.get("Oxygen Sensor")
        .expect("key is this fixture's arbitrary label, not a schema-significant name");
    assert_eq!(sensor.sensor_name, Some("ZR800 Oxygen Analyzer".to_string()));
    assert_eq!(sensor.port_id, Some(5));
}

#[test]
fn combined_fixture_has_opcua_and_synchronous_sensors_through_facade() {
    // Third-layer check for the combined fixture: hdf5.rs's own test module
    // proves the internal adapter, reader.rs proves the plain public
    // MachineConfigReader, this proves the capabilities facade too.
    let file = open_machine_config(REFERENCE_OPCUA_SENSORS).unwrap();
    assert!(file.get_opcua().is_ok(), "OPCUA must be present on the combined fixture");
    let cb = file.get_clearbox(0).unwrap().unwrap();
    assert_eq!(
        cb.synchronous_sensors["Oxygen Sensor"].sensor_name,
        Some("ZR800 Oxygen Analyzer".to_string())
    );
}

/// `CorrectionData` derives `PartialEq`, but IEEE 754 `NaN != NaN`, so a
/// direct `assert_eq!` on real grid data (which always contains NaN cells)
/// fails even when the two buffers are bit-for-bit identical. Compare shape
/// plus element-wise, treating "both NaN" as equal.
fn assert_correction_data_eq(
    a: &machine_config::models::CorrectionData,
    b: &machine_config::models::CorrectionData,
) {
    assert_eq!(a.shape, b.shape);
    assert_eq!(a.data.len(), b.data.len());
    for (x, y) in a.data.iter().zip(b.data.iter()) {
        if x.is_nan() || y.is_nan() {
            assert!(x.is_nan() && y.is_nan(), "NaN mismatch: {x} vs {y}");
        } else {
            assert_eq!(x, y);
        }
    }
}

#[test]
fn get_correction_data_matches_reader() {
    let file = open_machine_config(REFERENCE).unwrap();
    let reader = MachineConfigReader::open(REFERENCE).unwrap();
    assert_correction_data_eq(
        &file.get_correction_data(0).unwrap(),
        &reader.get_correction_data(0).unwrap(),
    );
    assert_correction_data_eq(
        &file.get_inverse_correction_data(0).unwrap(),
        &reader.get_inverse_correction_data(0).unwrap(),
    );
}

#[test]
fn get_correction_data_shape_and_nan_present_through_facade() {
    let file = open_machine_config(REFERENCE).unwrap();
    let cd = file.get_correction_data(0).unwrap();
    assert_eq!(cd.shape, [257, 257, 2]);
    assert!(cd.data.iter().any(|v| v.is_nan()));
}

#[test]
fn get_correction_data_missing_clearbox_is_not_present() {
    // Every stock fixture's trains have a ClearBox, so build one without: take
    // the reference config, strip train 0's ClearBox, re-write to a temp file.
    let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    config.optical_trains[0].optional_components.clearbox = None;
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&config).write(tmp.path()).unwrap();

    let file = open_machine_config(tmp.path()).unwrap();
    assert!(file.get_clearbox(0).unwrap().is_none());
    assert!(matches!(
        file.get_correction_data(0).unwrap_err(),
        CapabilityError::NotPresent(_)
    ));
    assert!(matches!(
        file.get_inverse_correction_data(0).unwrap_err(),
        CapabilityError::NotPresent(_)
    ));
}

#[test]
fn get_correction_data_works_on_create_based_instance_without_touching_disk() {
    // The specific case that rules out delegate-to-Reader-by-reopening: a
    // create()-d facade has no path at all, so this must convert the
    // already-loaded in-memory model, not re-read from anywhere.
    let file = create_machine_config("1.0").unwrap();
    let cd = file.get_correction_data(0).unwrap();
    assert_eq!(cd.shape, [257, 257, 2]);
    let icd = file.get_inverse_correction_data(0).unwrap();
    assert_eq!(icd.shape, [257, 257, 2]);
}

#[test]
fn create_set_meta_save_reopen() {
    let mut file = create_machine_config("1.0").unwrap();
    assert_eq!(file.file_version(), "1.0");
    let mut meta = file.get_meta().unwrap();
    meta.machine_name = "CreatedMachine".into();
    file.set_meta(meta, SetMode::Merge).unwrap();
    let tmp = NamedTempFile::new().unwrap();
    file.save(Some(tmp.path())).unwrap();

    let again = open_machine_config(tmp.path()).unwrap();
    assert_eq!(again.file_version(), "1.0");
    assert_eq!(again.get_meta().unwrap().machine_name, "CreatedMachine");
}

// ---------------------------------------------------------------------------
// is_valid() — CAPABILITIES_HASH_INTEGRATION_PLAN.md
// ---------------------------------------------------------------------------

static REFERENCE_V1_1: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/reference_config_v1_1.h5"
);

#[test]
fn is_valid_false_for_externally_authored_reference_fixture() {
    // Permanent regression guard, mirroring CONFIGURATION_HASH_PLAN.md's own
    // equivalent at the MachineConfigReader layer: reference_config.h5 was
    // authored externally, so MCF's recomputed hash will never match its
    // stored value. Expected and correct, not a defect.
    let file = open_machine_config(REFERENCE).unwrap();
    assert!(!file.is_valid());
}

#[test]
fn is_valid_true_for_self_written_file() {
    let mut file = create_machine_config("1.0").unwrap();
    let tmp = NamedTempFile::new().unwrap();
    file.save(Some(tmp.path())).unwrap();
    assert!(file.is_valid());

    let reopened = open_machine_config(tmp.path()).unwrap();
    assert!(reopened.is_valid());
}

#[test]
fn is_valid_flips_live_across_an_unsaved_edit_then_a_save() {
    // The test this whole live-method design exists to satisfy: a cached,
    // computed-once-at-open() value could never distinguish these states.
    let mut file = create_machine_config("1.0").unwrap();
    let tmp = NamedTempFile::new().unwrap();
    file.save(Some(tmp.path())).unwrap();
    assert!(file.is_valid(), "freshly saved content must be valid");

    let stored_before = file.get_meta().unwrap().configuration_hash;
    let mut scanner = file.get_scanner(0).unwrap();
    scanner.working_distance = Some(999.5);
    file.set_scanner(0, scanner, SetMode::Merge).unwrap();

    // Live-ness: an unsaved edit must be reflected immediately, with no
    // save()/reopen in between. This is expected, not a corruption signal
    // (see CAPABILITIES_HASH_INTEGRATION_PLAN.md's documented caveat).
    assert!(
        !file.is_valid(),
        "an unsaved edit must read as not-yet-hashed, live, without saving"
    );

    // The save→live round trip: is_valid() must flip back to true from the
    // *same session*, without reopening — this is what catches a save()
    // that forgets to update its own session state, which a test that only
    // reopens fresh could never distinguish from a save() that does it
    // correctly.
    file.save(Some(tmp.path())).unwrap();
    assert!(
        file.is_valid(),
        "save() must update session state so is_valid() is true without reopening"
    );
    assert_ne!(
        file.get_meta().unwrap().configuration_hash,
        stored_before,
        "the edit must have produced a different hash"
    );

    // And a completely fresh session on the same path agrees.
    let reopened = open_machine_config(tmp.path()).unwrap();
    assert!(reopened.is_valid());
    assert_eq!(
        reopened.get_meta().unwrap().configuration_hash,
        file.get_meta().unwrap().configuration_hash
    );
}

#[test]
fn v1_0_save_still_routes_through_real_hash_computation() {
    // Regression guard for the one language/version combination that was
    // already correct before this plan (v1.0's save() already used
    // MachineConfigWriter) — a future refactor must not silently
    // reintroduce a bypass back to a raw per-version writer call.
    let mut file = open_machine_config(REFERENCE).unwrap();
    let original_stored = file.get_meta().unwrap().configuration_hash;
    let tmp = NamedTempFile::new().unwrap();
    file.save(Some(tmp.path())).unwrap();

    let rt = open_machine_config(tmp.path()).unwrap();
    let rt_hash = rt.get_meta().unwrap().configuration_hash;
    assert_eq!(rt_hash.len(), 64);
    assert_ne!(rt_hash, original_stored);
    assert!(rt.is_valid());
}

#[test]
fn v1_1_save_computes_real_hash_and_updates_session() {
    // Targets the specific bug this plan found: v1.1's save() bypassed
    // MachineConfigWriter entirely (wrote via Hdf5WriterV1_1 directly),
    // silently passing through whatever configuration_hash was already in
    // memory rather than a fresh one.
    let mut file = open_machine_config(REFERENCE_V1_1).unwrap();
    assert_eq!(file.file_version(), "1.1");
    let original_stored = file.get_meta().unwrap().configuration_hash;

    let mut scanner = file.get_scanner(0).unwrap();
    scanner.working_distance = Some(555.0);
    file.set_scanner(0, scanner, SetMode::Merge).unwrap();
    assert!(!file.is_valid(), "unsaved edit must read false, live");

    let tmp = NamedTempFile::new().unwrap();
    file.save(Some(tmp.path())).unwrap();

    // Live, no reopen:
    assert!(file.is_valid());
    let saved_hash = file.get_meta().unwrap().configuration_hash;
    assert_eq!(saved_hash.len(), 64);
    assert_ne!(saved_hash, original_stored);

    // Fresh session on the written file agrees:
    let reopened = open_machine_config(tmp.path()).unwrap();
    assert_eq!(reopened.file_version(), "1.1");
    assert!(reopened.is_valid());
    assert_eq!(reopened.get_meta().unwrap().configuration_hash, saved_hash);
}
