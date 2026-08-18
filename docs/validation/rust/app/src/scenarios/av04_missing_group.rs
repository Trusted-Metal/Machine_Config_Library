// AV-04: Reader returns typed error when required group (Machine/) is absent
use super::common::av_fixture;
use machine_config::MachineConfigReader;
use std::path::Path;

pub fn run(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "missing_machine_group.h5");
    match MachineConfigReader::open(&fixture).and_then(|r| r.parse()) {
        Ok(_) => (false, "no error raised for missing Machine/ group".to_string()),
        Err(e) => (true, format!("error raised for missing Machine/ group: {e}")),
    }
}
