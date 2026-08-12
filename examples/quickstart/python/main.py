"""
Machine Config Library — Python Quickstart
==========================================
Run from the repo root:

    python examples/quickstart/python/main.py

Opens examples/dummy_2train.h5 via the stable model facade, navigates both
optical trains, round-trips a Merge set, and prints PASS / FAIL.
"""

import pathlib
import sys
import tempfile

REPO_ROOT = pathlib.Path(__file__).resolve().parents[3]
DUMMY = REPO_ROOT / "examples" / "dummy_2train.h5"

if not DUMMY.exists():
    sys.exit(
        f"Dummy file not found: {DUMMY}\n"
        "Run: python examples/generate_dummy.py"
    )

from machine_config import MachineConfigReader, SetMode, open_machine_config  # noqa: E402
from machine_config.capabilities.result import Ok  # noqa: E402

opened = open_machine_config(DUMMY)
if not isinstance(opened, Ok):
    sys.exit(f"open failed: {opened.error.code} — {opened.error.message}")
file = opened.value

print("=== Machine Config Quickstart ===\n")
print(f"File version   : {file.file_version()}")
meta = file.meta().get_model()
print(f"Machine name   : {meta['machine_name']}")
print(f"Optical trains : {len(file.optical_trains())}")

for i, train in enumerate(file.optical_trains()):
    scanner = train.get_scanner()
    print(
        f"  Train {i}  wd={scanner['working_distance']} {scanner.get('working_distance_unit') or ''}  "
        f"offset x={scanner.get('scan_head_offset_x')}, y={scanner.get('scan_head_offset_y')}"
    )
    oc = train.optional_components()
    if oc is None:
        print("           optionalComponents: none")
    else:
        cb = oc.clearbox()
        print(f"           clearbox: {'present' if cb.ok else cb.error.code}")

train0 = file.optical_train(0)
if not isinstance(train0, Ok):
    sys.exit(train0.error.message)
scanner0 = train0.value.get_scanner()

reader = MachineConfigReader(DUMMY)
correction = reader.get_correction_data(0)
print(f"Correction grid: {correction.shape}   (train 0)")

print()
with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as tmp:
    tmp_path = pathlib.Path(tmp.name)

set_r = train0.value.set_scanner(scanner0, SetMode.MERGE)
if not isinstance(set_r, Ok):
    sys.exit(f"set_scanner failed: {set_r.error.message}")
saved = file.save(str(tmp_path))
if not isinstance(saved, Ok):
    sys.exit(f"save failed: {saved.error.message}")
print(f"Written to     : {tmp_path.name}")

again = open_machine_config(tmp_path)
if not isinstance(again, Ok):
    sys.exit(again.error.message)
name2 = again.value.meta().get_model()["machine_name"]
n2 = len(again.value.optical_trains())
t2 = again.value.optical_train(0)
wd2 = t2.value.get_scanner()["working_distance"] if isinstance(t2, Ok) else None

failures = []
if name2 != meta["machine_name"]:
    failures.append("  machine_name mismatch after round-trip")
if n2 != len(file.optical_trains()):
    failures.append(f"  train_count: expected {len(file.optical_trains())}, got {n2}")
if wd2 != scanner0["working_distance"]:
    failures.append(f"  working_distance: expected {scanner0['working_distance']}, got {wd2}")

file.close()
again.value.close()
tmp_path.unlink()

print()
if failures:
    print("FAIL")
    for msg in failures:
        print(msg)
    sys.exit(1)
print("PASS")
