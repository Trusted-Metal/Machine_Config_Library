# Go Implementation Completion Plan

**Branch:** SD-1684  
**Date:** 2026-08-14  
**Status:** Reader complete. Writer, builder, and write-path CLI not yet implemented.

This document is the authoritative ground-truth plan for finishing the Go library.
Use the prompt in each section to drive implementation in a fresh AI chat session.
Complete steps in the numbered order — each step's prompt assumes prior steps are done.

---

## Ground-Truth State of the Go Module

### What is fully implemented

| Component | File |
|-----------|------|
| h5c read primitives (Open/Read/Group/Dataset) | `go/internal/h5c/h5c.go` |
| h5c write primitives (complete: 5 methods) | `go/internal/h5c/h5c.go` |
| All 15 model structs | `go/internal/models/models.go` |
| HDF5 reader — all fields, OPCUA, ClearBox, SFCF, grids | `go/capabilities/v1_0/hdf5/hdf5.go` |
| Type-dispatch attr helpers | `go/capabilities/v1_0/hdf5/helpers.go` |
| HDF5 path constants | `go/capabilities/v1_0/layout/layout.go` |
| Capabilities facade — Open, Create, Get*, Set*, GetOpcua | `go/capabilities/v1_0/file.go` |
| Public capabilities API + error types | `go/capabilities/file.go`, `types.go` |
| Merge / Replace generic logic | `go/capabilities/internal/api/api.go` |
| Public reader facade | `go/reader.go` |
| CLI: `export-json`, `correction-hash` | `go/cmd/machine-config-cli/main.go` |
| 17 reader tests + 9 capabilities tests | `go/reader_test.go`, `go/capabilities/file_test.go` |

### What is missing (ordered by dependency)

| # | Component | File to create | Blocked until |
|---|-----------|---------------|---------------|
| §5.1 | 3 h5c write primitives | `go/internal/h5c/h5c.go` (edit) | — |
| §5.2 | HDF5 v1.0 writer | `go/capabilities/v1_0/hdf5/writer.go` (new) | §5.1 |
| §5.3 | Wire `Save()` in capabilities | `go/capabilities/v1_0/file.go` (edit) | §5.2 |
| §5.4 | Public writer facade | `go/writer.go` (new) | §5.2 |
| §5.5 | CLI `write-hdf5` + `copy-hdf5` | `go/cmd/machine-config-cli/main.go` (edit) | §5.4 |
| §5.6 | `MockConfigBuilder` | `go/builder.go` (new) | §5.4 |
| §5.7 | Cross-check CI Phase 3 + 3.5 | `.github/workflows/cross_check.yml` (edit) | §5.5 |
| T-Go-1 | `TestSaveRoundTrip` (capabilities facade) | `go/capabilities/file_test.go` (edit) | §5.3 |
| T-Go-2 | Schema validation test | `go/reader_test.go` (edit) | §5.2 |
| T-Go-3 | Direct writer roundtrip tests (10 tests) | `go/writer_test.go` (new) | §5.4 |
| T-Go-4 | MockConfigBuilder tests (11 tests) | `go/builder_test.go` (new) | §5.6 |
| T-Rust | 13 tests (12 reader fields + 1 inverse correction writer) | `rust/tests/integration_test.rs` (edit) | — |
| T-Node-Reader | 1 inverse correction reader test | `nodejs/tests/reader.test.ts` (edit) | — |
| T-Node-Writer | 2 correction data hash writer tests | `nodejs/tests/writer.test.ts` (edit) | — |

---

## §5.1 — Add 3 Missing h5c Write Primitives

**File to edit:** `go/internal/h5c/h5c.go`

The h5c layer has `(*Group).WriteStringAttr`, `WriteFloat64Attr`, `WriteInt64Attr`, and
`CreateFloat64Dataset`, but the writer also needs to write attrs directly on `*Dataset`
objects (for the SFCF path) and write a uint8 dataset (for SFCF raw bytes).

**3 primitives added in §5.1, plus 2 open-variant addenda:**

1. `(*Dataset).WriteStringAttr(name, value string) error`
2. `(*Dataset).WriteInt64Attr(name string, value int64) error`
3. `(*Group).CreateUint8Dataset(name string, data []byte) error`
4. `(*Group).CreateFloat64DatasetOpen(name string, dims []uint64, data []float64) (*Dataset, error)` — like `CreateFloat64Dataset` but returns the dataset open for attribute writing; caller calls `Close()`
5. `(*Group).CreateUint8DatasetOpen(name string, data []byte) (*Dataset, error)` — same pattern for uint8

Use variants 4 and 5 in the writer wherever a dataset needs attributes attached after creation
(ClearBox correction grids and SFCF raw bytes). Use variants 1–3 for the closed forms.

### §5.1 Implementation Prompt

```
I am working in the Go module at go/ (module name: machine-config-go).
The CGo HDF5 wrapper lives in go/internal/h5c/h5c.go.

Read the full content of go/internal/h5c/h5c.go. Then add exactly three new exported functions
to that file:

1. (*Dataset).WriteStringAttr(name, value string) error
   - Write a variable-length UTF-8 string attribute on the dataset, identical in behaviour to
     (*Group).WriteStringAttr but operating on d.id instead of g.id.
   - Pattern: copy the Group version exactly, replace g.id → d.id.

2. (*Dataset).WriteInt64Attr(name string, value int64) error
   - Write a scalar int64 attribute on the dataset.
   - Pattern: use d.id, same structure as (*Group).writeScalar with C.H5T_NATIVE_INT64.

3. (*Group).CreateUint8Dataset(name string, data []byte) error
   - Create a 1-D contiguous uint8 dataset and write data into it.
   - Pattern: mirror CreateFloat64Dataset but with dims = []uint64{uint64(len(data))},
     C.H5T_NATIVE_UINT8 as the type, and unsafe.Pointer(&data[0]) as the write buffer.
   - Return error if len(data) == 0.

Do not modify any existing function. Add the three functions after the existing Dataset methods
(after ReadUint8). Keep the same comment/code style as the surrounding file.

After adding, verify the file compiles with: cd go && CGO_ENABLED=1 go build ./...
(on Linux; use the same CGo env vars as the CI workflow on Windows).
```

