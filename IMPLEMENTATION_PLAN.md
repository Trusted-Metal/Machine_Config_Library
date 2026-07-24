# Machine Config Library — Full Implementation Plan

---

## Developer Quick Reference

This section is updated after each phase. It is the single place to look up how to run tests, what each test suite covers, and what tools exist.

---

### Environment Setup

A virtual environment (`.venv/`) isolates this project's packages from your system Python. It is **not a running service** — it is just a directory. Activating it only changes the current terminal's `PATH` so that `python` resolves to `.venv\Scripts\python.exe` instead of the system Python. Every new terminal starts fresh; you must activate again or use the direct path.

**One-time setup (run once per machine / clone):**

```powershell
# PowerShell — from repo root
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e python/                 # installs package + all declared dependencies
pip install pytest                     # test runner (not a project dep)
```

```bash
# Git Bash — from repo root
python -m venv .venv
source .venv/Scripts/activate          # Windows Git Bash path
# source .venv/bin/activate            # macOS / Linux path
pip install -e python/
pip install pytest
```

**Every new terminal session — choose one:**

```powershell
# PowerShell — Option 1: activate, then use plain 'python'
.\.venv\Scripts\Activate.ps1
python -m pytest python/tests/ -v

# PowerShell — Option 2: call the venv Python directly (no activation needed)
.\.venv\Scripts\python.exe -m pytest python/tests/ -v
```

```bash
# Git Bash — Option 1: activate, then use plain 'python'
source .venv/Scripts/activate
python -m pytest python/tests/ -v

# Git Bash — Option 2: call the venv Python directly (no activation needed)
.venv/Scripts/python.exe -m pytest python/tests/ -v
```

> **VS Code tip**: Select `.venv\Scripts\python.exe` as the workspace interpreter (click the Python version in the bottom-left status bar). VS Code's integrated terminal will then activate the venv automatically on every new terminal, so you never have to activate manually.

> The `.venv/` directory is git-ignored. Re-create it on any new clone with the one-time setup above.

---

### Test Suites

| Suite | File | Phase | Command | What it covers |
|---|---|---|---|---|
| Schema self-tests | `python/tests/test_schema.py` | 0.4 | `.\.venv\Scripts\python.exe -m pytest python/tests/test_schema.py -v` | Schema parses as JSON; draft 2020-12 meta-validation; all `required` constraints; `minItems`/`maxItems`; hash length; golden file validation (skipped until Phase 1.7) |
| Generator tests | `python/tests/test_generator.py` | 0.7 | `.\.venv\Scripts\python.exe -m pytest python/tests/test_generator.py -v` | Generator runs cleanly on empty spec dir; all 4 templates are valid Jinja2; header, version strings, all 3 change types, empty-changes rendering |
| Reader tests | `python/tests/test_reader.py` | 1.3/1.6 | `.\.venv\Scripts\python.exe -m pytest python/tests/test_reader.py -v` | Full parse of real AconityMIDI `.h5` fixtures; all HDF5 → model field mappings; Rule 8 unit locking; ClearBox; scan-field correction file; thermal lensing; `get_raw_group`; JSON/schema round-trip; MockConfigBuilder roundtrip; unit-mismatch error; absent ClearBox; File_Version warning; OPCUA group values (Client/Pipe/Triggers/trigger subgroups) |
| CLI tests | `python/tests/test_cli.py` | 1.4/1.5 | `.\.venv\Scripts\python.exe -m pytest python/tests/test_cli.py -v` | `inspect`, `validate`, `export-json` against real fixture; `write`, `build --mock`, `build --from-yaml`, `demo` (Phase 1.5 implemented); `--version`, `--help` |
| Writer tests | `python/tests/test_writer.py` | 1.6 | `.\.venv\Scripts\python.exe -m pytest python/tests/test_writer.py -v` | HDF5 write → re-parse roundtrip for machine name, hash, build plate, train count, scanner offsets, thermal lensing, null fields, schema validity |
| Builder tests | `python/tests/test_builder.py` | 1.6 | `.\.venv\Scripts\python.exe -m pytest python/tests/test_builder.py -v` | `MockConfigBuilder` (1 and 2 lasers, plate dims, correction grid shape/non-zero); `YamlConfigBuilder` roundtrip; `ConfigEditor` offset mutation |

> **Run all Python tests at once** (as the suite grows):
> ```powershell
> # PowerShell
> .\.venv\Scripts\python.exe -m pytest python/tests/ -v
> ```
> ```bash
> # Git Bash
> .venv/Scripts/python.exe -m pytest python/tests/ -v
> ```
> Using the direct venv path ensures the correct interpreter regardless of shell.

---

### Command-Line Interface

The `machine-config` command is installed automatically by `pip install -e python/`.

```powershell
# PowerShell
.\.venv\Scripts\machine-config.exe inspect fixtures/reference_config.h5
.\.venv\Scripts\machine-config.exe validate fixtures/reference_config.h5
.\.venv\Scripts\machine-config.exe export-json fixtures/reference_config.h5
.\.venv\Scripts\machine-config.exe export-json fixtures/reference_config.h5 --output config.json
.\.venv\Scripts\machine-config.exe --help
```

```bash
# Git Bash — forward slashes, same .exe
.venv/Scripts/machine-config.exe inspect fixtures/reference_config.h5
.venv/Scripts/machine-config.exe validate fixtures/reference_config.h5
.venv/Scripts/machine-config.exe export-json fixtures/reference_config.h5
.venv/Scripts/machine-config.exe export-json fixtures/reference_config.h5 --output config.json
.venv/Scripts/machine-config.exe --help
```

```bash
# Either shell — activate first, then the plain command name works
source .venv/Scripts/activate     # bash
# .\.venv\Scripts\Activate.ps1   # PowerShell equivalent
machine-config inspect fixtures/reference_config.h5
```

> `write`, `build`, and `demo` are registered but exit with code 2 until Phase 1.5 (`builder.py`) is complete.

---

**Phase 1.5 additions** (not in the Phase 1.4 list above):

```powershell
# PowerShell
.\.venv\Scripts\machine-config.exe inspect fixtures/reference_config.h5 --verbose
.\.venv\Scripts\machine-config.exe write config.json --output config.h5
.\.venv\Scripts\machine-config.exe build --mock --output test_config.h5
.\.venv\Scripts\machine-config.exe build --mock --lasers 1 --output single_laser.h5
.\.venv\Scripts\machine-config.exe build --from-yaml spec.yaml --output config.h5
.\.venv\Scripts\machine-config.exe demo
```

```bash
# Git Bash
.venv/Scripts/machine-config.exe write config.json --output config.h5
.venv/Scripts/machine-config.exe build --mock --output test_config.h5
.venv/Scripts/machine-config.exe build --from-yaml spec.yaml --output config.h5
.venv/Scripts/machine-config.exe demo
```

> All six commands are fully implemented as of Phase 1.5.

---

### Files Produced So Far

| File | Phase | Description |
|---|---|---|
| `schema/machine_config_v1.schema.json` | 0.2 | Canonical JSON Schema — cross-language contract |
| `fixtures/reference_config.h5` | 0.1 | Real AconityMIDI machine config (2-laser, no OPCUA) |
| `fixtures/reference_config_opcua.h5` | 0.3 | Real AconityMIDI machine config (2-laser, with OPCUA group populated) |
| `python/tests/test_schema.py` | 0.4 | Schema self-test suite (15 tests + 1 deferred) |
| `tools/generate_adapters.py` | 0.7 | Jinja2 adapter boilerplate generator — run after any schema version bump |
| `tools/templates/adapter_python.py.j2` | 0.7 | Jinja2 template → Python adapter class |
| `tools/templates/adapter_typescript.ts.j2` | 0.7 | Jinja2 template → TypeScript adapter function |
| `tools/templates/adapter_rust.rs.j2` | 0.7 | Jinja2 template → Rust adapter function (operates on `serde_json::Value`) |
| `tools/templates/adapter_cpp.hpp.j2` | 0.7 | Jinja2 template → C++ adapter function (operates on `nlohmann::json`) |
| `python/tests/test_generator.py` | 0.7 | Generator test suite (29 tests across all 4 templates) |
| `python/pyproject.toml` | 1.1 | Package build config; `[tool.pytest.ini_options]` sets `pythonpath = ["src"]` and `testpaths = ["tests"]`; `[project.dependencies]` and `[project.scripts]` entry point `machine-config` |
| `python/src/machine_config/` (stubs) | 1.1 | Package scaffold: `__init__.py`, `models.py`, `reader.py`, `writer.py`, `builder.py`, `schema.py`, `cli.py`, `adapters/__init__.py`, `adapters/base.py` |
| `python/tests/` (stubs) | 1.1 | Test scaffold: `conftest.py`, `test_reader.py`, `test_writer.py`, `test_builder.py`, `test_adapters.py` |
| `python/src/machine_config/models.py` | 1.2 | All 11 dataclasses — full type-annotated model layer; hybrid unit field naming throughout |
| `python/src/machine_config/schema.py` | 1.2 | Schema loader — resolves `schema/machine_config_v1.schema.json` from repo root; exposes `SCHEMA: dict` |
| `python/src/machine_config/__init__.py` | 1.5 | Package exports: 11 model classes + `MachineConfigReader`, `MachineConfigWriter`, `MockConfigBuilder`, `YamlConfigBuilder`, `ConfigEditor`, `config_from_dict` |
| `python/src/machine_config/reader.py` | 1.3/1.5 | `MachineConfigReader` — parses `.h5` → `MachineConfig`; Rule 8 unit locking; `to_json()` produces schema-valid output; `config_from_dict()` module-level deserialiser (JSON dict → `MachineConfig`) |
| `python/tests/conftest.py` | 1.3 | Session-scoped fixtures: `reference_reader`, `reference_config`, `opcua_reader` |
| `python/tests/test_reader.py` | 1.3/1.6 | Reader test suite: 61 tests across 10 classes (includes `TestOpcua` with 12 OPCUA value tests against `reference_config_opcua.h5`; `scan_head_offset_y` asserted for both trains) |
| `python/src/machine_config/cli.py` | 1.4/1.5 | `machine-config` CLI: all 6 commands fully implemented (`inspect --verbose`, `validate`, `export-json`, `write`, `build --mock/--from-yaml`, `demo`) |
| `python/tests/test_cli.py` | 1.4/1.5 | CLI test suite: 40 tests across 5 classes using Click `CliRunner` |
| `python/src/machine_config/writer.py` | 1.5 | `MachineConfigWriter` — serialises `MachineConfig` → machine-config-schema-compatible `.h5`; machine-agnostic; exact mirror of `reader.py` type conventions |
| `python/src/machine_config/builder.py` | 1.5 | `MockConfigBuilder` (synthetic configs + Gaussian correction grids), `YamlConfigBuilder` (YAML spec → HDF5), `ConfigEditor` (read → modify with `dataclasses.replace` → write) |
| `python/tests/test_writer.py` | 1.6 | Writer test suite: 8 roundtrip tests (machine name, hash, build plate, train count, offsets, thermal lensing, null fields, schema validity) |
| `python/tests/test_builder.py` | 1.6 | Builder test suite: 7 tests (single/dual laser, plate dims, correction grid shape/non-zero, YAML roundtrip, ConfigEditor offset mutation) |
| `tools/generate_fixtures.py` | 1.7 | Generates `reference_output.json`, `reference_output.sha256`, and `synthetic_2laser.h5` from the reference HDF5 and `MockConfigBuilder` |
| `fixtures/reference_output.json` | 1.7 | Canonical JSON golden file — cross-language drift detector; human-reviewed before commit |
| `fixtures/reference_output.sha256` | 1.7 | SHA-256 checksum of `reference_output.json`; CI enforces both files are always committed together |
| `fixtures/synthetic_2laser.h5` | 1.7 | Small synthetic 2-laser fixture (~200 KB) generated by `MockConfigBuilder`; used in Node.js/Rust/C++ test suites |
| `.github/workflows/python.yml` | 1.7 | CI: pytest on Python 3.11+3.12, golden file checksum verification, CLI validate/export-json, golden file diff check, artifact upload |

---

### Pending Test Coverage

All planned tests are now active. No deferred tests remain.

| Test | Status |
|---|---|
| `test_golden_output_satisfies_schema` | **Active** — `fixtures/reference_output.json` generated in Phase 1.7 |

---

## Phase 0 — Foundation (Do First, Everything Depends On It)

**Goal**: Establish the repo scaffold, canonical schema, and golden fixtures before any language implementation begins. This phase is the contract that all four libraries must satisfy.

---

### 0.1 — Repository Structure

```
Machine_Config_Library/
│
├── Reference Materials/          ← existing, do not modify
│
├── schema/
│   ├── machine_config_v1.schema.json   ← canonical JSON Schema (see 0.2)
│   └── adapters/                       ← migration specs (one per version bump, see 0.6)
│
├── fixtures/
│   ├── reference_config.h5             ← symlink or copy of the sample .h5
│   ├── reference_output.json           ← golden file (generated in Phase 1, human-reviewed)
│   ├── reference_output.sha256         ← checksum of golden file; enforced in CI
│   ├── synthetic_2laser.h5             ← generated by MockConfigBuilder (Phase 1)
│   └── adapters/                       ← adapter test fixtures (see 0.6)
│
├── python/
├── nodejs/
├── rust/
├── cpp/
│
├── tools/
│   ├── cross_check.py                  ← diffs all 4 language outputs (Phase 5)
│   ├── generate_fixtures.py            ← regenerates golden files (Phase 1)
│   ├── generate_adapters.py            ← Jinja adapter boilerplate generator (see 0.7)
│   └── templates/                      ← Jinja2 templates for per-language adapter stubs
│       ├── adapter_python.py.j2
│       ├── adapter_typescript.ts.j2
│       ├── adapter_rust.rs.j2
│       └── adapter_cpp.hpp.j2
│
├── examples/
│   ├── quickstart/
│   │   ├── python/
│   │   ├── nodejs/
│   │   ├── rust/
│   │   └── cpp/
│   └── full_workflow/
│       ├── python/
│       ├── nodejs/
│       ├── rust/
│       └── cpp/
│
├── viewer/
│   └── machine_config_viewer.html      ← enhanced version (Phase 7)
│
└── .github/
    └── workflows/
        ├── python.yml
        ├── nodejs.yml
        ├── rust.yml
        ├── cpp.yml
        └── cross_check.yml
```

> **Status: Complete (2026-07-23).** All directories created; `.gitkeep` files placed in empty directories; `fixtures/reference_config.h5` copied from Reference Materials. Note: `Reference Materials/` also contains `machine_config_TM_LPBF_02__AconityMIDI__OG_wOPCUA.h5` (a variant file with the OPCUA group populated) — available for use in Phase 1 tests of `get_raw_group()`.

---

### 0.2 — Canonical JSON Schema (`schema/machine_config_v1.schema.json`)

