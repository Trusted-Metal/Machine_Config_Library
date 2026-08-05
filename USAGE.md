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
  - [CLI reference](#cli-reference-python)
  - [Quickstart example](#quickstart-example)
  - [Full workflow example](#full-workflow-example)
  - [Running the Python test suite](#running-the-python-test-suite)
- [Node.js](#nodejs) ← *Phase 2 complete*
  - [Installation](#installation-1)
  - [What's usable today](#whats-usable-today)
  - [TypeScript interfaces](#typescript-interfaces)
  - [Use case 1 — Parse a machine config file](#use-case-1--parse-a-machine-config-file-1)
  - [Use case 2 — Export to canonical JSON](#use-case-2--export-to-canonical-json-1)
  - [Use case 4 — Write a config back to HDF5](#use-case-4--write-a-config-back-to-hdf5-1)
  - [Use case 6 — Generate a synthetic test config](#use-case-6--generate-a-synthetic-test-config-1)
  - [Use case 8 — Read OPCUA telemetry configuration](#use-case-8--read-opcua-telemetry-configuration-1)
  - [Use case 9 — Access ClearBox correction arrays](#use-case-9--access-clearbox-correction-arrays-1)
  - [Use case 10 — Validate a config against the schema](#use-case-10--validate-a-config-against-the-schema-1)
  - [CLI reference](#cli-reference)
  - [Quickstart example](#quickstart-example-1)
  - [Full workflow example](#full-workflow-example-1)
  - [Running the Node.js test suite](#running-the-nodejs-test-suite)
- [Rust](#rust) ← *Phase 3 complete*
  - [What's usable today](#whats-usable-today-1)
  - [Use case 1 — Parse a machine config file](#use-case-1--parse-a-machine-config-file-2)
  - [Use case 2 — Export to canonical JSON](#use-case-2--export-to-canonical-json-2)
  - [Use case 4 — Write a config back to HDF5](#use-case-4--write-a-config-back-to-hdf5-2)
  - [Use case 6 — Generate a synthetic test config](#use-case-6--generate-a-synthetic-test-config-2)
  - [Use case 8 — Read OPCUA telemetry configuration](#use-case-8--read-opcua-telemetry-configuration-1)
  - [Use case 9 — Access ClearBox correction arrays](#use-case-9--access-clearbox-correction-arrays-2)
  - [Use case 10 — Validate a config against the schema](#use-case-10--validate-a-config-against-the-schema-2)
  - [CLI reference](#cli-reference-1)
  - [Quickstart example](#quickstart-example-2)
  - [Full workflow example](#full-workflow-example-2)
  - [Running the Rust test suite](#running-the-rust-test-suite)
- [C++](#c) ← *Phase 4 complete*
  - [What's usable today](#whats-usable-today-2)
  - [Build and install](#build-and-install)
  - [Use case 1 — Parse a machine config file](#use-case-1--parse-a-machine-config-file-3)
  - [Use case 2 — Export to canonical JSON](#use-case-2--export-to-canonical-json-3)
  - [Use case 4 — Write a config back to HDF5](#use-case-4--write-a-config-back-to-hdf5-3)
  - [Use case 6 — Generate a synthetic test config](#use-case-6--generate-a-synthetic-test-config-3)
  - [Use case 8 — Read OPCUA telemetry configuration](#use-case-8--read-opcua-telemetry-configuration-2)
  - [Use case 9 — Access ClearBox correction arrays](#use-case-9--access-clearbox-correction-arrays-3)
  - [Use case 10 — Validate a config against the JSON schema](#use-case-10--validate-a-config-against-the-json-schema)
  - [CLI reference](#cli-reference-cpp)
  - [Quickstart example](#quickstart-example-3)
  - [Full workflow example](#full-workflow-example-3)
  - [Running the C++ test suite](#running-the-c-test-suite)
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

### Language feature matrix

The table below summarises which features are available in each language. Python is the
reference implementation; Rust and Node.js cover the core read/write/hash/OPC-UA path and are
intended as integration-layer libraries. The Python-only items (`YamlConfigBuilder`,
`ConfigEditor`, `config_from_dict`) are workflow conveniences that have no equivalent in the
other language SDKs.

| Feature | Python | Rust | Node.js | C++ |
|---|:---:|:---:|:---:|:---:|
| HDF5 reader (`parse`) | ✅ | ✅ | ✅ | ✅ |
| HDF5 writer (`write`) | ✅ | ✅ | ✅ | ✅ |
| `MockConfigBuilder` | ✅ | ✅ | ✅ | ✅ |
| Schema validation | ✅ | ✅ *(serde)* | ✅ *(Ajv)* | ✅ *(pboettch)* |
| CLI: `export-json` | ✅ | ✅ | ✅ | ✅ |
| CLI: `write-hdf5` | ✅ | ✅ | ✅ | ✅ |
| CLI: `correction-hash` | ✅ | ✅ | ✅ | ✅ |
| CLI: `copy-hdf5` | ✅ | ✅ | ✅ | ✅ |
| OPC-UA config in model | ✅ | ✅ | ✅ | ✅ |
| `get_raw_group()` | ✅ | ✅ | ✅ | ✅ |
| Binary data accessors | ✅ *(numpy)* | ✅ *(ndarray)* | ✅ *(Float64Array)* | ✅ *(vector<double>)* |
| `parseWithBinary()` | ✅ | ✅ | ❌ | ✅ |
| `YamlConfigBuilder` | ✅ | ❌ | ❌ | ❌ |
| `ConfigEditor` | ✅ | ❌ | ❌ | ❌ |
| `config_from_dict` | ✅ | ❌ | ❌ | ❌ |
| CLI: `inspect` / `validate` / `demo` | ✅ | ❌ | ❌ | ❌ |

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

### Quickstart example

Run the end-to-end quickstart from the repo root:

```powershell
# PowerShell
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

The script resolves paths relative to the repo root automatically, so it runs correctly from any working directory.

---

### Full workflow example

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

The source lives at [examples/full_workflow/python/main.py](examples/full_workflow/python/main.py).
Uses `ConfigEditor` for immutable-style struct updates.

---

### Running the Python test suite

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
# Git Bash — full suite (310 tests)
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

*Phase 2 complete — `models.ts`, `reader.ts`, `writer.ts`, `builder.ts`, `schema.ts`, all three CLI subcommands (`export-json`/`write-hdf5`/`correction-hash`), `nodejs.yml` CI, and the hello-world quickstart all implemented (Phases 2.1, 2.4/1–2.4/6).*

The package lives in `nodejs/` and is built with TypeScript (ES2022, NodeNext modules).
It uses [`h5wasm`](https://github.com/usnistgov/h5wasm) (HDF5 compiled to WebAssembly by NIST)
so there is no native compilation step on any platform.

### Installation

```bash
# From the nodejs/ directory
cd nodejs
npm install
npm run build      # compile TypeScript → dist/
```

### What's usable today

`nodejs/src/models.ts` defines the full TypeScript interface tree — `MachineConfig`, `OpticalTrain`,
`Scanner`, `AxisConfig`, `LightSource`, `Collimator`, `ScannerCard`, `ClearBox`,
`ScanFieldCorrectionFile`, `OpcuaConfig` and friends — mirroring Python models with snake_case
field names throughout.

`nodejs/src/reader.ts` implements `MachineConfigReader`. Its JSON output has been cross-checked
against `fixtures/reference_output.json` for all three canonical fixtures.

`nodejs/src/schema.ts` exposes `validate(data): string[]` using Ajv (JSON Schema draft 2020-12).
An empty array means valid.

`nodejs/src/writer.ts` implements `MachineConfigWriter`, the exact inverse of `reader.ts`. A
write-then-read roundtrip has been verified against all three canonical fixtures (reference,
OPC-UA, synthetic 2-laser) and cross-checked live against the Python and Rust writers via
`tools/cross_check.py` Phase 3.

`reader.ts` also exposes `getCorrectionData(trainIndex)` / `getInverseCorrectionData(trainIndex)`,
returning the raw NaN-preserving correction grid as a flat `Float64Array` + shape — the same
data the JSON path null-converts for safety, but here left untouched for exact byte hashing.
These mirror Python's and Rust's identically-named reader methods.

`nodejs/src/cli.ts` implements all three subcommands: `export-json`, `write-hdf5`, and
`correction-hash <file> --train <n> [--inverse]` (SHA-256 of the flat little-endian float64
correction grid, via `node:crypto`). All three have been cross-checked live against Python and
Rust for all fixtures/trains/forward-or-inverse combinations — identical digests in every case.

`nodejs/src/builder.ts` implements `MockConfigBuilder`, mirroring Python's and Rust's builders
field-for-field: same defaults (2 lasers, 250×250×20 mm build plate), same per-train geometry,
and the same Gaussian correction-grid formula (peak 2.0 at centre; inverse grid = forward × 0.9).
See Use case 6 below.

### TypeScript interfaces

All types are fully exported from the package root:

```typescript
import type { MachineConfig, OpticalTrain, ClearBox } from './dist/index.js';
```

### Use case 1 — Parse a machine config file

```typescript
import { MachineConfigReader } from './dist/index.js';

const reader = new MachineConfigReader('fixtures/reference_config.h5');
const config = await reader.parse();

console.log(config.meta.machine_name);           // "TM-LPBF-02: AconityMIDI+_OG"
console.log(config.meta.configuration_hash);     // 64-character hex string
console.log(config.machine.build_plate_x);       // 250

for (const [i, train] of config.optical_trains.entries()) {
  const s = train.scanner;
  console.log(
    `Train ${i + 1}: WD=${s.working_distance} ${s.working_distance_unit}  ` +
    `offset=(${s.scan_head_offset_x}, ${s.scan_head_offset_y}) mm`
  );
}

// OPCUA — populated only when the HDF5 file has an OPCUA group
if (config.opcua) {
  console.log(config.opcua.client.server_url);
  for (const [name, trigger] of Object.entries(config.opcua.triggers)) {
    console.log(`${name}: signal=${trigger.signal}`);
  }
}
```

### Use case 2 — Export to canonical JSON

By default, `toJson()` produces metadata + scalar fields only. Binary datasets (ClearBox
correction arrays and raw `.fc3` bytes) are excluded unless `includeBinary: true` is passed.

```typescript
import { MachineConfigReader } from './dist/index.js';
import { writeFileSync } from 'node:fs';

const reader = new MachineConfigReader('fixtures/reference_config.h5');

// Default: metadata + scalars only (~13 KB)
const json = await reader.toJson({ indent: 2 });
writeFileSync('output.json', json, 'utf-8');

// Compact (no indentation)
const compact = await reader.toJson({ indent: 0 });

// Include correction grids and raw .fc3 bytes (large output, ~14 MB for a 2-laser config)
const full = await reader.toJson({ includeBinary: true, indent: 2 });
writeFileSync('output_full.json', full, 'utf-8');
```

> **Accessing binary data without JSON**: use `parse({ includeBinary: true })` to get
> `optional_components.clearbox.correction_data` (a `(number | null)[][][]` array of shape
> 257×257×2) directly in the model. `null` cells are out-of-field points (were NaN in HDF5).

### Use case 4 — Write a config back to HDF5

```typescript
import { MachineConfigReader, MachineConfigWriter } from './dist/index.js';

const config = await new MachineConfigReader('original.h5').parse();

// MachineConfig is a plain TS object — mutate a field with a spread, or in place.
config.meta.export_date = '2026-07-30T00:00:00Z';

await new MachineConfigWriter(config).write('copy.h5');

// Verify the roundtrip
const reread = await new MachineConfigReader('copy.h5').parse();
console.log(reread.meta.configuration_hash === config.meta.configuration_hash);
```

> **Binary data in the writer**: if `optional_components.clearbox.correction_data` /
> `inverse_correction_data` are absent (i.e. the config was parsed without
> `includeBinary: true`), the writer writes zero-filled `(257, 257, 2)` float64 datasets in
> their place — identical behaviour to the Python and Rust writers. Parse with
> `{ includeBinary: true }` first to preserve the original correction grids across a roundtrip.

---

### Use case 6 — Generate a synthetic test config

`MockConfigBuilder` creates structurally valid `.h5` files for testing without requiring access
to real machine hardware — mirroring Python's `MockConfigBuilder` and Rust's `MockConfigBuilder`
(same defaults, same per-train geometry, same Gaussian correction-grid formula).

```typescript
import { MockConfigBuilder, MachineConfigReader } from './dist/index.js';

// 2-laser config with ClearBox (default)
await new MockConfigBuilder().save('test_config.h5');

// Customise before saving
await new MockConfigBuilder({
  nLasers: 1,
  machineName: 'TestMachine',
  buildPlateX: 400.0,
  includeClearbox: false,
}).save('custom_config.h5');

// Build into memory without writing
const config = new MockConfigBuilder().build();
console.log(config.optical_trains.length);    // 2
console.log(config.meta.machine_name);        // "MockMachine"

// Verify the correction grid that was written
const reader = new MachineConfigReader('test_config.h5');
const cd = await reader.getCorrectionData(0);   // { data: Float64Array, shape: [257, 257, 2] }
console.log(cd.shape);
console.log('Peak correction:', cd.data[(128 * 257 + 128) * 2].toFixed(4)); // ≈ 2.0000 (Gaussian peak)
```

> **Defaults**: `nLasers: 2`, `buildPlateX/Y: 250`, `buildPlateZ: 20`, `includeClearbox: true`
> (gates both ClearBox *and* ScanFieldCorrectionFile together, per train), `machineName:
> "MockMachine"`, `manufacturer: "MockCo"`, `model: "MockMIDI+"`, `serialNumber: "MOCK-001"`.
> `machine.id` and `meta.export_date` are fixed constants rather than randomly generated, so
> builder output is reproducible run to run.

---

### Use case 8 — Read OPCUA telemetry configuration

OPCUA data lives in a separate `OPCUA` group that is absent in most files and is outside the
canonical schema. `parse()` returns `config.opcua` (populated when the file has an `OPCUA`
group, absent otherwise). For ad-hoc inspection of any HDF5 path — whether it is an OPCUA
sub-group or a scanner sub-axis — use `getRawGroup()`:

```typescript
import { MachineConfigReader } from './dist/index.js';

// Typed path through the model (best for known OPCUA fields)
const config = await new MachineConfigReader('fixtures/reference_config_opcua.h5').parse();
if (config.opcua) {
  console.log(config.opcua.client.server_url);    // "opc.tcp://..."
  console.log(config.opcua.triggers_enabled);      // true
  for (const [name, t] of Object.entries(config.opcua.triggers)) {
    console.log(name, t.signal, t.subsystem);
  }
}

// Raw attribute map for any HDF5 path — returns {} (not an error) if absent
const reader = new MachineConfigReader('fixtures/reference_config_opcua.h5');
const clientAttrs = await reader.getRawGroup('OPCUA/Client');
console.log(clientAttrs['Server_URL']);  // e.g. "opc.tcp://192.168.1.100:4840"
console.log(clientAttrs['Auth_Mode']);   // e.g. "UsernamePassword"

// Any path works — returns {} rather than throwing when absent:
const missing = await reader.getRawGroup('does/not/exist'); // {}
```

> `getRawGroup(path)` mirrors Python's `reader.get_raw_group(path)` and Rust's
> `reader.get_raw_group(path)` exactly: same empty-map-for-missing contract, same attribute
> value semantics. Useful for scanner sub-axes
> (`Machine/Optical_Trains/.../Scanner/X_Axis`) and any other non-schema group.

---

### Use case 9 — Access ClearBox correction arrays

`parse({ includeBinary: true })` gives you the JSON-safe, null-converted nested array (see the
note under Use case 2). For numerical work — or anything that must match Python/Rust bit-for-bit,
such as hashing — read the raw grid directly instead:

```typescript
import { MachineConfigReader } from './dist/index.js';

const reader = new MachineConfigReader('fixtures/reference_config.h5');

// Flat, row-major, NaN preserved (not JSON-safe) — shape [257, 257, 2]
const cd = await reader.getCorrectionData(0);          // train 0, forward grid
const icd = await reader.getInverseCorrectionData(0);  // train 0, inverse grid

console.log(cd.data instanceof Float64Array, cd.shape); // true [257, 257, 2]

// Sample the centre cell manually (row-major: offset = (i*257 + j)*2 + k)
const centreX = cd.data[(128 * 257 + 128) * 2 + 0];
console.log('Centre correction X:', centreX);
```

> **Why a separate accessor from `parse()`**: `parse({ includeBinary: true })` maps `NaN → null`
> so the result is valid JSON — but that conversion is lossy for exact byte comparisons.
> `getCorrectionData`/`getInverseCorrectionData` skip it entirely, returning exactly what h5wasm
> read off the dataset. This is what the `correction-hash` CLI subcommand uses internally.

---

### Use case 10 — Validate a config against the schema

```typescript
import { MachineConfigReader } from './dist/index.js';
import { validate } from './dist/schema.js';

const config = await new MachineConfigReader('fixtures/reference_config.h5').parse();
const errors = validate(config);
if (errors.length === 0) {
  console.log('Schema valid.');
} else {
  console.error('Validation errors:', errors);
}
```

### CLI reference

```bash
# From the repo root — build first if not already done
cd nodejs && npm run build && cd ..

# Export HDF5 → JSON to stdout (fully implemented)
node nodejs/dist/cli.js export-json fixtures/reference_config.h5

# Export to a file
node nodejs/dist/cli.js export-json fixtures/reference_config.h5 > output.json

# Write HDF5 from JSON (fully implemented)
node nodejs/dist/cli.js write-hdf5 config.json output.h5

# SHA-256 of the forward correction grid, train 0 (fully implemented)
node nodejs/dist/cli.js correction-hash fixtures/reference_config.h5 --train 0

# SHA-256 of the inverse correction grid, train 1
node nodejs/dist/cli.js correction-hash fixtures/reference_config.h5 --train 1 --inverse
```

> `correction-hash` hashes the grid as flat little-endian float64 bytes, so its output is
> directly comparable with `machine-config correction-hash` (Python) and `machine-config-cli
> correction-hash` (Rust) for the same file/train/direction — verified byte-identical in CI.

---

### Quickstart example

Build the library first (the quickstart imports `nodejs/dist/index.js`, the same compiled
output the CLI uses):

```bash
# Git Bash / PowerShell — from the repo root
cd nodejs && npm run build && cd ..
node examples/quickstart/nodejs/main.mjs
```

Expected output (same shape as Python and Rust):

```
=== Machine Config Quickstart ===

Machine name   : TM-LPBF-02: AconityMIDI+_OG
Optical trains : 2
Working dist   : 670 mm   (train 0)
Correction grid: [257, 257, 2]   (train 0)

Written to     : <tmp>.h5

PASS
```

The script resolves paths relative to the repo root automatically, so it runs correctly from any working directory.

---

### Full workflow example

**Scenario:** same calibration adjustment as the Python full workflow — load config, apply new
scanner offsets via direct field mutation (no editor helper needed in TypeScript), write,
and verify.

```bash
# Git Bash / PowerShell — from the repo root
node examples/full_workflow/nodejs/main.mjs
```

Expected output:

```
=== Full Workflow: Calibration Adjustment ===

Machine : TM-LPBF-02: AconityMIDI+_OG
Trains  : 2

Before calibration:
  Train 1  offset x=-87.5, y=23.5
           correction grid [257, 257, 2]
  Train 2  offset x=86.074, y=-21.695
           correction grid [257, 257, 2]

Written to : <tmp>.h5

After calibration:
  Train 1  offset x=-91.5, y=24
  Train 2  offset x=91.5, y=-24

PASS
```

The source lives at [examples/full_workflow/nodejs/main.mjs](examples/full_workflow/nodejs/main.mjs).

---

### Running the Node.js test suite

```powershell
# PowerShell — from the nodejs/ directory
cd nodejs
npm test           # 127 tests: 82 reader + 6 schema + 21 writer + 18 builder

# From repo root
cd nodejs ; npm test
```

```bash
# Git Bash
cd nodejs
npm test
```

*Use cases 3, 5, and 7 (reconstruct-from-JSON, edit-and-save via a `ConfigEditor`-equivalent, and
building from a YAML spec) have no Node.js port — `ConfigEditor` and `YamlConfigBuilder` are
Python-only conveniences, not part of the six-step §2.4 vertical slice (and Rust doesn't have
them either — see the Rust section's own note). Everything else, including `MockConfigBuilder`
(Use case 6) and the quickstart, is implemented and documented above.*

---

## Rust

*Phase 3 complete — data models, HDF5 reader, writer, `MockConfigBuilder`, CLI, and integration tests all implemented and verified.*

The crate lives in `rust/` (package `machine-config`, library `machine_config`). It has no system dependencies — `cargo build` compiles `libhdf5` from source on first run (see [IMPLEMENTATION_PLAN.md §3.2](IMPLEMENTATION_PLAN.md#32--platform--dependency-decision)).

### What's usable today

`rust/src/models.rs` defines the full data model — `MachineConfig` and its complete field tree (`Machine`, `OpticalTrain`, `Scanner`, `AxisConfig`, `LightSource`, `Collimator`, `ScannerCard`, `ClearBox`, `ScanFieldCorrectionFile`, `OpcuaConfig` and friends) — mirroring `python/src/machine_config/models.py`, with `#[derive(Serialize, Deserialize)]` so any value round-trips through `serde_json`.

`rust/src/reader.rs` implements `MachineConfigReader`, mirroring Python's `MachineConfigReader`. Its JSON output has been verified to deep-equal `fixtures/reference_output.json` (the Python golden file) for the reference AconityMIDI fixture.

`rust/src/writer.rs` implements `MachineConfigWriter`, mirroring Python's `MachineConfigWriter`. It is the exact inverse of the reader: every attribute written matches what the reader expects to find, and a write-then-read roundtrip preserves all scalar fields and the SHA-256 of the ClearBox correction grids.

`rust/src/builder.rs` implements `MockConfigBuilder`, which generates structurally valid synthetic `.h5` files for testing. It mirrors Python's `MockConfigBuilder` — same group/attribute layout, non-zero Gaussian correction grids, deterministic values.

`rust/src/main.rs` is the `machine-config-cli` binary. It exposes three subcommands — `export-json` (HDF5 → JSON to stdout), `write-hdf5` (JSON file → HDF5 file), and `correction-hash` (SHA-256 of a flat little-endian float64 correction grid) — mirroring the Python and Node.js CLIs. All three subcommands are verified cross-language identical in CI.

### Use case 1 — Parse a machine config file

```rust
use machine_config::reader::MachineConfigReader;

let reader = MachineConfigReader::open("fixtures/reference_config.h5")?;
let config = reader.parse()?;   // scalars + metadata only — see Use case 2

println!("{}", config.meta.machine_name);        // "TM-LPBF-02: AconityMIDI+_OG"
println!("{}", config.meta.configuration_hash);  // 64-character hex string

println!(
    "{} x {} x {} {}",
    config.machine.build_plate_x.unwrap(),
    config.machine.build_plate_y.unwrap(),
    config.machine.build_plate_z.unwrap(),
    config.machine.build_plate_x_unit.as_deref().unwrap()
);

for (i, train) in config.optical_trains.iter().enumerate() {
    let s = &train.scanner;
    println!(
        "Train {}: WD={:?} {:?}  axis_config={:?}",
        i + 1, s.working_distance, s.working_distance_unit, s.axis_configuration
    );
    // x_axis/y_axis are always present; z_axis is Some for "3D"/"3D+Focus",
    // focus is Some only for "3D+Focus".
    if let Some(z) = &s.z_axis {
        println!("  Z bit_resolution: {:?} {:?}", z.actual_bit_resolution, z.actual_bit_resolution_unit);
    }
}
```

---

### Use case 2 — Export to canonical JSON

By default, `parse()`/`to_json()` include only scalar and metadata fields. The ClearBox correction grids and raw `.fc3` bytes are intentionally left out — use `parse_with_binary()` / `to_json(_, include_binary: true)` or the dedicated accessors below when you need them.

```rust
use machine_config::reader::MachineConfigReader;

let reader = MachineConfigReader::open("fixtures/reference_config.h5")?;

// Default: metadata + scalars only, pretty-printed
let json = reader.to_json(true, false)?;
std::fs::write("output.json", json)?;

// With binary data: adds correction_data / inverse_correction_data (257×257×2
// nested arrays) to every ClearBox, and raw_bytes to every scan field
// correction file. Output is large (tens of MB for a 2-laser config).
let full_json = reader.to_json(true, true)?;
std::fs::write("output_full.json", full_json)?;
```

> **Accessing binary data without JSON**: `reader.get_correction_data(train_index)` and `reader.get_inverse_correction_data(train_index)` return an `ndarray::Array3<f64>` of shape `(257, 257, 2)`; `reader.get_scan_field_correction_bytes(train_index)` returns the raw `.fc3` bytes as `Vec<u8>`. These are the recommended paths for numerical work — they read directly from HDF5 without going through the model at all.

---

### Use case 4 — Write a config back to HDF5

`MachineConfigWriter` serialises any `MachineConfig` back to a machine-config-schema-compatible `.h5` file. The output is structurally identical to what the machine software exports, so it can be read back by both the Rust and Python readers.

```rust
use machine_config::reader::MachineConfigReader;
use machine_config::writer::MachineConfigWriter;

// Round-trip: read, mutate, write, re-read
let reader = MachineConfigReader::open("fixtures/reference_config.h5")?;
let mut config = reader.parse()?;

// Modify a field (e.g. update the export timestamp)
config.meta.export_date = "2026-07-28T00:00:00Z".to_owned();

MachineConfigWriter::new(&config).write("output.h5")?;

// Verify the roundtrip
let reread = MachineConfigReader::open("output.h5")?.parse()?;
assert_eq!(config.meta.machine_name, reread.meta.machine_name);
assert_eq!(config.meta.export_date,  reread.meta.export_date);
```

> **Binary data in the writer**: if `correction_data` / `inverse_correction_data` are `None` in the model (i.e. the config was parsed with `parse()`, not `parse_with_binary()`), the writer writes zero-filled `(257, 257, 2)` float64 datasets in their place. To preserve the original binary data across a roundtrip, either use `parse_with_binary()` before writing, or use `get_correction_data()` to obtain the arrays and re-attach them to the model before writing.

---

### Use case 6 — Generate a synthetic test config

`MockConfigBuilder` creates structurally valid `.h5` files for integration tests without requiring access to real machine hardware.

```rust
use machine_config::builder::MockConfigBuilder;
use machine_config::reader::MachineConfigReader;

// 2-laser config with ClearBox (default)
MockConfigBuilder::new(2).save("test_config.h5")?;

// Customise before saving
let mut b = MockConfigBuilder::new(1);
b.machine_name = "TestMachine".to_owned();
b.build_plate_x = 400.0;
b.include_clearbox = false;
b.save("custom_config.h5")?;

// Build into memory without writing
let config = MockConfigBuilder::new(2).build();
assert_eq!(config.optical_trains.len(), 2);
assert_eq!(config.meta.machine_name, "MockMachine");

// Verify the correction grid that was written
let reader = MachineConfigReader::open("test_config.h5")?;
let arr = reader.get_correction_data(0)?;  // Array3<f64>, shape (257, 257, 2)
assert_eq!(arr.shape(), &[257, 257, 2]);
println!("Peak correction: {:.4}", arr[[128, 128, 0]]);  // ≈ 2.0 (Gaussian peak)
```

---

### Use case 8 — Read OPCUA telemetry configuration

OPCUA data is outside the canonical model. `parse()` returns `config.opcua: Option<OpcuaConfig>` (populated only when the file has an `OPCUA` group), or use `get_raw_group()` for arbitrary non-schema paths:

```rust
use machine_config::reader::MachineConfigReader;

let reader = MachineConfigReader::open("fixtures/reference_config_opcua.h5")?;
let config = reader.parse()?;

if let Some(opcua) = &config.opcua {
    println!("{}", opcua.client.server_url);
    println!("{:?}", opcua.triggers_enabled);
    for (name, trigger) in &opcua.triggers {
        println!("{name}: signal={:?} subsystem={:?}", trigger.signal, trigger.subsystem);
    }
}

// Or fetch raw attributes for any path — returns an empty map, never an error,
// if the path doesn't exist:
let client_attrs = reader.get_raw_group("OPCUA/Client")?;
println!("{:?}", client_attrs.get("Server_URL"));
```

---

### Use case 9 — Access ClearBox correction arrays

```rust
use machine_config::reader::MachineConfigReader;

let reader = MachineConfigReader::open("fixtures/reference_config.h5")?;
let config = reader.parse_with_binary()?;

if let Some(cb) = &config.optical_trains[0].clearbox {
    let data = cb.correction_data.as_ref().unwrap(); // Vec<Vec<Vec<Option<f64>>>>, shape 257x257x2
    assert_eq!(data.len(), 257);
    assert_eq!(data[0].len(), 257);
    assert_eq!(data[0][0].len(), 2);
    // None means the point is outside the correction field (was NaN in HDF5).
    let centre = &data[128][128];
    println!("Centre correction: x={:?}, y={:?}", centre[0], centre[1]);
}

// For bulk numerical work, use the ndarray accessor directly:
let arr = reader.get_correction_data(0)?;  // Array3<f64>, shape (257, 257, 2)
println!("Non-finite cells: {}", arr.iter().filter(|v| !v.is_finite()).count());
```

---

### Use case 10 — Validate a config against the schema

Rust has no standalone `validate()` function. Schema compliance is enforced implicitly by
`serde_json` at parse time: `parse()` returns `Err` if a required field is missing or has the
wrong type. All fields that can be absent are modelled as `Option<T>`; a present value of the
wrong type is a deserialization error.

```rust
use machine_config::reader::MachineConfigReader;
use machine_config::writer::MachineConfigWriter;
use machine_config::builder::MockConfigBuilder;

// A well-formed file always parses without error.
let config = MachineConfigReader::open("fixtures/reference_config.h5")?.parse()?;
assert_eq!(config.meta.schema_version, "v1");
assert_eq!(config.meta.configuration_hash.len(), 64);

// MockConfigBuilder output satisfies every structural constraint.
let mock = MockConfigBuilder::new(2).build();
assert_eq!(mock.meta.schema_version, "v1");
assert!(mock.meta.configuration_hash.chars().all(|c| c == '0'));

// For explicit JSON Schema validation of export-json output, use Python or Node.js:
// python tools/cross_check.py --langs rust  (validates Rust output in CI)
```

> `tools/cross_check.py` (Phase 1) runs Python's `jsonschema` against every language's
> `export-json` output for every fixture in CI. This is the authoritative schema compliance
> gate for Rust. For a runtime `validate()` function, see the Python or C++ implementations.

---

### CLI reference

`machine-config-cli` exposes the same three subcommands as the Python and Node.js CLIs:

```powershell
# PowerShell — build first
cargo build --release --manifest-path rust/Cargo.toml

# Export HDF5 → JSON to stdout
.\rust\target\release\machine-config-cli.exe export-json fixtures\reference_config.h5

# With binary fields (correction grids + raw .fc3 bytes)
.\rust\target\release\machine-config-cli.exe export-json fixtures\reference_config.h5 --include-binary

# Write HDF5 from JSON
.\rust\target\release\machine-config-cli.exe write-hdf5 config.json output.h5

# SHA-256 of the forward correction grid, train 0
.\rust\target\release\machine-config-cli.exe correction-hash fixtures\reference_config.h5 --train 0

# SHA-256 of the inverse correction grid, train 1
.\rust\target\release\machine-config-cli.exe correction-hash fixtures\reference_config.h5 --train 1 --inverse
```

```bash
# Git Bash
cargo build --release --manifest-path rust/Cargo.toml
./rust/target/release/machine-config-cli export-json fixtures/reference_config.h5
./rust/target/release/machine-config-cli export-json fixtures/reference_config.h5 --include-binary > output_full.json
./rust/target/release/machine-config-cli write-hdf5 config.json output.h5
./rust/target/release/machine-config-cli correction-hash fixtures/reference_config.h5 --train 0
./rust/target/release/machine-config-cli correction-hash fixtures/reference_config.h5 --train 1 --inverse
```

> Errors go to stderr; stdout is always valid JSON (for `export-json`) or empty (for
> `write-hdf5` / `correction-hash`) on success. `correction-hash` output is byte-identical to
> Python's and Node.js's for the same file/train/direction — verified in CI.
>
> Use cases 3, 5, and 7 (`ConfigEditor`, YAML-spec builder, reconstruct-from-JSON) have no Rust port; `ConfigEditor` and `YamlConfigBuilder` are Python-only conveniences, not part of the crate.

---

### Quickstart example

Run the end-to-end quickstart from the **repo root**:

```powershell
# PowerShell — from repo root
cargo run --example quickstart --manifest-path rust/Cargo.toml
```

```bash
# Git Bash — from repo root
cargo run --example quickstart --manifest-path rust/Cargo.toml
```

Expected output:

```
=== Machine Config Quickstart ===

Machine name   : TM-LPBF-02: AconityMIDI+_OG
Optical trains : 2
Working dist   : 670 mm   (train 0)
Correction grid: [257, 257, 2]   (train 0)

Written to     : <tmp>.h5

PASS
```

The source lives at [rust/examples/quickstart.rs](rust/examples/quickstart.rs) (a standard Cargo example) with a reference copy at [examples/quickstart/rust/main.rs](examples/quickstart/rust/main.rs).

---

### Full workflow example

**Scenario:** same calibration adjustment — load config, apply new scanner offsets via direct
struct mutation, write, and verify correction data survived the round-trip.

```powershell
# PowerShell — from repo root
cargo run --example full_workflow --manifest-path rust/Cargo.toml
```

```bash
# Git Bash — from repo root
cargo run --example full_workflow --manifest-path rust/Cargo.toml
```

Expected output:

```
=== Full Workflow: Calibration Adjustment ===

Machine : TM-LPBF-02: AconityMIDI+_OG
Trains  : 2

Before calibration:
  Train 1  offset x=Some(-87.5), y=Some(23.5)
           correction grid [257, 257, 2]
  Train 2  offset x=Some(86.074), y=Some(-21.695)
           correction grid [257, 257, 2]

Written to : <tmp>.h5

After calibration:
  Train 1  offset x=Some(-91.5), y=Some(24.0)
  Train 2  offset x=Some(91.5), y=Some(-24.0)

PASS
```

The source lives at [rust/examples/full_workflow.rs](rust/examples/full_workflow.rs) with a
reference pointer at [examples/full_workflow/rust/main.rs](examples/full_workflow/rust/main.rs).

---

### Running the Rust test suite

```powershell
# PowerShell — from repo root
cd rust
cargo test --lib        # 46 unit tests: golden-file, SHA-256 writer roundtrip, builder roundtrip
cargo test              # 46 unit + 14 integration = 60 tests total
cargo bench             # criterion benchmarks: open_and_parse, to_json_pretty, open_and_parse_with_binary
cargo build --all-targets
```

```bash
# Git Bash
cd rust
cargo test --lib
cargo test
cargo bench
cargo build --all-targets
```

---

## C++

*Phase 4 complete — header-only reader/writer/builder/schema, four CLI subcommands, 63 Catch2 tests, and example programs all implemented and verified.*

The library lives in `cpp/include/machine_config/` and is **header-only**. Any C++17 project
that links HighFive (HDF5 wrapper), nlohmann/json, and HDF5 itself can include the headers
directly with no compilation step.

### What's usable today

`cpp/include/machine_config/models.hpp` defines the full data model with `nlohmann/json`
ADL pairs: `MachineConfig`, `OpticalTrain`, `Scanner`, `AxisConfig`, `LightSource`,
`Collimator`, `ScannerCard`, `ClearBox`, `ScanFieldCorrectionFile`, `OpcuaConfig` and all
sub-types. All fields use `std::optional<T>` to represent nullable values; correction
grids use `Grid3D` (nested `optional<double>`).

`cpp/include/machine_config/reader.hpp` implements `MachineConfigReader`. `parse()` returns a
`MachineConfig` with scalars and metadata; `parseWithBinary()` also populates correction grids
and raw `.fc3` bytes. `toJson()` produces canonical JSON identical to every other language.
Separate accessors — `getCorrectionData()`, `getInverseCorrectionData()`,
`getScanFieldCorrectionBytes()` — read binary data directly from HDF5.

`cpp/include/machine_config/writer.hpp` implements `MachineConfigWriter`, the exact inverse of
the reader. All scalar fields, correction grids, raw `.fc3` bytes, and OPCUA attributes are
written back verbatim. JSON output from a write-then-read roundtrip is verified identical to the
original by the cross-language test suite.

`cpp/src/main.cpp` is the `machine_config_cli` binary. It exposes four subcommands —
`export-json`, `write-hdf5`, `copy-hdf5`, and `correction-hash` — all cross-checked against
Python, Rust, and Node.js.

### Build and install

**Prerequisites**: CMake ≥ 3.20, a C++17 compiler (MSVC 19+, GCC 11+, Clang 14+), and HDF5
≥ 1.12 (vcpkg on Windows, build from source on Linux — see [cpp.yml](.github/workflows/cpp.yml)).

```powershell
# PowerShell — Windows (vcpkg provides HDF5)
cmake -S cpp -B cpp/build -DCMAKE_BUILD_TYPE=Release `
  "-DCMAKE_TOOLCHAIN_FILE=$env:VCPKG_INSTALLATION_ROOT\scripts\buildsystems\vcpkg.cmake"
cmake --build cpp/build --config Release
```

```bash
# Linux — build HDF5 1.14.6 from source first (see cpp.yml), or:
apt-get install -y cmake ninja-build  # Ubuntu 24.04+, HDF5 via source build
cmake -S cpp -B cpp/build -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_PREFIX_PATH=/usr/local/hdf5
cmake --build cpp/build
```

After building, the CLI is at:
- Windows: `cpp/build/Release/machine_config_cli.exe`
- Linux:   `cpp/build/machine_config_cli`

To use the library in your own CMake project, add `cpp/include` to your include path and link
HighFive and HDF5. All types live in the `machine_config` namespace.

---

### Use case 1 — Parse a machine config file

```cpp
#include "machine_config/reader.hpp"
using namespace machine_config;

MachineConfigReader reader{"fixtures/reference_config.h5"};
auto config = reader.parse();   // scalars + metadata only

std::cout << config.meta.machine_name << "\n";        // TM-LPBF-02: AconityMIDI+_OG
std::cout << config.meta.configuration_hash << "\n"; // 64-char hex
std::cout << config.optical_trains.size() << "\n";   // 2

for (size_t i = 0; i < config.optical_trains.size(); ++i) {
    const auto& s = config.optical_trains[i].scanner;
    std::cout << "Train " << (i + 1)
              << ": WD=" << s.working_distance.value_or(0.0)
              << " " << s.working_distance_unit.value_or("")
              << "  offset=(" << s.scan_head_offset_x.value_or(0.0)
              << ", " << s.scan_head_offset_y.value_or(0.0) << ")\n";
}

// OPCUA — populated only when the file has an OPCUA group
if (config.opcua) {
    std::cout << config.opcua->client.server_url << "\n";
}
```

---

### Use case 2 — Export to canonical JSON

```cpp
#include "machine_config/reader.hpp"
using namespace machine_config;

MachineConfigReader reader{"fixtures/reference_config.h5"};

// Scalar + metadata only (default), 2-space indent
std::string json = reader.toJson();
std::cout << json << "\n";

// Write to a file
std::ofstream{"output.json"} << json;

// Compact (no indentation)
std::string compact = reader.toJson(/*indent=*/0);
```

> **Binary data in JSON**: `toJson()` does not include correction grids or raw `.fc3` bytes
> in the JSON output (the parameter exists as a placeholder for a future release). Use the
> dedicated accessors — `getCorrectionData()` / `getInverseCorrectionData()` /
> `getScanFieldCorrectionBytes()` — for binary data.

---

### Use case 4 — Write a config back to HDF5

```cpp
#include "machine_config/reader.hpp"
#include "machine_config/writer.hpp"
using namespace machine_config;

// Round-trip: read, mutate a scalar, write, re-read
MachineConfigReader reader{"fixtures/reference_config.h5"};
auto config = reader.parse();

// Directly mutate any field (all fields are plain value types)
config.optical_trains[0].scanner.scan_head_offset_x = -91.5;
config.optical_trains[0].scanner.scan_head_offset_y =  24.0;

MachineConfigWriter{config}.write("output.h5");

// Verify the roundtrip
auto reread = MachineConfigReader{"output.h5"}.parse();
assert(config.meta.machine_name == reread.meta.machine_name);
assert(reread.optical_trains[0].scanner.scan_head_offset_x == -91.5);
```

> **Binary data in the writer**: if `correction_data` / `inverse_correction_data` are `nullopt`
> (i.e. the config was parsed with `parse()`, not `parseWithBinary()`), the writer writes
> zero-filled `(257, 257, 2)` float64 datasets in their place — identical behaviour to Python
> and Rust. Use `parseWithBinary()` before writing to preserve the original correction grids.

---

### Use case 6 — Generate a synthetic test config

`MockConfigBuilder` creates structurally valid `.h5` files for testing without requiring access
to real machine hardware. It mirrors Python's and Rust's `MockConfigBuilder` — same defaults,
same per-train geometry, same Gaussian correction grids.

```cpp
#include "machine_config/builder.hpp"
#include "machine_config/reader.hpp"
using namespace machine_config;

// 2-laser config with ClearBox (default)
MockConfigBuilder{}.save("test_config.h5");

// Customise before saving
MockConfigBuilder b;
b.laser_count     = 1;
b.machine_name    = "TestMachine";
b.build_plate_x   = 400.0;
b.include_clearbox = false;
b.save("custom_config.h5");

// Build into memory without writing
auto config = MockConfigBuilder{}.build();
assert(config.optical_trains.size() == 2);
assert(config.meta.machine_name == "MockMachine");

// Verify the correction grid that was written
auto cd = MachineConfigReader{"test_config.h5"}.getCorrectionData(0);
assert((cd.shape == std::array<size_t,3>{257, 257, 2}));
double peak = cd.data[(128 * 257 + 128) * 2 + 0]; // Gaussian peak ≈ 2.0
assert(peak > 1.9 && peak < 2.1);
```

> The Gaussian grid formula, per-train offsets, and scalar defaults are identical to Python's
> `MockConfigBuilder` and Rust's `MockConfigBuilder` — cross-language fixture equivalence is
> verified in CI via `tools/cross_check.py`.

OPCUA data is outside the canonical model. `parse()` returns `config.opcua` as
`std::optional<OpcuaConfig>` (populated only when the file has an `OPCUA` group):

```cpp
#include "machine_config/reader.hpp"
using namespace machine_config;

MachineConfigReader reader{"fixtures/reference_config_opcua.h5"};
auto config = reader.parse();

if (config.opcua) {
    const auto& opc = *config.opcua;
    std::cout << opc.client.server_url << "\n";       // opc.tcp://...
    std::cout << opc.client.auth_mode << "\n";        // UsernamePassword
    std::cout << opc.pipe.pipe_enabled.value_or(false) << "\n"; // 1
    for (const auto& [name, trigger] : opc.triggers) {
        std::cout << name << ": signal=" << trigger.signal.value_or("") << "\n";
    }
    if (opc.triggers_enabled)
        std::cout << "Triggers enabled: " << *opc.triggers_enabled << "\n";
}

// Standard fixture has no OPCUA group:
auto cfg2 = MachineConfigReader{"fixtures/reference_config.h5"}.parse();
assert(!cfg2.opcua.has_value());

// getRawGroup() — escape hatch for any HDF5 path, mirrors Python/Rust/Node.js
MachineConfigReader reader2{"fixtures/reference_config_opcua.h5"};
auto client_attrs = reader2.getRawGroup("OPCUA/Client");
std::cout << client_attrs["Server_URL"].get<std::string>() << "\n"; // opc.tcp://...

// Returns an empty object (not an error) when the path is absent:
auto missing = reader2.getRawGroup("does/not/exist");  // {}
assert(missing.empty());
```

> `getRawGroup(path)` returns a `nlohmann::json` object of all attributes at the given HDF5
> path. Returns an empty object (never throws) when the path does not exist. Mirrors the same
> contract as Python's `get_raw_group()`, Rust's `get_raw_group()`, and Node.js's `getRawGroup()`.

---

### Use case 10 — Validate a config against the JSON schema

`machine_config/schema.hpp` exposes a `validate()` free function backed by
[pboettch/json-schema-validator](https://github.com/pboettch/json-schema-validator). It
validates against `schema/machine_config_v1.schema.json` (draft 7 keywords).

```cpp
#include "machine_config/reader.hpp"
#include "machine_config/schema.hpp"  // requires SCHEMA_DIR compile definition
using namespace machine_config;

// Validate reader output for a real fixture
auto j = nlohmann::json::parse(MachineConfigReader{"fixtures/reference_config.h5"}.toJson());
auto errors = validate(j);
if (errors.empty()) {
    std::cout << "Valid\n";
} else {
    for (const auto& e : errors)
        std::cerr << e << "\n";
}

// Validate MockConfigBuilder output
MockConfigBuilder{}.save("test.h5");
auto j2 = nlohmann::json::parse(MachineConfigReader{"test.h5"}.toJson());
assert(validate(j2).empty());

// An empty object fails (missing meta/machine/optical_trains)
assert(!validate(nlohmann::json::object()).empty());
```

> `schema.hpp` requires `SCHEMA_DIR` to be defined as a compile-time string pointing to the
> directory containing `machine_config_v1.schema.json`. In the CMake build this is set
> automatically for the test executable; consumers must define it in their own build.

`CorrectionData` holds a flat row-major `std::vector<double>` buffer and a
`std::array<size_t, 3> shape`. NaN values indicate out-of-field cells (the JSON path maps
these to `null`; here they are preserved for exact byte hashing).

```cpp
#include "machine_config/reader.hpp"
using namespace machine_config;

MachineConfigReader reader{"fixtures/reference_config.h5"};

// Forward grid, train 0 — flat buffer + shape, NaN preserved
CorrectionData cd = reader.getCorrectionData(0);
assert((cd.shape == std::array<size_t,3>{257, 257, 2}));

// Inverse grid, train 0
CorrectionData icd = reader.getInverseCorrectionData(0);

// Row-major indexing: offset = (i * shape[1] + j) * shape[2] + k
size_t d1 = cd.shape[1], d2 = cd.shape[2];
double centre_x = cd.data[(128 * d1 + 128) * d2 + 0]; // X-channel at centre
double centre_y = cd.data[(128 * d1 + 128) * d2 + 1]; // Y-channel at centre
// NaN means the point is outside the correction field
if (!std::isnan(centre_x))
    std::cout << "Centre correction x=" << centre_x << "\n";

// Raw .fc3 bytes (scan field correction file)
auto bytes = reader.getScanFieldCorrectionBytes(0);  // std::vector<uint8_t>
std::cout << "fc3 size: " << bytes.size() << " bytes\n"; // 1138799 for train 0

// parseWithBinary() populates Grid3D (nested optional<double>) in the model
auto full = reader.parseWithBinary();
auto& cb = *full.optical_trains[0].optional_components.clearbox;
assert(cb.correction_data.has_value());
assert((*cb.correction_data).size() == 257);  // outer dimension
```

---

### CLI reference (C++)

Build first (see [Build and install](#build-and-install) above).

```powershell
# PowerShell — Windows Release build
$cli = ".\cpp\build\Release\machine_config_cli.exe"

# Export HDF5 → JSON to stdout
& $cli export-json fixtures/reference_config.h5

# Export to a file
& $cli export-json fixtures/reference_config.h5 > output.json

# Write HDF5 from canonical JSON
& $cli write-hdf5 output.json reconstructed.h5

# Binary round-trip copy (preserves correction grids and .fc3 bytes verbatim)
& $cli copy-hdf5 fixtures/reference_config.h5 copy.h5

# SHA-256 of the forward correction grid, train 0
& $cli correction-hash fixtures/reference_config.h5 --train 0

# SHA-256 of the inverse correction grid, train 1
& $cli correction-hash fixtures/reference_config.h5 --train 1 --inverse
```

```bash
# Linux
cli="cpp/build/machine_config_cli"

# Export HDF5 → JSON to stdout
"$cli" export-json fixtures/reference_config.h5

# Write HDF5 from canonical JSON
"$cli" write-hdf5 output.json reconstructed.h5

# Binary round-trip copy
"$cli" copy-hdf5 fixtures/reference_config.h5 copy.h5

# SHA-256 of a correction grid
"$cli" correction-hash fixtures/reference_config.h5 --train 0
"$cli" correction-hash fixtures/reference_config.h5 --train 0 --inverse
```

> `correction-hash` hashes the grid as flat little-endian float64 bytes, identical in output to
> the Python, Rust, and Node.js implementations for the same file/train/direction.

---

### Quickstart example

Build first, then run from the repo root:

```powershell
# PowerShell
cmake -S cpp -B cpp/build -DCMAKE_BUILD_TYPE=Release
cmake --build cpp/build --config Release
.\cpp\build\Release\quickstart.exe
```

```bash
# Linux
cmake -S cpp -B cpp/build -DCMAKE_BUILD_TYPE=Release
cmake --build cpp/build
cpp/build/quickstart

# Windows (Git Bash) — use the Debug build already present
cpp/build/Debug/quickstart.exe
```

Expected output:
```
=== Machine Config Quickstart ===

Machine name   : TM-LPBF-02: AconityMIDI+_OG
Optical trains : 2
Working dist   : 670 mm   (train 0)
Correction grid: [257, 257, 2]   (train 0)

Written to     : mc_quickstart_tmp.h5

PASS
```

The source lives at [examples/quickstart/cpp/main.cpp](examples/quickstart/cpp/main.cpp).

---

### Full workflow example

**Scenario:** same calibration adjustment as Python, Rust, and Node.js — load config, apply new
scanner offsets via direct struct mutation, write, and verify the changes persisted alongside
the binary correction data.

```powershell
# PowerShell
.\cpp\build\Release\full_workflow.exe
```

```bash
# Linux
cpp/build/full_workflow

# Windows (Git Bash)
cpp/build/Debug/full_workflow.exe
```

Expected output:
```
=== Full Workflow: Calibration Adjustment ===

Machine : TM-LPBF-02: AconityMIDI+_OG
Trains  : 2

Before calibration:
  Train 1  offset x=-87.5, y=23.5
           correction grid 257x257x2
  Train 2  offset x=86.074, y=-21.695
           correction grid 257x257x2

Written to : mc_full_workflow_tmp.h5

After calibration:
  Train 1  offset x=-91.5, y=24
  Train 2  offset x=91.5, y=-24

PASS
```

The source lives at [examples/full_workflow/cpp/main.cpp](examples/full_workflow/cpp/main.cpp).

---

### Running the C++ test suite

```powershell
# PowerShell — from repo root
cmake --build cpp/build --config Debug
ctest --test-dir cpp/build -C Debug --output-on-failure
```

```bash
# Linux — from repo root
cmake --build cpp/build
ctest --test-dir cpp/build --output-on-failure

# Windows (Git Bash) — Debug build already present
cmake --build cpp/build --config Debug
ctest --test-dir cpp/build -C Debug --output-on-failure
```

Expected output (52 tests across 3 test files):
```
100% tests passed, 0 tests failed out of 63
```

Test breakdown:

| File | Tests | Coverage |
|---|---|---|
| `test_models.cpp` | 6 | JSON serialisation, `nlohmann` ADL round-trips |
| `test_reader.cpp` | 44 | Root attrs, machine, optical trains, scanner, ClearBox scalars, binary data (hash + fc3 size), OPCUA, synthetic fixture, `getRawGroup()` |
| `test_writer.cpp` | 6 | Scalar roundtrip, OPCUA roundtrip, schema spot-check, binary roundtrip hash + fc3 size |
| `test_builder.cpp` | 6 | 1/2-laser roundtrip, plate dims, correction grid shape + value, no-clearbox |
| `test_schema.cpp` | 3 | Reference fixture validates, MockBuilder output validates, empty object fails |

---

## Contributor Workflows

### Running the cross-language check

`tools/cross_check.py` is the correctness heartbeat. It runs four phases: schema validation, read parity across all fixtures, write interoperability, and correction-data hash parity. Run it after any change to Python, Rust, or Node.js code.

**Prerequisites**: Python venv active (`pip install -e python/[dev] deepdiff`), Rust release binary built (`cargo build --release` inside `rust/`), Node.js built (`npm ci && npm run build` inside `nodejs/`).

```powershell
# PowerShell — from repo root

# All three languages (Phases 1–3 — Phase 4 requires the Node.js correction-hash CLI)
.\.venv\Scripts\python tools/cross_check.py --langs python,rust,nodejs --skip-correction-hash --verbose

# Python+Rust only (all 4 phases)
.\.venv\Scripts\python tools/cross_check.py --langs python,rust --verbose

# Skip write-interop for a faster schema+parity check
.\.venv\Scripts\python tools/cross_check.py --skip-write-interop --skip-correction-hash
```

```bash
# Git Bash
.venv/Scripts/python tools/cross_check.py --langs python,rust,nodejs --skip-correction-hash --verbose
```

Expected output (all three languages, Phases 1–3):
```
Active languages: python, rust, nodejs

=== Phase 1: Schema Validation ===
[PASS] 9 combinations validate against schema.

=== Phase 2: Read Parity ===
[PASS] 3 languages agree on all 3 fixtures (9 comparisons).

=== Phase 3: Write Interoperability ===
[PASS] Write interoperability: 3 writer(s) × 3 reader(s) — 6 parity + 3 fidelity checks passed.

All checks passed.
```

Expected output (Python + Rust, all 4 phases):
```
Active languages: python, rust

=== Phase 1: Schema Validation ===
[PASS] 6 combinations validate against schema.

=== Phase 2: Read Parity ===
[PASS] 2 languages agree on all 3 fixtures (3 comparisons).

=== Phase 3: Write Interoperability ===
[PASS] Write interoperability checks passed.

=== Phase 4: Correction Data Hashes ===
[PASS] 2 languages produce identical correction hashes.

All checks passed.
```

> **Windows note**: `cross_check.py` explicitly uses `encoding="utf-8"` in all subprocess calls. Without this, Windows subprocess decoding (CP1252) silently corrupts multi-byte unit strings such as `μm` and `μs` — the check would report spurious failures on every Windows run.

---

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
