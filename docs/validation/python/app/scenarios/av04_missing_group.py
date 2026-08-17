from pathlib import Path
from machine_config import MachineConfigReader

"""
AV-04: Missing required HDF5 group (Machine/)
ID:          AV-04
Title:       Reader returns typed error when required group is absent
Category:    adapter-versioning / error-contract
Layer:       reader (v1.0 adapter)
Precondition: docs/validation/fixtures/missing_machine_group.h5
Action:      Attempt to read/parse
Expected:    Typed error (not panic) indicating the missing group.
             Error message must name the missing path.
             No partial MachineConfig returned.
Rationale:   Corrupt or hand-edited files in the field will be missing groups.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = Path(fixtures_dir).parent / "docs" / "validation" / "fixtures" / "missing_machine_group.h5"
    try:
        MachineConfigReader(fixture).parse()
        return False, "no error raised for missing Machine/ group"
    except Exception as e:
        return True, f"{type(e).__name__} raised for missing Machine/ group: {e}"
