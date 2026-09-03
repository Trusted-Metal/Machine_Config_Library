//! File_Version 1.1 adapter tests — the real v1.1 adapter (not
//! `tests/mock_v1_1/`). Mirrors Python's
//! `python/tests/test_v1_1_adapter.py`, made concrete for
//! `docs/migrations/v1_0_to_v1_1.md`'s 5 changes, plus the extra hard-error
//! tests `V1_1_IMPLEMENTATION_PLAN.md`'s Testing section calls for.
//!
//! Phase 2 (V1_1_IMPLEMENTATION_PLAN.md) removed migrate_v1_to_v1_1/
//! migrate_v1_1_to_v1 — upgrade/downgrade is now just parse()/write(), using
//! `MachineConfigWriter::with_target_version`. Every test below that used to
//! call a migrate function directly now goes through a real HDF5 write+read
//! instead — a genuine strengthening, since it now exercises the same code
//! path a real caller uses.
//!
//! Tests
//! -----
//!   test_v1_1_read                                    natively-authored v1.1 fixture -> StableModel, all 5 changes asserted
//!   test_v1_1_roundtrip                                write -> read -> write -> read, cross-wiring guard
//!   test_v1_to_v1_1                                    real v1.0 fixture written as v1.1, values asserted
//!   test_v1_to_v1_1_output_path_disagreement_raises    Change 1 Consolidate hard error
//!   test_v1_to_v1_1_software_trigger_delay_disagreement_raises
//!   test_v1_to_v1_1_unrecognized_algorithm_type_best_effort              Change 3: never raises, best-effort
//!   test_v1_to_v1_1_unrecognized_algorithm_type_best_effort_for_light_source  Change 4
//!   test_v1_1_to_v1                                    real v1.1 fixture written as v1.0, values asserted
//!   test_v1_1_to_v1_reorders_constants_by_name         Change 3 backward: row-order hazard
//!   test_v1_1_to_v1_missing_derivation_constants_writes_blank
//!   test_v1_1_to_v1_missing_characterization_points_writes_blank
//!   test_v1_unaffected                                 v1.0 path undisturbed
//!   test_roundtrip_v1_0_to_v1_1_to_v1_0                the acceptance criterion, made concrete
//!   test_roundtrip_v1_1_to_v1_0_to_v1_1                mirror direction
//!   test_writer_target_version_overrides_meta
//!   test_writer_defaults_to_meta_file_version
//!   test_dispatcher_v1_to_v1_1 / test_dispatcher_v1_1_to_v1   full public API, real registry
//!   test_adapters_satisfy_protocol

use machine_config::capabilities::v1_1::{Hdf5AdapterV1_1, Hdf5WriterV1_1};
use machine_config::power_characterization::{
    forward_power_characterization_coefficients, forward_power_characterization_points,
};
use machine_config::reader::ReaderAdapter;
use machine_config::writer::WriterAdapter;
use machine_config::{
    CalibrationPoint, EquationConstant, MachineConfig, MachineConfigReader, MachineConfigWriter,
    MockConfigBuilder,
};
use tempfile::NamedTempFile;

const REFERENCE_V1_0: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/reference_config_opcua_synchronous_sensors.h5"
);
const REFERENCE_V1_1: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config_v1_1.h5");

fn mock_v1_0_config(n_lasers: usize) -> MachineConfig {
    MockConfigBuilder::new(n_lasers).build()
}

/// In-memory v1.1-shaped MachineConfig for tests that need one without
/// touching disk. `MockConfigBuilder` already builds `power_characterization`
/// natively now (POWER_CHARACTERIZATION_UNIFICATION_PLAN.md) — there's no
/// separate flat-field shape left to convert away from, so this is just the
/// v1.0 mock with `meta.file_version` overridden.
fn mock_v1_1_config(n_lasers: usize) -> MachineConfig {
    let mut cfg = mock_v1_0_config(n_lasers);
    cfg.meta.file_version = "1.1".to_string();
    cfg
}

fn names(constants: &[EquationConstant]) -> Vec<&str> {
    constants.iter().map(|c| c.name.as_str()).collect()
}

// ---------------------------------------------------------------------------
// test_v1_1_read
// ---------------------------------------------------------------------------

