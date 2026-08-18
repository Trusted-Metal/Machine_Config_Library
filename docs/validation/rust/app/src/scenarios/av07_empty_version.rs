// AV-07: Dispatcher handles empty string File_Version
//
// Either outcome is acceptable per VALIDATION_PLAN.md — this records the
// actual behavior rather than asserting one, mirroring Python's/Node's AV-07.
use super::common::av_fixture;
use machine_config::MachineConfigReader;
use std::path::Path;

pub fn run(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "empty_version.h5");
    match MachineConfigReader::open(&fixture).and_then(|r| r.parse()) {
        Ok(cfg) => (
            true,
            format!(
                "empty File_Version dispatches OK, file_version='{}'",
                cfg.meta.file_version
            ),
        ),
        Err(e) => (true, format!("empty File_Version raises {e}")),
    }
}
