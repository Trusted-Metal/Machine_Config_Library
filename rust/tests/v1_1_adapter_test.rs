//! File_Version 1.1 adapter tests — the real v1.1 adapter (not
//! `tests/mock_v1_1/`). Mirrors Python's
//! `python/tests/test_v1_1_adapter.py`, made concrete for
//! `docs/migrations/v1_0_to_v1_1.md`'s 5 changes.
//!
//! ClearBox is an opaque extension tree in core, always in the File_Version 1.1 layout
//! (`ClearBox/<train_id>/Power_Characterization/`, shared `Output_Path` and
//! `Software_Trigger_Delay` on the extension root). A File_Version 1.0 file is shimmed into and
//! out of that layout on read and write, so version conversion is just `parse()`/`write()` using
//! `MachineConfigWriter::with_target_version`.

use machine_config::capabilities::v1_1::{Hdf5AdapterV1_1, Hdf5WriterV1_1};
use machine_config::extension_contract::{
    Attr, ColumnData, Dtype, ExtensionTree, Scalar, Table, TableField,
};
use machine_config::power_characterization::forward_power_characterization_points;
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

/// In-memory v1.1-shaped MachineConfig for tests that need one without touching disk: the v1.0
/// mock with `meta.file_version` overridden (its ClearBox tree is already in the v1.1 layout).
fn mock_v1_1_config(n_lasers: usize) -> MachineConfig {
    let mut cfg = mock_v1_0_config(n_lasers);
    cfg.meta.file_version = "1.1".to_string();
    cfg
}

fn train_key(index: usize) -> String {
    format!("Optical_Train_{:02}", index + 1)
}

fn clearbox(cfg: &MachineConfig, index: usize) -> &ExtensionTree {
    &cfg.extensions["ClearBox"].children[&train_key(index)]
}

fn clearbox_mut(cfg: &mut MachineConfig, index: usize) -> &mut ExtensionTree {
    cfg.extensions.get_mut("ClearBox").unwrap().children.get_mut(&train_key(index)).unwrap()
}

fn power_characterization(cfg: &MachineConfig, index: usize) -> &ExtensionTree {
    &clearbox(cfg, index).children["Power_Characterization"]
}

/// Non-empty string attribute; the contract stores "absent" as `""`.
fn text<'a>(tree: &'a ExtensionTree, name: &str) -> Option<&'a str> {
    tree.attrs.get(name).and_then(|a| a.as_str()).filter(|s| !s.is_empty())
}

fn constants(pc: &ExtensionTree) -> Vec<(String, f64)> {
    let t = &pc.tables["Derivation_Equation_Constants"];
    match (&t.columns[0], &t.columns[1]) {
        (ColumnData::Utf8(n), ColumnData::F64(v)) => n.iter().cloned().zip(v.iter().copied()).collect(),
        _ => panic!("unexpected column types"),
    }
}

fn constant_names(pc: &ExtensionTree) -> Vec<String> {
    constants(pc).into_iter().map(|c| c.0).collect()
}

fn constant_values(pc: &ExtensionTree) -> Vec<f64> {
    constants(pc).into_iter().map(|c| c.1).collect()
}

fn point_count(pc: &ExtensionTree) -> usize {
    pc.tables["Characterization_Points"].rows
}