#[test]
fn test_v1_1_read() {
    let cfg = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    assert_eq!(cfg.meta.file_version, "1.1");

    for t in &cfg.optical_trains {
        let cb = t.optional_components.clearbox.as_ref().expect("clearbox present");
        // Change 1: Consolidate — same shared value on every train.
        assert_eq!(cb.output_path.as_deref(), Some("/recordings/"));
        assert_eq!(cb.software_trigger_delay, Some(3000));
        // Change 1: Addition.
        assert_eq!(cb.firmware_version.as_deref(), Some("2.4.1"));
        // Change 1: Removal.
        assert!(cb.selected_camera.is_none());
        assert!(cb.custom_video_format.is_none());
        assert!(cb.video_output.is_none());
        assert!(cb.show_console.is_none());
        assert!(cb.correction_grid_domain_shape.is_none());
        assert!(cb.inverse_grid_domain_shape.is_none());

        // Change 5: no on-disk source in v1.1, always None.
        assert!(t.scanner.x_axis.tuning_parameters.is_none());
        assert!(t.scanner.x_axis.tuning_type.is_none());
        assert!(t.scanner.y_axis.tuning_parameters.is_none());
        assert!(t.scanner.y_axis.tuning_type.is_none());
    }

    // Change 2: OPCUA relocated, contents unaffected.
    let opcua = cfg.opcua.as_ref().expect("OPCUA present");
    assert!(opcua.client.machine_profile.is_some());

    // Change 3: ClearBox Power_Characterization — train 1 LINEAR, train 2 POLYNOMIAL.
    let cb0 = cfg.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
    let pc0 = cb0.power_characterization.as_ref().expect("pc0 present");
    assert_eq!(pc0.algorithm_type.as_deref(), Some("LINEAR"));
    assert_eq!(pc0.algorithm_equation.as_deref(), Some("W = a*V + b"));
    assert_eq!(pc0.input_type.as_deref(), Some("0-10 V"));
    assert_eq!(pc0.units_derived_quantity.as_deref(), Some("Watts"));
    assert_eq!(names(&pc0.derivation_equation_constants), vec!["b", "a"]);
    assert_eq!(pc0.characterization_points.len(), 3);

    let cb1 = cfg.optical_trains[1].optional_components.clearbox.as_ref().unwrap();
    let pc1 = cb1.power_characterization.as_ref().expect("pc1 present");
    assert_eq!(pc1.algorithm_type.as_deref(), Some("POLYNOMIAL"));
    assert_eq!(pc1.algorithm_equation.as_deref(), Some("W = c0 + c1*V + c2*V^2"));
    assert_eq!(names(&pc1.derivation_equation_constants), vec!["c0", "c1", "c2"]);

    // Change 4: LightSource Power_Characterization — inverse data availability.
    let lspc0 =
        cfg.optical_trains[0].light_source.power_characterization.as_ref().expect("lspc0 present");
    assert_eq!(lspc0.algorithm_type.as_deref(), Some("LINEAR"));
    assert_eq!(lspc0.input_type.as_deref(), Some("Volts"));
    assert_eq!(lspc0.characterization_points.len(), 5);
    assert!(!lspc0.derivation_equation_constants.is_empty());
}

// ---------------------------------------------------------------------------
// test_v1_1_roundtrip
// ---------------------------------------------------------------------------

#[test]
fn test_v1_1_roundtrip() {
    // write -> read -> write -> read, no drift. ClearBox and Light_Source
    // power_characterization deliberately given different values in the
    // same object — direct test for the cross-wiring risk (same struct, two
    // HDF5 paths per train; a swapped read/write assignment would only be
    // caught if the two instances are distinguishable).
    let mut cfg = mock_v1_1_config(1);
    {
        let train = &mut cfg.optical_trains[0];
        let cb = train.optional_components.clearbox.as_mut().unwrap();
        cb.power_characterization = Some(PowerCharacterizationForTest::linear());
        train.light_source.power_characterization = Some(PowerCharacterizationForTest::polynomial());
    }

    let p1 = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&cfg).write(p1.path()).unwrap();
    let mid = MachineConfigReader::open(p1.path()).unwrap().parse().unwrap();
    let p2 = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&mid).write(p2.path()).unwrap();
    let result = MachineConfigReader::open(p2.path()).unwrap().parse().unwrap();

    let cb_r = result.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
    let ls_r = &result.optical_trains[0].light_source;
    let cb_pc = cb_r.power_characterization.as_ref().unwrap();
    let ls_pc = ls_r.power_characterization.as_ref().unwrap();
    assert_eq!(cb_pc.algorithm_type.as_deref(), Some("LINEAR"));
    assert_eq!(names(&cb_pc.derivation_equation_constants), vec!["b", "a"]);
    assert!(cb_pc.characterization_points.is_empty());
    assert_eq!(ls_pc.algorithm_type.as_deref(), Some("POLYNOMIAL"));
    assert!(ls_pc.derivation_equation_constants.is_empty());
    assert_eq!(ls_pc.characterization_points[0].input_value, 9.0);
    // Cross-wiring guard: the two instances must not have swapped.
    assert_ne!(cb_pc.algorithm_type, ls_pc.algorithm_type);
}

