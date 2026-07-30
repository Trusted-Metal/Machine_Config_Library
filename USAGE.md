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
  - [Quickstart example](#quickstart-example)
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
- [Node.js](#nodejs) ← *coming in Phase 2*
  - [Quickstart example](#quickstart-example-coming-in-phase-2)
  - [Use case 1 — Parse a machine config file](#use-case-1--parse-a-machine-config-file-nodejs)
  - [Use case 2 — Export to canonical JSON](#use-case-2--export-to-canonical-json-nodejs)
  - [Use case 3 — Reconstruct a config from JSON](#use-case-3--reconstruct-a-config-from-json-nodejs)
  - [Use case 4 — Write a config back to HDF5](#use-case-4--write-a-config-back-to-hdf5-nodejs)
  - [Use case 5 — Edit a field and save to a new file](#use-case-5--edit-a-field-and-save-to-a-new-file-nodejs)
  - [Use case 6 — Generate a synthetic test config](#use-case-6--generate-a-synthetic-test-config-nodejs)
  - [Use case 7 — Build a config from a YAML specification](#use-case-7--build-a-config-from-a-yaml-specification-nodejs)
  - [Use case 8 — Read OPCUA telemetry configuration](#use-case-8--read-opcua-telemetry-configuration-nodejs)
  - [Use case 9 — Access ClearBox correction arrays](#use-case-9--access-clearbox-correction-arrays-nodejs)
  - [Use case 10 — Validate a config against the schema](#use-case-10--validate-a-config-against-the-schema-nodejs)
  - [CLI reference](#cli-reference-nodejs)
- [Rust](#rust) ← *Phase 3 complete*
  - [Quickstart example](#quickstart-example-1)
  - [Use case 1 — Parse a machine config file](#use-case-1--parse-a-machine-config-file-1)
  - [Use case 2 — Export to canonical JSON](#use-case-2--export-to-canonical-json-1)
  - [Use case 3 — Read OPCUA telemetry configuration](#use-case-3--read-opcua-telemetry-configuration)
  - [Use case 4 — Access ClearBox correction arrays](#use-case-4--access-clearbox-correction-arrays)
  - [Use case 5 — Write a config to HDF5](#use-case-5--write-a-config-to-hdf5)
  - [Use case 6 — Generate a synthetic test fixture](#use-case-6--generate-a-synthetic-test-fixture)
  - [Use case 7 — Export JSON from the command line](#use-case-7--export-json-from-the-command-line)
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

*Coming in Phase 2.*

Section will cover:
- `npm install machine-config-library`
- Parsing `.h5` with `node-hdf5` in Node.js
- TypeScript interfaces
- Equivalent use cases to the Python section above
- CLI usage
- Viewer bundle integration

### Quickstart example *(coming in Phase 2)*

```bash
# Git Bash / PowerShell — from the repo root
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

---

## Rust

*Phase 3 complete — data models, HDF5 reader, writer, `MockConfigBuilder`, CLI, and integration tests all implemented and verified.*

The crate lives in `rust/` (package `machine-config`, library `machine_config`). It has no system dependencies — `cargo build` compiles `libhdf5` from source on first run (see [IMPLEMENTATION_PLAN.md §3.2](IMPLEMENTATION_PLAN.md#32--platform--dependency-decision)).

### What's usable today

`rust/src/models.rs` defines the full data model — `MachineConfig` and its complete field tree (`Machine`, `OpticalTrain`, `Scanner`, `AxisConfig`, `LightSource`, `Collimator`, `ScannerCard`, `ClearBox`, `ScanFieldCorrectionFile`, `OpcuaConfig` and friends) — mirroring `python/src/machine_config/models.py`, with `#[derive(Serialize, Deserialize)]` so any value round-trips through `serde_json`.

`rust/src/reader.rs` implements `MachineConfigReader`, mirroring Python's `MachineConfigReader`. Its JSON output has been verified to deep-equal `fixtures/reference_output.json` (the Python golden file) for the reference AconityMIDI fixture.

`rust/src/writer.rs` implements `MachineConfigWriter`, mirroring Python's `MachineConfigWriter`. It is the exact inverse of the reader: every attribute written matches what the reader expects to find, and a write-then-read roundtrip preserves all scalar fields and the SHA-256 of the ClearBox correction grids.

`rust/src/builder.rs` implements `MockConfigBuilder`, which generates structurally valid synthetic `.h5` files for testing. It mirrors Python's `MockConfigBuilder` — same group/attribute layout, non-zero Gaussian correction grids, deterministic values.

`rust/src/main.rs` is the `machine-config-cli` binary. It exposes a single subcommand (`export-json`) that writes pretty-printed JSON to stdout — the interface used by `cross_check.py` in Phase 5.

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

### Use case 3 — Read OPCUA telemetry configuration

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

### Use case 4 — Access ClearBox correction arrays

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

### Use case 5 — Write a config to HDF5

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

### Use case 6 — Generate a synthetic test fixture

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

### Use case 7 — Export JSON from the command line

`machine-config-cli` is the Rust binary equivalent of `machine-config export-json` in Python. It writes pretty-printed JSON to stdout and exits 0 on success.

```powershell
# PowerShell — build first
cargo build --release --manifest-path rust/Cargo.toml

# Export scalars only (default)
.\rust\target\release\machine-config-cli.exe export-json fixtures\reference_config.h5

# Export with binary fields (correction grids + raw .fc3 bytes)
.\rust\target\release\machine-config-cli.exe export-json fixtures\reference_config.h5 --include-binary

# Pipe to a file
.\rust\target\release\machine-config-cli.exe export-json fixtures\reference_config.h5 > output.json
```

```bash
# Git Bash
cargo build --release --manifest-path rust/Cargo.toml
./rust/target/release/machine-config-cli export-json fixtures/reference_config.h5
./rust/target/release/machine-config-cli export-json fixtures/reference_config.h5 --include-binary > output_full.json
```

> Errors (file not found, unrecognised format, etc.) go to stderr; stdout is always valid JSON on success.

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

*Coming in Phase 4.*

Section will cover:
- CMake `FetchContent` / vcpkg integration
- Parsing `.h5` with HighFive
- `nlohmann/json` serialisation
- Equivalent use cases to the Python section above
- CLI usage

---

## Contributor Workflows

### Running the cross-language check

`tools/cross_check.py` is the correctness heartbeat. It runs three phases: schema validation, read parity across all fixtures, and write interoperability. Run it after any change to Python or Rust code.

**Prerequisites**: Python venv active (`pip install -e python/[dev] deepdiff`), Rust release binary built (`cargo build --release` inside `rust/`).

```powershell
# PowerShell — from repo root
.\.venv\Scripts\python tools/cross_check.py --verbose

# Check a subset only (e.g. while another language binary is missing)
.\.venv\Scripts\python tools/cross_check.py --langs python,rust --verbose

# Skip write-interop (Phase 3) for a faster schema+parity-only check
.\.venv\Scripts\python tools/cross_check.py --skip-write-interop
```

```bash
# Git Bash
.venv/Scripts/python tools/cross_check.py --verbose
```

Expected output (all green):
```
Active languages: python, rust

=== Phase 1: Schema Validation ===
[PASS] 6 combinations validate against schema.

=== Phase 2: Read Parity ===
[PASS] 2 languages agree on all 3 fixtures (3 comparisons).

=== Phase 3: Write Interoperability ===
[PASS] Write interoperability checks passed.

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
