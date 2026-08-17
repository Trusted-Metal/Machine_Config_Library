from pathlib import Path
from machine_config import MachineConfigReader

"""
AV-05: Valid version, corrupt required scalar field (Build_Plate_X_Dimension)
ID:          AV-05
Title:       Reader handles corrupt required attribute gracefully
Category:    error-contract
Layer:       reader (v1.0 adapter)
Precondition: docs/validation/fixtures/corrupt_scalar.h5
Action:      Attempt to read/parse
Expected:    Typed error returned, not a panic.
             The error identifies the field and path.
Rationale:   Attribute type coercion is a real failure mode when files are
             written by non-library tools.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    fixture = Path(fixtures_dir).parent / "docs" / "validation" / "fixtures" / "corrupt_scalar.h5"
    try:
        MachineConfigReader(fixture).parse()
        return False, "no error raised for corrupt Build_Plate_X_Dimension"
    except (ValueError, TypeError) as e:
        return True, f"{type(e).__name__} raised for corrupt scalar: {e}"
    except Exception as e:
        return False, f"unexpected exception type {type(e).__name__}: {e}"
