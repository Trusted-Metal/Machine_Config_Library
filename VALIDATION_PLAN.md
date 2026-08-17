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
- [ ] C++ static tarball verified locally and CI packaging step added
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
2. Define a list of scenario functions in run order (S-01..S-09, AV-01..AV-08)
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
            be able to use all types via `#include <machine_config/machine_config.h>`
            with no other include paths needed.
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
mkdir ~/mcl_python_validation && cd ~/mcl_python_validation
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

**Scenarios to execute:** S-01 through S-09, AV-01 through AV-11

**AV-09–AV-11 status (Python):** ✅ Complete — implemented in `python/tests/test_adapter_migration.py`
as `MockV1_1Layout`, `MockV1_1Reader`, `MockV1_1Writer` (8 tests, all passing).
The validation app scenarios for AV-09–AV-11 are thin wrappers that invoke the same logic.

**App structure (`docs/validation/python/app/`):**
```
main.py          ← runs all scenarios sequentially, prints results
scenarios/
  s01_read_scalars.py
  s02_read_binary.py
  s03_read_real.py
  s04_write_modify.py
  s05_binary_roundtrip.py
  s06_builder.py
  s07_opcua.py
  s08_drastic_change.py
  av01_unknown_version.py
  av02_missing_version.py
  av03_future_version.py
  av04_missing_group.py
  av05_corrupt_scalar.py
  av06_whitespace_version.py
  av07_empty_version.py
  av08_version_fidelity.py
  av09_mock_adapter_isolation.py
  av10_forward_migration.py
  av11_backward_migration.py
  s09_type_exports.py
requirements.txt ← pinned to exact library version tested
```

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
mkdir ~/mcl_nodejs_validation && cd ~/mcl_nodejs_validation
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

**Scenarios to execute:** S-01 through S-09, AV-01 through AV-11

**AV-09–AV-11 (Node.js):** Implement `MockV1_1Layout`, `MockV1_1Reader`, `MockV1_1Writer`
in `nodejs/tests/` following the same 10 HDF5 changes in `docs/migrations/mock_v1_0_to_v1_1.md`.
Use Vitest. StableModel field names follow TypeScript camelCase convention.

**App structure (`docs/validation/nodejs/app/`):**
```
main.mts
scenarios/
  s01_read_scalars.mts
  s02_read_binary.mts
  ... (same pattern as Python)
  s09_type_exports.mts
package.json
tsconfig.json
```

**After app is green locally:** port into `docs/validation/nodejs/app/`, commit.

**CI integration:** add to `nodejs.yml`:
```yaml
- name: Run Node.js validation app
  shell: bash
  run: |
    npm ci && npm run build
    node docs/validation/nodejs/app/dist/main.js \
      "$PWD/fixtures/" "$PWD/Reference Materials/" \
      2>&1 | tee "$RUNNER_TEMP/nodejs_validation.txt"
  working-directory: nodejs
```

---

### 9.3 Rust

**Step 1 — Create external project:**
```bash
mkdir ~/mcl_rust_validation && cd ~/mcl_rust_validation
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

**Scenarios to execute:** S-01 through S-09, AV-01 through AV-11

**AV-09–AV-11 (Rust):** Implement mock v1.1 adapter in a `#[cfg(test)]` module in
`rust/tests/` following the same 10 HDF5 changes. Use `MockV1_1Layout` constants,
`MockV1_1Reader`, `MockV1_1Writer` structs. Verify adapter trait satisfaction.

**App structure (`docs/validation/rust/app/`):**
```
src/
  main.rs
  scenarios/
    mod.rs
    s01_read_scalars.rs
    ... (same pattern)
    s09_type_exports.rs
Cargo.toml
```

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

**Step 1 — Create external project:**
```bash
mkdir ~/mcl_go_validation && cd ~/mcl_go_validation
go mod init mcl_go_validation
```

