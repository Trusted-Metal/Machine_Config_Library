# Rust Validation Results

**Date:** 2026-08-18
**Library version:** machine-config 0.2.0-rc.4 (installed via a `path` dependency on the built crate)
**Rust:** 1.96.0
**Platform:** Windows 11, Git Bash
**Fixture set:** `fixtures/` + `Reference Materials/` + `docs/validation/fixtures/`
**App location:** `docs/validation/rust/app/` — its own standalone Cargo workspace (`[workspace]`
with no members other than itself), so it behaves like a real external consumer with an
independent build/lockfile, mirroring how the ported Node.js app is its own separate npm
project rather than being built as part of `nodejs/`'s own package. It was first developed
and verified externally at `C:\Users\ChrisParham\Desktop\Practice\machineconfiglibrarytesting\Rust`
for S-01–09/AV-01–08, per `VALIDATION_PLAN.md`'s "External validation project location"
convention. AV-09–11 were written directly in `rust/tests/` (see "Mock v1.1 adapter" section
below) — they cannot live in the external app at all, since they need `pub` items from the
crate but exercise no dispatcher registry (Rust's dispatcher is a hardcoded `match`, not a
table), so there is nothing for the external app to import that a real consumer couldn't
also reach.

---

## Run output (verbatim, 17-scenario run of the in-repo app)

```
[PASS] S-01: all scalar fields match expected values
[PASS] S-02: shapes OK, finite values OK, forward≠inverse, correction_data SHA-256=b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6
[PASS] S-03: machine_name="TM-LPBF-02: AconityMIDI+_OG" | file_version="1.0" | trains=2 | build_plate_x=Some(250.0) | build_plate_y=Some(250.0) | wd=Some(670.0) | rotation[0]=Some(0.0) | hash=9bc38c92c582a154...
[PASS] S-04: machine_name persisted, file_version and other fields unchanged
[PASS] S-05: correction_data preserved: SHA-256=b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6
[PASS] S-06: 2-laser build OK, center≈2.0, roundtrip OK
[PASS] S-07: OPCUA roundtrip OK: 2 triggers, url="opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer"
[PASS] S-08: 3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK
[PASS] S-09: all public types resolve and are usable from the crate root
[PASS] AV-01: UnsupportedVersion raised, version='2.0'
[PASS] AV-02: missing File_Version dispatches OK, file_version=''
[PASS] AV-03: UnsupportedVersion raised, version='1.1'
[PASS] AV-04: error raised for missing Machine/ group: Required HDF5 path missing: Machine
[PASS] AV-05: Parse error raised for corrupt scalar: Parse error: attribute 'Build_Plate_X_Dimension' has non-numeric string value "not_a_number"
[PASS] AV-06: whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK
[PASS] AV-07: empty File_Version dispatches OK, file_version=''
[PASS] AV-08: File_Version survives roundtrip unchanged: '1.0'

17 scenarios: 17 passed, 0 failed
```

## Run output (verbatim, AV-09–11 via `cargo test --test adapter_migration_test` in `rust/`)

```
running 5 tests
test v1_unaffected ... ok
test v1_to_v1_1_forward_migration ... ok
test v1_1_read_all_categories ... ok
test v1_1_roundtrip ... ok
test v1_1_to_v1_backward_migration ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
```

Full repo suite reconfirmed green after adding these: 74 lib + 8 capabilities + 30 integration
+ 5 adapter-migration + 2 doc tests, 119/119.

---

## Reference hashes (cross-language comparison)

| Field | SHA-256 |
|---|---|
| `correction_data` train 0 (reference_config.h5) | `b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6` |

Matches Python's and Node.js's recorded hash for the same fixture/train exactly — confirms
byte-identical correction-grid parity across all three finished languages. S-07's OPCUA
server URL also matches both languages' recorded value exactly.

---

## Mock v1.1 adapter — design and coverage (AV-09–11)

`rust/tests/mock_v1_1/mod.rs` implements `MockV1_1Reader`/`MockV1_1Writer` using the same
**delegate-then-patch** design as Node.js: the reader calls the real, public
`Hdf5AdapterV1_0::open(path)?.parse()?` for the whole file (every subcomponent this mock
doesn't change comes back correct; the documented differences come back `None`/empty, never
a panic), then patches exactly the 10 documented differences by reading their real, mock-v1.1
locations directly via `hdf5-metno`'s own public `Location`/`Group` API. The writer calls the
real, public `Hdf5WriterV1_0::new(config).write(path)?` to produce a fully valid v1.0-shaped
file, then reopens it read-write (`hdf5::File::open_rw`) and applies the same 10 edits in
place (`delete_attr`, `create_group`, `new_attr::<T>().create(name)?.write_scalar(&v)?`).

