from pathlib import Path
from machine_config import MachineConfigReader
from machine_config.capabilities.file_version import UnsupportedFileVersion

"""
AV-01: Unknown file version string
ID:          AV-01
Title:       Reader rejects unknown File_Version with typed error
Category:    adapter-versioning
Layer:       dispatcher
Precondition: docs/validation/fixtures/v2_0_unknown.h5
Action:      Attempt to read/parse the file
Expected:    UnsupportedFileVersion raised with version='2.0' on the error object.
             No panic. No silent default to v1.0.
Rationale:   Silent fallback to a wrong adapter is the most dangerous versioning failure.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = Path(fixtures_dir).parent / "docs" / "validation" / "fixtures" / "v2_0_unknown.h5"
    try:
        MachineConfigReader(fixture).parse()
        return False, "no error raised for File_Version='2.0'"
    except UnsupportedFileVersion as e:
        if e.version == "2.0":
            return True, f"UnsupportedFileVersion raised, version='{e.version}'"
        return False, f"UnsupportedFileVersion raised but version='{e.version}'"
    except Exception as e:
        return False, f"wrong exception: {type(e).__name__}: {e}"