This is the contract. Every language reader must produce output that validates against this. Key top-level structure:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "MachineConfig",
  "type": "object",
  "required": ["meta", "machine", "optical_trains"],
  "properties": {
    "meta": {
      "type": "object",
      "required": ["machine_name", "manufacturer", "model", "serial_number",
                   "file_version", "export_date", "configuration_hash"],
      "properties": {
        "machine_name":        { "type": "string" },
        "manufacturer":        { "type": "string" },
        "model":               { "type": "string" },
        "serial_number":       { "type": "string" },
        "file_version":        { "type": "string" },
        "export_date":         { "type": "string", "format": "date-time" },
        "configuration_hash":  { "type": "string", "minLength": 64, "maxLength": 64 }
      }
    },
    "machine": {
      "type": "object",
      "properties": {
        "id":                      { "type": ["string", "null"] },
        "build_plate_x":           { "type": ["number", "null"] },
        "build_plate_x_unit":      { "type": ["string", "null"] },
        "build_plate_y":           { "type": ["number", "null"] },
        "build_plate_y_unit":      { "type": ["string", "null"] },
        "build_plate_z":           { "type": ["number", "null"] },
        "build_plate_z_unit":      { "type": ["string", "null"] },
        "build_plate_radius":      { "type": ["number", "null"] },
        "build_plate_radius_unit": { "type": ["string", "null"] },
        "gas_flow_direction":      { "type": ["string", "null"] },
        "recoat_direction":        { "type": ["string", "null"] }
      }
    },
    "optical_trains": {
      "type": "array",
      "minItems": 1,
      "maxItems": 6,
      "items": {
        "type": "object",
        "required": ["train_id", "scanner", "light_source", "collimator", "scanner_card"],
        "properties": {
          "train_id":                                   { "type": "string" },
          "beam_waist_major":                           { "type": ["number", "null"] },
          "beam_waist_major_unit":                      { "type": ["string", "null"] },
          "beam_waist_minor":                           { "type": ["number", "null"] },
          "beam_waist_minor_unit":                      { "type": ["string", "null"] },
          "beam_waist_offset_z":                        { "type": ["number", "null"] },
          "beam_waist_offset_z_unit":                   { "type": ["string", "null"] },
          "m2_major":                                   { "type": ["number", "null"] },
          "m2_minor":                                   { "type": ["number", "null"] },
          "rayleigh_length_major":                      { "type": ["number", "null"] },
          "rayleigh_length_major_unit":                 { "type": ["string", "null"] },
          "rayleigh_length_minor":                      { "type": ["number", "null"] },
          "rayleigh_length_minor_unit":                 { "type": ["string", "null"] },
          "thermal_lensing_passed":                     { "type": ["boolean", "null"] },
          "thermal_lensing_focal_plane_shift":          { "type": ["number", "null"] },
          "thermal_lensing_focal_plane_shift_unit":     { "type": ["string", "null"] },
          "thermal_lensing_threshold":                  { "type": ["number", "null"] },
          "thermal_lensing_threshold_unit":             { "type": ["string", "null"] },
          "scanner": {
            "type": "object",
            "properties": {
              "manufacturer":           { "type": "string" },
              "model":                  { "type": "string" },
              "serial_number":          { "type": "string" },
              "working_distance":        { "type": ["number", "null"] },
              "working_distance_unit":   { "type": ["string", "null"] },
              "scan_field_x":            { "type": ["number", "null"] },
              "scan_field_x_unit":       { "type": ["string", "null"] },
              "scan_field_y":            { "type": ["number", "null"] },
              "scan_field_y_unit":       { "type": ["string", "null"] },
              "scan_field_z":            { "type": ["number", "null"] },
              "scan_field_z_unit":       { "type": ["string", "null"] },
              "scan_head_offset_x":      { "type": ["number", "null"] },
              "scan_head_offset_x_unit": { "type": ["string", "null"] },
              "scan_head_offset_y":      { "type": ["number", "null"] },
              "scan_head_offset_y_unit": { "type": ["string", "null"] },
              "scan_head_offset_z":      { "type": ["number", "null"] },
              "scan_head_offset_z_unit": { "type": ["string", "null"] },
              "scan_head_rotation":      { "type": ["number", "null"] },
              "scan_head_rotation_unit": { "type": ["string", "null"] },
              "axis_configuration":      { "type": ["string", "null"] }
            }
          },
          "light_source": {
            "type": "object",
            "properties": {
              "manufacturer":             { "type": "string" },
              "model":                    { "type": "string" },
              "serial_number":            { "type": "string" },
              "wavelength":               { "type": ["number", "null"] },
              "wavelength_unit":          { "type": ["string", "null"] },
              "power_max_nominal":        { "type": ["number", "null"] },
              "power_max_nominal_unit":   { "type": ["string", "null"] },
              "power_max_actual":         { "type": ["number", "null"] },
              "power_max_actual_unit":    { "type": ["string", "null"] },
              "power_min_actual":         { "type": ["number", "null"] },
              "power_min_actual_unit":    { "type": ["string", "null"] },
              "watts_to_volts_algorithm": { "type": ["string", "null"] },
              "watts_to_volts_params":    { "type": ["string", "null"] }
            }
          },
          "collimator": {
            "type": "object",
            "properties": {
              "manufacturer":    { "type": "string" },
              "model":           { "type": "string" },
              "serial_number":   { "type": "string" },
              "focal_length":      { "type": ["number", "null"] },
              "focal_length_unit": { "type": ["string", "null"] }
            }
          },
          "scanner_card": {
            "type": "object",
            "properties": {
              "manufacturer":           { "type": "string" },
              "model":                  { "type": "string" },
              "serial_number":          { "type": "string" },
              "communication_protocol": { "type": ["string", "null"] },
              "sample_period":          { "type": ["number", "null"] },
              "sample_period_unit":     { "type": ["string", "null"] }
            }
          },
          "clearbox": {
            "type": ["object", "null"],
            "properties": {
              "ip_address":                    { "type": "string" },
              "serial_number":                 { "type": ["string", "null"] },
              "data_port":                     { "type": ["integer", "null"] },
              "server_port":                   { "type": ["integer", "null"] },
              "actual_timing_offset":          { "type": ["integer", "null"] },
              "commanded_timing_offset":       { "type": ["integer", "null"] },
              "correction_data_shape":         { "type": "array", "items": {"type": "integer"} },
              "inverse_correction_data_shape": { "type": "array", "items": {"type": "integer"} }
            }
          },
          "scan_field_correction_file": {
            "type": ["object", "null"],
            "properties": {
              "document_name":    { "type": "string" },
              "document_id":      { "type": "string" },
              "file_size":        { "type": "integer" },
              "valid_as_of_date": { "type": "string" }
            }
          }
        }
      }
    }
  }
}
```

> **What the schema captures and what it intentionally omits**:
>
> - **`correction_data` and `inverse_correction_data` float64 arrays** — accessible via `reader.get_correction_data(train_index)` and `reader.get_inverse_correction_data(train_index)`. Never serialized to JSON. Their *shapes* are captured in the `clearbox` object so callers can reason about size without loading the full array.
> - **`scan_field_correction_file`** — stored in the HDF5 as a `uint8` byte-array *dataset*, not a group. Its HDF5 dataset attributes populate the `scan_field_correction_file` JSON object. The raw bytes (the embedded `.fc3` file) are accessible via `reader.get_scan_field_correction_bytes(train_index)` and are never part of canonical JSON.
> - **`OPCUA` group** — machine connectivity configuration, not optical config. Fully outside the canonical schema; accessible without commitment via `reader.get_raw_group("OPCUA")`.
> - **`Scanner/X_Axis`, `Y_Axis`, `Z_Axis` sub-groups** — scanner axis tuning parameters with sparse population (many fields are empty strings in real files). Outside the canonical schema; accessible via `reader.get_raw_group(path)`.
> - **Empty-string fields** — many HDF5 attributes that are typed as numbers in the schema are stored as `""` when unpopulated. The reader converts `""` → `null` for all non-required fields. This conversion is performed by a centralised `_read_float_attr` / `_read_str_attr` helper — never inline — so the policy is explicit and auditable.

> **Status: Complete (2026-07-23); updated for hybrid unit design (2026-07-23).** `schema/machine_config_v1.schema.json` created and updated. All numeric fields that have a companion `_unit` attribute in the HDF5 file now use bare field names (e.g. `working_distance`) with a companion `_unit` string field (e.g. `working_distance_unit`) instead of unit-suffixed names (e.g. `working_distance_mm`). Schema self-tests (0.4) deferred to Phase 1 when the Python environment is set up — no language implementation code is required to run them, only `jsonschema`.

---

### 0.3 — Fixture Strategy

| File | Source | Purpose |
|---|---|---|
| `fixtures/reference_config.h5` | Copy of AconityMIDI sample from Reference Materials | Full real-world test input |
| `fixtures/reference_output.json` | Generated in Phase 1.4 by Python reader | Ground truth all other languages check against |
| `fixtures/synthetic_2laser.h5` | Generated by `MockConfigBuilder` in Phase 1.5 | Lightweight fixture used in all language unit test suites |
| `fixtures/reference_config_opcua.h5` | Copy of OPCUA-variant AconityMIDI sample from Reference Materials | Used in Phase 1 to test `get_raw_group("OPCUA")` returns populated data, and to assert `get_raw_group("OPCUA")` returns `{}` on `reference_config.h5` |

> **Note on `Configuration/build_configuration`**: This group appears in certain HDF5 files as an artifact of the HeadlessMock configuration generator. It is not part of the machine configuration schema, is not produced by real machine software exports, and must not be present in the reference fixture or the synthetic fixture. The reader silently ignores any `Configuration` group it encounters at the root level.

---

### 0.4 — Schema Self-Tests

The schema is the contract all four implementations must satisfy. These tests verify the schema file itself is well-formed and that its constraints are correctly specified. They live in `python/tests/test_schema.py` and are part of the Phase 1 test suite — no language implementation code is required to run them.

| Test | What is verified |
|---|---|
| Schema file parses as valid JSON | `json.loads()` on the schema file succeeds |
| Schema is a valid JSON Schema draft 2020-12 | `jsonschema.Draft202012Validator.check_schema(schema)` passes |
| Golden output satisfies schema | `reference_output.json` validates without errors (run after Phase 1.7) |
| Missing `meta` is rejected | Object without `meta` key fails `required` constraint |
| Missing `machine` is rejected | Object without `machine` key fails `required` constraint |
| Missing `optical_trains` is rejected | Object without `optical_trains` key fails `required` constraint |
| Empty `optical_trains` array is rejected | `[]` violates `minItems: 1` |
| Seven optical trains are rejected | Array of 7 items violates `maxItems: 6` |
| `configuration_hash` shorter than 64 chars is rejected | `minLength: 64` constraint enforced |
| `configuration_hash` longer than 64 chars is rejected | `maxLength: 64` constraint enforced |
| Optical train item missing `train_id` is rejected | `required` constraint on array items enforced |
| Optical train item missing `scanner` is rejected | `required` constraint on array items enforced |
| Optical train item missing `light_source` is rejected | `required` constraint on array items enforced |
| Optical train item missing `collimator` is rejected | `required` constraint on array items enforced |
| Optical train item missing `scanner_card` is rejected | `required` constraint on array items enforced |

> **Status: Complete (2026-07-23).** `python/tests/test_schema.py` created. Run inside `.venv` (jsonschema 4.26.0, pytest 9.1.1): **15 passed, 1 skipped**. The skipped test (`test_golden_output_satisfies_schema`) is `skipif`-gated on `fixtures/reference_output.json` existing — it will activate automatically in Phase 1.7 when the golden file is generated. A sanity test (`test_minimal_valid_config_passes`) was added beyond the 15 planned rows to guard against the base fixture being silently invalid.

---

### 0.5 — Flexibility and Extensibility Rules

These rules apply to all four language implementations equally. They are as binding as the schema itself. A reader that violates any of these rules is incorrect even if its output passes schema validation.

**Rule 1 — Presence checks, not blanket exception handling.**
Before reading any group, check that it exists in the HDF5 file. Required groups (`Collimator`, `Scanner_Card`) must be present — their absence raises an error. Optional groups (`ClearBox`, `scan_field_correction_file`) return `null`/`None`/`Option::None` on absence. Never use a catch-all try/except to hide a missing required field.

**Rule 2 — No injected defaults.**
If a value is absent from the HDF5, the model field is `null`. The reader never substitutes a value (e.g. `670.0` for a missing `Working_Distance`). Default-filling is the caller's responsibility, not the library's.

**Rule 3 — Explicit empty-string handling at the boundary.**
Many HDF5 float-typed attributes are stored as `""` when unpopulated. The conversion `"" → null` is performed by a single named helper per language — never inline. The rule is:

| HDF5 attribute state | Model field value |
|---|---|
| Key absent from HDF5 | `null` |
| Key present, value is `""` | `null` |
| Key present, valid parseable number | parsed numeric value |
| Key present, non-empty and non-parseable | `ValueError` / `ParseError` — not silently `null` |

**Rule 4 — Integer → bool coercion is named and documented.**
`Thermal_Lensing_Test_Passed` is stored as HDF5 Integer `0` / `1`. Conversion uses a named helper (`_read_bool_from_int_attr`) that maps `0 → false`, `1 → true`, any other integer → error. Silent `bool(n)` casts are forbidden.

**Rule 5 — `parse()` returns only what the schema defines.**
Unknown or peripheral HDF5 groups are silently skipped by `parse()`. They are accessible without any schema commitment via:
```python
reader.get_raw_group("OPCUA")           # → dict of all attributes in that group
reader.get_raw_group("Machine/Optical_Trains/Optical_Train_01/Scanner/X_Axis")
```
`get_raw_group()` returns an empty dict (not an error) if the path does not exist.

**Rule 6 — File version is checked before parsing.**
The reader reads `File_Version` from the root attributes first. An unrecognised version emits a `UserWarning` but proceeds. An incompatible future major version raises a clear error rather than silently producing wrong output.

**Rule 7 — Schema `required` reflects the minimum viable file.**
`train_id`, `scanner`, `light_source`, `collimator`, and `scanner_card` are all required per optical train — a file missing any of these fails loudly. `clearbox` and `scan_field_correction_file` are optional: a machine config without a ClearBox is valid and common (ClearBox is an add-on component). A file missing a scanner or collimator group is not valid.

**Rule 8 — Numeric fields with unit companions are stored as value+unit pairs; units are locked at parse time.**
Every HDF5 attribute that has a companion `_unit` attribute produces two model fields: the numeric value under the bare field name (e.g. `working_distance`) and the unit string under `<field>_unit` (e.g. `working_distance_unit`). The reader asserts the unit matches the expected locked value for the current schema version — a mismatch raises `ValueError`, not a warning. If the unit attribute is absent or empty, both the value and unit fields are `null`. Benefits: (1) canonical JSON is self-documenting — the unit is always visible alongside the value; (2) the schema does not need a version bump if a unit string ever changes (the `_unit` field type is already `["string", "null"]`); (3) only the reader's locked unit assertion needs updating if a unit changes.

---

### 0.6 — Version Adapter Architecture

The `File_Version` root attribute in each HDF5 file identifies which schema version was used to write it. When the schema evolves, readers must parse older files without breaking. The adapter layer handles this by transforming a parsed `MachineConfig` from one version's shape to the next — without touching the HDF5 file and without injecting defaults.

**Adapter spec format** (`schema/adapters/<from>_to_<to>.yaml`):

Each spec declares what changed between two adjacent schema versions. It is the single source of truth for that migration — all four language implementations write their adapter by reading this spec.

```yaml
# schema/adapters/v1_0_to_v1_1.yaml
from_version: "1.0"
to_version:   "1.1"
breaking:     false    # false = v1.0 readers can ignore new optional fields in v1.1 files
description:  "Adds major_axis_angle (+ unit) per optical train; renames axis_configuration"

changes:
  # Fields added in v1.1 — v1.0 files do not have them.
  # Per Rule 2: null only. No default value is injected.
  # Rule 8: new numeric+unit pair → two field_add entries.
  - type: field_add
    path: optical_trains[*]
    field: major_axis_angle
    nullable: true
  - type: field_add
    path: optical_trains[*]
    field: major_axis_angle_unit
    nullable: true

  # Field renamed in v1.1.
  - type: field_rename
    path: optical_trains[*].scanner
    old_field: axis_configuration
    new_field: axis_config_label

  # Field removed in v1.1 after deprecation in v1.0.
  - type: field_remove
    path: optical_trains[*]
    field: deprecated_field
