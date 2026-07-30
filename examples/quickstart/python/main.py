"""
Machine Config Library — Python Quickstart
==========================================
Run from the repo root:

    python examples/quickstart/python/main.py

Demonstrates the six essential operations:
  1. Open an HDF5 machine config file
  2. Read scalar fields (machine name, optical train count, working distance)
  3. Inspect binary data shape (ClearBox correction grid)
  4. Write the config to a temporary HDF5 file
  5. Read the temporary file back
  6. Assert round-trip fidelity and print PASS / FAIL

No additional dependencies beyond the library itself.
"""

import pathlib
import sys
import tempfile

# ---------------------------------------------------------------------------
# Resolve the repo root so this script works regardless of working directory
# ---------------------------------------------------------------------------
REPO_ROOT = pathlib.Path(__file__).resolve().parents[3]
FIXTURE = REPO_ROOT / "fixtures" / "reference_config.h5"

if not FIXTURE.exists():
    sys.exit(f"Fixture not found: {FIXTURE}\nRun from the repo root or ensure fixtures/ is present.")

# ---------------------------------------------------------------------------
# Import the library
# ---------------------------------------------------------------------------
from machine_config import MachineConfigReader, MachineConfigWriter  # noqa: E402

# ---------------------------------------------------------------------------
# Step 1 & 2 — Open the file and read scalar fields
# ---------------------------------------------------------------------------
reader = MachineConfigReader(FIXTURE)
config = reader.parse()

print("=== Machine Config Quickstart ===\n")
print(f"Machine name   : {config.meta.machine_name}")
print(f"Optical trains : {len(config.optical_trains)}")

train0 = config.optical_trains[0]
wd = train0.scanner.working_distance
wd_unit = train0.scanner.working_distance_unit
print(f"Working dist   : {wd} {wd_unit}   (train 0)")

# ---------------------------------------------------------------------------
# Step 3 — Binary data shape (correction grid)
# ---------------------------------------------------------------------------
correction = reader.get_correction_data(0)          # numpy.ndarray, shape (257, 257, 2)
print(f"Correction grid: {correction.shape}   (train 0)")

# ---------------------------------------------------------------------------
# Step 4 — Write to a temporary file
# ---------------------------------------------------------------------------
print()
with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as tmp:
    tmp_path = pathlib.Path(tmp.name)

MachineConfigWriter(config).write(tmp_path)
print(f"Written to     : {tmp_path.name}")

# ---------------------------------------------------------------------------
# Step 5 — Read the temporary file back
# ---------------------------------------------------------------------------
config2 = MachineConfigReader(tmp_path).parse()

# ---------------------------------------------------------------------------
# Step 6 — Assert round-trip fidelity
# ---------------------------------------------------------------------------
failures = []

if config2.meta.machine_name != config.meta.machine_name:
    failures.append(
        f"  machine_name: expected {config.meta.machine_name!r}, got {config2.meta.machine_name!r}"
    )

if len(config2.optical_trains) != len(config.optical_trains):
    failures.append(
        f"  train_count: expected {len(config.optical_trains)}, got {len(config2.optical_trains)}"
    )

wd2 = config2.optical_trains[0].scanner.working_distance
if wd2 != wd:
    failures.append(f"  working_distance: expected {wd}, got {wd2}")

tmp_path.unlink()

print()
if failures:
    print("FAIL")
    for msg in failures:
        print(msg)
    sys.exit(1)
else:
    print("PASS")