---

## §5.2 — Implement the HDF5 v1.0 Writer

**New file:** `go/capabilities/v1_0/hdf5/writer.go`

This is the largest step. The authoritative reference is `rust/src/capabilities/v1_0/writer.rs`
(~480 lines). Translate it into Go using the h5c primitives instead of the hdf5-metno crate.
The field-for-field mapping is exact — every HDF5 attribute name, every default unit fallback,
every NaN convention must match.

**Key translation rules from Rust → Go:**

| Rust helper | Go equivalent | Nil case |
|-------------|---------------|----------|
| `ws(grp, key, val)` | `grp.WriteStringAttr(key, val)` | pass `""` |
| `wf(grp, key, Some(v))` | `grp.WriteFloat64Attr(key, v)` | `grp.WriteStringAttr(key, "")` |
| `wi(grp, key, Some(v))` | `grp.WriteInt64Attr(key, int64(v))` | `grp.WriteStringAttr(key, "")` |
| `wb(grp, key, Some(true))` | `grp.WriteInt64Attr(key, 1)` | `grp.WriteStringAttr(key, "")` |
| `ws_ds(ds, key, val)` | `ds.WriteStringAttr(key, val)` | — |
| `wi_ds(ds, key, val)` | `ds.WriteInt64Attr(key, val)` | — |

**Correction grid flatten rule:** `ClearBox.CorrectionData` is `*[][][]*float64`.
Convert nil pointers to `math.NaN()` and write as a flat `[]float64` of length
257×257×2 = 132098 using `CreateFloat64Dataset`. If `CorrectionData` is nil, write
a zero-filled slice of the same length. Same for `InverseCorrectionData`.

**Power_Bit_Resolution** is stored as a string in real HDF5 files (confirmed by reading
reference fixture). Write with `WriteStringAttr`, not `WriteFloat64Attr`.

**Scanner axis subgroup rule** (all four languages follow identical nil-pointer logic):

| Go model field | Go type | HDF5 subgroup | Write condition |
|---|---|---|---|
| `s.XAxis` | `AxisConfig` (value) | `X_Axis/` | **Always** — value type, never nil |
| `s.YAxis` | `AxisConfig` (value) | `Y_Axis/` | **Always** — value type, never nil |
| `s.ZAxis` | `*AxisConfig` | `Z_Axis/` | Only if `s.ZAxis != nil` |
| `s.Focus` | `*AxisConfig` | `Focus/` | Only if `s.Focus != nil` |

The writer does **not** inspect `AxisConfiguration` string ("2D"/"3D"/"3D+Focus") to
decide which axes to write. It relies solely on Go pointer nil-ness — Python's
`__post_init__`, the schema, and the reader all ensure the model is self-consistent
before it reaches the writer. `AxisConfiguration` itself is `*string`: nil → `""`,
non-nil → write the value verbatim.

`writeAxis` takes a value receiver (`ax AxisConfig`, not a pointer). All 11 `AxisConfig`
fields are `*int`, `*float64`, or `*string`; nil → `""` via the standard helpers.

**SFCF raw bytes:** write the uint8 dataset using the new `CreateUint8Dataset`. If
`RawBytes` is nil, write a zero-length byte slice (length 0 is acceptable for the dataset
schema; the file_size attribute stores the canonical size).

**OPCUA triggers:** each trigger is a subgroup named by its display name (e.g.
`"Laser Emission Interlock"`). Enumerate `OpcuaConfig.Triggers` map, create one subgroup
per entry, write ID/Signal/Subsystem/Rule_Enabled/Start_Value/Stop_Value + extras.

### §5.2 Implementation Prompt