```

> **Rule 2 applies to adapters without exception.** No adapter spec may include a `default` value for an added field. `field_add` entries always produce `null` in the output for files that predate the field. Default-filling is the caller’s responsibility.

**Adapter chaining**: The reader resolves a chain from the file’s version to the library’s current version and applies adapters in order:

```
HDF5 (v1.0) → parse(v1.0 fields) → adapt(v1.0→1.1) → adapt(v1.1→1.2) → MachineConfig
```

**Adapter test fixtures**: Every adapter spec is accompanied by a JSON input/output pair so each language can test the adapter in isolation, without any HDF5 file:

```
fixtures/adapters/v1_0_to_v1_1/
    input.json     ← canonical JSON from a v1.0 parse
    expected.json  ← expected JSON after adapter runs
```

The adapter test applies the adapter to `input.json` and diffs the result against `expected.json`. This is a pure in-memory test — no HDF5 dependency.

**Adapter registration** (Python example):

```python
# python/src/machine_config/adapters/__init__.py
from .v1_0_to_v1_1 import V1_0_to_V1_1

# Keys are (from_version, to_version); chain is resolved in order.
ADAPTERS: dict[tuple[str, str], type] = {
    ("1.0", "1.1"): V1_0_to_V1_1,
}

def get_chain(file_version: str, current_version: str) -> list:
    """Returns ordered adapter instances needed to reach current_version."""
    # e.g. file_version="1.0", current="1.2" yields [V1_0_to_V1_1, V1_1_to_V1_2]
    ...
```

**Current state**: There is one schema version (`1.0`). `schema/adapters/` and `fixtures/adapters/` exist but are empty. The infrastructure is in place; the first spec file is added only when the schema actually changes. When one is added, run `python tools/generate_adapters.py` — see Section 0.7.

---

### 0.7 — Jinja Adapter Code Generator

The adapter spec YAML is machine-readable. When a new spec is added under `schema/adapters/`, a Jinja2 generator produces the adapter class boilerplate in all four languages automatically. This closes the most likely source of adapter drift: forgetting to implement a field rename or `null` insertion in one language while updating the other three.

**What the generator produces** (from a single spec file):

| Change type | Generated output in each language |
|---|---|
| `field_add` | Sets field to `null` / `None` / `std::nullopt` — no default ever injected |
| `field_rename` | Moves value from old name to new name, clears old field |
| `field_remove` | Deletes or omits the field; a comment marks the removal |

Adapters that require logic beyond those three types (e.g. a unit conversion) are generated first, then hand-edited. The generator header marks them as generated so CI can enforce they were not written from scratch.

**Template directory** (`tools/templates/`):

```
tools/templates/
├── adapter_python.py.j2        → python/src/machine_config/adapters/{name}.py
├── adapter_typescript.ts.j2    → nodejs/src/adapters/{name}.ts
├── adapter_rust.rs.j2          → rust/src/adapters/{name}.rs
└── adapter_cpp.hpp.j2          → cpp/include/machine_config/adapters/{name}.hpp
```

**Example — Python template** (`tools/templates/adapter_python.py.j2`):

```jinja
# AUTO-GENERATED from schema/adapters/{{ name }}.yaml
# DO NOT EDIT MANUALLY — regenerate with: python tools/generate_adapters.py
from .base import BaseAdapter
from machine_config.models import MachineConfig


class V{{ spec.from_version | replace('.', '_') }}_to_V{{ spec.to_version | replace('.', '_') }}(BaseAdapter):
    from_version = "{{ spec.from_version }}"
    to_version   = "{{ spec.to_version }}"

    def adapt(self, config: MachineConfig) -> MachineConfig:
{%- for change in spec.changes %}
{%- if change.type == 'field_add' %}
        # field_add: {{ change.field }} (path: {{ change.path }})
        for train in config.optical_trains:
            if not hasattr(train, '{{ change.field }}'):
                object.__setattr__(train, '{{ change.field }}', None)
{%- elif change.type == 'field_rename' %}
        # field_rename: {{ change.old_field }} → {{ change.new_field }} (path: {{ change.path }})
        for train in config.optical_trains:
            val = getattr(train.scanner, '{{ change.old_field }}', None)
            object.__setattr__(train.scanner, '{{ change.new_field }}', val)
{%- elif change.type == 'field_remove' %}
        # field_remove: {{ change.field }} dropped (path: {{ change.path }})
{%- endif %}
{%- endfor %}
        return config
```

**Generator script** (`tools/generate_adapters.py`):

```python
#!/usr/bin/env python3
"""
For every spec in schema/adapters/, render the four language adapter files
using Jinja2 templates and update each language's adapter registry.
Idempotent — re-running overwrites existing generated files.
"""
from pathlib import Path
import yaml
from jinja2 import Environment, FileSystemLoader

SPEC_DIR     = Path("schema/adapters")
TEMPLATE_DIR = Path("tools/templates")
OUTPUTS = {
    "python":     ("adapter_python.py.j2",
                   "python/src/machine_config/adapters/{name}.py"),
    "typescript": ("adapter_typescript.ts.j2",
                   "nodejs/src/adapters/{name}.ts"),
    "rust":       ("adapter_rust.rs.j2",
                   "rust/src/adapters/{name}.rs"),
    "cpp":        ("adapter_cpp.hpp.j2",
                   "cpp/include/machine_config/adapters/{name}.hpp"),
}

env = Environment(
    loader=FileSystemLoader(str(TEMPLATE_DIR)),
    keep_trailing_newline=True,
)

for spec_path in sorted(SPEC_DIR.glob("*.yaml")):
    spec = yaml.safe_load(spec_path.read_text())
    name = spec_path.stem          # e.g. "v1_0_to_v1_1"
    for lang, (tmpl_name, out_pattern) in OUTPUTS.items():
        rendered = env.get_template(tmpl_name).render(spec=spec, name=name)
        out = Path(out_pattern.format(name=name))
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(rendered)
        print(f"[{lang}] wrote {out}")

print("Done.  Review generated files before committing.")
```

**Workflow for adding a new schema version**:

1. Write `schema/adapters/v1_1_to_v1_2.yaml` (the spec YAML)
2. Run `python tools/generate_adapters.py`
3. Review the four generated files — mechanical changes (`field_add`, `field_rename`, `field_remove`) are complete
4. Hand-edit any adapter requiring logic beyond those three types (e.g. a unit conversion); the `DO NOT EDIT MANUALLY` header is removed only in that case
5. Write `fixtures/adapters/v1_1_to_v1_2/input.json` and `expected.json` by hand
6. Run the adapter test suite; all four languages must pass before committing

**CI enforcement — generated header check**:

```yaml
- name: Verify adapter files carry the generated-file header
  run: |
    find python/src/machine_config/adapters nodejs/src/adapters \
         rust/src/adapters cpp/include/machine_config/adapters \
         \( -name 'v*.py' -o -name 'v*.ts' -o -name 'v*.rs' -o -name 'v*.hpp' \) \
      | xargs grep -rL 'DO NOT EDIT MANUALLY' | tee /tmp/missing_header.txt
    if [ -s /tmp/missing_header.txt ]; then
      echo "FAIL: above adapter files are missing the generated-file header."
      echo "Regenerate with: python tools/generate_adapters.py"
      exit 1
    fi
```

**Scope boundary — what Jinja generates and what stays hand-written**:

| Component | Managed by |
|---|---|
| `models.py` / `models.ts` / `models.rs` / `models.hpp` | Hand-written; cross-check catches drift |
| Reader logic (HDF5 path → field mapping) | Hand-written — domain knowledge, not derivable from schema |
| Writer logic (MachineConfig → HDF5 attribute mapping) | Hand-written — symmetric counterpart to reader |
| Builder and CLI | Hand-written |
| Adapter test fixtures (`input.json` / `expected.json`) | Hand-written |
| Adapter class boilerplate | **Generated by Jinja** |
| Adapter registry (`ADAPTERS` dict / `mod.rs` exports) | **Generated by Jinja** (registry template) |

> **Why not generate models too**: The schema defines the output shape. It does not define how to read it from HDF5 — that mapping (`Working_Distance` attribute → `working_distance` field + `working_distance_unit` field, empty-string → `null`, integer 0/1 → `bool`) is domain knowledge that cannot be inferred from the JSON Schema alone. The adapter layer is purely JSON-to-JSON and entirely derivable from the spec YAML, which is why generation is correct and safe there.

> **Status: Complete (2026-07-23).** `tools/generate_adapters.py` and all four `.j2` templates created. Generator confirmed idempotent on empty spec dir. `python/tests/test_generator.py` created: **29 tests pass** across all four templates (template validity, header, version strings, all three change types, empty-changes case). Full suite: **44 passed, 1 skipped**.

---

## Phase 1 — Python (Reference Implementation)

**Libraries**: `h5py`, `pydantic`, `pytest`, `jsonschema`, `click`, `numpy`

**Install**: `pip install machine-config-library`

**Priority**: Highest — Python generates the golden file used by all other languages.

---

### 1.1 — Package Structure

```
python/
├── src/machine_config/
│   ├── __init__.py          ← exports MachineConfigReader, MockConfigBuilder
│   ├── models.py            ← dataclasses: MachineConfig, OpticalTrain, etc.
│   ├── reader.py            ← h5py-based reader → produces MachineConfig
│   ├── writer.py            ← h5py-based writer ← consumes MachineConfig (inverse of reader)
│   ├── builder.py           ← MockConfigBuilder, YamlConfigBuilder (synthetic/test HDF5)
│   ├── schema.py            ← loads and exposes machine_config_v1.schema.json
│   ├── cli.py               ← click CLI: inspect, validate, build, write, export-json
│   └── adapters/
│       ├── __init__.py      ← exports get_chain(); ADAPTERS registry
│       └── base.py          ← BaseAdapter protocol (adapt(MachineConfig) → MachineConfig)
├── tests/
│   ├── conftest.py          ← pytest fixtures: paths to reference files
│   ├── test_reader.py       ← unit tests against reference_config.h5
│   ├── test_writer.py       ← roundtrip tests: read → write → read → compare
│   ├── test_builder.py      ← unit tests for MockConfigBuilder
│   ├── test_schema.py       ← JSON Schema validation tests
│   └── test_adapters.py     ← adapter fixture tests (added when first adapter is written)
└── pyproject.toml
```

> **Status: Complete (2026-07-23).** `python/src/machine_config/` package scaffold created (9 stub source files: `__init__.py`, `models.py`, `reader.py`, `writer.py`, `builder.py`, `schema.py`, `cli.py`, `adapters/__init__.py`, `adapters/base.py`). Test stubs added: `conftest.py`, `test_reader.py`, `test_writer.py`, `test_builder.py`, `test_adapters.py`. `python/pyproject.toml` created with minimal build config and `[tool.pytest.ini_options]` (pythonpath + testpaths). All 44 existing tests pass; 1 skipped.

---

### 1.2 — `models.py` — Data Models

Key design: every field is typed. Numeric fields that have a companion `_unit` attribute in the HDF5 file use a **hybrid** convention: the bare field name (e.g. `working_distance`) for the value plus a `<field>_unit` companion (e.g. `working_distance_unit`) for the unit string. Dimensionless and boolean fields keep bare names without a unit companion. Every field that can be absent from the HDF5 or stored as an empty string is typed `Optional` — never defaulted.

```python
# python/src/machine_config/models.py
from __future__ import annotations
from dataclasses import dataclass
from typing import Optional


@dataclass
class ScanFieldCorrectionFile:
    document_name: str
    document_id: str
    file_size: int
    valid_as_of_date: str


@dataclass
class ClearBox:
    ip_address: str
    serial_number: Optional[str]
    data_port: Optional[int]
    server_port: Optional[int]
    actual_timing_offset: Optional[int]
    commanded_timing_offset: Optional[int]
    correction_data_shape: tuple[int, int, int]          # e.g. (257, 257, 2)
    inverse_correction_data_shape: tuple[int, int, int]  # same shape


@dataclass
class Collimator:
    manufacturer: str
    model: str
    serial_number: str
    focal_length: Optional[float]
    focal_length_unit: Optional[str]


@dataclass
class ScannerCard:
    manufacturer: str
    model: str
    serial_number: str
    communication_protocol: Optional[str]
    sample_period: Optional[float]
    sample_period_unit: Optional[str]


@dataclass
class Scanner:
    manufacturer: str
    model: str
    serial_number: str
    working_distance: Optional[float]
    working_distance_unit: Optional[str]
    scan_field_x: Optional[float]
    scan_field_x_unit: Optional[str]
    scan_field_y: Optional[float]
    scan_field_y_unit: Optional[str]
    scan_field_z: Optional[float]
    scan_field_z_unit: Optional[str]
    scan_head_offset_x: Optional[float]
    scan_head_offset_x_unit: Optional[str]
    scan_head_offset_y: Optional[float]
    scan_head_offset_y_unit: Optional[str]
    scan_head_offset_z: Optional[float]
    scan_head_offset_z_unit: Optional[str]
    scan_head_rotation: Optional[float]
    scan_head_rotation_unit: Optional[str]
    axis_configuration: Optional[str]


@dataclass
class LightSource:
    manufacturer: str
    model: str
    serial_number: str
    wavelength: Optional[float]
    wavelength_unit: Optional[str]
    power_max_nominal: Optional[float]
    power_max_nominal_unit: Optional[str]
    power_max_actual: Optional[float]
    power_max_actual_unit: Optional[str]
    power_min_actual: Optional[float]
    power_min_actual_unit: Optional[str]
    watts_to_volts_algorithm: Optional[str]
    watts_to_volts_params: Optional[str]


@dataclass
class OpticalTrain:
    train_id: str
    beam_waist_major: Optional[float]
    beam_waist_major_unit: Optional[str]
    beam_waist_minor: Optional[float]
    beam_waist_minor_unit: Optional[str]
    beam_waist_offset_z: Optional[float]
    beam_waist_offset_z_unit: Optional[str]
    m2_major: Optional[float]
    m2_minor: Optional[float]
    rayleigh_length_major: Optional[float]
    rayleigh_length_major_unit: Optional[str]
    rayleigh_length_minor: Optional[float]
    rayleigh_length_minor_unit: Optional[str]
    thermal_lensing_passed: Optional[bool]                        # HDF5 int 0/1; None if absent
    thermal_lensing_focal_plane_shift: Optional[float]
    thermal_lensing_focal_plane_shift_unit: Optional[str]
    thermal_lensing_threshold: Optional[float]
    thermal_lensing_threshold_unit: Optional[str]
    scanner: Scanner
    light_source: LightSource
    collimator: Collimator                                        # required; see Rule 7
    scanner_card: ScannerCard                                     # required; see Rule 7
    clearbox: Optional[ClearBox]
    scan_field_correction_file: Optional[ScanFieldCorrectionFile]


@dataclass
class BuildPlate:
    x: Optional[float]
    x_unit: Optional[str]
    y: Optional[float]
    y_unit: Optional[str]
    z: Optional[float]
    z_unit: Optional[str]
    corner_radius: Optional[float]
    corner_radius_unit: Optional[str]


@dataclass
class Machine:
    id: Optional[str]
    machine_name: str
    manufacturer: str
    model: str
    serial_number: str
    build_plate: BuildPlate
    gas_flow_direction: Optional[str]
    recoat_direction: Optional[str]


@dataclass
class MachineConfigMeta:
    machine_name: str
    manufacturer: str
    model: str
    serial_number: str
    file_version: str
    export_date: str
    configuration_hash: str


@dataclass
class MachineConfig:
    meta: MachineConfigMeta
    machine: Machine
    optical_trains: list[OpticalTrain]
