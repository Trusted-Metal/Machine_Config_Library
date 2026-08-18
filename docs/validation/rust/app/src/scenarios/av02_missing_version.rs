// AV-02: Reader handles absent File_Version attribute predictably
//
// Either outcome (defaults to v1.0 dispatch, or a typed error) is acceptable
// per VALIDATION_PLAN.md — behavior must simply be documented, and must match
// across all five languages. This scenario records what actually happens
// rather than asserting one outcome, mirroring Python's/Node's own AV-02.
use super::common::av_fixture;
use machine_config::MachineConfigReader;
use std::path::Path;

pub fn run(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let fixture = av_fixture(fixtures_dir, "missing_version.h5");
    match MachineConfigReader::open(&fixture).and_then(|r| r.parse()) {
        Ok(cfg) => (
            true,
            format!(
                "missing File_Version dispatches OK, file_version='{}'",
                cfg.meta.file_version
            ),
        ),
        Err(e) => (true, format!("missing File_Version raises {e}")),
    }
}