/// Tiny local helper so the roundtrip test above reads cleanly — builds the
/// two deliberately-distinguishable PowerCharacterization values inline.
struct PowerCharacterizationForTest;
impl PowerCharacterizationForTest {
    fn linear() -> machine_config::PowerCharacterization {
        machine_config::PowerCharacterization {
            algorithm_type: Some("LINEAR".to_string()),
            algorithm_equation: None,
            input_type: None,
            units_derived_quantity: None,
            derivation_equation_constants: vec![
                EquationConstant { name: "b".to_string(), value: 1.0 },
                EquationConstant { name: "a".to_string(), value: 2.0 },
            ],
            characterization_points: vec![],
        }
    }
    fn polynomial() -> machine_config::PowerCharacterization {
        machine_config::PowerCharacterization {
            algorithm_type: Some("POLYNOMIAL".to_string()),
            algorithm_equation: None,
            input_type: None,
            units_derived_quantity: None,
            derivation_equation_constants: vec![],
            characterization_points: vec![CalibrationPoint { input_value: 9.0, output_value: 99.0 }],
        }
    }
}

// ---------------------------------------------------------------------------
// test_v1_to_v1_1 (v1.0 fixture written as v1.1)
// ---------------------------------------------------------------------------

#[test]
fn test_v1_to_v1_1() {
    let source = MachineConfigReader::open(REFERENCE_V1_0).unwrap().parse().unwrap();
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&source, "1.1").write(out.path()).unwrap();
    let migrated = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    assert_eq!(migrated.meta.file_version, "1.1");

    for (i, t) in migrated.optical_trains.iter().enumerate() {
        let src_cb = source.optical_trains[i].optional_components.clearbox.as_ref().unwrap();
        let cb = t.optional_components.clearbox.as_ref().unwrap();
        // Consolidate: same per-train value carried straight through (real
        // fixture already agrees, confirmed in docs/migrations/v1_0_to_v1_1.md).
        assert_eq!(cb.output_path, src_cb.output_path);
        assert_eq!(cb.software_trigger_delay, src_cb.software_trigger_delay);
        // Addition: no v1.0 source.
        assert!(cb.firmware_version.is_none());
        // Removal.
        assert!(cb.selected_camera.is_none());
        assert!(cb.custom_video_format.is_none());
        // Change 3: derived ClearBox data has real constants, zero points.
        let pc = cb.power_characterization.as_ref().unwrap();
        assert!(pc.characterization_points.is_empty());
        assert!(!pc.derivation_equation_constants.is_empty());
        assert_eq!(pc.algorithm_type, src_cb.power_characterization.as_ref().unwrap().algorithm_type);

        // Change 4: derived Light_Source data is the inverse — zero
        // constants, real points.
        let ls_pc = t.light_source.power_characterization.as_ref().unwrap();
        assert!(ls_pc.derivation_equation_constants.is_empty());
        assert!(!ls_pc.characterization_points.is_empty());
    }
}

#[test]
fn test_v1_to_v1_1_output_path_disagreement_raises() {
    let mut cfg = mock_v1_0_config(2);
    cfg.optical_trains[1].optional_components.clearbox.as_mut().unwrap().output_path =
        Some("/other/".to_string());
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    let err = MachineConfigWriter::with_target_version(&cfg, "1.1").write(tmp.path()).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Consolidate conflict on 'Output_Path'"), "unexpected message: {msg}");
}