**Step 2 — Edit `go.mod` to add the local dependency:**
```
module mcl_go_validation

go 1.24

require machine-config-go v0.0.0

// replace tells the Go toolchain to use the local directory instead of a
// module proxy. Use absolute path. On Windows use forward slashes.
replace machine-config-go => /absolute/path/to/repo/go
```
Remove the `replace` directive and use a tagged version once the branch is merged.

**Step 3 — CGo environment setup:**

*Linux:*
```bash
sudo apt-get install -y libhdf5-dev  # Ubuntu/Debian
export CGO_ENABLED=1
go build ./...
```

*Windows (MSYS2 MinGW64 shell — NOT PowerShell):*
```bash
export MSYSTEM=MINGW64
export PATH="/mingw64/bin:/c/Program Files/Go/bin:$PATH"
export CGO_ENABLED=1
export CC=$(cygpath -w /mingw64/bin/gcc.exe)
go build ./...
```
Document the full environment setup in `results.md` under "Environment Setup" —
this is part of the consumer experience record.

**Step 4 — Verify S-09 type exports:**
```go
import mc "machine-config-go"
// All consumer-facing types must be accessible here.
// Never import machine-config-go/internal/... in the standalone app.
var cfg *mc.MachineConfig
var scanner *mc.Scanner
```
If a type requires `machine-config-go/internal/models`, that is a public API gap.

**Note on builder:** Go has `MockConfigBuilder` in `go/builder.go`.
S-06 is fully supported.

**Scenarios to execute:** S-01 through S-09, AV-01 through AV-11

**AV-09–AV-11 (Go):** Implement mock v1.1 adapter in `go/`'s `_test.go` files following
the same 10 HDF5 changes. Verify the adapter satisfies the reader/writer interfaces.

**App structure (`docs/validation/go/app/`):**
```
main.go
scenarios/
  s01_read_scalars.go
  s02_read_binary.go
  ... (same pattern)
  s09_type_exports.go
go.mod
go.sum
```

**After app is green locally:** port into `docs/validation/go/app/`, commit.

**CI integration:** add to `go.yml`:
```yaml
- name: Run Go validation app (Linux)
  if: runner.os == 'Linux'
  shell: bash
  run: |
    CGO_ENABLED=1 go run ./docs/validation/go/app/ \
      "$PWD/fixtures/" "$PWD/Reference Materials/" \
      2>&1 | tee "$RUNNER_TEMP/go_validation.txt"

- name: Run Go validation app (Windows)
  if: runner.os == 'Windows'
  shell: msys2 {0}
  run: |
    export CGO_ENABLED=1
    export CC=$(cygpath -w /mingw64/bin/gcc.exe)
    go run ./docs/validation/go/app/ \
      "$(cygpath -w "$PWD/fixtures/")" \
      "$(cygpath -w "$PWD/Reference Materials/")" \
      2>&1 | tee "$RUNNER_TEMP/go_validation.txt"
```

---

### 9.5 C++

**Minimum requirements:** CMake 3.20+, C++17 compiler
(GCC 10+ / Clang 12+ / MSVC 2019+), HDF5 1.12+ development headers.

**Step 1 — Write `CMakeLists.txt` for from-source install:**
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
#include <machine_config/machine_config.h>  // only include allowed
```
Temporarily add `-Werror` to `CMakeLists.txt` and rebuild. If it compiles
clean, the public header is self-contained. If not, identify which types
require additional includes and record each as S-09 FAIL in `results.md`.

**Note on builder:** C++ has `MockConfigBuilder` in `cpp/src/builder.cpp`.
S-06 is fully supported.

**Type export verification:** The public header (`machine_config/machine_config.h`)
must expose all consumer-facing types — `MachineConfig`, `Scanner`, `OpticalTrain`,
etc. — without requiring `#include` of any internal header. Verify by writing the
standalone app with only `#include <machine_config/machine_config.h>` and confirming
it compiles cleanly. If any type requires an additional include, that is a public
API gap. Record in `results.md` under S-09. This check is run twice: once from
source, and once from the static tarball (§8 Step 4).

