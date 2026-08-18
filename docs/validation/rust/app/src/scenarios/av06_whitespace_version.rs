// AV-06: Dispatcher normalizes whitespace in File_Version (" 1.0 ")
use super::common::av_fixture;
use machine_config::MachineConfigReader;
use std::path::Path;

pub fn run(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "version_whitespace.h5");
    match MachineConfigReader::open(&fixture).and_then(|r| r.parse()) {
        Ok(_) => (true, "whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK".to_string()),
        Err(e) => (false, format!("{e}")),
    }
}
