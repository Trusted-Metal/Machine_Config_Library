//! Stable File_Version model facade API.

pub mod errors;
pub mod generated;
pub mod merge;
pub mod result;
pub mod v1_0;

pub use errors::CapabilityError;
pub use generated::SetMode;
pub use merge::apply_set_mode;
pub use result::Result;
pub use v1_0::MachineConfigFileV10;

use crate::reader::MachineConfigReader;
use std::path::Path;

pub fn open_machine_config(path: impl AsRef<Path>) -> Result<MachineConfigFileV10, CapabilityError> {
    let path = path.as_ref();
    let reader =
        MachineConfigReader::open(path).map_err(|e| CapabilityError::IoError(e.to_string()))?;
    let config = reader
        .parse()
        .map_err(|e| CapabilityError::IoError(e.to_string()))?;
    let version = {
        let v = config.meta.file_version.trim();
        if v.is_empty() { "1.0" } else { v }
    };
    match version {
        "1.0" => MachineConfigFileV10::open(path),
        other => Err(CapabilityError::UnsupportedVersion(format!(
            "No capability adapter registered for File_Version \"{other}\""
        ))),
    }
}

pub fn create_machine_config(version: &str) -> Result<MachineConfigFileV10, CapabilityError> {
    MachineConfigFileV10::create(version)
}

pub fn supported_file_versions() -> &'static [&'static str] {
    &["1.0"]
}
