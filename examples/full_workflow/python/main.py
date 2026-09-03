"""
Machine Config Library — Python Full Workflow: Calibration Adjustment
======================================================================
Run from the repo root:

    python examples/full_workflow/python/main.py

Load examples/dummy_2train.h5, apply new scanner offsets via set_scanner(Merge),
save, reopen, and verify both trains plus correction grids.
"""

import pathlib
import sys
import tempfile

REPO_ROOT = pathlib.Path(__file__).resolve().parents[3]
DUMMY = REPO_ROOT / "examples" / "dummy_2train.h5"

if not DUMMY.exists():
    sys.exit(
        f"Dummy file not found: {DUMMY}\nRun: python examples/generate_dummy.py"
    )

from machine_config import MachineConfigReader, SetMode, open_machine_config  # noqa: E402
from machine_config.capabilities.result import Ok  # noqa: E402

opened = open_machine_config(DUMMY)
if not isinstance(opened, Ok):
    sys.exit(f"open failed: {opened.error.message}")
file = opened.value

print("=== Full Workflow: Calibration Adjustment ===\n")
print(f"Machine : {file.meta().get_model()['machine_name']}")
print(f"Trains  : {len(file.optical_trains())}")
print()
print("Before calibration:")
reader = MachineConfigReader(DUMMY)
for i, train in enumerate(file.optical_trains()):
    s = train.get_scanner()
    print(f"  Train {i + 1}  offset x={s.get('scan_head_offset_x')}, y={s.get('scan_head_offset_y')}")
    cd = reader.get_correction_data(i)
    print(f"           correction grid {cd.shape}")
print()

NEW_OFFSETS = [(-91.5, 24.0), (91.5, -24.0)]
for i, (x, y) in enumerate(NEW_OFFSETS):
    train = file.optical_train(i)
    if not isinstance(train, Ok):
        sys.exit(train.error.message)
    scanner = train.value.get_scanner()
    scanner["scan_head_offset_x"] = x
    scanner["scan_head_offset_y"] = y
    set_r = train.value.set_scanner(scanner, SetMode.MERGE)
    if not isinstance(set_r, Ok):
        sys.exit(set_r.error.message)

with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as tmp:
    out_path = pathlib.Path(tmp.name)

saved = file.save(str(out_path))
if not isinstance(saved, Ok):
    sys.exit(f"save failed: {saved.error.message}")
file.close()
print(f"Written to : {out_path.name}\n")

again = open_machine_config(out_path)
if not isinstance(again, Ok):
    sys.exit(again.error.message)
updated = again.value
reader2 = MachineConfigReader(out_path)
failures = []

for i, (ex, ey) in enumerate(NEW_OFFSETS):
    train = updated.optical_train(i)
    if not isinstance(train, Ok):
        failures.append(f"  train{i + 1}: {train.error.message}")
        continue
    s = train.value.get_scanner()
    if s.get("scan_head_offset_x") != ex:
        failures.append(f"  train{i + 1} offset_x: expected {ex}, got {s.get('scan_head_offset_x')}")
    if s.get("scan_head_offset_y") != ey:
        failures.append(f"  train{i + 1} offset_y: expected {ey}, got {s.get('scan_head_offset_y')}")
    cd = reader2.get_correction_data(i)
    if tuple(cd.shape) != (257, 257, 2):
        failures.append(f"  train{i + 1} correction shape: expected (257, 257, 2), got {cd.shape}")

print("After calibration:")
for i, train in enumerate(updated.optical_trains()):
    s = train.get_scanner()
    print(f"  Train {i + 1}  offset x={s.get('scan_head_offset_x')}, y={s.get('scan_head_offset_y')}")
print()

updated.close()
out_path.unlink()

if failures:
    print("FAIL")
    for msg in failures:
        print(msg)
    sys.exit(1)
print("PASS")