**Scenarios to execute:** S-01 through S-09, AV-01 through AV-11

**AV-09–AV-11 (C++):** Implement mock v1.1 adapter as test-only `.cpp` files linked only
in the test binary. Verify adapter satisfies the reader/writer abstract interface.

**App structure (`docs/validation/cpp/app/`):**
```
CMakeLists.txt
src/
  main.cpp
  scenarios/
    s01_read_scalars.cpp
    s02_read_binary.cpp
    ... (same pattern)
```

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

## 10. C++ Static Tarball

**When:** After the C++ standalone app is green from source (§7.5 done-gate met).

**Goal:** A self-contained tarball that a C++ consumer can use without installing
HDF5 separately. Contains headers, static library, and CMake config.

### Step 1 — Build HDF5 statically

```bash
# Linux
cmake -S hdf5-src -B hdf5-static-build \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX=./hdf5-static-install \
  -DBUILD_SHARED_LIBS=OFF \
  -DHDF5_BUILD_EXAMPLES=OFF \
  -DHDF5_BUILD_TOOLS=OFF \
  -DHDF5_BUILD_TESTS=OFF \
  -DHDF5_BUILD_HL_LIB=OFF \
  -DHDF5_ENABLE_Z_LIB_SUPPORT=OFF
cmake --build hdf5-static-build -j$(nproc)
cmake --install hdf5-static-build
```

```pwsh
# Windows — vcpkg static triplet
vcpkg install hdf5:x64-windows-static
```

### Step 2 — Build machine_config statically against it

```bash
cmake -S cpp -B cpp/static-build \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_SHARED_LIBS=OFF \
  -DCMAKE_PREFIX_PATH=./hdf5-static-install \
  -DCMAKE_INSTALL_PREFIX=./machine-config-install
cmake --build cpp/static-build --target machine_config
cmake --install cpp/static-build
```

### Step 3 — Package the tarball

```bash
tar -czf machine-config-cpp-v$(VERSION)-$(PLATFORM)-$(ARCH).tar.gz \
  -C machine-config-install .
```

Contents of tarball:
```
include/machine_config/    ← public headers ONLY (no internal/ subdirectory)
lib/libmachine_config.a    ← static library (HDF5 linked in)
cmake/MachineConfigConfig.cmake
cmake/MachineConfigConfigVersion.cmake
README.md
```

**Critical:** `include/machine_config/` must contain only the public-facing header(s).
Internal implementation headers must not appear here. A consumer who can
`#include <machine_config/internal/models.hpp>` and construct library types
directly has bypassed the adapter layer — that is a packaging bug, not a
consumer error. Verify during Step 4 that the standalone app compiles and runs
with only `#include <machine_config/machine_config.h>` and no other includes.

### Step 4 — Verify locally as external consumer

```bash
# Fresh directory, no repo context
mkdir ~/tarball_test && cd ~/tarball_test
tar -xzf /path/to/machine-config-cpp-*.tar.gz -C ./machine-config
# Write minimal CMakeLists.txt:
#   find_package(MachineConfig REQUIRED)
#   target_link_libraries(test_app PRIVATE MachineConfig::machine_config)
cmake -S . -B build -DCMAKE_PREFIX_PATH=./machine-config
cmake --build build
./build/test_app fixtures/reference_config.h5
```

**Record the output in `docs/validation/cpp/tarball/results.md`.**

### Step 5 — Add to CI (cpp.yml or release.yml)

```yaml
- name: Build C++ static tarball (Linux)
  if: runner.os == 'Linux'
  run: |
    cmake -S cpp -B cpp/static-build \
      -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_SHARED_LIBS=OFF \
      -DCMAKE_INSTALL_PREFIX=cpp/install
    cmake --build cpp/static-build --target machine_config
    cmake --install cpp/static-build
    tar -czf machine-config-cpp-linux-x86_64.tar.gz -C cpp/install .

- name: Upload C++ tarball artifact
  uses: actions/upload-artifact@v4
  with:
    name: machine-config-cpp-${{ matrix.os }}
    path: machine-config-cpp-*.tar.gz
```