```
I am working in the Go module at go/ (module name: machine-config-go).
I need to create a new file: go/capabilities/v1_0/hdf5/writer.go

Context files to read first (read ALL of them):
  - rust/src/capabilities/v1_0/writer.rs  (authoritative reference — translate this)
  - go/capabilities/v1_0/hdf5/hdf5.go    (reader — mirrors what writer must produce)
  - go/capabilities/v1_0/layout/layout.go (HDF5 path constants)
  - go/internal/h5c/h5c.go               (the write primitives to use)
  - go/internal/models/models.go          (Go model types)

Task: Create go/capabilities/v1_0/hdf5/writer.go implementing:

  package hdf5

  // Write serialises cfg to a File_Version 1.0 HDF5 file at path.
  func Write(cfg *models.MachineConfig, path string) error

The function must produce an HDF5 file that the existing Parse() function in this same
package reads back identically. Use the Rust writer as the field-by-field reference.

Internal helpers (unexported, in this file):
  ws(g *h5c.Group, key, val string) error          // variable-length string attr
  wf(g *h5c.Group, key string, v *float64) error   // float64 or "" when nil
  wi(g *h5c.Group, key string, v *int) error        // int64(v) or "" when nil
  wb(g *h5c.Group, key string, v *bool) error       // 0/1 int64 or "" when nil
  wsDS(d *h5c.Dataset, key, val string) error       // string attr on dataset
  wiDS(d *h5c.Dataset, key string, val int64) error // int64 attr on dataset
  writeExtra(g *h5c.Group, key string, val any) error // re-type extra values
  writeAxis(g *h5c.Group, ax AxisConfig) error      // writes all 11 AxisConfig fields (value, not pointer)

  flattenGrid(data *[][][]*float64) []float64
    // converts nil ptrs to NaN; returns 257*257*2 zero slice if data is nil

Dataset creation — use the open variants when attrs must be written after the data:
  g.CreateFloat64DatasetOpen(name, dims, flat) → (*Dataset, error)  // caller calls ds.Close()
  g.CreateUint8DatasetOpen(name, data)          → (*Dataset, error)  // caller calls ds.Close()
  // Use the non-open forms (CreateFloat64Dataset / CreateUint8Dataset) only when no
  // attrs are needed on the dataset — they close the handle internally.

Rules:
  - nil *float64 → WriteStringAttr(key, "")  NOT omit
  - nil *int     → WriteStringAttr(key, "")
  - nil *bool    → WriteStringAttr(key, "")
  - nil *string  → WriteStringAttr(key, "")
  - non-nil unit *string with nil value field → write default unit string (see Rust)
  - Power_Bit_Resolution → always WriteStringAttr (stored as string in real files)

  Scanner axis subgroup rule (same in Rust, Python, Node.js, C++):
  - s.XAxis is AxisConfig (value, never nil) → ALWAYS create X_Axis group, call writeAxis
  - s.YAxis is AxisConfig (value, never nil) → ALWAYS create Y_Axis group, call writeAxis
  - s.ZAxis is *AxisConfig → create Z_Axis group and call writeAxis ONLY IF s.ZAxis != nil
  - s.Focus is *AxisConfig → create Focus group and call writeAxis ONLY IF s.Focus != nil
  - s.AxisConfiguration is *string → nil → "", non-nil → write verbatim ("2D"/"3D"/"3D+Focus")
  - The writer does NOT branch on the string value; nil-ness of the pointer fields is the gate
  - writeAxis writes all 11 AxisConfig fields; each *int/*float64/*string → nil → ""

  - ClearBox Correction_Data: use CreateFloat64DatasetOpen with dims [257,257,2]; then write
    dataset attrs (dimensions="H,W,D", dtype="float64", shape="257x257x2") on the returned *Dataset; then Close()
  - SFCF dataset: use CreateUint8DatasetOpen; then write dataset attrs (document_name, document_id,
    valid_as_of_date, document_created_at, document_type, original_uri, file_size) on the returned *Dataset; then Close()
  - OPCUA: create OPCUA/ group, Client/ and Pipe/ subgroups with attrs,
    Triggers/ subgroup with Triggers_Enabled attr + one subgroup per trigger name
  - Write extras (meta.Extra, client.Extra, trigger.Extra) via writeExtra

Write sections in this order (same as Rust):
  1. Root group attrs (MachineConfigMeta)
  2. Machine/ group (Machine fields)
  3. Machine/Optical_Trains/ → one subgroup per train:
       train attrs, Scanner/ + X_Axis/ + Y_Axis/ + optional Z_Axis/ + Focus/,
       Light_Source/, Collimator/, Scanner_Card/,
       Optional_Components/ClearBox/ (if clearbox present),
       scan_field_correction_file dataset (if SFCF present)
  4. OPCUA/ (if opcua present)

After creating the file, run: cd go && CGO_ENABLED=1 go build ./...
Fix any compile errors. Do not run tests yet.
```

---

## §5.3 — Wire `Save()` in the Capabilities File Facade

**File to edit:** `go/capabilities/v1_0/file.go`

The `Save()` method currently returns a stub error. After §5.2, replace it with a real
call to `hdf5.Write`.

### §5.3 Implementation Prompt

```
I am working in the Go module at go/ (module name: machine-config-go).

Read go/capabilities/v1_0/file.go in full.

The Save() method currently reads:

    func (f *File) Save(path string) *api.Error {
        if err := f.assertOpen(); err != nil {
            return err
        }
        out := path
        if out == "" {
            out = f.path
        }
        if out == "" {
            return api.Errf(api.ErrValidation, "save() requires a path for create()-d files")
        }
        _ = filepath.Clean(out)
        _ = os.DevNull
        return api.Errf(api.ErrValidation, "Go MachineConfigWriter not implemented yet (§5.7); in-memory set* works")
    }

Replace the body with a real implementation:
  1. Keep the assertOpen check and the out/path resolution logic unchanged.
  2. Call v1_0hdf5.Write(f.config, out). If it returns a non-nil error, wrap it in
     api.Errf(api.ErrIo, err.Error()) and return it.
  3. On success, set f.path = out and return nil.

The import for v1_0hdf5 is already present as:
  v1_0hdf5 "machine-config-go/capabilities/v1_0/hdf5"

Also remove the now-unused imports of os and filepath if they become unused after the change.

After editing, run: cd go && CGO_ENABLED=1 go build ./...
Then run: cd go && CGO_ENABLED=1 go test ./capabilities/... -run TestSaveRoundTrip -v
(TestSaveRoundTrip currently has t.Skip — that is fine; it will be un-skipped in T-Go-1.)
```

---

## §5.4 — Public Writer Facade

**New file:** `go/writer.go`

Mirrors `go/reader.go` in structure. Dispatches to the v1_0 adapter by File_Version.

### §5.4 Implementation Prompt

```
I am working in the Go module at go/ (module name: machine-config-go).

Read go/reader.go and go/file_version.go for context on the existing pattern.

Create go/writer.go in package machineconfig with:

  // MachineConfigWriter serialises a MachineConfig to an HDF5 file.
  type MachineConfigWriter struct{}

  // NewWriter returns a MachineConfigWriter.
  func NewWriter() *MachineConfigWriter { return &MachineConfigWriter{} }

  // Write serialises cfg to path, creating or overwriting the file.
  // Dispatches by cfg.Meta.FileVersion; defaults to "1.0" when empty.
  func (w *MachineConfigWriter) Write(cfg *MachineConfig, path string) error

The Write method must:
  - Trim whitespace from cfg.Meta.FileVersion; treat "" as "1.0".
  - For "1.0": call v1_0hdf5.Write(cfg, path) and return the error directly.
  - For any other version: return &UnsupportedFileVersionError{Version: fv}.

Import v1_0hdf5 as: v1_0hdf5 "machine-config-go/capabilities/v1_0/hdf5"

After creating the file, run: cd go && CGO_ENABLED=1 go build ./...
```

---

## §5.5 — CLI: `write-hdf5` and `copy-hdf5`

**File to edit:** `go/cmd/machine-config-cli/main.go`

Add two new subcommands. The current CLI handles `export-json` and `correction-hash`.
The run() switch just needs two new cases.

### §5.5 Implementation Prompt

