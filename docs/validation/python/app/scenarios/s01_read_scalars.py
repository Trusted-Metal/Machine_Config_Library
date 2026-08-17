from pathlib import Path
from machine_config import MachineConfigReader

"""
S-01: Read reference fixture, all scalar fields
ID:          S-01
Title:       Read reference fixture and verify all scalar fields
Category:    happy-path
Layer:       reader
Precondition: fixtures/reference_config.h5
Action:      Parse the file, print key fields to stdout
Expected:    machine_name = "TM-LPBF-02: AconityMIDI+_OG"
             build_plate_x ≈ 250.0
             build_plate_y ≈ 250.0
             len(optical_trains) = 2
             train[0].scanner.working_distance ≈ 670.0
             train[0].scanner.scan_head_rotation ≈ 0.0
             train[1].scanner.scan_head_rotation ≈ 180.0
             configuration_hash: 64 hex characters
             file_version: "1.0"
Rationale:   Establishes baseline read correctness against a known-good fixture.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    cfg = MachineConfigReader(Path(fixtures_dir) / "reference_config.h5").parse()

    if cfg.meta.machine_name != "TM-LPBF-02: AconityMIDI+_OG":
        return False, f"machine_name: got '{cfg.meta.machine_name}'"
    if cfg.machine.build_plate.x is None or abs(cfg.machine.build_plate.x - 250.0) > 0.001:
        return False, f"build_plate_x: got {cfg.machine.build_plate.x}"
    if cfg.machine.build_plate.y is None or abs(cfg.machine.build_plate.y - 250.0) > 0.001:
        return False, f"build_plate_y: got {cfg.machine.build_plate.y}"
    if len(cfg.optical_trains) != 2:
        return False, f"optical_trains count: got {len(cfg.optical_trains)}"

    wd = cfg.optical_trains[0].scanner.working_distance
    if wd is None or abs(wd - 670.0) > 0.1:
        return False, f"train[0].working_distance: got {wd}"

    r0 = cfg.optical_trains[0].scanner.scan_head_rotation
    if r0 is None or abs(r0) > 0.001:
        return False, f"train[0].scan_head_rotation: got {r0}"

    r1 = cfg.optical_trains[1].scanner.scan_head_rotation
    if r1 is None or abs(r1 - 180.0) > 0.001:
        return False, f"train[1].scan_head_rotation: got {r1}"

    h = cfg.meta.configuration_hash
    if len(h) != 64 or not all(c in "0123456789abcdefABCDEF" for c in h):
        return False, f"configuration_hash invalid: '{h}'"
    if cfg.meta.file_version.strip() != "1.0":
        return False, f"file_version: got '{cfg.meta.file_version}'"

    return True, "all scalar fields match expected values"
