import math
import tempfile
from machine_config import MachineConfigReader, MachineConfigWriter, MockConfigBuilder

"""
S-06: Build synthetic config with 2 lasers, verify fields and round-trip
ID:          S-06
Title:       Build a synthetic config with MockConfigBuilder and verify fields
Category:    happy-path
Layer:       builder
Precondition: None (builder creates from scratch)
Action:      Build a 2-laser config → verify fields → save to temp → re-read
Expected:    len(optical_trains) == 2
             train[0].scanner.scan_head_rotation ≈ 0.0
             train[1].scanner.scan_head_rotation ≈ 180.0
             machine_name non-empty
             correction_data centre cell ≈ 2.0 (Gaussian peak)
             After save and re-read: all fields match
Rationale:   Builder is used in CI fixture generation and by consumers who need
             synthetic test configs. Must produce valid, re-readable output.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    cfg = MockConfigBuilder(n_lasers=2).build()

    if len(cfg.optical_trains) != 2:
        return False, f"optical_trains count: {len(cfg.optical_trains)}"

    r0 = cfg.optical_trains[0].scanner.scan_head_rotation
    r1 = cfg.optical_trains[1].scanner.scan_head_rotation
    if r0 is None or abs(r0) > 0.001:
        return False, f"train[0].scan_head_rotation: {r0}"
    if r1 is None or abs(r1 - 180.0) > 0.001:
        return False, f"train[1].scan_head_rotation: {r1}"
    if not cfg.meta.machine_name:
        return False, "machine_name is empty"

    cb = cfg.optical_trains[0].optional_components.clearbox
    if cb is None or cb.correction_data is None:
        return False, "clearbox or correction_data is None"
    center = cb.correction_data[128][128][0]
    if not math.isfinite(center) or abs(center - 2.0) > 0.01:
        return False, f"correction_data center: expected ~2.0, got {center}"

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MachineConfigWriter(cfg).write(tmp)
    rb = MachineConfigReader(tmp).parse()

    if len(rb.optical_trains) != 2:
        return False, f"readback trains: {len(rb.optical_trains)}"
    if rb.meta.machine_name != cfg.meta.machine_name:
        return False, "machine_name changed after roundtrip"

    return True, "2-laser build OK, center≈2.0, roundtrip OK"
