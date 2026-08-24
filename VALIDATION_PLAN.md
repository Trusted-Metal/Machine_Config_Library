# Machine Config Library — Validation Plan

**Branch:** SD-1684  
**Date:** 2026-08-14  
**Audience:** Library author (current), external consumers (eventual), Build Log Library team (portable)  
**Status:** In progress — CI green, validation not yet begun

This document is the authoritative plan for validating the Machine Config Library
across all five language implementations before merging to `main`. It is structured
to be portable: the scenario templates and documentation structure apply directly
to the Build Log Library and any future monorepo library following the same pattern.

---

## Contents

1. [Goals](#1-goals)
2. [Completion Gate — When Is It Safe to Merge?](#2-completion-gate)
3. [Documentation Structure](#3-documentation-structure)
4. [Entry Point Contract](#4-entry-point-contract)
5. [Validation Fixture Generation](#5-validation-fixture-generation)
6. [Scenario Template](#6-scenario-template)
7. [Adapter and Versioning Scenario Definitions](#7-adapter-and-versioning-scenario-definitions)
8. [Happy-Path Scenario Definitions](#8-happy-path-scenario-definitions)
9. [Per-Language Execution Plan](#9-per-language-execution-plan)
   - [9.1 Python](#91-python)
   - [9.2 Node.js](#92-nodejs)
   - [9.3 Rust](#93-rust)
   - [9.4 Go](#94-go)
   - [9.5 C++](#95-c)
10. [C++ Static Tarball](#10-c-static-tarball)
11. [CI Summary Plumbing](#11-ci-summary-plumbing)
12. [Portability Notes — Build Log Library](#12-portability-notes)
13. [Master Completion Checklist](#13-master-completion-checklist)

---

## 1. Goals

The goal is **demonstrated correctness in context**, not just green CI. Specifically:

- Every language reads, writes, and round-trips real machine config files correctly
- The adapter/versioning dispatch system fails loudly and predictably for every
  error class — not silently, not with a panic
- The external consumer experience (install from artifact, not from source) is
  validated and documented for every language
- Every language exposes its public types (`MachineConfig`, `Scanner`, `OpticalTrain`,
  etc.) from the library's public import surface — consumers must never need to define
  or re-implement these types themselves
- Every scenario is recorded with actual observed output so regressions are
  detectable and the evidence survives the author's memory
- The documentation structure is reusable for the Build Log Library with no
  restructuring

The CI test suite verifies mechanics. This plan verifies correctness of behavior
in realistic conditions that the CI cannot fully simulate.

---

## 2. Completion Gate

SD-1684 is ready to merge to `main` when ALL of the following are true:

- [ ] CI green on both platforms (ubuntu-latest, windows-latest) for all workflows
- [ ] Standalone app complete and documented for all five languages
- [ ] All happy-path scenarios pass and are recorded for all five languages
- [ ] All adapter/versioning error scenarios pass and are recorded for all five languages
- [ ] At least one real machine config file (AconityMIDI, from `Reference Materials/`)
      read, modified, written, and re-read successfully in every language
- [x] C++ package (thin, `find_package`-based — see §10) verified locally and CI verification step added (artifact-*producing* step still open — see §10 Step 5)
- [ ] Mock v1.1 adapter migration tests passing in all five languages (AV-09–AV-11)
- [ ] `docs/validation/README.md` master summary complete

---

## 3. Documentation Structure

All validation artifacts live under `docs/validation/`. The structure is identical
for Machine Config and Build Log Library — only the content differs.

```
docs/validation/
├── README.md                  ← master summary (compiled from per-language files)
├── SCENARIO_DEFINITIONS.md    ← language-agnostic scenario specs (this section)
├── fixtures/
│   ├── v1_1_simulated.h5      ← hand-crafted future-version fixture (see §5)
│   ├── missing_required.h5    ← fixture with required group removed
│   ├── missing_version.h5     ← fixture with File_Version attr absent
│   └── README.md              ← how each fixture was created
├── python/
│   ├── app/                   ← standalone validation app (ported from external)
│   ├── results.md             ← observed output for every scenario
│   └── PASS_FAIL.md           ← one-line verdict per scenario
├── nodejs/
│   ├── app/
│   ├── results.md
│   └── PASS_FAIL.md
├── rust/
│   ├── app/
│   ├── results.md
│   └── PASS_FAIL.md
├── go/
│   ├── app/
│   ├── results.md
│   └── PASS_FAIL.md
└── cpp/
    ├── app/
    ├── results.md
    ├── PASS_FAIL.md
    └── tarball/               ← static tarball build artifacts and instructions
```

### `results.md` format (per language)

Each scenario gets one entry:

```markdown
## S-01: Read reference fixture, all scalar fields

**Date:** YYYY-MM-DD  
**Platform:** Windows 11 / Ubuntu 24.04  
**Library version:** commit SHA or tag

**Command run:**
\`\`\`
<exact command>
\`\`\`

**Observed output:**
\`\`\`
<exact terminal output, unedited>
\`\`\`

**Expected:** <what correct behavior looks like>  
**Verdict:** PASS / FAIL  
**Notes:** <anything anomalous, platform-specific, or worth flagging>
```

### `docs/validation/README.md` format

A table compiling the verdict from every language's `PASS_FAIL.md`:

```markdown
| Scenario | Python | Node.js | Rust | Go | C++ |
|----------|--------|---------|------|----|-----|
| S-01 Read reference fixture | ✅ | ✅ | ✅ | ✅ | ✅ |
| S-02 Read with binary data  | ✅ | ✅ | ✅ | ✅ | ✅ |
...
```

---

## 4. Entry Point Contract

Every language's validation app must follow this contract so CI integration and
manual runs are consistent and comparable across languages.

**stdout format:** Each scenario prints one line on completion:
```
[PASS] S-01: Read reference fixture, all scalar fields
[FAIL] AV-03: v1.0 reader encountering a v1.1 file fails loudly
       Detail: expected UnsupportedFileVersionError, got no error
```

**Exit code:** The process exits `0` if all scenarios pass, `1` if any scenario
fails. Do not swallow exceptions and print PASS.

**`main` entry point pattern (implement identically in all languages):**
```
1. Accept two command-line arguments: <fixtures_dir> <real_dir>
   fixtures_dir = absolute path to repo fixtures/ directory
   real_dir     = absolute path to repo "Reference Materials/" directory
2. Define a list of scenario functions in run order (S-01..S-10, AV-01..AV-08)
3. For each scenario:
   a. Call scenario function with (fixtures_dir, real_dir)
   b. Print "[PASS] <ID>: <title>" or "[FAIL] <ID>: <title>\n       <detail>"
4. Print summary: "N/M scenarios passed."
5. Exit 0 if N == M, else exit 1
```

**Scenario function signature (each scenario file exports exactly one function):**
```python
# Python
def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]: ...
```
```typescript
// Node.js
export async function run(fixturesDir: string, realDir: string): Promise<{passed: boolean; detail: string}>
```
```rust
// Rust
pub fn run(fixtures_dir: &std::path::Path, real_dir: &std::path::Path) -> (bool, String)
```
```go
// Go
func Run(fixturesDir, realDir string) (passed bool, detail string)
```
```cpp
// C++
struct Result { bool passed; std::string detail; };
Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path& realDir);
```

**Capturing output for `results.md`:**
```bash
# Linux / macOS / Git Bash
python main.py /path/to/fixtures/ "/path/to/Reference Materials/" 2>&1 | tee run_output.txt

# PowerShell
python main.py /path/to/fixtures/ "C:/path/Reference Materials/" 2>&1 | Tee-Object run_output.txt
```
Paste the full contents of `run_output.txt` verbatim into each scenario's `results.md`
entry. Do not paraphrase.

---

## 5. Validation Fixture Generation

The AV scenarios require hand-crafted HDF5 fixtures that do not exist in the repo.
Generate all fixtures once using the script below, commit them to
`docs/validation/fixtures/`, and document how each was made in
`docs/validation/fixtures/README.md`.

**Generate all AV fixtures:**
```bash
# From repo root, with venv active (h5py is included in machine_config[dev]):
python docs/validation/fixtures/generate_fixtures.py \
  --source fixtures/reference_config.h5 \
  --outdir docs/validation/fixtures/
```

**What `generate_fixtures.py` must produce** (write this script as the first task):

| Output file | Transformation |
|---|---|
| `v2_0_unknown.h5` | Copy reference; overwrite `File_Version` attr → `"2.0"` |
| `missing_version.h5` | Copy reference; delete `File_Version` attr |
| `v1_1_simulated.h5` | Copy reference; overwrite `File_Version` → `"1.1"`; add `Future_Group/` subgroup |
| `missing_machine_group.h5` | Copy reference; delete `Machine/` group |
| `corrupt_scalar.h5` | Copy reference; overwrite `Machine/Build_Plate_X` with string `"not_a_number"` |
| `version_whitespace.h5` | Copy reference; overwrite `File_Version` → `" 1.0 "` |
| `empty_version.h5` | Copy reference; overwrite `File_Version` → `""` |

**h5py transformation pattern:**
```python
import h5py, shutil

def copy_and_modify(src: str, dst: str):
    shutil.copy2(src, dst)
    with h5py.File(dst, 'r+') as f:
        # Overwrite attr:  f.attrs['File_Version'] = '2.0'
        # Delete attr:     del f.attrs['File_Version']
        # Delete group:    del f['Machine']
        # Add group:       f.create_group('Future_Group')
        # Overwrite scalar attr with wrong type:
        #   del f['Machine'].attrs['Build_Plate_X']
        #   f['Machine'].attrs['Build_Plate_X'] = 'not_a_number'
        pass
```

**Verify all fixtures open without crashing:**
```bash
python -c "
import h5py, pathlib
for p in sorted(pathlib.Path('docs/validation/fixtures').glob('*.h5')):
    try:
        h5py.File(p, 'r').close()
        print(f'OK  {p.name}')
    except Exception as e:
        print(f'ERR {p.name}: {e}')
"
```

---

## 6. Scenario Template

Each scenario is defined once here (language-agnostic) and executed per language.
The scenario ID is stable — `S-01` means the same thing in Python, Rust, and in
the Build Log Library.

### Scenario definition format

```
ID:          S-NN
Title:       <one-line description>
Category:    happy-path | adapter-versioning | error-contract
Layer:       reader | writer | builder | adapter | dispatcher
Precondition: <starting file/state>
Action:      <what the app does>
Expected:    <correct behavior — specific field values, error type, exit code>
Rationale:   <why this scenario exists — what bug class it catches>
```

---

## 7. Adapter and Versioning Scenario Definitions

These scenarios target the dispatch and error contract layer specifically.
Each targets exactly one failure mode so attribution is unambiguous.

---

### AV-01: Unknown File_Version string

```
ID:          AV-01
Title:       Reader rejects unknown File_Version with typed error
Category:    adapter-versioning
Layer:       dispatcher
Precondition: reference_config.h5 read into memory, File_Version overwritten to "2.0"
              in memory (or a hand-crafted fixture with File_Version="2.0")
Action:      Attempt to read/parse the file
Expected:    Typed error returned — UnsupportedFileVersionError (or language equivalent)
             with version string "2.0" accessible on the error object.
             No panic. No silent default to v1.0.
             Exit code non-zero if run via CLI.
Rationale:   Silent fallback to a wrong adapter is the most dangerous versioning failure.
             Every language must make this impossible.
```

**Fixture:** Use the MockConfigBuilder to write a v1.0 file, then use h5py/hdf5-rs/h5c
to overwrite the `File_Version` root attribute to `"2.0"` and save as
`docs/validation/fixtures/v2_0_unknown.h5`.

---

### AV-02: Missing File_Version attribute

```
ID:          AV-02
Title:       Reader handles absent File_Version attribute predictably
Category:    adapter-versioning
Layer:       dispatcher
Precondition: HDF5 file with no File_Version root attribute
              (hand-crafted: create minimal HDF5, write no version attr)
Action:      Attempt to read/parse the file
Expected:    Either: defaults to "1.0" and reads successfully (if that is the
             documented contract), OR returns a typed MissingFileVersionError.
             The behavior must be identical across all five languages.
             No panic. Behavior must be documented in the results.
Rationale:   Real-world files from pre-versioning tools will lack this attribute.
             Inconsistent handling across languages breaks interop.
```

**Fixture:** `docs/validation/fixtures/missing_version.h5` — create with Python
`h5py`: create root group, write no `File_Version` attr, write one dummy dataset.

---

### AV-03: Simulated future version (v1.1) file

```
ID:          AV-03
Title:       v1.0 reader encountering a v1.1 file fails loudly
Category:    adapter-versioning
Layer:       dispatcher
Precondition: HDF5 file with File_Version="1.1", otherwise valid v1.0 structure
              plus one additional group ("Future_Group/") not in v1.0 schema
Action:      Attempt to read/parse with each language's library
Expected:    UnsupportedFileVersionError with version "1.1".
             The "Future_Group/" data is NOT silently ignored and parsed as v1.0.
             No panic. No partial read.
Rationale:   Forward compatibility contract. A v1.0 reader must not silently
             truncate a v1.1 file — it must fail so the caller knows to upgrade.
```

**Fixture:** `docs/validation/fixtures/v1_1_simulated.h5` — copy reference_config.h5,
overwrite `File_Version` to `"1.1"`, add a dummy `Future_Group/` subgroup.

---

### AV-04: Missing required HDF5 group

```
ID:          AV-04
Title:       Reader returns typed error when required group is absent
Category:    adapter-versioning / error-contract
Layer:       reader (v1.0 adapter)
Precondition: HDF5 file with File_Version="1.0" but Machine/ group deleted
Action:      Attempt to read/parse
Expected:    Typed error (not panic) indicating the missing group.
             Error message must name the missing path.
             No partial MachineConfig returned.
Rationale:   Corrupt or hand-edited files in the field will be missing groups.
             A panic here brings down the consuming process.
```

**Fixture:** `docs/validation/fixtures/missing_machine_group.h5` — copy
reference_config.h5, delete `Machine/` group with h5py.

---

### AV-05: Valid version, corrupt required scalar field

```
ID:          AV-05
Title:       Reader handles corrupt required attribute gracefully
Category:    error-contract
Layer:       reader (v1.0 adapter)
Precondition: HDF5 file with File_Version="1.0", Machine/Build_Plate_X attribute
              set to a byte string that cannot be parsed as float64
Action:      Attempt to read/parse
Expected:    Typed error returned, not a panic.
             The error identifies the field and path.
             The rest of the file is NOT partially returned.
Rationale:   Attribute type coercion is a real failure mode when files are
             written by non-library tools. Must fail loudly.
```

**Fixture:** `docs/validation/fixtures/corrupt_scalar.h5` — copy reference_config.h5,
overwrite `Machine/Build_Plate_X` with a string value `"not_a_number"`.

---

### AV-06: Version string whitespace variants

```
ID:          AV-06
Title:       Dispatcher normalizes whitespace in File_Version
Category:    adapter-versioning
Layer:       dispatcher
Precondition: reference_config.h5 with File_Version=" 1.0 " (leading/trailing space)
Action:      Read/parse
Expected:    Successfully dispatches to v1.0 adapter and reads correctly.
             All five languages must behave identically.
Rationale:   Real files from legacy tools have been observed with whitespace in
             version strings. Inconsistent trimming breaks cross-language parity.
```

**Fixture:** `docs/validation/fixtures/version_whitespace.h5` — copy reference_config.h5,
overwrite `File_Version` attr to `" 1.0 "`.

---

### AV-07: Empty string File_Version

```
ID:          AV-07
Title:       Dispatcher handles empty string File_Version
Category:    adapter-versioning
Layer:       dispatcher
Precondition: HDF5 file with File_Version=""
Action:      Read/parse
Expected:    Either: defaults to "1.0" (if that is the documented contract for
             empty string), OR returns typed error.
             Behavior must be identical across all five languages.
             Document the actual contract in results.
Rationale:   Establishes and verifies the empty-string contract explicitly.
```

**Fixture:** `docs/validation/fixtures/empty_version.h5` — copy reference_config.h5,
overwrite `File_Version` to `""`.

---

### AV-08: Roundtrip version string fidelity

```
ID:          AV-08
Title:       File_Version string survives write→read unchanged
Category:    adapter-versioning
Layer:       writer + reader
Precondition: reference_config.h5
Action:      Read → write to temp → read temp → compare File_Version strings
Expected:    File_Version is exactly "1.0" after roundtrip in all five languages.
             No whitespace added, no truncation, no case change.
Rationale:   A writer that silently normalizes the version string would break
             future version dispatch for files it produces.
```

---

### AV-09: Mock v1.1 adapter — new adapter does not affect existing v1.0 adapter

```
ID:          AV-09
Title:       Adding a mock v1.1 adapter leaves the v1.0 adapter and all existing tests unchanged
Category:    adapter-versioning
Layer:       adapter isolation
Precondition: Existing v1.0 reader/writer; mock v1.1 reader/writer added to tests only
Action:      Run the full existing test suite after adding the mock v1.1 adapter
Expected:    All pre-existing tests pass unchanged. No v1.0 adapter code modified.
Rationale:   The adapter pattern's primary promise is that adding a new version
             is additive — it must not require modifying any existing adapter.
```

**Mock v1.1 layout:** defined in `docs/migrations/mock_v1_0_to_v1_1.md`.
All five languages implement the same 10 synthetic HDF5 changes from that manifest.
The HDF5 keys are the cross-language contract; StableModel field names follow each
language's naming convention.

**Serialization safety (all languages, required before merging AV-09–AV-11):**
The mock's ADDITION fields (`facility_id`/`config_author`, or each language's
naming-convention equivalent) must never appear in that language's canonical JSON
export — the output `tools/cross_check.py` compares against Python's golden file
(Python: `_config_to_dict()`; each other language's `to_json`/`toJson`/`ToJson`/
equivalent). If they leak in, `cross_check.py` Phase 1 (schema validation — key not
in `schema/machine_config_v1.schema.json`) and Phase 2 (read parity — extra key vs.
Python's golden output) both fail, for every fixture, not just the mock's.

- **Python** is safe by construction: `_config_to_dict()` is a hand-written
  whitelist — a field not explicitly listed is never emitted, mock or not.
- **Rust is not safe by default.** `MachineConfigMeta` derives `Serialize` via
  serde — any new field on the struct is included in every JSON export
  automatically unless annotated. The mock fields must carry
  `#[serde(skip_serializing_if = "Option::is_none", default)]` (or `#[serde(skip)]`),
  the same pattern already used for other optional fields in `rust/src/models.rs`.
- **Node.js**: `hdf5.ts`'s exporter calls `JSON.stringify(config, ...)` directly on
  the constructed object, so it's safe as long as the real v1.0 reader path never
  sets the mock keys on the object it returns. Do not add them to the v1.0 reader's
  object literal — only the mock reader/writer should ever populate them.
- **C++ / Go**: before implementing, confirm whether that language's JSON exporter
  enumerates fields explicitly (safe) or serializes the struct/object wholesale
  (unsafe without an explicit skip/omit annotation), and annotate accordingly.

Each language's AV-09 implementation must assert the mock's ADDITION fields are
absent from that language's `export-json` output for a v1.0 fixture — this is the
regression check that would have caught a serialization leak.

---

### AV-10: Mock v1.1 adapter — forward migration (v1.0 → v1.1)

```
ID:          AV-10
Title:       v1.0 fixture migrates forward to v1.1 correctly across all change categories
Category:    adapter-versioning
Layer:       adapter + StableModel
Precondition: fixtures/reference_config.h5 (v1.0)
Action:      Read with v1.0 adapter → write with mock v1.1 adapter → read back with
             mock v1.1 adapter. Verify all five change categories:
               ADDITION  — facility_id and config_author are None (no v1.0 source);
                           HDF5 attrs are present but empty
               REMOVAL   — gas_flow_direction and recoat_direction are None (dropped)
               NAME      — machine_name and working_distance values preserved
               PATH      — build_plate.z and build_plate.corner_radius values preserved
               NAME+PATH — build_plate.x and build_plate.y values preserved
Expected:    Preserved fields identical before and after. Lossy fields are None.
Rationale:   Verifies the StableModel is the correct handoff point between adapters
             and that each change category behaves as documented in the manifest.
```

---

### AV-11: Mock v1.1 adapter — backward migration (v1.1 → v1.0)

```
ID:          AV-11
Title:       v1.1 file migrates backward to v1.0 correctly; ADDITION fields are lost
Category:    adapter-versioning
Layer:       adapter + StableModel
Precondition: Mock v1.1 file written by AV-10 (or a fresh mock v1.1 file with
             facility_id and config_author populated)
Action:      Read with mock v1.1 adapter → write with v1.0 adapter → read back with
             v1.0 adapter. Verify:
               ADDITION  — facility_id and config_author are None after v1.0 read
                           (v1.0 writer does not write these attrs — intentionally lost)
               NAME/PATH — all preserved fields survive back to v1.0 layout
Expected:    ADDITION fields None after roundtrip. All other preserved fields intact.
Rationale:   Verifies the backward migration contract: additions introduced in v1.1
             are explicitly lost when downgrading, not silently corrupted.
```

---

## 8. Happy-Path Scenario Definitions

---

### S-01: Read reference fixture, all scalar fields

```
ID:          S-01
Title:       Read reference fixture and verify all scalar fields
Category:    happy-path
Layer:       reader
Precondition: fixtures/reference_config.h5
Action:      Parse the file, print key fields to stdout
Expected:    machine_name = "TM-LPBF-02: AconityMIDI+_OG"
             build_plate_x ≈ 250.0
             build_plate_y ≈ 250.0
             len(optical_trains) = 2
             train[0].scanner.working_distance ≈ 670.0
             train[0].scanner.scan_head_rotation ≈ 0.0
             train[1].scanner.scan_head_rotation ≈ 180.0
             configuration_hash: 64 hex characters
             file_version: "1.0"
Rationale:   Establishes baseline read correctness against a known-good fixture.
```

---

### S-02: Read reference fixture with binary data (correction grids)

```
ID:          S-02
Title:       Read correction grids and verify shape and finite values
Category:    happy-path
Layer:       reader
Precondition: fixtures/reference_config.h5
Action:      Parse with includeBinary=true, access correction_data and
             inverse_correction_data for train 0
Expected:    correction_data shape: [257, 257, 2]
             inverse_correction_data shape: [257, 257, 2]
             Both contain at least one finite (non-NaN) value
             Forward and inverse arrays differ (not byte-identical)
             SHA-256 hash of correction_data bytes matches reference value
             (record the hash in results.md on first run)
Rationale:   Binary data round-trips are the highest-risk correctness area.
```

---

### S-03: Read real AconityMIDI fixture

```
ID:          S-03
Title:       Read real-world AconityMIDI machine config file
Category:    happy-path
Layer:       reader
Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
Action:      Parse the file, print all fields to stdout
Expected:    No error. All fields present in output match known machine parameters.
             (Record actual observed values in results.md — these become the
             reference for regression detection.)
Rationale:   This is the primary real-world validation. If the library cannot
             read a real file from the actual machine, it is not production-ready.
```

---

### S-04: Write modified config and verify field change survives roundtrip

```
ID:          S-04
Title:       Modify a scalar field, write, re-read, verify change persisted
Category:    happy-path
Layer:       writer + reader
Precondition: fixtures/reference_config.h5
Action:      Read → change machine_name to "VALIDATION_TEST_MACHINE" →
             write to temp file → read temp file → assert machine_name matches
Expected:    machine_name == "VALIDATION_TEST_MACHINE" after roundtrip.
             All other fields unchanged.
             file_version == "1.0" unchanged.
Rationale:   Basic write correctness. If a modified field does not survive,
             the writer has a silent data loss bug.
```

---

### S-05: Full binary roundtrip with correction hash verification

```
ID:          S-05
Title:       Copy config with binary data, verify correction hash unchanged
Category:    happy-path
Layer:       writer + reader (copy-hdf5 path)
Precondition: fixtures/reference_config.h5
Action:      Read with includeBinary=true → write to temp → read temp with
             includeBinary=true → compare SHA-256 of correction_data bytes
Expected:    SHA-256 of correction_data for train 0 identical before and after.
             SHA-256 of inverse_correction_data for train 0 identical before and after.
             file_size of scan_field_correction_file unchanged.
Rationale:   Silent precision loss in binary data is undetectable without hash comparison.
             This is the same check the cross_check Phase 4 performs — verify it
             also passes through the standalone app.
```

---

### S-06: Build synthetic config and verify fields

```
ID:          S-06
Title:       Build a synthetic config with MockConfigBuilder and verify fields
Category:    happy-path
Layer:       builder
Precondition: None (builder creates from scratch)
Action:      Build a 2-laser config → verify fields → save to temp → re-read
Expected:    len(optical_trains) == 2
             train[0].scanner.scan_head_rotation ≈ 0.0
             train[1].scanner.scan_head_rotation ≈ 180.0
             machine_name non-empty
             correction_data centre cell ≈ 2.0 (Gaussian peak)
             After save and re-read: all fields match
Rationale:   Builder is used in CI fixture generation and by consumers who need
             synthetic test configs. Must produce valid, re-readable output.
```

---

### S-07: OPCUA config roundtrip

```
ID:          S-07
Title:       Read, write, and re-read OPCUA fixture
Category:    happy-path
Layer:       reader + writer (OPCUA path)
Precondition: fixtures/reference_config_opcua.h5
Action:      Read → write to temp → read temp
Expected:    opcua.client.server_url unchanged
             opcua.client.session_timeout unchanged
             opcua.triggers_enabled unchanged
             All trigger names preserved
             "Chamber Oxygen Level" trigger: signal, subsystem, rule_enabled,
             start_value, stop_value all unchanged
Rationale:   OPCUA is an optional complex subgraph. Silent data loss in triggers
             would not be caught by scalar field checks.
```

---

### S-08: Modify real AconityMIDI file and verify adapter pipeline

```
ID:          S-08
Title:       Drastic field change to real file, verify adapter pipeline integrity
Category:    happy-path (applied)
Layer:       reader + writer + adapter
Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
Action:      Read the real file → make ALL of the following changes:
               1. Add a third optical train (clone train 1, change train_id)
               2. Change build_plate_x from 250.0 to 350.0
               3. Set scan_head_rotation on new train to 90.0
               4. Clear all correction data (set to None/null/nil)
               5. Change machine_name to "MODIFIED_ACONITY_VALIDATION"
             → write to temp → read temp → verify all five changes persisted
Expected:    len(optical_trains) == 3
             build_plate_x ≈ 350.0
             train[2].scanner.scan_head_rotation ≈ 90.0
             train[2].optional_components.clearbox.correction_data is None/null/nil
             machine_name == "MODIFIED_ACONITY_VALIDATION"
             file_version still "1.0" (adapter did not change the version)
Rationale:   This is the "drastic change" scenario. Tests that the adapter pipeline
             handles structural changes (new train, nil correction data) without
             silent corruption or wrong-adapter dispatch.
```

---

### S-09: Public type export surface

```
ID:          S-09
Title:       Verify all public model types are importable from the library surface
Category:    happy-path
Layer:       public API / packaging
Precondition: Library installed from artifact (wheel / tarball / module), not from source
Action:      In the standalone app, import MachineConfig, Scanner, OpticalTrain,
             LightSource, Collimator, ScannerCard, ClearBox, ScanFieldCorrectionFile,
             OpcuaConfig, and MockConfigBuilder directly from the library's public
             import path. Construct or type-annotate a variable with each type.
             Do NOT import from any internal/private submodule path.
Expected:    All imports resolve without error.
             IDE type-checking (mypy / tsc / rust-analyzer / gopls) reports no
             unknown type errors when the app is opened in an editor.
             No type needs to be defined or re-implemented by the consumer.
Rationale:   If a consumer must import from internal paths or define their own
             Scanner class to type-annotate a variable, the library's public
             API surface is incomplete. This scenario catches that gap at packaging
             time, not at a consumer's bug report.
Notes per language:
  Python  — all types must be importable from `machine_config` (top-level);
            `py.typed` marker must be present in the wheel for mypy/pyright support.
  Node.js — all types must be importable from the package root; `package.json`
            must have a `types` field pointing at the root `.d.ts` file;
            TypeScript consumers must get full IntelliSense without extra steps.
  Rust    — all public structs/enums must be re-exported from the crate root
            (`machine_config::MachineConfig`, not `machine_config::models::MachineConfig`).
  Go      — all exported types must be accessible via the top-level package import;
            consumers should not need to import internal sub-packages directly.
  C++     — the tarball `include/` directory must contain ONLY the public header(s);
            internal implementation headers must not be present; consumers must
            be able to use all types via `#include <machine_config/machine_config.hpp>`
            with no other include paths needed.
```

---

### S-10: Public facade export surface

**Status: implemented and passing in all five languages' committed validation apps**
(`docs/validation/<lang>/app/`), run against the repo's source directly rather than a
built wheel/tarball/module install — see "Execution note" below for why, and what
that trade-off means for this scenario's own Precondition line.

- Rust: `docs/validation/rust/app/src/scenarios.rs::run_s10` — 20/20 scenarios pass
  (`docs/validation/rust/app`, built via `cargo build --release`).
- Python: `docs/validation/python/app/scenarios.py::run_s10` — 23/23 pass (run
  directly against the editable-installed repo source, no wheel build).
- Node.js: `docs/validation/nodejs/app/scenarios.mts::runS10` — 23/23 pass (app's
  `node_modules/machine-config-library` is a symlink to `nodejs/`, rebuilt via
  `npm run build` before running — no tarball).
- Go: `docs/validation/go/app/scenarios/scenarios.go::RunS10FacadeExportSurface` —
  20/20 pass (the app counts it a pass since the facade is fully reachable and
  usable via its documented `machine-config-go/capabilities` path). Per this
  scenario's own guidance below, record the *root-level-parity* question as
  **N/A (documented exception)**, not PASS or FAIL — the scenario function
  passing confirms the facade works; it doesn't by itself mean Go matches the
  other four languages' root-level reachability, which is intentionally not
  the goal for Go.
- C++: `docs/validation/cpp/app/src/scenarios/scenarios.cpp::s10::run` — 20/20 pass.

**Execution note (rigor level chosen):** run as a lightweight in-repo check, not the
full external-package-install process S-01–S-09 originally called for (building an
actual wheel/tarball/crate-path-dep/module and testing from a project outside the
repo). All five validation apps already depend on the library via a local path/file
reference (Rust: `path = "../../../../rust"`; Go: `replace machine-config-go =>
../../../../go`; Node: `"file:../../../../nodejs"` resolved as a symlink; C++:
`add_subdirectory(...cpp...)`) rather than an installed artifact, so this was already
effectively in-repo before S-10 existed — the "external package" framing in S-01–S-09's
shared Precondition line was aspirational for the facade-specific check this scenario
adds, not a rigor level actually enforced for it. A full packaged-artifact run remains
possible later using the same apps; nothing here forecloses it.

**C++ gap found and fixed while implementing this scenario:** C++ had no
`supportedFileVersions()` at all — not a re-export gap like Rust's, the function
simply didn't exist (checked `cpp/include/machine_config/capabilities/file.hpp`
directly; only `openMachineConfig`/`createMachineConfig` were declared). Added
`supportedFileVersions()` there (returns `{"1.0"}`, mirroring the other four
languages) before S-10 could even be written for C++, plus a new unit test
(`CapabilitySupportedFileVersionsListsV1_0` in `cpp/tests/test_capabilities.cpp`).
Confirmed via the full `machine_config_tests` suite: 579 assertions in 105 test
cases (up from 578/104), all passing.

```
ID:          S-10
Title:       Verify the stable capability facade is importable from the library
             surface, in parity across all five languages
Category:    happy-path
Layer:       public API / packaging
Precondition: Library installed from artifact (wheel / tarball / module), not from
             surface; this scenario covers the separate stable facade (session
             open/create, get/set with SetMode, CapabilityError) added later, which
             needs its own explicit check — see Rationale.
Action:      In the standalone app, import the facade's open/create entry points,
             its session type, its error type, and SetMode directly from the
             library's public import path (see Notes per language for the exact
             names). Open a fixture through the facade and call at least one
             get*/set* method. Do NOT import from any version-specific submodule
             path (e.g. a `capabilities::v1_0` / `capabilities/v1_0` / `v1_0::hdf5`
             path) to do this — if the facade is only reachable that way, the
             surface is incomplete.
Expected:    All imports resolve without error, from the same top-level import path
             already used for S-09's model types (one language's documented
             exception — Go — aside; see Notes per language).
             IDE type-checking reports no unknown type errors.
             grep across the app for a version-specific submodule path
             (`capabilities::v1_0::`, `capabilities/v1_0/`, `capabilities.v1_0.`, or
             that language's equivalent) used to reach the facade itself returns
             zero matches — the version-specific adapter internals must stay
             unreachable/unnecessary from application code, exactly as S-09
             requires for model types.
Rationale:   This scenario exists because the gap it checks for actually happened:
             Rust's crate root (`lib.rs`) re-exported every other public module
             (builder, error, models, reader, writer) but omitted `capabilities`
             entirely — contradicting `lib.rs`'s own stated policy that consumers
             must never need a sub-module path for a public type. Nothing in S-09
             (scoped to model types only) or anywhere else in this plan would have
             caught it; it surfaced only when an application tried to integrate the
             facade and found it unreachable from the crate root. S-10 exists so
             this class of drift — one language's facade silently falling behind
             the others' export surface — has an actual checklist item, the same
             role S-09 already plays for model types.
Notes per language:
  Python  — `open_machine_config`, `create_machine_config`, `supported_file_versions`,
            `CapabilityError`, `MachineConfigFileV1_0`, `SetMode`, plus
            `MissingRequiredGroup` / `SessionClosedError` / `UnsupportedFileVersion`,
            all importable from `machine_config` (top-level) — already correct,
            re-exported in `machine_config/__init__.py`.
  Node.js — `openMachineConfig`, `createMachineConfig`, `supportedFileVersions`,
            `MachineConfigFileV1_0`, `SetMode`, and the `CapabilityError`/
            `MachineConfigFile` types, all importable from the package root —
            already correct, re-exported in `src/index.ts`.
  Rust    — `open_machine_config`, `create_machine_config`, `supported_file_versions`,
            `CapabilityError`, `MachineConfigFileV1_0`, `SetMode` re-exported from
            the crate root (`machine_config::open_machine_config`, not
            `machine_config::capabilities::open_machine_config`) — fixed; see
            CORRECTION_DATA_FACADE_PLAN.md's discussion for the fix and why it's a
            curated re-export list (`pub use capabilities::{...}`), not
            `pub use capabilities::*` (a wildcard would also re-export
            `capabilities`'s child modules — `errors`, `generated`, `merge`,
            `result`, `v1_0` — as new crate-root paths, and `capabilities::Result`
            is a different, two-parameter type from this crate's own,
            already-in-use `error::Result`).
  Go      — **documented exception, not a gap:** the facade
            (`OpenMachineConfig`/`CreateMachineConfig`/`File`/`Error`/`SetMode`) is
            reachable only via `machine-config-go/capabilities` — a second import,
            not the module root. This is deliberately accepted, unlike Rust's case,
            for two reasons: (1) `capabilities` is a normal, publicly exported Go
            package — not an `internal/` path — so nothing about this requires
            reaching into forbidden internals; Go's own compiler already enforces
            that boundary, which is the thing S-09/S-10 exist to catch in languages
            that lack it. (2) Go's model-type re-exports at the root
            (`go/models.go`) are free, zero-maintenance type aliases
            (`type MachineConfig = internal/models.MachineConfig`); Go has no
            equivalent zero-cost mechanism for re-exporting *functions* — doing the
            same for the facade would mean hand-written forwarding wrappers at the
            root (`func OpenMachineConfig(path string) (*capabilities.File,
            *capabilities.Error) { return capabilities.OpenMachineConfig(path) }`,
            one per facade function) that must be kept in lockstep with
            `capabilities`'s real signatures by hand — a second, ongoing place for
            exactly the kind of silent drift this scenario exists to prevent, not a
            fix for it. Record this scenario as **N/A (documented exception)** for
            Go, not FAIL — but if a future contributor ever adds root-level
            forwarding wrappers for convenience, re-litigate this note rather than
            let both the wrapper and `capabilities` drift independently.
  C++     — `openMachineConfig`, `createMachineConfig`, `MachineConfigFileV1_0`,
            `CapabilityError`, `SetMode` all reachable via
            `#include <machine_config/machine_config.hpp>` — already correct;
            `machine_config.hpp`'s own header comment documents the facade as part
            of "the default surface" alongside models/reader/writer/builder.
```

---

## 9. Per-Language Execution Plan

Work through each language completely (all scenarios, app, results documented)
before moving to the next. Recommended order: Python → Node.js → Rust → Go → C++.

### Done-enough-to-proceed gate per language

A language is done when:
- [ ] All S-0N and AV-0N scenarios executed and recorded in `results.md`
- [ ] All verdicts entered in `PASS_FAIL.md`
- [ ] App code committed to `docs/validation/<lang>/app/`
- [ ] Any FAIL verdict has an associated issue or explanation in `results.md`
- [ ] `docs/validation/README.md` master summary and scenario matrix updated for
      this language — do this as each language finishes, not only at final merge
      (Python's row was updated this way; treat that as the pattern, not the
      exception — see §13)
- [ ] Validation status cross-linked from `docs/<lang>.md` — a short note plus a
      link to `docs/validation/<lang>/PASS_FAIL.md`, so the per-language usage doc
      matches what's actually been tested. (Done for Python — see
      `docs/python.md` §"External validation status" — treat that as the template.)

### External validation project location

Every "Step 1 — Create external project" instruction below used a `~/`-relative
path. Standardizing on one fixed location instead, so every language's
standalone app lives in the same place during development:

```
C:\Users\ChrisParham\Desktop\Practice\machineconfiglibrarytesting\mcl_<lang>_validation
```

e.g. `...\machineconfiglibrarytesting\mcl_nodejs_validation`,
`...\machineconfiglibrarytesting\mcl_rust_validation`. This is scratch/working
space — nothing under it is committed to the repo. Once the app is green
locally, it gets ported into `docs/validation/<lang>/app/` per the "After app is
green locally" step in each section below; that's the only copy that ships.

### Recommended step ordering when a language needs new StableModel fields

AV-09–AV-11 needs test-fixture fields on the StableModel (mirroring Python's
`facility_id`/`config_author` — see `docs/migrations/mock_v1_0_to_v1_1.md`) that
ship in the real package, unlike the mock reader/writer classes themselves,
which stay in the test tree and are never packaged. Because of that asymmetry,
add those fields *before* Step 1 in each language's plan below, not after:

1. Add the two nullable/optional test-fixture fields to the language's
   StableModel, with the same "TEST FIXTURE... not a real schema field, never
   serialized" comment used in `python/src/machine_config/models.py`.
2. Rebuild immediately and confirm nothing breaks. The fields are optional, so
   this should be a no-op check, not a real risk — but it's a free, immediate
   signal if something unexpected depends on the model's exact shape.
3. Grep that language's real v1.0 JSON exporter for the new field names to
   confirm they're absent. This verifies the "Serialization safety" property
   (§8, under AV-09) by inspection, before any test exists that could catch a
   leak — cheaper than finding out from a failing cross-language parity check.
4. Only then proceed to Step 1 (build/pack) onward. This way the external
   project is built once, against the library's final shape for this round of
   work, instead of needing a rebuild-and-reinstall partway through once
   AV-09–AV-11 work starts.

---

### 9.1 Python

**Step 1 — Build the wheel:**
```bash
# From repo root, with build installed:
pip install build
python -m build python/
# Output: python/dist/machine_config-<version>-py3-none-any.whl
```

**Step 2 — Create external project and install:**
```bash
mkdir -p "/c/Users/ChrisParham/Desktop/Practice/machineconfiglibrarytesting/mcl_python_validation"
cd "/c/Users/ChrisParham/Desktop/Practice/machineconfiglibrarytesting/mcl_python_validation"
python -m venv .venv
source .venv/bin/activate           # Linux / macOS
source .venv/Scripts/activate       # Windows (Git Bash / MSYS2)
# .venv\Scripts\Activate.ps1        # Windows PowerShell
pip install /path/to/repo/python/dist/machine_config-*.whl
```

**Step 3 — Verify `py.typed` marker:**
```bash
find .venv -name 'py.typed' -path '*/machine_config/*'
# Must print a path. If empty, record as S-09 FAIL and open an issue.
```

**Step 4 — Verify S-09 public type exports:**
```bash
python -c "
from machine_config import (
    MachineConfig, MachineConfigMeta, Machine, OpticalTrain,
    Scanner, LightSource, Collimator, ScannerCard,
    OptionalComponents, ClearBox, ScanFieldCorrectionFile,
    OpcuaConfig, MockConfigBuilder,
)
b: MockConfigBuilder = MockConfigBuilder()
print('PASS: all public types importable from machine_config top-level')
"
# Must print PASS. If ImportError is raised, record the missing name as S-09 FAIL.
```

**Step 5 — Run mypy to verify static types:**
```bash
pip install mypy
mypy --strict --ignore-missing-imports -c "
from machine_config import (
    MachineConfig, MachineConfigMeta, Machine, OpticalTrain,
    Scanner, LightSource, Collimator, ScannerCard,
    OptionalComponents, ClearBox, ScanFieldCorrectionFile,
    OpcuaConfig, MockConfigBuilder,
)
b: MockConfigBuilder = MockConfigBuilder()
"
# Must report: Success: no issues found in 1 source file
```

**Fixtures path:** pass absolute paths to `fixtures/` and `Reference Materials/`
from the repo as command-line arguments — the app does not hardcode paths.

**Scenarios to execute:** S-01 through S-10, AV-01 through AV-11

**AV-09–AV-11 status (Python):** ✅ Complete — implemented in `python/tests/test_adapter_migration.py`
as `MockV1_1Layout`, `MockV1_1Reader`, `MockV1_1Writer` (8 tests, all passing).
The validation app scenarios for AV-09–AV-11 are thin wrappers that invoke the same logic.

**App structure (`docs/validation/python/app/`):**
```
main.py          ← runs all scenarios sequentially, prints results
scenarios.py     ← all 20 run_sXX/run_avXX functions (S-01–09, AV-01–11)
requirements.txt ← pinned to exact library version tested
```
Consolidated from one-file-per-scenario (22 files: `__init__.py` + 21
scenario modules) down to one file per app (§0 "condense per-scenario test
files" decision, applied consistently across all five languages) — no
`scenarios/` package directory needed once it's a single flat module.
`main.py` needed only its import line and `SCENARIOS` table updated to the
new flat function names (`run_s01` instead of `s01_read_scalars.run`).
Two things to watch when re-merging: (1) every scenario file defined its own
`def run(...)`, so each had to be renamed uniquely (`run_s01`, `run_av09`,
etc.) to avoid collisions once flattened into one module; (2) AV-09–11's
`sys.path.insert(0, str(Path(__file__).parent...))` walk to reach
`python/tests/` needed one fewer `.parent` — the file moved one directory
level shallower (out of `scenarios/`), so the relative walk to the repo root
got one segment shorter.

**After app is green locally:** port into `docs/validation/python/app/`, commit,
verify it runs in CI.

**CI integration:** add to `python.yml`:
```yaml
- name: Run Python validation app
  shell: bash
  run: |
    pip install -e "python/[dev]"
    python docs/validation/python/app/main.py \
      "$PWD/fixtures/" "$PWD/Reference Materials/" \
      2>&1 | tee "$RUNNER_TEMP/python_validation.txt"
```

---

### 9.2 Node.js

**Step 1 — Build and pack the tarball:**
```bash
cd nodejs
npm run build      # compile TypeScript → dist/
npm pack           # produces machine-config-<version>.tgz in nodejs/
cd ..
```

**Step 2 — Verify `.d.ts` files and `types` field are in the package:**
```bash
tar -tzf nodejs/machine-config-*.tgz | grep '\.d\.ts'
# Must list files. If empty, TypeScript types are not packaged — S-09 FAIL.

node -e "const p = require('./nodejs/package.json'); console.log('types:', p.types || 'MISSING')"
# Must print a path, not 'MISSING'.
```

**Step 3 — Create external project and install:**
```bash
mkdir -p "/c/Users/ChrisParham/Desktop/Practice/machineconfiglibrarytesting/mcl_nodejs_validation"
cd "/c/Users/ChrisParham/Desktop/Practice/machineconfiglibrarytesting/mcl_nodejs_validation"
npm init -y
npm install /path/to/repo/nodejs/machine-config-*.tgz
```

**Step 4 — Verify S-09 type imports:** Confirm these imports compile without error:
```typescript
import type { MachineConfig, Scanner, OpticalTrain, LightSource,
              Collimator, ScannerCard, ClearBox, OpcuaConfig } from 'machine-config';
import { MachineConfigReader, MachineConfigWriter, MockConfigBuilder } from 'machine-config';
```
Run `tsc --noEmit` to verify zero type errors. Record full output in `results.md` under S-09.

**Note on builder:** Node.js has `MockConfigBuilder` in `nodejs/src/builder.ts`.
S-06 is fully supported.

**Scenarios to execute:** S-01 through S-10, AV-01 through AV-11

**AV-09–AV-11 (Node.js):** Implement `MockV1_1Layout`, `MockV1_1Reader`, `MockV1_1Writer`
in `nodejs/tests/` following the same 10 HDF5 changes in `docs/migrations/mock_v1_0_to_v1_1.md`.
Use Vitest. StableModel field names follow TypeScript camelCase convention.
See "Serialization safety" under AV-09 (§8) — verify the mock fields never leak into `hdf5.ts`'s
`JSON.stringify` output for a v1.0 fixture.

**App structure (`docs/validation/nodejs/app/`):**
```
main.mts
scenarios.mts    ← all 20 runSXX/runAvXX functions (S-01–09, AV-01–11)
package.json
tsconfig.json
```
Consolidated from one-file-per-scenario (21 files: 20 scenario modules +
`main.mts`) down to one file per app (§0 "condense per-scenario test files"
decision, applied consistently across all five languages) — no `scenarios/`
directory needed once it's a single flat module. `main.mts` needed only its
import line and `SCENARIOS` table updated to the new function names
(`runS01` instead of a namespace import per file). Verified `tsc --noEmit`
still passes with zero errors after merging (S-09's actual point). One
relative-path adjustment: AV-09–11's import of the mock adapter
(`nodejs/tests/mockV1_1.js`) needed one fewer `../` segment, since the file
moved one directory level shallower (out of `scenarios/`).

**After app is green locally:** port into `docs/validation/nodejs/app/`, commit.

Note — this is what actually happened, not what was originally speculated
here: the ported app is its **own** small npm project, not compiled as part
of `nodejs/`'s own build. `docs/validation/nodejs/app/package.json` depends on
the local package via `"machine-config-library": "file:../../../../nodejs"`
(requires `nodejs/` to have been built at least once — `npm run build`), and
the app runs directly against its `.mts` sources via `tsx`, not plain `node`
and not a separate tsc-to-`dist/` compile step — Node's own native TypeScript
support can't handle the constructor-parameter-property shorthand the real
adapter classes use throughout (`constructor(private readonly x: T) {}`),
which `tsx` (esbuild-based, same class of tool Vitest already uses
internally) handles correctly. AV-09–11 specifically were written directly in
`docs/validation/nodejs/app/scenarios/` rather than developed externally
first — see the Node.js section of §13 for why.

**CI integration:** add to `nodejs.yml`:
```yaml
- name: Run Node.js validation app
  shell: bash
  run: |
    npm ci && npm run build
  working-directory: nodejs
- name: Install and run the validation app
  shell: bash
  run: |
    npm install
    npx tsx main.mts "$PWD/../../../../fixtures/" "$PWD/../../../../Reference Materials/" \
      2>&1 | tee "$RUNNER_TEMP/nodejs_validation.txt"
  working-directory: docs/validation/nodejs/app
```

---

### 9.3 Rust

**Step 1 — Create external project:**
```bash
mkdir -p "/c/Users/ChrisParham/Desktop/Practice/machineconfiglibrarytesting/mcl_rust_validation"
cd "/c/Users/ChrisParham/Desktop/Practice/machineconfiglibrarytesting/mcl_rust_validation"
cargo init --name mcl_rust_validation
```

**Step 2 — Add dependency to `Cargo.toml`:**
```toml
# Option A — local path (during development):
[dependencies]
machine-config = { path = "/absolute/path/to/repo/rust" }

# Option B — git dependency (once merged to main):
[dependencies]
machine-config = { git = "https://github.com/org/machine-config-library", subdirectory = "rust" }
```
The `path` key takes the absolute path to the `rust/` subdirectory. No `replace`
directive needed — `path` is sufficient for local development.

**Step 3 — Verify S-09 crate-root re-exports:**
```rust
// This must compile without any sub-module paths:
use machine_config::{MachineConfig, Scanner, OpticalTrain, MachineConfigReader, MockConfigBuilder};
// If this fails with "unresolved import", check rust/src/lib.rs for a missing `pub use`.
```

**Note on builder:** Rust has `MockConfigBuilder` in `rust/src/builder.rs`.
S-06 is fully supported.

**Error type mapping for AV scenarios:**
- `UnsupportedFileVersionError` → `MachineConfigError::UnsupportedVersion`
- Missing group → `MachineConfigError::Io` or `MachineConfigError::Parse`
- Record the exact Rust error variant in `results.md`

**Scenarios to execute:** S-01 through S-10, AV-01 through AV-11

**AV-09–AV-11 (Rust):** Implement the mock v1.1 adapter as plain modules under
`rust/tests/` — e.g. `rust/tests/mock_v1_1.rs` (layout constants + `MockV1_1Reader`/
`MockV1_1Writer`, mirroring `nodejs/tests/mockV1_1.ts`'s delegate-then-patch design:
call the real, public `Hdf5AdapterV1_0::parse`/`Hdf5WriterV1_0::write` for the
subcomponents that don't change, then patch the 10 documented differences) plus
`rust/tests/adapter_migration_test.rs` (the `#[test]` functions). **Not** a
`#[cfg(test)]` module — that attribute is for code living inside `src/` that needs
private/`pub(crate)` access; `rust/tests/` is already a separate integration-test
crate, compiled only for `cargo test`, that can only see the library's `pub` items.
Verified this is sufficient: `Hdf5AdapterV1_0::parse` and `Hdf5WriterV1_0::write`
are already fully `pub` (`rust/src/capabilities/v1_0/hdf5.rs`,
`rust/src/capabilities/v1_0/writer.rs`), reachable from `rust/tests/` exactly the
way `rust/tests/integration_test.rs` already reaches them today — no new `pub`
exports needed to build the mock.

**Dispatch-injection is not available in Rust — resolve this before implementing,
don't rediscover it mid-work:** unlike Python's `_ADAPTERS` dict (monkeypatchable)
and Node's exported `_READERS`/`_WRITERS` (temporarily mutable), Rust's dispatcher
is a hardcoded `match version.as_str() { "1.0" => ..., other => Err(...) }` in
`rust/src/reader.rs` and `rust/src/writer.rs` — there is no registry to inject a
`"1.1-mock"` entry into. Adding a real match arm for it would put test-only logic
in shipped dispatch code; refactoring the dispatcher into a registry solely to
enable this test would be architecture change the task doesn't otherwise need.
Given that, AV-09–AV-11 for Rust test the mock adapter directly — call
`MockV1_1Reader`/`MockV1_1Writer` as plain structs, not through the public
`MachineConfigReader`/`MachineConfigWriter` facade. AV-09's actual rationale
("adding v1.1 doesn't require modifying the v1.0 adapter") is satisfied by the
mock living in its own file with zero edits to `capabilities/v1_0/` and the full
existing test suite re-run green after it's added — that already proves the claim
without needing literal dispatch-table injection.

**Verify the HDF5 binding's mutation API before committing to a mock design:**
the crate in use is `hdf5-metno` (`rust/Cargo.toml`), not h5wasm — its support for
deleting an attribute and reopening an already-written file in read/write mode
(the two operations the delegate-then-patch design depends on) has not been
checked. Do not assume parity with h5wasm's API; write a small standalone spike
against `hdf5-metno` first, the same way a standalone h5wasm spike preceded the
Node.js mock, and adjust the design if the binding doesn't support one of these
operations directly.

See "Serialization safety" under AV-09 (§8) — the mock fields **require** an explicit
`#[serde(skip_serializing_if = "Option::is_none", default)]` annotation or they will
leak into every JSON export, not just the mock's, and break `cross_check.py`.

**App structure (`docs/validation/rust/app/`):**
```
src/
  main.rs
  scenarios.rs
  scenarios/
    common.rs
Cargo.toml
```
Consolidated from one-file-per-scenario (20 files) down to one file per app
(§0 "condense per-scenario test files" decision, applied consistently across
all five languages) — `scenarios.rs` holds all 17 `run_sXX`/`run_avXX`
functions (S-01–09, AV-01–08) in ID order and declares `mod common;`, which
resolves to the sibling `scenarios/common.rs` (Rust's file-with-sibling-
submodule-directory convention — no `mod.rs` needed once `scenarios/` no
longer needs to BE the module root). `main.rs` needed only its
`scenario_list` table updated to the new flat function names
(`scenarios::run_s01` instead of `scenarios::s01_read_scalars::run`).

**After app is green locally:** port into `docs/validation/rust/app/`, commit.

**CI integration:** add to `rust.yml`:
```yaml
- name: Run Rust validation app
  shell: bash
  run: |
    cargo run --manifest-path docs/validation/rust/app/Cargo.toml -- \
      "$PWD/fixtures/" "$PWD/Reference Materials/" \
      2>&1 | tee "$RUNNER_TEMP/rust_validation.txt"
```

---

### 9.4 Go

> **Audited against the real `go/` tree before writing this section** (the codebase
> is far more built-out than the previous draft assumed: a `capabilities/` stable-facade
> layer, a real CGo/HDF5 binding in `internal/h5c/`, an existing `_test.go` suite for
> reader/writer/builder/capabilities, and a CLI at `go/cmd/machine-config-cli/`). The
> corrections below replace unverified assumptions with facts read directly from
> `go/reader.go`, `go/writer.go`, `go/builder.go`, `go/file_version.go`,
> `go/models.go`, `go/internal/models/models.go`, `go/internal/h5c/h5c.go`,
> `go/capabilities/**`, and `.github/workflows/go.yml`.

**Step 1 — Create external project:**
```bash
mkdir -p "/c/Users/ChrisParham/Desktop/Practice/machineconfiglibrarytesting/mcl_go_validation"
cd "/c/Users/ChrisParham/Desktop/Practice/machineconfiglibrarytesting/mcl_go_validation"
go mod init mcl_go_validation
```

**Step 2 — Edit `go.mod` to add the local dependency:**
```
module mcl_go_validation

go 1.22

require machine-config-go v0.0.0

// replace tells the Go toolchain to use the local directory instead of a
// module proxy. Use absolute path. On Windows use forward slashes.
replace machine-config-go => /absolute/path/to/repo/go
```
`go/go.mod` itself declares `go 1.22` (verified) — that's the module's minimum
language version, not a CI mismatch: `go.yml` installs toolchain `1.24.x` to build
it, which is fully compatible. Match the external project's directive to whatever
`go version` reports locally; it does not need to equal `1.22`.
Remove the `replace` directive and use a tagged version once the branch is merged.

**Step 3 — CGo environment setup.** `go/internal/h5c/h5c.go` is the entire HDF5
I/O layer (read + write) via CGo; its cgo directives are:
```c
#cgo linux pkg-config: hdf5
#cgo linux CFLAGS: -I/usr/include/hdf5/serial
#cgo linux LDFLAGS: -L/usr/lib/x86_64-linux-gnu/hdf5/serial -lhdf5
#cgo windows CFLAGS: -I/mingw64/include
#cgo windows LDFLAGS: -L/mingw64/lib -lhdf5
```

*Linux:*
```bash
sudo apt-get install -y libhdf5-dev pkg-config  # matches go.yml exactly
export CGO_ENABLED=1
go build ./...
```

*Windows — must run inside an actual MSYS2 MinGW64 shell, not PowerShell/Git-Bash,
because `/mingw64/include` and `/mingw64/lib` above are POSIX paths gcc resolves
against its own sysroot, not the Windows filesystem root* (verified against
`go.yml`'s working, CI-green recipe — reproduced here verbatim, no changes needed):
```bash
export CGO_ENABLED=1
export CC=$(cygpath -w /mingw64/bin/gcc.exe)
export CXX=$(cygpath -w /mingw64/bin/g++.exe)
go build ./...
```
On this dev machine specifically, MSYS2 is installed at `C:\msys64`, so launch
`C:\msys64\usr\bin\bash.exe --login` (or `C:\msys64\mingw64.exe`) first so `/mingw64/...`
resolves to `C:\msys64\mingw64\...` — `go.yml`'s `msys2/setup-msys2@v2` action sets up
the identical `/mingw64` root in CI, so the recipe is unchanged between local and CI.
Document the full environment setup in `results.md` under "Environment Setup" —
this is part of the consumer experience record.

**Step 4 — Verify S-09 type exports.** `go/models.go` re-exports the model surface via
Go type aliases (`type MachineConfig = internal/models.MachineConfig`, etc. — Go's
equivalent of Rust's `pub use`). Confirmed by reading `go/models.go` in full: **18
aliased types**, all resolvable from the module root with no `internal/` import:
`CorrectionData, MachineConfig, MachineConfigMeta, BuildPlate, Machine, OpticalTrain,
Scanner, AxisConfig, LightSource, Collimator, ScannerCard, OptionalComponents,
ClearBox, ScanFieldCorrectionFile, OpcuaConfig, OpcuaClientConfig, OpcuaPipeConfig,
OpcuaTrigger` — plus four scalar-pointer helpers (`StrPtr`, `Float64Ptr`, `IntPtr`,
`BoolPtr`). `MachineConfigReader`, `MachineConfigWriter`, `MockConfigBuilder`,
`ParseOptions`, and `UnsupportedFileVersionError` need **no alias at all** — `reader.go`,
`writer.go`, and `builder.go` already declare `package machineconfig` directly, so
they're already at the root. Verify all of the above import cleanly:
```go
import mc "machine-config-go"
var cfg *mc.MachineConfig
var scanner *mc.Scanner
var reader *mc.MachineConfigReader   // concrete struct, not an interface — see below
```
No public type currently requires `machine-config-go/internal/...` — S-09 has no
known gap.

> **Open question to resolve before writing S-09, not silently decided here:**
> `BuildPlate` (`go/internal/models/models.go`) is a real, aliased, exported type —
> but it is **never constructed or referenced anywhere** in `reader.go`, `writer.go`,
> `builder.go`, or any existing test. `Machine` carries build-plate dimensions as its
> own flat `BuildPlateX/Y/Z *float64` fields (matching Rust/C++/Node's convention), not
> a nested `BuildPlate` value — confirmed by reading `internal/models/models.go`'s
> `Machine` struct and every test that touches build-plate fields (e.g.
> `reader_test.go`'s `TestReadMachineBuildPlate`, which — despite its name — asserts
> against `cfg.Machine.BuildPlateX`, not a `BuildPlate` struct instance). It appears
> to be dead/vestigial code from an earlier design. Two options: (a) type-annotate it
> anyway in S-09 per this plan's own precedent (Rust's S-09 does exactly this for
> `OpcuaConfig`, a type the mock builder never populates — "construct OR
> type-annotate" is explicitly allowed, §8), or (b) flag `BuildPlate` for removal as
> dead code before writing S-09, shrinking the export checklist by one type. Pick one
> before implementing; don't let the app quietly paper over unused exported surface.

**Note on builder:** `go/builder.go`'s `MockConfigBuilder` is fully built out and
already has its own passing unit tests (`go/builder_test.go`) that hand S-06 its exact
expected values for free: 2 trains by default, rotations `0.0`/`180.0`, machine_name
`"MockMachine"`, a 257×257×2 Gaussian correction grid whose centre cell is `≈2.0`
(matching the Rust builder's documented formula exactly — `2*exp(-(x²+y²)/0.5)`), and
inverse-grid ratio `0.9`. S-06 is fully supported; no new logic needed, only the
validation-app scenario wrapper.

**Version dispatch mechanism — verified, matches Rust/C++, not Python/Node:**
`reader.go`'s `ParseWithOptions`/`GetCorrectionData`/`GetInverseCorrectionData` and
`writer.go`'s `Write` all dispatch via a hardcoded `switch fv { case "1.0": ...;
default: return &UnsupportedFileVersionError{...} }` — not a registry/map like
Python's `_ADAPTERS` or Node's `_READERS`/`_WRITERS`. `capabilities/file.go`'s
`OpenMachineConfig`/`CreateMachineConfig` and `SupportedFileVersions()` are the same:
a direct `if fv != v1_0.FileVersion` check and a literal `[]string{v1_0.FileVersion}`,
not a lookup table. This has the identical consequence already documented for Rust
in §9.3: **there is no dispatch-table entry to inject a mock `"1.1-mock"` adapter
into.**

**Error-type mapping for AV scenarios — verified against real code, not assumed:**
- `PeekFileVersion` (`go/file_version.go`) reads `File_Version` and: on read failure
  (attribute missing) → returns `"1.0", nil` (**no error** — silently defaults);
  after `strings.TrimSpace`, an empty string → also returns `"1.0", nil`. This means
  **AV-02 (absent version) and AV-07 (empty version) are already handled, by
  construction, as "treat as v1.0"** — not an error path at all. Confirm this is the
  intended cross-language behavior before writing the scenario (Rust's AV-02/AV-07
  titles say "handles ... predictably" / "handles empty string" without committing to
  error-vs-default; check `results.md` for the other three languages' actual choice
  and make sure Go's is being recorded as consistent, not silently divergent).
- Whitespace (AV-06): also handled inside `PeekFileVersion` via `strings.TrimSpace`
  before the empty-check — a whitespace-only `File_Version` normalizes to `"1.0"`
  the same way an empty one does.
- Unknown version, e.g. `"2.0"` (AV-01) / future version (AV-03): returns
  `*machineconfig.UnsupportedFileVersionError{Version: fv}` from the public reader,
  and `*capabilities.Error{Code: capabilities.ErrUnsupportedVersion}` from the
  stable-facade layer — both confirmed via `reader_test.go`'s
  `TestUnknownFileVersionDoesNotUseV1Layout`, which already exercises this exact path
  end-to-end (both layers) and can be lifted almost directly into the AV-01/AV-03
  scenario files.
- Missing required group (AV-04): `capabilities/v1_0/hdf5/hdf5.go`'s `parse()` calls
  `f.Group("Machine")`, which (per `h5c.go`) returns a **plain, untyped**
  `fmt.Errorf("H5Gopen2(%s) failed", path)` on failure — there is no
  `MissingGroupError` type. Assert `err != nil` with the message recorded, mirroring
  Rust's AV-04 (`Err(e) => (true, format!("... {e}"))`) rather than matching a
  specific error variant.
- Corrupt required scalar (AV-05): not yet read against `readRequiredStr`/
  `readFloatAttr` in `hdf5.go` — verify the exact failure mode (panic vs. wrapped
  `error`) before writing this scenario; do not assume it matches AV-04's shape.
- File_Version roundtrip fidelity (AV-08): trivial by construction —
  `writer.go` writes `cfg.Meta.FileVersion` (or defaults to `"1.0"` if blank) and
  `PeekFileVersion` reads it back verbatim after `TrimSpace`; should pass without new
  code, same as Rust's AV-08.

**Scenarios to execute:** S-01 through S-10, AV-01 through AV-08 via the standalone
app; AV-09 through AV-11 in `go/`'s own test tree (see below) — **not** the app, for
the same structural reason as Rust.

**AV-09–AV-11 (Go):** Mirror Rust's `rust/tests/mock_v1_1/` + `adapter_migration_test.rs`
design exactly, adapted to Go idiom — same "delegate-then-patch" shape as both Rust's
and Node's mocks: a `MockV1_1Reader`/`MockV1_1Writer` pair that calls the real, public
`v1_0hdf5.Parse`/`v1_0hdf5.Write` (already fully exported from
`machine-config-go/capabilities/v1_0/hdf5`, reachable exactly the way `reader.go`/
`writer.go` already reach them — no new exports needed) for every subcomponent that
doesn't change, then patches the same 10 documented differences. Put it in
`go/mock_v1_1_test.go` or a `go/internal/mockv11/` package imported only from
`_test.go` files — Go has no `tests/`-directory convention like Cargo; an internal
test-only package under `go/internal/` is invisible to real consumers, which is the
property that matters here, not file location. Since — as established above — Go's
dispatcher is a hardcoded `switch`, not a registry, this test calls
`MockV1_1Reader`/`MockV1_1Writer` **directly**, not through the public
`MachineConfigReader`/`MachineConfigWriter` facade, exactly like Rust. AV-09's actual
rationale ("adding v1.1 doesn't require modifying the v1.0 adapter") is satisfied by
zero edits to `capabilities/v1_0/` plus the full pre-existing suite (`go test ./...`)
staying green after the mock is added.
Define the mock's ADDITION fields (`facility_id`, `config_author` — confirmed absent
from `MachineConfigMeta` today) as typed `*string` fields on a **mock-only** meta
struct, not by stuffing them into the real `MachineConfigMeta.Extra map[string]any` —
matching how Rust/Node/Python keep the mock's typed fields separate from the real
model.
**Serialization safety (§8, AV-09):** Go's `encoding/json` is annotation-based like
Rust's serde, **not** enumerate-based like Python's manual dict construction — it
reflects over every exported struct field with a `json:"..."` tag and serializes it
unless the value is a nil/zero-value pointer with `omitempty`. Every existing optional
field in `internal/models/models.go` is already a pointer with `omitempty` (verified
by reading the struct definitions), so this is not a new risk *if* any future real
`facility_id`/`config_author` fields are added to `MachineConfigMeta` the same way —
but it must be verified explicitly for those two fields when/if they're ever added to
the real (non-mock) model, the same caution already flagged for Rust's serde.

**App structure (`docs/validation/go/app/`):**
```
main.go
scenarios/
  common.go
  scenarios.go
go.mod
```
Consolidated from one-file-per-scenario (19 files) down to one file per app
(§0 "condense per-scenario test files" decision, applied consistently across
all five languages) — `scenarios.go` holds all 17 `RunXxx` functions (S-01–09,
AV-01–08) in ID order, `common.go` keeps the shared helpers (fixture-path
resolution, hashing, NaN-aware equality) separate since that's infrastructure,
not scenario logic. `main.go` is unchanged by the merge — Go function
references are package-scoped, not file-scoped, so `main.go`'s
`[]struct{ id string; run runFn }` table needed zero edits. `main.go` prints
`[PASS]`/`[FAIL] <id>: <detail>` per scenario plus a final `N passed, M failed`
summary line, and exits non-zero on any failure. AV-01/02/04/05/06/07 read their
fixtures from the shared, language-agnostic `docs/validation/fixtures/` directory
(`v2_0_unknown.h5`, `missing_version.h5`, `missing_machine_group.h5`,
`corrupt_scalar.h5`, `version_whitespace.h5`, `empty_version.h5` — confirmed present,
generated by `docs/validation/fixtures/generate_fixtures.py`); AV-03 uses
`v1_1_simulated.h5` from the same directory.

**After app is green locally:** port into `docs/validation/go/app/`, commit.

**CI integration:** add to `go.yml`, after the existing `Test (Linux)`/`Test (Windows)`
steps (both already set up `CGO_ENABLED=1`/`CC` correctly — reuse that, don't
reintroduce a second HDF5 setup):
```yaml
- name: Run Go validation app (Linux)
  if: runner.os == 'Linux'
  run: |
    CGO_ENABLED=1 go run ./docs/validation/go/app/ \
      "$PWD/fixtures/" "$PWD/Reference Materials/" \
      2>&1 | tee "$RUNNER_TEMP/go_validation.txt"
  working-directory: .

- name: Run Go validation app (Windows)
  if: runner.os == 'Windows'
  shell: msys2 {0}
  run: |
    export CGO_ENABLED=1
    export CC=$(cygpath -w /mingw64/bin/gcc.exe)
    export CXX=$(cygpath -w /mingw64/bin/g++.exe)
    go run ./docs/validation/go/app/ \
      "$(cygpath -w "$PWD/fixtures/")" \
      "$(cygpath -w "$PWD/Reference Materials/")" \
      2>&1 | tee "$RUNNER_TEMP/go_validation.txt"
```
(Path fixed relative to repo root, matching how the existing `Test (Linux)`/`Test
(Windows)` steps in `go.yml` already `cd go` before running — the validation app step
should run from repo root instead, since `docs/validation/go/app/` is its own module
outside `go/`.)

---

### 9.5 C++

**Minimum requirements:** CMake 3.20+, C++17 compiler
(GCC 10+ / Clang 12+ / MSVC 2019+), HDF5 1.12+ development headers.

**Prerequisite — create the umbrella public header (it does not exist yet):**
Step 3 below and §10's tarball packaging both require `#include <machine_config/machine_config.hpp>`
to be the *only* include a consumer needs. That header does not exist in `cpp/include/machine_config/`
today — the real, currently-working convention (used by `cpp/src/main.cpp` and both
`examples/*/cpp/main.cpp`) is several separate includes (`reader.hpp`, `writer.hpp`,
`capabilities.hpp`, etc.). This is not stale documentation to fix; it is a header that must
be created — a thin file that `#include`s the existing public headers — before S-09 or the
tarball step can be attempted as written. Do this first, then update the CLI and both
examples to use it (a good, cheap regression check that it actually re-exports everything).

**Serialization safety (resolved, unlike the generic per-language note under AV-09):** C++
is safe by construction. `models.hpp` uses manual, explicit ADL `to_json`/`from_json`
functions (nlohmann's customization-point idiom) — e.g. `to_json(json&, const MachineConfigMeta&)`
lists each field by name — not an intrusive all-fields macro. Adding `facility_id`/
`config_author` to `MachineConfigMeta` will not leak into JSON output unless explicitly added
to that function, the same guarantee Python's hand-written `_config_to_dict()` gives. No
annotation or extra step is needed here, unlike Rust's required `#[serde(skip_serializing_if...)]`.

**No dispatch registry — same situation as Rust:** `MachineConfigReader::adapter()` in
`reader.hpp` is a single hardcoded `if (ver != "1.0") throw ...`, not a table, so there is no
seam to inject a mock adapter into the public `MachineConfigReader`/`MachineConfigWriter`
facade. AV-09–11 should call the mock adapter directly, exactly as decided for Rust in §9.3 —
this is stated explicitly here so it isn't rediscovered mid-implementation.

**No typed exception hierarchy:** every error path in `reader.hpp`/`writer.hpp`/
`capabilities/v1_0/hdf5.hpp` throws a bare `std::runtime_error` or `std::out_of_range` —
there is no C++ equivalent of Rust's `MachineConfigError` enum for the plain reader/writer
(the `CapabilityError` struct in `capabilities/errors.hpp` belongs only to the separate
stable-facade API). AV-01/03/04/05 will need to catch `std::exception` and inspect `.what()`
for the version string / field name, closer to Node's generic-`Error` situation than Rust's
typed variants — don't expect or add a typed hierarchy to make these scenarios pass.

**HighFive supports the delegate-then-patch mock design**, verified against the vendored
source at `cpp/build/_deps/highfive-src/include/highfive/` (already fetched by a prior
build), not assumed: `File::ReadWrite` (reopen read-write), `AnnotateTraits::deleteAttribute`/
`createAttribute`, and `NodeTraits::createGroup`/`getGroup` are all real, stable public API —
the same three operations the Node.js and Rust mocks needed, confirmed present here too.

**Mock adapter placement is simpler than both other languages:** the library is header-only
(`add_library(machine_config INTERFACE)`) and `tests/CMakeLists.txt` already compiles one
Catch2 binary from an explicit `.cpp` file list — there's no Rust-style flat-file-vs-subdirectory
registration gotcha, and no visibility barrier to design around, since every header under
`include/machine_config/` is already reachable from any test file. Add a `test_adapter_migration.cpp`
(and a `mock_v1_1.hpp` it includes) to that list; no CMake restructuring needed.

**Step 1 — Create external project and write `CMakeLists.txt` for from-source install:**
```bash
mkdir -p "/c/Users/ChrisParham/Desktop/Practice/machineconfiglibrarytesting/mcl_cpp_validation/src"
cd "/c/Users/ChrisParham/Desktop/Practice/machineconfiglibrarytesting/mcl_cpp_validation"
```
```cmake
cmake_minimum_required(VERSION 3.20)
project(mcl_cpp_validation CXX)
set(CMAKE_CXX_STANDARD 17)
find_package(HDF5 REQUIRED)
# Adjust to the absolute path of <repo>/cpp on your machine:
add_subdirectory("/absolute/path/to/repo/cpp" machine_config_build)
add_executable(validation_app src/main.cpp)
target_link_libraries(validation_app PRIVATE machine_config HDF5::HDF5)
```

**Step 2 — Build and run:**
```bash
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build
./build/validation_app "/absolute/path/to/fixtures/" "/absolute/path/to/Reference Materials/"
```

**Step 3 — Verify S-09 single-header compile test:**
The standalone app's `main.cpp` must only include the one public header:
```cpp
#include <machine_config/machine_config.hpp>  // only include allowed
```
Temporarily add `-Werror` to `CMakeLists.txt` and rebuild. If it compiles
clean, the public header is self-contained. If not, identify which types
require additional includes and record each as S-09 FAIL in `results.md`.

**Note on builder:** C++ has `MockConfigBuilder` in `cpp/include/machine_config/builder.hpp`
(header-only, like the rest of the library — there is no `cpp/src/builder.cpp`; `src/`
contains only `main.cpp`, the CLI). S-06 is fully supported.

**Type export verification:** The public header (`machine_config/machine_config.hpp`)
must expose all consumer-facing types — `MachineConfig`, `Scanner`, `OpticalTrain`,
etc. — without requiring `#include` of any internal header. Verify by writing the
standalone app with only `#include <machine_config/machine_config.hpp>` and confirming
it compiles cleanly. If any type requires an additional include, that is a public
API gap. Record in `results.md` under S-09. This check is run twice: once from
source, and once from the static tarball (§8 Step 4).

**Scenarios to execute:** S-01 through S-10, AV-01 through AV-11

**AV-09–AV-11 (C++):** Implement the mock v1.1 adapter as a test-only header
(`tests/mock_v1_1.hpp`) plus a `test_adapter_migration.cpp` added to `tests/CMakeLists.txt`'s
source list, calling the mock directly rather than through `MachineConfigReader`/
`MachineConfigWriter` (see "No dispatch registry" above). Mirror the delegate-then-patch
design already verified for Node.js and Rust: delegate to the real v1.0 adapter/writer for
everything unchanged, then patch the 10 documented differences via HighFive's own
`deleteAttribute`/`createGroup`/`createAttribute` (verified above). The serialization-safety
question is already resolved above — C++ is safe by construction, no annotation needed.

**App structure (`docs/validation/cpp/app/`):**
```
CMakeLists.txt
src/
  main.cpp
  scenarios/
    common.hpp
    scenarios.hpp
    scenarios.cpp
```
Consolidated from one-file-per-scenario (36 files: 17 `.hpp`/`.cpp` pairs +
`common.hpp` + `main.cpp`) down to one `.hpp`/`.cpp` pair per app (§0
"condense per-scenario test files" decision, applied consistently across all
five languages) — C++ saw the largest cut of the five since it was the only
language paying the header/source split tax per scenario on top of the
per-scenario file split itself. Each scenario keeps its own namespace
(`s01`, `av05`, etc.) inside the merged files, exactly as before — this
means `main.cpp`'s `scenarioList` (`{"S-01", s01::run}`, etc.) needed **zero**
changes, only its 18 `#include "scenarios/sXX_....hpp"` lines collapsing to
one `#include "scenarios/scenarios.hpp"`. `CMakeLists.txt`'s
`add_executable(validation_app ...)` source list shrank from 18 `.cpp` files
to 2 (`main.cpp`, `scenarios/scenarios.cpp`).

Rebuilding after the merge surfaced a real, pre-existing bug unrelated to the
consolidation itself: `common.hpp`'s `avFixture()` used
`fixturesDir.parent_path()`, which has the exact same trailing-separator
quirk as Go's `filepath.Dir()` (§9.4) — a `fixturesDir` ending in `/` makes
the final path component empty, so `parent_path()` has nothing to "drop" and
returns the path unchanged, breaking every AV fixture lookup. Fixed with the
same technique as Go: `(fixturesDir / "..").lexically_normal()` instead of
`.parent_path()`, which resolves `..` correctly regardless of a trailing
separator. `common.hpp` itself was otherwise untouched — verified via `git
diff` before attributing the bug to something other than the merge.

**After app is green from source:** proceed to §8 (static tarball).
After tarball is verified, rebuild the app against the tarball and record
that in `results.md` as a separate "tarball consumer" run.

**CI integration:** add to `cpp.yml`:
```yaml
- name: Run C++ validation app
  run: |
    cmake -S docs/validation/cpp/app -B docs/validation/cpp/app/build \
      -DCMAKE_PREFIX_PATH=<tarball path>
    cmake --build docs/validation/cpp/app/build
    ./docs/validation/cpp/app/build/validation_app
```

---

## 10. C++ Packaging (thin, vcpkg-oriented — supersedes the original "static tarball" design)

**When:** After the C++ standalone app is green from source (§9.5 done-gate met).

> **Design change, recorded here rather than silently overwritten:** this section originally
> described a fully self-contained static tarball — headers, a static `.a`/`.lib`, and a
> bundled copy of HDF5 itself, so a consumer needs nothing else installed. That design was
> abandoned in favor of the "thin" one below. Full reasoning, the two real CMake errors hit
> while building it, and the verbatim fresh-consumer proof are all in
> `docs/validation/cpp/tarball/results.md` — read that first if anything below is unclear.
> Short version: the reason a C++ package is wanted at all is the eventual possibility of a
> vcpkg port, and vcpkg ports *declare* their dependencies rather than bundling them — a
> self-contained bundle would fight that model, not prepare for it.

**Goal (revised):** `machine_config`'s own headers, installed, plus a generated
`MachineConfigConfig.cmake` that asks a consumer's CMake to `find_dependency(HighFive)` and
`find_dependency(nlohmann_json)` (HDF5 resolves transitively through HighFive's own config) —
**not** a tarball that bundles those dependencies' files. A consumer still needs HighFive,
nlohmann_json, and HDF5 separately discoverable (today: built/installed by hand; eventually:
a vcpkg environment). That is intentional, not a gap to close later.

### Step 1 — Add install()/export infrastructure to `cpp/CMakeLists.txt`

No HDF5-specific build step is needed for this design (HDF5 is `find_dependency`'d, not
bundled). What's needed instead, all inside `cpp/CMakeLists.txt`:

- `set(JSON_Install ON CACHE BOOL "" FORCE)` before `FetchContent_MakeAvailable(nlohmann_json)`
  — its own `install()`/export rules default off when it's a sub-project, not the top-level
  CMake project (`${MAIN_PROJECT}` check).
- `target_include_directories(machine_config INTERFACE ...)` must use the
  `$<BUILD_INTERFACE:...>` / `$<INSTALL_INTERFACE:...>` generator-expression pair, not a bare
  relative path — the bare form works for this repo's own build but `install(EXPORT ...)`
  rejects it (real error hit; see results.md).
- `install(TARGETS machine_config EXPORT MachineConfigTargets)`
- `install(DIRECTORY include/machine_config DESTINATION include)`
- `install(EXPORT MachineConfigTargets FILE MachineConfigTargets.cmake NAMESPACE MachineConfig:: DESTINATION lib/cmake/MachineConfig)`
- `configure_package_config_file(cmake/MachineConfigConfig.cmake.in ...)` +
  `write_basic_package_version_file(...)` (both via `CMakePackageConfigHelpers`), each
  installed to `lib/cmake/MachineConfig`.

`cmake/MachineConfigConfig.cmake.in`:
```cmake
@PACKAGE_INIT@
include(CMakeFindDependencyMacro)
find_dependency(HighFive)
find_dependency(nlohmann_json)
include("${CMAKE_CURRENT_LIST_DIR}/MachineConfigTargets.cmake")
```

### Step 2 — Build and install

```bash
cmake -S cpp -B cpp/pkg-build -DCMAKE_INSTALL_PREFIX=cpp/pkg-install
cmake --install cpp/pkg-build
```

No `--target machine_config` build step is needed (or possible) — it's `INTERFACE`-only, so
there is nothing to compile; only the install step matters.

### Step 3 — Package (deferred; not needed for local verification)

A literal tarball (`tar -czf ...` around the install prefix) is straightforward once Step 2's
output exists, but wasn't produced in this pass — the fresh-consumer test in Step 4 pointed
`CMAKE_PREFIX_PATH` directly at the install prefix, which is sufficient to prove the packaging
works. Produce the actual archive when wiring up Step 5's CI job.

**On "public headers only":** unlike the original design, this is largely a non-issue here —
this library is header-only with no enforced internal/public split beyond the
`machine_config.hpp` umbrella-header convention (see §9.5), so installing all of
`include/machine_config/` verbatim is correct; there is no `internal/` subdirectory to exclude.

### Step 4 — Verify locally as a genuinely fresh external consumer

```bash
# A separate location, no add_subdirectory, no reference to this repo's source tree.
# (Distinct from the from-source app used for S-01–09/AV-01–08, which does use
# add_subdirectory — this one proves the installed package doesn't secretly need it.)
cd .../machineconfiglibrarytesting/CppTarballConsumer
cmake -S . -B build -DCMAKE_PREFIX_PATH=<path to cpp/pkg-install>
cmake --build build
./build/consumer_app fixtures/reference_config.h5
```

CMakeLists.txt for that consumer is exactly:
```cmake
find_package(MachineConfig REQUIRED)
add_executable(consumer_app src/main.cpp)
target_link_libraries(consumer_app PRIVATE MachineConfig::machine_config)
```

**Recorded, verbatim, in `docs/validation/cpp/tarball/results.md`** — configure output, the
one real fix required, and the final `[PASS]` run against the real reference fixture.

### Step 5 — CI verification: done. CD artifact production: done.

**Verification (every push/PR).** `cpp.yml`'s existing `test` job installs `machine_config`
from that job's own already-configured `cpp/build` — no separate HDF5/HighFive/nlohmann_json
resolution needed, since that build already resolved them per-platform (vcpkg on Windows,
from-source on Ubuntu) a few steps earlier — then configures, builds, and runs the ported
fresh-consumer check (`docs/validation/cpp/tarball/consumer/`) against it, failing the job on
anything but a `[PASS]` whose reported version also matches `cpp/cmake/Version.cmake` (added
once `version.hpp`/`kVersionString` existed — see below). Verified locally against the real
`cpp/build` directory specifically, using the exact commands the YAML runs.

**Artifact production (release only).** `.releaserc.json`'s `publishCmd` now also runs
`scripts/package-cpp.sh` (after Node's `npm pack` / Python's `python -m build`), producing
`cpp/machine-config-cpp-<version>.tar.gz`, uploaded via `@semantic-release/github`'s `assets`.
`release.yml` gained the same HDF5-from-source build+cache `cpp.yml`'s Ubuntu leg already has
(the `release` job had zero C++ setup before this). Deliberately **not** failure-guarded — a
broken C++ package blocks the whole release exactly the way a broken Node/Python package
already does today (checked: their existing `publishCmd` has never had a guard either). This
was a real decision, not a default: semantic-release's own `publish` lifecycle step is the one
step that does *not* set `settleAll: true` (verified by reading
`lib/definitions/plugins.js`/`lib/plugins/pipeline.js` directly), so a thrown error there skips
every later plugin in the same run — `git` and `github` are both listed after `exec` in the
plugins array. Chose loud failure over swallowing it, on request.

A real correctness bug was found and fixed before any of this could work at all: writing the
repo's actual current version (`0.2.0-rc.4`) into `project(... VERSION ...)` would have hard-
errored on every future `cmake` configure — CMake's `VERSION` field only accepts numeric
components, confirmed by direct test. Fixed by truncating for that field while preserving the
full version separately via a new `cpp/cmake/Version.cmake` → `configure_file()` →
`include/machine_config/version.hpp` (`machine_config::kVersionString`) → a new
`machine_config_cli --version` flag. Full account, including why CMake's *comparison*
operators (not just its parser) have no prerelease-ordering concept either — checked directly
in CMake's own `BasicConfigVersion-AnyNewerVersion.cmake.in` — is in
`docs/validation/cpp/tarball/results.md`.

**What could not be verified locally:** an actual `npx semantic-release` run (needs a real git
tag/token/release context) and the Ubuntu-CI HDF5-from-source path specifically (this machine
resolves HDF5 via the HDF Group's own installer, not a from-source build). Both disclosed, not
hidden — the first real signal is the next push to `main`, which releases as an `rc.N`
prerelease per `.releaserc.json`'s branch config, not a distant `release`-branch-only event.

---

## 11. CI Summary Plumbing

**Deferred** — implement after all validation scenarios are written and passing.

At that point, observe the actual CI output volume and decide:

**Option A (minimal):** Pipe cross_check output to `$GITHUB_STEP_SUMMARY`:
```yaml
- name: Phase N — ...
  shell: bash
  run: python tools/cross_check.py ... --verbose | tee -a "$GITHUB_STEP_SUMMARY"
```

**Option B (structured):** Switch Rust to `cargo-nextest`, Go to `gotestsum`,
emit JUnit XML from all runners, aggregate with `test-summary/action`.

Decision gate: if the plain text summary is readable and useful after the full
test suite is written, stay with Option A. If the output is too noisy to scan,
implement Option B.

---

## 12. Portability Notes — Build Log Library

The Build Log Library follows the same monorepo structure (5 languages, same
adapter pattern, same HDF5 backend). To port this validation plan:

**Keep unchanged:**
- `docs/validation/` folder structure (identical)
- All AV-0N scenario definitions (version dispatch is language/format agnostic)
- Per-language app structure and scenario file naming
- `results.md` and `PASS_FAIL.md` templates
- C++ static tarball process (§8)
- CI summary plumbing approach (§9)

**Replace with Build Log equivalents:**
- All S-0N scenario field values (different HDF5 schema, different field names)
- Fixture files (Build Log fixtures, not Machine Config fixtures)
- Real-world file path in S-03 (Build Log file from actual machine)
- S-08 "drastic change" field list (Build Log-specific structural changes)
- Install artifact paths (Build Log wheel/tarball names)

**Template files to copy verbatim and fill in:**
- `VALIDATION_PLAN.md` (this document — replace §5, §6 field values)
- `docs/validation/fixtures/README.md`
- Per-language `PASS_FAIL.md` (same table structure)
- Per-language `results.md` (same entry format)

---

## 13. Master Completion Checklist

### Fixture preparation
- [ ] `docs/validation/fixtures/generate_fixtures.py` written and committed
- [ ] All 7 AV fixtures generated and committed (verify with h5py check command in §5)
- [ ] `docs/validation/fixtures/README.md` written (documents each fixture's transformation)

### Python
- [x] Standalone app written externally and verified
- [x] App ported to `docs/validation/python/app/`
- [x] All S-01–S-09 scenarios recorded in `docs/validation/python/results.md`
- [x] All AV-01–AV-08 scenarios recorded in `docs/validation/python/results.md`
- [x] AV-09–AV-11: mock v1.1 adapter implemented and passing (`python/tests/test_adapter_migration.py`, 8 tests)
- [x] AV-09–AV-11 recorded in `docs/validation/python/results.md`
- [x] S-09: `py.typed` marker confirmed present in installed wheel
- [x] S-09: all types importable from `machine_config` top-level (no internal paths)
- [x] `docs/validation/python/PASS_FAIL.md` complete
- [x] `docs/validation/README.md` master summary and scenario matrix updated for Python
- [x] Validation status cross-linked from `docs/python.md`
- [ ] CI integration added to `python.yml`

### Node.js
- [x] Standalone app written externally and verified (`C:\Users\ChrisParham\Desktop\Practice\machineconfiglibrarytesting\node`) for S-01–09/AV-01–08
- [x] App ported to `docs/validation/nodejs/app/` (AV-09–11 were written directly in-repo — see note below)
- [x] All S-01–S-09 and AV-01–AV-08 scenarios recorded in `docs/validation/nodejs/results.md`
- [x] AV-09–AV-11: mock v1.1 adapter implemented in `nodejs/tests/mockV1_1.ts` and passing
- [x] AV-09–AV-11 recorded in `docs/validation/nodejs/results.md`
- [x] S-09: `package.json` `types` field confirmed; `.d.ts` files in tarball
- [x] S-09: all types importable from package root (no internal paths)
- [x] `docs/validation/nodejs/PASS_FAIL.md` complete — 20/20
- [x] `docs/validation/README.md` master summary and scenario matrix updated for Node.js
- [x] Validation status cross-linked from `docs/nodejs.md`
- [ ] CI integration added to `nodejs.yml`

**Bugs found and fixed during this pass (see `docs/validation/nodejs/results.md` for full detail):**
- AV-05 caught a real defect: `attrFloat()` in `capabilities/v1_0/hdf5.ts` silently
  returned `null` for a corrupt/non-numeric attribute instead of throwing, unlike
  Python's `ValueError` for the same input. Fixed to throw `TypeError`; full
  Node.js suite reconfirmed green after each subsequent change (151 tests as of
  the AV-09–11 additions), validation app reconfirmed passing throughout.
- Building the AV-09–11 mock surfaced a second real defect: the mock reader's
  `facility_id`/`config_author` ended up duplicated into `meta.extra` (the base
  v1.0 parser's `KNOWN_ROOT` set doesn't know they're typed fields), causing a
  double-write collision on the next write. Fixed by stripping those keys from
  `extra` before setting the typed fields — the exact serialization-safety
  failure mode flagged earlier in this plan, confirmed real, not theoretical.

**Note on AV-09–11 placement:** unlike S-01–09/AV-01–08, these three scenarios
were written directly in `docs/validation/nodejs/app/scenarios/` rather than
being developed externally first and ported in. They import the mock adapter
from `nodejs/tests/mockV1_1.ts` at a fixed relative path — that only works when
the scenario file and the mock live in the same repo checkout (mirroring
exactly how Python's AV-09–11 reach `python/tests/test_adapter_migration.py`),
so there was never an externally-runnable version of these three. The app's
`package.json` (in `docs/validation/nodejs/app/`) depends on the local package
via `file:../../../../nodejs` and is run with `npx tsx main.mts <fixtures> <real>`
— `tsx` is required (not plain `node`) because the real adapter classes use
TypeScript's constructor-parameter-property shorthand, which Node's native
type-stripping cannot handle.

### Rust
- [x] Standalone app written externally and verified (`C:\Users\ChrisParham\Desktop\Practice\machineconfiglibrarytesting\Rust`) for S-01–09/AV-01–08
- [x] App ported to `docs/validation/rust/app/` (its own standalone Cargo workspace — see results.md; AV-09–11 cannot live here at all, not just "were written directly in-repo" as with Node.js — see below)
- [x] All S-01–S-09 and AV-01–AV-08 scenarios recorded in `docs/validation/rust/results.md`
- [x] AV-09–AV-11: mock v1.1 adapter implemented in `rust/tests/mock_v1_1/mod.rs` and passing (`rust/tests/adapter_migration_test.rs`, 5 tests)
- [x] AV-09–AV-11 recorded in `docs/validation/rust/results.md`
- [x] S-09: all public types re-exported from crate root (no sub-module paths needed) — this was a real gap closed during this pass, not already true; see `rust/src/lib.rs`
- [x] `docs/validation/rust/PASS_FAIL.md` complete — 20/20
- [x] `docs/validation/README.md` master summary and scenario matrix updated for Rust
- [x] Validation status cross-linked from `docs/rust.md`
- [ ] CI integration added to `rust.yml`

**Bugs found and fixed during this pass (see `docs/validation/rust/results.md` for full detail):**
- S-09 revealed a real, pre-existing library gap: no public type (`MachineConfig`, `Scanner`,
  `MockConfigBuilder`, etc.) was re-exported from the crate root — consumers needed
  `machine_config::models::MachineConfig` instead of `machine_config::MachineConfig`. Fixed by
  adding `pub use` re-exports to `rust/src/lib.rs`, matching Python's `__init__.py` and Node's
  `index.ts`. Unlike Node.js, no runtime/serialization defect was found — AV-04/AV-05 already
  returned typed errors from the start, and the mock's `meta.extra` handling was written
  correctly on the first pass (the equivalent bug had already surfaced in Node's pass earlier
  in this effort).

**Note on AV-09–11 placement:** unlike Node.js, where these three scenarios were merely
*written* directly in-repo rather than developed externally first, Rust's AV-09–11 cannot
exist in the external app at all, at any point — Rust's dispatcher is a hardcoded `match` in
`reader.rs`/`writer.rs`, not a registry, so there is no dispatch-table injection seam for a
mock adapter to hook into the public `MachineConfigReader`/`Writer` facade. The mock
(`rust/tests/mock_v1_1/mod.rs`) is exercised directly by `rust/tests/adapter_migration_test.rs`
via `cargo test`, never through the external app. This was a deliberate decision documented
in §9.3 before implementation began, not a gap discovered mid-work.

### Go
- [x] Standalone app written externally and verified (`C:\Users\ChrisParham\Desktop\Practice\machineconfiglibrarytesting\Go`) for S-01–09/AV-01–08
- [x] App ported to `docs/validation/go/app/` (its own standalone Go module, `replace machine-config-go => ../../../../go`)
- [x] All S-01–S-09 and AV-01–AV-08 scenarios recorded in `docs/validation/go/results.md`
- [x] AV-09–AV-11: mock v1.1 adapter implemented in `go/internal/mockv1_1/` + `go/mock_v1_1_test.go` and passing (5 tests)
- [x] AV-09–AV-11 recorded in `docs/validation/go/results.md`
- [x] S-09: all exported types accessible via top-level package (no internal imports) — already true, no gap found
- [x] `docs/validation/go/PASS_FAIL.md` complete — 20/20
- [x] `docs/validation/README.md` master summary and scenario matrix updated for Go
- [x] Validation status cross-linked from `docs/go.md`
- [ ] CI integration added to `go.yml` (consistent with Python/Node/Rust/C++ — none of the five have this yet; not Go-specific)

**Production code changes made during this pass (see `docs/validation/go/results.md` for full detail):**
- `go/internal/h5c/h5c.go` gained two new primitives — `OpenRW` (open existing file
  read/write without truncating) and `(g *Group) DeleteAttr` — needed for the mock v1.1
  writer's delegate-then-patch design. Rust's/Node's underlying HDF5 libraries already
  expose equivalent operations; Go's `internal/h5c` is the project's own minimal CGo
  wrapper, so these had to be added. Raised explicitly with the user before implementing
  (the alternative — building the mock file from scratch instead of patching a real
  v1.0-written file — was rejected because it would stop exercising the real v1.0 writer
  for unchanged subcomponents, undermining AV-09's actual point).
- `go/internal/models/models.go`'s `MachineConfigMeta` gained two new `omitempty` pointer
  fields, `FacilityID`/`ConfigAuthor`, explicitly marked test-fixture-only and never
  populated by the real v1.0 read path — mirroring the identical fields already present in
  `python/src/machine_config/models.py` and `rust/src/models.rs`.

**Note on AV-09–11 placement:** same structural reason as Rust — Go's dispatcher
(`reader.go`/`writer.go`/`capabilities/file.go`) is a hardcoded `switch`, not a registry, so
there is no dispatch-table injection seam for a mock adapter to hook into the public
`MachineConfigReader`/`Writer` facade. The mock (`go/internal/mockv1_1/`) is exercised
directly by `go/mock_v1_1_test.go` via `go test`, never through the external app. This
mirrors §9.4's design section, written and confirmed before implementation began.

**Open item, deliberately left unresolved:** `BuildPlate` (`go/internal/models/models.go`)
is exported and aliased at the module root but never constructed anywhere in the real
codebase. S-09 type-annotates it without constructing it (matching Rust's own precedent for
`OpcuaConfig`). Whether to keep it as reserved public surface or remove it as dead code was
explicitly raised with the user, who chose to flag it and decide later rather than resolve
it as a side effect of this pass.

**Note on `docs/go.md` cross-link:** now resolved — `## Running the Go test suite` and
`## Validation results` sections added to `docs/go.md` in this pass.

### C++
- [x] Standalone app written externally and verified (from source) (`C:\Users\ChrisParham\Desktop\Practice\machineconfiglibrarytesting\Cpp`)
- [x] App ported to `docs/validation/cpp/app/` (its own standalone CMake project, `add_subdirectory`-consuming `cpp/`)
- [x] All S-01–S-09 and AV-01–AV-08 scenarios recorded (from-source run) in `docs/validation/cpp/results.md`
- [x] AV-09–AV-11: mock v1.1 adapter implemented in `cpp/tests/mock_v1_1.hpp`, test-only, and passing (`cpp/tests/test_adapter_migration.cpp`, 5 tests)
- [x] AV-09–AV-11 recorded in `docs/validation/cpp/results.md`
- [x] S-09: standalone app compiles with only `#include <machine_config/machine_config.hpp>` — this required *creating* that header; it did not exist before this pass
- [x] C++ package built and verified locally as a fresh external consumer — **design changed
      from §10's original self-contained static tarball to a "thin", `find_package`-based
      package** (see `docs/validation/cpp/tarball/results.md` for full reasoning and the two
      real CMake errors hit while building it)
- [x] S-09 (no internal headers): non-issue for this design — header-only with no
      internal/public split beyond the `machine_config.hpp` umbrella-header convention, so
      installing `include/machine_config/` verbatim is correct by construction
- [x] Consumer run recorded, verbatim, in `docs/validation/cpp/tarball/results.md`
- [x] CI *verification* step added to `cpp.yml` — installs `machine_config` from that job's
      already-configured `cpp/build` (no separate HDF5/vcpkg resolution needed), builds and
      runs the ported consumer, fails the job if it doesn't print `[PASS]`. Fires on every
      push/PR; produces/persists nothing. Verified locally beforehand with the exact commands
      the YAML runs, against the real `cpp/build` dev directory specifically (not just the
      earlier `cpp/pkg-build` proof) — see `docs/validation/cpp/tarball/results.md`.
- [x] A real, distributable artifact: `.releaserc.json`'s `publishCmd` runs
      `scripts/package-cpp.sh`, producing `cpp/machine-config-cpp-<version>.tar.gz`, uploaded
      via `@semantic-release/github`'s `assets`. `release.yml` gained the HDF5 setup this
      needed (previously zero C++ awareness). Deliberately not failure-guarded — matches
      Node/Python's existing, equally-unguarded `publishCmd` steps, per explicit decision.
- [x] Literal tarball archive (`tar -czf`) produced — built and verified locally end-to-end
      (extracted, confirmed the correct version baked into the packaged `version.hpp`); the
      real `npx semantic-release` run itself is the one thing that couldn't be exercised
      locally (needs a real tag/token/release context) — first real signal is the next push
      to `main`
- [x] `docs/validation/cpp/PASS_FAIL.md` complete — 20/20
- [x] `docs/validation/README.md` master summary and scenario matrix updated for C++
- [x] Validation status cross-linked from `docs/cpp.md`

**Bugs found and fixed during this pass (see `docs/validation/cpp/results.md` for full detail):**
- No umbrella public header (`machine_config/machine_config.hpp`) existed, even though §8 S-09
  and §10 both require it — created it, and wired the CLI and both examples to it as a
  regression check.
- `cpp/CMakeLists.txt` and `cpp/tests/CMakeLists.txt` used `CMAKE_SOURCE_DIR` (top of the
  whole CMake project tree) where `PROJECT_SOURCE_DIR` (anchored to this library's own
  `project()` call) was needed — it broke the instant `cpp/` was consumed via
  `add_subdirectory` from an external project, exactly what §9.5 Step 1's own template does.
  Fixed; caught a subtly-wrong first attempt (`CMAKE_CURRENT_SOURCE_DIR`, wrong specifically
  inside `tests/CMakeLists.txt`) by re-running the full suite rather than trusting the fix —
  it had silently broken 57 of 74 test cases.
- No runtime/serialization defects were found, unlike Node.js's pass (which found two: an
  `attrFloat` silent-null and a `meta.extra` double-write). This library's error paths and
  the mock's `meta.extra` handling were already correct.
- **Packaging pass (§10, separate from the above):** `cpp/CMakeLists.txt` had zero `install()`
  rules of any kind before this — the entire packaging story §10 assumed existed had never
  been built. While building it: `target_include_directories(machine_config INTERFACE include)`
  used a bare relative path, which `install(EXPORT ...)` rejects (real error, real fix — the
  same `$<BUILD_INTERFACE:...>`/`$<INSTALL_INTERFACE:...>` pattern HighFive's own CMakeLists.txt
  already uses); and `nlohmann_json`'s `JSON_Install` option defaults off as a sub-project,
  silently skipping its own export setup until forced on. Full account in
  `docs/validation/cpp/tarball/results.md`.

### Documentation
- [ ] `docs/validation/README.md` master summary table complete (final check —
      should already be true if each language's own checklist above was kept
      current; this is a re-verification, not the first time it's touched)
- [ ] All five `docs/<lang>.md` files cross-link their validation status
- [ ] All FAIL verdicts have associated explanation or issue reference

### Merge gate
- [ ] All five languages: done-gate met (all scenarios recorded, all verdicts entered)
- [ ] CI green on both platforms for all workflows
- [ ] Real AconityMIDI file validated end-to-end in all five languages (S-03, S-08)
- [ ] `docs/validation/README.md` shows all ✅ in master table
