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

use crate::capabilities::v1_0::hdf5::peek_file_version_source;
use crate::source::FileSource;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

type OpenFn = fn(&Path) -> Result<Box<dyn MachineConfigFile>, CapabilityError>;
/// Version -> constructor for that version's facade, already type-erased to
/// the common `MachineConfigFile` trait. A real registry
/// (DISPATCH_REGISTRY_PLAN.md): adding a version means adding an entry here,
/// never editing `open_machine_config` itself.
pub type OpenRegistry = HashMap<&'static str, OpenFn>;

type SourceOpenFn = fn(&FileSource) -> Result<Box<dyn MachineConfigFile>, CapabilityError>;
/// Like [`OpenRegistry`], for constructors that take a [`FileSource`] (a path, or a path plus a
/// FAPL hook such as an MCF stored inside an LBL container). This is the registry the production
/// open_machine_config* functions dispatch through: adding a version means adding an entry here.
pub type SourceOpenRegistry = HashMap<&'static str, SourceOpenFn>;

fn open_v1_0(source: &FileSource) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    MachineConfigFileV1_0::open_source(source.clone()).map(|f| Box::new(f) as Box<dyn MachineConfigFile>)
}

fn open_v1_1(source: &FileSource) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    MachineConfigFileV1_1::open_source(source.clone()).map(|f| Box::new(f) as Box<dyn MachineConfigFile>)
}

static PRODUCTION_OPEN_REGISTRY: LazyLock<SourceOpenRegistry> = LazyLock::new(|| {
    let mut m: SourceOpenRegistry = HashMap::new();
    m.insert("1.0", open_v1_0 as SourceOpenFn);
    m.insert("1.1", open_v1_1 as SourceOpenFn);
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

/// [`resolve_open`] for a [`SourceOpenRegistry`].
pub fn resolve_open_source(
    version: &str,
    source: &FileSource,
    registry: &SourceOpenRegistry,
) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    match registry.get(version) {
        Some(f) => f(source),
        None => Err(CapabilityError::UnsupportedVersion(format!(
            "No capability adapter registered for File_Version \"{version}\""
        ))),
    }
}

pub fn open_machine_config(
    path: impl AsRef<Path>,
) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    open_machine_config_source(FileSource::new(path))
}

/// Like [`open_machine_config`], for a path or a FAPL-hook source (e.g. an MCF stored inside an
/// LBL container, see [`crate::source`]). Sessions opened through a hook have no default save path.
pub fn open_machine_config_source(
    source: impl Into<FileSource>,
) -> Result<Box<dyn MachineConfigFile>, CapabilityError> {
    let source = source.into();
    let version =
        peek_file_version_source(&source).map_err(|e| CapabilityError::IoError(e.to_string()))?;
    resolve_open_source(&version, &source, &PRODUCTION_OPEN_REGISTRY)
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
        Collimator, LightSource, Machine, MachineConfigMeta, OpticalTrain, Scanner, ScannerCard,
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
        fn is_valid(&self) -> bool {
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
    fn source_open_registry_dispatches_via_injected_test_file() {
        let mut registry: SourceOpenRegistry = HashMap::new();
        registry.insert("9.9-test", (|_source: &FileSource| {
            Ok(Box::new(FakeMachineConfigFile) as Box<dyn MachineConfigFile>)
        }) as SourceOpenFn);
        let source = FileSource::new(REFERENCE);
        let file = resolve_open_source("9.9-test", &source, &registry).unwrap();
        assert_eq!(file.file_version(), "9.9-test-sentinel");
        let missing = resolve_open_source("9.9-nope", &source, &registry);
        assert!(matches!(missing, Err(CapabilityError::UnsupportedVersion(_))));
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
