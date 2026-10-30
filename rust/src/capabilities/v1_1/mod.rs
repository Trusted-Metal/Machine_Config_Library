//! File_Version 1.1 capability adapter.
//!
//! Deliberately independent of `capabilities::v1_0` — see each submodule's
//! doc comment for the specific independence rule it follows. Enforced by
//! `tests/version_adapter_isolation_test.rs`.
//!
//! No migration functions here — Phase 2 (V1_1_IMPLEMENTATION_PLAN.md)
//! removed migrate_v1_to_v1_1/migrate_v1_1_to_v1. Upgrade/downgrade is now
//! just parse()/write(): each writer writes its own native fields, falling
//! back to crate::power_characterization's shape-conversion functions only
//! when its own native field is absent.

pub mod file;
pub mod hdf5;
pub mod layout;
pub mod writer;

pub use file::MachineConfigFileV1_1;
pub use hdf5::Hdf5AdapterV1_1;
pub use writer::Hdf5WriterV1_1;