fn sorted(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

fn make_power_characterization(algorithm: &str, constants: &[(&str, f64)]) -> ExtensionTree {
    let mut tree = ExtensionTree::default();
    for (name, value) in [
        ("Algorithm_Type", algorithm),
        ("Algorithm_Equation", ""),
        ("Input_Type", ""),
        ("Units_Derived_Quantity", ""),
    ] {
        tree.attrs.insert(name.into(), Attr::scalar(Scalar::Utf8(value.into())));
    }
    tree.tables.insert(
        "Derivation_Equation_Constants".into(),
        Table {
            fields: vec![TableField::utf8("name", 64), TableField::numeric("value", Dtype::F64)],
            rows: constants.len(),
            columns: vec![
                ColumnData::Utf8(constants.iter().map(|c| c.0.to_string()).collect()),
                ColumnData::F64(constants.iter().map(|c| c.1).collect()),
            ],
            attrs: Default::default(),
        },
    );
    tree.tables.insert(
        "Characterization_Points".into(),
        Table {
            fields: vec![
                TableField::numeric("input_value", Dtype::F64),
                TableField::numeric("output_value", Dtype::F64),
            ],
            rows: 0,
            columns: vec![ColumnData::F64(vec![]), ColumnData::F64(vec![])],
            attrs: Default::default(),
        },
    );
    tree
}

fn write_as(cfg: &MachineConfig, version: &str) -> NamedTempFile {
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::with_target_version(cfg, version).write(out.path()).unwrap();
    out
}

fn read(file: &NamedTempFile) -> MachineConfig {
    MachineConfigReader::open(file.path()).unwrap().parse().unwrap()
}

fn digest(cfg: &MachineConfig) -> String {
    machine_config::canonical_digest(&cfg.extensions["ClearBox"]).unwrap()
}

// ---------------------------------------------------------------------------
// test_v1_1_read
// ---------------------------------------------------------------------------

#[test]
fn test_v1_1_read() {
    let cfg = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    assert_eq!(cfg.meta.file_version, "1.1");

    // Change 1: Consolidate — one shared value, on the extension root.
    let root = &cfg.extensions["ClearBox"];
    assert_eq!(root.attrs["Output_Path"].as_str(), Some("/recordings/"));
    assert_eq!(root.attrs["Software_Trigger_Delay"].as_i64(), Some(3000));

    for (i, t) in cfg.optical_trains.iter().enumerate() {
        let cb = clearbox(&cfg, i);
        // Change 1: Addition.
        assert_eq!(cb.attrs["Firmware_Version"].as_str(), Some("2.4.1"));
        // Change 1: Removal.
        for removed in [
            "Selected_Camera",
            "Custom_Video_Format",
            "Video_Output",
            "Show_Console",
            "Output_Path",
            "Software_Trigger_Delay",
        ] {
            assert!(!cb.attrs.contains_key(removed), "{removed} must not be on a v1.1 train");
        }

        // Change 5: no on-disk source in v1.1, always None.
        assert!(t.scanner.x_axis.tuning_parameters.is_none());
        assert!(t.scanner.x_axis.tuning_type.is_none());
        assert!(t.scanner.y_axis.tuning_parameters.is_none());
        assert!(t.scanner.y_axis.tuning_type.is_none());
    }

    // Change 2: OPCUA relocated, contents unaffected.
    let opcua = cfg.extensions.get("TM_OPCUA").expect("OPCUA present");
    assert!(opcua.children["Client"].attrs.contains_key("Machine_Profile"));

    // Change 3: ClearBox Power_Characterization — train 1 LINEAR, train 2 POLYNOMIAL.
    let pc0 = power_characterization(&cfg, 0);
    assert_eq!(text(pc0, "Algorithm_Type"), Some("LINEAR"));
    assert_eq!(text(pc0, "Algorithm_Equation"), Some("W = a*V + b"));
    assert_eq!(text(pc0, "Input_Type"), Some("0-10 V"));
    assert_eq!(text(pc0, "Units_Derived_Quantity"), Some("Watts"));
    assert_eq!(constant_names(pc0), vec!["b", "a"]);
    assert_eq!(point_count(pc0), 3);

    let pc1 = power_characterization(&cfg, 1);
    assert_eq!(text(pc1, "Algorithm_Type"), Some("POLYNOMIAL"));
    assert_eq!(text(pc1, "Algorithm_Equation"), Some("W = c0 + c1*V + c2*V^2"));
    assert_eq!(constant_names(pc1), vec!["c0", "c1", "c2"]);

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
    // write -> read -> write -> read, no drift. ClearBox and Light_Source power
    // characterization deliberately given different values — direct test for the
    // cross-wiring risk (two HDF5 paths per train; a swapped read/write assignment would only be
    // caught if the two are distinguishable).
    let mut cfg = mock_v1_1_config(1);
    clearbox_mut(&mut cfg, 0).children.insert(
        "Power_Characterization".into(),
        make_power_characterization("LINEAR", &[("b", 1.0), ("a", 2.0)]),
    );
    cfg.optical_trains[0].light_source.power_characterization =
        Some(machine_config::PowerCharacterization {
            algorithm_type: Some("POLYNOMIAL".to_string()),
            algorithm_equation: None,
            input_type: None,
            units_derived_quantity: None,
            derivation_equation_constants: vec![],
            characterization_points: vec![CalibrationPoint { input_value: 9.0, output_value: 99.0 }],
        });

    let p1 = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&cfg).write(p1.path()).unwrap();
    let mid = read(&p1);
    let p2 = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&mid).write(p2.path()).unwrap();
    let result = read(&p2);

    let cb_pc = power_characterization(&result, 0);
    let ls_pc = result.optical_trains[0].light_source.power_characterization.as_ref().unwrap();
    assert_eq!(text(cb_pc, "Algorithm_Type"), Some("LINEAR"));
    assert_eq!(constant_names(cb_pc), vec!["b", "a"]);
    assert_eq!(point_count(cb_pc), 0);
    assert_eq!(ls_pc.algorithm_type.as_deref(), Some("POLYNOMIAL"));
    assert!(ls_pc.derivation_equation_constants.is_empty());
    assert_eq!(ls_pc.characterization_points[0].input_value, 9.0);
    // Cross-wiring guard: the two instances must not have swapped.
    assert_ne!(text(cb_pc, "Algorithm_Type"), ls_pc.algorithm_type.as_deref());
    // The whole ClearBox tree, grids included, is bit-stable across the double round trip.
    assert_eq!(digest(&cfg), digest(&result));
}

