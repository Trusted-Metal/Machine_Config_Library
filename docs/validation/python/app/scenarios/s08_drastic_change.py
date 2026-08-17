import copy
import tempfile
from dataclasses import replace
from pathlib import Path
from machine_config import MachineConfigReader, MachineConfigWriter

"""
S-08: Drastic change: add new train, modify build_plate_x, clearbox, scan_head_rotation using real AconityMIDI file
ID:          S-08
Title:       Drastic field change to real file, verify adapter pipeline integrity
Category:    happy-path (applied)
Layer:       reader + writer + adapter
Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
Action:      Read the real file → make ALL of the following changes:
               1. Add a third optical train (clone train 1, change train_id)
               2. Change build_plate_x from 250.0 to 350.0
               3. Set scan_head_rotation on new train to 90.0
               4. Clear all correction data (set to None/null/nil)
               5. Change machine_name to "MODIFIED_ACONITY_VALIDATION"
             → write to temp → read temp → verify all five changes persisted
Expected:    len(optical_trains) == 3
             build_plate_x ≈ 350.0
             train[2].scanner.scan_head_rotation ≈ 90.0
             train[2].optional_components.clearbox.correction_data is None/null/nil
             machine_name == "MODIFIED_ACONITY_VALIDATION"
             file_version still "1.0" (adapter did not change the version)
Rationale:   This is the "drastic change" scenario. Tests that the adapter pipeline
             handles structural changes (new train, nil correction data) without
             silent corruption or wrong-adapter dispatch.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    matches = [
        p for p in Path(real_dir).glob("*.h5")
        if "AconityMIDI" in p.name and "OG_178" in p.name
    ]
    if not matches:
        return False, f"real AconityMIDI file not found in {real_dir}"

    cfg = MachineConfigReader(matches[0]).parse()

    new_train = copy.deepcopy(cfg.optical_trains[1])
    new_train = replace(new_train, train_id="Optical_Train_03")
    new_bp = replace(cfg.machine.build_plate, x=350.0)
    new_train = replace(new_train, scanner=replace(new_train.scanner, scan_head_rotation=90.0))
    new_train = replace(new_train,
        optional_components=replace(new_train.optional_components, clearbox=None))

    modified = replace(cfg,
        meta=replace(cfg.meta, machine_name="MODIFIED_ACONITY_VALIDATION"),
        machine=replace(cfg.machine, build_plate=new_bp, machine_name="MODIFIED_ACONITY_VALIDATION"),
        optical_trains=cfg.optical_trains + [new_train],
    )

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MachineConfigWriter(modified).write(tmp)
    rb = MachineConfigReader(tmp).parse()

    if len(rb.optical_trains) != 3:
        return False, f"optical_trains: expected 3, got {len(rb.optical_trains)}"
    if rb.machine.build_plate.x is None or abs(rb.machine.build_plate.x - 350.0) > 0.001:
        return False, f"build_plate_x: got {rb.machine.build_plate.x}"
    r2 = rb.optical_trains[2].scanner.scan_head_rotation
    if r2 is None or abs(r2 - 90.0) > 0.001:
        return False, f"train[2].scan_head_rotation: got {r2}"
    if rb.optical_trains[2].optional_components.clearbox is not None:
        return False, "train[2].clearbox should be None"
    if rb.meta.machine_name != "MODIFIED_ACONITY_VALIDATION":
        return False, f"machine_name: got '{rb.meta.machine_name}'"
    if rb.meta.file_version.strip() != "1.0":
        return False, f"file_version changed: '{rb.meta.file_version}'"

    return True, "3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK"
