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
| Reader tests | `python/tests/test_reader.py` | 1.3/1.6 | `.\.venv\Scripts\python.exe -m pytest python/tests/test_reader.py -v` | Full parse of real AconityMIDI `.h5` fixtures; all HDF5 → model field mappings; Rule 8 unit locking; ClearBox; scan-field correction file; thermal lensing; `get_raw_group`; JSON/schema round-trip; MockConfigBuilder roundtrip; unit-mismatch error; absent ClearBox; File_Version warning; OPCUA group values (Client/Pipe/Triggers/trigger subgroups); `include_binary` flag (`TestIncludeBinaryFlag`: default excludes correction arrays and raw bytes; `include_binary=True` restores them with full data) |
| CLI tests | `python/tests/test_cli.py` | 1.4/1.5 | `.\.venv\Scripts\python.exe -m pytest python/tests/test_cli.py -v` | `inspect`, `validate`, `export-json` against real fixture; `write`, `build --mock`, `build --from-yaml`, `demo` (Phase 1.5 implemented); `--version`, `--help` |
| Writer tests | `python/tests/test_writer.py` | 1.6 | `.\.venv\Scripts\python.exe -m pytest python/tests/test_writer.py -v` | HDF5 write → re-parse roundtrip for machine name, hash, build plate, train count, scanner offsets, thermal lensing, null fields, schema validity |
| Builder tests | `python/tests/test_builder.py` | 1.6 | `.\.venv\Scripts\python.exe -m pytest python/tests/test_builder.py -v` | `MockConfigBuilder` (1 and 2 lasers, plate dims, correction grid shape/non-zero); `YamlConfigBuilder` roundtrip; `ConfigEditor` offset mutation |
| Writer roundtrip tests | `python/tests/test_writer_roundtrip.py` | 1.8d | `.\.venv\Scripts\python.exe -m pytest python/tests/test_writer_roundtrip.py -v` | Explicit field-level roundtrip: every scalar field; all 18 ClearBox scalar attributes; NaN↔None convention; axis_configuration "2D"/"3D"/"3D+Focus"; no-ClearBox path; schema validity |
| OPCUA roundtrip tests | `python/tests/test_opcua_roundtrip.py` | 1.8e | `.\.venv\Scripts\python.exe -m pytest python/tests/test_opcua_roundtrip.py -v` | OPCUA model construction and bidirectional roundtrip: OpcuaClientConfig, OpcuaPipeConfig, OpcuaTrigger (known fields + extra); write with OPCUA → read back → assert all fields; without-OPCUA path; schema validity with and without OPCUA |
| Schema tests (Node.js) | `nodejs/tests/schema.test.ts` | 2.1 | `cd nodejs && npx vitest run tests/schema.test.ts` | Schema loads; `validate()` rejects empty object, missing `meta`, empty `optical_trains`, 7-train array; accepts a minimal valid document |
| Writer tests (Node.js) | `nodejs/tests/writer.test.ts` | 2.4/4 | `cd nodejs && npx vitest run tests/writer.test.ts` | 21 tests: class smoke tests; reference-fixture roundtrip (machine name, hash, train count, build plate X, working distance, thermal lensing, SFCF document name/file size, ClearBox ip_address); OPC-UA fixture roundtrip (client, session timeout, triggers_enabled, trigger names/signal); synthetic 2-laser roundtrip (train count, scan_head_rotation) |
| Reader tests (Node.js) | `nodejs/tests/reader.test.ts` | 2.4/2.5/5 | `cd nodejs && npm test` | 78 fixture-based tests across all 3 canonical fixtures: meta (`schema_version`, `machine_name`, `configuration_hash` 64 hex chars), machine geometry (`build_plate_x/y` ≈ 250 mm), optical trains (counts, `train_id`, `optional_components`), scanner (`working_distance`, `scan_head_offset_x/y` both trains, `scan_head_rotation` 0°/180°, units), collimator (`focal_length` 120 mm), light_source (`wavelength_unit` nm), scanner_card (`SP-ICE-3`, `sample_period_unit` μs), thermal lensing (`false` train 0, `true` train 1), ClearBox presence + `data_port` type, ScanFieldCorrectionFile `file_size` (1138799/1142763), correction data arrays `includeBinary:true` (shape [257][257][2], `null` for NaN), `getCorrectionData`/`getInverseCorrectionData` (shape, NaN preserved, forward ≠ inverse, train 0 ≠ train 1, deterministic re-read — feeds `correction-hash` CLI), OPCUA 13 tests (`bfs_max_depth=16`, `session_timeout=60000`, `"Laser Emission Interlock"`, trigger fields, pipe buffer 65536), JSON serialization, AJV schema validation all 3 fixtures |
| Builder tests (Node.js) | `nodejs/tests/builder.test.ts` | 2.4 | `cd nodejs && npx vitest run tests/builder.test.ts` | 18 tests, new: `build()` in-memory (default 2 lasers, `nLasers` override, default/custom build-plate dims, `machine_name`, `configuration_hash` length, alternating scan-head offset sign/rotation per train, ClearBox+SFCF present/absent via `includeClearbox`, correction grid shape + ~2.0 centre peak); `save()` + read-back roundtrip (train count, `machine_name`, correction grid shape/peak/non-zero via `getCorrectionData`, inverse grid = forward × 0.9 via `getInverseCorrectionData`, no-clearbox path, schema validity) |

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

### Rust Library (Phase 3, in progress)

The Rust crate lives in `rust/`. It has no system dependencies to install — the first `cargo build` compiles `libhdf5` from source (see §3.2) and takes a few minutes; subsequent builds are fast.

```powershell
# PowerShell — from repo root
cd rust
cargo test              # runs lib unit tests + integration tests + doctests
cargo test --lib        # lib unit tests only (fast; needs fixtures/*.h5, already committed)
cargo build --all-targets
```

```bash
# Git Bash
cd rust
cargo test
cargo test --lib
cargo build --all-targets
```

**Current status**: Phase 3 complete — all phases §3.1–§3.9 done. **60 tests passing** (46 lib + 14 integration), `cargo build --all-targets` clean, zero warnings. CI added: `.github/workflows/rust.yml` (matrix: `windows-latest` + `ubuntu-latest`; uploads JSON + correction hash artifacts; compare job diffs Linux vs Windows). Cross-check added: `.github/workflows/cross_check.yml` + `tools/cross_check.py` (4 phases: schema validation, read parity, write interop, correction hash parity; runs on both `ubuntu-latest` and `windows-latest`). All cross-check phases green locally and in CI (Python ↔ Rust).

**Public API additions (Phase 3.9+)**:
- `CorrectionData { data: Vec<f64>, shape: [usize; 3] }` — ndarray removed from public API; correction grids exposed as plain `Vec<f64>`
- `get_correction_data(train) -> Result<CorrectionData>` / `get_inverse_correction_data(train) -> Result<CorrectionData>`
- CLI: `write-hdf5 <json> <output.h5>` — writes HDF5 from canonical JSON input
- CLI: `correction-hash <path> [--train N] [--inverse]` — prints SHA-256 of correction grid as flat little-endian f64 bytes

**Reading a config today**:

```rust
use machine_config::reader::MachineConfigReader;

let reader = MachineConfigReader::open("fixtures/reference_config.h5")?;
let config = reader.parse()?;                    // scalars/metadata only
println!("{}", config.meta.machine_name);

let full = reader.parse_with_binary()?;           // + ClearBox correction grids, raw .fc3 bytes
let json = reader.to_json(true, false)?;          // pretty JSON, binary fields omitted
```

---

### Node.js Library (Phase 2 — complete: reader, writer, builder, CLI, quickstart, CI)

The package lives in `nodejs/`. No native compilation — `h5wasm` is the HDF5 C library compiled to WebAssembly by NIST; `npm install` is the only setup step.

```powershell
# PowerShell — from repo root
cd nodejs
npm install
npm run build      # compile TypeScript → dist/
npm test           # 123 tests: 78 reader + 6 schema + 21 writer + 18 builder
```

```bash
# Git Bash
cd nodejs
npm install
npm run build
npm test
```

**Current status**: Phase 2.3 (write interop refactor) + all six Phase 2.4 vertical-slice steps + Phase 2.5 (reader tests) complete — `models.ts`, `reader.ts`, `schema.ts`, `writer.ts`, `builder.ts`, all three CLI subcommands (`export-json`/`write-hdf5`/`correction-hash`), `.github/workflows/nodejs.yml`, `examples/quickstart/nodejs/main.mjs`, and data-driven `cross_check.py` Phases 1–4 all implemented. **123 tests passing** (78 reader, incl. `getCorrectionData`/`getInverseCorrectionData` + 6 schema + 21 writer roundtrip + 18 builder). `nodejs.yml` runs on `ubuntu-latest` + `windows-latest`; `compare` job diffs both JSON output and correction-hash output between platforms. `cross_check.yml` updated: **all four phases now run for `python,rust,nodejs`** — schema, read parity, write interop, and correction-hash all verified live across all three languages (byte-for-byte identical SHA-256 digests). `MockConfigBuilder` mirrors Python's/Rust's builder (same defaults, same Gaussian correction-grid formula); the quickstart runs the same six-step read/write/round-trip demo as Python and Rust. Phase 2 has no remaining gaps.

**Reading a config today**:

```typescript
import { MachineConfigReader } from './dist/index.js';

const reader = new MachineConfigReader('fixtures/reference_config.h5');
const config = await reader.parse();
console.log(config.meta.machine_name);           // "TM-LPBF-02: AconityMIDI+_OG"
console.log(config.meta.configuration_hash);     // 64-character hex string
console.log(config.machine.build_plate_x);       // 250

for (const train of config.optical_trains) {
  const s = train.scanner;
  console.log(`WD=${s.working_distance} ${s.working_distance_unit}`);
}

if (config.opcua) {
  console.log(config.opcua.client.session_timeout);  // 60000
}

const json = await reader.toJson({ indent: 2 });     // metadata + scalars only (~13 KB)
```

**CLI (export-json implemented)**:

```powershell
# PowerShell — from repo root
node nodejs/dist/cli.js export-json fixtures/reference_config.h5
node nodejs/dist/cli.js export-json fixtures/reference_config.h5 > output.json
```

```bash
# Git Bash
node nodejs/dist/cli.js export-json fixtures/reference_config.h5
```

