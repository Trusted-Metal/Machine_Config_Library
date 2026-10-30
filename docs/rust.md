# Rust — Machine Config Library

The Rust crate lives in `rust/` (package `machine-config`, library `machine_config`).
It has no system dependencies — `cargo build` compiles `libhdf5` from source on first run
(~2 min) via the `hdf5-metno` crate with the `static` feature. No system HDF5 install required.

← [Back to index](../USAGE.md)

---

## Contents

- [Use case 1 — Parse a machine config file](#use-case-1--parse-a-machine-config-file)
- [Use case 2 — Export to canonical JSON](#use-case-2--export-to-canonical-json)
- [Use case 4 — Write a config back to HDF5](#use-case-4--write-a-config-back-to-hdf5)
- [Use case 6 — Generate a synthetic test config](#use-case-6--generate-a-synthetic-test-config)
- [Use case 8 — Read OPCUA telemetry configuration](#use-case-8--read-opcua-telemetry-configuration)
- [Use case 9 — Access ClearBox correction arrays](#use-case-9--access-clearbox-correction-arrays)
- [Use case 10 — Validate a config against the schema](#use-case-10--validate-a-config-against-the-schema)
- [CLI reference](#cli-reference)
- [Quickstart example](#quickstart-example)
- [Full workflow example](#full-workflow-example)
- [Running the Rust test suite](#running-the-rust-test-suite)
- [Validation results](#validation-results)
- [Capability API (stable model facade)](#capability-api-stable-model-facade)

> Use cases 3, 5, and 7 (`config_from_dict`, `ConfigEditor`, `YamlConfigBuilder`) are
> Python-only conveniences with no Rust port.

---

## Capability API (stable model facade)

Preferred for applications. Peeks root `File_Version`, dispatches to a version
adapter, then index-based full-model get/set with `SetMode::Merge` / `Replace`.
The v1.0 facade and layout live in `rust/src/capabilities/v1_0/`;
`capabilities/mod.rs` only dispatches.

```rust
use machine_config::capabilities::{open_machine_config, SetMode};

let mut file = open_machine_config("machine.h5")?;
let mut scanner = file.get_scanner(0)?;
scanner.working_distance = Some(680.0);
file.set_scanner(0, scanner, SetMode::Merge)?;
file.save(Some(std::path::Path::new("out.h5")))?;
```

See [USAGE.md](../USAGE.md) and `schema/capabilities/`.

---

## Installation

Add to `Cargo.toml` — no system dependencies required (`libhdf5` is compiled from source on first build via `hdf5-metno` with the `static` feature):

```toml
# Pin to a specific tag
machine-config = { git = "https://github.com/Trusted-Metal/Machine_Config_Library", tag = "v0.2.0-rc.1" }

# Track main (locked in Cargo.lock; run `cargo update -p machine-config` to advance)
machine-config = { git = "https://github.com/Trusted-Metal/Machine_Config_Library" }
```

SSH URL also works:
```toml
machine-config = { git = "ssh://git@github.com/Trusted-Metal/Machine_Config_Library.git", tag = "v0.2.0-rc.1" }
```

---

## Use case 1 — Parse a machine config file

```rust
use machine_config::reader::MachineConfigReader;

let reader = MachineConfigReader::open("fixtures/reference_config.h5")?;
let config = reader.parse()?;   // scalars + metadata only

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
    // x_axis/y_axis always present; z_axis is Some for "3D"/"3D+Focus";
    // focus is Some only for "3D+Focus".
    if let Some(z) = &s.z_axis {
        println!("  Z bit_resolution: {:?} {:?}", z.actual_bit_resolution, z.actual_bit_resolution_unit);
    }
}
```

---

## Use case 2 — Export to canonical JSON

```rust
use machine_config::reader::MachineConfigReader;

let reader = MachineConfigReader::open("fixtures/reference_config.h5")?;

// Default: metadata + scalars only, pretty-printed
let json = reader.to_json(true, false)?;
std::fs::write("output.json", json)?;

// With binary data: adds raw_bytes to every scan-field correction file. The ClearBox grids are
// extension datasets and are always included.
let full_json = reader.to_json(true, true)?;
std::fs::write("output_full.json", full_json)?;
```

> **Accessing binary data without JSON**: ClearBox grids are in `config.extensions["ClearBox"]` (use
> case 9). `reader.get_scan_field_correction_bytes(train_index)` returns `Vec<u8>` and reads
> directly from HDF5 without going through the model.

---

## Use case 4 — Write a config back to HDF5

```rust
use machine_config::reader::MachineConfigReader;
use machine_config::writer::MachineConfigWriter;

let reader = MachineConfigReader::open("fixtures/reference_config.h5")?;
let mut config = reader.parse()?;

config.meta.export_date = "2026-07-28T00:00:00Z".to_owned();

MachineConfigWriter::new(&config).write("output.h5")?;

let reread = MachineConfigReader::open("output.h5")?.parse()?;
assert_eq!(config.meta.machine_name, reread.meta.machine_name);
assert_eq!(config.meta.export_date,  reread.meta.export_date);
```

> **Binary data in the writer**: the ClearBox grids travel inside `config.extensions`, so they are
> written back verbatim. The scan-field correction bytes are only present when the config was
> parsed with `parse_with_binary()`; otherwise the writer writes a zero-filled placeholder for them.

---

## Use case 6 — Generate a synthetic test config

```rust
use machine_config::builder::MockConfigBuilder;
use machine_config::reader::MachineConfigReader;

// 2-laser config with ClearBox (default)
MockConfigBuilder::new(2).save("test_config.h5")?;

// Customise before saving
let mut b = MockConfigBuilder::new(1);
b.machine_name    = "TestMachine".to_owned();
b.build_plate_x   = 400.0;
b.include_clearbox = false;
b.save("custom_config.h5")?;

// Build into memory without writing
let config = MockConfigBuilder::new(2).build();
assert_eq!(config.optical_trains.len(), 2);
assert_eq!(config.meta.machine_name, "MockMachine");

// Verify the correction grid (ClearBox is an extension tree; see use case 9)
let config = MachineConfigReader::open("test_config.h5")?.parse()?;
let ds = &config.extensions["ClearBox"].children["Optical_Train_01"].datasets["Correction_Data"];
assert_eq!(ds.shape, vec![257, 257, 2]);
if let DatasetData::F64(v) = &ds.data {
    println!("Peak correction: {:.4}", v[(128 * 257 + 128) * 2]);  // ≈ 2.0 (Gaussian peak)
}
```

---

## Use case 8 — Read an extension (OPC UA) tree

Core has no OPC UA model. Every group under `Extensions/` that core does not model (and, in
`File_Version` 1.0, the legacy root `OPCUA/` group, exposed as `TM_OPCUA`) is returned verbatim in
`config.extensions`, and written back unchanged. Pack and unpack it with the extension's own
library (see `docs/EXTENSION_CONTRACT.md`); `get_raw_group()` remains for ad-hoc attribute inspection:

```rust
use machine_config::{canonical_digest, reader::MachineConfigReader};

let reader = MachineConfigReader::open("fixtures/reference_config_opcua.h5")?;
let config = reader.parse()?;

if let Some(opcua) = config.extensions.get("TM_OPCUA") {
    println!("{:?}", opcua.children.keys().collect::<Vec<_>>());      // ["Client", "Pipe", "Triggers"]
    println!("{:?}", opcua.children["Client"].attrs["Server_URL"].as_str());
    println!("{}", canonical_digest(opcua)?);                          // identical in Node and Rust
}

// Raw attribute map for any HDF5 path — returns empty map, never an error, if absent
let client_attrs = reader.get_raw_group("OPCUA/Client")?;
println!("{:?}", client_attrs.get("Server_URL"));
```

---

## Use case 9 — Access ClearBox correction arrays

ClearBox is not a typed model in core: it is the opaque `ClearBox` extension tree
(`config.extensions["ClearBox"]`), in the File_Version 1.1 layout for both file versions (see
[EXTENSION_CONTRACT.md](EXTENSION_CONTRACT.md#clearbox)). Each train is a child
`Optical_Train_NN` with `Correction_Data` / `Inverse_Correction_Data` datasets, `(257, 257, 2)` f64
row-major with `NaN` for out-of-field points.

```rust
use machine_config::extension_contract::DatasetData;
use machine_config::reader::MachineConfigReader;

let config = MachineConfigReader::open("fixtures/reference_config.h5")?.parse()?;

let root = &config.extensions["ClearBox"];
let train = &root.children["Optical_Train_01"];
let grid = &train.datasets["Correction_Data"];   // grid.shape == [257, 257, 2]
if let DatasetData::F64(v) = &grid.data {
    println!("Non-finite cells: {}", v.iter().filter(|x| !x.is_finite()).count());
    println!("Centre: x={}, y={}", v[(128 * 257 + 128) * 2], v[(128 * 257 + 128) * 2 + 1]);
}

// Per-train scalars are extension attrs (with their stored dtypes); shared values are on the root.
println!("{:?} {:?}", train.attrs["Ip_Address"], root.attrs["Output_Path"]);

// Power characterization is a child group with two tables.
let pc = &train.children["Power_Characterization"];
println!("{:?}", pc.tables["Derivation_Equation_Constants"].rows);
```

---

## Use case 10 — Validate a config against the schema

Rust has no standalone `validate()` function. Schema compliance is enforced implicitly:
`parse()` returns `Err` if a required field is missing or has the wrong type. All optional
fields are modelled as `Option<T>`; a wrong type is a deserialization error.

```rust
use machine_config::reader::MachineConfigReader;
use machine_config::builder::MockConfigBuilder;

// A well-formed file always parses without error.
let config = MachineConfigReader::open("fixtures/reference_config.h5")?.parse()?;
assert_eq!(config.meta.schema_version, "v1");
assert_eq!(config.meta.configuration_hash.len(), 64);

// MockConfigBuilder output satisfies every structural constraint.
let mock = MockConfigBuilder::new(2).build();
assert_eq!(mock.meta.schema_version, "v1");
```

> For explicit JSON Schema validation, use Python (`jsonschema.validate`) or C++
> (`machine_config::validate()`). `tools/cross_check.py` validates Rust's `export-json` output
> against the schema on every CI run — this is the authoritative compliance gate for Rust.

---

## CLI reference

```powershell
# PowerShell — build first
cargo build --release --manifest-path rust/Cargo.toml

# Export HDF5 → JSON to stdout
.\rust\target\release\machine-config-cli.exe export-json fixtures\reference_config.h5

# With binary fields
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
./rust/target/release/machine-config-cli write-hdf5 config.json output.h5
./rust/target/release/machine-config-cli correction-hash fixtures/reference_config.h5 --train 0
./rust/target/release/machine-config-cli correction-hash fixtures/reference_config.h5 --train 1 --inverse
```

> Errors go to stderr; stdout is always valid JSON (for `export-json`) or empty on success.
> `correction-hash` output is byte-identical to Python and Node.js for the same input — verified in CI.

---

## Quickstart example

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

Machine name   : ExampleDummy-2Train
Optical trains : 2
  Train 0  wd=Some(670.0) mm  offset x=Some(-87.5), y=Some(23.5)
           clearbox: present
  Train 1  wd=Some(670.0) mm  offset x=Some(87.5), y=Some(-23.5)
           clearbox: present
Correction grid: [257, 257, 2]   (train 0)

Written to     : <tmp>.h5

PASS
```

Source: [rust/examples/quickstart.rs](../rust/examples/quickstart.rs)

---

## Full workflow example

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

Machine : ExampleDummy-2Train
Trains  : 2

Before calibration:
  Train 1  offset x=Some(-87.5), y=Some(23.5)
           correction grid [257, 257, 2]
  Train 2  offset x=Some(87.5), y=Some(-23.5)
           correction grid [257, 257, 2]

Written to : <tmp>.h5

After calibration:
  Train 1  offset x=Some(-91.5), y=Some(24.0)
  Train 2  offset x=Some(91.5), y=Some(-24.0)

PASS
```

Source: [rust/examples/full_workflow.rs](../rust/examples/full_workflow.rs)

---

## Running the Rust test suite

```powershell
# PowerShell — from repo root
Push-Location rust
cargo test --lib        # 74 unit tests (error, models, reader, builder, writer modules)
cargo test              # 74 unit + 8 capabilities + 30 integration + 5 adapter-migration + 2 doc = 119 tests total
cargo bench             # Criterion benchmarks: open_and_parse, to_json_pretty, parse_with_binary
Pop-Location
```

```bash
# Git Bash — from repo root
cd rust
cargo test --lib
cargo test
cargo bench
```

| Test binary | Count | Location |
|---|---|---|
| Library unit tests | 74 | `src/error.rs`, `src/models.rs`, `src/reader.rs`, `src/builder.rs`, `src/writer.rs` |
| Capabilities tests | 8 | `tests/capabilities_test.rs` |
| Integration tests | 30 | `tests/integration_test.rs` |
| Adapter migration tests | 5 | `tests/adapter_migration_test.rs` — mock v1.1 adapter (AV-09–11) |
| Doc tests | 2 | `src/models.rs` |
| **Total** | **119** | |

---

## Validation results

20 / 20 scenarios pass on this SDK. See the full table in
[docs/validation/rust/PASS_FAIL.md](../docs/validation/rust/PASS_FAIL.md) and verbatim
output in [docs/validation/rust/results.md](../docs/validation/rust/results.md).

No real library defects were found during this validation pass — the corrupt-scalar and
missing-group scenarios (AV-04/AV-05) already returned typed errors (`MissingGroup`/`Parse`)
from the start, and the mock v1.1 adapter's `meta.extra` handling was written correctly on
the first pass, having already seen the equivalent bug surface in Node.js's implementation
earlier in this validation effort.

For the master cross-language matrix see [docs/validation/README.md](../docs/validation/README.md).