// ---------------------------------------------------------------------------
// test_v1_to_v1_1 (v1.0 fixture written as v1.1)
// ---------------------------------------------------------------------------

#[test]
fn test_v1_to_v1_1() {
    let source = MachineConfigReader::open(REFERENCE_V1_0).unwrap().parse().unwrap();
    let out = write_as(&source, "1.1");
    let migrated = read(&out);
    assert_eq!(migrated.meta.file_version, "1.1");

    // The shimmed tree is already in the v1.1 layout, so it survives unchanged.
    assert_eq!(digest(&source), digest(&migrated));
    let root = &migrated.extensions["ClearBox"];
    assert!(root.attrs.contains_key("Output_Path"));
    assert!(root.attrs.contains_key("Software_Trigger_Delay"));

    for (i, t) in migrated.optical_trains.iter().enumerate() {
        let cb = clearbox(&migrated, i);
        // Addition: no v1.0 source.
        assert!(!cb.attrs.contains_key("Firmware_Version"));
        // v1.0-only attributes ride along as extra per-train attributes.
        assert!(cb.attrs.contains_key("Selected_Camera"));
        // Change 3: derived ClearBox data has real constants, zero points.
        let pc = power_characterization(&migrated, i);
        assert_eq!(point_count(pc), 0);
        assert!(!constant_names(pc).is_empty());
        assert_eq!(
            text(pc, "Algorithm_Type"),
            text(power_characterization(&source, i), "Algorithm_Type")
        );

        // Change 4: derived Light_Source data is the inverse — zero constants, real points.
        let ls_pc = t.light_source.power_characterization.as_ref().unwrap();
        assert!(ls_pc.derivation_equation_constants.is_empty());
        assert!(!ls_pc.characterization_points.is_empty());
    }
}

#[test]
fn test_v1_0_first_train_wins_for_shared_attributes() {
    // File_Version 1.0 stores Output_Path and Software_Trigger_Delay per train; the extension tree
    // holds one value each, taken from the first train. A later train's differing value is not
    // carried (documented in docs/migrations/v1_0_to_v1_1.md).
    let mut cfg = mock_v1_0_config(2);
    clearbox_mut(&mut cfg, 1)
        .attrs
        .insert("Output_Path".into(), Attr::scalar(Scalar::Utf8("/other/".into())));
    clearbox_mut(&mut cfg, 1)
        .attrs
        .insert("Software_Trigger_Delay".into(), Attr::scalar(Scalar::I64(9999)));
    let back = read(&write_as(&cfg, "1.0"));
    let root = &back.extensions["ClearBox"];
    assert_eq!(root.attrs["Output_Path"].as_str(), Some("/recordings/"));
    assert_eq!(root.attrs["Software_Trigger_Delay"].as_i64(), Some(3000));
    assert!(!clearbox(&back, 1).attrs.contains_key("Output_Path"));
}

