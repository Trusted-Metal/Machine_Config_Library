// AV-01: Reader rejects unknown File_Version with typed error
use super::common::av_fixture;
use machine_config::{MachineConfigError, MachineConfigReader};
use std::path::Path;

pub fn run(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "v2_0_unknown.h5");
    match MachineConfigReader::open(&fixture) {
        Ok(_) => (false, "no error raised for File_Version='2.0'".to_string()),
        Err(MachineConfigError::UnsupportedVersion(v)) if v == "2.0" => {
            (true, format!("UnsupportedVersion raised, version='{v}'"))
        }
        Err(MachineConfigError::UnsupportedVersion(v)) => {
            (false, format!("UnsupportedVersion raised but version='{v}'"))
        }
        Err(e) => (false, format!("wrong error variant: {e}")),
    }
}