```

> **Status: Complete (2026-07-23); extended after HDF5 structure audit (2026-07-23).** `python/src/machine_config/models.py` implemented with all 11 dataclasses. `python/src/machine_config/schema.py` implemented. Package installed in editable mode (`pip install -e python/`). After comparing both reference `.h5` structure files against the initial schema/models, the following fields were added:
> - **`OpticalTrain`**: `id`, `beam_profile_type`, `beam_waist_definition`, `build_plane_offset_major`/`_unit`, `build_plane_offset_minor`/`_unit`, `collimator_focal_length`/`_unit` (train-level duplicate), `major_axis_angle`/`_unit`, `scanner_number`
> - **`LightSource`**: `power_min_nominal`/`_unit`, `power_bit_resolution`/`_unit`
> - **`ClearBox`**: `manufacturer`, `model`, `output_path`, `selected_camera`, `custom_video_format`, `video_output`, `show_console`, `software_trigger_delay`, `volts_to_watts_algorithm`, `volts_to_watts_params`, `correction_grid_domain_shape`, `inverse_grid_domain_shape`
> - **`ScanFieldCorrectionFile`**: `document_created_at`, `document_type`, `original_uri`
> - **`machine` schema object**: `machine_name`, `manufacturer`, `model`, `serial_number` (duplicated from Machine group attrs; already in Python model)
>
> All 44 tests pass; 1 skipped.

---

### 1.3 — `reader.py` — Core Reader

```python
# python/src/machine_config/reader.py
import h5py, json, numpy as np
from pathlib import Path
from .models import *


class MachineConfigReader:
    def __init__(self, path: str | Path):
        self.path = Path(path)

    def parse(self) -> MachineConfig:
        with h5py.File(self.path, 'r') as f:
            self._check_file_version(f)
            return self._parse(f)

    def get_correction_data(self, train_index: int) -> np.ndarray:
        """Returns the (257,257,2) float64 correction grid for optical train N (0-indexed)."""
        with h5py.File(self.path, 'r') as f:
            return f[self._clearbox_path(train_index) + "/Correction_Data"][:]

    def get_inverse_correction_data(self, train_index: int) -> np.ndarray:
        """Returns the (257,257,2) float64 inverse correction grid for optical train N (0-indexed)."""
        with h5py.File(self.path, 'r') as f:
            return f[self._clearbox_path(train_index) + "/Inverse_Correction_Data"][:]

    def get_scan_field_correction_bytes(self, train_index: int) -> bytes:
        """Returns the raw .fc3 file bytes embedded as a uint8 dataset for optical train N."""
        with h5py.File(self.path, 'r') as f:
            train_id = f"Optical_Train_{train_index+1:02d}"
            path = f"Machine/Optical_Trains/{train_id}/scan_field_correction_file"
            return bytes(f[path][:])

    def get_raw_group(self, hdf5_path: str) -> dict:
        """
        Returns all attributes of an arbitrary HDF5 group as a plain dict.
        Returns an empty dict (not an error) if the path does not exist.
        Use for non-schema groups: OPCUA, Scanner axis sub-groups, etc.
        """
        with h5py.File(self.path, 'r') as f:
            if hdf5_path not in f:
                return {}
            return dict(f[hdf5_path].attrs)

    def to_json(self, indent: int = 2) -> str:
        """Parse and serialize to canonical JSON string."""
        config = self.parse()
        return json.dumps(self._config_to_dict(config), indent=indent)

    # ------------------------------------------------------------------
    # Attribute-reading helpers — all HDF5 attribute access goes through
    # these.  The empty-string → None and int → bool policies live here,
    # nowhere else.
    # ------------------------------------------------------------------

    @staticmethod
    def _read_str(attrs, key: str) -> Optional[str]:
        """None if absent or empty string.  Never injects a default."""
        val = attrs.get(key)
        if val is None:
            return None
        s = str(val).strip()
        return s if s else None

    @staticmethod
    def _read_float(attrs, key: str) -> Optional[float]:
        """None if absent or empty string.  ValueError if present but non-numeric."""
        val = attrs.get(key)
        if val is None:
            return None
        if isinstance(val, str) and val.strip() == "":
            return None
        return float(val)  # raises ValueError if non-numeric and non-empty

    @staticmethod
    def _read_int(attrs, key: str) -> Optional[int]:
        """None if absent or empty string."""
        val = attrs.get(key)
        if val is None:
            return None
        if isinstance(val, str) and val.strip() == "":
            return None
        return int(val)

    @staticmethod
    def _read_bool_from_int(attrs, key: str) -> Optional[bool]:
        """Converts HDF5 integer 0/1 to bool.  ValueError for any other integer."""
        val = attrs.get(key)
        if val is None:
            return None
        i = int(val)
        if i == 0:
            return False
        if i == 1:
            return True
        raise ValueError(
            f"Attribute '{key}' has unexpected integer value {i!r}; expected 0 or 1"
        )

    @staticmethod
    def _read_str_locked(attrs, unit_key: str, expected_unit: str) -> Optional[str]:
        """Read a _unit attribute and assert it matches the locked expected value.
        Returns None if the attribute is absent or an empty string.
        Raises ValueError if the value is present but does not match expected_unit.
        Enforces Rule 8: unit attributes are locked at parse time.
        """
        val = attrs.get(unit_key)
        if val is None:
            return None
        s = str(val).strip()
        if not s:
            return None
        if s != expected_unit:
            raise ValueError(
                f"Unit attribute '{unit_key}' has value {s!r}; "
                f"expected {expected_unit!r} — update the reader if the HDF5 "
                f"exporter changed units"
            )
        return s

    # ------------------------------------------------------------------
    # Private parse helpers
    # ------------------------------------------------------------------

    def _check_file_version(self, f: h5py.File) -> None:
        version = f.attrs.get('File_Version', '')
        if version != '1.0':
            import warnings
            warnings.warn(
                f"File_Version is {version!r}; this reader targets '1.0'. "
                "Output may be incomplete or incorrect.",
                UserWarning, stacklevel=3,
            )

    def _clearbox_path(self, train_index: int) -> str:
        train_id = f"Optical_Train_{train_index+1:02d}"
        return f"Machine/Optical_Trains/{train_id}/Optional_Components/ClearBox"

    def _parse(self, f: h5py.File) -> MachineConfig:
        meta = MachineConfigMeta(
            machine_name       = str(f.attrs.get('machine_name', '')),
            manufacturer       = str(f.attrs.get('manufacturer', '')),
            model              = str(f.attrs.get('model', '')),
            serial_number      = str(f.attrs.get('serial_number', '')),
            file_version       = str(f.attrs.get('File_Version', '')),
            export_date        = str(f.attrs.get('Export_Date', '')),
            configuration_hash = str(f.attrs.get('Configuration_Hash', '')),
        )
        # ... full implementation continues per structure.txt

    def _config_to_dict(self, config: MachineConfig) -> dict:
        """Convert to canonical JSON-serializable dict matching the schema."""
        # ... maps dataclasses to dicts
```

> **Status: Complete (2026-07-23).** `python/src/machine_config/reader.py` implemented with the full `MachineConfigReader` class. All HDF5 → model attribute mappings from both structure files are covered. Rule 8 unit locking enforced via `_read_str_locked()`. `python/tests/conftest.py` populated with `reference_reader`, `reference_config`, and `opcua_reader` session-scoped fixtures. `python/tests/test_reader.py` implemented with 43 tests across 9 classes (meta, machine, scanner, light source, collimator, scanner card, ClearBox, scan-field correction file, thermal lensing, locked units, `get_raw_group`, JSON/schema validation). 4 tests deferred (skip-marked) pending Phase 1.5 `MockConfigBuilder` — activated in Phase 1.5/1.6, bringing the reader test count to 47. `h5py 3.16.0` and `numpy 2.5.1` installed into `.venv`. `__init__.py` updated to export all 11 model classes and `MachineConfigReader`. Full suite at end of Phase 1.3: **87 passed, 5 skipped**.

---

### 1.4 — `cli.py` — Command-Line Interface

Users interact with the library from the command line before writing any integration code:

| Command | Description |
|---|---|
| `machine-config inspect path/to/config.h5` | Prints pretty summary: machine name, plate size, laser count, working distances |
| `machine-config validate path/to/config.h5` | Validates against schema, prints PASS/FAIL with details |
| `machine-config export-json path/to/config.h5 --output output.json` | Exports canonical JSON |
| `machine-config write path/to/config.json --output config.h5` | Writes a canonical JSON file back to a valid HDF5 file |
| `machine-config build --from-yaml spec.yaml --output config.h5` | Builds `.h5` from YAML spec |
| `machine-config build --mock --lasers 2 --output test_config.h5` | Generates a synthetic config |
| `machine-config demo` | Generates synthetic config and prints summary — zero files needed |

> **Status: Complete (2026-07-23; extended 2026-07-24).** `python/src/machine_config/cli.py` implemented with `click 8.4.2`. Phase 1.4 delivered `inspect` (with `--verbose` YAML dump), `validate`, `export-json` fully implemented; `write`, `build`, `demo` stub-registered. Phase 1.5 completed all three stubs: `write` (JSON → `config_from_dict` → `MachineConfigWriter`), `build --mock` (`MockConfigBuilder`), `build --from-yaml` (`YamlConfigBuilder`), `demo` (generates 2-laser synthetic config and prints summary). `[project.scripts]` and `[project.dependencies]` in `python/pyproject.toml`. `python/tests/test_cli.py` expanded to 40 tests. Full suite after Phase 1.5: **146 passed, 1 skipped**.

---

### 1.5 — `builder.py` — Config Builder

Three builder classes covering all use cases:

**`MockConfigBuilder`** — generates plausible synthetic `.h5` for testing:
```python
builder = MockConfigBuilder(n_lasers=2, build_plate_x=250, build_plate_y=250)
builder.save("test_config.h5")
```
Generates real-looking values: scanner offsets are randomized within plausible ranges, correction grids are populated with a smooth Gaussian warp pattern rather than zeros — zeros would hide bugs in correction-application code.

**`YamlConfigBuilder`** — YAML spec → `.h5`:
```python
builder = YamlConfigBuilder("spec.yaml")
builder.save("config.h5")
```

Example YAML spec:
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
      power_max_nominal: 1000
      wavelength: 1070
  - scanner:
      working_distance: 670
      scan_head_offset_x: 86.0
      scan_head_offset_y: -21.7
      scan_field_x: 600
      scan_field_y: 600
```

> **Note on correction grids**: Users building from scratch won't have a real `.fc3` correction file. The builder accepts either a path to a real `.fc3` file or generates a zeroed/identity grid as a placeholder.

**`ConfigEditor`** — read → modify → write back:
```python
editor = ConfigEditor("existing_config.h5")
editor.set_scanner_offset(train_index=0, x=-90.0, y=25.0)
editor.save("modified_config.h5")
```

**`MachineConfigWriter`** — the direct inverse of `MachineConfigReader`. Takes a fully populated `MachineConfig` object and writes it to a valid HDF5 file with the correct group structure, attribute names, and types the reader expects. Unlike the builders, which generate data from scratch or from specs, the writer faithfully serializes whatever it receives:

```python
# python/src/machine_config/writer.py
import h5py
from pathlib import Path
from .models import MachineConfig


class MachineConfigWriter:
    def __init__(self, config: MachineConfig):
        self.config = config

    def write(self, path: str | Path) -> None:
        """Write the MachineConfig to an HDF5 file at the given path."""
        with h5py.File(Path(path), 'w') as f:
            self._write_root_attrs(f)
            self._write_machine(f)
            self._write_optical_trains(f)

    # ------------------------------------------------------------------
    # Type conventions (mirror reader helpers in reverse):
    #   null  → empty string "" for HDF5 attributes (matches machine software)
    #   bool  → integer 0 or 1 (e.g. Thermal_Lensing_Test_Passed)
    #   float → stored as native HDF5 float64
    # ------------------------------------------------------------------
```

Usage for a real edit-and-write-back workflow:

```python
from dataclasses import replace
from machine_config import MachineConfigReader, MachineConfigWriter

config = MachineConfigReader("original_config.h5").parse()

# Replace a scanner offset immutably
train0 = config.optical_trains[0]
new_scanner = replace(train0.scanner, scan_head_offset_x=-90.0)
new_train0  = replace(train0, scanner=new_scanner)
new_config  = replace(config, optical_trains=[new_train0, *config.optical_trains[1:]])

MachineConfigWriter(new_config).write("updated_config.h5")
```

> **`writer.py` vs `builder.py`**: The writer serializes a `MachineConfig` faithfully — use it when the source of truth is a real parsed config. The builders (`MockConfigBuilder`, `YamlConfigBuilder`) generate data from specifications and are test utilities. Never use a builder to re-write a config read from a real file.

> **Status: Complete (2026-07-24).** `python/src/machine_config/writer.py` implemented as a faithful mirror of `reader.py` — all type conventions (None → `""`, bool → `0`/`1`, `Power_Bit_Resolution` as string) match the real machine software. `python/src/machine_config/builder.py` implemented with `MockConfigBuilder` (Gaussian correction grids, configurable n_lasers, working_distance_unit for unit-lock testing, `from_config()` classmethod for roundtrip use), `YamlConfigBuilder` (YAML spec → HDF5 with sensible defaults), and `ConfigEditor` (`dataclasses.replace`-based immutable edit + save). `reader.py` updated with `config_from_dict()` deserialiser and empty-string guard in `_read_bool_from_int`. CLI `write`, `build`, and `demo` commands fully wired. `__init__.py` updated with all new exports. 4 previously-skipped `test_reader.py` tests activated. `test_writer.py` and `test_builder.py` fully implemented. Full suite: **146 passed, 1 skipped** (golden file test deferred to Phase 1.7).

---

### 1.6 — Tests

