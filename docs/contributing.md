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

1. Fix the reader bug in `python/src/machine_config/reader.py`
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
