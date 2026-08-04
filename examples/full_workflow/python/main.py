"""
Machine Config Library — Python Full Workflow: Calibration Adjustment
======================================================================
Run from the repo root:

    python examples/full_workflow/python/main.py

Scenario: a field calibration measured new scanner-head positions for both
optical trains.  Load the current machine config, apply the updated offsets,
write the modified config to a new file, and verify the changes persisted
alongside the binary correction data.
"""

import pathlib
import sys
import tempfile

REPO_ROOT = pathlib.Path(__file__).resolve().parents[3]
FIXTURE   = REPO_ROOT / "fixtures" / "reference_config.h5"

if not FIXTURE.exists():
    sys.exit(f"Fixture not found: {FIXTURE}\nRun from the repo root or ensure fixtures/ is present.")

from machine_config import ConfigEditor, MachineConfigReader  # noqa: E402

# ---------------------------------------------------------------------------
# 1. Print pre-calibration summary
# ---------------------------------------------------------------------------
original = MachineConfigReader(FIXTURE).parse()

print("=== Full Workflow: Calibration Adjustment ===\n")
print(f"Machine : {original.meta.machine_name}")
print(f"Trains  : {len(original.optical_trains)}")
print()
print("Before calibration:")
for i, train in enumerate(original.optical_trains):
    s  = train.scanner
    cb = train.optional_components.clearbox
    print(f"  Train {i + 1}  offset x={s.scan_head_offset_x}, y={s.scan_head_offset_y}")
    if cb and cb.correction_data:
        h = len(cb.correction_data)
        w = len(cb.correction_data[0])
        d = len(cb.correction_data[0][0])
        print(f"           correction grid {h}×{w}×{d}")
print()

# ---------------------------------------------------------------------------
# 2. Apply new scanner offsets (post-calibration values)
# ---------------------------------------------------------------------------
NEW_OFFSETS = [(-91.5, 24.0), (91.5, -24.0)]

editor = ConfigEditor(FIXTURE)
for i, (x, y) in enumerate(NEW_OFFSETS):
    editor.set_scanner_offset(train_index=i, x=x, y=y)

# ---------------------------------------------------------------------------
# 3. Write updated config
# ---------------------------------------------------------------------------
with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as tmp:
    out_path = pathlib.Path(tmp.name)

editor.save(out_path)
print(f"Written to : {out_path.name}\n")

# ---------------------------------------------------------------------------
# 4. Read back and verify
# ---------------------------------------------------------------------------
reader2  = MachineConfigReader(out_path)
updated  = reader2.parse()
failures = []

for i, (ex, ey) in enumerate(NEW_OFFSETS):
    got_x = updated.optical_trains[i].scanner.scan_head_offset_x
    got_y = updated.optical_trains[i].scanner.scan_head_offset_y
    if got_x != ex:
        failures.append(f"  train{i + 1} offset_x: expected {ex}, got {got_x}")
    if got_y != ey:
        failures.append(f"  train{i + 1} offset_y: expected {ey}, got {got_y}")
    cd = reader2.get_correction_data(i)
    if tuple(cd.shape) != (257, 257, 2):
        failures.append(f"  train{i + 1} correction shape: expected (257, 257, 2), got {cd.shape}")

out_path.unlink()

print("After calibration:")
for i, train in enumerate(updated.optical_trains):
    s = train.scanner
    print(f"  Train {i + 1}  offset x={s.scan_head_offset_x}, y={s.scan_head_offset_y}")
print()

if failures:
    print("FAIL")
    for msg in failures:
        print(msg)
    sys.exit(1)
else:
    print("PASS")