---

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
| `python/src/machine_config/reader.py` | 1.3/1.5 | `MachineConfigReader` — parses `.h5` → `MachineConfig`; Rule 8 unit locking; `to_json(include_binary=False)` produces schema-valid metadata-only JSON (correction arrays and raw `.fc3` bytes excluded by default, accessible via `get_correction_data()` / `get_scan_field_correction_bytes()` or `to_json(include_binary=True)`); `config_from_dict()` module-level deserialiser (JSON dict → `MachineConfig`) |
| `python/tests/conftest.py` | 1.3 | Session-scoped fixtures: `reference_reader`, `reference_config`, `opcua_reader` |
| `python/tests/test_reader.py` | 1.3/1.6 | Reader test suite: 68 tests across 11 classes (includes `TestOpcua` with 12 OPCUA value tests against `reference_config_opcua.h5`; `scan_head_offset_y` asserted for both trains; `TestIncludeBinaryFlag` with 7 tests covering the `include_binary` flag) |
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
| `python/tests/test_writer_roundtrip.py` | 1.8d | Bidirectional write roundtrip suite: 110 tests across 6 classes; every scalar field; all 18 ClearBox scalar attrs; NaN↔None; axis_configuration "2D"/"3D"/"3D+Focus"; no-ClearBox path; schema validity |
| `python/tests/test_opcua_roundtrip.py` | 1.8e | OPCUA roundtrip suite: OpcuaClientConfig/OpcuaPipeConfig/OpcuaTrigger models; write-with-OPCUA → read-back field assertion; `extra` passthrough; without-OPCUA path; schema validity |
| `nodejs/package.json` | 2.0 | npm package manifest: `h5wasm ^0.10.3`, `ajv ^8.17.1`, `commander ^12.1.0`; devDeps: `typescript ^5.5.4`, `vitest ^3.0.0`, `@types/node ^22.0.0`; scripts: `build` (tsc), `test` (vitest run), `typecheck` |
| `nodejs/tsconfig.json` | 2.0 | TypeScript config: target ES2022, module NodeNext, strict mode, `outDir dist/`, `rootDir src/` |
| `nodejs/src/models.ts` | 2.4/1 | Full TypeScript interface tree (15 interfaces) mirroring Python models; snake_case throughout; `optional_components: OptionalComponents` always present; `opcua?: OpcuaConfig` optional |
| `nodejs/src/schema.ts` | 2.1 | Schema loader + AJV compiler: loads `schema/machine_config_v1.schema.json` via `createRequire`; exports `validate(data): string[]` — empty array = valid; `strict: false` to suppress format warnings |
| `nodejs/src/reader.ts` | 2.4/2/5 | `MachineConfigReader` — h5wasm-based HDF5 → `MachineConfig`; `parse(options?)` and `toJson(options?)`; `includeBinary` flag for correction data and raw `.fc3` bytes; `getCorrectionData(trainIndex)` / `getInverseCorrectionData(trainIndex)` — raw NaN-preserving `Float64Array` + shape, bypassing JSON null-conversion, mirroring Python/Rust; NODERAWFS host filesystem access; type-dispatching attribute helpers (Rules 1–8 §0.5) |
| `nodejs/src/writer.ts` | 2.4/4 | `MachineConfigWriter` — h5wasm-based `MachineConfig` → HDF5; exact inverse of `reader.ts`; `ws`/`wf`/`wi`/`wb` helpers mirror Python `_s`/`_f`/`_i`/`_b` and Rust conventions; zero-filled correction grids when binary data absent; `extra` passthrough; OPCUA write path |
| `nodejs/src/builder.ts` | 2.4 | `MockConfigBuilder` — mirrors `python/src/machine_config/builder.py`/`rust/src/builder.rs` field-for-field (same defaults, same Gaussian correction-grid formula, peak 2.0, inverse grid ×0.9); `build()` (sync, in-memory) and `save(path)` (async, via `MachineConfigWriter`); fixed `machine.id`/`meta.export_date` constants for reproducibility, matching Rust's convention rather than Python's random-UUID/current-time one |
| `nodejs/src/cli.ts` | 2.4/5 | `export-json`, `write-hdf5`, and `correction-hash` (`--train <n> [--inverse]`, SHA-256 of flat little-endian float64 bytes via `node:crypto`) all fully implemented |
| `nodejs/src/index.ts` | 2.1 | Package exports: `MachineConfigReader`, `MachineConfigWriter`, `MockConfigBuilder`, `SCHEMA_VERSION`/`getSchema`/`validate`, adapters, all model types |
| `nodejs/src/adapters/` | 2.1 | Adapter layer scaffold — empty until first schema bump |
| `nodejs/tests/schema.test.ts` | 2.1 | 6 live tests: schema loads; `validate()` rejects empty/missing-meta/empty-trains/7-trains objects; accepts minimal valid document |
| `nodejs/tests/writer.test.ts` | 2.4/4 | 21 tests — see Test Suites table |
| `nodejs/tests/reader.test.ts` | 2.4/2.5/5 | 78 fixture-based reader tests across all 3 canonical fixtures (see Test Suites table) |
| `nodejs/tests/builder.test.ts` | 2.4 | 18 tests — see Test Suites table |
| `examples/quickstart/nodejs/main.mjs` | 2.4/6 | Node.js quickstart — plain ESM script (no `.ts`/build step of its own); mirrors `examples/quickstart/python/main.py` and `rust/examples/quickstart.rs`; imports from `nodejs/dist/index.js`; resolves repo root via `import.meta.url`; prints PASS/FAIL with field-level diagnostics |
| `rust/Cargo.toml` | 3.1 | Rust package manifest: `hdf5-metno 0.12` (aliased as `hdf5`, static; no system install), `hdf5-metno-sys 0.11`, `serde`/`serde_json`, `thiserror`, `indexmap`, `clap`, `ndarray 0.16`, `sha2 0.10`; CI matrix: `windows-latest` + `ubuntu-latest` |
| `rust/src/lib.rs` | 3.1 | Crate root — `pub mod` declarations for all six modules |
| `rust/src/error.rs` | 3.3 | `MachineConfigError` enum (Hdf5, Json, Parse, UnitMismatch, UnsupportedVersion, MissingGroup); `pub type Result<T>`; 6 inline unit tests |
| `rust/src/models.rs` | 3.4 | All 15 data model structs (`MachineConfig` and its full field tree) mirroring `python/src/machine_config/models.py`; `ExtraAttrs` = `IndexMap<String, serde_json::Value>` type alias; `CorrectionData { data: Vec<f64>, shape: [usize; 3] }` public type (ndarray removed from public API); 7 inline unit tests |
| `rust/src/reader.rs` | 3.5 | `MachineConfigReader` — `open`/`parse`/`parse_with_binary`/`get_correction_data(usize) → Result<CorrectionData>`/`get_inverse_correction_data(usize) → Result<CorrectionData>`/`get_scan_field_correction_bytes`/`get_raw_group`/`to_json`; ndarray internal-only; type-dispatching attribute helpers (Rules 1–8, §3.11); 15 inline unit tests incl. a deep-equality check against `fixtures/reference_output.json` |
| `rust/src/writer.rs` | 3.6 | `MachineConfigWriter<'a>` — HDF5 writer (inverse of reader); `ws`/`wf`/`wi`/`wb` helpers; correction-grid write with NaN; OPCUA; `extra` passthrough; 11 inline unit tests incl. SHA-256 correction roundtrip |
| `rust/src/builder.rs` | 3.7 | `MockConfigBuilder` — synthetic config generator with Gaussian correction grids; `build()`/`save()`; 6 inline unit tests incl. shape, nonzero-peak, no-clearbox path |
| `rust/src/main.rs` | 3.8 | `machine-config-cli` binary — three subcommands: `export-json <path> [--include-binary]` (JSON to stdout); `write-hdf5 <json> <output.h5>` (writes HDF5 from canonical JSON); `correction-hash <path> [--train N] [--inverse]` (SHA-256 of flat little-endian f64 correction bytes) |
| `rust/src/adapters/mod.rs` | 3.1 | Adapter layer scaffold — empty until first schema bump |
| `rust/src/adapters/registry.rs` | 3.1 | Adapter registry scaffold — empty until first schema bump |
| `rust/tests/integration_test.rs` | 3.9 | 14 integration tests across 3 fixtures: structural, correction-data, OPCUA, builder roundtrip, writer roundtrip |
| `.github/workflows/rust.yml` | 3.CI | Rust CI: `cargo test` + `cargo build --release` on matrix `ubuntu-latest`/`windows-latest`; CLI smoke-test exports JSON + correction hashes for all 3 fixtures (forward + inverse); both artifacts uploaded; `compare` job diffs Linux vs Windows JSON and correction hashes |
| `.github/workflows/cross_check.yml` | 3.CI/2.4/5 | Cross-language CI: matrix `ubuntu-latest`/`windows-latest`; `PYTHONUTF8=1` job-level env; installs Python + deepdiff, builds Rust + Node.js; **all four phases now run for `python,rust,nodejs`** (schema, read parity, write interop, correction-hash all verified live for all three); uploads per-OS inspection artifacts including `nodejs_output_xcheck.json` |
| `.github/workflows/nodejs.yml` | 2.4/3/5 | Node.js CI: `npm ci`, `npm run build`, `npm test` (123 tests) on matrix `ubuntu-latest`/`windows-latest`; CLI smoke-tests export-json and correction-hash; `compare` job diffs Linux vs Windows JSON output *and* correction-hash output |
| `tools/cross_check.py` | 3.CI/2.4/5 | Four-phase correctness checker: (1) schema validation × 3 fixtures × N langs; (2) read parity deep-diff; (3) write interop (Python/Rust/Node.js writer roundtrips); (4) correction hash parity — SHA-256 of flat little-endian f64 bytes, all 3 languages, all 3 fixtures × {forward, inverse}. `RUNNERS` + `WRITERS` + `BINARIES` dicts (`BINARIES` returns an argv *prefix* — `["node", ".../cli.js"]` for Node.js — so a bare string is no longer assumed); `PYTHONUTF8=1` in all subprocess envs; per-phase `--skip-*` flags. Adding a language = register in `RUNNERS`/`WRITERS`/`BINARIES` + add build steps to `cross_check.yml`. |
| `examples/quickstart/python/main.py` | 1.9 | Python quickstart — 6-step read+write roundtrip demo; resolves repo root via `__file__`; prints PASS/FAIL with field-level diagnostics |
| `rust/examples/quickstart.rs` | 3.QS | Rust quickstart — mirrors Python quickstart; run with `cargo run --example quickstart --manifest-path rust/Cargo.toml`; uses `CARGO_MANIFEST_DIR` to resolve repo root |
| `examples/quickstart/rust/main.rs` | 3.QS | Reference copy of Rust quickstart source for multi-language directory convention |

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

    def to_json(self, indent: int = 2, include_binary: bool = False) -> str:
        """Parse and serialize to canonical JSON string.

        By default (include_binary=False) correction arrays and raw .fc3 bytes
        are omitted — the JSON contains only scalar/metadata fields, keeping
        the output compact and human-readable.  Pass include_binary=True to
        add correction_data, inverse_correction_data, and raw_bytes (base64).
        """
        config = self.parse()
        return json.dumps(self._config_to_dict(config, include_binary=include_binary), indent=indent)

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

> **Coverage note**: All items on this checklist are asserted by name in `python/tests/test_reader.py` (as of Phase 1.7 prep). If the full suite passes, every line above is machine-verified before the golden file is generated. The `TestOpcua` class separately verifies the OPCUA fixture via `get_raw_group()` — OPCUA data does not appear in `reference_output.json` (it is out-of-scope for the canonical model **at Phase 1.7**) but its correctness is confirmed independently. After Phase 1.8e, OPCUA becomes an optional first-class field and `parse()` on `reference_config_opcua.h5` will populate `config.opcua`; `reference_output.json` is unaffected (generated from `reference_config.h5` which has no OPCUA group).

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

### 1.8 — Complete ClearBox Model and Production Write Capability

**Architecture note**: This library is machine-agnostic. ClearBox is an optional component — not every machine has one — but when present, it carries the same set of attributes across all machine types. The 17 currently-unread attributes listed below are universal ClearBox fields, not AconityMIDI-specific ones. The OPC UA connection parameters stored in ClearBox (`Data_Port`, `Server_Port`, etc.) are plain config data; the library stores and retrieves them as regular typed fields — no OPC UA client library is involved.

**Simplified contract**: The library's write path is JSON object → HDF5; the read path is HDF5 → JSON object. There are no file ingestion utilities — correction array data is provided directly in the JSON spec. The write API is `MachineConfigWriter.write(config)` where `config` is a fully-populated `MachineConfig`; there is no staged editor pattern.

**Why before Phase 2**: Phase 1.8a changes the canonical JSON output — `to_json()` gains 17 previously-unread ClearBox attributes, full correction array data (replacing shape-only metadata), and the Scanner axis subgroup structure. That changes the golden file SHA-256. Node.js is tested by diffing against `reference_output.json` — the cross-check target must be stable before Phase 2 begins. Phase 1.8d adds explicit bidirectional write verification — every language must be able to both read and write a machine config, and Python must demonstrate both paths before it serves as the cross-language reference oracle.

---

#### 1.8a — Complete the ClearBox Model (reader / writer / schema)

**What the HDF5 ClearBox group actually contains** (verified against `reference_config.h5`):

Two datasets — both currently unread beyond shape:
| Dataset | Shape | Notes |
|---|---|---|
| `Correction_Data` | (257, 257, 2) float64 | Forward correction; ~112K NaNs for out-of-field points |
| `Inverse_Correction_Data` | (257, 257, 2) float64 | Inverse correction (bit-space → build-space); ~112K NaNs |

17 attributes currently unread:

| Attribute | Type | Example value |
|---|---|---|
| `Actual_Timing_Offset` | int | -8 |
| `Commanded_Timing_Offset` | int | 50 |
| `Correction_Grid_Domain_Shape` | str | `''` (empty — not populated by machine) |
| `Inverse_Grid_Domain_Shape` | str | `''` (empty) |
| `Data_Port` | int | 5001 |
| `Server_Port` | int | 20101 |
| `Show_Console` | bool-as-int | 0 |
| `Software_Trigger_Delay` | int | 3000 |
| `Output_Path` | str | `'/recordings/'` |
| `Selected_Camera` | str | `'Default'` |
| `Custom_Video_Format` | str | `'MP4'` |
| `Video_Output` | str | `'HDMI'` |
| `Volts_To_Watts_Algorithm` | str | `'LINEAR'` |
| `Volts_To_Watts_Params` | str | `'48.5,105'` |
| `Manufacturer` | str | `''` |
| `Model` | str | `''` |
| `Serial_Number` | str | `''` |

**Null/NaN convention**: Out-of-field points in correction arrays are stored as IEEE 754 NaN in HDF5. In the canonical JSON representation, NaN is encoded as JSON `null` (`null` → `float('nan')` on read; `float('nan')` → `null` on write). Bare `NaN` is intentionally avoided — it is non-standard JSON and will fail to parse in all Phase 2+ language implementations.

**Scanner Axis_Configuration discriminator**: The Scanner group contains a variable number of subgroups depending on the `Axis_Configuration` attribute. This attribute is always present and its value determines the required subgroups:

| `Axis_Configuration` | Required subgroups under `Scanner` |
|---|---|
| `"2D"` | `X_Axis`, `Y_Axis` |
| `"3D"` | `X_Axis`, `Y_Axis`, `Z_Axis` |
| `"3D+Focus"` | `X_Axis`, `Y_Axis`, `Z_Axis`, `Focus` |

**Changes required:**

- `models.py`:
  - Expand `ClearBox` dataclass with all 17 attributes
  - Replace `correction_data_shape: tuple | None` with `correction_data: list[list[list[float | None]]] | None` — full (257,257,2) array; `None` cells represent out-of-field points (NaN in HDF5)
  - Add `inverse_correction_data: list[list[list[float | None]]] | None` — same shape and convention
  - Add `axis_configuration: str` to `Scanner` dataclass (enum: `"2D"`, `"3D"`, `"3D+Focus"`)
  - Add `z_axis: AxisConfig | None = None` and `focus: AxisConfig | None = None` to `Scanner`; add `__post_init__` validator asserting presence/absence of each is consistent with `axis_configuration`

- `reader.py`:
  - Read all 17 new ClearBox attributes
  - Read `Correction_Data` and `Inverse_Correction_Data` as full numpy arrays; convert `float('nan')` → `None` for the model/JSON representation
  - Read Scanner `Axis_Configuration` attribute; conditionally read `Z_Axis` and `Focus` subgroups based on value

- `writer.py`:
  - Write all 17 new ClearBox attributes
  - Write `correction_data` and `inverse_correction_data` arrays to HDF5; convert `None` → `float('nan')` before writing
  - Write Scanner subgroups based on `axis_configuration` value only (no extra groups written)

- `schema/machine_config_v1.schema.json`:
  - Add all 17 new ClearBox attribute fields as optional (nullable) properties
  - Add `correction_data` and `inverse_correction_data` as 3D array fields; items at the innermost level typed as `{"type": ["number", "null"]}` to allow finite floats and `null` (out-of-field) cells
  - Set `additionalProperties: false` on the `clearbox` definition
  - Leave `Optional_Components` schema open (no `additionalProperties` constraint)
  - Add `axis_configuration` as a required `enum` field on `scanner`; add `if/then/else` blocks enforcing which axis subgroups are required/forbidden for each value

- `python/tests/test_reader.py`:
  - Add assertions for all 17 new ClearBox attributes by name and value
  - Add assertion that correction array shape is `(257, 257, 2)` and that out-of-field positions are `None` in the model
  - Add assertion for Scanner `axis_configuration` value and presence/absence of `z_axis`/`focus` subgroups

- `tools/generate_fixtures.py` — re-run to regenerate golden file; SHA-256 committed. **Note**: the golden file will be significantly larger (~5–10 MB) due to full correction array data (four 257×257×2 arrays across two optical trains ≈ 528,392 values). Human review applies to scalar/metadata fields only — array correctness is verified by the roundtrip test in Phase 1.8d.

**Deliverable gate**: Re-run full suite (should be ~180+ passed), regenerate golden file, commit new SHA-256. Golden file is now the stable target for all subsequent cross-language work.

---

#### ~~1.8b — ConfigEditor Production Write Methods~~

> **Removed from scope.** Under the simplified JSON↔HDF5 contract, the write API is `MachineConfigWriter.write(config)` where `config` is a fully-populated `MachineConfig`. No staged editor pattern; no file-ingestion methods. The configuration hash is a plain field in the model.

---

#### ~~1.8c — ClearBox JSON Conversion Utility~~

> **Removed from scope.** The library does not ingest calibration files. Correction array data is provided directly as part of the `MachineConfig` JSON spec. Conversion from calibration software output to this library's format is an integration concern outside the library boundary.

---

#### 1.8d — Python Bidirectional Write Roundtrip Test

**Purpose**: Before Phase 2 begins, Python must be verified as a bidirectional reference implementation — both read path and write path proven correct. Every language in this library must be able to both read and write a machine config. Python is the oracle against which other languages' write output will be validated; its own write correctness must be established first.

**Design**: A new test file `python/tests/test_writer_roundtrip.py` with explicit field-level assertions (not just shape/structure, but actual values). This is distinct from the existing `MockConfigBuilder` roundtrip tests in `test_builder.py`, which exercise write→read at a coarse level but do not assert every individual field value.

**Tests**:
- `test_write_read_all_scalar_fields` — construct a fully-populated `MachineConfig` with known values for every scalar field; write via `MachineConfigWriter`; read back via `MachineConfigReader`; assert each field value matches the input
- `test_write_read_clearbox_all_attributes` — all 18 ClearBox attributes (post-1.8a complete model) round-trip correctly
- `test_write_read_correction_arrays_null_nan` — correction arrays containing `None` values write as HDF5 NaN and read back as `None`; non-`None` float values are bit-exact after roundtrip
- `test_write_read_scanner_axis_configurations` — configs with `axis_configuration` set to `"2D"`, `"3D"`, and `"3D+Focus"` each write the correct subgroups and read back with matching structure
- `test_write_read_without_clearbox` — a config with no ClearBox optional component round-trips correctly (validates machine-agnostic path: not all machines have ClearBox)
- `test_written_hdf5_validates_against_schema` — the HDF5 written by `MachineConfigWriter`, when read back as JSON, passes `machine_config_v1.schema.json` validation

**Semantic comparison note**: HDF5 files produced by different writers are not byte-for-byte identical (chunk layout, creation timestamps). The write roundtrip test compares field values after reading back through `MachineConfigReader`, not raw file bytes. This is the correct comparison mode for all cross-language write validation in subsequent phases.

**Deliverable gate**: All roundtrip tests pass in CI (`python.yml`). Python bidirectional capability confirmed. Phase 2 can begin.

> **Status: Complete (2026-07-24).** `python/tests/test_writer_roundtrip.py` created with 110 tests across 6 classes (`TestScalarFieldRoundtrip`, `TestClearBoxAttributeRoundtrip`, `TestCorrectionArrayNullNaN`, `TestScannerAxisConfigurations`, `TestWithoutClearBox`, `TestSchemaValidation`). All 281 tests pass (171 pre-1.8d + 110 new). Module-level fixtures used throughout (no deprecated class-instance-method fixture pattern). Both write paths confirmed: config-with-ClearBox and config-without-ClearBox both validate against the schema.

---

#### 1.8e — OPCUA as Optional First-Class Field

> **Status: Complete (2026-07-24). 308 tests pass (10 new in `test_opcua_roundtrip.py`, 11 new in `TestOpcuaModel` in `test_reader.py`, 7 new in `TestIncludeBinaryFlag` in `test_reader.py`). Golden file regenerated after `include_binary` flag introduced — `reference_output.json` reduced from ~14 MB to ~13 KB by excluding binary datasets from default `to_json()` output.**

**Background**: OPCUA connectivity configuration is currently accessible only via `reader.get_raw_group("OPCUA/Client")` etc. — a raw escape hatch with no model, no write path, and no schema coverage. As of this phase, OPCUA becomes a typed optional field on `MachineConfig` (`opcua: OpcuaConfig | None = None`), fully supported by reader, writer, JSON serialization, and schema validation.

**Design decisions** (confirmed with Tony, 2026-07-24):
- The three subgroups (`Client`, `Pipe`, `Triggers`) are stable across all machines — same groups, same field names, values vary per machine.
- `Client` and `Pipe` fields are fully typed and required when their parent group is present.
- Triggers are machine-specific in *name* and *count* but uniform in *field shape*. Known trigger fields (`ID`, `Signal`, `Subsystem`, `Rule_Enabled`, `Start_Value`, `Stop_Value`) are all optional in the evaluation — `null` does not fail a match. New machines discovered via OPCUA server discovery may introduce additional trigger fields; these are captured in an `extra: dict[str, str]` field on `OpcuaTrigger`.

**New dataclasses** (add to `models.py`):

```python
@dataclass
class OpcuaClientConfig:
    server_url: str
    auth_mode: str
    security_mode: str
    security_policy: str
    bfs_max_depth: int
    publish_interval: int
    sampling_interval: int
    session_timeout: int

@dataclass
class OpcuaPipeConfig:
    pipe_enabled: bool          # HDF5 int 0/1
    buffer_size: int

@dataclass
class OpcuaTrigger:
    id: Optional[str]
    signal: Optional[str]
    subsystem: Optional[str]
    rule_enabled: Optional[bool]  # HDF5 int 0/1
    start_value: Optional[str]
    stop_value: Optional[str]
    extra: dict[str, str]         # absorbs machine-specific discovery fields

@dataclass
class OpcuaConfig:
    client: OpcuaClientConfig
    pipe:   OpcuaPipeConfig
    triggers: dict[str, OpcuaTrigger]  # key = trigger name e.g. "Laser Emission Interlock"
```

Add to `MachineConfig`:
```python
opcua: Optional[OpcuaConfig] = None
```

**Changes required**:

| File | Change |
|---|---|
| `models.py` | Add `OpcuaClientConfig`, `OpcuaPipeConfig`, `OpcuaTrigger`, `OpcuaConfig`; add `opcua: Optional[OpcuaConfig] = None` to `MachineConfig` |
| `reader.py` | In `parse()`: if `OPCUA` group exists, parse `OPCUA/Client`, `OPCUA/Pipe`, and all subgroups of `OPCUA/Triggers` into the model. `extra` on each trigger captures any attributes not in the known field set. If `OPCUA` is absent, `opcua` is `None`. |
| `writer.py` | If `config.opcua is not None`: write `OPCUA/Client`, `OPCUA/Pipe`, `OPCUA/Triggers` groups with correct attribute types; for each trigger, write known fields then all `extra` key/value pairs. |
| `schema/machine_config_v1.schema.json` | Add optional top-level `opcua` property: `Client` and `Pipe` as typed objects with all fields required; `Triggers` as `additionalProperties` each conforming to a trigger schema with all known fields optional. |
| `__init__.py` | Export `OpcuaClientConfig`, `OpcuaPipeConfig`, `OpcuaTrigger`, `OpcuaConfig`. |
| `python/tests/test_reader.py` | Add assertions that `parse()` on `reference_config_opcua.h5` now populates `config.opcua` with correct typed values (complements existing `get_raw_group`-level `TestOpcua` class). |
| `python/tests/test_opcua_roundtrip.py` | New test file — see below. |

> **Note on golden file**: The canonical golden file is generated from `reference_config.h5`, which has no OPCUA group. `config.opcua` is `None` for that file and `to_json()` emits no `opcua` key. The golden file was regenerated after the `include_binary` flag was introduced — binary datasets (correction arrays, raw `.fc3` bytes) are excluded from the default `to_json()` output, so `reference_output.json` is a compact metadata-only snapshot (~13 KB). The SHA-256 reflects this content.