```python
# tests/test_reader.py

def test_meta_machine_name(reference_config):
    assert reference_config.meta.machine_name == "TM-LPBF-02: AconityMIDI+_OG"

def test_build_plate_dimensions(reference_config):
    assert reference_config.machine.build_plate.x == 250.0
    assert reference_config.machine.build_plate.y == 250.0

def test_optical_train_count(reference_config):
    assert len(reference_config.optical_trains) == 2

def test_laser1_working_distance(reference_config):
    assert reference_config.optical_trains[0].scanner.working_distance == 670.0

def test_laser1_scanner_offset(reference_config):
    assert reference_config.optical_trains[0].scanner.scan_head_offset_x == -87.5

def test_correction_data_shape(reference_reader):
    data = reference_reader.get_correction_data(0)
    assert data.shape == (257, 257, 2)
    assert data.dtype == np.float64

def test_configuration_hash_present(reference_config):
    assert len(reference_config.meta.configuration_hash) == 64

def test_schema_validation(reference_reader):
    import jsonschema
    output = json.loads(reference_reader.to_json())
    jsonschema.validate(output, SCHEMA)  # must not raise

def test_roundtrip(tmp_path, reference_config):
    # Build → read back → compare key fields
    builder = MockConfigBuilder.from_config(reference_config)
    out = tmp_path / "roundtrip.h5"
    builder.save(out)
    config2 = MachineConfigReader(out).parse()
    assert config2.meta.machine_name == reference_config.meta.machine_name

def test_thermal_lensing_train0_failed(reference_config):
    # Train 01 has Thermal_Lensing_Test_Passed = 0 in the real fixture
    assert reference_config.optical_trains[0].thermal_lensing_passed is False

def test_thermal_lensing_train1_passed(reference_config):
    # Train 02 has Thermal_Lensing_Test_Passed = 1 in the real fixture
    assert reference_config.optical_trains[1].thermal_lensing_passed is True

def test_empty_string_field_is_none(reference_config):
    # axis_configuration stored as '' in some trains should parse to None
    for train in reference_config.optical_trains:
        ax = train.scanner.axis_configuration
        assert ax is None or isinstance(ax, str), "must be None or a non-empty string"

def test_collimator_present(reference_config):
    for train in reference_config.optical_trains:
        assert train.collimator is not None
        assert train.collimator.focal_length == 120.0
        assert train.collimator.focal_length_unit == "mm"

def test_unit_mismatch_raises_value_error(tmp_path):
    # Rule 8: reader raises ValueError if a _unit attribute does not match the
    # locked expected value for this schema version.
    builder = MockConfigBuilder(n_lasers=1, working_distance_unit="inches")
    out = tmp_path / "bad_units.h5"
    builder.save(out)
    with pytest.raises(ValueError, match="working_distance_unit"):
        MachineConfigReader(out).parse()

def test_unit_values_are_locked(reference_config):
    # Rule 8: confirm the expected locked units are present in a real fixture.
    train = reference_config.optical_trains[0]
    assert train.scanner.working_distance_unit == "mm"
    assert train.light_source.wavelength_unit == "nm"
    assert train.light_source.power_max_nominal_unit == "W"
    assert train.collimator.focal_length_unit == "mm"
    assert train.scanner_card.sample_period_unit == "μs"

def test_scanner_card_present(reference_config):
    for train in reference_config.optical_trains:
        assert train.scanner_card is not None
        assert train.scanner_card.model == "SP-ICE-3"

def test_get_inverse_correction_data_shape(reference_reader):
    import numpy as np
    data = reference_reader.get_inverse_correction_data(0)
    assert data.shape == (257, 257, 2)
    assert data.dtype == np.float64

def test_get_scan_field_correction_bytes_length(reference_config, reference_reader):
    raw = reference_reader.get_scan_field_correction_bytes(0)
    expected_size = reference_config.optical_trains[0].scan_field_correction_file.file_size
    assert len(raw) == expected_size

def test_get_raw_group_returns_empty_for_missing_path(reference_reader):
    result = reference_reader.get_raw_group("NonExistentGroup/That/Does/Not/Exist")
    assert result == {}

def test_get_raw_group_opcua_when_absent(reference_reader):
    # The reference fixture (real machine export) may or may not have OPCUA;
    # either way get_raw_group must not raise
    result = reference_reader.get_raw_group("OPCUA")
    assert isinstance(result, dict)

def test_absent_clearbox_is_none(tmp_path):
    # Build a minimal fixture with no ClearBox and confirm the field is None
    builder = MockConfigBuilder(n_lasers=1, include_clearbox=False)
    out = tmp_path / "no_clearbox.h5"
    builder.save(out)
    config = MachineConfigReader(out).parse()
    assert config.optical_trains[0].clearbox is None

def test_file_version_warning(tmp_path):
    import warnings
    builder = MockConfigBuilder(n_lasers=1, file_version="2.0")
    out = tmp_path / "future.h5"
    builder.save(out)
    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")
        MachineConfigReader(out).parse()
    assert any("File_Version" in str(warning.message) for warning in w)
```

```python
# tests/test_builder.py

def test_mock_builder_single_laser(tmp_path):
    builder = MockConfigBuilder(n_lasers=1)
    out = tmp_path / "single.h5"
    builder.save(out)
    config = MachineConfigReader(out).parse()
    assert len(config.optical_trains) == 1

def test_mock_builder_two_lasers(tmp_path):
    builder = MockConfigBuilder(n_lasers=2)
    out = tmp_path / "two.h5"
    builder.save(out)
    config = MachineConfigReader(out).parse()
    assert len(config.optical_trains) == 2

def test_mock_builder_build_plate_dimensions(tmp_path):
    builder = MockConfigBuilder(n_lasers=1, build_plate_x=300, build_plate_y=300)
    out = tmp_path / "plate.h5"
    builder.save(out)
    config = MachineConfigReader(out).parse()
    assert config.machine.build_plate.x == 300.0
    assert config.machine.build_plate.y == 300.0

def test_mock_builder_correction_grid_shape(tmp_path):
    builder = MockConfigBuilder(n_lasers=1)
    out = tmp_path / "grid.h5"
    builder.save(out)
    data = MachineConfigReader(out).get_correction_data(0)
    assert data.shape == (257, 257, 2)
    assert data.dtype == np.float64

def test_mock_builder_correction_grid_is_nonzero(tmp_path):
    """Gaussian warp pattern must not be all zeros — would hide correction-application bugs."""
    builder = MockConfigBuilder(n_lasers=1)
    out = tmp_path / "nonzero.h5"
    builder.save(out)
    data = MachineConfigReader(out).get_correction_data(0)
    assert np.any(data != 0.0)

def test_yaml_builder_roundtrip(tmp_path):
    spec = tmp_path / "spec.yaml"
    spec.write_text("""\
machine:
  name: "Test-01"
  manufacturer: "TestCo"
  model: "TestModel"
  build_plate_x: 200
  build_plate_y: 200
  gas_flow_direction: "Y+"
  recoat_direction: "X+"
optical_trains:
  - scanner:
      working_distance: 500
    light_source:
      power_max_nominal: 400
      wavelength: 1070
""")
    out = tmp_path / "yaml_built.h5"
    YamlConfigBuilder(spec).save(out)
    config = MachineConfigReader(out).parse()
    assert config.meta.machine_name == "Test-01"
    assert config.machine.build_plate.x == 200.0
    assert len(config.optical_trains) == 1

def test_config_editor_modifies_offset(tmp_path):
    builder = MockConfigBuilder(n_lasers=1)
    src = tmp_path / "original.h5"
    builder.save(src)
    editor = ConfigEditor(src)
    editor.set_scanner_offset(train_index=0, x=-99.0, y=12.5)
    out = tmp_path / "edited.h5"
    editor.save(out)
    config = MachineConfigReader(out).parse()
    assert config.optical_trains[0].scanner.scan_head_offset_x == -99.0
    assert config.optical_trains[0].scanner.scan_head_offset_y == 12.5
```

```python
# tests/test_writer.py

def test_writer_roundtrip_machine_name(reference_config, tmp_path):
    out = tmp_path / "rt.h5"
    MachineConfigWriter(reference_config).write(out)
    config2 = MachineConfigReader(out).parse()
    assert config2.meta.machine_name == reference_config.meta.machine_name

def test_writer_roundtrip_configuration_hash(reference_config, tmp_path):
    out = tmp_path / "rt.h5"
    MachineConfigWriter(reference_config).write(out)
    config2 = MachineConfigReader(out).parse()
    assert config2.meta.configuration_hash == reference_config.meta.configuration_hash

def test_writer_roundtrip_build_plate(reference_config, tmp_path):
    out = tmp_path / "rt.h5"
    MachineConfigWriter(reference_config).write(out)
    config2 = MachineConfigReader(out).parse()
    assert config2.machine.build_plate.x == reference_config.machine.build_plate.x
    assert config2.machine.build_plate.y == reference_config.machine.build_plate.y

def test_writer_roundtrip_optical_train_count(reference_config, tmp_path):
    out = tmp_path / "rt.h5"
    MachineConfigWriter(reference_config).write(out)
    config2 = MachineConfigReader(out).parse()
    assert len(config2.optical_trains) == len(reference_config.optical_trains)

def test_writer_roundtrip_scanner_offsets(reference_config, tmp_path):
    out = tmp_path / "rt.h5"
    MachineConfigWriter(reference_config).write(out)
    config2 = MachineConfigReader(out).parse()
    for i, (t1, t2) in enumerate(zip(reference_config.optical_trains, config2.optical_trains)):
        assert t2.scanner.scan_head_offset_x == t1.scanner.scan_head_offset_x, f"train {i} offset_x"
        assert t2.scanner.scan_head_offset_y == t1.scanner.scan_head_offset_y, f"train {i} offset_y"
        assert t2.scanner.working_distance   == t1.scanner.working_distance,   f"train {i} working_distance"

def test_writer_roundtrip_thermal_lensing(reference_config, tmp_path):
    # bool (True/False) written as int 0/1 and read back as bool
    out = tmp_path / "rt.h5"
    MachineConfigWriter(reference_config).write(out)
    config2 = MachineConfigReader(out).parse()
    for i, (t1, t2) in enumerate(zip(reference_config.optical_trains, config2.optical_trains)):
        assert t2.thermal_lensing_passed == t1.thermal_lensing_passed, f"train {i} thermal_lensing_passed"

def test_writer_null_field_survives_roundtrip(tmp_path):
    """A null optional field written and read back must still be null — not coerced."""
    builder = MockConfigBuilder(n_lasers=1)
    src = tmp_path / "src.h5"
    builder.save(src)
    config = MachineConfigReader(src).parse()
    config.optical_trains[0].scanner.axis_configuration = None
    out = tmp_path / "rt.h5"
    MachineConfigWriter(config).write(out)
    config2 = MachineConfigReader(out).parse()
    assert config2.optical_trains[0].scanner.axis_configuration is None

def test_writer_produces_schema_valid_output(reference_config, tmp_path):
    import json, jsonschema
    out = tmp_path / "rt.h5"
    MachineConfigWriter(reference_config).write(out)
    output = json.loads(MachineConfigReader(out).to_json())
    jsonschema.validate(output, SCHEMA)
```

```python
# tests/test_schema.py

import json, pytest, jsonschema
from machine_config.schema import SCHEMA

def test_schema_is_valid_json_schema():
    jsonschema.Draft202012Validator.check_schema(SCHEMA)

def test_schema_accepts_reference_output(reference_reader):
    output = json.loads(reference_reader.to_json())
    jsonschema.validate(output, SCHEMA)  # must not raise

def test_schema_rejects_missing_meta():
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate(
            {"machine": {}, "optical_trains": [{"train_id": "t", "scanner": {}, "light_source": {}}]},
            SCHEMA
        )

def test_schema_rejects_empty_optical_trains():
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate(
            {"meta": {}, "machine": {}, "optical_trains": []},
            SCHEMA
        )

def test_schema_rejects_too_many_optical_trains():
    dummy = {"train_id": "t", "scanner": {}, "light_source": {}}
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate(
            {"meta": {}, "machine": {}, "optical_trains": [dummy] * 7},
            SCHEMA
        )

def test_schema_rejects_short_configuration_hash():
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate(
            {"meta": {"machine_name": "x", "manufacturer": "x", "model": "x",
                      "serial_number": "x", "file_version": "1.0",
                      "export_date": "2024-01-01T00:00:00Z",
                      "configuration_hash": "tooshort"},
             "machine": {},
             "optical_trains": [{"train_id": "t", "scanner": {}, "light_source": {}}]},
            SCHEMA
        )

def test_schema_rejects_long_configuration_hash():
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate(
            {"meta": {"machine_name": "x", "manufacturer": "x", "model": "x",
                      "serial_number": "x", "file_version": "1.0",
                      "export_date": "2024-01-01T00:00:00Z",
                      "configuration_hash": "a" * 65},
             "machine": {},
             "optical_trains": [{"train_id": "t", "scanner": {}, "light_source": {}}]},
            SCHEMA
        )
```

---

### 1.7 — Generate and Verify the Golden File

**What the golden file is and is not**: `reference_output.json` is a cross-language *drift detector*, not the source of correctness. Correctness comes from the per-language unit tests in section 1.6, which hardcode specific expected values derived directly from the HDF5 structure files. Those tests must pass before the golden file is generated. The golden file then ensures all languages continue to agree with each other after the fact.

There are two independent correctness layers:

| Layer | Mechanism | What it catches |
|---|---|---|
| **1 — Field correctness** | Per-language unit tests asserting specific values (`working_distance == 670.0`) | A bug in how any field is parsed in any one language |
| **2 — Cross-language consistency** | Golden file cross-check | Drift between language outputs |

Layer 2 cannot catch a case where all languages agree on a wrong value. Layer 1 is the guard against that. The risk of Python producing a wrong golden file is therefore bounded to fields that are not individually unit-tested — which is why human review is required before the golden file is committed.

**Generation script**:

```python
# tools/generate_fixtures.py
from machine_config import MachineConfigReader
import hashlib

reader = MachineConfigReader("fixtures/reference_config.h5")
output = reader.to_json()

with open("fixtures/reference_output.json", "w") as f:
    f.write(output)

digest = hashlib.sha256(output.encode()).hexdigest()
with open("fixtures/reference_output.sha256", "w") as f:
    f.write(digest + "\n")

print(f"Golden file written.  SHA-256: {digest}")
print("REVIEW REQUIRED before committing: compare fixtures/reference_output.json")
print("against Reference Materials/*_structure.txt field by field.")
```

**Human review checklist** — every value checked against `Reference Materials/*_structure.txt` before the PR is approved:

- [ ] `meta.machine_name` matches root `machine_name` attribute
- [ ] `meta.configuration_hash` is exactly 64 hex characters and matches root `Configuration_Hash`
- [ ] `machine.build_plate_x` = 250.0, `build_plate_y` = 250.0, `build_plate_z` = 20.0
- [ ] Two optical trains present
- [ ] Train 01: `scanner.working_distance` = 670.0, `scan_head_offset_x` = −87.5, `scan_head_offset_y` = 23.5, `scan_head_rotation` = 0.0
- [ ] Train 02: `scanner.scan_head_offset_x` = 86.074, `scan_head_offset_y` = −21.695, `scan_head_rotation` = 180.0
- [ ] Train 01 `thermal_lensing_passed` = false (HDF5 integer 0)
- [ ] Train 02 `thermal_lensing_passed` = true (HDF5 integer 1)
- [ ] Both trains: `clearbox.correction_data_shape` = [257, 257, 2]
- [ ] Train 01 `scan_field_correction_file.file_size` = 1138799
- [ ] Train 02 `scan_field_correction_file.file_size` = 1142763
- [ ] No OPCUA fields appear anywhere in the output

> **Coverage note**: All items on this checklist are asserted by name in `python/tests/test_reader.py` (as of Phase 1.7 prep). If the full suite passes, every line above is machine-verified before the golden file is generated. The `TestOpcua` class separately verifies the OPCUA fixture via `get_raw_group()` — OPCUA data does not appear in `reference_output.json` (it is out-of-scope for the canonical model) but its correctness is confirmed independently.

**Checksum enforcement in CI** (added to `cross_check.yml` before the cross-check runs):

```yaml
- name: Verify golden file checksum matches committed .sha256
  run: |
    ACTUAL=$(sha256sum fixtures/reference_output.json | cut -d' ' -f1)
    EXPECTED=$(tr -d '[:space:]' < fixtures/reference_output.sha256)
    if [ "$ACTUAL" != "$EXPECTED" ]; then
      echo "FAIL: reference_output.json has changed without a matching .sha256 update."
      echo "Regenerate with tools/generate_fixtures.py, review manually, then commit both."
      exit 1
    fi
```

`reference_output.json` and `reference_output.sha256` must always be committed together. A PR that changes one without the other fails CI. Updating the golden file requires explicit human review sign-off in the PR, not just passing tests.

> **Deliverable gate**: Python must produce `fixtures/reference_output.json`, `fixtures/reference_output.sha256`, and `fixtures/synthetic_2laser.h5`, with the golden file human-reviewed and checksummed, before Phase 2 begins. At this point, `.github/workflows/python.yml` is also added so every subsequent commit is automatically tested — the CI habit starts here, not in Phase 9.

> **Status: Complete (2026-07-24).** `tools/generate_fixtures.py` implemented and run. All three fixture files generated. Golden file human-reviewed against full checklist — all 12 items verified. `test_golden_output_satisfies_schema` now active: **161 passed, 0 skipped**. `pyproject.toml` updated with `[dev]` optional extras and `requires-python = ">=3.11"`. `.github/workflows/python.yml` created — runs pytest, checksum verification, CLI validate/export-json, and golden file diff check on Python 3.11 and 3.12. SHA-256: `0cdd9a489f8a0079920485a838047fb1bd70e9e98f7204a97e328ed5e9c19ef6`.

---