#[test]
fn test_v1_to_v1_1_software_trigger_delay_disagreement_raises() {
    let mut cfg = mock_v1_0_config(2);
    cfg.optical_trains[1]
        .optional_components
        .clearbox
        .as_mut()
        .unwrap()
        .software_trigger_delay = Some(9999);
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    let err = MachineConfigWriter::with_target_version(&cfg, "1.1").write(tmp.path()).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Consolidate conflict on 'Software_Trigger_Delay'"),
        "unexpected message: {msg}"
    );
}

#[test]
fn test_v1_to_v1_1_unrecognized_algorithm_type_best_effort() {
    // Phase 2: never a hard error — best-effort, verbatim algorithm_type,
    // positionally-named constants, blank algorithm_equation.
    let mut cfg = mock_v1_0_config(1);
    {
        let cb = cfg.optical_trains[0].optional_components.clearbox.as_mut().unwrap();
        cb.power_characterization = forward_power_characterization_coefficients(
            Some("EXPONENTIAL"),
            Some("1.5,2.5,3.5"),
        )
        .unwrap();
    }
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&cfg, "1.1").write(out.path()).unwrap();
    let result = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    let pc = result.optical_trains[0]
        .optional_components
        .clearbox
        .as_ref()
        .unwrap()
        .power_characterization
        .as_ref()
        .unwrap();
    assert_eq!(pc.algorithm_type.as_deref(), Some("EXPONENTIAL"));
    assert!(pc.algorithm_equation.is_none());
    assert_eq!(names(&pc.derivation_equation_constants), vec!["0", "1", "2"]);
    let values: Vec<f64> = pc.derivation_equation_constants.iter().map(|c| c.value).collect();
    assert_eq!(values, vec![1.5, 2.5, 3.5]);

    // Round trip: writing back to v1.0 and re-reading reproduces the same
    // structure — v1.0's reader now forward-derives power_characterization
    // from whatever backward_flat_fields_coefficients wrote, so there's no
    // more raw string field to compare directly.
    let v1_0_out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&result, "1.0").write(v1_0_out.path()).unwrap();
    let back = MachineConfigReader::open(v1_0_out.path()).unwrap().parse().unwrap();
    let back_pc = back.optical_trains[0]
        .optional_components
        .clearbox
        .as_ref()
        .unwrap()
        .power_characterization
        .as_ref()
        .unwrap();
    assert_eq!(back_pc.algorithm_type.as_deref(), Some("EXPONENTIAL"));
    let back_values: Vec<f64> = back_pc.derivation_equation_constants.iter().map(|c| c.value).collect();
    assert_eq!(back_values, vec![1.5, 2.5, 3.5]);
}

#[test]
fn test_v1_to_v1_1_unrecognized_algorithm_type_best_effort_for_light_source() {
    let mut cfg = mock_v1_0_config(1);
    {
        let ls = &mut cfg.optical_trains[0].light_source;
        ls.power_characterization =
            forward_power_characterization_points(Some("QUADRATIC"), Some("1,2,3,4")).unwrap();
    }
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&cfg, "1.1").write(out.path()).unwrap();
    let result = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    let pc = result.optical_trains[0].light_source.power_characterization.as_ref().unwrap();
    assert_eq!(pc.algorithm_type.as_deref(), Some("QUADRATIC"));
    assert!(pc.algorithm_equation.is_none());
    assert_eq!(pc.characterization_points.len(), 2);
}

// ---------------------------------------------------------------------------
// test_v1_1_to_v1 (v1.1 fixture written as v1.0)
// ---------------------------------------------------------------------------

