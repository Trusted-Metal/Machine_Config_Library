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