**`test_opcua_roundtrip.py` tests**:
- `test_opcua_client_fields_roundtrip` — construct `OpcuaConfig` with known `OpcuaClientConfig` values; write via `MachineConfigWriter`; read back via `MachineConfigReader.parse()`; assert each `client` field matches
- `test_opcua_pipe_fields_roundtrip` — same for `OpcuaPipeConfig`
- `test_opcua_trigger_known_fields_roundtrip` — trigger with all known optional fields populated; write and read back; assert all field values
- `test_opcua_trigger_extra_fields_roundtrip` — trigger with entries in `extra`; write and read back; assert `extra` dict survives the roundtrip
- `test_opcua_trigger_null_optional_fields` — trigger with only `signal` populated (all other known fields `None`); assert `None` fields are `None` after roundtrip (no injection of defaults)
- `test_opcua_multiple_triggers_roundtrip` — config with two triggers; assert both are present with correct names and values after roundtrip
- `test_opcua_absent_when_none` — config with `opcua=None` written and read back; assert `config.opcua is None` and no `OPCUA` group in the HDF5
- `test_opcua_parse_from_reference_opcua_fixture` — read `fixtures/reference_config_opcua.h5`; assert `config.opcua` is not `None` and all known field values match those already asserted in `TestOpcua` via `get_raw_group`
- `test_opcua_schema_valid_with_opcua` — written HDF5 with OPCUA reads back as schema-valid JSON
- `test_opcua_schema_valid_without_opcua` — written HDF5 without OPCUA reads back as schema-valid JSON

**Deliverable gate**: All tests pass. `parse()` on `reference_config_opcua.h5` returns a fully populated `OpcuaConfig`. `MachineConfigWriter` writes OPCUA groups that survive a full reader roundtrip. Schema validates both OPCUA-present and OPCUA-absent configs.

---

> **Phase 2 start condition**: Phase 1.8a complete (golden file regenerated, SHA-256 committed, CI green on updated golden file) AND Phase 1.8d complete (Python bidirectional write roundtrip verified in CI) AND Phase 1.8e complete (OPCUA first-class field implemented, reader/writer/schema updated, roundtrip tests passing).

---

## Phase 1.9 — Python Hello World

> **Status: Complete (2026-07-30).** `examples/quickstart/python/main.py` implemented and passing. 310 Python tests green.

**Goal**: Produce `examples/quickstart/python/main.py` — a standalone runnable program that demonstrates both the read and write paths of the Python library. This becomes the template all other language hello worlds follow.

**What it does** (same structure for every language):
1. Open `fixtures/reference_config.h5` via `MachineConfigReader`
2. Print: machine name, optical train count, working distance (train 0), correction data shape (train 0)
3. Write a copy to a temp file via `MachineConfigWriter`
4. Read the copy back via `MachineConfigReader`
5. Assert machine name, train count, and working distance match the original
6. Print `PASS` or `FAIL` with details

This is a self-contained read+write roundtrip. If it passes, the library's reader and writer are both functional from a consumer perspective.

**No new tests or CI changes** — the Python library is fully tested; this is documentation for consumers, not a test harness.

---

## Phase 3.10 — Rust Integration (clearbox-tauri)

**Goal**: Replace clearbox-tauri's existing machine config reader with `MachineConfigReader` from this library. Writer replacement is deferred to the Lossless Build Log phase.

**Branch**: new branch off clearbox-tauri `main` (work lives in the clearbox-tauri repo, not here).

**Root workspace** (required for git dep support):

A `Cargo.toml` at the repo root registers `rust/` as a Cargo workspace member so Cargo can locate the `machine-config` package when this repo is referenced as a git dependency. Without it, `cargo` cannot find the crate by name.

```toml
# Machine_Config_Library/Cargo.toml
[workspace]
resolver = "2"
members = ["rust"]
```

**Dependency** — three modes:

```toml
# 1. Local path dep (active co-development, no push required)
machine-config = { path = "../../Machine_Config_Library/rust" }

# 2. Git RC dep (integration-test a candidate before production)
machine-config = { git = "https://github.com/your-org/Machine_Config_Library", tag = "v0.2.0-rc.1" }

# 3. Git production dep (stable, pinned to a release tag)
machine-config = { git = "https://github.com/your-org/Machine_Config_Library", tag = "v0.2.0" }
```

When developing both projects simultaneously with a git tag in the dep declaration, add a `[patch]` override so local changes take effect without modifying the pinned version:

```toml
# clearbox-tauri/Cargo.toml — local development override (remove before merging)
[patch."https://github.com/your-org/Machine_Config_Library"]
machine-config = { path = "../../Machine_Config_Library/rust" }
```

**ndarray**: Resolved. This library was bumped to ndarray 0.17 to match `hdf5-metno 0.12`'s resolution in clearbox-tauri's workspace. Both projects now compile against a single ndarray 0.17.2 — no duplicate in the dependency tree. The public API (`CorrectionData`) carries only `Vec<f64>` and `[usize; 3]` across the crate boundary — no ndarray type is ever exposed — so future ndarray bumps in either project are independent.

**Completion criteria (branch merge conditions)**:
- Existing clearbox-tauri tests pass with the new reader
- `MachineConfigReader::open()` + `reader.parse()` replaces the existing reader at all call sites
- `CorrectionData` used wherever correction grids are accessed
- Writer not touched (existing clearbox-tauri writer remains)
- `cargo build --release` clean on Windows (the primary deployment target)

**No changes to this repo's CI** — this work lives entirely in clearbox-tauri.

---

## Phase 2 — Node.js

> **Status: Complete (2026-07-30).** All phases §2.0–§2.5 and all six §2.4 vertical-slice steps done, including the quickstart (Phase 2.4/6) and `builder.ts`'s `MockConfigBuilder` (not one of the six steps, but implemented alongside). 123 tests passing (78 reader + 6 schema + 21 writer + 18 builder). See Developer Quick Reference for current CLI surface and public API.

> **Implementation order**: Python (Phase 1) → Rust (Phase 3) → Python hello world (Phase 1.9) → clearbox-tauri integration (Phase 3.10) → Node.js scaffold (Phase 2.0) → Node.js implementation (Phase 2) → C++ (Phase 4) → Go (Phase 5). Phase numbering reflects original plan order; new phases inserted with decimal suffixes to avoid renumbering.

**Libraries**: `h5wasm` (libhdf5 compiled to WASM via Emscripten; zero native compilation), `vitest` (test runner), `ajv` (JSON Schema validation), `typescript`

> **Why `h5wasm` not a native binding**: `h5wasm` is the HDF5 **C library** compiled to WebAssembly by NIST — it is not a WASM build of this project's Rust library and shares none of its parsing logic. Cross-check independence is fully preserved: each language has its own layer-3 domain model (`reader.ts`, `models.ts`) reading the same HDF5 bytes through an independent execution path. The practical reason to prefer `h5wasm` over native N-API bindings (e.g. `node-hdf5`) is zero build friction: no node-gyp, no MSVC, no system HDF5 install required on Windows, Linux, or macOS — just `npm install`.

**Install**: `npm install machine-config-library`

**Start condition**: Phase 1.9 (Python hello world) and Phase 3.10 (clearbox-tauri integration) complete. `python.yml` and `rust.yml` CI green.

> **CI milestones**:
> - `.github/workflows/nodejs.yml` — **Complete (Phase 2.4/3, updated 2.4/5, 2026-07-30).** Runs on `ubuntu-latest` + `windows-latest`; `npm ci`, `npm run build`, `npm test` (123 tests); CLI smoke-tests for both `export-json` and `correction-hash`; `compare` job diffs Linux vs Windows JSON output *and* correction-hash output. `cross_check.yml` extended: **all four phases now include `nodejs`** — schema, read parity, write interop, and correction-hash are all verified live across `python,rust,nodejs`.
> - `cross_check.py` Phase 3 write interop refactored to data-driven — required before §2.4 step 4 (writer), not before the scaffold/models/reader. See §2.3. **Complete (2026-07-30)** — Node.js registered in `WRITERS`; 3×3 writer/reader parity + fidelity checks pass live.
> - `cross_check.yml` `--langs` extended to `python,rust,nodejs`; Node.js build steps added — after Node.js CLI (`export-json`) is working. **Complete.**

---

### 2.0 — Node.js Scaffold ✅ COMPLETE

Create the package structure and interface contract so the active Node.js consumer has a defined surface to build against. No HDF5 implementation yet — stubs only.

**Deliverables**:
- `nodejs/package.json` with `h5wasm`, `typescript`, `vitest`, `ajv`, `commander` dependencies declared
- `nodejs/tsconfig.json`
- `nodejs/src/models.ts` — TypeScript interfaces for all schema types (fully typed, not stubs)
- `nodejs/src/reader.ts` — `MachineConfigReader` class shell with method signatures
- `nodejs/src/writer.ts` — `MachineConfigWriter` class shell
- `nodejs/src/cli.ts` — `export-json`, `write-hdf5`, `correction-hash` subcommand stubs (same surface as Python/Rust CLIs)
- `RUNNERS["nodejs"]` and `BINARIES["nodejs"]` entries in `cross_check.py` — commented out, pointing to the built CLI

The consumer sees the full TypeScript interface immediately; the implementation fills in later.

### 2.1 — Package Structure ✅ COMPLETE

```
nodejs/
├── src/
│   ├── index.ts             ← exports MachineConfigReader, MachineConfigWriter, MockConfigBuilder
│   ├── models.ts            ← TypeScript interfaces matching the schema
│   ├── reader.ts            ← h5wasm-based reader
│   ├── writer.ts            ← h5wasm-based writer (inverse of reader; MachineConfig → HDF5)
│   ├── builder.ts           ← MockConfigBuilder using h5wasm write mode
│   ├── schema.ts            ← loads and validates via ajv
│   ├── cli.ts               ← export-json, write-hdf5, correction-hash subcommands
│   └── adapters/
│       ├── index.ts         ← exports getChain(); adapter registry
│       └── base.ts          ← Adapter interface: adapt(config: MachineConfig): MachineConfig
├── tests/
│   ├── reader.test.ts       ← vitest tests against synthetic_2laser.h5 + reference fixtures
│   ├── writer.test.ts       ← write roundtrip tests
│   ├── schema.test.ts       ← validates canonical JSON output
│   └── adapters.test.ts     ← adapter fixture tests (added when first adapter is written)
├── package.json
└── tsconfig.json
```

TypeScript interfaces mirror the Python dataclasses exactly, with the same field names. This ensures canonical JSON output is structurally identical across all implementations.

---

### 2.2 — Why Node.js Runs Against the Real Fixtures

`h5wasm` reads files directly from the host filesystem in Node.js via Emscripten NODERAWFS. The test suite runs against all three fixtures (`reference_config.h5`, `reference_config_opcua.h5`, `synthetic_2laser.h5`) matching the Rust integration test scope. The WASM startup cost is a one-time module load; per-file read performance is equivalent to native bindings for config-file-sized HDF5 (< 10 MB).

---

### 2.3 — cross_check Phase 3 Refactor ✅

> **Status: Complete.** `phase_write_interop()` refactored to a data-driven `WRITERS` dict loop. Python now writes via its CLI subprocess (no in-process imports in `cross_check.py`). Phase 3a (MockConfigBuilder 1-laser synthetic) dropped — its read-parity coverage is superseded by Phase 2 validating all active languages against `fixtures/synthetic_2laser.h5`. Phase 3 now runs: for each writer language → write HDF5 from canonical JSON → all reader languages verify parity + fidelity.

`phase_write_interop()` was refactored from three hardcoded Python↔Rust blocks to a data-driven `WRITERS` dict loop over all language pairs. Adding a new writer language now requires only a single entry in `WRITERS`.

**`copy-hdf5` subcommand (implemented):** Each language now exposes a `copy-hdf5` CLI subcommand (read HDF5 with full binary data → write HDF5, no JSON intermediate). This closes the binary round-trip testing gap: Phase 3 write-interop uses a JSON intermediate that excludes correction arrays, so correction-grid encoding was only tested by Phase 4 reading the original fixture. Phase 3.5 (`phase_binary_copy`) uses `copy-hdf5` to verify that each language can faithfully round-trip binary data end-to-end, and that all readers agree on the resulting correction hash.

---

### 2.4 — Implementation Order (vertical slice)

