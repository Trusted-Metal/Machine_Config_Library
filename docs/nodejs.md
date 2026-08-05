# Node.js — Machine Config Library

The Node.js SDK lives in `nodejs/` and is built with TypeScript (ES2022, NodeNext modules).
It uses [h5wasm](https://github.com/usnistgov/h5wasm) — HDF5 compiled to WebAssembly by NIST —
so there is no native compilation step and no system HDF5 library required on any platform.

← [Back to index](../USAGE.md)

---

## Contents

- [Installation](#installation)
- [TypeScript interfaces](#typescript-interfaces)
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
- [Running the Node.js test suite](#running-the-nodejs-test-suite)

> Use cases 3, 5, and 7 (reconstruct-from-JSON, `ConfigEditor`, `YamlConfigBuilder`) are
> Python-only conveniences with no Node.js port.

---

## Installation

```bash
cd nodejs
npm ci
npm run build      # compile TypeScript → dist/
```

All types and the runtime are exported from `dist/index.js`. Build output must be present
before running any example or CLI command.

---

## TypeScript interfaces

All types are fully exported from the package root:

```typescript
import type { MachineConfig, OpticalTrain, ClearBox, OpcuaConfig } from './dist/index.js';
```

`models.ts` defines the full interface tree — `MachineConfig`, `OpticalTrain`, `Scanner`,
`AxisConfig`, `LightSource`, `Collimator`, `ScannerCard`, `ClearBox`,
`ScanFieldCorrectionFile`, `OpcuaConfig` and friends — with snake_case field names
throughout, mirroring Python's model.

---

## Use case 1 — Parse a machine config file

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

---

## Use case 2 — Export to canonical JSON

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

// Include correction grids and raw .fc3 bytes (~14 MB for a 2-laser config)
const full = await reader.toJson({ includeBinary: true, indent: 2 });
writeFileSync('output_full.json', full, 'utf-8');
```

> **Accessing binary data without JSON**: use `parse({ includeBinary: true })` to get
> `optional_components.clearbox.correction_data` as a `(number | null)[][][]` array of shape
> 257×257×2 in the model. `null` cells are out-of-field points (NaN in HDF5).

---

## Use case 4 — Write a config back to HDF5

```typescript
import { MachineConfigReader, MachineConfigWriter } from './dist/index.js';

const config = await new MachineConfigReader('original.h5').parse();

// MachineConfig is a plain TS object — mutate fields directly or with spread
config.meta.export_date = '2026-07-30T00:00:00Z';

await new MachineConfigWriter(config).write('copy.h5');

// Verify the roundtrip
const reread = await new MachineConfigReader('copy.h5').parse();
console.log(reread.meta.configuration_hash === config.meta.configuration_hash);
```

> **Binary data in the writer**: if `correction_data` / `inverse_correction_data` are absent
> (config parsed without `includeBinary: true`), the writer writes zero-filled `(257, 257, 2)`
> float64 datasets — identical behaviour to Python and Rust. Parse with `{ includeBinary: true }`
> first to preserve the original correction grids.

---

## Use case 6 — Generate a synthetic test config

`MockConfigBuilder` creates structurally valid `.h5` files for testing — same defaults,
per-train geometry, and Gaussian correction-grid formula as Python and Rust.

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

// Verify the correction grid
const reader = new MachineConfigReader('test_config.h5');
const cd = await reader.getCorrectionData(0);   // { data: Float64Array, shape: [257, 257, 2] }
console.log('Peak correction:', cd.data[(128 * 257 + 128) * 2].toFixed(4)); // ≈ 2.0000
```

> **Defaults**: `nLasers: 2`, `buildPlateX/Y: 250`, `buildPlateZ: 20`, `includeClearbox: true`,
> `machineName: "MockMachine"`, `manufacturer: "MockCo"`, `model: "MockMIDI+"`,
> `serialNumber: "MOCK-001"`. Output is reproducible run to run.

---

## Use case 8 — Read OPCUA telemetry configuration

OPCUA data lives in a separate `OPCUA` group absent in most files. `parse()` returns
`config.opcua` when present, `undefined` otherwise. Use `getRawGroup()` for ad-hoc inspection:

```typescript
import { MachineConfigReader } from './dist/index.js';

// Typed path through the model
const config = await new MachineConfigReader('fixtures/reference_config_opcua.h5').parse();
if (config.opcua) {
  console.log(config.opcua.client.server_url);
  console.log(config.opcua.triggers_enabled);
  for (const [name, t] of Object.entries(config.opcua.triggers)) {
    console.log(name, t.signal, t.subsystem);
  }
}

// Raw attribute map for any HDF5 path — returns {} (not an error) if absent
const reader = new MachineConfigReader('fixtures/reference_config_opcua.h5');
const clientAttrs = await reader.getRawGroup('OPCUA/Client');
console.log(clientAttrs['Server_URL']);
console.log(clientAttrs['Auth_Mode']);

const missing = await reader.getRawGroup('does/not/exist');  // {}
```

---

## Use case 9 — Access ClearBox correction arrays

For numerical work — or anything that must match Python/Rust bit-for-bit such as hashing —
read the raw grid directly via the dedicated accessors:

```typescript
import { MachineConfigReader } from './dist/index.js';

const reader = new MachineConfigReader('fixtures/reference_config.h5');

// Flat, row-major, NaN preserved (not JSON-safe) — shape [257, 257, 2]
const cd  = await reader.getCorrectionData(0);           // train 0, forward grid
const icd = await reader.getInverseCorrectionData(0);    // train 0, inverse grid

console.log(cd.data instanceof Float64Array, cd.shape);  // true [257, 257, 2]

// Row-major offset = (i*257 + j)*2 + k
const centreX = cd.data[(128 * 257 + 128) * 2 + 0];
console.log('Centre correction X:', centreX);
```

> `getCorrectionData`/`getInverseCorrectionData` preserve `NaN` exactly (they skip the
> `NaN → null` conversion that `parse({ includeBinary: true })` applies). This is what the
> `correction-hash` CLI uses internally for byte-identical cross-language hashing.

---

## Use case 10 — Validate a config against the schema

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

The validator uses [Ajv](https://ajv.js.org/) (JSON Schema draft 2020-12). An empty array means valid.

---

## CLI reference

```bash
# From the repo root — build first if not already done
cd nodejs && npm run build && cd ..

# Export HDF5 → JSON to stdout
node nodejs/dist/cli.js export-json fixtures/reference_config.h5

# Export to a file
node nodejs/dist/cli.js export-json fixtures/reference_config.h5 > output.json

# Write HDF5 from JSON
node nodejs/dist/cli.js write-hdf5 config.json output.h5

# SHA-256 of the forward correction grid, train 0
node nodejs/dist/cli.js correction-hash fixtures/reference_config.h5 --train 0

# SHA-256 of the inverse correction grid, train 1
node nodejs/dist/cli.js correction-hash fixtures/reference_config.h5 --train 1 --inverse
```

> `correction-hash` hashes the grid as flat little-endian float64 bytes — output is
> byte-identical to Python and Rust for the same file/train/direction, verified in CI.

---

## Quickstart example

```bash
# Git Bash / PowerShell — from the repo root
cd nodejs && npm run build && cd ..
node examples/quickstart/nodejs/main.mjs
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

---

## Full workflow example

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

Source: [examples/full_workflow/nodejs/main.mjs](../examples/full_workflow/nodejs/main.mjs)

---

## Running the Node.js test suite

```bash
# From the nodejs/ directory
cd nodejs
npm test           # 130 tests: 82 reader + 6 schema + 24 writer + 18 builder
```

```powershell
# PowerShell — from repo root
Push-Location nodejs ; npm test ; Pop-Location
```

| Suite | Tests | What it covers |
|---|---|---|
| `reader.test.ts` | 82 | All 3 fixtures; meta, machine geometry, optical trains, scanner, ClearBox, SFCF, correction data shape/NaN, OPCUA (13 tests), JSON serialisation, Ajv schema validation |
| `schema.test.ts` | 6 | Schema loads; `validate()` rejects empty/invalid; accepts minimal valid document |
| `writer.test.ts` | 24 | Reference fixture roundtrip; OPCUA roundtrip incl. trigger field values; synthetic 2-laser roundtrip |
| `builder.test.ts` | 18 | `build()` in-memory; `save()` + read-back; correction grid shape/peak; no-clearbox path; schema validity |
