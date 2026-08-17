from pathlib import Path
from machine_config import MachineConfigReader
from machine_config.capabilities.file_version import UnsupportedFileVersion

"""
AV-03: Simulated future file version (v1.1) file
ID:          AV-03
Title:       v1.0 reader encountering a v1.1 file fails loudly
Category:    adapter-versioning
Layer:       dispatcher
Precondition: docs/validation/fixtures/v1_1_simulated.h5
Action:      Attempt to read/parse with the v1.0 reader
Expected:    UnsupportedFileVersion raised with version='1.1'.
             The Future_Group/ data is NOT silently ignored. No partial read.
Rationale:   A v1.0 reader must not silently truncate a v1.1 file.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = Path(fixtures_dir).parent / "docs" / "validation" / "fixtures" / "v1_1_simulated.h5"
    try:
        MachineConfigReader(fixture).parse()
        return False, "no error raised for File_Version='1.1'"
    except UnsupportedFileVersion as e:
        if e.version == "1.1":
            return True, f"UnsupportedFileVersion raised, version='{e.version}'"
        return False, f"UnsupportedFileVersion raised but version='{e.version}'"
    except Exception as e:
        return False, f"wrong exception: {type(e).__name__}: {e}"
