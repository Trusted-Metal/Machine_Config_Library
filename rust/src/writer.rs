//! Public HDF5 writer: dispatch to a File_Version adapter.
//!
//! On-disk group paths and HDF5 attribute names live in the matching adapter.

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use crate::capabilities::v1_0::writer::Hdf5WriterV1_0;
use crate::capabilities::v1_1::writer::Hdf5WriterV1_1;
use crate::error::{MachineConfigError, Result};
use crate::models::MachineConfig;

/// Version-agnostic writer adapter — mirrors `Hdf5WriterV1_0`'s public
/// surface. Non-generic (`path: &Path`, not `impl AsRef<Path>`) so the trait
/// is object-safe; `MachineConfigWriter::write` stays generic for callers and
/// converts once via `path.as_ref()` before reaching the trait object.
pub trait WriterAdapter {
    fn write(&self, path: &Path) -> Result<()>;
}

/// Constructs the `WriterAdapter` for one version from a borrowed
/// `MachineConfig`, tying the adapter's lifetime to the config's borrow —
/// mirrors `Hdf5WriterV1_0<'a>`'s own lifetime-parameterized shape.
type ConstructFn = for<'a> fn(&'a MachineConfig) -> Box<dyn WriterAdapter + 'a>;
/// Version -> constructor for that version's `WriterAdapter`. A real registry
/// (DISPATCH_REGISTRY_PLAN.md): adding a version means adding an entry here,
/// never editing `MachineConfigWriter` itself.
pub type WriterRegistry = HashMap<&'static str, ConstructFn>;

fn new_v1_0_writer(config: &MachineConfig) -> Box<dyn WriterAdapter + '_> {
    Box::new(Hdf5WriterV1_0::new(config))
}

fn new_v1_1_writer(config: &MachineConfig) -> Box<dyn WriterAdapter + '_> {
    Box::new(Hdf5WriterV1_1::new(config))
}

static PRODUCTION_WRITER_REGISTRY: LazyLock<WriterRegistry> = LazyLock::new(|| {
    let mut m: WriterRegistry = HashMap::new();
    m.insert("1.0", new_v1_0_writer as ConstructFn);
    m.insert("1.1", new_v1_1_writer as ConstructFn);
    m
});

/// Registry-parameterized so tests can inject a fake entry without touching
/// global state — see DISPATCH_REGISTRY_PLAN.md's shared testing pattern.
/// Construction itself never fails (mirrors `Hdf5WriterV1_0::new`, infallible)
/// — the `Result` here covers only the unregistered-version case.
pub fn resolve_writer<'a>(
    version: &str,
    config: &'a MachineConfig,
    registry: &WriterRegistry,
) -> Result<Box<dyn WriterAdapter + 'a>> {
    match registry.get(version) {
        Some(f) => Ok(f(config)),
        None => Err(MachineConfigError::UnsupportedVersion(version.to_string())),
    }
}

/// Serialises a [`MachineConfig`] to a machine-config HDF5 file.
///
/// Peeks `File_Version` on the model and routes to that version's adapter,
/// unless [`Self::with_target_version`] was used to override it.
pub struct MachineConfigWriter<'a> {
    config: &'a MachineConfig,
    target_version: Option<String>,
}

impl<'a> MachineConfigWriter<'a> {
    pub fn new(config: &'a MachineConfig) -> Self {
        Self { config, target_version: None }
    }

    /// Like [`Self::new`], but writes as `target_version` regardless of
    /// `config.meta.file_version` — lets a caller upgrade/downgrade without
    /// mutating the model just to express intent (e.g. reading a v1.0 file
    /// and writing it as v1.1 no longer requires setting
    /// `config.meta.file_version = "1.1"` first). Never mutates `config`
    /// itself; only the on-disk `File_Version` changes.
    pub fn with_target_version(config: &'a MachineConfig, target_version: impl Into<String>) -> Self {
        Self { config, target_version: Some(target_version.into()) }
    }