#[test]
fn test_v1_1_to_v1() {
    let source = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&source, "1.0").write(out.path()).unwrap();
    let back = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    assert_eq!(back.meta.file_version, "1.0");

    let src_cb0 = source.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
    let cb0 = back.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
    let src_pc0 = src_cb0.power_characterization.as_ref().unwrap();
    let pc0 = cb0.power_characterization.as_ref().unwrap();
    assert_eq!(pc0.algorithm_type, src_pc0.algorithm_type);
    let mut expected: Vec<f64> =
        src_pc0.derivation_equation_constants.iter().map(|c| c.value).collect();
    expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut actual: Vec<f64> = pc0.derivation_equation_constants.iter().map(|c| c.value).collect();
    actual.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(actual, expected);
    assert_eq!(cb0.output_path, src_cb0.output_path);
    assert_eq!(cb0.software_trigger_delay, src_cb0.software_trigger_delay);

    let src_ls0 = &source.optical_trains[0].light_source;
    let ls0 = &back.optical_trains[0].light_source;
    let src_ls_pc0 = src_ls0.power_characterization.as_ref().unwrap();
    let ls_pc0 = ls0.power_characterization.as_ref().unwrap();
    assert_eq!(ls_pc0.algorithm_type, src_ls_pc0.algorithm_type);
    let expected_points: Vec<(f64, f64)> = src_ls_pc0
        .characterization_points
        .iter()
        .map(|p| (p.input_value, p.output_value))
        .collect();
    let actual_points: Vec<(f64, f64)> =
        ls_pc0.characterization_points.iter().map(|p| (p.input_value, p.output_value)).collect();
    assert_eq!(actual_points, expected_points);

    // Change 1 Removal fields: lost forever, not restored.
    assert!(cb0.selected_camera.is_none());
}

#[test]
fn test_v1_1_to_v1_reorders_constants_by_name() {
    // Change 3 backward: Derivation_Equation_Constants rows aren't
    // positionally guaranteed — must re-sort by name before joining as CSV.
    let mut cfg = mock_v1_1_config(1);
    {
        let cb0 = cfg.optical_trains[0].optional_components.clearbox.as_mut().unwrap();
        cb0.power_characterization = Some(machine_config::PowerCharacterization {
            algorithm_type: Some("LINEAR".to_string()),
            algorithm_equation: None,
            input_type: None,
            units_derived_quantity: None,
            derivation_equation_constants: vec![
                EquationConstant { name: "a".to_string(), value: 2.0 },
                EquationConstant { name: "b".to_string(), value: 1.0 },
            ],
            characterization_points: vec![],
        });
    }
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&cfg, "1.0").write(out.path()).unwrap();
    let back = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    let back_pc = back.optical_trains[0]
        .optional_components
        .clearbox
        .as_ref()
        .unwrap()
        .power_characterization
        .as_ref()
        .unwrap();
    // b before a for LINEAR confirms the writer re-sorted by name before
    // joining as CSV — checked via the reader's forward-derivation, since
    // the raw CSV string is no longer exposed on the StableModel directly.
    assert_eq!(names(&back_pc.derivation_equation_constants), vec!["b", "a"]);
    let values: Vec<f64> = back_pc.derivation_equation_constants.iter().map(|c| c.value).collect();
    assert_eq!(values, vec![1.0, 2.0]);
}

#[test]
fn test_v1_1_to_v1_missing_derivation_constants_writes_blank() {
    // Change 3 backward: empty Derivation_Equation_Constants -> blank, not an error.
    let mut cfg = mock_v1_1_config(1);
    {
        let cb0 = cfg.optical_trains[0].optional_components.clearbox.as_mut().unwrap();
        let mut pc = cb0.power_characterization.clone().unwrap();
        pc.derivation_equation_constants = vec![];
        cb0.power_characterization = Some(pc);
    }
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&cfg, "1.0").write(out.path()).unwrap();
    let back = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    let back_pc = back.optical_trains[0]
        .optional_components
        .clearbox
        .as_ref()
        .unwrap()
        .power_characterization
        .as_ref()
        .unwrap();
    // Volts_To_Watts_Params written blank ("", not an error) — forward-
    // derivation on read correctly comes back with zero constants, not a
    // crash. Checked via power_characterization since the raw string is no
    // longer exposed on the StableModel directly.
    assert!(back_pc.derivation_equation_constants.is_empty());
}

#[test]
fn test_v1_1_to_v1_missing_characterization_points_writes_blank() {
    // Change 4 backward: empty Characterization_Points -> blank, not an error.
    let mut cfg = mock_v1_1_config(1);
    {
        let ls0 = &mut cfg.optical_trains[0].light_source;
        let mut pc = ls0.power_characterization.clone().unwrap();
        pc.characterization_points = vec![];
        ls0.power_characterization = Some(pc);
    }
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&cfg, "1.0").write(out.path()).unwrap();
    let back = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    // Same reasoning as the Change 3 case above.
    assert!(back.optical_trains[0]
        .light_source
        .power_characterization
        .as_ref()
        .unwrap()
        .characterization_points
        .is_empty());
}

