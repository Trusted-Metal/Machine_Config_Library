//! Stable File_Version model facade API.
//!
//! Peek root `File_Version`, then dispatch to a version adapter. Adapters own
//! on-disk layout and map to stable models.

pub mod errors;
pub mod generated;
pub mod merge;
pub mod result;
pub mod v1_0;

pub use errors::CapabilityError;
pub use generated::SetMode;
pub use merge::apply_set_mode;
pub use result::Result;
pub use v1_0::MachineConfigFileV1_0;

use crate::capabilities::v1_0::hdf5::peek_file_version;
use std::path::Path;

pub fn open_machine_config(path: impl AsRef<Path>) -> Result<MachineConfigFileV1_0, CapabilityError> {
    let path = path.as_ref();
    let version = peek_file_version(path).map_err(|e| CapabilityError::IoError(e.to_string()))?;
    match version.as_str() {
        "1.0" => MachineConfigFileV1_0::open(path),
        other => Err(CapabilityError::UnsupportedVersion(format!(
            "No capability adapter registered for File_Version \"{other}\""
        ))),
    }
}

pub fn create_machine_config(version: &str) -> Result<MachineConfigFileV1_0, CapabilityError> {
    MachineConfigFileV1_0::create(version)
}

pub fn supported_file_versions() -> &'static [&'static str] {
    &["1.0"]
}
