//! AV-09–AV-11: mock v1.1 adapter migration tests.
//!
//! Mirrors `python/tests/test_adapter_migration.py`'s adapter-level tests
//! (test_v1_1_read, test_v1_1_roundtrip, test_v1_to_v1_1, test_v1_1_to_v1,
//! test_v1_unaffected) and `nodejs/tests/adapterMigration.test.ts`.
//!
//! Deliberately missing, by design (see VALIDATION_PLAN.md §9.3 and
//! `mock_v1_1/mod.rs`'s module doc): Python's/Node's *dispatcher-level*
//! tests (`test_dispatcher_v1_to_v1_1`, `test_dispatcher_v1_1_to_v1`), which
//! prove the public `MachineConfigReader`/`Writer` facade itself routes to
//! the mock via a temporarily-injected dispatch-table entry. Rust's public
//! dispatcher is a hardcoded `match "1.0" => ...` in `reader.rs`/`writer.rs`,
//! not a registry — there's no entry to inject. AV-09's actual rationale
//! ("adding v1.1 doesn't require modifying the v1.0 adapter") is satisfied
//! here by the mock living in its own file with zero edits to
//! `capabilities/v1_0/`, plus the full pre-existing suite (run below via
//! `v1_unaffected`, and via every other test in this crate) staying green.
//! Also skipped: Python's `test_mock_adapters_satisfy_protocol` — Rust has
//! no adapter trait/protocol to satisfy (the `adapters/` module was removed
//! as dead code earlier this branch).

mod mock_v1_1;

use machine_config::{MachineConfigReader, MachineConfigWriter};
use mock_v1_1::{make_mock_config, MockV1_1Reader, MockV1_1Writer};
use tempfile::NamedTempFile;

static REFERENCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config.h5");

/// Mock v1.1 file → StableModel — all five change categories asserted.
#[test]
fn v1_1_read_all_categories() {
    let cfg = make_mock_config(Some("MigrationTestMachine"), Some("Lab-001"), Some("TestEngineer"));
    let p = NamedTempFile::new().unwrap();
    MockV1_1Writer::new(&cfg).write(p.path()).unwrap();
    let result = MockV1_1Reader::new(p.path()).parse().unwrap();

    // ADDITION (x2)
    assert_eq!(result.meta.facility_id, Some("Lab-001".to_string()));
    assert_eq!(result.meta.config_author, Some("TestEngineer".to_string()));

    // REMOVAL (x2)
    assert_eq!(result.machine.gas_flow_direction, None);
    assert_eq!(result.machine.recoat_direction, None);

    // NAME (x2)
    assert_eq!(result.machine.machine_name, "MigrationTestMachine");
    assert_eq!(result.optical_trains[0].scanner.working_distance, cfg.optical_trains[0].scanner.working_distance);

    // PATH (x2)
    assert_eq!(result.machine.build_plate_z, cfg.machine.build_plate_z);
    assert_eq!(result.machine.build_plate_radius, cfg.machine.build_plate_radius);

    // NAME+PATH (x2)
    assert_eq!(result.machine.build_plate_x, cfg.machine.build_plate_x);
    assert_eq!(result.machine.build_plate_y, cfg.machine.build_plate_y);
}

/// Mock v1.1 → StableModel → mock v1.1 → StableModel — all categories
/// survive both passes.
#[test]
fn v1_1_roundtrip() {
    let cfg = make_mock_config(None, Some("RoundtripLab"), Some("RoundtripEngineer"));
    let p1 = NamedTempFile::new().unwrap();
    MockV1_1Writer::new(&cfg).write(p1.path()).unwrap();
    let mid = MockV1_1Reader::new(p1.path()).parse().unwrap();

    let p2 = NamedTempFile::new().unwrap();
    MockV1_1Writer::new(&mid).write(p2.path()).unwrap();
    let result = MockV1_1Reader::new(p2.path()).parse().unwrap();

    assert_eq!(result.meta.facility_id, Some("RoundtripLab".to_string()));
    assert_eq!(result.meta.config_author, Some("RoundtripEngineer".to_string()));
    assert_eq!(result.machine.gas_flow_direction, None);
    assert_eq!(result.machine.recoat_direction, None);
    assert_eq!(result.machine.machine_name, cfg.machine.machine_name);
    assert_eq!(
        result.optical_trains[0].scanner.working_distance,
        cfg.optical_trains[0].scanner.working_distance
    );
    assert_eq!(result.machine.build_plate_z, cfg.machine.build_plate_z);
    assert_eq!(result.machine.build_plate_radius, cfg.machine.build_plate_radius);
    assert_eq!(result.machine.build_plate_x, cfg.machine.build_plate_x);
    assert_eq!(result.machine.build_plate_y, cfg.machine.build_plate_y);
}