#[test]
fn test_v1_to_v1_1_unrecognized_algorithm_type_best_effort() {
    // Never a hard error — best-effort, verbatim algorithm_type, positionally-named constants,
    // blank algorithm_equation.
    let mut cfg = mock_v1_1_config(1);
    clearbox_mut(&mut cfg, 0).children.insert(
        "Power_Characterization".into(),
        make_power_characterization("EXPONENTIAL", &[("0", 1.5), ("1", 2.5), ("2", 3.5)]),
    );
    let v1_0 = read(&write_as(&cfg, "1.0"));
    let pc = power_characterization(&v1_0, 0);
    assert_eq!(text(pc, "Algorithm_Type"), Some("EXPONENTIAL"));
    assert!(text(pc, "Algorithm_Equation").is_none());
    assert_eq!(constant_names(pc), vec!["0", "1", "2"]);
    assert_eq!(constant_values(pc), vec![1.5, 2.5, 3.5]);

    let back = read(&write_as(&v1_0, "1.1"));
    let back_pc = power_characterization(&back, 0);
    assert_eq!(text(back_pc, "Algorithm_Type"), Some("EXPONENTIAL"));
    assert_eq!(constant_values(back_pc), vec![1.5, 2.5, 3.5]);
}

#[test]
fn test_v1_to_v1_1_unrecognized_algorithm_type_best_effort_for_light_source() {
    let mut cfg = mock_v1_0_config(1);
    cfg.optical_trains[0].light_source.power_characterization =
        forward_power_characterization_points(Some("QUADRATIC"), Some("1,2,3,4")).unwrap();
    let result = read(&write_as(&cfg, "1.1"));
    let pc = result.optical_trains[0].light_source.power_characterization.as_ref().unwrap();
    assert_eq!(pc.algorithm_type.as_deref(), Some("QUADRATIC"));
    assert!(pc.algorithm_equation.is_none());
    assert_eq!(pc.characterization_points.len(), 2);
}

#[test]
fn test_clearbox_train_without_a_matching_optical_train_is_rejected_for_v1_0() {
    let mut cfg = mock_v1_1_config(1);
    let extra = clearbox(&cfg, 0).clone();
    cfg.extensions.get_mut("ClearBox").unwrap().children.insert("Optical_Train_07".into(), extra);
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    let err = MachineConfigWriter::with_target_version(&cfg, "1.0").write(out.path()).unwrap_err();
    assert!(err.to_string().contains("Optical_Train_07"), "unexpected message: {err}");
}

// ---------------------------------------------------------------------------
// test_v1_1_to_v1 (v1.1 fixture written as v1.0)
// ---------------------------------------------------------------------------

#[test]
fn test_v1_1_to_v1() {
    let source = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    let back = read(&write_as(&source, "1.0"));
    assert_eq!(back.meta.file_version, "1.0");

    let src_pc0 = power_characterization(&source, 0);
    let pc0 = power_characterization(&back, 0);
    assert_eq!(text(pc0, "Algorithm_Type"), text(src_pc0, "Algorithm_Type"));
    assert_eq!(sorted(constant_values(pc0)), sorted(constant_values(src_pc0)));
    let (src_root, root) = (&source.extensions["ClearBox"], &back.extensions["ClearBox"]);
    assert_eq!(root.attrs["Output_Path"], src_root.attrs["Output_Path"]);
    assert_eq!(root.attrs["Software_Trigger_Delay"], src_root.attrs["Software_Trigger_Delay"]);

    let src_ls_pc0 = source.optical_trains[0].light_source.power_characterization.as_ref().unwrap();
    let ls_pc0 = back.optical_trains[0].light_source.power_characterization.as_ref().unwrap();
    assert_eq!(ls_pc0.algorithm_type, src_ls_pc0.algorithm_type);
    let points = |p: &machine_config::PowerCharacterization| -> Vec<(f64, f64)> {
        p.characterization_points.iter().map(|p| (p.input_value, p.output_value)).collect()
    };
    assert_eq!(points(ls_pc0), points(src_ls_pc0));

    // v1.1-only fields: lost on the way down, not restored.
    assert!(!clearbox(&back, 0).attrs.contains_key("Firmware_Version"));
    assert!(!clearbox(&back, 0).attrs.contains_key("Selected_Camera"));
}