```
I am working in the Go module at go/ (module name: machine-config-go).

Read go/cmd/machine-config-cli/main.go in full.

Add two new subcommand handlers to the run() switch:

  case "write-hdf5":
      return cmdWriteHDF5(args[1:])

  case "copy-hdf5":
      return cmdCopyHDF5(args[1:])

Also update the usage message in the default case to list all four subcommands.

Implement the two functions:

func cmdWriteHDF5(args []string) error
  Usage: write-hdf5 <input.json> <output.h5>
  - args[0] = path to a JSON file (or "-" for stdin)
  - args[1] = output .h5 path
  - Read JSON bytes from the file (or os.Stdin if "-")
  - json.Unmarshal into *machineconfig.MachineConfig
  - Call machineconfig.NewWriter().Write(cfg, args[1])
  - Return any error; on success print nothing.

func cmdCopyHDF5(args []string) error
  Usage: copy-hdf5 <src.h5> <dst.h5>
  - args[0] = source .h5 path
  - args[1] = destination .h5 path
  - Read: machineconfig.NewReader(args[0]).ParseWithOptions(machineconfig.ParseOptions{IncludeBinary: true})
  - Write: machineconfig.NewWriter().Write(cfg, args[1])
  - Return any error; on success print nothing.

Use only stdlib imports (os, encoding/json, fmt already imported).

After editing, build:
  cd go
  CGO_ENABLED=1 go build -o bin/machine-config-cli ./cmd/machine-config-cli/

Smoke-test both commands:
  echo '{"meta":{"machine_name":"test","file_version":"1.0",...}}' > /tmp/t.json
  # Or use a real fixture JSON:
  ./bin/machine-config-cli export-json fixtures/reference_config.h5 > /tmp/ref.json
  ./bin/machine-config-cli write-hdf5 /tmp/ref.json /tmp/ref_rt.h5
  ./bin/machine-config-cli export-json /tmp/ref_rt.h5
  ./bin/machine-config-cli copy-hdf5 fixtures/reference_config.h5 /tmp/copy.h5
  ./bin/machine-config-cli export-json /tmp/copy.h5
```

---

## §5.6 — MockConfigBuilder

**New file:** `go/builder.go`

Independent of steps §5.1–§5.5 but depends on §5.4 (writer). This is lower priority —
implement after the write path is CI-green. Needed for cross-check Phase 3 `build` path
and for the builder test parity table.

The Gaussian correction grid formula is identical across Python, Rust, and Node.js.
The Rust implementation is in `rust/src/builder.rs` — use it as the direct reference.

### §5.6 Implementation Prompt

```
I am working in the Go module at go/ (module name: machine-config-go).

Read the following files for context:
  - rust/src/builder.rs                   (authoritative reference — translate to Go)
  - go/capabilities/v1_0/file.go          (Create() shows the minimal stub config shape)
  - go/capabilities/v1_0/layout/layout.go (TrainID() helper)
  - go/internal/models/models.go          (model types)

Create go/builder.go in package machineconfig with:

  type MockConfigBuilder struct {
      NLasers         int     // default 2
      MachineName     string  // default "MockMachine"
      BuildPlateX     float64 // default 250.0
      BuildPlateY     float64 // default 250.0
      IncludeClearbox bool    // default true
  }

  func NewMockConfigBuilder() *MockConfigBuilder // returns builder with defaults above

  func (b *MockConfigBuilder) Build() *MachineConfig
    // Constructs the in-memory MachineConfig (same fields as Python/Rust/Node.js builder).
    // Gaussian correction grid formula: for each (i,j) in 257×257 grid,
    //   cx = float64(i) - 128.0
    //   cy = float64(j) - 128.0
    //   fwd[i][j][0] = 2.0 * exp(-(cx*cx + cy*cy) / (2.0 * 50.0 * 50.0))
    //   fwd[i][j][1] = fwd[i][j][0]
    //   inv[i][j][0] = fwd[i][j][0] * 0.9
    //   inv[i][j][1] = fwd[i][j][1] * 0.9
    // Cells where sqrt(cx*cx+cy*cy) > 120 → NaN in both grids (border masking).
    // Store grids as *[][][]*float64 on ClearBox (pointer to nil for out-of-field cells).
    // Fixed values for reproducibility (same as Rust/Node.js constants):
    //   meta.ExportDate = "2026-01-01T00:00:00.000Z"
    //   machine.ID = "00000000-0000-0000-0000-000000000000"
    //   ConfigurationHash = 64 zeros
    // For nLasers trains: train 0 has ScanHeadRotation=0, train 1 has ScanHeadRotation=180.
    // ScanHeadOffsetX alternates +0.0 / -0.0 (or 0.0 / 0.0) — see Rust builder.
    // If IncludeClearbox is false, OptionalComponents.Clearbox remains nil.

  func (b *MockConfigBuilder) Save(path string) error
    // Calls b.Build() then NewWriter().Write(cfg, path).

After creating the file, run: cd go && CGO_ENABLED=1 go build ./...
```

---

## §5.7 — Enable Go in Cross-Check Phase 3 and 3.5

**File to edit:** `.github/workflows/cross_check.yml`

After §5.5 is complete and CI-green for phases 1, 2, and 4, add Go to phases 3 and 3.5.

Also verify that `tools/cross_check.py` has Go registered in its `WRITERS` dict
(the dict that maps language → `write-hdf5` argv prefix).

### §5.7 Implementation Prompt

```
I am working in this repository. The CI workflow is at .github/workflows/cross_check.yml
and the cross-check script is at tools/cross_check.py.

Read BOTH files in full before making changes.

Task 1 — cross_check.yml:
Find these two steps:

  - name: Phase 3 — Write interoperability (all languages)
    run: python tools/cross_check.py --langs python,rust,nodejs,cpp --skip-schema ...

  - name: Phase 3.5 — Binary copy round-trip (all languages)
    run: python tools/cross_check.py --langs python,rust,nodejs,cpp --skip-schema ...

Change --langs to include go in both steps:
  --langs python,rust,nodejs,cpp,go

Also update the comment block at the top of the file (lines 7-9) that says
"Go participates in Phases 1, 2, 4" to say "Go participates in all phases."

Task 2 — tools/cross_check.py:
Find the WRITERS dict (or equivalent). Confirm Go is registered with the correct
write-hdf5 argv prefix pointing to go/bin/machine-config-cli (or .exe on Windows).
If Go is missing from WRITERS, add it using the same pattern as the other languages.
The binary path logic must match the BINARIES dict pattern (platform-aware .exe suffix).

After editing, do not run the workflow — just confirm the YAML parses and the Python
script has no syntax errors: python -c "import tools.cross_check" or
python tools/cross_check.py --help
```

