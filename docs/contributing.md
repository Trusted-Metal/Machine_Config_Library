# Contributing — Machine Config Library

Workflows for contributors: generating fixtures, running cross-language checks, and maintaining
the golden file.

← [Back to index](../USAGE.md)

---

## Contents

- [Running the cross-language check](#running-the-cross-language-check)
- [Running the smoke test](#running-the-smoke-test)
- [Generating the golden file and synthetic fixture](#generating-the-golden-file-and-synthetic-fixture)
- [Regenerating the golden file after a reader fix](#regenerating-the-golden-file-after-a-reader-fix)
- [How the cross-check pipeline works](#how-the-cross-check-pipeline-works)
- [File_Version adapters](#file_version-adapters)
- [Adding a new file version — release checklist](#adding-a-new-file-version--release-checklist)
- [Mock fixtures and real version numbers](#mock-fixtures-and-real-version-numbers)

---

## Running the cross-language check

`tools/cross_check.py` is the correctness heartbeat. Run it after any change to Python, Rust,
Node.js, or C++ code. It runs five phases:

1. **Schema validation** — every language's `export-json` output validates against the JSON schema
2. **Read parity** — all languages produce identical JSON for all three fixtures
3. **Write interoperability** — each language's writer output is readable and identical across all readers
4. **Binary copy round-trip** — `copy-hdf5` preserves correction grids verbatim
5. **Correction data hash parity** — SHA-256 of raw correction grids matches across all languages

**Prerequisites (all four languages active):**

```powershell
# PowerShell — activate venv, build Rust + Node.js + C++
.\.venv\Scripts\Activate.ps1
# Rust
cargo build --release --manifest-path rust/Cargo.toml
# Node.js
Push-Location nodejs ; npm ci ; npm run build ; Pop-Location
# C++ (Windows)
cmake -S cpp -B cpp/build -DCMAKE_BUILD_TYPE=Release `
  "-DCMAKE_TOOLCHAIN_FILE=$env:VCPKG_INSTALLATION_ROOT\scripts\buildsystems\vcpkg.cmake"
cmake --build cpp/build --config Release
```

```powershell
# Run all five phases for all four languages
.\.venv\Scripts\python.exe tools/cross_check.py --verbose
```

```bash
# Git Bash
.venv/Scripts/python.exe tools/cross_check.py --verbose
```

Expected output (all four languages):

```
Active languages: python, rust, nodejs, cpp

=== Phase 1: Schema Validation ===
[PASS] 12 combinations validate against schema.

=== Phase 2: Read Parity ===
[PASS] 4 languages agree on all 3 fixtures (12 comparisons).

=== Phase 3: Write Interoperability ===
[PASS] Write interoperability: 4 writer(s) × 4 reader(s) — 12 parity + 4 fidelity checks passed.

=== Phase 3.5: Binary Copy Round-trip ===
[PASS] Binary copy round-trip: all fixtures preserved.

=== Phase 4: Correction Data Hashes ===
[PASS] 4 languages produce identical correction hashes.

All checks passed.
```

Selective flags for faster iteration:

```powershell
# Schema validation only
.\.venv\Scripts\python.exe tools/cross_check.py --skip-read-parity --skip-write-interop --skip-binary-copy --skip-correction-hash

# Python + Rust only, all phases
.\.venv\Scripts\python.exe tools/cross_check.py --langs python,rust --verbose

# Skip write-interop for a fast schema + read-parity check
.\.venv\Scripts\python.exe tools/cross_check.py --skip-write-interop --skip-binary-copy --skip-correction-hash
```

> **Windows note**: `cross_check.py` uses `encoding="utf-8"` in all subprocess calls.
> Without this, Windows subprocess decoding (CP1252) silently corrupts multi-byte unit strings
> such as `μm` and `μs` — the check would report spurious failures on every Windows run.

---

## Running the smoke test

Before committing, run this from outside the `python/` package to simulate a real caller:

```powershell
# PowerShell
.\.venv\Scripts\python.exe scratch/smoke_test.py
```

```bash
# Git Bash
.venv/Scripts/python.exe scratch/smoke_test.py
```

This exercises the public API from a caller's perspective. It catches missing exports,
confusing API surfaces, and path-dependent bugs that pytest's `pythonpath` injection can mask.

---

## Generating the golden file and synthetic fixture

**Prerequisites:** all Python tests pass.

```powershell
# PowerShell — Step 1: confirm the suite is green
.\.venv\Scripts\python.exe -m pytest python/tests/ -v

# Step 2: generate the three fixture files
.\.venv\Scripts\python.exe tools/generate_fixtures.py
```

```bash
# Git Bash
.venv/Scripts/python.exe -m pytest python/tests/ -v
.venv/Scripts/python.exe tools/generate_fixtures.py
```

This writes:

- `fixtures/reference_output.json` — canonical JSON from the reference fixture
- `fixtures/reference_output.sha256` — SHA-256 of that JSON
- `fixtures/synthetic_2laser.h5` — MockConfigBuilder output for non-Python language tests

```powershell
# Step 3: re-run the suite — the previously-skipped golden file test now activates
.\.venv\Scripts\python.exe -m pytest python/tests/ -v
# Expected: all tests passed, 0 skipped
```

**Human review checklist** — verify `fixtures/reference_output.json` before committing:

- [ ] `meta.machine_name` = `"TM-LPBF-02: AconityMIDI+_OG"`
- [ ] `meta.configuration_hash` is exactly 64 hex characters
- [ ] `machine.build_plate_x` = 250.0, `build_plate_y` = 250.0, `build_plate_z` = 20.0
- [ ] Two optical trains present
- [ ] Train 01: `working_distance` = 670.0, `scan_head_offset_x` = −87.5, `scan_head_offset_y` = 23.5, `scan_head_rotation` = 0.0
- [ ] Train 02: `scan_head_offset_x` = 86.074, `scan_head_offset_y` = −21.695, `scan_head_rotation` = 180.0
- [ ] Train 01 `thermal_lensing_passed` = false, Train 02 = true
- [ ] Both trains: `clearbox.correction_data` is 257×257×2 nested list
- [ ] Train 01 `scan_field_correction_file.file_size` = 1138799
- [ ] Train 02 `scan_field_correction_file.file_size` = 1142763
- [ ] No OPCUA fields appear anywhere in the output

```bash
# Step 4: commit all three files together (never split across commits)
git add fixtures/reference_output.json fixtures/reference_output.sha256 fixtures/synthetic_2laser.h5
git commit -m "chore: regenerate golden file and synthetic fixture"
```

---

## Regenerating the golden file after a reader fix

If a bug is found in the reader after the golden file is committed:

1. Fix the File_Version adapter in that version's folder (see [File_Version adapters](#file_version-adapters))
2. Add or update a specific unit test asserting the now-correct value
3. Run the full suite — confirm it passes
4. Re-run `tools/generate_fixtures.py` — it overwrites both files atomically
5. Check `git diff fixtures/reference_output.json` — verify **only** the corrected field changed
6. Commit `reference_output.json` and `reference_output.sha256` together

If other languages are already implemented, check whether they independently produce the
correct value. If they agree with Python's old wrong value, all implementations share the
same bug — update all affected language unit tests after verifying the correct value from
the `Reference Materials/` structure files.

---

## How the cross-check pipeline works

```
python.yml    → exports /tmp/python_output.json
nodejs.yml    → exports /tmp/nodejs_output.json
rust.yml      → exports /tmp/rust_output.json
cpp.yml       → exports /tmp/cpp_output.json
                        │
              cross_check.yml runs cross_check.py for all four languages
                        │
                        ├── Phase 1: schema validation
                        │   Python's jsonschema validates each language's JSON
                        │   against schema/machine_config_v1.schema.json
                        │
                        ├── Phase 2: read parity
                        │   deepdiff compares every language's output
                        │   against the Python golden file
                        │   → any difference = CI failure with exact diff shown
                        │
                        ├── Phase 3: write interop
                        │   each writer's output is read by every reader
                        │
                        ├── Phase 3.5: binary copy round-trip
                        │
                        └── Phase 4: correction data hashes
                            SHA-256 of raw float64 correction grids
                            must be byte-identical across all languages

The sha256 guard in python.yml recomputes the hash of reference_output.json
and compares it to the committed .sha256 file — fails if someone edited the
JSON by hand without regenerating the checksum.
```

The cross-check catches bugs that per-language tests cannot: two languages independently
producing the same wrong value will both pass their own tests but disagree with the golden file.

> **Machine neutrality**: the golden file and fixtures use an AconityMIDI machine as the
> reference example. This is not a constraint on the library — any machine that maps its HDF5
> attributes into the MachineConfig structure is a valid input. New machine types are validated
> by adding their fixture files and asserting their field values follow the schema.

---

## File_Version adapters

Do not add version-specific facades or on-disk layout to the shared capabilities
root (`file.hpp`, `__init__.py`, `index.ts`, `mod.rs`, `file.go`). That file
stays a dispatcher: peek `File_Version`, then call the matching adapter.

Each on-disk version gets its own folder:

| Language | Adapter folder |
|---|---|
| Python | `python/src/machine_config/capabilities/v1_0/` (`file.py`, `layout.py`, `hdf5.py`, `writer.py`) |
| C++ | `cpp/include/machine_config/capabilities/v1_0/` (`file.hpp`, `layout.hpp`, `hdf5.hpp`, `writer.hpp`) |
| Node.js | `nodejs/src/capabilities/v1_0/` (`file.ts`, `layout.ts`, `hdf5.ts`, `writer.ts`) |
| Rust | `rust/src/capabilities/v1_0/` (`file.rs`, `layout.rs`, `hdf5.rs`, `writer.rs`) |
| Go | `go/capabilities/v1_0/` (`file.go`) plus `layout/` and `hdf5/` packages |

A new `File_Version` (e.g. `1.1`) adds `capabilities/v1_1/` plus a registry
entry in the capabilities root. Apps keep calling `open_machine_config` /
`openMachineConfig`.

---

## Adding a new file version — release checklist

Every new `File_Version` must satisfy all four layers before merging.

### 1 — Migration manifest

Create `docs/migrations/v_PREV_to_v_NEW.md` following the format in
[`docs/migrations/mock_v1_0_to_v1_1.md`](migrations/mock_v1_0_to_v1_1.md).

The manifest is the shared specification — every language team reads the same table
to implement their adapter. Fill in every field change with its category, source location,
target location, StableModel field mapping, and whether migration is lossy.

Change categories:

| Category | Definition |
|---|---|
| **Addition** | New HDF5 attr/group with no equivalent in the previous version |
| **Removal** | Existing attr/group dropped; value is lost during forward migration |
| **Name** | Same group, different attr key; value preserved |
| **Path** | Same attr key, different HDF5 group; value preserved |
| **Name+Path** | Both group and key change; value preserved |
| **Consolidate** | **N** per-instance attributes (one per optical train) collapse into **one** shared attribute. Lossless only if every source value already agrees; disagreement is a hard error, not a silent pick — see `docs/migrations/v1_0_to_v1_1.md` Change 1 for a worked example. |
| **Transform** | An attribute's value is parsed/restructured, not merely relocated — new shape, explicit derivation rule required, not a straight copy — see `docs/migrations/v1_0_to_v1_1.md` Change 3 for a worked example. |

### 2 — Structural (all languages)

For each language, the following files must exist and be registered:

```
capabilities/v1_1/
    file.py   / file.ts   / file.rs   / file.hpp   / file.go
    layout.py / layout.ts / layout.rs / layout.hpp / layout.go
    hdf5.py   / hdf5.ts   / hdf5.rs   / hdf5.hpp   / hdf5.go
    writer.py / writer.ts / writer.rs / writer.hpp / writer.go
```

Python dispatcher registration — **all three** registries must move together.
Each is a legitimate, separate "version → its own adapter" table (reader,
writer, and the capability facade each have their own per-version
implementation), so none of them is redundant to drop — but that also means
none of them can be forgotten. Missing one silently leaves that surface
routing to the old version's adapter while the others correctly pick up the
new one:

```python
# python/src/machine_config/reader.py
_ADAPTERS = {
    "1.0": Hdf5AdapterV1_0,
    "1.1": Hdf5AdapterV1_1,   # add here
}

# python/src/machine_config/writer.py
_ADAPTERS = {
    "1.0": Hdf5WriterV1_0,
    "1.1": Hdf5WriterV1_1,    # add here
}

# python/src/machine_config/capabilities/__init__.py
_OPEN = {
    "1.0": MachineConfigFileV1_0.open,
    "1.1": MachineConfigFileV1_1.open,   # add here
}
```

### 3 — StableModel rules

`MachineConfig` and its nested dataclasses are the shared contract across all versions
and all languages. Violating these rules requires a coordinated update across every adapter
in every language simultaneously.

- **New optional field** — add `field: Optional[T] = None` to the relevant dataclass.
  All existing adapters automatically return `None` for it. ✅ Safe.
- **Remove a field** — forbidden without a major breaking change review.
- **Rename a field** — forbidden; update `layout.py` in the new adapter instead.
- **New required field** — only if all existing adapters can supply a sensible default.

If a new version introduces a concept that has no place in `MachineConfig`, that is the
signal to add an optional field to the model — not to work around it via `extra` dicts.

### 4 — Test requirements (Python — template all other languages)

Copy the pattern from `python/tests/test_adapter_migration.py`. Every new version
must have all of the following passing, with zero regressions in the existing suite:

| Test | Requirement |
|---|---|
| `test_vX_Y_read` | All manifest change categories asserted by value |
| `test_vX_Y_roundtrip` | Write → read → write → read with no drift |
| `test_vprev_to_vX_Y` | Forward migration from previous version |
| `test_vX_Y_to_vprev` | Backward migration to previous version |
| `test_vprev_unaffected` | Previous adapter path is undisturbed |
| `test_dispatcher_vX_Y_*` | Full public API via `MachineConfigReader`/`MachineConfigWriter` |
| `test_satisfies_protocol` | `isinstance(reader, ReaderAdapter)` and `isinstance(writer, WriterAdapter)` |

### 5 — Cross-language verification

Run `tools/cross_check.py` with the new version fixture included. All five phases
must pass for all active languages. See [Running the cross-language check](#running-the-cross-language-check).

### 6 — CHANGELOG entry

`CHANGELOG.md` is generated by semantic-release from conventional commit messages
(`.releaserc.json`) — do not hand-author a section in it. Land the new version's work as
`feat` commits (or `feat!`/a `BREAKING CHANGE:` footer, if the new version actually
removes or changes an existing public API surface) with descriptive bodies; the
release-notes-generator turns those into the release's changelog entry.

The consumer-facing version of the migration manifest is instead an **API impact**
section in the manifest itself (see `docs/migrations/v1_0_to_v1_1.md`'s for the
pattern) — a short, prose summary of what changed in the writer/reader API surface
that consumers actually call, separate from the field-by-field HDF5 layout tables
above it. Add one whenever the new version's implementation changes that surface
(e.g. a new constructor parameter, a removed helper function), not only when the
on-disk format changes.

---

## Mock fixtures and real version numbers

`python/tests/test_adapter_migration.py` (and its per-language equivalents once
implemented — see `VALIDATION_PLAN.md` §8) uses on-disk version string
`"1.1-mock"` and field names (`facility_id`, `config_author`) purely to exercise
the change-category architecture (Addition/Removal/Name/Path/Name+Path)
end-to-end, independent of any real schema decision. The `-mock` suffix is
deliberate — it cannot be mistaken for, or collide with, a real `File_Version`.
See `docs/migrations/mock_v1_0_to_v1_1.md` for the manifest.

**Why this needs no decommissioning.** The mock's classes are never registered
in a language's real adapter registry (Python: `_ADAPTERS`) except transiently,
via test-time monkeypatching/injection that reverts after each test — so there
was never a *runtime* collision risk. The `-mock` suffix removes the *naming*
collision risk too: no real version will ever be numbered `"1.1-mock"`, so the
mock stands permanently as generic architecture-regression coverage, decoupled
from whatever version numbers actually ship. A real v1.1 (or v2.0, or anything
else) gets its own dedicated tests under the checklist above (§4), **in
addition to** the mock's, never in place of it. Retiring the mock's coverage
the moment a real version arrives would throw away the only place that
exercises all five categories symmetrically — a real version's content is
driven by product needs and may not touch Name/Path/Removal again for a long
time.

**If a future mock fixture is ever given a real-looking version number**
(avoid this — use a `-mock` or similarly unambiguous suffix from the start),
resolve the collision before merging the real version that reuses it:

1. Rename or delete the mock's layout/reader/writer classes so nothing in the
   test suite still claims to be that version except the real adapter.
2. Audit any StableModel fields the mock added: if the real version reuses
   those names with different semantics, resolve the collision explicitly — do
   not let two unrelated meanings share a field name. If the real version
   doesn't reuse them, remove them; left in place, they become unexplained
   cruft with no adapter that populates them.
3. Confirm no remaining test asserts against the mock's HDF5 layout under a
   `File_Version` value the real adapter now owns.