#[test]
fn test_v1_1_to_v1_reorders_constants_by_name() {
    // Change 3 backward: Derivation_Equation_Constants rows aren't positionally guaranteed —
    // must re-sort by name before joining as CSV.
    let mut cfg = mock_v1_1_config(1);
    clearbox_mut(&mut cfg, 0).children.insert(
        "Power_Characterization".into(),
        make_power_characterization("LINEAR", &[("a", 2.0), ("b", 1.0)]),
    );
    let back = read(&write_as(&cfg, "1.0"));
    let pc = power_characterization(&back, 0);
    assert_eq!(constant_names(pc), vec!["b", "a"]);
    assert_eq!(constant_values(pc), vec![1.0, 2.0]);
}

#[test]
fn test_v1_1_to_v1_missing_derivation_constants_writes_blank() {
    // Change 3 backward: empty Derivation_Equation_Constants -> blank, not an error.
    let mut cfg = mock_v1_1_config(1);
    clearbox_mut(&mut cfg, 0)
        .children
        .insert("Power_Characterization".into(), make_power_characterization("LINEAR", &[]));
    let back = read(&write_as(&cfg, "1.0"));
    assert!(constants(power_characterization(&back, 0)).is_empty());
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
    let back = read(&write_as(&cfg, "1.0"));
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
    // The v1.0 read path still reads the same on-disk attributes; the ClearBox flat
    // Volts_To_Watts fields surface as Power_Characterization in the extension tree.
    let config = MachineConfigReader::open(REFERENCE_V1_0).unwrap().parse().unwrap();
    assert_eq!(config.meta.file_version, "1.0");
    assert!(!config.optical_trains.is_empty());
    assert_eq!(text(power_characterization(&config, 0), "Algorithm_Type"), Some("LINEAR"));
    assert!(!clearbox(&config, 0).attrs.contains_key("Volts_To_Watts_Algorithm"));
}

// ---------------------------------------------------------------------------
// Round-trip tests — the acceptance criterion stated 2026-09-01, made concrete
// ---------------------------------------------------------------------------

#[test]
fn test_roundtrip_v1_0_to_v1_1_to_v1_0() {
    let source = MachineConfigReader::open(REFERENCE_V1_0).unwrap().parse().unwrap();
    let mid = read(&write_as(&source, "1.1"));
    let back = read(&write_as(&mid, "1.0"));

    assert_eq!(back.meta.file_version, "1.0");
    assert_eq!(digest(&source), digest(&back));
    for (i, t) in back.optical_trains.iter().enumerate() {
        let src_ls = &source.optical_trains[i].light_source;
        let src_ls_pc = src_ls.power_characterization.as_ref().unwrap();
        let ls_pc = t.light_source.power_characterization.as_ref().unwrap();
        assert_eq!(ls_pc.algorithm_type, src_ls_pc.algorithm_type);
        let pts = |p: &machine_config::PowerCharacterization| -> Vec<(f64, f64)> {
            p.characterization_points.iter().map(|p| (p.input_value, p.output_value)).collect()
        };
        assert_eq!(pts(ls_pc), pts(src_ls_pc));
    }
}

