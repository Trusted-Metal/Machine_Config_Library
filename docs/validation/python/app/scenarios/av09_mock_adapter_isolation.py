import sys
from pathlib import Path
from machine_config import MachineConfigReader

# mock adapter lives in the test suite, not the installed wheel
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent.parent.parent / "python" / "tests"))
from test_adapter_migration import MockV1_1Reader, MockV1_1Writer  # noqa: E402

"""
AV-09: Mock v1.1 adapter — adding a new adapter leaves the v1.0 adapter untouched
ID:          AV-09
Title:       Adding a mock v1.1 adapter leaves the v1.0 adapter and all existing behaviour unchanged
Category:    adapter-versioning
Layer:       adapter isolation
Precondition: fixtures/reference_config.h5
Action:      Import MockV1_1Reader and MockV1_1Writer alongside the real v1.0 reader.
             Read the reference fixture with the real v1.0 reader and verify correct output.
Expected:    v1.0 reader still returns correct values — importing the mock v1.1 adapter
             has no side-effect on the v1.0 adapter's behaviour.
Rationale:   The adapter pattern's primary promise is that adding a new version is
             purely additive — no existing adapter code is modified.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    path = Path(fixtures_dir) / "reference_config.h5"
    cfg = MachineConfigReader(path).parse()

    if cfg.meta.machine_name != "TM-LPBF-02: AconityMIDI+_OG":
        return False, f"v1.0 adapter broken after importing mock v1.1: machine_name='{cfg.meta.machine_name}'"
    if cfg.meta.file_version.strip() != "1.0":
        return False, f"v1.0 adapter returned wrong file_version: '{cfg.meta.file_version}'"
    if len(cfg.optical_trains) != 2:
        return False, f"v1.0 adapter returned wrong train count: {len(cfg.optical_trains)}"

    return True, "v1.0 adapter unaffected by mock v1.1 adapter import; all fields correct"