## Phase 2 — Node.js

**Libraries**: `h5wasm` (same as viewer), `vitest` (test runner), `ajv` (JSON Schema), `typescript`

**Install**: `npm install machine-config-library`

**Start condition**: `fixtures/reference_output.json` committed and sha256 verified; `python.yml` CI green. Can run in parallel with Rust once the golden file exists.

> **CI added at end of Phase 2**: `.github/workflows/nodejs.yml` and `.github/workflows/cross_check.yml` (with `needs: [python, nodejs]`). This is the earliest the cross-check can run and the point at which end-to-end drift detection begins — two languages is enough.

---

### 2.1 — Package Structure

```
nodejs/
├── src/
│   ├── index.ts             ← exports MachineConfigReader, MockConfigBuilder
│   ├── models.ts            ← TypeScript interfaces matching the schema
│   ├── reader.ts            ← h5wasm-based reader
│   ├── writer.ts            ← h5wasm-based writer (inverse of reader; MachineConfig → HDF5)
│   ├── builder.ts           ← generates .h5 using h5wasm write mode
│   ├── schema.ts            ← loads and validates via ajv
│   └── adapters/
│       ├── index.ts         ← exports getChain(); adapter registry
│       └── base.ts          ← Adapter interface: adapt(config: MachineConfig): MachineConfig
├── tests/
│   ├── reader.test.ts       ← vitest tests against synthetic_2laser.h5
│   ├── schema.test.ts       ← validates canonical JSON output
│   └── adapters.test.ts     ← adapter fixture tests (added when first adapter is written)
├── examples/
│   └── quickstart.mjs
├── package.json
└── tsconfig.json
```

TypeScript interfaces mirror the Python dataclasses exactly, with the same field names. This ensures canonical JSON output is structurally identical across both implementations.

---

### 2.2 — Why Node.js Runs Against Synthetic Fixture

The reference `.h5` is 6.5 MB with scan correction file blobs. In CI with `h5wasm`, loading it is functional but slow. The `synthetic_2laser.h5` is ~200 KB and exercises all the same code paths. The cross-check (`tools/cross_check.py`) separately runs against the full reference file using Python as the oracle.

---

### 2.3 — Viewer Integration (Critical Seam)

The Node.js reader becomes the core of the viewer's **Build** and **Demo** modes:

- Bundle the Node.js reader with `esbuild` into a single `machine_config_reader.bundle.js`
- The viewer replaces its inline `readH5()` function with `import { MachineConfigReader } from './machine_config_reader.bundle.js'`
- The viewer is then always running the same tested, validated reader code — not a parallel copy

---

### 2.4 — Tests

The Node.js test suite runs against `fixtures/synthetic_2laser.h5`. Tests cover the same logical assertions as the Python suite so any structural gap between implementations surfaces immediately.

```typescript
// tests/reader.test.ts
import { describe, it, expect, beforeAll } from 'vitest';
import { MachineConfigReader } from '../src';
import type { MachineConfig } from '../src/models';

const FIXTURE = '../fixtures/synthetic_2laser.h5';
let config: MachineConfig;

beforeAll(async () => {
  config = await new MachineConfigReader(FIXTURE).parse();
});

describe('MachineConfigReader — metadata', () => {
  it('parses a non-empty machine name', () => {
    expect(config.meta.machineName).toBeTruthy();
  });

  it('configuration hash is exactly 64 characters', () => {
    expect(config.meta.configurationHash).toHaveLength(64);
  });
});

describe('MachineConfigReader — machine geometry', () => {
  it('build plate X matches expected value', () => {
    expect(config.machine.buildPlateXMm).toBeCloseTo(250.0, 5);
  });

  it('build plate Y matches expected value', () => {
    expect(config.machine.buildPlateYMm).toBeCloseTo(250.0, 5);
  });
});

describe('MachineConfigReader — optical trains', () => {
  it('returns 2 optical trains for the synthetic_2laser fixture', () => {
    expect(config.opticalTrains).toHaveLength(2);
  });

  it('reads working distance for train 0', () => {
    expect(config.opticalTrains[0].scanner.workingDistanceMm).toBeCloseTo(670.0, 5);
  });

  it('train_id is non-empty for each train', () => {
    for (const train of config.opticalTrains) {
      expect(train.trainId).toBeTruthy();
    }
  });
});

describe('MachineConfigReader — JSON serialization', () => {
  it('toJson() produces valid JSON', async () => {
    const json = await new MachineConfigReader(FIXTURE).toJson();
    expect(() => JSON.parse(json)).not.toThrow();
  });

  it('JSON output satisfies the canonical schema', async () => {
    const { validateSchema } = await import('../src/schema');
    const json = await new MachineConfigReader(FIXTURE).toJson();
    const result = validateSchema(JSON.parse(json));
    expect(result.valid).toBe(true);
  });
});
```

```typescript
// tests/schema.test.ts
import { describe, it, expect } from 'vitest';
import Ajv from 'ajv/dist/2020';
import addFormats from 'ajv-formats';
import schema from '../src/schema.json';

const ajv = new Ajv({ strict: false });
addFormats(ajv);
const validate = ajv.compile(schema);

describe('JSON Schema constraints', () => {
  it('rejects an object missing meta', () => {
    expect(validate({
      machine: {},
      optical_trains: [{ train_id: 't', scanner: {}, light_source: {} }],
    })).toBe(false);
  });

  it('rejects empty optical_trains array', () => {
    expect(validate({ meta: {}, machine: {}, optical_trains: [] })).toBe(false);
  });

  it('rejects more than 6 optical trains', () => {
    const dummy = { train_id: 't', scanner: {}, light_source: {} };
    expect(validate({ meta: {}, machine: {}, optical_trains: Array(7).fill(dummy) })).toBe(false);
  });

  it('accepts a minimal valid document', () => {
    const valid = {
      meta: {
        machine_name: 'x', manufacturer: 'x', model: 'x', serial_number: 'x',
        file_version: '1.0', export_date: '2024-01-01T00:00:00Z',
        configuration_hash: 'a'.repeat(64),
      },
      machine: {},
      optical_trains: [{ train_id: 't', scanner: {}, light_source: {} }],
    };
    expect(validate(valid)).toBe(true);
  });
});
```

---

## Phase 3 — Rust

**Libraries**: `hdf5` crate (0.8+), `serde` / `serde_json`, `thiserror` for error types, built-in `#[test]`

**Install**: `cargo add machine-config`

**Start condition**: `fixtures/reference_output.json` committed and sha256 verified; `python.yml` CI green. Can run in parallel with Node.js. The cross-check becomes active as soon as Rust OR Node.js finishes — whichever lands first.

> **CI added at end of Phase 3**: `.github/workflows/rust.yml` added; `cross_check.yml` `needs:` extended to include `rust`.

---

### 3.1 — Package Structure

```
rust/
├── src/
│   ├── lib.rs               ← pub use reader::*; pub use builder::*;
│   ├── models.rs            ← #[derive(Serialize, Deserialize)] structs
│   ├── reader.rs            ← hdf5::File → MachineConfig
│   ├── writer.rs            ← MachineConfig → hdf5::File (inverse of reader)
│   ├── builder.rs           ← hdf5::File write mode → .h5 from MachineConfigSpec
│   ├── error.rs             ← thiserror Error enum
│   └── adapters/
│       ├── mod.rs           ← pub trait Adapter; get_chain()
│       └── registry.rs      ← adapter registry (empty until first schema bump)
├── tests/
│   ├── integration_test.rs  ← tests against synthetic_2laser.h5
│   └── adapter_test.rs      ← adapter fixture tests (added when first adapter is written)
├── benches/
│   └── reader_bench.rs      ← criterion benchmark (parse 1000x)
└── Cargo.toml
```

---

### 3.2 — Key Design Decision: `hdf5` Crate System Dependency

The `hdf5` crate links against the system's `libhdf5`.

| Platform | Install Command |
|---|---|
| Ubuntu/Debian | `apt-get install libhdf5-dev` |
| macOS | `brew install hdf5` |
| Windows | Pre-built HDF5 from The HDF Group; set `HDF5_DIR` env var |

> **Alternative**: The `hdf5-metno` fork bundles a static `libhdf5`, eliminating the system dependency entirely. Worth evaluating before committing to the standard crate.

---

### 3.3 — Tests

```rust
// tests/integration_test.rs
use machine_config::MachineConfigReader;

static FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../fixtures/synthetic_2laser.h5"
);

#[test]
fn test_machine_name() {
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    assert!(!config.meta.machine_name.is_empty());
}

#[test]
fn test_optical_train_count() {
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    assert_eq!(config.optical_trains.len(), 2);
}

#[test]
fn test_working_distance() {
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let wd = config.optical_trains[0].scanner.working_distance.unwrap();
    assert!((wd - 670.0).abs() < 1e-6);
}

#[test]
fn test_json_output_validates_schema() {
    // Serialize to JSON, validate against embedded schema bytes using jsonschema crate
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let json = serde_json::to_string(&config).unwrap();
    // jsonschema validation here
}

#[test]
fn test_configuration_hash_length() {
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    assert_eq!(config.meta.configuration_hash.len(), 64);
}

#[test]
fn test_correction_data_shape() {
    let reader = MachineConfigReader::open(FIXTURE).unwrap();
    let data = reader.get_correction_data(0).unwrap();
    assert_eq!(data.shape(), &[257, 257, 2]);
}

#[test]
fn test_correction_data_is_nonzero() {
    let reader = MachineConfigReader::open(FIXTURE).unwrap();
    let data = reader.get_correction_data(0).unwrap();
    // Gaussian warp must not be all zeros — would hide correction-application bugs
    assert!(data.iter().any(|&v| v != 0.0_f64));
}

#[test]
fn test_builder_roundtrip() {
    use machine_config::builder::MockConfigBuilder;
    use tempfile::NamedTempFile;
    let file = NamedTempFile::with_suffix(".h5").unwrap();
    MockConfigBuilder::new(2).save(file.path()).unwrap();
    let config = MachineConfigReader::open(file.path()).unwrap().parse().unwrap();
    assert_eq!(config.optical_trains.len(), 2);
    assert!(!config.meta.machine_name.is_empty());
}
```

---

## Phase 4 — C++

**Libraries**: HighFive 2.x (header-only HDF5 C++ wrapper), nlohmann/json, Catch2, CMake 3.16+

**Distribution**: CMake `FetchContent` + vcpkg port

**Start condition**: Cross-check is green for at least two languages (Python + one other). C++ is built last because it has the most environment friction — doing it after Rust means the `libhdf5` system dependency issues are already understood and the HDF5 group/attribute paths have been validated by three prior implementations.

> **CI added at end of Phase 4**: `.github/workflows/cpp.yml` added; `cross_check.yml` `needs:` extended to include `cpp` — all four languages now in the cross-check.

---

### 4.1 — Package Structure

```
cpp/
├── include/machine_config/
│   ├── reader.hpp           ← template-heavy reader, header-only option
│   ├── writer.hpp           ← MachineConfig → HDF5 (inverse of reader)
│   ├── models.hpp           ← POD structs + nlohmann/json serialization
│   ├── builder.hpp          ← config writer using HighFive write mode
│   └── adapters/
│       ├── adapter.hpp      ← Adapter base struct (virtual adapt(MachineConfig&) = 0)
│       └── registry.hpp     ← adapter chain resolution (empty until first schema bump)
├── src/
│   └── reader.cpp           ← optional .cpp for compilation units
├── tests/
│   ├── CMakeLists.txt
│   ├── test_reader.cpp      ← Catch2 tests
│   └── test_adapters.cpp    ← adapter fixture tests (added when first adapter is written)
├── examples/
│   ├── quickstart.cpp
│   └── full_workflow.cpp
└── CMakeLists.txt
```

---

### 4.2 — CMakeLists.txt Key Setup

```cmake
cmake_minimum_required(VERSION 3.16)
project(machine_config VERSION 0.1.0)

include(FetchContent)

# HighFive — header-only HDF5 C++ wrapper
FetchContent_Declare(HighFive
  GIT_REPOSITORY https://github.com/BlueBrain/HighFive.git
  GIT_TAG        v2.10.0)
FetchContent_MakeAvailable(HighFive)

# nlohmann/json
FetchContent_Declare(nlohmann_json
  URL https://github.com/nlohmann/json/releases/download/v3.11.3/json.tar.xz)
FetchContent_MakeAvailable(nlohmann_json)

# Catch2 (tests only)
FetchContent_Declare(Catch2
  GIT_REPOSITORY https://github.com/catchorg/Catch2.git
  GIT_TAG v3.7.1)
FetchContent_MakeAvailable(Catch2)

find_package(HDF5 REQUIRED)   # system libhdf5

add_library(machine_config INTERFACE)
target_include_directories(machine_config INTERFACE include)
target_link_libraries(machine_config INTERFACE
  HighFive
  nlohmann_json::nlohmann_json)
```

---

### 4.3 — Why C++ Is Done Last

C++ has the most environment friction (system HDF5, CMake, vcpkg setup). Building it last means:

- The schema is 100% stable before the C++ headers are written
- Tests from Python, Node.js, and Rust define the exact expected values
- C++ verifies a known-correct implementation rather than discovering schema bugs

---

### 4.4 — Tests

C++ tests use Catch2 v3. `FIXTURES_DIR` is injected as a compiler definition from CMake (`target_compile_definitions(test_runner PRIVATE FIXTURES_DIR="${CMAKE_SOURCE_DIR}/../fixtures")`).

```cpp
// tests/test_reader.cpp
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>
#include <nlohmann/json.hpp>
#include "machine_config/reader.hpp"
#include "machine_config/builder.hpp"

static const std::string FIXTURE =
    std::string(FIXTURES_DIR) + "/synthetic_2laser.h5";

TEST_CASE("Machine name is non-empty", "[reader][meta]") {
    auto config = MachineConfig::Reader(FIXTURE).parse();
    REQUIRE_FALSE(config.meta.machine_name.empty());
}

TEST_CASE("Configuration hash is exactly 64 characters", "[reader][meta]") {
    auto config = MachineConfig::Reader(FIXTURE).parse();
    REQUIRE(config.meta.configuration_hash.size() == 64);
}

TEST_CASE("Build plate X dimension", "[reader][machine]") {
    auto config = MachineConfig::Reader(FIXTURE).parse();
    REQUIRE_THAT(config.machine.build_plate_x,
                 Catch::Matchers::WithinAbs(250.0, 1e-5));
}

TEST_CASE("Optical train count for 2-laser fixture", "[reader][optical]") {
    auto config = MachineConfig::Reader(FIXTURE).parse();
    REQUIRE(config.optical_trains.size() == 2);
}

TEST_CASE("Working distance Laser 1", "[reader][optical]") {
    auto config = MachineConfig::Reader(FIXTURE).parse();
    REQUIRE_THAT(config.optical_trains[0].scanner.working_distance.value(),
                 Catch::Matchers::WithinAbs(670.0, 1e-5));
}

TEST_CASE("train_id is non-empty for every train", "[reader][optical]") {
    auto config = MachineConfig::Reader(FIXTURE).parse();
    for (const auto& train : config.optical_trains) {
        REQUIRE_FALSE(train.train_id.empty());
    }
}

TEST_CASE("JSON output satisfies canonical schema", "[reader][schema]") {
    auto config = MachineConfig::Reader(FIXTURE).parse();
    nlohmann::json j = config;
    // Uses nlohmann/json-schema-validator (add to CMakeLists.txt as FetchContent)
    REQUIRE(MachineConfig::validate_schema(j) == true);
}

TEST_CASE("Correction data shape is 257x257x2", "[reader][correction]") {
    auto reader = MachineConfig::Reader(FIXTURE);
    auto data = reader.get_correction_data(0);
    REQUIRE(data.shape[0] == 257);
    REQUIRE(data.shape[1] == 257);
    REQUIRE(data.shape[2] == 2);
}

TEST_CASE("MockConfigBuilder roundtrip", "[builder]") {
    auto tmp = std::filesystem::temp_directory_path() / "mc_test_builder.h5";
    MachineConfig::MockConfigBuilder(2).save(tmp);
    auto config = MachineConfig::Reader(tmp).parse();
    REQUIRE(config.optical_trains.size() == 2);
    REQUIRE_FALSE(config.meta.machine_name.empty());
    std::filesystem::remove(tmp);
}
```