// ---------------------------------------------------------------------------
// test_v1_unaffected
// ---------------------------------------------------------------------------

#[test]
fn test_v1_unaffected() {
    // Existing v1.0 read path still reads the same underlying HDF5
    // attributes — the unification plan didn't change what's on disk for
    // v1.0 files, only how the StableModel represents it (always via
    // power_characterization now, forward-derived; the flat fields it used
    // to populate no longer exist on the StableModel at all).
    let config = MachineConfigReader::open(REFERENCE_V1_0).unwrap().parse().unwrap();
    assert_eq!(config.meta.file_version, "1.0");
    assert!(!config.optical_trains.is_empty());
    let cb0 = config.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
    let pc0 = cb0.power_characterization.as_ref().expect("power_characterization present");
    assert_eq!(pc0.algorithm_type.as_deref(), Some("LINEAR"));
}

// ---------------------------------------------------------------------------
// Round-trip tests — the acceptance criterion stated 2026-09-01, made concrete
// ---------------------------------------------------------------------------

#[test]
fn test_roundtrip_v1_0_to_v1_1_to_v1_0() {
    let source = MachineConfigReader::open(REFERENCE_V1_0).unwrap().parse().unwrap();
    let v1_1_path = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&source, "1.1").write(v1_1_path.path()).unwrap();
    let mid = MachineConfigReader::open(v1_1_path.path()).unwrap().parse().unwrap();
    let v1_0_path = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&mid, "1.0").write(v1_0_path.path()).unwrap();
    let back = MachineConfigReader::open(v1_0_path.path()).unwrap().parse().unwrap();

    assert_eq!(back.meta.file_version, "1.0");
    for (i, t) in back.optical_trains.iter().enumerate() {
        let src_cb = source.optical_trains[i].optional_components.clearbox.as_ref().unwrap();
        let cb = t.optional_components.clearbox.as_ref().unwrap();
        assert_eq!(cb.output_path, src_cb.output_path);
        assert_eq!(cb.software_trigger_delay, src_cb.software_trigger_delay);
        let src_pc = src_cb.power_characterization.as_ref().unwrap();
        let pc = cb.power_characterization.as_ref().unwrap();
        assert_eq!(pc.algorithm_type, src_pc.algorithm_type);
        let mut a: Vec<f64> = pc.derivation_equation_constants.iter().map(|c| c.value).collect();
        let mut b: Vec<f64> = src_pc.derivation_equation_constants.iter().map(|c| c.value).collect();
        a.sort_by(|x, y| x.partial_cmp(y).unwrap());
        b.sort_by(|x, y| x.partial_cmp(y).unwrap());
        assert_eq!(a, b);

        let src_ls = &source.optical_trains[i].light_source;
        let ls = &t.light_source;
        let src_ls_pc = src_ls.power_characterization.as_ref().unwrap();
        let ls_pc = ls.power_characterization.as_ref().unwrap();
        assert_eq!(ls_pc.algorithm_type, src_ls_pc.algorithm_type);
        let src_points: Vec<(f64, f64)> =
            src_ls_pc.characterization_points.iter().map(|p| (p.input_value, p.output_value)).collect();
        let back_points: Vec<(f64, f64)> =
            ls_pc.characterization_points.iter().map(|p| (p.input_value, p.output_value)).collect();
        assert_eq!(back_points, src_points);
    }
}

