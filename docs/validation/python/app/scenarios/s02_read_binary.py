import hashlib
from pathlib import Path
import numpy as np
from machine_config import MachineConfigReader

"""
S-02: Read reference fixture, correction data
ID:          S-02
Title:       Read correction grids and verify shape and finite values
Category:    happy-path
Layer:       reader
Precondition: fixtures/reference_config.h5
Action:      Parse with includeBinary=true, access correction_data and
             inverse_correction_data for train 0
Expected:    correction_data shape: [257, 257, 2]
             inverse_correction_data shape: [257, 257, 2]
             Both contain at least one finite (non-NaN) value
             Forward and inverse arrays differ (not byte-identical)
             SHA-256 hash of correction_data bytes matches reference value
             (record the hash in results.md on first run)
Rationale:   Binary data round-trips are the highest-risk correctness area.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    reader = MachineConfigReader(Path(fixtures_dir) / "reference_config.h5")
    cd = reader.get_correction_data(0)
    icd = reader.get_inverse_correction_data(0)

    if cd.shape != (257, 257, 2):
        return False, f"correction_data shape: {cd.shape}"
    if icd.shape != (257, 257, 2):
        return False, f"inverse_correction_data shape: {icd.shape}"
    if not np.any(np.isfinite(cd)):
        return False, "correction_data: no finite values"
    if not np.any(np.isfinite(icd)):
        return False, "inverse_correction_data: no finite values"
    if np.array_equal(cd, icd, equal_nan=True):
        return False, "correction_data and inverse_correction_data are identical"

    cd_hash = hashlib.sha256(cd.tobytes()).hexdigest()
    return True, f"shapes OK, finite values OK, forward≠inverse, correction_data SHA-256={cd_hash}"