1. Models (`models.ts`) — TypeScript types matching the schema ✅
2. Reader (`reader.ts`) — h5wasm → models; add to cross_check Phase 1+2 immediately ✅
3. `nodejs.yml` CI — add when first test passes ✅
4. Writer (`writer.ts`) — models → h5wasm; add to cross_check Phase 3 ✅
5. `correction-hash` CLI subcommand — add to cross_check Phase 4 ✅
6. Hello world (`examples/quickstart/nodejs/main.mjs`) — capstone after all above complete ✅

> **Status (step 5): Complete (2026-07-30).** `MachineConfigReader.getCorrectionData(trainIndex)` / `getInverseCorrectionData(trainIndex)` added to `reader.ts` — return the raw NaN-preserving `Float64Array` + shape directly from the HDF5 dataset (bypassing the null-converted JSON path used by `parse()`), mirroring Python's and Rust's same-named reader methods. `cli.ts`'s `correction-hash <file> --train <n> [--inverse]` hashes those bytes as flat little-endian float64 via `node:crypto`'s `createHash('sha256')`, matching Python's `hashlib.sha256(arr.astype("<f8").tobytes())` and Rust's `Sha256::digest(&le_bytes)` byte-for-byte. `tools/cross_check.py`'s `BINARIES` dict changed from `Callable[[], str]` to `Callable[[], list[str]]` (an argv *prefix*) to accommodate Node.js's two-token invocation (`["node", ".../cli.js"]`); Python and Rust entries updated to match. Verified live: all 12 fixture × train × {forward, inverse} combinations produce byte-identical SHA-256 digests across Python, Rust, and Node.js. `cross_check.yml` and `nodejs.yml` updated accordingly (see their file-table rows below).

> **Status (step 6): Complete (2026-07-30).** `examples/quickstart/nodejs/main.mjs` implemented — a plain ESM script (not a `.ts` file compiled via `tsc`) mirroring `examples/quickstart/python/main.py` and `rust/examples/quickstart.rs` step-for-step: open the reference fixture, print machine name / optical train count / working distance / correction-grid shape, write to a temp file, read it back, and assert round-trip fidelity (machine name, train count, working distance) before printing `PASS`/`FAIL`. Imports the library from `nodejs/dist/index.js` (the compiled output, same as `cli.ts` does), so it has no build step of its own beyond `npm run build` inside `nodejs/` first — exits early with a clear message if that build output is missing. Verified working from both the repo root and from inside `nodejs/`. (Earlier drafts of this plan named the file `main.ts`; the shipped file is `main.mjs`, matching what `USAGE.md` had already documented as the run command.)

