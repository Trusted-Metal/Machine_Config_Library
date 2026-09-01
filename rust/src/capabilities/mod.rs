//! Stable File_Version model facade API.
//!
//! Peek root `File_Version`, then dispatch to a version adapter. Adapters own
//! on-disk layout and map to stable models.

pub mod errors;
pub mod generated;
pub mod merge;
pub mod result;
pub mod v1_0;
pub mod v1_1;

pub use errors::CapabilityError;
pub use generated::{MachineConfigFile, SetMode};
pub use merge::apply_set_mode;
pub use result::Result;
pub use v1_0::MachineConfigFileV1_0;
pub use v1_1::MachineConfigFileV1_1;

use crate::capabilities::v1_0::hdf5::peek_file_version;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

type OpenFn = fn(&Path) -> Result<Box<dyn MachineConfigFile>, CapabilityError>;
/// Version -> constructor for that version's facade, already type-erased to
/// the common `MachineConfigFile` trait. A real registry
/// (DISPATCH_REGISTRY_PLAN.md): adding a version means adding an entry here,
/// never editing `open_machine_config` itself.
pub type OpenRegistry = HashMap<&'static str, OpenFn>;

fn open_v1_0(path: &Path) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    MachineConfigFileV1_0::open(path).map(|f| Box::new(f) as Box<dyn MachineConfigFile>)
}

fn open_v1_1(path: &Path) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    MachineConfigFileV1_1::open(path).map(|f| Box::new(f) as Box<dyn MachineConfigFile>)
}

static PRODUCTION_OPEN_REGISTRY: LazyLock<OpenRegistry> = LazyLock::new(|| {
    let mut m: OpenRegistry = HashMap::new();
    m.insert("1.0", open_v1_0 as OpenFn);
    m.insert("1.1", open_v1_1 as OpenFn);
    m
});

/// Registry-parameterized so tests can inject a fake entry without touching
/// global state — see DISPATCH_REGISTRY_PLAN.md's shared testing pattern.
pub fn resolve_open(
    version: &str,
    path: &Path,
    registry: &OpenRegistry,
) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    match registry.get(version) {
        Some(f) => f(path),
        None => Err(CapabilityError::UnsupportedVersion(format!(
            "No capability adapter registered for File_Version \"{version}\""
        ))),
    }
}

pub fn open_machine_config(
    path: impl AsRef<Path>,
) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    let path = path.as_ref();
    let version = peek_file_version(path).map_err(|e| CapabilityError::IoError(e.to_string()))?;
    resolve_open(&version, path, &PRODUCTION_OPEN_REGISTRY)
}

type CreateFn = fn(&str) -> Result<Box<dyn MachineConfigFile>, CapabilityError>;
/// Same shape as [`OpenRegistry`], for `create()`.
pub type CreateRegistry = HashMap<&'static str, CreateFn>;

fn create_v1_0(version: &str) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    MachineConfigFileV1_0::create(version).map(|f| Box::new(f) as Box<dyn MachineConfigFile>)
}

fn create_v1_1(version: &str) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    MachineConfigFileV1_1::create(version).map(|f| Box::new(f) as Box<dyn MachineConfigFile>)
}

static PRODUCTION_CREATE_REGISTRY: LazyLock<CreateRegistry> = LazyLock::new(|| {
    let mut m: CreateRegistry = HashMap::new();
    m.insert("1.0", create_v1_0 as CreateFn);
    m.insert("1.1", create_v1_1 as CreateFn);
    m
});

/// Registry-parameterized so tests can inject a fake entry without touching
/// global state — see DISPATCH_REGISTRY_PLAN.md's shared testing pattern.
pub fn resolve_create(
    version: &str,
    registry: &CreateRegistry,
) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    match registry.get(version) {
        Some(f) => f(version),
        None => Err(CapabilityError::UnsupportedVersion(format!(
            "create() unsupported for File_Version \"{version}\""
        ))),
    }
}

pub fn create_machine_config(version: &str) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    resolve_create(version, &PRODUCTION_CREATE_REGISTRY)
}