---

## T-Go-1 — Enable and Implement `TestSaveRoundTrip`

> Tests `Save()` through the capabilities facade. Complements T-Go-3 which tests the writer directly.

**File to edit:** `go/capabilities/file_test.go`

The test currently contains only `t.Skip(...)`. Replace it with a real roundtrip test
that matches the scope of `nodejs/tests/writer.test.ts` (25 tests compressed into one
table-driven Go test function).

### T-Go-1 Implementation Prompt

```
I am working in the Go module at go/ (module name: machine-config-go).

Read these files for context:
  - go/capabilities/file_test.go       (current test file — replace the t.Skip body)
  - nodejs/tests/writer.test.ts        (reference scope for what to assert)
  - rust/src/capabilities/v1_0/writer.rs (roundtrip_* test functions at bottom)
  - go/capabilities/v1_0/file.go       (the Save() API)

In go/capabilities/file_test.go, find TestSaveRoundTrip:

    func TestSaveRoundTrip(t *testing.T) {
        t.Skip("Go writer §5.7 not implemented — re-enable when Save() is complete")
    }

Replace the entire body (remove the t.Skip) with a real test that does:

1. Reference fixture roundtrip:
   - OpenMachineConfig(reference_config.h5)
   - GetMeta() — record original machine_name and configuration_hash (len == 64)
   - GetScanner(0) — record working_distance and manufacturer
   - Save(t.TempDir() + "/rt.h5")
   - OpenMachineConfig the saved file
   - Assert meta.machine_name matches original
   - Assert len(meta.configuration_hash) == 64
   - Assert GetScanner(0).WorkingDistance matches original
   - Assert GetScanner(0).Manufacturer matches original
   - Assert OpticalTrainCount() == 2

2. OPCUA fixture roundtrip:
   - OpenMachineConfig(reference_config_opcua.h5)
   - GetOpcua() — record server_url
   - Save(t.TempDir() + "/opcua_rt.h5")
   - OpenMachineConfig the saved file
   - Assert GetOpcua() returns non-error
   - Assert server_url matches

3. Synthetic 2-laser roundtrip:
   - OpenMachineConfig(synthetic_2laser.h5)
   - OpticalTrainCount() == 2
   - GetScanner(0) and GetScanner(1) serial numbers differ
   - Save(t.TempDir() + "/synth_rt.h5")
   - Reopen, assert OpticalTrainCount() == 2 and serial numbers still differ

Use subtests (t.Run("reference roundtrip", ...), t.Run("opcua roundtrip", ...),
t.Run("synthetic roundtrip", ...)) so failures are identifiable.

Use the existing fixturesDir(t) helper that is already in the file.

After editing, run: cd go && CGO_ENABLED=1 go test ./capabilities/... -run TestSaveRoundTrip -v
All three subtests must pass.
```

---

## T-Go-2 — Schema Validation Test

One new test that validates the JSON output of the reference fixture against the canonical
schema. Mirrors `test_json_output_validates_schema` in `rust/tests/integration_test.rs`.

### T-Go-2 Implementation Prompt

```
I am working in the Go module at go/ (module name: machine-config-go).

Read these files:
  - go/reader_test.go                    (add the new test here)
  - rust/tests/integration_test.rs       (see test_json_output_validates_schema for scope)
  - schema/machine_config_v1.schema.json (the schema to validate against)

Add a new test function TestJSONSchemaValidation to go/reader_test.go.

The test must:
1. Parse fixtures/reference_config.h5 using NewReader().Parse()
2. Marshal the result to JSON using encoding/json
3. Unmarshal the JSON into a map[string]any
4. Assert the following structural invariants (same assertions as Rust test):
   - Top-level keys "meta", "machine", "optical_trains" are all present
   - meta["machine_name"] is a non-empty string
   - meta["configuration_hash"] is a string of length 64
   - optical_trains is a []any with length >= 1
   - Each train has a "train_id" key
   - Each train has a "scanner" key
5. Load schema/machine_config_v1.schema.json relative to the repo root
   (use runtime.Caller(0) to find the test file location, then ../../../schema/...)
6. Assert the schema file is valid JSON (os.ReadFile + json.Unmarshal into any)

Do NOT add a JSON Schema validation library dependency. The structural key-presence
checks are sufficient — they match what Rust's test does.

The fixtures root path helper repoRoot(t) can be derived from fixturesDir(t) which is
already in reader_test.go — use that to locate the schema file.

After adding the test, run:
  cd go && CGO_ENABLED=1 go test . -run TestJSONSchemaValidation -v
The test must pass.
```

---

## T-Rust — 12 Reader Field Tests

**File to edit:** `rust/tests/integration_test.rs`

These add field-level reader coverage that is currently absent from Rust but present in
Go, Python, and Node.js. All tests use fixtures already loaded in the file.

### T-Rust Implementation Prompt