---

## Phase 5 — Cross-Check Tooling

### 5.1 — `tools/cross_check.py`

The correctness heartbeat. Runs all four language CLIs, collects their canonical JSON output, and diffs every field:

```python
#!/usr/bin/env python3
"""
Run all four language readers against reference_config.h5,
compare their canonical JSON output, report any discrepancies.
"""
import json, subprocess, sys
from pathlib import Path
from deepdiff import DeepDiff

FIXTURE = Path("fixtures/reference_config.h5")
GOLDEN  = Path("fixtures/reference_output.json")

RUNNERS = {
    "python": ["python", "-m", "machine_config", "export-json", str(FIXTURE)],
    "nodejs": ["node", "nodejs/dist/cli.js", "export-json", str(FIXTURE)],
    "rust":   ["rust/target/release/machine-config-cli", "export-json", str(FIXTURE)],
    "cpp":    ["cpp/build/machine_config_cli", "export-json", str(FIXTURE)],
}

def main():
    golden = json.loads(GOLDEN.read_text())
    results = {}

    for lang, cmd in RUNNERS.items():
        out = subprocess.run(cmd, capture_output=True, text=True)
        if out.returncode != 0:
            print(f"[FAIL] {lang}: process exited {out.returncode}")
            print(out.stderr)
            sys.exit(1)
        results[lang] = json.loads(out.stdout)

    all_passed = True
    for lang, result in results.items():
        diff = DeepDiff(
            golden,
            result,
            significant_digits=8,   # tolerate float repr differences across languages
            ignore_order=False
        )
        if diff:
            print(f"[FAIL] {lang} differs from golden:")
            print(diff.pretty())
            all_passed = False
        else:
            print(f"[PASS] {lang}")

    sys.exit(0 if all_passed else 1)

if __name__ == "__main__":
    main()
```

---

### 5.2 — Float Precision Policy

- All languages serialize floats with at least 8 significant digits
- The cross-check uses `significant_digits=8` tolerance
- Correction grid arrays are **excluded** from cross-check JSON — checked separately via shape and a checksum of the first/last row
- The `configuration_hash` field is compared as an exact string with no tolerance

---

### 5.3 — Cross-Check as Integration Test

The cross-check is a system-level integration test: it asserts that every language reader produces byte-for-byte identical canonical output from the same input file, to within the float tolerance defined in 5.2. It is the final correctness gate before any release.

**The cross-check starts with two languages, not four.** As soon as Python and one other language (Node.js or Rust, whichever completes first) are both CI-green, the cross-check runs against those two. Each additional language is added to the check as it lands. Waiting for all four before running any cross-check is not the plan — two languages catching drift is the point.

**Exit contract** — `cross_check.py` exits with code `0` only when all of the following pass:

| Check | Pass condition |
|---|---|
| All four language processes exit 0 | Non-zero exit from any runner fails immediately |
| All four outputs parse as valid JSON | `json.loads()` succeeds for each |
| All four outputs satisfy the schema | `jsonschema.validate()` passes for each |
| All scalar fields match the golden file | `DeepDiff` with `significant_digits=8` returns no diff |
| `configuration_hash` matches golden exactly | Exact string equality, zero tolerance |
| Correction grid shapes match per-train | Each train reports `[257, 257, 2]` |
| Correction grid row checksums match golden | SHA-256 of first and last row of each grid is identical to golden |
| Writer roundtrip: read → write → read | All scalar fields identical to pre-write parse; correction grid SHA-256 unchanged |

**Running manually** (requires all four language builds to be present):

```bash
python tools/cross_check.py
```

To check a subset while builds are in progress:

```bash
python tools/cross_check.py --langs python,nodejs
```

---

## Phase 6 — Examples

Two tiers per language, living under `examples/`. These are the primary onboarding path for new users.

---

### Quickstart (≤20 lines each)

Each quickstart does exactly five things: open the file, read a scalar, read a string, read an array shape, print a summary. Nothing else. Users run it in under 2 minutes and confirm the library works on their machine.

```python
# examples/quickstart/python/quickstart.py
from machine_config import MachineConfigReader

config = MachineConfigReader("../../../fixtures/reference_config.h5").parse()

print(f"Machine:          {config.meta.machine_name}")
print(f"Build plate:      {config.machine.build_plate.x} x {config.machine.build_plate.y} {config.machine.build_plate.x_unit}")
print(f"Optical trains:   {len(config.optical_trains)}")
print(f"Working distance: {config.optical_trains[0].scanner.working_distance} {config.optical_trains[0].scanner.working_distance_unit}")
print(f"Correction grid:  {config.optical_trains[0].clearbox.correction_data_shape}")
```

Equivalent examples are provided in Node.js (`.mjs`), Rust (`main.rs`), and C++ (`quickstart.cpp`) performing the same five operations and printing the same five values.

---

### Full Workflow (demonstrates real use)

The full workflow example shows a realistic processing pipeline. The same six steps are implemented identically in all four languages so users can compare implementations directly:

1. Open and parse the config
2. Validate the `Configuration_Hash` matches a recomputed hash
3. Extract the `(257,257,2)` correction grid for Laser 1
4. Apply the scanner `Scan_Head_Offset_X/Y` to transform a set of nominal coordinates
5. Serialize the canonical metadata to JSON
6. Print a human-readable report

---

### 6.3 — Example Smoke Tests

Each example is covered by a lightweight smoke test that runs the script and asserts the output contains the five expected lines. These tests live in `tools/test_examples.py` and run in CI after the per-language test suites pass.

```python
# tools/test_examples.py
import subprocess

def _run(cmd: list[str]) -> str:
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    assert result.returncode == 0, f"{cmd!r} exited {result.returncode}:\n{result.stderr}"
    return result.stdout

EXPECTED_LINES = ["Machine:", "Build plate:", "Optical trains:", "Working distance:", "Correction grid:"]

def _assert_quickstart_output(out: str) -> None:
    for line in EXPECTED_LINES:
        assert line in out, f"Expected {line!r} in output:\n{out}"

def test_python_quickstart():
    _assert_quickstart_output(_run(["python", "examples/quickstart/python/quickstart.py"]))

def test_nodejs_quickstart():
    _assert_quickstart_output(_run(["node", "examples/quickstart/nodejs/quickstart.mjs"]))

def test_rust_quickstart():
    _assert_quickstart_output(_run(["examples/quickstart/rust/target/release/quickstart"]))

def test_cpp_quickstart():
    _assert_quickstart_output(_run(["examples/quickstart/cpp/build/quickstart"]))
```

The smoke tests do not validate exact field values — that is the responsibility of the per-language unit tests and the cross-check. They only confirm that each quickstart binary runs without error and emits the expected output shape.

---

## Phase 7 — Viewer Enhancement

The existing viewer (`Reference Materials/machine_config_viewer.html`) already has: h5wasm HDF5 loading, canvas rendering of scanner fields, CMM dot overlay, 3MF build plate, pan/zoom. All enhancements layer on top without breaking existing functionality.

---

### 7.1 — Mode Switcher (Header Addition)

Add three tab buttons to the header between the machine name and the action buttons:

```
[ Machine Config Viewer ]  TM-LPBF-02: AconityMIDI+_OG  |  [View] [Demo] [Build]  |  Load H5  Load CSV  Load 3MF  Reset View
```

---

### 7.2 — Demo Mode

- "Demo" tab triggers `loadDemoConfig()` which loads a base64-encoded `synthetic_2laser.h5` embedded directly in the HTML
- The synthetic config is generated by `MockConfigBuilder` during the build step and embedded as a data URI
- The empty state is replaced with the full machine visualization immediately — no file drop required, no CDN wait
- A **"View Canonical JSON"** collapsible panel appears in the sidebar showing the parsed output as a syntax-highlighted code block
- This panel is the spec users implement against when writing their own language integrations

---

### 7.3 — Build Mode

The sidebar transforms into a multi-section form when Build mode is active:

```
── Machine ────────────────────────
Name:          [                    ]
Manufacturer:  [                    ]
Model:         [                    ]
Build Plate X: [250  ] mm
Build Plate Y: [250  ] mm
Gas Flow:      [Y+ ▼]
Recoat:        [X+ ▼]

── Optical Train 1 ────────────────
Working Dist:  [670  ] mm
Offset X:      [-87.5] mm
Offset Y:      [23.5 ] mm
Scan Field X:  [600  ] mm
Scan Field Y:  [600  ] mm
Max Power:     [1000 ] W

[+ Add Optical Train]

── Output ─────────────────────────
[Generate & Download H5]
[Copy Canonical JSON]
```

The viewport updates in real-time as the user types — scanner field rectangles reposition and resize live. The **Generate & Download H5** button uses h5wasm's write mode to produce a valid `.h5` in-browser and triggers a browser download. The user can immediately drop the downloaded file into **View mode** or run it against the library quickstart examples.

---

### 7.4 — Correction Grid Heatmap (View Mode Enhancement)

Add a new layer toggle: **Correction Grid**. When enabled:

- Reads `Correction_Data` (257×257×2) from the loaded H5 file
- Renders as a 2D heatmap on the canvas over each scanner's field area
- Color maps the displacement magnitude at each grid point (green → yellow → red)
- Toggle between X-channel, Y-channel, and magnitude views
- This replaces the need to load a separate CSV for correction visualization

---

### 7.5 — "Export Canonical JSON" Button

Added to the header action bar. Serializes `S.config` using the same logic as the Node.js reader bundle and triggers a `machine_config_<serial>_<date>.json` download. This is the in-browser equivalent of `machine-config export-json`.

---

### 7.6 — Node.js Reader Bundle Integration

After Phase 2, the viewer is updated to use the compiled Node.js reader bundle instead of its own inline parsing code:

- Bundle the Node.js reader with `esbuild` → `viewer/machine_config_reader.bundle.js`
- Replace the viewer's inline `readH5()` function with a call to `MachineConfigReader` from the bundle
- The viewer is then always running the same tested, validated code as the Node.js library

---

### 7.7 — Viewer Tests

Viewer tests use Playwright to automate a headless Chromium browser. They live in `viewer/tests/` and run in CI after the Node.js bundle step completes.

**Setup** (`viewer/tests/package.json`):
```json
{
  "devDependencies": {
    "@playwright/test": "^1.44.0"
  },
  "scripts": {
    "test": "playwright test"
  }
}
```

```typescript
// viewer/tests/viewer.spec.ts
import { test, expect } from '@playwright/test';
import path from 'path';

const VIEWER = `file://${path.resolve('viewer/machine_config_viewer.html')}`;

test.describe('Demo mode', () => {
  test('Demo tab loads without console errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.goto(VIEWER);
    await page.click('[data-tab="demo"]');
    await page.waitForSelector('canvas');
    expect(errors).toHaveLength(0);
  });

  test('Canvas has non-zero pixel content after Demo loads', async ({ page }) => {
    await page.goto(VIEWER);
    await page.click('[data-tab="demo"]');
    await page.waitForTimeout(1500); // allow h5wasm to process
    const hasContent = await page.evaluate(() => {
      const canvas = document.querySelector('canvas') as HTMLCanvasElement;
      const ctx = canvas.getContext('2d')!;
      const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
      return data.some((v) => v !== 0);
    });
    expect(hasContent).toBe(true);
  });

  test('Canonical JSON panel is present and contains expected keys', async ({ page }) => {
    await page.goto(VIEWER);
    await page.click('[data-tab="demo"]');
    await page.waitForSelector('[data-panel="canonical-json"]');
    const text = await page.textContent('[data-panel="canonical-json"]');
    expect(text).toContain('"meta"');
    expect(text).toContain('"optical_trains"');
    expect(text).toContain('"machine"');
  });
});

test.describe('Build mode', () => {
  test('Build tab renders the configuration form', async ({ page }) => {
    await page.goto(VIEWER);
    await page.click('[data-tab="build"]');
    await expect(page.locator('input[name="build_plate_x"]')).toBeVisible();
    await expect(page.locator('button[data-action="generate-h5"]')).toBeVisible();
  });

  test('Editing build plate X does not throw a JS error', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.goto(VIEWER);
    await page.click('[data-tab="build"]');
    await page.fill('input[name="build_plate_x"]', '300');
    await page.waitForTimeout(500);
    expect(errors).toHaveLength(0);
  });
});

test.describe('Export Canonical JSON', () => {
  test('Export button triggers a file download with the correct name pattern', async ({ page }) => {
    await page.goto(VIEWER);
    await page.click('[data-tab="demo"]');
    await page.waitForTimeout(1500);
    const [download] = await Promise.all([
      page.waitForEvent('download'),
      page.click('[data-action="export-json"]'),
    ]);
    expect(download.suggestedFilename()).toMatch(/machine_config_.*\.json/);
  });
});
```

**CI step** — add to `nodejs.yml` after the Node.js library tests pass:

```yaml
- run: npm ci
  working-directory: viewer/tests
- run: npx playwright install --with-deps chromium
  working-directory: viewer/tests
- run: npm test
  working-directory: viewer/tests
```

---

## Phase 8 — Distribution & Packaging

| Language | Registry | Package Name | Notes |
|---|---|---|---|
| Python | PyPI | `machine-config-library` | `pyproject.toml` with `[project.scripts]` entry point for the CLI |
| Node.js | npm | `machine-config-library` | ESM + CJS dual output via `tsup`; same bundle works in browser |
| Rust | crates.io | `machine-config` | Feature flag `cli` gates the binary; `default-features = false` for lib-only use |
| C++ | vcpkg | `machine-config` | Port overlay in `cpp/vcpkg-port/`; also supports CMake `FetchContent` directly |

---

### Versioning Policy

All four packages share the same semver version. When the schema changes in a way that alters the canonical JSON output, all four packages bump their major version together. This is enforced by the cross-check CI job: if `$schema` version in `machine_config_v1.schema.json` changes, the CI pipeline requires all four packages to have updated their version numbers before merging.

---

### 8.2 — Package Install Smoke Tests

After each package is published (or in a release-candidate CI job), a clean-environment smoke test installs the published artifact and runs a minimal script to confirm end-to-end functionality. These run in a separate `smoke.yml` workflow triggered only on tag pushes, not on every pull request.

**Python** — fresh `venv`:
```bash
python -m venv /tmp/smoke_py && source /tmp/smoke_py/bin/activate
pip install machine-config-library
python - <<'EOF'
from machine_config import MachineConfigReader
config = MachineConfigReader("fixtures/reference_config.h5").parse()
assert config.meta.machine_name, "machine_name must be non-empty"
assert len(config.optical_trains) >= 1, "must have at least one optical train"
print("Python install smoke: PASS")
EOF
```

**Node.js** — fresh temp directory:
```bash
mkdir /tmp/smoke_node && cd /tmp/smoke_node && npm init -y
npm install machine-config-library
node --input-type=module <<'EOF'
import { MachineConfigReader } from 'machine-config-library';
const config = await new MachineConfigReader(process.env.FIXTURE).parse();
if (!config.meta.machineName) throw new Error('machineName must be non-empty');
console.log('Node.js install smoke: PASS');
EOF
```

**Rust** — `cargo add` in a scratch project:
```bash
cargo new /tmp/smoke_rust && cd /tmp/smoke_rust
cargo add machine-config
# Write a minimal main.rs, then:
cargo run -- export-json $FIXTURE
```

**C++** — `FetchContent` integration in a scratch project:
```cmake
cmake_minimum_required(VERSION 3.16)
project(smoke)
include(FetchContent)
FetchContent_Declare(machine_config
  GIT_REPOSITORY <repo_url>
  GIT_TAG        <release_tag>)
