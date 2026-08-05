# Machine Config Library — Usage Guide

This document covers shared concepts and links to per-language documentation.

- **Integrators** — start with the [feature matrix](#language-feature-matrix) and your language's doc below.
- **Contributors** — see [docs/contributing.md](docs/contributing.md) for fixture generation, cross-check, and golden file workflows.

The [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) covers *how the library is built*.

---

## Language documentation

| Language | Status | Document |
|---|---|---|
| Python | Phase 1 complete | [docs/python.md](docs/python.md) |
| Node.js | Phase 2 complete | [docs/nodejs.md](docs/nodejs.md) |
| Rust | Phase 3 complete | [docs/rust.md](docs/rust.md) |
| C++ | Phase 4 complete | [docs/cpp.md](docs/cpp.md) |
| Go | Phase 5 — planned | [docs/go.md](docs/go.md) |

Other docs:

| Document | Contents |
|---|---|
| [docs/schema.md](docs/schema.md) | JSON schema field reference — all types, required fields, unit conventions |
| [docs/contributing.md](docs/contributing.md) | Cross-language check, golden file generation, smoke test |
| [docs/clearbox-tauri-integration.md](docs/clearbox-tauri-integration.md) | ClearBox integration with Tauri desktop applications |

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
cross-check CI pipeline (`cross_check.yml`). The format is defined once in the schema and
never duplicated.

### Fixtures

| File | What it is |
|---|---|
| `fixtures/reference_config.h5` | Real AconityMIDI 2-laser config — primary correctness fixture |
| `fixtures/reference_config_opcua.h5` | Same machine with OPCUA telemetry group populated |
| `fixtures/synthetic_2laser.h5` | MockConfigBuilder output — used by non-Python language test suites |
| `fixtures/reference_output.json` | Golden file — Python's canonical JSON output; all languages must match |
| `fixtures/reference_output.sha256` | SHA-256 of the golden file — CI tamper guard |

### Language feature matrix

| Feature | Python | Rust | Node.js | C++ | Go |
|---|:---:|:---:|:---:|:---:|:---:|
| HDF5 reader (`parse`) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| HDF5 writer (`write`) | ✅ | ✅ | ✅ | ✅ | ⬜ |
| `MockConfigBuilder` | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Schema validation | ✅ | ✅ *(serde)* | ✅ *(Ajv)* | ✅ *(pboettch)* | ⬜ |
| CLI: `export-json` | ✅ | ✅ | ✅ | ✅ | ⬜ |
| CLI: `write-hdf5` | ✅ | ✅ | ✅ | ✅ | ⬜ |
| CLI: `correction-hash` | ✅ | ✅ | ✅ | ✅ | ⬜ |
| CLI: `copy-hdf5` | ✅ | ✅ | ✅ | ✅ | ⬜ |
| OPC-UA config in model | ✅ | ✅ | ✅ | ✅ | ⬜ |
| `get_raw_group()` | ✅ | ✅ | ✅ | ✅ | ⬜ |
| Binary data accessors | ✅ *(numpy)* | ✅ *(ndarray)* | ✅ *(Float64Array)* | ✅ *(vector\<double\>)* | ⬜ |
| `parseWithBinary()` | ✅ | ✅ | ❌ | ✅ | ⬜ |
| `YamlConfigBuilder` | ✅ | ❌ | ❌ | ❌ | ❌ |
| `ConfigEditor` | ✅ | ❌ | ❌ | ❌ | ❌ |
| `config_from_dict` | ✅ | ❌ | ❌ | ❌ | ❌ |
| CLI: `inspect` / `validate` / `demo` | ✅ | ❌ | ❌ | ❌ | ❌ |