```
I am working in a Rust crate at rust/.

Read rust/tests/integration_test.rs in full. Note the existing fixture path constants:
  const REFERENCE: &str = ...
  const REFERENCE_OPCUA: &str = ...
  const SYNTHETIC: &str = ...

Add the following 12 new #[test] functions to the file. Each follows the same pattern
as the existing tests (open fixture → parse → assert field).

1. test_machine_serial_number
   - Parse REFERENCE, assert machine.serial_number is non-empty string

2. test_machine_gas_flow_direction
   - Parse REFERENCE, assert machine.gas_flow_direction.is_some()

3. test_build_plate_y
   - Parse REFERENCE, assert machine.build_plate_y ≈ 250.0 (within 1.0)

4. test_build_plate_z
   - Parse REFERENCE, assert machine.build_plate_z.is_some()

5. test_train_ids
   - Parse SYNTHETIC (2-laser fixture), assert:
     optical_trains[0].train_id == "Optical_Train_01"
     optical_trains[1].train_id == "Optical_Train_02"

6. test_scan_head_rotation_two_lasers
   - Parse SYNTHETIC, assert:
     optical_trains[0].scanner.scan_head_rotation ≈ Some(0.0)
     optical_trains[1].scanner.scan_head_rotation ≈ Some(180.0)
   - Use: (val - expected).abs() < 0.01

7. test_scan_head_offset_y_two_lasers
   - Parse SYNTHETIC, assert both trains have scan_head_offset_y that is Some
     and that the two values differ (opposite sign or different magnitude)

8. test_light_source_wavelength
   - Parse REFERENCE, assert optical_trains[0].light_source.wavelength.is_some()
     and the value is > 0.0

9. test_scanner_card_sample_period
   - Parse REFERENCE, assert optical_trains[0].scanner_card.sample_period.is_some()
     and the value is > 0.0

10. test_thermal_lensing_both_trains
    - Parse REFERENCE, assert:
      optical_trains[0].thermal_lensing_passed == Some(false)
      optical_trains[1].thermal_lensing_passed == Some(true)

11. test_sfcf_file_size
    - Parse REFERENCE, assert:
      optical_trains[0].scan_field_correction_file.as_ref().unwrap().file_size > 0

12. test_inverse_correction_differs_from_forward
    - Use MachineConfigReader::open(REFERENCE)
    - Call reader.get_correction_data(0) and reader.get_inverse_correction_data(0)
    - Assert the first finite (non-NaN) value in each differs
    - (The reference fixture has correction_data and inverse_correction_data that differ)

13. test_writer_roundtrip_inverse_correction_data_checksum
    - Same structure as the existing test_writer_roundtrip_correction_data_checksum,
      but using get_inverse_correction_data(0) instead of get_correction_data(0).
    - Use parse_with_binary() so the model has the inverse grid before writing.
    - Write to a NamedTempFile, reopen, call get_inverse_correction_data(0) again.
    - Assert the bit-hash of the original inverse data equals the hash after roundtrip.
    - Note: the existing test_writer_roundtrip_correction_data_checksum covers the forward
      grid; this test is the companion for the inverse grid, which the inline writer.rs
      tests cover but integration_test.rs did not.

After adding all 13 tests, run: cd rust && cargo test 2>&1 | tail -20
All new tests must pass with no existing tests broken.
```

---

## T-Go-3 — Direct Writer Roundtrip Tests

**New file:** `go/writer_test.go`

T-Go-1 tests the writer through the capabilities `Save()` facade. This section tests
`NewWriter().Write()` directly, field by field — the same coverage Rust provides through
`integration_test.rs` (`test_writer_roundtrip_*`) and the inline `writer.rs` tests.
These tests catch writer bugs that the capabilities layer might mask.

### T-Go-3 Implementation Prompt

```
I am working in the Go module at go/ (module name: machine-config-go).

Read the following for context:
  - go/reader.go          (NewReader + ParseWithOptions)
  - go/writer.go          (NewWriter + Write)
  - go/reader_test.go     (repoRoot / fixtureDir pattern to copy)
  - rust/tests/integration_test.rs  (test_writer_roundtrip_* tests — translate these)
  - rust/src/capabilities/v1_0/writer.rs  (roundtrip_* inline tests — translate these)

Create go/writer_test.go in package machineconfig_test with a roundtrip() helper:

  func roundtrip(t *testing.T, src string) *machineconfig.MachineConfig {
      t.Helper()
      original, err := machineconfig.NewReader(src).Parse()
      if err != nil { t.Fatal(err) }
      tmp := filepath.Join(t.TempDir(), "rt.h5")
      if err := machineconfig.NewWriter().Write(original, tmp); err != nil { t.Fatal(err) }
      rt, err := machineconfig.NewReader(tmp).Parse()
      if err != nil { t.Fatal(err) }
      return rt
  }

Then add these test functions (use the fixture path helper from reader_test.go):

TestWriterRoundtripMetaFields
  - roundtrip(REFERENCE) and compare meta.machine_name, meta.manufacturer,
    meta.configuration_hash, meta.file_version, meta.export_date

TestWriterRoundtripMachineFields
  - roundtrip(REFERENCE) and compare machine.build_plate_x, build_plate_x_unit,
    gas_flow_direction, recoat_direction

TestWriterRoundtripTrainCount
  - roundtrip(REFERENCE) and assert len(optical_trains) matches original

TestWriterRoundtripScannerOffsets
  - roundtrip(REFERENCE) and compare optical_trains[0].scanner.scan_head_offset_x
    and working_distance

TestWriterRoundtripAxisConfiguration
  - roundtrip(REFERENCE), for train 0 assert:
    * scanner.axis_configuration matches original (expect "3D")
    * scanner.z_axis is non-nil (reference fixture has "3D" → Z_Axis written, Focus absent)
    * scanner.focus is nil (Focus group absent for "3D")
    * scanner.x_axis.smoothing_kernel matches original (X_Axis always present)TestWriterRoundtripThermalLensing
  - roundtrip(REFERENCE), assert thermal_lensing_passed matches for both trains

TestWriterRoundtripClearboxScalars
  - roundtrip(REFERENCE), assert for train 0:
    * clearbox.ip_address matches
    * clearbox.data_port matches
    * clearbox.show_console matches
    * clearbox.correction_grid_domain_shape matches

TestWriterRoundtripSFCFMetadata
  - roundtrip(REFERENCE), assert for train 0:
    * scan_field_correction_file.document_name matches
    * scan_field_correction_file.file_size matches
    * scan_field_correction_file.document_id matches

TestWriterRoundtripCorrectionDataHash
  - Read REFERENCE with ParseWithOptions{IncludeBinary: true}
  - GetCorrectionData(0) before write → hash the []float64 slice (SHA-256 of
    little-endian float64 bytes — same formula as the correction-hash CLI command)
  - Write to tmp, then GetCorrectionData(0) from tmp → hash
  - Assert hashes match
  - Repeat for GetInverseCorrectionData(0)

TestWriterRoundtripOPCUA
  - roundtrip(REFERENCE_OPCUA), assert:
    * opcua.client.server_url matches
    * opcua.client.session_timeout matches
    * opcua.client.bfs_max_depth matches
    * opcua.pipe.buffer_size matches
    * opcua.triggers_enabled matches
    * len(opcua.triggers) matches
    * "Chamber Oxygen Level" trigger: signal, subsystem, rule_enabled, start_value,
      stop_value all match

TestWriterRejectsUnknownFileVersion
  - Parse REFERENCE, set cfg.Meta.FileVersion = "2.0"
  - Call NewWriter().Write(cfg, t.TempDir()+"/bad.h5")
  - Assert the returned error is *UnsupportedFileVersionError
  - Assert err.(*UnsupportedFileVersionError).Version == "2.0"

After creating the file, run:
  cd go && CGO_ENABLED=1 go test . -run TestWriter -v
All 10 tests must pass.
```

