# Machine Config Library

Multi-language library for reading, writing, and validating LPBF machine configuration files (`.h5`). Every supported language produces a canonical, schema-validated JSON representation that is byte-identical across implementations — enforced in CI on every push.

**Supported languages:** Python · Node.js · Rust · C++  
**Planned:** Go

---

## Contents

- [What it does](#what-it-does)
- [Feature matrix](#feature-matrix)
- [Quick install](#quick-install)
- [CI pipelines](#ci-pipelines)
- [Commit convention](#commit-convention)
- [Documentation](#documentation)

---

## What it does

LPBF (Laser Powder Bed Fusion) machines export their configuration as HDF5 (`.h5`) files. This library provides a consistent, machine-agnostic API across multiple languages to:

- **Read** any conforming `.h5` config into a typed model
- **Write** a model back to a spec-compliant `.h5` file
- **Export** to a canonical JSON format (identical output across all languages)
- **Validate** that JSON against `schema/machine_config_v1.schema.json`
- **Generate** synthetic test configs via `MockConfigBuilder`
- **Hash** scan-field correction grids for integrity verification

The schema and data model are defined once. All language SDKs are cross-checked in CI against the same reference fixtures.

---

## Feature matrix

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
| `YamlConfigBuilder` | ✅ | ❌ | ❌ | ❌ | ❌ |
| `ConfigEditor` | ✅ | ❌ | ❌ | ❌ | ❌ |
| CLI: `inspect` / `validate` / `demo` | ✅ | ❌ | ❌ | ❌ | ❌ |

---

## Quick install

### Python

```powershell
# PowerShell — from repo root
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e python/
```

```bash
# Git Bash / Linux / macOS
python -m venv .venv
source .venv/Scripts/activate   # Windows Git Bash
# source .venv/bin/activate     # Linux / macOS
pip install -e python/
```

The `machine-config` CLI is installed automatically alongside the package.

### Node.js

```bash
cd nodejs
npm ci
npm run build
```

Uses [h5wasm](https://github.com/usnistgov/h5wasm) — HDF5 compiled to WebAssembly, no system library required.

### Rust

```bash
cd rust
cargo build --release
```

Uses [hdf5-metno](https://github.com/metno/hdf5-rust) with the `static` feature — compiles `libhdf5` from source on first run (~2 min). No system HDF5 install required.

### C++

```bash
# Ubuntu — HDF5 1.14.6 built from source (first run ~8 min, cached thereafter)
cmake -S cpp -B cpp/build -DCMAKE_BUILD_TYPE=Release
cmake --build cpp/build --config Release

# Windows (requires vcpkg on PATH)
cmake -S cpp -B cpp/build -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_TOOLCHAIN_FILE="$env:VCPKG_INSTALLATION_ROOT\scripts\buildsystems\vcpkg.cmake"
cmake --build cpp/build --config Release
```

All other dependencies (HighFive, nlohmann/json, CLI11, Catch2, json-schema-validator) are fetched automatically via CMake `FetchContent`.

---

## CI pipelines

Six workflows run on every push to `main`/`release` and on all pull requests.

| Workflow | File | Platforms | What it does |
|---|---|---|---|
| **Python** | `python.yml` | Ubuntu + Windows (Python 3.11 & 3.12) | Full pytest suite · golden file checksum guard · CLI smoke tests · JSON output parity check |
| **Node.js** | `nodejs.yml` | Ubuntu + Windows | vitest suite (127 tests) · CLI smoke tests · correction-hash parity · Linux vs Windows JSON diff |
| **Rust** | `rust.yml` | Ubuntu + Windows | `cargo test` · CLI smoke tests · correction-hash parity · Linux vs Windows JSON diff |
| **C++** | `cpp.yml` | Ubuntu + Windows | 63 Catch2 tests · CLI smoke tests · Linux vs Windows JSON diff |
| **Cross-Language Check** | `cross_check.yml` | Ubuntu + Windows | Runs `tools/cross_check.py` — 5 phases verifying JSON parity across all four languages against shared fixtures |
| **Release** | `release.yml` | Ubuntu | `semantic-release` — reads commit history, bumps version, generates `CHANGELOG.md`, creates GitHub release |

### Caching strategy

| Language | Cache mechanism | Cache key | Cold run |
|---|---|---|---|
| Python | `actions/setup-python cache: pip` | `pip` lock hash | ~1 min |
| Node.js | `actions/setup-node cache: npm` | `package-lock.json` hash | ~30 s |
| Rust | `Swatinem/rust-cache` on `workspaces: rust` | Cargo.lock + source hash | ~2–3 min |
| C++ (Ubuntu) | `actions/cache` on `hdf5-install/` | `hdf5-1.14.6-no-zlib-<os>` (static key) | ~8–10 min |
| C++ (Windows) | `actions/cache` on `~\AppData\Local\vcpkg\archives` | `vcpkg-archives-x64-windows-<sha256 of vcpkg.exe>` | ~6.5 min |
| C++ FetchContent | `actions/cache` on `cpp/build/_deps` | hash of both `CMakeLists.txt` files | ~3–4 min |

### Known CI issue — Windows vcpkg HDF5 (resolved 2026-08-05)

**Symptom:** CMake error `Could NOT find HDF5 (missing: HDF5_LIBRARIES HDF5_INCLUDE_DIRS)` on Windows, despite a prior successful run.

**Root cause:** The original approach cached the vcpkg `installed/` directory tree. When `windows-latest` updated to VS 2026 (MSVC 19.51), the cached tree — built against the old compiler — was restored and vcpkg's cmake config files pointed to stale paths. A secondary bug (`hashFiles(format('{0}/vcpkg.exe', env.VCPKG_INSTALLATION_ROOT))`) produced an empty hash, collapsing all cache keys to the same value and preventing natural rotation.

**Fix:** Cache only `~\AppData\Local\vcpkg\archives` (vcpkg's binary artifact store), never `installed/`. The `vcpkg install hdf5:x64-windows` step runs unconditionally — vcpkg owns and rebuilds `installed/` from archives on every run (~30 s on a cache hit vs ~6.5 min cold). The key uses the literal path `C:/vcpkg/vcpkg.exe` so `hashFiles` resolves reliably. A `restore-keys` prefix allows a partial hit when the runner image updates, so old archives are still useful as a starting point.

**Future recurrence signal:** If this error reappears after a `windows-latest` image update, check whether `~\AppData\Local\vcpkg\archives` is still the correct binary cache path for the new runner. Run `vcpkg env` on the runner to verify, or explicitly set `VCPKG_BINARY_SOURCES=clear;files,${{ runner.temp }}/vcpkg-archives,readwrite` and cache that path instead.

---

## Commit convention

Commits follow [Conventional Commits](https://www.conventionalcommits.org/) and are enforced by commitlint. `semantic-release` reads the commit history on every push to `main` or `release` to determine the next version and generate `CHANGELOG.md`.

**Format:** `type(scope): subject`  
**Scope:** (optional, warns if non-standard): `rust` · `python` · `nodejs` · `cpp` · `schema` · `ci` · `docs`

**Note:** if a CI update creates a pipeline artifact such as generating a whl or .tgz file, may want to use fix/feat, etc to ensure version bump to test pipeline works correctly. This has been a frustration I have encountered more than once. 

| Type | When to use | Version bump |
|---|---|---|
| `feat` | New user-visible capability (new API, new CLI subcommand, new language support) | **minor** |
| `fix` | Corrects incorrect behaviour (wrong field value, off-by-one, encoding bug) | **patch** |
| `perf` | Measurably faster or lower memory, no behaviour change | **patch** |
| `revert` | Reverts a previous commit | **patch** |
| `refactor` | Restructures code with no observable behaviour change | **patch** |
| `docs` | Documentation only — no code touched | none |
| `style` | Whitespace, formatting, comments — no logic change | none |
| `test` | Adds or modifies tests only | none |
| `build` | Build system files: `CMakeLists.txt`, `Cargo.toml`, `pyproject.toml`, `package.json` | none |
| `ci` | CI workflow files under `.github/workflows/` | none |
| `chore` | Maintenance, dependency bumps, tooling configuration | none |

**Examples:**

```
feat(cpp): add getRawGroup() method
fix(ci): switch vcpkg Windows caching to archives-only
docs: restructure README and add per-language usage docs
test(rust): add OPCUA roundtrip tests
build(cpp): bump HighFive to 2.10.0
ci: add HDF5 vcpkg config verification step on Windows
```

> `BREAKING CHANGE:` in the commit footer triggers a **major** version bump regardless of type.

---

## Documentation

| Document | Audience | Contents |
|---|---|---|
| [USAGE.md](USAGE.md) | Integrators & contributors | End-to-end usage guide for all languages — install, use cases 1–10, CLI reference, quickstart, full workflow, test suite |
| [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) | Contributors | Phase-by-phase build plan, developer quick reference, test suite index, environment setup |
| [CHANGELOG.md](CHANGELOG.md) | Everyone | Auto-generated release history |
| [docs/clearbox-tauri-integration.md](docs/clearbox-tauri-integration.md) | Integrators | ClearBox integration with Tauri desktop applications |
| [schema/machine_config_v1.schema.json](schema/machine_config_v1.schema.json) | Integrators | JSON Schema (draft 2020-12) — the canonical field reference |