> **`builder.ts` — Complete (2026-07-30).** `MockConfigBuilder` is not one of the six vertical-slice steps above (it was never required to complete §2.4, since `fixtures/synthetic_2laser.h5` is already generated by the Python builder and consumed as-is by the Node.js test suite) but is now implemented anyway, mirroring `python/src/machine_config/builder.py` and `rust/src/builder.rs` field-for-field: same defaults (2 lasers, 250×250×20 mm build plate, `MockMachine`/`MockCo`/`MockMIDI+`/`MOCK-001`), same per-train geometry (alternating ±87.5/∓23.5 mm scan-head offsets, 0°/180° rotation), and the same Gaussian correction-grid formula (`2.0 * exp(-(x²+y²)/0.5)` over a `linspace(-1,1,257)` grid, peak 2.0 at centre; inverse grid = forward × 0.9, matching Rust's builder and Python's on-disk `save()` output — Python's *in-memory* `build()` uses a `×0.95` inverse factor that never survives its own `save()`, so the Node.js port intentionally follows the `×0.9` value both languages agree on after a round-trip). Departs from Python/Rust in two deliberate ways: (1) `machine.id` and `meta.export_date` are fixed constants (`00000000-0000-0000-0000-000000000001` / `2026-01-01T00:00:00.000Z`) rather than randomly/time generated, matching Rust's reproducibility convention rather than Python's; (2) the existing stub's two independent `includeClearbox`/`includeSfcf` options were collapsed into the single `includeClearbox` flag that Python and Rust both use to gate ClearBox and ScanFieldCorrectionFile together (nothing depended on the stub's two-flag runtime behavior, since it only ever threw). 18 new tests in `nodejs/tests/builder.test.ts` cover both `build()` (in-memory) and `save()` + read-back (full HDF5 round-trip): laser count, default/custom build-plate dimensions, offset/rotation alternation, ClearBox/SFCF presence and absence, correction-grid shape/centre-peak/non-zero, inverse-grid ratio, and schema validity.

---

### 2.5 — Tests

> **Status: Complete.** 78 fixture-based reader tests added to `nodejs/tests/reader.test.ts`, replacing the 4 scaffold smoke tests. Tests run against all 3 fixtures (`synthetic_2laser.h5`, `reference_config.h5`, `reference_config_opcua.h5`). Full suite: 123 tests passing (78 reader + 6 schema + 21 writer + 18 builder). Test groups: meta, machine geometry, optical trains, scanner, collimator, light_source, scanner_card, thermal lensing, ClearBox, ScanFieldCorrectionFile, correction data arrays (includeBinary:true), raw `getCorrectionData`/`getInverseCorrectionData` accessors (as of Phase 2.4/5), OPCUA client/pipe/triggers, JSON serialization, schema validation, writer roundtrip across all 3 fixtures including OPC-UA triggers (as of Phase 2.4/4), and `MockConfigBuilder` build/save/round-trip (`nodejs/tests/builder.test.ts`, new).

The Node.js test suite runs against all three canonical fixtures. Tests cover the same logical assertions as the Python/Rust suites so any structural gap between implementations surfaces immediately.

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

> **Status: Complete (2026-07-28).** All phases §3.1–§3.9 done + quickstart (Phase 3.QS, 2026-07-30). 60 tests passing (46 lib + 14 integration). See Developer Quick Reference for current CLI surface and public API.

**Libraries**: `hdf5-metno` (static; no system HDF5 required), `serde` / `serde_json`, `thiserror`, `indexmap`, `clap`, `ndarray`

**Install**: `cargo build` (first build compiles libhdf5 from source via `hdf5-metno --features static`; no system install needed on any platform)

**Start condition**: `fixtures/reference_output.json` committed and sha256 verified; `python.yml` CI green. Can run in parallel with Node.js. The cross-check becomes active as soon as Rust OR Node.js finishes — whichever lands first.

> **CI added at end of Phase 3**: `.github/workflows/rust.yml` added (matrix: `windows-latest`, `ubuntu-latest`); `cross_check.yml` `needs:` extended to include `rust`.

---

### 3.1 — Package Structure

> **Status: Complete.** All scaffold files created. No logic implemented yet — modules contain placeholder comments only.

```
rust/
├── src/
│   ├── lib.rs               ← pub mod declarations
│   ├── models.rs            ← #[derive(Serialize, Deserialize)] structs (Phase 3.4)
│   ├── reader.rs            ← hdf5::File → MachineConfig (Phase 3.5)
│   ├── writer.rs            ← MachineConfig → hdf5::File (Phase 3.6)
│   ├── builder.rs           ← MockConfigBuilder (Phase 3.7)
│   ├── error.rs             ← MachineConfigError enum (Phase 3.3)
│   ├── main.rs              ← CLI entry point (Phase 3.8)
│   └── adapters/
│       ├── mod.rs           ← pub mod registry
│       └── registry.rs      ← adapter registry (empty until first schema bump)
├── tests/
│   ├── integration_test.rs  ← tests against fixtures (Phase 3.9)
│   └── adapter_test.rs      ← adapter fixture tests (added when first adapter is written)
├── benches/
│   └── reader_bench.rs      ← criterion benchmark (Phase 3.9)
└── Cargo.toml
```

---

### 3.2 — Platform & Dependency Decision

**Decision: use `hdf5-metno` with the `static` feature.** This compiles and statically links libhdf5 from source during `cargo build`, eliminating all system HDF5 dependencies on every platform.

| Platform | Setup required |
|---|---|
| Windows | None — `cargo build` handles everything |
| Ubuntu/Debian | None — `cargo build` handles everything |
| macOS | None — `cargo build` handles everything |

The `Cargo.toml` entries:

```toml
[dependencies]
hdf5     = { package = "hdf5-metno",     version = "0.12" }
hdf5-sys = { package = "hdf5-metno-sys", version = "0.11", features = ["static", "zlib"] }
ndarray  = "0.16"   # must match hdf5-metno 0.12's internal ndarray; 0.17 causes type mismatch
sha2     = "0.10"   # for correction-hash CLI subcommand
```

> **ndarray version constraint**: `hdf5-metno 0.12` uses ndarray 0.16 internally. Using ndarray 0.17 in `Cargo.toml` causes a type mismatch at the `dataset.read::<f64, Ix3>()` boundary because two incompatible `Array3` types exist in the build. ndarray 0.16 is the correct version and must not be bumped until hdf5-metno is updated.

**Tradeoff**: first `cargo build` is slower (~2–3 min on a cold cache while libhdf5 compiles from source). Subsequent builds use the cached compiled artifact.

**CI matrix** — `rust.yml` targets both platforms from the start:

```yaml
strategy:
  matrix:
    os: [windows-latest, ubuntu-latest]
runs-on: ${{ matrix.os }}
```

No platform-specific setup steps are needed in CI for either runner.

---

### 3.3 — `error.rs` — Error Types

> **Status: Complete.** 6 inline unit tests passing (`cargo test --lib`).

Define the crate-wide error type and `Result` alias before any other module imports them.

```rust
// src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MachineConfigError {
    #[error("HDF5 error: {0}")]
    Hdf5(#[from] hdf5::Error),

    #[error("JSON serialisation error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Unit mismatch on attribute '{attr}': expected '{expected}', got '{actual}'")]
    UnitMismatch { attr: String, expected: String, actual: String },

    #[error("File version '{0}' is not supported by this reader (expected \"1.0\")")]
    UnsupportedVersion(String),

    #[error("Required HDF5 path missing: {0}")]
    MissingGroup(String),
}

pub type Result<T> = std::result::Result<T, MachineConfigError>;
```

All modules use `crate::error::Result<T>`. The `UnitMismatch` variant enforces Rule 8 (unit locking) from §0.5.

> **`hdf5` alias note**: `Cargo.toml` imports `hdf5-metno` under the name `hdf5` (`hdf5 = { package = "hdf5-metno", ... }`) so all crate code uses `hdf5::` — making the dependency a true drop-in and simplifying any future switch back to the upstream crate.

---

### 3.4 — `models.rs` — Data Models

All structs mirror Python `models.py`. Derive `Debug`, `Clone`, `PartialEq`, `serde::Serialize`, `serde::Deserialize` on every type (`PartialEq` added beyond the original plan for test-equality assertions; harmless since every field type already supports it).

**Key decisions:**
- `Option<T>` for every field that can be absent or stored as `""` in HDF5
- `IndexMap<String, serde_json::Value>` for all `.extra` dicts (preserves insertion order for JSON parity) — exposed as the `ExtraAttrs` type alias
- `IndexMap<String, OpcuaTrigger>` for `OpcuaConfig.triggers` (trigger key order must match HDF5 group enumeration order)
- `Scanner.axis_configuration` constraint (`"2D"` / `"3D"` / `"3D+Focus"`) is validated in the reader, not in the struct (Rust has no `__post_init__`; use a `validate()` method called from `parse()`)
- `ClearBox.correction_data` and `inverse_correction_data` are `Option<Vec<Vec<Vec<Option<f64>>>>>` (3-D nested, matching the `(257, 257, 2)` grid shape and Python's `list[list[list[float | None]]]`) when stored in the model for JSON serialisation; the raw `ndarray::Array3<f64>` form is only used in the reader/writer and is never part of the model struct
- `Machine` inlines the build-plate fields directly (`build_plate_x`, `build_plate_x_unit`, ... `build_plate_radius`, `build_plate_radius_unit`) instead of nesting a separate `BuildPlate` struct as Python's dataclass does — this matches the flat `machine` object in the canonical JSON schema (§0.2) and the field list in §3.10's attribute table, so `serde_json::to_string(&config)` on the struct tree produces schema-shaped JSON directly with no hand-written dict step (unlike Python's `_config_to_dict`). This is why `BuildPlate` does not appear in the struct inventory below.
- Fields that Python's `_config_to_dict` only adds to the output dict conditionally — `MachineConfig.opcua`, `ClearBox.correction_data`/`inverse_correction_data`, `ScanFieldCorrectionFile.raw_bytes` — use `#[serde(skip_serializing_if = "Option::is_none", default)]` so they are omitted from JSON entirely when `None`, rather than serialised as `null` like ordinary optional fields

**Struct inventory** (one-to-one with Python `models.py`, less `BuildPlate` — see above):
`ScanFieldCorrectionFile`, `ClearBox`, `Collimator`, `ScannerCard`, `AxisConfig`, `Scanner`, `LightSource`, `OpticalTrain`, `Machine`, `MachineConfigMeta`, `OpcuaClientConfig`, `OpcuaPipeConfig`, `OpcuaTrigger`, `OpcuaConfig`, `MachineConfig`

> **Status: Complete (2026-07-28).** `rust/src/models.rs` implemented with all 15 structs plus the `ExtraAttrs` type alias. 7 inline unit tests added (`cargo test --lib`) covering: full serde roundtrip equality, `opcua` omission when absent, `opcua` presence/order when set, conditional omission vs. inclusion of binary fields (`correction_data`, etc.), `extra`/`triggers` `IndexMap` insertion-order preservation through a JSON roundtrip, `configuration_hash` length, and `Option::None` → JSON `null` for ordinary optional fields. Full crate suite: **13 passed** (6 from `error.rs` + 7 new), `cargo build --all-targets` clean with zero warnings.

---

### 3.5 — `reader.rs` — HDF5 Reader

Public API mirrors Python `MachineConfigReader`:

```rust
pub struct MachineConfigReader {
    path: PathBuf,
}

impl MachineConfigReader {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self>;
    pub fn parse(&self) -> Result<MachineConfig>;              // scalars only
    pub fn parse_with_binary(&self) -> Result<MachineConfig>;  // includes correction_data + raw_bytes
    pub fn get_correction_data(&self, train_index: usize) -> Result<ndarray::Array3<f64>>;
    pub fn get_inverse_correction_data(&self, train_index: usize) -> Result<ndarray::Array3<f64>>;
    pub fn get_scan_field_correction_bytes(&self, train_index: usize) -> Result<Vec<u8>>;
    pub fn to_json(&self, indent: bool, include_binary: bool) -> Result<String>;
}
```

Implementation order within this phase:
1. Attribute-reading helpers (`read_str`, `read_f64`, `read_i64`, `read_bool_from_int`, `read_bool_from_f64`, `read_unit_locked`, `collect_extra`) — apply all rules from §3.11
2. Root attrs → `MachineConfigMeta`
3. `Machine` group → `Machine`
4. Per-train parsing (Scanner + axes, LightSource, Collimator, ScannerCard, ClearBox, SFCF)
5. OPCUA group (optional)
6. `to_json()` using `serde_json::to_string_pretty`

See §3.10 for the complete HDF5 path and attribute name mapping table.

> **Status: Complete (2026-07-28).** `rust/src/reader.rs` implemented per the public API above, plus `get_raw_group(&self, hdf5_path: &str) -> Result<ExtraAttrs>` (Rule 5 accessor for non-schema groups like `OPCUA` or `Scanner/X_Axis`, matching Python's method of the same name — not shown in the original snippet above but required by §0.5).
>
> **Key implementation finding**: real HDF5 attributes do not have a fixed HDF5 type per field the way the model's field types might suggest. A quick diagnostic against the actual fixtures (via a throwaway `cargo run --example`, since removed) showed that optional numeric fields such as `Range_Of_Motion` are stored as `Float(U8)` when populated but as an **empty `VarLenUnicode` string** when not — the machine-export software writes `""` for an unset numeric attribute rather than omitting it, exactly as Rule 3 (§0.5) describes. Every attribute-reading helper (`read_str`, `read_float`, `read_int`, `read_bool_from_int`, `read_bool_from_float`, `read_unit_locked`) therefore dispatches on the attribute's *actual* runtime `TypeDescriptor` (`VarLenUnicode`/`VarLenAscii` → string, `Integer`/`Unsigned` → `i64`, `Float` → `f64`) rather than assuming a fixed type per field — this is what makes a single generic helper correctly handle both `Range_Of_Motion` (float-or-empty-string) and `Power_Bit_Resolution` (always a numeric string) with the same code path, mirroring Python's duck-typed `float(val)` exactly.
>
> **Deliberate divergence from Python**: Python's single `parse()` always reads the ClearBox `(257, 257, 2)` float64 correction grids and the raw `.fc3` bytes into the model; `include_binary` only gates whether `to_json()` serialises them. The Rust reader instead splits this at parse time: `parse()` leaves `correction_data`/`inverse_correction_data`/`raw_bytes` as `None` (skipping those reads entirely), and `parse_with_binary()` populates them — `to_json(pretty, include_binary)` picks whichever is needed. This avoids the cost of reading multi-megabyte payloads for callers who only want scalar/metadata fields, while `get_correction_data()`/`get_inverse_correction_data()`/`get_scan_field_correction_bytes()` remain available as direct, model-independent accessors exactly as in Python.
>
> **Verification**: 15 new inline unit tests (`cargo test --lib`, 28 total in the crate) against the real fixtures (`reference_config.h5`, `reference_config_opcua.h5`, `synthetic_2laser.h5`), covering meta/machine/optical-train scalars, the `Range_Of_Motion` empty-string case, ClearBox/SFCF metadata-without-binary vs. `parse_with_binary()`, `get_correction_data` shape and NaN presence, `get_scan_field_correction_bytes` length, `get_raw_group` on both a missing path and `OPCUA/Client`, and OPCUA client/pipe/trigger parsing including `extra` passthrough (e.g. `Trigger_Label`). The strongest check is `matches_python_golden_file`: it deep-compares `MachineConfigReader::open(REFERENCE).to_json(true, false)` against `fixtures/reference_output.json` as parsed `serde_json::Value` trees — **they are identical**, i.e. the Rust reader already produces byte-for-value-equivalent output to the Python reference implementation for the full real AconityMIDI fixture, ahead of Phase 5's `cross_check.py` formalising this. `cargo build --all-targets` is clean with zero warnings.

---

### 3.6 — `writer.rs` — HDF5 Writer

> **Status: Complete (2026-07-28).** 11 inline unit tests passing. Full crate suite: **40 passed** (`cargo test --lib`), `cargo build --all-targets` clean with zero warnings.

Public API mirrors Python `MachineConfigWriter`:

```rust
pub struct MachineConfigWriter<'a> {
    config: &'a MachineConfig,
}

impl<'a> MachineConfigWriter<'a> {
    pub fn new(config: &'a MachineConfig) -> Self;
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()>;
}
```

The writer is the exact inverse of the reader. Applies all write rules from §3.12 (`None` → `""`, bool → 0/1 int, `triggers_enabled` → f64, correction arrays → zero-filled when absent, `extra` dict passthrough). Writer roundtrip tests in §3.9 verify SHA-256 of correction grids is unchanged across a write-then-read cycle.

**Key implementation note**: `VarLenUnicode` in `hdf5-metno` implements `FromStr`, not `From<&str>` / `From<String>`. All string-attribute helpers therefore use `.parse().map_err(|e| MachineConfigError::Parse(...))?` rather than `.into()`. This is the correct pattern for all string writes throughout the crate.

---

### 3.7 — `builder.rs` — MockConfigBuilder

> **Status: Complete (2026-07-28).** 6 inline unit tests passing. Full crate suite: **46 passed** (`cargo test --lib`), `cargo build --all-targets` clean with zero warnings.

```rust
pub struct MockConfigBuilder {
    pub laser_count: usize,
    pub build_plate_x: f64,
    pub build_plate_y: f64,
    pub build_plate_z: f64,
    pub include_clearbox: bool,
    pub machine_name: String,
    // manufacturer, model, serial_number ...
}

impl MockConfigBuilder {
    pub fn new(laser_count: usize) -> Self;
    pub fn build(&self) -> MachineConfig;
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()>;
}
```

Writes the same HDF5 group/attribute structure as Python's `MockConfigBuilder` so both implementations can round-trip `fixtures/synthetic_2laser.h5`. Correction grids are non-zero Gaussian warps (`gaussian_correction_grid()`, shape `(257, 257, 2)`). Does not produce identical float values to Python — just structurally valid and non-trivial. The `include_clearbox: false` path produces trains without `Optional_Components/ClearBox` or `scan_field_correction_file`.

---

### 3.8 — CLI (`main.rs`)

> **Status: Complete (2026-07-28).** `cargo build --all-targets` clean; all 46 lib tests still pass. Manual smoke test confirmed: `machine-config-cli export-json fixtures/reference_config.h5` outputs well-formed JSON to stdout.

Single subcommand using `clap` derive:

```
machine-config-cli export-json <path> [--include-binary]
```

Outputs pretty-printed JSON to stdout; exits 0 on success, 1 on any error (error message to stderr). This is the interface `cross_check.py` calls in Phase 5.

```rust
use clap::{Parser, Subcommand};
use machine_config::reader::MachineConfigReader;

#[derive(Parser)]
#[command(name = "machine-config-cli", about = "...", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    ExportJson {
        path: std::path::PathBuf,
        #[arg(long)]
        include_binary: bool,
    },
}
```

---

### 3.9 — Integration Tests

> **Status: Complete (2026-07-28).** 14 integration tests passing (`cargo test`). Full suite: **46 lib + 14 integration = 60 tests**, all green. Criterion benchmark stubs in `benches/reader_bench.rs` build and `cargo bench` runs clean (3 benchmarks: `open_and_parse`, `to_json_pretty`, `open_and_parse_with_binary`).

Fixtures used:

| Constant | File | Notes |
|---|---|---|
| `FIXTURE` | `fixtures/synthetic_2laser.h5` | Python `MockConfigBuilder` output, no OPCUA |
| `REFERENCE` | `fixtures/reference_config.h5` | Real AconityMIDI, no OPCUA |
| `REFERENCE_OPCUA` | `fixtures/reference_config_opcua.h5` | Real AconityMIDI, with OPCUA |

Tests implemented:

| Test | Fixture | What it checks |
|---|---|---|
| `test_machine_name` | FIXTURE | `meta.machine_name` non-empty |
| `test_optical_train_count` | FIXTURE | 2 optical trains |
| `test_working_distance` | FIXTURE | WD = 670.0 mm on train 0 |
| `test_json_output_validates_schema` | FIXTURE | JSON has `meta`/`machine`/`optical_trains`; `opcua` absent |
| `test_configuration_hash_length` | FIXTURE | `configuration_hash` is 64 chars |
| `test_meta_extra_preserved` | FIXTURE | `meta.extra` accessible without panic |
| `test_correction_data_shape` | FIXTURE | `get_correction_data(0)` shape = `[257, 257, 2]` |
| `test_correction_data_is_nonzero` | FIXTURE | at least one non-zero cell |
| `test_nan_to_null_in_correction_data` | REFERENCE | real fixture has NaN border cells → `None` in nested Vec |
| `test_builder_roundtrip` | temp | builder writes 2-laser config, reads back |
| `test_opcua_client_fields` | REFERENCE_OPCUA | `bfs_max_depth=16`, `session_timeout=60000` |
| `test_opcua_triggers_parsed` | REFERENCE_OPCUA | `triggers_enabled=Some(true)`, key `"Laser Emission Interlock"` present |
| `test_writer_roundtrip_scalars` | FIXTURE | `machine_name` and `working_distance` survive write→read |
| `test_writer_roundtrip_correction_data_checksum` | FIXTURE | `DefaultHasher` digest of correction grid unchanged |

Note: `test_writer_roundtrip_correction_data_checksum` intentionally uses the `DefaultHasher` approach (matching the writer’s own roundtrip tests) rather than adding `sha2` + `bytemuck` dev-dependencies — the goal is bit-for-bit identity, not a cryptographic guarantee.

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

#[test]
fn test_opcua_client_fields() {
    let config = MachineConfigReader::open(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config_opcua.h5")
    ).unwrap().parse().unwrap();
    let client = config.opcua.unwrap().client;
    assert!(!client.server_url.is_empty());
    assert_eq!(client.bfs_max_depth, 16);
    assert_eq!(client.session_timeout, 60000);
}

#[test]
fn test_opcua_triggers_parsed() {
    let config = MachineConfigReader::open(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config_opcua.h5")
    ).unwrap().parse().unwrap();
    let opcua = config.opcua.unwrap();
    assert_eq!(opcua.triggers_enabled, Some(true));
    assert!(opcua.triggers.contains_key("Laser Emission Interlock"));
}

#[test]
fn test_meta_extra_preserved() {
    // Any HDF5 root attributes beyond the known set must land in meta.extra
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    // extra may be empty for synthetic fixture — just assert it doesn't panic
    let _ = &config.meta.extra;
}

#[test]
fn test_nan_to_null_in_correction_data() {
    // NaN values in the correction grid must round-trip as JSON null
    let config = MachineConfigReader::open(FIXTURE).unwrap().parse_with_binary().unwrap();
    let cb = config.optical_trains[0].clearbox.as_ref().unwrap();
    let data = cb.correction_data.as_ref().unwrap();
    // The synthetic fixture has NaN border cells — at least one null must be present
    assert!(data.iter().flatten().flatten().any(|v| v.is_none()));
}

#[test]
fn test_writer_roundtrip_scalars() {
    use machine_config::writer::MachineConfigWriter;
    use tempfile::NamedTempFile;
    let original = MachineConfigReader::open(FIXTURE).unwrap().parse().unwrap();
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&original).write(tmp.path()).unwrap();
    let roundtripped = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();
    assert_eq!(original.meta.machine_name, roundtripped.meta.machine_name);
    assert_eq!(
        original.optical_trains[0].scanner.working_distance,
        roundtripped.optical_trains[0].scanner.working_distance,
    );
}

#[test]
fn test_writer_roundtrip_correction_data_checksum() {
    use machine_config::writer::MachineConfigWriter;
    use sha2::{Digest, Sha256};
    use tempfile::NamedTempFile;
    let reader = MachineConfigReader::open(FIXTURE).unwrap();
    let original = reader.get_correction_data(0).unwrap();
    let config = reader.parse().unwrap();
    let tmp = NamedTempFile::with_suffix(".h5").unwrap();
    MachineConfigWriter::new(&config).write(tmp.path()).unwrap();
    let roundtripped = MachineConfigReader::open(tmp.path()).unwrap().get_correction_data(0).unwrap();
    let h1 = Sha256::digest(bytemuck::cast_slice(&original));
    let h2 = Sha256::digest(bytemuck::cast_slice(&roundtripped));
    assert_eq!(h1, h2, "Correction data SHA-256 changed across write roundtrip");
}
```

---

### 3.10 — HDF5 Group & Attribute Path Reference

The HDF5 file has a fixed structure. The Rust reader must traverse these exact paths. Attribute names are PascalCase/Snake_Case in the HDF5 file; the canonical JSON and Rust struct fields use `snake_case`.

**Root attributes → `MachineConfigMeta`**

| HDF5 attr | Rust field | Type |
|---|---|---|
| `machine_name` | `meta.machine_name` | `String` |
| `manufacturer` | `meta.manufacturer` | `String` |
| `model` | `meta.model` | `String` |
| `serial_number` | `meta.serial_number` | `String` |
| `File_Version` | `meta.file_version` | `String` |
| `Export_Date` | `meta.export_date` | `String` |
| `Configuration_Hash` | `meta.configuration_hash` | `String` (len=64) |
| *(any other)* | `meta.extra` | `HashMap<String, JsonValue>` |

**`Machine` group → `Machine`**

| HDF5 attr | Rust field | Notes |
|---|---|---|
| `ID` | `machine.id` | `Option<String>` |
| `Machine_Name` | `machine.machine_name` | `String` |
| `Manufacturer` | `machine.manufacturer` | `String` |
| `Model` | `machine.model` | `String` |
| `Serial_Number` | `machine.serial_number` | `String` |
| `Build_Plate_X_Dimension` | `machine.build_plate_x` | `Option<f64>` |
| `Build_Plate_X_Dimension_unit` | `machine.build_plate_x_unit` | locked `"mm"` |
| `Build_Plate_Y_Dimension` | `machine.build_plate_y` | `Option<f64>` |
| `Build_Plate_Y_Dimension_unit` | `machine.build_plate_y_unit` | locked `"mm"` |
| `Build_Plate_Z_Dimension` | `machine.build_plate_z` | `Option<f64>` |
| `Build_Plate_Z_Dimension_unit` | `machine.build_plate_z_unit` | locked `"mm"` |
| `Build_Plate_Corner_Radius` | `machine.build_plate_radius` | `Option<f64>` |
| `Build_Plate_Corner_Radius_unit` | `machine.build_plate_radius_unit` | locked `"mm"` |
| `Gas_Flow_Direction` | `machine.gas_flow_direction` | `Option<String>` |
| `Recoat_Direction` | `machine.recoat_direction` | `Option<String>` |

**Optical train group path**: `Machine/Optical_Trains/Optical_Train_NN/` (NN is zero-padded 2-digit index, 1-based; groups are sorted lexicographically to determine order)

**Scanner attrs** (group `…/Scanner/`)

| HDF5 attr | JSON key | Notes |
|---|---|---|
| `Manufacturer` | `scanner.manufacturer` | |
| `Model` | `scanner.model` | |
| `Serial_Number` | `scanner.serial_number` | |
| `Working_Distance` | `scanner.working_distance` | locked unit `"mm"` |
| `Scan_Field_Size_X` | `scanner.scan_field_x` | locked unit `"mm"` |
| `Scan_Field_Size_Y` | `scanner.scan_field_y` | locked unit `"mm"` |
| `Scan_Field_Size_Z` | `scanner.scan_field_z` | locked unit `"mm"` |
| `Scan_Head_Offset_X` | `scanner.scan_head_offset_x` | locked unit `"mm"` |
| `Scan_Head_Offset_Y` | `scanner.scan_head_offset_y` | locked unit `"mm"` |
| `Scan_Head_Offset_Z` | `scanner.scan_head_offset_z` | locked unit `"mm"` |
| `Scan_Head_Rotation` | `scanner.scan_head_rotation` | locked unit `"degrees"` |
| `Axis_Configuration` | `scanner.axis_configuration` | `"2D"` / `"3D"` / `"3D+Focus"` |

Scanner axis sub-groups: `X_Axis/`, `Y_Axis/`, `Z_Axis/` (only if `axis_configuration` ≠ `"2D"`), `Focus/` (only if `"3D+Focus"`). Each has the same 11 attrs (`Actual_Bit_Resolution`, `Commanded_Bit_Resolution`, `Control_Type`, `Range_Of_Motion`, `Smoothing_Kernel`, `Smoothing_Parameters`, `Tuning_Parameters`, `Tuning_Type` plus unit variants).

**LightSource attrs** — `Light_Wavelength` (unit locked `"nm"`), `Power_Max_Nominal`/`Power_Max_Actual`/`Power_Min_Nominal`/`Power_Min_Actual` (unit locked `"W"`), `Power_Bit_Resolution` (unit locked `"bits"` — **stored as HDF5 string, not number**; must be parsed with `str::parse::<f64>()`), `Watts_To_Volts_Algorithm`, `Watts_To_Volts_Params`.

**ClearBox** — group path: `…/Optional_Components/ClearBox/`. May be absent. Attrs include `Ip_Address`, `Data_Port`, `Server_Port`, `Actual_Timing_Offset`, `Commanded_Timing_Offset`, `Show_Console` (int 0/1), and all the string fields. Datasets: `Correction_Data` and `Inverse_Correction_Data` (both `(257, 257, 2)` float64; out-of-field cells are IEEE 754 NaN → must be serialised as JSON `null`).

**scan_field_correction_file** — this is an **HDF5 Dataset**, not a Group. Path: `…/scan_field_correction_file`. The dataset payload is `uint8[]` (raw `.fc3` bytes). Metadata lives in dataset attrs: `document_name`, `document_id`, `file_size` (int), `valid_as_of_date`, `document_created_at`, `document_type`, `original_uri`.

**OPCUA** — group `OPCUA/` is optional (absent in most configs). Sub-groups: `Client/`, `Pipe/`, `Triggers/`. `Triggers/` contains one sub-group per trigger (key is the trigger label string). `Triggers/` group itself has an attr `Triggers_Enabled` stored as **float64** `0.0`/`1.0` (not int).

---

### 3.11 — Type Coercion Rules (Rust equivalents of Python reader helpers)

All of these quirks exist in the real HDF5 files and must be handled identically in Rust:

| Rule | Python behaviour | Rust equivalent |
|---|---|---|
| Absent optional attr | `None` | `Option::None` |
| Empty-string optional attr | Returns `None` (stripped) | `if s.trim().is_empty() { None } else { Some(s) }` |
| `bool` from int attr | `0` → `false`, `1` → `true`, other → `Err` | Same; reject non-0/1 |
| `bool` from float attr | `triggers_enabled` only: `0.0` → `false`, `1.0` → `true` | Read as `f64`, cast to `bool` |
| NaN in float64 dataset | Represented as `None` in JSON (list elements) | `if v.is_nan() { None } else { Some(v) }` |
| `power_bit_resolution` | HDF5 stores as string; Python does `float(val)` | Read as `String`, then `s.trim().parse::<f64>()` |
| `None` written as `""` | Writer outputs empty string for absent optional | `let s = v.as_deref().unwrap_or("");` |
| `bool` written as int | Writer outputs `0`/`1` | `hdf5_attr = if b { 1i32 } else { 0i32 }` |
| `triggers_enabled` written as float | Writer outputs `np.float64(0.0)` or `np.float64(1.0)` | Write as `f64` |
| Unit attrs | Read and validated against locked expected value | Return `Err` if present but wrong; `None` if absent |
| `extra` dicts | All attrs not in known-key set → `HashMap` | Collect into `HashMap<String, serde_json::Value>` |
| File version | `!= "1.0"` → `warn!()`, not `Err` | `tracing::warn!` or `eprintln!` |

---

### 3.12 — Writer Reference

The Rust writer is the exact inverse of the reader. It must produce an HDF5 file that, when read back, yields an identical `MachineConfig`. The Python writer is the reference.

**Public API:**

```rust
pub struct MachineConfigWriter<'a> {
    config: &'a MachineConfig,
}

impl<'a> MachineConfigWriter<'a> {
    pub fn new(config: &'a MachineConfig) -> Self { ... }
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> { ... }
}
```

**Key writer rules (mirror the Python helpers):**

- `Option<String>` → `""` when `None` (not omit; the attr must be written)
- `Option<f64>` → `""` when `None` (h5py compat; downstream readers handle empty string)
- `Option<i32>` → `""` when `None`
- `Option<bool>` → `""` when `None`; `true` → `1i32`, `false` → `0i32`
- Correction arrays: `Option<()>` absent → write zero-filled `(257, 257, 2)` float64 dataset. Dataset must carry attrs `dimensions = "H,W,D"`, `dtype = "float64"`, `shape = "257x257x2"`.
- `scan_field_correction_file`: if `raw_bytes` is `None`, write `vec![0u8; max(file_size, 1)]`
- `meta.extra`, `opcua.client.extra`, `opcua.pipe.extra`, `trigger.extra` → write each key/value directly as HDF5 attrs on the appropriate group
- `opcua.triggers_enabled` → write as `f64` on the `OPCUA/Triggers` group attrs
- All groups must be created with `require_group` semantics (create if absent, reuse if present)

---

## Phase 4 — C++

> **Detail deferred.** This section will be fully fleshed out after Node.js (Phase 2) is complete and the Phase 3 write interop refactor is proven. The patterns established in Node.js CI (libhdf5 on Windows, data-driven Phase 3) directly inform the C++ implementation.

**Library choices** (decided now, not subject to change):
- **HDF5 wrapper**: HighFive (header-only C++ wrapper over the HDF5 C library; CMake `FetchContent`)
- **JSON**: nlohmann/json (header-only; `FetchContent`)
- **Build**: CMake 3.20+
- **Argument parsing**: CLI11 (header-only)
- **Tests**: Catch2 (header-only via `FetchContent`)

**Implementation order** (same vertical slice as all other languages):
1. Models (`models.hpp`) — POD structs matching the schema
2. Reader — add to cross_check Phase 1+2 immediately
3. `cpp.yml` CI — add when first test passes (matrix: Ubuntu + Windows via vcpkg)
4. Writer — add to cross_check Phase 3
5. `correction-hash` CLI — add to cross_check Phase 4
6. `copy-hdf5` CLI — add to `COPIERS` dict in cross_check for Phase 3.5
7. Hello world (`examples/quickstart/cpp/main.cpp`) — capstone

**Windows CI approach**: vcpkg with `hdf5` port. The experience from Node.js CI will clarify whether this is sufficient or whether a pre-built static HDF5 is needed.

> Full directory structure, test patterns, and cross_check integration details to be added when Phase 2 is complete.

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

> **§4.1–§4.4 (directory structure, CMakeLists.txt, test patterns, cross_check integration) — deferred until Phase 2 (Node.js) is complete.** See Phase 4 intro above for library choices and implementation order.

---

## Phase 5 — Go

> **Detail deferred.** This section will be fully fleshed out after C++ (Phase 4) is complete. Go is last because CGO + Windows CI is the most complex dependency setup of any language in this project.

**Library choices** (decided):
- **HDF5 wrapper**: `gonum/hdf5` (CGO wrapper around the HDF5 C library)
- **CLI**: `cobra`
- **Tests**: standard `go test`
- **JSON Schema**: `github.com/santhosh-tekuri/jsonschema/v6`

**Implementation order**: same vertical slice as all other languages (models → reader → CI → writer → correction-hash → copy-hdf5 → hello world).

**Windows CI**: deferred. CGO on Windows requires MinGW-w64 + libhdf5, which is non-trivial. Ubuntu CI is added first; Windows CI added after the pattern is validated.

**Prerequisite**: Phase 4 (C++) complete and cross-check CI green for all implemented languages.

> Full directory structure, test patterns, and cross_check integration details to be added when Phase 4 (C++) is complete.

---

## Phase 6 — Cross-Check Tooling

### 6.1 — `tools/cross_check.py`

> **Status: Implemented (2026-07-28).** Four-phase script, all phases green locally and in CI (Python ↔ Rust on Ubuntu and Windows). See Developer Quick Reference for current phase descriptions and CLI flags.

The script runs in four phases:

| Phase | What it checks | Pass condition |
|---|---|---|
| 1 Schema validation | Every language × every fixture validates against `schema/machine_config_v1.schema.json` | `jsonschema.validate()` passes for all combinations |
| 2 Read parity | All languages produce identical JSON for all three fixtures | `DeepDiff` (significant_digits=8) returns no diff for each pair |
| 3 Write interop | Python builder/writer → Rust reader; Rust writer → Python + Rust readers (parity + fidelity) | All readers agree on all written files |
| 4 Correction hash | SHA-256 of flat little-endian f64 correction bytes matches across all language pairs, all fixtures, forward + inverse | All language pairs produce identical hex digests |

Design principles:
- **Extensible**: add a language by adding entries to `RUNNERS` and `BINARIES` — no other changes needed to phase logic
- **Windows-safe**: `PYTHONUTF8=1` in all subprocess envs; `shell: bash` on steps using bash syntax in CI
- **Per-phase skip flags**: `--skip-schema`, `--skip-read-parity`, `--skip-write-interop`, `--skip-correction-hash`
- **Subset mode**: `--langs python,rust` to check only available languages during development

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

### 5.4 — Viewer JSON-Load Extension

The existing `viewer/machine_config_viewer.html` reads HDF5 directly via the `h5wasm` CDN. A small addition — a **"Load JSON" button** alongside the current "Load H5" button — lets the viewer accept the canonical JSON produced by any language library, making it a lightweight visual diff tool.

**Scope:** Add a second file-input path to the viewer that accepts a JSON file, parses it using the library's canonical key names (e.g. `scanner.scan_head_offset_x`, `scanner.scan_field_x/y`), and renders the same canvas (build plate, scanner field rectangles, direction arrows). No library dependency is introduced — the viewer stays a standalone HTML file.

**Why this is useful for cross-language validation:**
- Run `python tools/cross_check.py --emit-json python,nodejs`, drop both JSON files into the viewer in two tabs, and immediately see if scanner field placement differs visually.
- A numeric field mismatch (e.g. sign flip on `scan_head_offset_x`) is immediately obvious as a scanner field on the wrong side of the build plate; the CLI diff only reports a number.

**Relationship to `cross_check.py`:** The viewer JSON-load is a developer convenience tool, not a CI gate. `cross_check.py` is still the authoritative correctness check. The viewer surfaces spatial/geometric disagreements that numbers alone don't communicate.

**Timing:** Implement after at least one non-Python language is CI-green, so there are two JSON outputs to compare. The viewer HTML edit itself is ~1 day of work.

---

## Phase 7 — Examples

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

EXPECTED_LINES = ["Machine name", "Optical trains", "Working dist", "Correction grid"]

def _assert_quickstart_output(out: str) -> None:
    for line in EXPECTED_LINES:
        assert line in out, f"Expected {line!r} in output:\n{out}"

def test_python_quickstart():
    _assert_quickstart_output(_run(["python", "examples/quickstart/python/main.py"]))

def test_nodejs_quickstart():
    _assert_quickstart_output(_run(["node", "examples/quickstart/nodejs/main.mjs"]))

def test_rust_quickstart():
    # Rust quickstart is a Cargo example; assumes release binary has been pre-built
    _assert_quickstart_output(_run(["rust/target/release/examples/quickstart"]))

def test_cpp_quickstart():
    _assert_quickstart_output(_run(["examples/quickstart/cpp/build/quickstart"]))
```

The smoke tests do not validate exact field values — that is the responsibility of the per-language unit tests and the cross-check. They only confirm that each quickstart binary runs without error and emits the expected output shape.

---

## Phase 8 — Viewer Enhancement

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

## Phase 9 — Distribution & Packaging

| Language | Registry | Package Name | Notes |
|---|---|---|---|
| Python | PyPI | `machine-config-library` | `pyproject.toml` with `[project.scripts]` entry point for the CLI |
| Node.js | npm | `machine-config-library` | ESM + CJS dual output via `tsup`; same bundle works in browser |
| Rust | crates.io | `machine-config` | Feature flag `cli` gates the binary; `default-features = false` for lib-only use |
| C++ | vcpkg | `machine-config` | Port overlay in `cpp/vcpkg-port/`; also supports CMake `FetchContent` directly |

---

### Versioning Policy

All four packages share the **same semver version number** at all times. A `v0.3.0` tag means the Python, Rust, Node.js, and C++ libraries all implement the same schema revision at that point.

#### Tooling

| Tool | Role |
|---|---|
| `semantic-release` | Analyses commits, determines next version, creates git tag, generates release notes, triggers version-file updates |
| `@semantic-release/exec` | Calls `scripts/bump-versions.mjs` to update all language manifests atomically |
| `@semantic-release/git` | Commits the updated manifests back to the branch with a `chore(release):` message |
| `@semantic-release/github` | Creates the GitHub Release with auto-generated notes |
| `commitlint` | Validates every commit message against the Conventional Commits spec |
| `husky` | Runs `commitlint` as a `commit-msg` git hook so invalid messages are rejected before push |

All tooling lives in a root `package.json` (tooling-only, never published). The `scripts/bump-versions.mjs` script updates `rust/Cargo.toml`, `python/pyproject.toml`, and `nodejs/package.json` (and eventually `cpp/CMakeLists.txt`) to the new version in a single atomic step.

#### Branch strategy

| Branch | Channel | Tag format | Example |
|---|---|---|---|
| `main` | Pre-release (RC) | `vX.Y.Z-rc.N` | `v0.3.0-rc.1` |
| `release` | Production | `vX.Y.Z` | `v0.3.0` |

- Every merge to `main` that contains a version-bumping commit type triggers a new RC automatically.
- Merging `main` into `release` promotes the current RC to a production release. No new commits are added; semantic-release strips the `-rc.N` suffix and creates the final tag.
- `release` is a promotion-only branch — nothing is ever developed directly on it.

#### Commit type → version bump rules

| Commit type | Bump | Example |
|---|---|---|
| `feat` | minor | `feat(nodejs): add getRawGroup` |
| `fix`, `perf`, `refactor` | patch | `fix(python): reader handles empty attrs` |
| `feat!` or `BREAKING CHANGE:` footer | major | `feat!: remove legacy parse_v0 API` |
| `docs`, `style`, `test`, `build`, `ci`, `chore` | none | `docs: update USAGE.md feature matrix` |

semantic-release analyses **all commits since the last production tag** and applies the highest-priority bump found. If a `feat:` and three `fix:` commits land between two production releases, the result is a minor bump.

#### Scope convention

Scopes are informational — they appear in the CHANGELOG and help readers understand which language or subsystem changed. They do not gate which packages get bumped (all bump together).

Standard scopes: `rust`, `python`, `nodejs`, `cpp`, `schema`, `ci`, `docs`.

#### Setup order (one-time, from a clean `main` baseline)

1. Merge all pending feature branches to `main`
2. Manually tag the current tip of `main` as `v0.1.0` — gives semantic-release its anchor point
3. Create the `release` branch from `main` at that same commit
4. On a new branch (`chore/release-pipeline`), add in order:
   - Root `Cargo.toml` workspace file (two lines)
   - Root `package.json` + install semantic-release + commitlint
   - `.releaserc.json`
   - `scripts/bump-versions.mjs`
   - `commitlint.config.js`
   - `.husky/commit-msg` hook
   - `.github/workflows/release.yml`
5. Dry-run: `npx semantic-release --dry-run` — verify config without pushing
6. PR `chore/release-pipeline` → `main` (all `chore:` commits → no RC triggered)
7. Set branch protection rules on GitHub for both `main` and `release`
8. The next `feat:` or `fix:` PR merged to `main` creates the first live RC (`v0.2.0-rc.1`)
9. Validate RC, then PR `main` → `release` → first production release (`v0.2.0`)

#### Extending to C++

When the C++ implementation lands, add one line to `scripts/bump-versions.mjs` that patches the version in `cpp/CMakeLists.txt`. No other part of the pipeline changes.

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

## Phase 10 — CI/CD Hardening (`.github/workflows/`)

The individual language workflows (`python.yml`, `nodejs.yml`, `rust.yml`, `cpp.yml`) and the cross-check are added incrementally as each language lands — **not** all at once here. By the time Phase 9 is reached, all five workflow files already exist and the cross-check is running all four languages.

Phase 9 adds what cannot be done incrementally: **release automation, smoke tests, and publish pipelines**.

| What Phase 9 adds | Why it waits until here |
|---|---|
| Tag-triggered publish to PyPI / npm / crates.io / vcpkg | Needs all four libraries stable enough to release |
| `smoke.yml` — clean-install smoke tests per language | Needs published packages to install |
| Version-bump enforcement in `cross_check.yml` | Needs all four languages to have agreed on semver policy |
| Automated release notes generation | Needs a stable commit history across all languages |

The versioning mechanics (semantic-release, conventional commits, branch strategy, bump-versions script) are specified in full in [§ Phase 9 — Versioning Policy](#versioning-policy). Phase 10 adds only the publish steps on top of that foundation.

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
    └──► Phase 6 — Cross-check active from Phase 2 onward
                 2 languages after Phase 2; 3 after Phase 3; 4 after Phase 4; 5 after Phase 5.
                 Each new language is one `needs:` addition to cross_check.yml.
                      │
                      └─ Gate: cross-check green for 3+ languages
                      ▼
             Phase 4 — C++ (complete vertical slice, ~3–5 days)
                 reader → writer → builder → tests → cpp.yml; extend cross_check to 4 languages → vcpkg v0.1.0
                      ▼
             Phase 5 — Go (complete vertical slice, ~3–4 days)
                 reader → writer → builder → tests → go.yml; extend cross_check to 5 languages

Phase 7 — Examples      ← parallel with Phases 2–5              (~1–2 days)
Phase 8 — Viewer        ← after Phase 2 (Node.js) complete      (~2–3 days)
Phase 9 — Distribution  ← after each language is CI-green        (~1–2 days/language)
Phase 10 — CI Hardening ← smoke tests, publish automation, tags  (~1 day)
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