/// Registered `File_Version` strings this dispatcher understands. Derived
/// from the registry's own keys, not a separately-maintained literal.
pub fn supported_file_versions() -> Vec<&'static str> {
    PRODUCTION_OPEN_REGISTRY.keys().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ClearBox, Collimator, CorrectionData, LightSource, Machine, MachineConfigMeta,
        OpcuaConfig, OpticalTrain, Scanner, ScannerCard,
    };

    const REFERENCE: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config.h5");

    // DISPATCH_REGISTRY_PLAN.md — proves resolve_open()/resolve_create() are
    // genuinely registry-driven. `file_version()` returns a sentinel string;
    // every other method panics if called, since the tests below only ever
    // check identity via that sentinel, never real facade behavior.
    struct FakeMachineConfigFile;
    impl MachineConfigFile for FakeMachineConfigFile {
        fn file_version(&self) -> &str {
            "9.9-test-sentinel"
        }
        fn optical_train_count(&self) -> Result<usize, CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn get_meta(&self) -> Result<MachineConfigMeta, CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn set_meta(&mut self, _model: MachineConfigMeta, _mode: SetMode) -> Result<(), CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn get_machine(&self) -> Result<Machine, CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn set_machine(&mut self, _model: Machine, _mode: SetMode) -> Result<(), CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn get_train(&self, _index: usize) -> Result<OpticalTrain, CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn set_train(
            &mut self,
            _index: usize,
            _model: OpticalTrain,
            _mode: SetMode,
        ) -> Result<(), CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn get_scanner(&self, _index: usize) -> Result<Scanner, CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn set_scanner(
            &mut self,
            _index: usize,
            _model: Scanner,
            _mode: SetMode,
        ) -> Result<(), CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn get_light_source(&self, _index: usize) -> Result<LightSource, CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn set_light_source(
            &mut self,
            _index: usize,
            _model: LightSource,
            _mode: SetMode,
        ) -> Result<(), CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn get_collimator(&self, _index: usize) -> Result<Collimator, CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn set_collimator(
            &mut self,
            _index: usize,
            _model: Collimator,
            _mode: SetMode,
        ) -> Result<(), CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn get_scanner_card(&self, _index: usize) -> Result<ScannerCard, CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn set_scanner_card(
            &mut self,
            _index: usize,
            _model: ScannerCard,
            _mode: SetMode,
        ) -> Result<(), CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn get_clearbox(&self, _index: usize) -> Result<Option<ClearBox>, CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn set_clearbox(
            &mut self,
            _index: usize,
            _model: ClearBox,
            _mode: SetMode,
        ) -> Result<(), CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn get_correction_data(&self, _index: usize) -> Result<CorrectionData, CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn get_inverse_correction_data(
            &self,
            _index: usize,
        ) -> Result<CorrectionData, CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn get_opcua(&self) -> Result<OpcuaConfig, CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn set_opcua(&mut self, _model: OpcuaConfig, _mode: SetMode) -> Result<(), CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn save(&mut self, _path: Option<&Path>) -> Result<(), CapabilityError> {
            unreachable!("not exercised by the registry-dispatch tests")
        }
        fn close(&mut self) {}
    }

    #[test]
    fn open_registry_rejects_unregistered_version() {
        let registry = OpenRegistry::new();
        let result = resolve_open("9.9-nope", Path::new(REFERENCE), &registry);
        assert!(matches!(result, Err(CapabilityError::UnsupportedVersion(_))));
    }

    #[test]
    fn open_registry_dispatches_via_injected_test_file() {
        let mut registry: OpenRegistry = HashMap::new();
        registry.insert("9.9-test", (|_path: &Path| {
            Ok(Box::new(FakeMachineConfigFile) as Box<dyn MachineConfigFile>)
        }) as OpenFn);
        let file = resolve_open("9.9-test", Path::new(REFERENCE), &registry).unwrap();
        assert_eq!(file.file_version(), "9.9-test-sentinel");
    }

    #[test]
    fn create_registry_rejects_unregistered_version() {
        let registry = CreateRegistry::new();
        let result = resolve_create("9.9-nope", &registry);
        assert!(matches!(result, Err(CapabilityError::UnsupportedVersion(_))));
    }

    #[test]
    fn create_registry_dispatches_via_injected_test_file() {
        let mut registry: CreateRegistry = HashMap::new();
        registry.insert("9.9-test", (|_version: &str| {
            Ok(Box::new(FakeMachineConfigFile) as Box<dyn MachineConfigFile>)
        }) as CreateFn);
        let file = resolve_create("9.9-test", &registry).unwrap();
        assert_eq!(file.file_version(), "9.9-test-sentinel");
    }

    #[test]
    fn supported_file_versions_reflects_production_registry() {
        // HashMap iteration order is not guaranteed, so sort both sides
        // before comparing rather than asserting an exact-order Vec.
        let mut versions = supported_file_versions();
        versions.sort();
        assert_eq!(versions, vec!["1.0", "1.1"]);
    }
}
