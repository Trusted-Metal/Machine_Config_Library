# Python — Machine Config Library

The reference implementation. Lives in `python/` (package `machine_config`).
Python is the source of truth for fixture generation, the golden file, and schema definition.

← [Back to index](../USAGE.md)

---

## Contents

- [Installation](#installation)
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
- [CLI reference](#cli-reference)
- [Quickstart example](#quickstart-example)
- [Full workflow example](#full-workflow-example)
- [Running the Python test suite](#running-the-python-test-suite)

---

## Installation

**One-time setup (from repo root):**

```powershell
# PowerShell
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e python/
```

```bash
# Git Bash / Linux / macOS
python -m venv .venv
source .venv/Scripts/activate   # Windows Git Bash
# source .venv/bin/activate     # Linux / macOS
pip install -e python/
```

The `machine-config` CLI command is installed automatically alongside the package.

> **VS Code tip**: Select `.venv\Scripts\python.exe` as the workspace interpreter. VS Code's
> integrated terminal will then activate the venv automatically on every new terminal.

---

## Use case 1 — Parse a machine config file

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
    # x_axis, y_axis always present; z_axis present for '3D'/'3D+Focus';
    # focus present only for '3D+Focus'
    print(f"  X smoothing_kernel: {s.x_axis.smoothing_kernel}")
    if s.z_axis is not None:
        print(f"  Z bit_resolution: {s.z_axis.actual_bit_resolution} {s.z_axis.actual_bit_resolution_unit}")
```

---

## Use case 2 — Export to canonical JSON

By default, `to_json()` produces a compact, human-readable JSON snapshot containing all
**scalar and metadata fields**. Binary datasets — ClearBox correction arrays and raw `.fc3`
scan-field correction bytes — are intentionally excluded. Use the dedicated accessors or
`include_binary=True` when you need them.

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

> **Accessing binary data without JSON**: use `reader.get_correction_data(train_index)` (returns
> `numpy.ndarray` shape `(257, 257, 2)`) and `reader.get_scan_field_correction_bytes(train_index)`
> (returns raw `bytes`). These are the recommended paths for numerical work.

---

## Use case 3 — Reconstruct a config from JSON

Useful when you have a JSON file (e.g. from `export-json`) and need to write it back to HDF5,
or when passing configs between services as JSON.

```python
import json, pathlib
from machine_config import MachineConfigReader, MachineConfigWriter, config_from_dict

data = json.loads(pathlib.Path("output.json").read_text(encoding="utf-8"))
config = config_from_dict(data)

MachineConfigWriter(config).write("reconstructed.h5")

config2 = MachineConfigReader("reconstructed.h5").parse()
assert config2.meta.configuration_hash == config.meta.configuration_hash
```

---

## Use case 4 — Write a config back to HDF5

```python
from machine_config import MachineConfigReader, MachineConfigWriter

config = MachineConfigReader("original.h5").parse()

# MachineConfig is immutable (dataclasses). Use dataclasses.replace() to change fields,
# then write the modified config to a new file.
MachineConfigWriter(config).write("copy.h5")
```

---

## Use case 5 — Edit a field and save to a new file

```python
from machine_config.builder import ConfigEditor

editor = ConfigEditor("original.h5")

# Adjust scanner offsets for optical train 0 (0-based index)
editor.set_scanner_offset(train_index=0, x=-90.0, y=25.0)

# Write the modified config — original.h5 is never touched
editor.save("adjusted.h5")
```

---

## Use case 6 — Generate a synthetic test config

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

## Use case 7 — Build a config from a YAML specification

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

## Use case 8 — Read OPCUA telemetry configuration

OPCUA data is optional machine-telemetry config stored in a separate `OPCUA` HDF5 group.
`parse()` returns `config.opcua` (populated when the group exists, `None` otherwise).
Use `get_raw_group()` for ad-hoc inspection of any path:

```python
from machine_config import MachineConfigReader

reader = MachineConfigReader("fixtures/reference_config_opcua.h5")
config = reader.parse()

if config.opcua:
    print(config.opcua.client.server_url)       # opc.tcp://172.17.20.240:...
    print(config.opcua.client.auth_mode)        # UsernamePassword
    for name, trigger in config.opcua.triggers.items():
        print(f"{name}: signal={trigger.signal}")

# Raw attribute map — returns {} for missing paths, never raises
client = reader.get_raw_group("OPCUA/Client")
print(client["Server_URL"])
print(client["Auth_Mode"])

interlock = reader.get_raw_group("OPCUA/Triggers/Laser Emission Interlock")
print(interlock["Signal"])       # yellow_light
print(interlock["Subsystem"])    # Chamber

result = reader.get_raw_group("does/not/exist")   # {}
```

---

## Use case 9 — Access ClearBox correction arrays

ClearBox correction grids are stored as `(257, 257, 2)` float64 arrays in HDF5 and
represented in JSON as nested Python lists. Out-of-field points that are `NaN` in HDF5
become `None` in the list (and `null` in JSON); finite floats pass through as-is.

```python
from machine_config import MachineConfigReader
import numpy as np

reader = MachineConfigReader("fixtures/reference_config.h5")
config = reader.parse()

cb = config.optical_trains[0].clearbox
if cb is not None:
    data = cb.correction_data        # list[list[list[float | None]]], shape 257×257×2
    centre_x = data[128][128][0]     # X-channel at centre; float or None
    centre_y = data[128][128][1]

# For bulk numerical work, use the raw numpy API:
arr = reader.get_correction_data(0)          # np.ndarray, shape (257, 257, 2), float64
inv = reader.get_inverse_correction_data(0)  # same shape; NaN preserved
print(f"Non-finite cells: {np.sum(~np.isfinite(arr))}")
```

---

## Use case 10 — Validate a config against the schema

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

## CLI reference

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
.venv/Scripts/machine-config.exe inspect fixtures/reference_config.h5
.venv/Scripts/machine-config.exe validate fixtures/reference_config.h5
.venv/Scripts/machine-config.exe export-json fixtures/reference_config.h5
.venv/Scripts/machine-config.exe export-json fixtures/reference_config.h5 --output out.json
.venv/Scripts/machine-config.exe export-json fixtures/reference_config.h5 --include-binary --output full.json
.venv/Scripts/machine-config.exe write out.json --output reconstructed.h5
.venv/Scripts/machine-config.exe build --from-yaml spec.yaml --output config.h5
.venv/Scripts/machine-config.exe build --mock --output test_config.h5
.venv/Scripts/machine-config.exe demo
```

---

## Quickstart example

```powershell
# PowerShell — from repo root
.\.venv\Scripts\python.exe examples/quickstart/python/main.py
```

```bash
# Git Bash
.venv/Scripts/python.exe examples/quickstart/python/main.py
```

Expected output:

```
=== Machine Config Quickstart ===

Machine name   : TM-LPBF-02: AconityMIDI+_OG
Optical trains : 2
Working dist   : 670.0 mm   (train 0)
Correction grid: (257, 257, 2)   (train 0)

Written to     : <tmp>.h5

PASS
```

---

## Full workflow example

**Scenario:** a field calibration has produced new scanner-head positions. Load the current
config, apply the updated offsets for both trains, write a modified config, and verify the
changes persisted alongside the binary correction data.

```powershell
# PowerShell
.venv\Scripts\python.exe examples/full_workflow/python/main.py
```

```bash
# Git Bash
.venv/Scripts/python.exe examples/full_workflow/python/main.py
```

Expected output:

```
=== Full Workflow: Calibration Adjustment ===

Machine : TM-LPBF-02: AconityMIDI+_OG
Trains  : 2

Before calibration:
  Train 1  offset x=-87.5, y=23.5
           correction grid 257×257×2
  Train 2  offset x=86.074, y=-21.695
           correction grid 257×257×2

Written to : <tmp>.h5

After calibration:
  Train 1  offset x=-91.5, y=24.0
  Train 2  offset x=91.5, y=-24.0

PASS
```

Source: [examples/full_workflow/python/main.py](../examples/full_workflow/python/main.py). Uses `ConfigEditor` for immutable-style struct updates.

---

## Running the Python test suite

```powershell
# PowerShell — full suite (310 tests)
.\.venv\Scripts\python.exe -m pytest python/tests/ -v

# Individual suites
.\.venv\Scripts\python.exe -m pytest python/tests/test_reader.py -v
.\.venv\Scripts\python.exe -m pytest python/tests/test_writer.py -v
.\.venv\Scripts\python.exe -m pytest python/tests/test_writer_roundtrip.py -v
.\.venv\Scripts\python.exe -m pytest python/tests/test_opcua_roundtrip.py -v
.\.venv\Scripts\python.exe -m pytest python/tests/test_builder.py -v
.\.venv\Scripts\python.exe -m pytest python/tests/test_cli.py -v
.\.venv\Scripts\python.exe -m pytest python/tests/test_schema.py -v
```

```bash
# Git Bash
.venv/Scripts/python.exe -m pytest python/tests/ -v
```

| Suite | Tests | What it covers |
|---|---|---|
| `test_reader.py` | — | Full parse of real AconityMIDI fixtures; all HDF5→model field mappings; OPCUA; `get_raw_group`; `include_binary` flag |
| `test_writer.py` | — | HDF5 write→re-parse roundtrip for machine name, hash, build plate, ClearBox scalars, schema validity |
| `test_writer_roundtrip.py` | — | Every scalar field; all 18 ClearBox scalar attributes; NaN↔None; axis configurations |
| `test_opcua_roundtrip.py` | — | OpcuaConfig full roundtrip: client, pipe, triggers (known fields + extra), write with OPCUA→read back |
| `test_builder.py` | — | `MockConfigBuilder`; `YamlConfigBuilder` roundtrip; `ConfigEditor` offset mutation |
| `test_cli.py` | — | `inspect`, `validate`, `export-json`, `write`, `build --mock`, `build --from-yaml`, `demo` |
| `test_schema.py` | — | Schema parses as JSON; draft 2020-12 meta-validation; all `required` constraints; golden file validation |
