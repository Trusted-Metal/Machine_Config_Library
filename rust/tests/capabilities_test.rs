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
fn opcua_extension_tree_survives_a_session_save() {
    let mut file = open_machine_config(REFERENCE_OPCUA).unwrap();
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    file.save(Some(out.path())).unwrap();

    let before = MachineConfigReader::open(REFERENCE_OPCUA).unwrap().parse().unwrap();
    let after = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    assert_eq!(
        machine_config::canonical_digest(&after.extensions["TM_OPCUA"]).unwrap(),
        machine_config::canonical_digest(&before.extensions["TM_OPCUA"]).unwrap()
    );
}

#[test]
fn clearbox_extension_present_through_the_facade_session_save() {
    let mut file = open_machine_config(REFERENCE).unwrap();
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    file.save(Some(out.path())).unwrap();

    let before = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    let after = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    assert_eq!(
        machine_config::canonical_digest(&after.extensions["ClearBox"]).unwrap(),
        machine_config::canonical_digest(&before.extensions["ClearBox"]).unwrap()
    );
}

#[test]
fn clearbox_synchronous_sensors_absent_when_fixture_has_none() {
    // reference_config.h5 deliberately has no Synchronous_Sensors group at all.
    let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    let cb = &config.extensions["ClearBox"].children["Optical_Train_01"];
    assert!(!cb.children.contains_key("Synchronous_Sensors"));
}

#[test]
fn clearbox_synchronous_sensors_present_and_named_on_dedicated_fixture() {
    let config = MachineConfigReader::open(REFERENCE_SENSORS).unwrap().parse().unwrap();
    let sensors = &config.extensions["ClearBox"].children["Optical_Train_01"].children
        ["Synchronous_Sensors"];
    assert_eq!(sensors.children.len(), 1);
    let sensor = sensors
        .children
        .get("Oxygen Sensor")
        .expect("key is this fixture's arbitrary label, not a schema-significant name");
    assert_eq!(sensor.attrs["Sensor_Name"].as_str(), Some("ZR800 Oxygen Analyzer"));
    assert_eq!(sensor.attrs["Port_ID"].as_i64(), Some(5));
}

#[test]
fn combined_fixture_has_opcua_and_synchronous_sensors_through_facade() {
    // The facade opens the combined fixture, and the extension trees reach a session save intact.
    let mut file = open_machine_config(REFERENCE_OPCUA_SENSORS).unwrap();
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    file.save(Some(out.path())).unwrap();
    let config = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    assert!(config.extensions.contains_key("TM_OPCUA"), "OPCUA must be present");
    let sensors = &config.extensions["ClearBox"].children["Optical_Train_01"].children
        ["Synchronous_Sensors"];
    assert_eq!(
        sensors.children["Oxygen Sensor"].attrs["Sensor_Name"].as_str(),
        Some("ZR800 Oxygen Analyzer")
    );
}

#[test]
fn clearbox_absent_when_the_extension_is_removed() {
    // Every stock fixture has a ClearBox, so strip it and re-write.
    let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    config.extensions.remove("ClearBox");
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&config).write(tmp.path()).unwrap();

    let again = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();
    assert!(!again.extensions.contains_key("ClearBox"));
}

#[test]
fn create_based_instance_carries_the_clearbox_extension() {
    // A create()-d facade has no path at all, so the extension must come from the in-memory model.
    let mut file = create_machine_config("1.0").unwrap();
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    file.save(Some(out.path())).unwrap();
    let config = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    assert!(config.extensions.contains_key("ClearBox"));
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