#[test]
fn test_roundtrip_v1_1_to_v1_0_to_v1_1() {
    // Mirror direction: starting from a natively-authored v1.1 file,
    // downgrade then upgrade again. Fields v1.0 can represent survive;
    // fields only v1.1 can hold come back blank — expected, documented
    // richer-to-simpler loss, not a bug.
    let source = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    let v1_0_path = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&source, "1.0").write(v1_0_path.path()).unwrap();
    let mid = MachineConfigReader::open(v1_0_path.path()).unwrap().parse().unwrap();
    let v1_1_path = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&mid, "1.1").write(v1_1_path.path()).unwrap();
    let back = MachineConfigReader::open(v1_1_path.path()).unwrap().parse().unwrap();

    assert_eq!(back.meta.file_version, "1.1");
    for (i, t) in back.optical_trains.iter().enumerate() {
        let src_cb = source.optical_trains[i].optional_components.clearbox.as_ref().unwrap();
        let cb = t.optional_components.clearbox.as_ref().unwrap();
        let src_pc = src_cb.power_characterization.as_ref().unwrap();
        let pc = cb.power_characterization.as_ref().unwrap();
        assert_eq!(pc.algorithm_type, src_pc.algorithm_type);
        let mut expected: Vec<f64> =
            src_pc.derivation_equation_constants.iter().map(|c| c.value).collect();
        expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut actual: Vec<f64> = pc.derivation_equation_constants.iter().map(|c| c.value).collect();
        actual.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(actual, expected);
        // Expected loss: v1.1-only fields have no v1.0 round-trip path.
        assert!(cb.firmware_version.is_none());
        assert!(pc.input_type.is_none());
        assert!(pc.units_derived_quantity.is_none());
        assert!(pc.characterization_points.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Writer target_version parameter
// ---------------------------------------------------------------------------

#[test]
fn test_writer_target_version_overrides_meta() {
    let source = MachineConfigReader::open(REFERENCE_V1_0).unwrap().parse().unwrap();
    assert_eq!(source.meta.file_version, "1.0");
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&source, "1.1").write(out.path()).unwrap();
    let result = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    assert_eq!(result.meta.file_version, "1.1");
    // Writer must not mutate its input.
    assert_eq!(source.meta.file_version, "1.0");

    let v1_1_source = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    assert_eq!(v1_1_source.meta.file_version, "1.1");
    let out2 = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&v1_1_source, "1.0").write(out2.path()).unwrap();
    let result2 = MachineConfigReader::open(out2.path()).unwrap().parse().unwrap();
    assert_eq!(result2.meta.file_version, "1.0");
    assert_eq!(v1_1_source.meta.file_version, "1.1");
}

#[test]
fn test_writer_defaults_to_meta_file_version() {
    let source = MachineConfigReader::open(REFERENCE_V1_0).unwrap().parse().unwrap();
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&source).write(out.path()).unwrap(); // no target_version
    let result = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    assert_eq!(result.meta.file_version, "1.0");
}

// ---------------------------------------------------------------------------
// Dispatcher-level tests — full public API via the real "1.1" registry entry.
// ---------------------------------------------------------------------------

#[test]
fn test_dispatcher_v1_to_v1_1() {
    let source = MachineConfigReader::open(REFERENCE_V1_0).unwrap().parse().unwrap();
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&source, "1.1").write(out.path()).unwrap();
    let result = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    assert_eq!(result.meta.file_version, "1.1");
    let cb = result.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
    assert_eq!(cb.output_path.as_deref(), Some("/recordings/"));
    assert!(cb.power_characterization.is_some());
}

#[test]
fn test_dispatcher_v1_1_to_v1() {
    let v1_1_cfg = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(&v1_1_cfg, "1.0").write(out.path()).unwrap();
    let result = MachineConfigReader::open(out.path()).unwrap().parse().unwrap();
    assert_eq!(result.meta.file_version, "1.0");
    let cb = result.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
    assert_eq!(
        cb.power_characterization.as_ref().unwrap().algorithm_type.as_deref(),
        Some("LINEAR")
    );
}

// ---------------------------------------------------------------------------
// test_adapters_satisfy_protocol
// ---------------------------------------------------------------------------

fn assert_is_reader_adapter(_r: &dyn ReaderAdapter) {}
fn assert_is_writer_adapter(_w: &dyn WriterAdapter) {}

#[test]
fn test_adapters_satisfy_protocol() {
    // In Rust this is primarily a compile-time fact (proved by the trait
    // impls in capabilities/v1_1/{hdf5,writer}.rs); the assert_is_*
    // functions below make it a concrete, runtime-checked test too, by
    // requiring a real `Hdf5AdapterV1_1`/`Hdf5WriterV1_1` value to coerce to
    // `&dyn ReaderAdapter`/`&dyn WriterAdapter`.
    let cfg = mock_v1_1_config(1);
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    Hdf5WriterV1_1::new(&cfg).write(tmp.path()).unwrap();

    let adapter = Hdf5AdapterV1_1::open(tmp.path()).unwrap();
    assert_is_reader_adapter(&adapter);

    let writer = Hdf5WriterV1_1::new(&cfg);
    assert_is_writer_adapter(&writer);
}