    fn resolved_file_version(&self) -> &str {
        match self.target_version.as_deref() {
            Some(v) => v,
            None => {
                let fv = self.config.meta.file_version.trim();
                if fv.is_empty() { "1.0" } else { fv }
            }
        }
    }

    /// Writes the config to `path`, creating or overwriting the file.
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let fv = self.resolved_file_version();
        let current = {
            let c = self.config.meta.file_version.trim();
            if c.is_empty() { "1.0" } else { c }
        };
        // Every adapter stamps config.meta.file_version verbatim as the
        // on-disk File_Version attribute — if target_version overrides the
        // adapter choice, the config handed to the adapter must reflect that
        // too, or the file would claim the wrong version on disk. A clone,
        // not a mutation of the caller's config, and only made when actually
        // needed (the common case — no override — never pays for it).
        if fv == current {
            let backend = resolve_writer(fv, self.config, &PRODUCTION_WRITER_REGISTRY)?;
            backend.write(path.as_ref())
        } else {
            let mut corrected = self.config.clone();
            corrected.meta.file_version = fv.to_string();
            let backend = resolve_writer(fv, &corrected, &PRODUCTION_WRITER_REGISTRY)?;
            backend.write(path.as_ref())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::MachineConfigReader;
    use tempfile::NamedTempFile;

    const REFERENCE: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config.h5");
    const REFERENCE_OPCUA: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config_opcua.h5");
    const SYNTHETIC: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/synthetic_2laser.h5");
    const REFERENCE_SENSORS: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/reference_config_synchronous_sensors.h5"
    );

    fn roundtrip(src: &str) -> crate::models::MachineConfig {
        let original = MachineConfigReader::open(src).unwrap().parse().unwrap();
        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        MachineConfigWriter::new(&original).write(tmp.path()).unwrap();
        MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap()
    }

