from pathlib import Path
from machine_config import MachineConfigReader

"""
AV-06: Version string whitespace variants
ID:          AV-06
Title:       Dispatcher normalizes whitespace in File_Version
Category:    adapter-versioning
Layer:       dispatcher
Precondition: docs/validation/fixtures/version_whitespace.h5 (File_Version=" 1.0 ")
Action:      Read/parse
Expected:    Successfully dispatches to v1.0 adapter and reads correctly.
Rationale:   Real files from legacy tools have been observed with whitespace in
             version strings.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = Path(fixtures_dir).parent / "docs" / "validation" / "fixtures" / "version_whitespace.h5"
    try:
        MachineConfigReader(fixture).parse()
        return True, "whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK"
    except Exception as e:
        return False, f"{type(e).__name__}: {e}"
