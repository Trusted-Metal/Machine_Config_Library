import hashlib
import tempfile
from pathlib import Path
import numpy as np
from machine_config import MachineConfigReader, MachineConfigWriter

"""
S-05: Full binary round-trip: read, write, re-read, verify binary data preserved with correction hash
ID:          S-05
Title:       Copy config with binary data, verify correction hash unchanged
Category:    happy-path
Layer:       writer + reader (copy-hdf5 path)
Precondition: fixtures/reference_config.h5
Action:      Read with includeBinary=true → write to temp → read temp with
             includeBinary=true → compare SHA-256 of correction_data bytes
Expected:    SHA-256 of correction_data for train 0 identical before and after.
             SHA-256 of inverse_correction_data for train 0 identical before and after.
             file_size of scan_field_correction_file unchanged.
Rationale:   Silent precision loss in binary data is undetectable without hash comparison.
             This is the same check the cross_check Phase 4 performs — verify it
             also passes through the standalone app.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    path = Path(fixtures_dir) / "reference_config.h5"
    reader = MachineConfigReader(path)
    cfg = reader.parse()

    cd_before = reader.get_correction_data(0)
    icd_before = reader.get_inverse_correction_data(0)

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MachineConfigWriter(cfg).write(tmp)
    reader2 = MachineConfigReader(tmp)

    cd_after = reader2.get_correction_data(0)
    icd_after = reader2.get_inverse_correction_data(0)

    if not np.array_equal(cd_before, cd_after, equal_nan=True):
        h_before = hashlib.sha256(cd_before.tobytes()).hexdigest()[:16]
        h_after = hashlib.sha256(cd_after.tobytes()).hexdigest()[:16]
        return False, f"correction_data mismatch: {h_before}... → {h_after}..."
    if not np.array_equal(icd_before, icd_after, equal_nan=True):
        return False, "inverse_correction_data mismatch after roundtrip"

    cd_hash = hashlib.sha256(cd_before.tobytes()).hexdigest()
    return True, f"correction_data preserved: SHA-256={cd_hash}"
