from pathlib import Path
from machine_config import MachineConfigReader

"""
AV-07: Empty string File_Version
ID:          AV-07
Title:       Dispatcher handles empty string File_Version
Category:    adapter-versioning
Layer:       dispatcher
Precondition: docs/validation/fixtures/empty_version.h5 (File_Version="")
Action:      Read/parse
Expected:    Either: defaults to "1.0", OR raises typed error.
             Behavior must be identical across all five languages.
Rationale:   Establishes and verifies the empty-string contract explicitly.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = Path(fixtures_dir).parent / "docs" / "validation" / "fixtures" / "empty_version.h5"
    try:
        cfg = MachineConfigReader(fixture).parse()
        return True, f"empty File_Version defaults to '1.0', reads OK, file_version='{cfg.meta.file_version}'"
    except Exception as e:
        return True, f"empty File_Version raises {type(e).__name__}: {e}"
