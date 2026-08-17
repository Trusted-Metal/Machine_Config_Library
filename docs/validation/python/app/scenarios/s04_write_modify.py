import tempfile
from dataclasses import replace
from pathlib import Path
from machine_config import MachineConfigReader, MachineConfigWriter

"""
S-04: Write modified config and verify field change survives round-trip
ID:          S-04
Title:       Modify a scalar field, write, re-read, verify change persisted
Category:    happy-path
Layer:       writer + reader
Precondition: fixtures/reference_config.h5
Action:      Read → change machine_name to "VALIDATION_TEST_MACHINE" →
             write to temp file → read temp file → assert machine_name matches
Expected:    machine_name == "VALIDATION_TEST_MACHINE" after roundtrip.
             All other fields unchanged.
             file_version == "1.0" unchanged.
Rationale:   Basic write correctness. If a modified field does not survive,
             the writer has a silent data loss bug.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    path = Path(fixtures_dir) / "reference_config.h5"
    cfg = MachineConfigReader(path).parse()
    orig_x = cfg.machine.build_plate.x

    modified = replace(cfg,
        meta=replace(cfg.meta, machine_name="VALIDATION_TEST_MACHINE"),
        machine=replace(cfg.machine, machine_name="VALIDATION_TEST_MACHINE"),
    )

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MachineConfigWriter(modified).write(tmp)
    rb = MachineConfigReader(tmp).parse()

    if rb.meta.machine_name != "VALIDATION_TEST_MACHINE":
        return False, f"machine_name not persisted: '{rb.meta.machine_name}'"
    if rb.meta.file_version.strip() != "1.0":
        return False, f"file_version changed: '{rb.meta.file_version}'"
    if rb.machine.build_plate.x != orig_x:
        return False, f"build_plate_x changed: {orig_x} → {rb.machine.build_plate.x}"

    return True, "machine_name persisted, file_version and other fields unchanged"