#[test]
fn test_roundtrip_v1_1_to_v1_0_to_v1_1() {
    // Mirror direction: from a natively-authored v1.1 file, downgrade then upgrade again. Fields
    // v1.0 can represent survive; fields only v1.1 can hold come back blank — expected,
    // documented richer-to-simpler loss, not a bug.
    let source = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    let mid = read(&write_as(&source, "1.0"));
    let back = read(&write_as(&mid, "1.1"));

    assert_eq!(back.meta.file_version, "1.1");
    for i in 0..back.optical_trains.len() {
        let src_pc = power_characterization(&source, i);
        let pc = power_characterization(&back, i);
        assert_eq!(text(pc, "Algorithm_Type"), text(src_pc, "Algorithm_Type"));
        assert_eq!(sorted(constant_values(pc)), sorted(constant_values(src_pc)));
        // Expected loss: v1.1-only fields have no v1.0 round-trip path.
        assert!(!clearbox(&back, i).attrs.contains_key("Firmware_Version"));
        assert!(text(pc, "Input_Type").is_none());
        assert!(text(pc, "Units_Derived_Quantity").is_none());
        assert_eq!(point_count(pc), 0);
    }
}

// ---------------------------------------------------------------------------
// Writer target_version parameter
// ---------------------------------------------------------------------------

#[test]
fn test_writer_target_version_overrides_meta() {
    let source = MachineConfigReader::open(REFERENCE_V1_0).unwrap().parse().unwrap();
    assert_eq!(source.meta.file_version, "1.0");
    let result = read(&write_as(&source, "1.1"));
    assert_eq!(result.meta.file_version, "1.1");
    // Writer must not mutate its input.
    assert_eq!(source.meta.file_version, "1.0");

    let v1_1_source = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    assert_eq!(v1_1_source.meta.file_version, "1.1");
    let result2 = read(&write_as(&v1_1_source, "1.0"));
    assert_eq!(result2.meta.file_version, "1.0");
    assert_eq!(v1_1_source.meta.file_version, "1.1");
}

#[test]
fn test_writer_defaults_to_meta_file_version() {
    let source = MachineConfigReader::open(REFERENCE_V1_0).unwrap().parse().unwrap();
    let out = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&source).write(out.path()).unwrap(); // no target_version
    assert_eq!(read(&out).meta.file_version, "1.0");
}

// ---------------------------------------------------------------------------
// Dispatcher-level tests — full public API via the real "1.1" registry entry.
// ---------------------------------------------------------------------------

#[test]
fn test_dispatcher_v1_to_v1_1() {
    let source = MachineConfigReader::open(REFERENCE_V1_0).unwrap().parse().unwrap();
    let result = read(&write_as(&source, "1.1"));
    assert_eq!(result.meta.file_version, "1.1");
    assert_eq!(
        result.extensions["ClearBox"].attrs["Output_Path"].as_str(),
        Some("/recordings/")
    );
    assert!(clearbox(&result, 0).children.contains_key("Power_Characterization"));
}

#[test]
fn test_dispatcher_v1_1_to_v1() {
    let v1_1_cfg = MachineConfigReader::open(REFERENCE_V1_1).unwrap().parse().unwrap();
    let result = read(&write_as(&v1_1_cfg, "1.0"));
    assert_eq!(result.meta.file_version, "1.0");
    assert_eq!(text(power_characterization(&result, 0), "Algorithm_Type"), Some("LINEAR"));
}

// ---------------------------------------------------------------------------
// test_adapters_satisfy_protocol
// ---------------------------------------------------------------------------

fn assert_is_reader_adapter(_r: &dyn ReaderAdapter) {}
fn assert_is_writer_adapter(_w: &dyn WriterAdapter) {}

#[test]
fn test_adapters_satisfy_protocol() {
    // Primarily a compile-time fact (proved by the trait impls in
    // capabilities/v1_1/{hdf5,writer}.rs); the assert_is_* functions make it a concrete,
    // runtime-checked test too.
    let cfg = mock_v1_1_config(1);
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    Hdf5WriterV1_1::new(&cfg).write(tmp.path()).unwrap();

    let adapter = Hdf5AdapterV1_1::open(tmp.path()).unwrap();
    assert_is_reader_adapter(&adapter);

    let writer = Hdf5WriterV1_1::new(&cfg);
    assert_is_writer_adapter(&writer);
}

#[allow(dead_code)]
fn _unused(_: EquationConstant) {}