---

## T-Go-4 — MockConfigBuilder Tests

**New file:** `go/builder_test.go`

Mirrors `rust/tests/integration_test.rs::test_builder_roundtrip` and the builder tests
in Python (`test_builder.py`, 7 tests) and Node.js (`builder.test.ts`, 18 tests).
Depends on §5.6 (builder implementation).

### T-Go-4 Implementation Prompt

```
I am working in the Go module at go/ (module name: machine-config-go).

Read the following for context:
  - go/builder.go                      (MockConfigBuilder — implement §5.6 first)
  - rust/tests/integration_test.rs     (test_builder_roundtrip)
  - rust/src/builder.rs                (field values and grid formula)
  - nodejs/tests/builder.test.ts       (scope of builder tests to match)

Create go/builder_test.go in package machineconfig_test.

Add these test functions:

TestMockBuilderDefaultsToTwoLasers
  - NewMockConfigBuilder().Build()
  - Assert len(optical_trains) == 2

TestMockBuilderNLasersOverride
  - builder := NewMockConfigBuilder(); builder.NLasers = 1; builder.Build()
  - Assert len(optical_trains) == 1

TestMockBuilderMachineName
  - NewMockConfigBuilder().Build()
  - Assert cfg.Meta.MachineName == cfg.Machine.MachineName (non-empty)

TestMockBuilderBuildPlateDimensions
  - NewMockConfigBuilder().Build()
  - Assert machine.build_plate_x ≈ 250.0, build_plate_y ≈ 250.0

TestMockBuilderConfigurationHashLength
  - NewMockConfigBuilder().Build()
  - Assert len(cfg.Meta.ConfigurationHash) == 64

TestMockBuilderCorrectionGridShape
  - builder := NewMockConfigBuilder(); builder.IncludeClearbox = true
  - cfg := builder.Build()
  - GetCorrectionData() is not available in-memory; instead check directly:
    cb := cfg.OpticalTrains[0].OptionalComponents.Clearbox
    Assert cb != nil
    cd := *cb.CorrectionData
    Assert len(cd) == 257 (first dim) and len(cd[0]) == 257 (second dim) and len(cd[0][0]) == 2

TestMockBuilderCorrectionGridPeak
  - Build() with clearbox=true
  - Centre cell cd[128][128][0] must be non-nil and ≈ 2.0 (within 0.01)

TestMockBuilderInverseGridRatio
  - Build() with clearbox=true
  - Centre cell of InverseCorrectionData[128][128][0] ≈ 0.9 × CorrectionData[128][128][0]

TestMockBuilderNoClearboxPath
  - builder := NewMockConfigBuilder(); builder.IncludeClearbox = false; cfg := builder.Build()
  - Assert cfg.OpticalTrains[0].OptionalComponents.Clearbox == nil

TestMockBuilderSaveRoundtrip
  - NewMockConfigBuilder().Save(t.TempDir() + "/mock.h5")
  - NewReader("mock.h5").Parse()
  - Assert len(optical_trains) == 2
  - Assert meta.machine_name non-empty
  - Assert optical_trains[0].scanner.working_distance non-nil

TestMockBuilderScanHeadRotation
  - Build() with NLasers=2
  - Assert optical_trains[0].scanner.scan_head_rotation ≈ 0.0
  - Assert optical_trains[1].scanner.scan_head_rotation ≈ 180.0

After creating the file, run:
  cd go && CGO_ENABLED=1 go test . -run TestMockBuilder -v
All tests must pass.
```

---

## T-Node-Reader — 1 Inverse Correction Reader Test

**File to edit:** `nodejs/tests/reader.test.ts`

One new `it()` block verifying that the reader returns different data for forward vs inverse grids.

### T-Node-Reader Implementation Prompt

```
I am working in the Node.js package at nodejs/.

Read nodejs/tests/reader.test.ts in full to understand the test structure and the
REFERENCE fixture path constant.

Add one new it() block inside the appropriate describe block (the one covering
getCorrectionData / getInverseCorrectionData). The test:

  it('getInverseCorrectionData differs from getCorrectionData for the same train', async () => {
    const reader = new MachineConfigReader(REFERENCE);
    const fwd = await reader.getCorrectionData(0);
    const inv = await reader.getInverseCorrectionData(0);
    // Shapes must match
    expect(inv.shape).toEqual(fwd.shape);
    // Find the first finite value in each array
    const firstFiniteFwd = Array.from(fwd.data).find(v => isFinite(v));
    const firstFiniteInv = Array.from(inv.data).find(v => isFinite(v));
    expect(firstFiniteFwd).toBeDefined();
    expect(firstFiniteInv).toBeDefined();
    // The grids must differ — inverse ≠ forward by construction
    expect(Math.abs(firstFiniteInv! - firstFiniteFwd!)).toBeGreaterThan(0.0001);
  });

After adding the test, run: cd nodejs && npm test
The new test must pass and the total test count must increase by 1.
```

---

## T-Node-Writer — Correction Data Hash Preservation Tests

**File to edit:** `nodejs/tests/writer.test.ts`