**Before committing to this design, its two load-bearing operations were verified against
the real `hdf5-metno` 0.12.5 crate source** (vendored at
`~/.cargo/registry/src/index.crates.io-*/hdf5-metno-0.12.5/`), not assumed from memory or
docs.rs: `File::open_rw` (`hl/file.rs`), `Location::delete_attr`/`Location::new_attr`
(`hl/location.rs`), and `Group::create_group`/`Group::group`/`Group::member_names`
(`hl/group.rs`) are all part of the crate's stable public API, and the attribute-write idiom
(`new_attr::<T>().create(key)?.write_scalar(&v)?`) already matches what the real writer uses
throughout `capabilities/v1_0/writer.rs`.

**No dispatch-table injection, unlike Python's `_ADAPTERS` or Node's `_READERS`/`_WRITERS`:**
Rust's public dispatcher (`MachineConfigReader`/`MachineConfigWriter`) is a hardcoded
`match version.as_str() { "1.0" => ..., other => Err(...) }` in `reader.rs`/`writer.rs`, not
a registry — there is no entry to temporarily inject a mock into. `rust/tests/adapter_migration_test.rs`'s
5 tests therefore call `MockV1_1Reader`/`MockV1_1Writer` directly rather than through the
public facade. AV-09's actual rationale — "adding v1.1 doesn't require modifying the v1.0
adapter" — is satisfied by the mock living in its own file with zero edits to
`capabilities/v1_0/`, plus the full pre-existing suite staying green after it was added
(the `v1_unaffected` test, and every other test in the crate). This was a deliberate,
documented decision (see `VALIDATION_PLAN.md` §9.3), not a gap discovered mid-implementation.

**File layout note:** the mock lives at `rust/tests/mock_v1_1/mod.rs`, not a flat
`rust/tests/mock_v1_1.rs`. A flat file directly under `tests/` is always auto-registered by
Cargo as its own independent integration-test binary; putting the mock there would have
collided with `adapter_migration_test.rs`'s `mod mock_v1_1;` inclusion of the same code.

**Version identifier:** the mock uses `"1.1-mock"` from the very first line, matching
Python's and Node's convention (`docs/contributing.md`'s "Mock fixtures and real version
numbers" section).

No real library defects were found while building this mock — unlike Python's and Node's
passes, which each found and fixed one genuine bug (Node's `attrFloat` silent-null and
`meta.extra` double-write; see `docs/validation/nodejs/results.md`). Rust's `read_float`
already threw a typed `MachineConfigError::Parse` for a corrupt scalar from the start (see
AV-05's output above), and the mock reader's `meta.extra.shift_remove(...)` step was written
correctly on the first pass — it was added proactively, having already seen the exact same
bug class surface in Node's implementation earlier this session, not discovered by a failing
test here.

---

## Behavioral notes for cross-language comparison

**AV-02 / AV-07 (missing / empty File_Version):** identical behavior and identical resulting
value to Python and Node.js — `file_version` is the empty string `""` in all three languages
when the attribute is absent or empty, while dispatch still succeeds against the v1.0
adapter. This was verified by actually running the fixtures, not inferred from reading
`reader.rs`'s dispatch `match` in isolation — the raw `match` statement alone looks like it
would only accept an exact `"1.0"` string, but the observed behavior matches the other two
languages exactly, meaning version normalization happens upstream of that `match` (in
`peek_file_version`).

**AV-04 error type:** `MachineConfigError::MissingGroup(String)` — a typed, library-defined
error naming the missing path (`"Required HDF5 path missing: Machine"`). This is a real
improvement over Python's bare `h5py`-raised `KeyError` and Node's bare `h5wasm`-raised
generic `Error` for the same scenario — Rust is the first of the three finished languages to
return a purpose-built error type here instead of a generic one.

**AV-05 error type:** `MachineConfigError::Parse(String)`, naming both the attribute and the
bad value (`"attribute 'Build_Plate_X_Dimension' has non-numeric string value \"not_a_number\""`).
Same category as Python's `ValueError`/Node's `TypeError` — a descriptive, typed error, not a
panic.
