// AV-05: Reader handles corrupt required attribute (Build_Plate_X_Dimension) gracefully
use super::common::av_fixture;
use machine_config::{MachineConfigError, MachineConfigReader};
use std::path::Path;

pub fn run(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "corrupt_scalar.h5");
    match MachineConfigReader::open(&fixture).and_then(|r| r.parse()) {
        Ok(_) => (false, "no error raised for corrupt Build_Plate_X_Dimension".to_string()),
        Err(e @ MachineConfigError::Parse(_)) => {
            (true, format!("Parse error raised for corrupt scalar: {e}"))
        }
        Err(e) => (false, format!("unexpected error variant: {e}")),
    }
}
