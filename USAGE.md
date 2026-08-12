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
| Go | Phase 5 — in progress | [docs/go.md](docs/go.md) |

Other docs:

| Document | Contents |
|---|---|
| [docs/schema.md](docs/schema.md) | JSON schema field reference — all types, required fields, unit conventions |
| [docs/contributing.md](docs/contributing.md) | Cross-language check, golden file generation, smoke test |
| [docs/clearbox-tauri-integration.md](docs/clearbox-tauri-integration.md) | ClearBox integration with Tauri desktop applications |

---

## Installing as a dependency

> All methods use git credentials that org members already have. Python and Node.js release
> assets additionally require the [GitHub CLI](https://cli.github.com/) (`gh auth login` once).

| Language | Mechanism | Auth |
|---|---|---|
| Python | `pip install git+...` or wheel from release | git credentials or `gh` CLI |
| Node.js | tgz from release | `gh` CLI |
| Rust | `Cargo.toml` git reference | git credentials |
| C++ | CMake `FetchContent` git tag | git credentials |

### Python

```bash
# Pin to a specific tag
pip install "git+https://github.com/Trusted-Metal/Machine_Config_Library.git@v0.2.0-rc.1#subdirectory=python"

# Track main (locks to HEAD on first install)
pip install "git+https://github.com/Trusted-Metal/Machine_Config_Library.git#subdirectory=python"

# From a release wheel (requires gh CLI)
gh release download v0.2.0-rc.1 --repo Trusted-Metal/Machine_Config_Library --pattern "*.whl"
pip install machine_config_library-0.2.0rc1-py3-none-any.whl
```

Pin in `pyproject.toml`:
```toml
dependencies = [
    "machine-config-library @ git+https://github.com/Trusted-Metal/Machine_Config_Library.git@v0.2.0-rc.1#subdirectory=python",
]
```

See [docs/python.md](docs/python.md) for full usage.

### Node.js

Node.js has no git install method — npm does not support `#subdirectory=` installs, so the tgz release asset is required.

```bash
# Download tgz with gh CLI, then install
gh release download v0.2.0-rc.1 --repo Trusted-Metal/Machine_Config_Library --pattern "*.tgz"
npm install machine-config-library-0.2.0-rc.1.tgz
```

Pin in `package.json` (after downloading tgz to your project):
```json
"machine-config-library": "file:./machine-config-library-0.2.0-rc.1.tgz"
```

See [docs/nodejs.md](docs/nodejs.md) for full usage.

### Rust

```toml
# Cargo.toml — pin to a specific tag
machine-config = { git = "https://github.com/Trusted-Metal/Machine_Config_Library", tag = "v0.2.0-rc.1" }

# Track main (locked in Cargo.lock; run `cargo update -p machine-config` to advance)
machine-config = { git = "https://github.com/Trusted-Metal/Machine_Config_Library" }
```

See [docs/rust.md](docs/rust.md) for full usage.

### C++

```cmake
FetchContent_Declare(machine_config
  GIT_REPOSITORY https://github.com/Trusted-Metal/Machine_Config_Library.git
  GIT_TAG        v0.2.0-rc.1
  SOURCE_SUBDIR  cpp)
FetchContent_MakeAvailable(machine_config)
target_link_libraries(your_target PRIVATE machine_config)
```

See [docs/cpp.md](docs/cpp.md) for full usage.

### Updating to a new version

| Language | Pinned to tag | Tracking main |
|---|---|---|
| **Python** | Edit tag in `pip install` command or `pyproject.toml`, re-run install | `pip install --upgrade "git+https://github.com/Trusted-Metal/Machine_Config_Library.git#subdirectory=python"` |
| **Node.js** | `gh release download` new version + `npm install new.tgz` | N/A — always a manual download |
| **Rust** | Edit `tag = "..."` in `Cargo.toml` + `cargo update -p machine-config` | `cargo update -p machine-config` |
| **C++** | Edit `GIT_TAG` in `CMakeLists.txt`, reconfigure CMake | `cmake --fresh -S . -B build` |

> Rust and C++ lock the fetched commit locally (`Cargo.lock` / CMake cache) — the update commands
> above are required to advance even when tracking main. Python and Node.js always fetch
> fresh on install when no tag is specified.

---

## Shared Concepts

### Capability API (preferred for applications)

Applications should prefer the **stable model facade** over raw `MachineConfigReader` field
access. Call `openMachineConfig` / `open_machine_config` (Python/Rust/Go) /
`openMachineConfig` (C++). The library peeks HDF5 `File_Version`, selects an adapter, and
exposes uniform navigation + full-model get/set (`getScanner` / `setScanner`, …) with
`SetMode.Merge` (default) or `SetMode.Replace`. Use `create(version)` for new files;
`open` preserves the on-disk version. Go is in progress (reader + facade in-memory; writer
next — see [docs/go.md](docs/go.md)).

| Concept | Meaning |
|---|---|
| **File_Version** | Version of the on-disk `.h5` layout (`1.0`, `1.1`, …) — adapter key |
| **Package semver** | Library release (`0.2.0-rc.x`) — when you upgrade the SDK |
| **SetMode** | `Merge` updates provided fields; `Replace` replaces the whole node (incl. `extra`) |
| **Result / Outcome** | Boundary errors (`InvalidIndex`, `NotPresent`, `UnsupportedVersion`, …); programmer bugs throw |

Spec + bindings live under `schema/capabilities/` (`api.yaml`, `models.yaml`). Regenerate language stubs with:

```bash
python tools/generate_capabilities.py
python tools/generate_capabilities.py --check   # CI drift guard
```

Low-level `MachineConfigReader` / `Writer` remain for CLI, interop, and adapters’ internal use.

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