The existing Node.js writer tests verify scalar fields (machine_name, build_plate_x,
working_distance, SFCF metadata, ClearBox ip_address, OPCUA fields) but do not verify
that the binary correction data datasets survive a write→read roundtrip with bit-identical
contents. This is the only language other than Rust that has a writer and does not test this.
The cross_check Phase 4 validates correction hash parity across languages, so any regression
here shows up in CI but not in unit tests without these additions.

### T-Node-Writer Implementation Prompt

```
I am working in the Node.js package at nodejs/.

Read the following for context:
  - nodejs/tests/writer.test.ts   (existing writer tests — add inside this file)
  - nodejs/src/writer.ts          (MachineConfigWriter)
  - nodejs/src/reader.ts          (MachineConfigReader + getCorrectionData / getInverseCorrectionData)
  - nodejs/src/cli.ts             (correction-hash command — shows the hash formula)

The correction hash formula used by the CLI (and by cross_check Phase 4) is:
  SHA-256 of the flat Float64Array reinterpreted as little-endian bytes.
In Node.js this is:
  import { createHash } from 'node:crypto';
  function correctionHash(data: Float64Array): string {
    const buf = Buffer.from(data.buffer, data.byteOffset, data.byteLength);
    return createHash('sha256').update(buf).digest('hex');
  }

Add a new describe block to writer.test.ts:

  describe('MachineConfigWriter — correction data hash roundtrip', () => {

    it('forward correction data hash is preserved across write roundtrip', async () => {
      // Read original correction data
      const srcReader = new MachineConfigReader(REFERENCE);
      const originalFwd = await srcReader.getCorrectionData(0);
      const originalHash = correctionHash(originalFwd.data);

      // Write the config to a temp file, then read back
      const src = await srcReader.parse({ includeBinary: true });
      const tmp = join(tmpdir(), `fwd_rt_${Date.now()}.h5`);
      try {
        const writer = new MachineConfigWriter(src);
        await writer.write(tmp);
        const rtReader = new MachineConfigReader(tmp);
        const rtFwd = await rtReader.getCorrectionData(0);
        expect(correctionHash(rtFwd.data)).toBe(originalHash);
      } finally {
        rmSync(tmp, { force: true });
      }
    });

    it('inverse correction data hash is preserved across write roundtrip', async () => {
      const srcReader = new MachineConfigReader(REFERENCE);
      const originalInv = await srcReader.getInverseCorrectionData(0);
      const originalHash = correctionHash(originalInv.data);

      const src = await srcReader.parse({ includeBinary: true });
      const tmp = join(tmpdir(), `inv_rt_${Date.now()}.h5`);
      try {
        const writer = new MachineConfigWriter(src);
        await writer.write(tmp);
        const rtReader = new MachineConfigReader(tmp);
        const rtInv = await rtReader.getInverseCorrectionData(0);
        expect(correctionHash(rtInv.data)).toBe(originalHash);
      } finally {
        rmSync(tmp, { force: true });
      }
    });
  });

Add the required imports at the top of the file (if not already present):
  import { createHash } from 'node:crypto';
  import { join } from 'node:path';
  import { tmpdir } from 'node:os';
  import { rmSync } from 'node:fs';

Define correctionHash() as a module-level helper before the describe blocks.

Use the existing REFERENCE constant for the fixture path.
Ensure includeBinary: true is passed to parse() so correction data is loaded before writing.

After adding the tests, run: cd nodejs && npm test
Both new tests must pass and the total test count must increase by 2.
```

---

## Completion Checklist

Use this to track progress. Check off each item after its CI step is green.

| Item | Status |
|------|--------|
| §5.1 — h5c write primitives (3 additions) | ✅ |
| §5.2 — `go/capabilities/v1_0/hdf5/writer.go` | ✅ |
| §5.3 — `Save()` wired in `file.go` | ✅ |
| §5.4 — `go/writer.go` public facade | ✅ |
| §5.5 — `write-hdf5` + `copy-hdf5` CLI | ✅ |
| §5.6 — `MockConfigBuilder` in `go/builder.go` | ✅ |
| §5.7 — cross_check.yml Phase 3 + 3.5 includes Go | ✅ |
| T-Go-1 — `TestSaveRoundTrip` (capabilities facade) enabled and passing | ✅ |
| T-Go-2 — `TestJSONSchemaValidation` passing | ✅ |
| T-Go-3 — 10 direct writer roundtrip tests passing | ✅ |
| T-Go-4 — 11 MockConfigBuilder tests passing | ✅ |
| T-Rust — 13 tests passing (12 reader fields + 1 inverse correction writer) | ✅ |
| T-Node-Reader — 1 inverse correction reader test passing | ✅ |
| T-Node-Writer — 2 correction data hash writer tests passing | ✅ |
| CI: go.yml green (all platforms) | ⬜ |
| CI: cross_check.yml green (phases 1-4, all platforms) | ⬜ |

---

## Verification Commands

After each step, run these locally to confirm nothing is broken before pushing.

**Go (run from `go/` dir from within MSYS2 shell — `bash -l`):**
```bash
# Windows — run via MSYS2 bash (PATH includes /mingw64/bin where HDF5 lives)
# Shortcut: & "C:\msys64\usr\bin\bash.exe" -l C:/Temp/build_h5c.sh
export MSYSTEM=MINGW64
export PATH="/mingw64/bin:/c/Program Files/Go/bin:$PATH"
export CGO_ENABLED=1
export CC=$(cygpath -w /mingw64/bin/gcc.exe)
export CXX=$(cygpath -w /mingw64/bin/g++.exe)
cd /c/Users/ChrisParham/Desktop/Repo/Machine_Config_Library/go
go build ./...
go test ./... -count=1

# Linux
CGO_ENABLED=1 go build ./...
CGO_ENABLED=1 go test ./... -v
```

**Rust:**
```bash
cd rust && cargo test
```

**Node.js:**
```bash
cd nodejs && npm test
```

**Cross-check (all phases, after §5.5):**
```bash
# From repo root — requires Go bin to be built, Python venv active
python tools/cross_check.py --langs python,rust,nodejs,cpp,go --verbose
```
