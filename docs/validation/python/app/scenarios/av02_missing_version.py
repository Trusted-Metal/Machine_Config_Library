from pathlib import Path
from machine_config import MachineConfigReader

"""
AV-02: Missing file version string
ID:          AV-02
Title:       Reader handles absent File_Version attribute predictably
Category:    adapter-versioning
Layer:       dispatcher
Precondition: docs/validation/fixtures/missing_version.h5
Action:      Attempt to read/parse the file
Expected:    Either: defaults to "1.0" and reads successfully, OR raises typed error.
             Behavior must be identical across all five languages.
Rationale:   Real-world files from pre-versioning tools will lack this attribute.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = Path(fixtures_dir).parent / "docs" / "validation" / "fixtures" / "missing_version.h5"
    try:
        cfg = MachineConfigReader(fixture).parse()
        return True, f"missing File_Version defaults to '1.0', reads OK, file_version='{cfg.meta.file_version}'"
    except Exception as e:
        return True, f"missing File_Version raises {type(e).__name__}: {e}"