**Note:** Windows tarball uses `hdf5:x64-windows-static` vcpkg triplet.
Produce separate artifacts per platform. Fire on merge to `main` or on
release tag — not on every PR.

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
- [ ] CI integration added to `python.yml`

### Node.js
- [ ] Standalone app written externally and verified
- [ ] App ported to `docs/validation/nodejs/app/`
- [ ] All S-01–S-09 and AV-01–AV-08 scenarios recorded in `docs/validation/nodejs/results.md`
- [ ] AV-09–AV-11: mock v1.1 adapter implemented in `nodejs/tests/` and passing
- [ ] AV-09–AV-11 recorded in `docs/validation/nodejs/results.md`
- [ ] S-09: `package.json` `types` field confirmed; `.d.ts` files in tarball
- [ ] S-09: all types importable from package root (no internal paths)
- [ ] `docs/validation/nodejs/PASS_FAIL.md` complete
- [ ] CI integration added to `nodejs.yml`

### Rust
- [ ] Standalone app written externally and verified
- [ ] App ported to `docs/validation/rust/app/`
- [ ] All S-01–S-09 and AV-01–AV-08 scenarios recorded in `docs/validation/rust/results.md`
- [ ] AV-09–AV-11: mock v1.1 adapter implemented in `rust/tests/` and passing
- [ ] AV-09–AV-11 recorded in `docs/validation/rust/results.md`
- [ ] S-09: all public types re-exported from crate root (no sub-module paths needed)
- [ ] `docs/validation/rust/PASS_FAIL.md` complete
- [ ] CI integration added to `rust.yml`

### Go
- [ ] Standalone app written externally and verified
- [ ] App ported to `docs/validation/go/app/`
- [ ] All S-01–S-09 and AV-01–AV-08 scenarios recorded in `docs/validation/go/results.md`
- [ ] AV-09–AV-11: mock v1.1 adapter implemented in `go/` `_test.go` files and passing
- [ ] AV-09–AV-11 recorded in `docs/validation/go/results.md`
- [ ] S-09: all exported types accessible via top-level package (no internal imports)
- [ ] `docs/validation/go/PASS_FAIL.md` complete
- [ ] CI integration added to `go.yml`

### C++
- [ ] Standalone app written externally and verified (from source)
- [ ] App ported to `docs/validation/cpp/app/`
- [ ] All S-01–S-09 and AV-01–AV-08 scenarios recorded (from-source run) in `docs/validation/cpp/results.md`
- [ ] AV-09–AV-11: mock v1.1 adapter implemented as test-only `.cpp` and passing
- [ ] AV-09–AV-11 recorded in `docs/validation/cpp/results.md`
- [ ] S-09: standalone app compiles with only `#include <machine_config/machine_config.h>`
- [ ] Static tarball built and verified locally (§8 Steps 1–4)
- [ ] S-09: tarball `include/` contains NO internal headers — verified by inspection
- [ ] Tarball consumer run recorded in `docs/validation/cpp/tarball/results.md`
- [ ] CI tarball packaging step added (§8 Step 5)
- [ ] All S and AV scenarios re-recorded (tarball-consumer run)
- [ ] `docs/validation/cpp/PASS_FAIL.md` complete

### Documentation
- [ ] `docs/validation/README.md` master summary table complete
- [ ] All FAIL verdicts have associated explanation or issue reference

### Merge gate
- [ ] All five languages: done-gate met (all scenarios recorded, all verdicts entered)
- [ ] CI green on both platforms for all workflows
- [ ] Real AconityMIDI file validated end-to-end in all five languages (S-03, S-08)
- [ ] `docs/validation/README.md` shows all ✅ in master table
