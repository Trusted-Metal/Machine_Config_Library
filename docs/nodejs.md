# Node.js — Machine Config Library

The Node.js SDK lives in `nodejs/` and is built with TypeScript (ES2022, NodeNext modules).
It uses [h5wasm](https://github.com/usnistgov/h5wasm) — HDF5 compiled to WebAssembly by NIST —
so there is no native compilation step and no system HDF5 library required on any platform.

← [Back to index](../USAGE.md)

---

## Contents

- [Installation](#installation)
- [Capability API (File_Version adapters)](#capability-api-file_version-adapters)
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
- [Validation results](#validation-results)

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

### As a dependency

Node.js has no git install method — npm does not support `#subdirectory=` installs.
Use the tgz release asset (requires [GitHub CLI](https://cli.github.com/) — `gh auth login` once):

```bash
gh release download v0.2.0-rc.1 --repo Trusted-Metal/Machine_Config_Library --pattern "*.tgz"
npm install machine-config-library-0.2.0-rc.1.tgz
```

Pin in `package.json` (after downloading tgz to your project):
```json
"machine-config-library": "file:./machine-config-library-0.2.0-rc.1.tgz"
```

---

## Capability API (File_Version adapters)

Preferred for application code. Authored in TypeScript; consumers get transpiled JS + `.d.ts`.

Stable navigation + **full-model** get/set (`getScanner` / `setScanner`, …) with
`SetMode.Merge` (default) or `SetMode.Replace`. Apps do not branch on `File_Version` for
normal reads/writes.

```typescript
import {
  openMachineConfig,
  createMachineConfig,
  SetMode,
  isOk,
  type MachineConfigFile,
} from 'machine-config-library';

const opened = await openMachineConfig('machine.h5');
if (!opened.ok) throw new Error(opened.error.message);

const file: MachineConfigFile = opened.value;
console.log(file.fileVersion()); // "1.0"
console.log(file.meta().getModel().machine_name);

const train = file.opticalTrain(0);
if (train.ok) {
  const scanner = train.value.getScanner(); // snapshot
  await train.value.setScanner(
    { ...scanner, working_distance: 680 },
    SetMode.Merge,
  );
}

await file.save(); // or file.save('out.h5')
file.close();

// New file from scratch (version preserved on save):
const created = createMachineConfig('1.0');
```

Adapters are keyed by HDF5 `File_Version`. Today only `1.0` is registered; unknown versions
return `UnsupportedVersion`. The v1.0 facade and layout live in
`nodejs/src/capabilities/v1_0/`; `capabilities/index.ts` only dispatches.
See `schema/capabilities/` and [USAGE.md](../USAGE.md).

---

## TypeScript interfaces

All types are fully exported from the package root:

```typescript
import type { MachineConfig, OpticalTrain, ExtensionTree } from './dist/index.js';
```

`models.ts` defines the full interface tree — `MachineConfig`, `OpticalTrain`, `Scanner`,
`AxisConfig`, `LightSource`, `Collimator`, `ScannerCard`,
`ScanFieldCorrectionFile`, `ExtensionTree` and friends — with snake_case field names
throughout, mirroring Python's model. ClearBox has no model: it is the `ClearBox` extension tree
(use case 9).

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

// Extensions (e.g. OPC UA) — opaque trees, interpreted by the matching extension library
for (const name of Object.keys(config.extensions ?? {})) {
  console.log(`extension: ${name}`);
}
```

---

## Use case 2 — Export to canonical JSON

`toJson()` produces metadata, scalar fields and every extension tree (the ClearBox correction grids
are extension datasets, so they are always included). Raw `.fc3` bytes are excluded unless
`includeBinary: true` is passed.

```typescript
import { MachineConfigReader } from './dist/index.js';
import { writeFileSync } from 'node:fs';

const reader = new MachineConfigReader('fixtures/reference_config.h5');

// Default: metadata, scalars and extensions (~14 MB for a 2-laser config)
const json = await reader.toJson({ indent: 2 });
writeFileSync('output.json', json, 'utf-8');

// Compact (no indentation)
const compact = await reader.toJson({ indent: 0 });

// Also include raw .fc3 bytes
const full = await reader.toJson({ includeBinary: true, indent: 2 });
writeFileSync('output_full.json', full, 'utf-8');
```

> **Accessing binary data without JSON**: the ClearBox grids are `Float64Array`s in
> `config.extensions.ClearBox` (use case 9); `NaN` marks out-of-field points.

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

> **Binary data in the writer**: the ClearBox grids travel inside `config.extensions`, so they are
> written back verbatim. Only the scan-field correction bytes need `parse({ includeBinary: true })`;
> otherwise the writer writes a zero-filled placeholder for them.

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

// Verify the correction grid (ClearBox is an extension tree; see use case 9)
const reader = new MachineConfigReader('test_config.h5');
const cd = (await reader.parse()).extensions!.ClearBox.children.Optical_Train_01.datasets.Correction_Data;
console.log('Peak correction:', (cd.data as Float64Array)[(128 * 257 + 128) * 2].toFixed(4)); // ≈ 2.0000
```

> **Defaults**: `nLasers: 2`, `buildPlateX/Y: 250`, `buildPlateZ: 20`, `includeClearbox: true`,
> `machineName: "MockMachine"`, `manufacturer: "MockCo"`, `model: "MockMIDI+"`,
> `serialNumber: "MOCK-001"`. Output is reproducible run to run.

---

## Use case 8 — Read an extension (OPC UA) tree

Core has no OPC UA model. Every group under `Extensions/` that core does not model (and, in
`File_Version` 1.0, the legacy root `OPCUA/` group, exposed as `TM_OPCUA`) is returned verbatim in
`config.extensions`, and written back unchanged. Pack and unpack it with the extension's own
library (see `docs/EXTENSION_CONTRACT.md`); `getRawGroup()` remains for ad-hoc attribute inspection:

```typescript
import { MachineConfigReader, canonicalDigest } from './dist/index.js';

const config = await new MachineConfigReader('fixtures/reference_config_opcua.h5').parse();
const opcua = config.extensions?.TM_OPCUA;
if (opcua) {
  console.log(Object.keys(opcua.children));                  // ['Client', 'Pipe', 'Triggers']
  console.log(opcua.children.Client.attrs.Server_URL);       // { dtype: 'utf8', value: '...' }
  console.log(canonicalDigest(opcua));                       // identical in Node and Rust
}

// Raw attribute map for any HDF5 path — returns {} (not an error) if absent
const reader = new MachineConfigReader('fixtures/reference_config_opcua.h5');
const clientAttrs = await reader.getRawGroup('OPCUA/Client');
console.log(clientAttrs['Server_URL']);

const missing = await reader.getRawGroup('does/not/exist');  // {}
```

---

## Use case 9 — Access ClearBox correction arrays

ClearBox is not a typed model in core: it is the opaque `ClearBox` extension tree
(`config.extensions.ClearBox`), in the File_Version 1.1 layout for both file versions (see
[EXTENSION_CONTRACT.md](EXTENSION_CONTRACT.md#clearbox)). Each train is a child
`Optical_Train_NN` with `Correction_Data` / `Inverse_Correction_Data` datasets: flat, row-major
`Float64Array`s of shape `[257, 257, 2]` with `NaN` for out-of-field points.

```typescript
import { MachineConfigReader } from './dist/index.js';

const config = await new MachineConfigReader('fixtures/reference_config.h5').parse();
const root = config.extensions!.ClearBox;
const train = root.children.Optical_Train_01;

const cd = train.datasets.Correction_Data;
console.log(cd.data instanceof Float64Array, cd.shape);  // true [257, 257, 2]

// Row-major offset = (i*257 + j)*2 + k
const centreX = (cd.data as Float64Array)[(128 * 257 + 128) * 2 + 0];
console.log('Centre correction X:', centreX);

// Per-train scalars are extension attrs (with their stored dtypes); shared values are on the root.
console.log(train.attrs.Ip_Address.value, root.attrs.Output_Path.value);

// Power characterization is a child group with two tables.
const pc = train.children.Power_Characterization;
console.log(pc.tables!.Derivation_Equation_Constants.rows);
```

> The `correction-hash` CLI subcommand hashes this dataset's little-endian float64 bytes, which is
> what gives byte-identical cross-language hashes.

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

Machine name   : ExampleDummy-2Train
Optical trains : 2
  Train 0  wd=670 mm  offset x=-87.5, y=23.5
           clearbox: present
  Train 1  wd=670 mm  offset x=87.5, y=-23.5
           clearbox: present
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

Machine : ExampleDummy-2Train
Trains  : 2

Before calibration:
  Train 1  offset x=-87.5, y=23.5
           correction grid [257, 257, 2]
  Train 2  offset x=87.5, y=-23.5
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
npm test           # 151 tests: 84 reader + 6 schema + 27 writer + 18 builder + 8 capabilities + 8 adapterMigration
```

```powershell
# PowerShell — from repo root
Push-Location nodejs ; npm test ; Pop-Location
```

| Suite | Tests | What it covers |
|---|---|---|
| `reader.test.ts` | 84 | All 3 fixtures; meta, machine geometry, optical trains, scanner, ClearBox, SFCF, correction data shape/NaN, OPCUA (13 tests), JSON serialisation, Ajv schema validation |
| `schema.test.ts` | 6 | Schema loads; `validate()` rejects empty/invalid; accepts minimal valid document |
| `writer.test.ts` | 27 | Reference fixture roundtrip; OPCUA roundtrip incl. trigger field values; synthetic 2-laser roundtrip |
| `builder.test.ts` | 18 | `build()` in-memory; `save()` + read-back; correction grid shape/peak; no-clearbox path; schema validity |
| `capabilities.test.ts` | 8 | Capability API: `openMachineConfig`, `createMachineConfig`, `SetMode`, `fileVersion`, `save` |
| `adapterMigration.test.ts` | 8 | Mock v1.1 adapter isolation (AV-09), forward migration v1.0→v1.1 (AV-10), backward migration v1.1→v1.0 (AV-11) |

---

## Validation results

20 / 20 scenarios pass on this SDK. See the full table in
[docs/validation/nodejs/PASS_FAIL.md](../docs/validation/nodejs/PASS_FAIL.md) and verbatim
output in [docs/validation/nodejs/results.md](../docs/validation/nodejs/results.md).

Two real defects were found and fixed during the validation pass:
- **AV-05** — `attrFloat()` silently returned `null` for a corrupt attribute instead of throwing; fixed to throw `TypeError`.
- **AV-10/11** — `meta.extra` accumulated typed addition fields (`facility_id`/`config_author`) because the v1.0 base parser's `KNOWN_ROOT` set didn't know they were typed; fixed by stripping those keys from `extra` before assigning the typed fields.

For the master cross-language matrix see [docs/validation/README.md](../docs/validation/README.md).