FetchContent_MakeAvailable(machine_config)
add_executable(smoke main.cpp)
target_link_libraries(smoke PRIVATE machine_config)
```

---

## Phase 9 — CI/CD Hardening (`.github/workflows/`)

The individual language workflows (`python.yml`, `nodejs.yml`, `rust.yml`, `cpp.yml`) and the cross-check are added incrementally as each language lands — **not** all at once here. By the time Phase 9 is reached, all five workflow files already exist and the cross-check is running all four languages.

Phase 9 adds what cannot be done incrementally: **release automation, smoke tests, and publish pipelines**.

| What Phase 9 adds | Why it waits until here |
|---|---|
| Tag-triggered publish to PyPI / npm / crates.io / vcpkg | Needs all four libraries stable enough to release |
| `smoke.yml` — clean-install smoke tests per language | Needs published packages to install |
| Version-bump enforcement in `cross_check.yml` | Needs all four languages to have agreed on semver policy |
| Automated release notes generation | Needs a stable commit history across all languages |

The workflow files below are the **final state** after all incremental additions. The per-language files are shown for completeness; `cross_check.yml` shows the full four-language form.

---

### `python.yml`

```yaml
name: Python
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with: { python-version: "3.12" }
      - run: pip install -e "python/[test]"
      - run: pytest python/tests/ -v --tb=short
      - run: machine-config validate fixtures/reference_config.h5
      - run: machine-config export-json fixtures/reference_config.h5 --output /tmp/python_output.json
      - uses: actions/upload-artifact@v4
        with:
          name: python-output
          path: /tmp/python_output.json
```

---

### `nodejs.yml`

```yaml
name: Node.js
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: "20" }
      - run: npm ci
        working-directory: nodejs
      - run: npm run build
        working-directory: nodejs
      - run: npx vitest run
        working-directory: nodejs
      - run: node nodejs/dist/cli.js export-json fixtures/reference_config.h5 > /tmp/nodejs_output.json
      - uses: actions/upload-artifact@v4
        with:
          name: nodejs-output
          path: /tmp/nodejs_output.json
```

---

### `rust.yml`

```yaml
name: Rust
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: sudo apt-get install -y libhdf5-dev
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-features
        working-directory: rust
      - run: cargo build --release --features cli
        working-directory: rust
      - run: rust/target/release/machine-config-cli export-json fixtures/reference_config.h5 > /tmp/rust_output.json
      - uses: actions/upload-artifact@v4
        with:
          name: rust-output
          path: /tmp/rust_output.json
```

---

### `cpp.yml`

```yaml
name: C++
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: sudo apt-get install -y libhdf5-dev cmake ninja-build
      - run: cmake -B build -G Ninja -DBUILD_TESTS=ON
        working-directory: cpp
      - run: cmake --build build
        working-directory: cpp
      - run: ctest --test-dir build --output-on-failure
        working-directory: cpp
      - run: cpp/build/machine_config_cli export-json fixtures/reference_config.h5 > /tmp/cpp_output.json
      - uses: actions/upload-artifact@v4
        with:
          name: cpp-output
          path: /tmp/cpp_output.json
```

---

### `cross_check.yml`

```yaml
name: Cross-Language Check
on: [push, pull_request]
jobs:
  cross_check:
    needs: [python, nodejs, rust, cpp]    # only runs if all four pass
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with: { python-version: "3.12" }
      - run: pip install deepdiff
      - uses: actions/download-artifact@v4
        with: { path: /tmp/lang-outputs }
      - run: python tools/cross_check.py --artifacts-dir /tmp/lang-outputs
```

> The cross-check job failing means the libraries agree internally but disagree with the golden file — the most important class of bug to catch.

---

## Phase 10 — Go (Deferred — begin only after Phase 9 is stable)

**Prerequisite**: Phases 1–9 complete, cross-check CI green for at least one full release cycle, and Decision 5 revisited with a confirmed use case. No Go code is written until this gate is cleared.

**Libraries**: `gonum/hdf5` (CGO binding to `libhdf5`) for HDF5 access, `encoding/json` (stdlib), `github.com/santhosh-tekuri/jsonschema/v6` for JSON Schema validation, `github.com/spf13/cobra` for CLI

**Install**: `go get github.com/<org>/machine-config`

---

### 10.1 — Package Structure

```
go/
├── machineconfig/
│   ├── models.go            ← structs with json tags matching the schema
│   ├── reader.go            ← HDF5 → MachineConfig
│   ├── writer.go            ← MachineConfig → HDF5 (inverse of reader)
│   ├── builder.go           ← MockConfigBuilder, YamlConfigBuilder
│   ├── schema.go            ← embeds machine_config_v1.schema.json; validates
│   └── adapters/
│       ├── adapters.go      ← Adapter interface; GetChain()
│       └── registry.go      ← adapter registry (auto-generated by Jinja)
├── cmd/machine-config/
│   └── main.go              ← CLI: inspect, validate, export-json, write, build
├── go.mod
└── go.sum
```

---

### 10.2 — HDF5 Binding

The `gonum/hdf5` CGO binding links against system `libhdf5` — same install commands as Rust (`apt-get install libhdf5-dev`, `brew install hdf5`, HDF Group pre-built on Windows). Evaluate whether a static-linking option exists before committing, following the same decision process as Rust’s `hdf5-metno` alternative.

---

### 10.3 — Adapter Generator Extension

Add `adapter_go.go.j2` to `tools/templates/` and one entry to `OUTPUTS` in `tools/generate_adapters.py`:

```python
"go": ("adapter_go.go.j2", "go/machineconfig/adapters/{name}.go"),
```

Running `python tools/generate_adapters.py` then generates the Go adapter alongside the other four automatically. No other changes to the generator are required.

---

### 10.4 — Tests

```go
// machineconfig/reader_test.go
package machineconfig_test

import (
    "path/filepath"
    "runtime"
    "testing"
    mc "github.com/<org>/machine-config/machineconfig"
)

var fixture = func() string {
    _, f, _, _ := runtime.Caller(0)
    return filepath.Join(filepath.Dir(f), "../../fixtures/synthetic_2laser.h5")
}()

func TestMachineName(t *testing.T) {
    config, err := mc.Open(fixture).Parse()
    if err != nil { t.Fatal(err) }
    if config.Meta.MachineName == "" {
        t.Error("machine_name must be non-empty")
    }
}

func TestOpticalTrainCount(t *testing.T) {
    config, _ := mc.Open(fixture).Parse()
    if len(config.OpticalTrains) != 2 {
        t.Errorf("expected 2 optical trains, got %d", len(config.OpticalTrains))
    }
}

func TestWorkingDistance(t *testing.T) {
    config, _ := mc.Open(fixture).Parse()
    wd := config.OpticalTrains[0].Scanner.WorkingDistance
    if wd == nil || *wd < 669.9 || *wd > 670.1 {
        t.Errorf("expected working_distance ≈ 670.0, got %v", wd)
    }
}

func TestWriterRoundtrip(t *testing.T) {
    config, _ := mc.Open(fixture).Parse()
    tmp := t.TempDir() + "/roundtrip.h5"
    if err := mc.NewWriter(config).Write(tmp); err != nil { t.Fatal(err) }
    config2, _ := mc.Open(tmp).Parse()
    if config2.Meta.MachineName != config.Meta.MachineName {
        t.Error("roundtrip machine_name mismatch")
    }
    if config2.Meta.ConfigurationHash != config.Meta.ConfigurationHash {
        t.Error("roundtrip configuration_hash mismatch")
    }
}

func TestSchemaValidation(t *testing.T) {
    config, _ := mc.Open(fixture).Parse()
    if err := mc.ValidateSchema(config); err != nil {
        t.Errorf("schema validation failed: %v", err)
    }
}
```

---

### 10.5 — CI

Add `go.yml` and extend `cross_check.yml` `needs:` to include the Go job:

```yaml
# .github/workflows/go.yml
name: Go
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: sudo apt-get install -y libhdf5-dev
      - uses: actions/setup-go@v5
        with: { go-version: "1.22" }
      - run: go test ./...
        working-directory: go
      - run: go build -o /tmp/mc-go-cli ./cmd/machine-config
        working-directory: go
      - run: /tmp/mc-go-cli export-json fixtures/reference_config.h5 > /tmp/go_output.json
      - uses: actions/upload-artifact@v4
        with:
          name: go-output
          path: /tmp/go_output.json
```

---

## Execution Model — Vertical Slices, Not Horizontal Layers

**The principle**: each language is built as a complete shippable milestone — reader, writer, builder, CLI, tests, CI pipeline, package registry — before the next language begins. Do not build all readers first and then all writers. Get one language working end-to-end, ship it, learn from it, then add the next. The schema stabilizes during Python; by the time C++ starts, three prior implementations have validated the HDF5 path and attribute mappings.

## Sequencing & Dependencies

```
Phase 0 — Foundation (shared, always first, ~2–3 days)
    Schema, fixture strategy, adapter architecture, Jinja generator setup
         │
         └─ Gate: schema v1.0 committed; fixture strategy confirmed
         ▼

Phase 1 — Python (complete vertical slice, ~1–2 weeks)
    reader → writer → builder → CLI → tests → golden file → CI (python.yml) → PyPI v0.1.0
         │
         └─ Gate: golden file committed + sha256 verified; python.yml CI green
         ▼

    ├──► Phase 2 — Node.js (complete vertical slice, ~3–4 days)
    │       reader → writer → builder → tests → nodejs.yml + cross_check.yml (2 languages) → npm v0.1.0
    │            │
    │            └─ Gate: cross-check green (Python + Node.js); Node.js bundle exists
    │            ▼
    │       Phase 7 — Viewer (~2–3 days)
    │
    ├──► Phase 3 — Rust (complete vertical slice, ~3–4 days)
    │       reader → writer → builder → tests → rust.yml; extend cross_check to 3 languages → crates.io v0.1.0
    │
    └──► Phase 5 — Cross-check active from Phase 2 onward
                 2 languages after Phase 2; 3 after Phase 3; 4 after Phase 4.
                 Each new language is one `needs:` addition to cross_check.yml.
                      │
                      └─ Gate: cross-check green for 3+ languages
                      ▼
             Phase 4 — C++ (complete vertical slice, ~3–5 days)
                 reader → writer → builder → tests → cpp.yml; extend cross_check to 4 languages → vcpkg v0.1.0

Phase 6 — Examples      ← parallel with Phases 2–4              (~1–2 days)
Phase 8 — Distribution  ← after each language is CI-green        (~1–2 days/language)
Phase 9 — CI Hardening  ← smoke tests, publish automation, tags  (~1 day)

Phase 10 — Go           ← DEFERRED: after Phase 9 stable
                             + Decision 5 gate cleared             (~3–4 days)
```

**Milestone release gates** — each language is independently shippable and the next language does not start until the prior gate passes:

| Milestone | Gate before work on the next language begins |
|---|---|
| **Python v0.1.0** | Golden file committed + sha256 verified; `python.yml` CI green |
| **Node.js v0.1.0** | `nodejs.yml` + `cross_check.yml` (2 languages) green; viewer bundle builds |
| **Rust v0.1.0** | `rust.yml` green; `cross_check.yml` extended to 3 languages and green |
| **C++ v0.1.0** | `cpp.yml` green; `cross_check.yml` extended to 4 languages and green |

**Total calendar estimate** (one developer, full focus): ~6–8 weeks end-to-end. Python alone is usable in 1–2 weeks. Python + Node.js + viewer is the point at which real users can interact with the library through the browser without installing anything — reachable in 2–3 weeks.

---

## User Onboarding Flow

This is the intended path for a new user arriving at the library:

```
New user arrives
      │
      ▼
Opens viewer HTML → clicks "Demo" → sees machine visualization immediately
(zero files required, zero install)
      │
      ▼
Reads "View Canonical JSON" panel → understands the data model and field names
      │
      ├─► Has a real .h5?
      │       └── clicks "View", drops file → full visualization immediately
      │
      ├─► Wants to test without a real file?
      │       └── clicks "Build", fills form, clicks "Generate & Download H5"
      │           → downloads a valid .h5 → runs quickstart example against it
      │
      └─► Ready to integrate?
              └── installs library for their language
                  → runs quickstart (confirms it works, ~2 min)
                  → runs full_workflow example (real pipeline, ~10 min)
                  → uses YamlConfigBuilder or MockConfigBuilder for test fixtures
```

---

## Decision Points Before Starting

Three decisions to make before writing any code:

### Decision 1 — Correction Grid Arrays in Canonical JSON?

The plan above **excludes** them from JSON (accessible via a separate `get_correction_data()` method). Including them would make the canonical JSON ~8 MB per file and make the cross-check slow. Confirm this exclusion before defining the schema.

### Decision 2 — Synthetic Fixture vs. Real File in Tests?

Using `MockConfigBuilder`-generated `synthetic_2laser.h5` keeps the repo clean (no large binary committed). Using a copy of the real `.h5` makes tests more realistic but adds ~6.5 MB to the repository. Recommendation: use the synthetic fixture in unit tests, use the real file only in the cross-check job (fetched from a release asset or LFS).

### Decision 3 — Viewer Build Step or Single HTML File?

| Approach | Pros | Cons |
|---|---|---|
| Single HTML file | Zero install, works from `file://`, fully portable | Node.js bundle must be inlined (~500 KB); no tree-shaking |
| Build step (esbuild/vite) | Clean imports, tree-shaken, proper module structure | Requires Node.js to build; two artifacts to distribute |

Recommendation: single HTML file for the distributed viewer (portable, no install), build step only in CI to regenerate it from the Node.js source.

### Decision 4 — Peripheral HDF5 Groups (OPCUA, Axis Config)

Some HDF5 files contain groups that are outside the machine optics domain — `OPCUA` (machine connectivity), `Scanner/X_Axis` / `Y_Axis` / `Z_Axis` (axis tuning), and potentially others in future file versions.

| Option | Description | Recommendation |
|---|---|---|
| Silently ignore | `parse()` skips unknown groups entirely | ✓ **Adopt** — keeps canonical output stable |
| Expose via `get_raw_group(path)` | Caller requests any HDF5 path; returns raw attribute dict | ✓ **Adopt** — no schema commitment required |
| Add to canonical schema | OPCUA fields in the JSON output | ✗ Reject — ties the schema to connectivity config that varies by deployment |

**Resolution**: `parse()` ignores peripheral groups. `get_raw_group(hdf5_path)` exposes them on demand, returning an empty dict for any path that does not exist. This gives callers full access without baking volatile connectivity config into the cross-language contract.

### Decision 5 — Go Language Implementation

Go offers strong typing, fast compilation, excellent CLI tooling, and a growing presence in industrial infrastructure and cloud-connected manufacturing systems. It is a viable fifth language for this library. The `gonum/hdf5` CGO binding carries the same system `libhdf5` dependency as the Rust crate, and the Jinja adapter generator already has an extension point — adding `adapter_go.go.j2` to `tools/templates/` and one entry to `OUTPUTS` covers Go automatically.

**Resolution**: Defer Go until the four-language core (Python, Node.js, Rust, C++) is stable with all tests passing and the cross-check green. Implementation does not begin until:

1. All Phases 1–9 are complete and CI is green
2. A specific use case for Go has been identified (e.g. a Go-based machine control service or data pipeline)

Go is tracked as Phase 10. The CI workflow file is added only when implementation begins — not as a placeholder.
