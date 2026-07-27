# Machine Config Library — End-to-End Usage Guide

This document covers how to install, use, and verify the library in each supported language.
It is written for two audiences:

- **Integrators** — building systems that consume machine configuration files
- **Contributors** — verifying library correctness, generating golden fixtures, extending the library

The [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) covers *how the library is built*.
This document covers *how to use it once built*.

---

## Contents

- [Shared Concepts](#shared-concepts)
- [Python](#python)
  - [Use case 1 — Parse a machine config file](#use-case-1--parse-a-machine-config-file)
  - [Use case 2 — Export to canonical JSON](#use-case-2--export-to-canonical-json)
  - [Use case 3 — Reconstruct a config from JSON](#use-case-3--reconstruct-a-config-from-json)
  - [Use case 4 — Write a config back to HDF5](#use-case-4--write-a-config-back-to-hdf5)
  - [Use case 5 — Edit a field and save to a new file](#use-case-5--edit-a-field-and-save-to-a-new-file)
  - [Use case 6 — Generate a synthetic test config](#use-case-6--generate-a-synthetic-test-config)
  - [Use case 7 — Build a config from a YAML specification](#use-case-7--build-a-config-from-a-yaml-specification)
  - [Use case 8 — Read OPCUA telemetry configuration](#use-case-8--read-opcua-telemetry-configuration)
  - [Use case 9 — Access ClearBox correction arrays](#use-case-9--access-clearbox-correction-arrays)
  - [Use case 10 — Validate a config against the schema](#use-case-10--validate-a-config-against-the-schema)
- [Node.js](#nodejs) ← *coming in Phase 2*
- [Rust](#rust) ← *coming in Phase 3*
- [C++](#c) ← *coming in Phase 4*
- [Contributor Workflows](#contributor-workflows)

---

## Shared Concepts

### The HDF5 file

LPBF machine configurations are stored as `.h5` files. The library is **machine-agnostic** — any
machine that maps its attributes into the MachineConfig structure can be read, written, and
validated. The reference fixtures happen to come from an AconityMIDI system, but no
vendor-specific logic is encoded in the library.

The canonical output is a JSON document that matches `schema/machine_config_v1.schema.json`.
Any machine whose HDF5 export follows the schema structure is a valid input.

### The canonical JSON format

Every language produces identical JSON from the same `.h5` file. This is enforced by the
cross-check CI pipeline. The format is defined once in the schema and never duplicated.

### Fixtures

| File | What it is |
|---|---|
| `fixtures/reference_config.h5` | Real AconityMIDI 2-laser config — primary correctness fixture (example machine) |
| `fixtures/reference_config_opcua.h5` | Same machine with OPCUA telemetry group populated |
| `fixtures/synthetic_2laser.h5` | MockConfigBuilder output — used by non-Python language test suites |
| `fixtures/reference_output.json` | Golden file — Python's canonical JSON output; all languages must match |
| `fixtures/reference_output.sha256` | SHA-256 of the golden file — CI tamper guard |

---

## Python

### Installation

**One-time setup (from repo root):**

```powershell
# PowerShell
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e python/
```

```bash
# Git Bash
python -m venv .venv
source .venv/Scripts/activate
pip install -e python/
```

The `machine-config` CLI command is installed automatically.

---

### Use case 1 — Parse a machine config file

```python
from machine_config import MachineConfigReader

reader = MachineConfigReader("fixtures/reference_config.h5")
config = reader.parse()

print(config.meta.machine_name)          # "TM-LPBF-02: AconityMIDI+_OG"
print(config.meta.configuration_hash)    # 64-character hex string

bp = config.machine.build_plate
print(f"{bp.x} x {bp.y} x {bp.z} {bp.x_unit}")  # 250.0 x 250.0 x 20.0 mm

for i, train in enumerate(config.optical_trains):
    s = train.scanner
    print(f"Train {i+1}: WD={s.working_distance} {s.working_distance_unit}  "
          f"offset=({s.scan_head_offset_x}, {s.scan_head_offset_y}) mm  "
          f"axis_config={s.axis_configuration}")
    # Access axis subgroups — always present: x_axis, y_axis
    # z_axis present for '3D' and '3D+Focus'; focus present only for '3D+Focus'
    print(f"  X smoothing_kernel: {s.x_axis.smoothing_kernel}")
    if s.z_axis is not None:
        print(f"  Z bit_resolution: {s.z_axis.actual_bit_resolution} {s.z_axis.actual_bit_resolution_unit}")
```

---

### Use case 2 — Export to canonical JSON

By default, `to_json()` produces a compact, human-readable JSON snapshot containing all **scalar and metadata fields**. Binary datasets — ClearBox correction arrays and raw `.fc3` scan-field correction bytes — are intentionally excluded. The HDF5 file is the source of truth for those; use the dedicated accessors or `include_binary=True` when you need them.

```python
from machine_config import MachineConfigReader
import pathlib

reader = MachineConfigReader("fixtures/reference_config.h5")

# Default: metadata + scalars only (~13 KB for a typical 2-laser config)
print(reader.to_json(indent=2))
pathlib.Path("output.json").write_text(reader.to_json(indent=2), encoding="utf-8")

# With binary data included: adds correction_data, inverse_correction_data (257×257×2
# float64 arrays as nested lists) and raw_bytes (base64-encoded .fc3 file bytes).
# Output is large (~14 MB for a 2-laser config with ClearBox).
pathlib.Path("output_full.json").write_text(
    reader.to_json(indent=2, include_binary=True), encoding="utf-8"
)
```

> **When to use `include_binary=True`**: debugging correction array values, creating a fully self-contained JSON archive, or comparing correction grids between two configs programmatically. For routine inspection, logging, or CI drift detection, the default is preferred — it is fast and the output is readable.

> **Accessing binary data without JSON**: use `reader.get_correction_data(train_index)` (returns a `numpy.ndarray` of shape `(257, 257, 2)`) and `reader.get_scan_field_correction_bytes(train_index)` (returns raw `bytes`). These are the recommended paths for numerical work.

---

### Use case 3 — Reconstruct a config from JSON

Useful when you have a JSON file (e.g. from `export-json`) and need to write it back to HDF5,
or when passing configs between services as JSON.

```python
import json
from machine_config import MachineConfigReader, MachineConfigWriter, config_from_dict

# Read an existing canonical JSON file
data = json.loads(pathlib.Path("output.json").read_text(encoding="utf-8"))
config = config_from_dict(data)

# Write it back to HDF5
MachineConfigWriter(config).write("reconstructed.h5")

# Verify the roundtrip
config2 = MachineConfigReader("reconstructed.h5").parse()
assert config2.meta.configuration_hash == config.meta.configuration_hash
```

---

### Use case 4 — Write a config back to HDF5

```python
from machine_config import MachineConfigReader, MachineConfigWriter

config = MachineConfigReader("original.h5").parse()

# MachineConfig is immutable (dataclasses). Use dataclasses.replace() to change fields.
# Then write the modified config to a new file.
MachineConfigWriter(config).write("copy.h5")
```

---

### Use case 5 — Edit a field and save to a new file

```python
from machine_config.builder import ConfigEditor

editor = ConfigEditor("original.h5")

# Adjust scanner offsets for optical train 0 (0-based index)
editor.set_scanner_offset(train_index=0, x=-90.0, y=25.0)

# Write the modified config — original.h5 is never touched
editor.save("adjusted.h5")
```

---

### Use case 6 — Generate a synthetic test config

```python
from machine_config.builder import MockConfigBuilder

# 2-laser config with ClearBox and correction files
MockConfigBuilder(n_lasers=2).save("test_config.h5")

# 1-laser, no ClearBox, custom build plate
MockConfigBuilder(
    n_lasers=1,
    include_clearbox=False,
    build_plate_x=300.0,
    build_plate_y=300.0,
    machine_name="TestMachine",
).save("single_laser.h5")
```

---

### Use case 7 — Build a config from a YAML specification

```python
from machine_config.builder import YamlConfigBuilder

YamlConfigBuilder("spec.yaml").save("config.h5")
```

**Minimal `spec.yaml`:**

```yaml
machine:
  name: "My-LPBF-01"
  manufacturer: "Acme"
  model: "AcmeMIDI+"
  build_plate_x: 250
  build_plate_y: 250
  gas_flow_direction: "Y+"
  recoat_direction: "X+"

optical_trains:
  - scanner:
      working_distance: 670
      scan_head_offset_x: -87.5
      scan_head_offset_y: 23.5
      scan_field_x: 600
      scan_field_y: 600
    light_source:
      wavelength: 1070
      power_max_nominal: 1000
```

Any key not present in the YAML gets `null` in the output.
ClearBox and scan field correction file groups are not written by `YamlConfigBuilder`.

---

### Use case 8 — Read OPCUA telemetry configuration

OPCUA data is outside the canonical model (it is machine-telemetry config, not optical train data).
Access it via `get_raw_group()`:

```python
from machine_config import MachineConfigReader

reader = MachineConfigReader("fixtures/reference_config_opcua.h5")

client = reader.get_raw_group("OPCUA/Client")
print(client["Server_URL"])       # opc.tcp://172.17.20.240:62541/...
print(client["Auth_Mode"])        # UsernamePassword
print(client["Security_Mode"])    # SignAndEncrypt

pipe = reader.get_raw_group("OPCUA/Pipe")
print(pipe["Pipe_Enabled"])       # 1

triggers = reader.get_raw_group("OPCUA/Triggers")
interlock = reader.get_raw_group("OPCUA/Triggers/Laser Emission Interlock")
print(interlock["Signal"])        # yellow_light
print(interlock["Subsystem"])     # Chamber

# Returns {} for groups that don't exist — never raises
result = reader.get_raw_group("OPCUA")     # {} on standard (non-OPCUA) fixture
```

---

### Use case 9 — Access ClearBox correction arrays

ClearBox correction grids are stored as `(257, 257, 2)` float64 arrays in HDF5 and
represented in JSON as nested Python lists. Out-of-field points that are `NaN` in HDF5
become `None` in the list (and `null` in JSON); finite floats pass through as-is.

```python
from machine_config import MachineConfigReader

reader = MachineConfigReader("fixtures/reference_config.h5")
config = reader.parse()

cb = config.optical_trains[0].clearbox
if cb is not None:
    data = cb.correction_data        # list[list[list[float | None]]], shape 257×257×2
    inv  = cb.inverse_correction_data

    # Shape inspection
    assert len(data) == 257          # first dimension
    assert len(data[0]) == 257       # second dimension
    assert len(data[0][0]) == 2      # two channels (X warp, Y warp)

    # Sample values — None means the point is outside the correction field
    centre_x = data[128][128][0]     # X-channel at centre; float or None
    centre_y = data[128][128][1]     # Y-channel at centre
    print(f"Centre correction: x={centre_x}, y={centre_y}")

# For bulk numerical work, use the raw numpy API (unchanged from earlier phases):
arr = reader.get_correction_data(0)          # np.ndarray, shape (257, 257, 2), float64
inv = reader.get_inverse_correction_data(0)  # same shape; NaN preserved
import numpy as np
print(f"Non-finite cells: {np.sum(~np.isfinite(arr))}")
```

---

### Use case 10 — Validate a config against the schema

```python
import json, jsonschema
from machine_config import MachineConfigReader
from machine_config.schema import SCHEMA

reader = MachineConfigReader("fixtures/reference_config.h5")
output = json.loads(reader.to_json())
jsonschema.validate(output, SCHEMA)   # raises ValidationError if invalid
print("Schema valid.")
```

---

### CLI reference (Python)

```powershell
# PowerShell

# Inspect a config — brief summary
.\.venv\Scripts\machine-config.exe inspect fixtures/reference_config.h5

# Inspect with full YAML dump of every field
.\.venv\Scripts\machine-config.exe inspect fixtures/reference_config.h5 --verbose

# Validate against the schema
.\.venv\Scripts\machine-config.exe validate fixtures/reference_config.h5

# Export canonical JSON to stdout (metadata + scalars only, ~13 KB)
.\.venv\Scripts\machine-config.exe export-json fixtures/reference_config.h5

# Export canonical JSON to a file
.\.venv\Scripts\machine-config.exe export-json fixtures/reference_config.h5 --output out.json

# Include correction arrays and raw .fc3 bytes (large output, ~14 MB)
.\.venv\Scripts\machine-config.exe export-json fixtures/reference_config.h5 --include-binary --output full.json

# Write a canonical JSON file back to HDF5
.\.venv\Scripts\machine-config.exe write out.json --output reconstructed.h5

# Build from a YAML spec
.\.venv\Scripts\machine-config.exe build --from-yaml spec.yaml --output config.h5

# Build a synthetic 2-laser mock config
.\.venv\Scripts\machine-config.exe build --mock --output test_config.h5

# Build a synthetic 1-laser mock config
.\.venv\Scripts\machine-config.exe build --mock --lasers 1 --output single.h5

# Generate a synthetic config and print its full JSON (no files needed)
.\.venv\Scripts\machine-config.exe demo
```

```bash
# Git Bash

# Inspect a config — brief summary
.venv/Scripts/machine-config.exe inspect fixtures/reference_config.h5

# Inspect with full YAML dump of every field
.venv/Scripts/machine-config.exe inspect fixtures/reference_config.h5 --verbose

# Validate against the schema
.venv/Scripts/machine-config.exe validate fixtures/reference_config.h5

# Export canonical JSON to stdout (metadata + scalars only, ~13 KB)
.venv/Scripts/machine-config.exe export-json fixtures/reference_config.h5

# Export canonical JSON to a file
.venv/Scripts/machine-config.exe export-json fixtures/reference_config.h5 --output out.json

# Include correction arrays and raw .fc3 bytes (large output, ~14 MB)
.venv/Scripts/machine-config.exe export-json fixtures/reference_config.h5 --include-binary --output full.json

# Write a canonical JSON file back to HDF5
.venv/Scripts/machine-config.exe write out.json --output reconstructed.h5

# Build from a YAML spec
.venv/Scripts/machine-config.exe build --from-yaml spec.yaml --output config.h5

# Build a synthetic 2-laser mock config
.venv/Scripts/machine-config.exe build --mock --output test_config.h5

# Build a synthetic 1-laser mock config
.venv/Scripts/machine-config.exe build --mock --lasers 1 --output single.h5

# Generate a synthetic config and print its full JSON (no files needed)
.venv/Scripts/machine-config.exe demo
```

---

### Running the Python test suite

```powershell
# PowerShell — full suite (301 tests)
.\.venv\Scripts\python.exe -m pytest python/tests/ -v

# Individual suites
.\.venv\Scripts\python.exe -m pytest python/tests/test_reader.py -v
.\.venv\Scripts\python.exe -m pytest python/tests/test_writer.py -v
.\.venv\Scripts\python.exe -m pytest python/tests/test_writer_roundtrip.py -v.\.\.venv\Scripts\python.exe -m pytest python/tests/test_opcua_roundtrip.py -v.\.venv\Scripts\python.exe -m pytest python/tests/test_builder.py -v
.\.venv\Scripts\python.exe -m pytest python/tests/test_cli.py -v
.\.venv\Scripts\python.exe -m pytest python/tests/test_schema.py -v
```

```bash
# Git Bash — full suite (301 tests)
.venv/Scripts/python.exe -m pytest python/tests/ -v

# Individual suites
.venv/Scripts/python.exe -m pytest python/tests/test_reader.py -v
.venv/Scripts/python.exe -m pytest python/tests/test_writer.py -v
.venv/Scripts/python.exe -m pytest python/tests/test_writer_roundtrip.py -v
.venv/Scripts/python.exe -m pytest python/tests/test_opcua_roundtrip.py -v
.venv/Scripts/python.exe -m pytest python/tests/test_builder.py -v
.venv/Scripts/python.exe -m pytest python/tests/test_cli.py -v
.venv/Scripts/python.exe -m pytest python/tests/test_schema.py -v
```

---

## Node.js

*Coming in Phase 2.*

Section will cover:
- `npm install machine-config-library`
- Parsing `.h5` with `h5wasm` in Node.js and browser environments
- TypeScript interfaces
- Equivalent use cases to the Python section above
- CLI usage via `node dist/cli.js`
- Viewer bundle integration

---

## Rust

*Coming in Phase 3.*

Section will cover:
- `cargo add machine-config`
- Parsing `.h5` with the `hdf5` crate
- Rust struct model and serde serialisation
- Equivalent use cases to the Python section above
- CLI usage via `machine-config-cli`

---

## C++

*Coming in Phase 4.*

Section will cover:
- CMake `FetchContent` / vcpkg integration
- Parsing `.h5` with HighFive
- `nlohmann/json` serialisation
- Equivalent use cases to the Python section above
- CLI usage

---

## Contributor Workflows

### Running the smoke test (pre-commit sanity check)

Before committing, run this outside the `python/` package to simulate a real caller:

```powershell
# PowerShell
.\.venv\Scripts\python.exe scratch/smoke_test.py
```

```bash
# Git Bash
.venv/Scripts/python.exe scratch/smoke_test.py
```

This exercises the public API from a caller's perspective — it catches missing exports,
confusing API surfaces, and path-dependent bugs that pytest's `pythonpath` injection can mask.

---

### Generating the golden file and synthetic fixture (Phase 1.7)

**Prerequisites:** all 170 tests pass (1 skipped — the golden file test).

```powershell
# PowerShell

# Step 1 — confirm the suite is green
.\.venv\Scripts\python.exe -m pytest python/tests/ -v

# Step 2 — generate the three fixture files
.\.venv\Scripts\python.exe tools/generate_fixtures.py
```

```bash
# Git Bash

# Step 1 — confirm the suite is green
.venv/Scripts/python.exe -m pytest python/tests/ -v

# Step 2 — generate the three fixture files
.venv/Scripts/python.exe tools/generate_fixtures.py
```

This writes:
- `fixtures/reference_output.json` — canonical JSON from the reference fixture (AconityMIDI example machine)
- `fixtures/reference_output.sha256` — SHA-256 of that JSON
- `fixtures/synthetic_2laser.h5` — MockConfigBuilder output for non-Python language tests

```powershell
# PowerShell — Step 3: re-run the suite; the previously-skipped test now activates
.\.venv\Scripts\python.exe -m pytest python/tests/ -v
# Expected: 171 passed, 0 skipped
```

```bash
# Git Bash — Step 3
.venv/Scripts/python.exe -m pytest python/tests/ -v
# Expected: 171 passed, 0 skipped
```

**Human review checklist** — verify `fixtures/reference_output.json` against
`Reference Materials/*_structure.txt` before committing:

- [ ] `meta.machine_name` = `"TM-LPBF-02: AconityMIDI+_OG"`
- [ ] `meta.configuration_hash` is exactly 64 hex characters
- [ ] `machine.build_plate_x` = 250.0, `build_plate_y` = 250.0, `build_plate_z` = 20.0
- [ ] Two optical trains present
- [ ] Train 01: `working_distance` = 670.0, `scan_head_offset_x` = −87.5, `scan_head_offset_y` = 23.5, `scan_head_rotation` = 0.0
- [ ] Train 02: `scan_head_offset_x` = 86.074, `scan_head_offset_y` = −21.695, `scan_head_rotation` = 180.0
- [ ] Train 01 `thermal_lensing_passed` = false
- [ ] Train 02 `thermal_lensing_passed` = true
- [ ] Both trains: `clearbox.correction_data` is a 257×257×2 nested list (outer dimensions 257, inner dimension 2)
- [ ] Train 01 `scan_field_correction_file.file_size` = 1138799
- [ ] Train 02 `scan_field_correction_file.file_size` = 1142763
- [ ] No OPCUA fields appear anywhere in the output

```powershell
# PowerShell — Step 4: commit all three files together (never split across commits)
git add fixtures/reference_output.json fixtures/reference_output.sha256 fixtures/synthetic_2laser.h5
git commit -m "Phase 1.7: golden file + synthetic fixture"
```

```bash
# Git Bash — Step 4
git add fixtures/reference_output.json fixtures/reference_output.sha256 fixtures/synthetic_2laser.h5
git commit -m "Phase 1.7: golden file + synthetic fixture"
```

---

### Regenerating the golden file after a reader fix

If a bug is found in the reader after the golden file is committed:

1. Fix the reader bug in `python/src/machine_config/reader.py`
2. Add or update a specific unit test asserting the now-correct value
3. Run the full suite — confirm it passes
4. Re-run `tools/generate_fixtures.py` — it overwrites both files atomically
5. Check `git diff fixtures/reference_output.json` — verify only the corrected field changed
6. Commit `reference_output.json` and `reference_output.sha256` together

If other languages are already implemented, check whether they independently produce the correct
value. If they agree with Python's old wrong value, all implementations share the same bug —
update all affected language unit tests after verifying the correct value from the structure files.

---

### What the cross-check does

After Phase 2, every push to GitHub runs:

```
python.yml    → exports /tmp/python_output.json
nodejs.yml    → exports /tmp/nodejs_output.json
               (rust.yml and cpp.yml added as each language lands)
                        │
              cross_check.yml runs only if all language jobs pass
                        │
                        ├── sha256 guard: recomputes hash of reference_output.json,
                        │   compares to committed .sha256
                        │   → fails if someone edited the JSON by hand
                        │
                        └── deepdiff: compares each language's output
                            against reference_output.json
                            → any difference = CI failure with exact diff shown
```

The cross-check catches bugs that per-language tests cannot: two languages
independently producing the same wrong value will both pass their own tests
but disagree with the golden file.

> **Note on machine neutrality**: the golden file and fixtures use an AconityMIDI machine
> as the reference example. This is not a constraint on the library — any machine that maps
> its HDF5 attributes into the MachineConfig structure is a valid input. New machine types
> are validated by adding their fixture files and asserting their field values follow the
> same schema.