    #[test]
    fn roundtrip_meta_fields() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        assert_eq!(orig.meta.machine_name, rt.meta.machine_name);
        assert_eq!(orig.meta.manufacturer, rt.meta.manufacturer);
        assert_eq!(orig.meta.configuration_hash, rt.meta.configuration_hash);
        assert_eq!(orig.meta.file_version, rt.meta.file_version);
        assert_eq!(orig.meta.export_date, rt.meta.export_date);
    }

    #[test]
    fn roundtrip_meta_extra_preserved() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        assert_eq!(orig.meta.extra, rt.meta.extra);
    }

    #[test]
    fn roundtrip_machine_fields() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        assert_eq!(orig.machine.build_plate_x, rt.machine.build_plate_x);
        assert_eq!(orig.machine.build_plate_x_unit, rt.machine.build_plate_x_unit);
        assert_eq!(orig.machine.gas_flow_direction, rt.machine.gas_flow_direction);
        assert_eq!(orig.machine.recoat_direction, rt.machine.recoat_direction);
        assert_eq!(orig.machine.recoater_blade_type, rt.machine.recoater_blade_type);
    }

    #[test]
    fn roundtrip_train_count() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        assert_eq!(orig.optical_trains.len(), rt.optical_trains.len());
    }

    #[test]
    fn roundtrip_scanner_offsets() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        assert_eq!(
            orig.optical_trains[0].scanner.scan_head_offset_x,
            rt.optical_trains[0].scanner.scan_head_offset_x,
        );
        assert_eq!(
            orig.optical_trains[0].scanner.working_distance,
            rt.optical_trains[0].scanner.working_distance,
        );
    }

    #[test]
    fn roundtrip_axis_configuration_3d() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        let t1_orig = &orig.optical_trains[0];
        let t1_rt = &rt.optical_trains[0];
        assert_eq!(t1_orig.scanner.axis_configuration, t1_rt.scanner.axis_configuration);
        assert!(t1_rt.scanner.z_axis.is_some());
        assert!(t1_rt.scanner.focus.is_none());
    }

    #[test]
    fn roundtrip_thermal_lensing() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        assert_eq!(
            orig.optical_trains[0].thermal_lensing_passed,
            rt.optical_trains[0].thermal_lensing_passed,
        );
        assert_eq!(
            orig.optical_trains[1].thermal_lensing_passed,
            rt.optical_trains[1].thermal_lensing_passed,
        );
    }

    #[test]
    fn roundtrip_clearbox_scalar_fields() {
        // reference_config_synchronous_sensors.h5, not reference_config.h5,
        // which deliberately has no sensors — the two fixtures are
        // byte-identical outside the new group, so every pre-existing
        // assertion below still holds.
        let orig = MachineConfigReader::open(REFERENCE_SENSORS).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE_SENSORS);
        let cb_orig = orig.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
        let cb_rt = rt.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
        assert_eq!(cb_orig.ip_address, cb_rt.ip_address);
        assert_eq!(cb_orig.data_port, cb_rt.data_port);
        assert_eq!(cb_orig.show_console, cb_rt.show_console);
        assert_eq!(cb_orig.correction_grid_domain_shape, cb_rt.correction_grid_domain_shape);

        let sensor_orig = &cb_orig.synchronous_sensors["Oxygen Sensor"];
        let sensor_rt = &cb_rt.synchronous_sensors["Oxygen Sensor"];
        assert_eq!(sensor_orig.sensor_name, sensor_rt.sensor_name);
        assert_eq!(
            sensor_orig.derivation_equation_constants,
            sensor_rt.derivation_equation_constants
        );
        assert_eq!(sensor_orig.calibration_points, sensor_rt.calibration_points);
    }

    #[test]
    fn roundtrip_sfcf_metadata() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        let sfcf_orig = orig.optical_trains[0].scan_field_correction_file.as_ref().unwrap();
        let sfcf_rt = rt.optical_trains[0].scan_field_correction_file.as_ref().unwrap();
        assert_eq!(sfcf_orig.document_name, sfcf_rt.document_name);
        assert_eq!(sfcf_orig.file_size, sfcf_rt.file_size);
        assert_eq!(sfcf_orig.document_id, sfcf_rt.document_id);
    }

    #[test]
    fn roundtrip_correction_data_sha256() {
        use crate::models::CorrectionData;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn cd_hash(cd: &CorrectionData) -> u64 {
            let mut h = DefaultHasher::new();
            for v in &cd.data {
                v.to_bits().hash(&mut h);
            }
            h.finish()
        }

        let reader = MachineConfigReader::open(REFERENCE).unwrap();
        let original_corr = reader.get_correction_data(0).unwrap();
        let original_inv = reader.get_inverse_correction_data(0).unwrap();

        let config = reader.parse_with_binary().unwrap();
        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        MachineConfigWriter::new(&config).write(tmp.path()).unwrap();
        let rt_reader = MachineConfigReader::open(tmp.path()).unwrap();
        let rt_corr = rt_reader.get_correction_data(0).unwrap();
        let rt_inv = rt_reader.get_inverse_correction_data(0).unwrap();

        assert_eq!(
            cd_hash(&original_corr),
            cd_hash(&rt_corr),
            "Correction_Data hash changed across write roundtrip"
        );
        assert_eq!(
            cd_hash(&original_inv),
            cd_hash(&rt_inv),
            "Inverse_Correction_Data hash changed across write roundtrip"
        );
    }

    #[test]
    fn roundtrip_opcua_fields() {
        let orig = MachineConfigReader::open(REFERENCE_OPCUA).unwrap().parse().unwrap();
        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        MachineConfigWriter::new(&orig).write(tmp.path()).unwrap();
        let rt = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();

        let orig_opcua = orig.opcua.as_ref().unwrap();
        let rt_opcua = rt.opcua.as_ref().unwrap();
        assert_eq!(orig_opcua.client.server_url, rt_opcua.client.server_url);
        assert_eq!(orig_opcua.client.bfs_max_depth, rt_opcua.client.bfs_max_depth);
        assert_eq!(orig_opcua.client.extra, rt_opcua.client.extra);
        assert_eq!(orig_opcua.pipe.buffer_size, rt_opcua.pipe.buffer_size);
        assert_eq!(orig_opcua.triggers_enabled, rt_opcua.triggers_enabled);
        assert_eq!(orig_opcua.triggers.len(), rt_opcua.triggers.len());

        // Spot-check a representative subset of the newly-promoted fields
        // through the public MachineConfigWriter facade specifically — the
        // exhaustive 22-field check lives on the capabilities-layer copy of
        // this test (capabilities::v1_0::writer::tests::roundtrip_opcua_fields).
        assert_eq!(orig_opcua.client.machine_profile, rt_opcua.client.machine_profile);
        assert_eq!(orig_opcua.client.root_node, rt_opcua.client.root_node);
        assert_eq!(orig_opcua.pipe.configure_client, rt_opcua.pipe.configure_client);
        assert_eq!(orig_opcua.pipe.pipe_name, rt_opcua.pipe.pipe_name);
        assert_eq!(orig_opcua.trigger_stop_ceiling_layers, rt_opcua.trigger_stop_ceiling_layers);
        assert_eq!(orig_opcua.trigger_stop_ceiling_layers, Some(3));

        let orig_t = &orig_opcua.triggers["Laser Emission Interlock"];
        let rt_t = &rt_opcua.triggers["Laser Emission Interlock"];
        assert_eq!(orig_t.signal, rt_t.signal);
        assert_eq!(orig_t.rule_enabled, rt_t.rule_enabled);
        assert_eq!(orig_t.extra, rt_t.extra);
        assert_eq!(orig_t.event, rt_t.event);
        assert_eq!(orig_t.trigger_label, rt_t.trigger_label);
    }

    #[test]
    fn roundtrip_synthetic_2laser() {
        let orig = MachineConfigReader::open(SYNTHETIC).unwrap().parse().unwrap();
        let rt = roundtrip(SYNTHETIC);
        assert_eq!(orig.optical_trains.len(), rt.optical_trains.len());
        assert_eq!(orig.meta.machine_name, rt.meta.machine_name);
        assert_eq!(
            orig.optical_trains[0].scanner.working_distance,
            rt.optical_trains[0].scanner.working_distance,
        );
    }

    #[test]
    fn rejects_unknown_file_version() {
        let mut cfg = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        cfg.meta.file_version = "2.0".into();
        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        let err = MachineConfigWriter::new(&cfg).write(tmp.path()).unwrap_err();
        assert!(matches!(
            err,
            MachineConfigError::UnsupportedVersion(ref v) if v == "2.0"
        ));
    }

    // DISPATCH_REGISTRY_PLAN.md — proves resolve_writer() is genuinely
    // registry-driven, not a relocated hardcoded check. The fake panics if
    // ever actually asked to write — reaching the assertion without touching
    // disk already proves dispatch routed to the fake, not the real "1.0"
    // adapter.
    struct FakeWriterAdapter;
    impl WriterAdapter for FakeWriterAdapter {
        fn write(&self, _path: &Path) -> Result<()> {
            unreachable!("not exercised by the registry-dispatch test")
        }
    }

    #[test]
    fn writer_registry_rejects_unregistered_version() {
        let registry = WriterRegistry::new();
        let cfg = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let result = resolve_writer("9.9-nope", &cfg, &registry);
        assert!(matches!(
            result,
            Err(MachineConfigError::UnsupportedVersion(ref v)) if v == "9.9-nope"
        ));
    }

    #[test]
    fn writer_registry_dispatches_via_injected_test_adapter() {
        let mut registry: WriterRegistry = HashMap::new();
        registry.insert("9.9-test", (|_cfg: &MachineConfig| {
            Box::new(FakeWriterAdapter) as Box<dyn WriterAdapter>
        }) as ConstructFn);
        let cfg = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let adapter = resolve_writer("9.9-test", &cfg, &registry).unwrap();
        drop(adapter);
    }
}
