from pathlib import Path
from machine_config import MachineConfigReader

"""
S-03: Read real AconityMIDI fixture file
ID:          S-03
Title:       Read real-world AconityMIDI machine config file
Category:    happy-path
Layer:       reader
Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
Action:      Parse the file, print all fields to stdout
Expected:    No error. All fields present in output match known machine parameters.
             (Record actual observed values in results.md — these become the
             reference for regression detection.)
Rationale:   This is the primary real-world validation. If the library cannot
             read a real file from the actual machine, it is not production-ready.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    matches = [
        p for p in Path(real_dir).glob("*.h5")
        if "AconityMIDI" in p.name and "OG_178" in p.name
    ]
    if not matches:
        return False, f"real AconityMIDI file not found in {real_dir}"

    cfg = MachineConfigReader(matches[0]).parse()
    fields = " | ".join([
        f"machine_name={cfg.meta.machine_name!r}",
        f"file_version={cfg.meta.file_version!r}",
        f"trains={len(cfg.optical_trains)}",
        f"build_plate_x={cfg.machine.build_plate.x}",
        f"build_plate_y={cfg.machine.build_plate.y}",
        f"wd={cfg.optical_trains[0].scanner.working_distance}",
        f"rotation[0]={cfg.optical_trains[0].scanner.scan_head_rotation}",
        f"hash={cfg.meta.configuration_hash[:16]}...",
    ])
    return True, fields