/// Real v1.0 fixture → StableModel → mock v1.1 layout — surviving fields
/// preserved; ADDITION fields None (no v1.0 source).
#[test]
fn v1_to_v1_1_forward_migration() {
    let source = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    let mut migrated_input = source.clone();
    migrated_input.meta.file_version = "1.1-mock".to_string();

    let out = NamedTempFile::new().unwrap();
    MockV1_1Writer::new(&migrated_input).write(out.path()).unwrap();
    let result = MockV1_1Reader::new(out.path()).parse().unwrap();

    // NAME
    assert_eq!(result.machine.machine_name, source.machine.machine_name);
    // NAME+PATH
    assert_eq!(result.machine.build_plate_x, source.machine.build_plate_x);
    assert_eq!(result.machine.build_plate_y, source.machine.build_plate_y);
    // PATH
    assert_eq!(result.machine.build_plate_z, source.machine.build_plate_z);
    assert_eq!(result.machine.build_plate_radius, source.machine.build_plate_radius);
    // NAME (scanner)
    assert_eq!(
        result.optical_trains[0].scanner.working_distance,
        source.optical_trains[0].scanner.working_distance
    );
    // REMOVAL: always None regardless of what the v1.0 source contained
    assert_eq!(result.machine.gas_flow_direction, None);
    assert_eq!(result.machine.recoat_direction, None);
    // ADDITION: no v1.0 source -> typed fields None after forward migration
    assert_eq!(result.meta.facility_id, None);
    assert_eq!(result.meta.config_author, None);
}

/// Mock v1.1 file → StableModel → v1.0 layout — surviving fields preserved;
/// ADDITION fields lost (v1.0 writer doesn't write them).
#[test]
fn v1_1_to_v1_backward_migration() {
    let cfg = make_mock_config(None, Some("Lab-V11"), Some("MigrationBot"));

    let v1_1_file = NamedTempFile::new().unwrap();
    MockV1_1Writer::new(&cfg).write(v1_1_file.path()).unwrap();
    let v1_1_config = MockV1_1Reader::new(v1_1_file.path()).parse().unwrap();

    let mut downgrade_input = v1_1_config.clone();
    downgrade_input.meta.file_version = "1.0".to_string();

    let v1_out = NamedTempFile::new().unwrap();
    MachineConfigWriter::new(&downgrade_input).write(v1_out.path()).unwrap();
    let result = MachineConfigReader::open(v1_out.path()).unwrap().parse().unwrap();

    assert_eq!(result.machine.machine_name, cfg.machine.machine_name);
    assert_eq!(result.machine.build_plate_x, cfg.machine.build_plate_x);
    assert_eq!(result.machine.build_plate_y, cfg.machine.build_plate_y);
    assert_eq!(result.machine.build_plate_z, cfg.machine.build_plate_z);
    assert_eq!(result.machine.build_plate_radius, cfg.machine.build_plate_radius);
    assert_eq!(
        result.optical_trains[0].scanner.working_distance,
        cfg.optical_trains[0].scanner.working_distance
    );

    // REMOVAL: absent in v1.1 -> remain None after round-trip through v1.0
    assert_eq!(result.machine.gas_flow_direction, None);
    assert_eq!(result.machine.recoat_direction, None);

    // ADDITION: typed v1.1 fields are lost during backward migration
    assert_eq!(result.meta.facility_id, None);
    assert_eq!(result.meta.config_author, None);
}

/// Existing v1.0 read path is undisturbed — no mock adapter involved. This,
/// combined with every other test in this crate staying green after the
/// mock was added, is AV-09's actual proof: adding v1.1 required zero edits
/// to the v1.0 adapter.
#[test]
fn v1_unaffected() {
    let config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
    assert_eq!(config.meta.file_version, "1.0");
    assert!(!config.optical_trains.is_empty());
    assert!(config.machine.build_plate_x.is_some());
}
