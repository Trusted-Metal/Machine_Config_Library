# C++ Validation Results

**Date:** 2026-08-18
**Library version:** machine_config 0.1.0 (header-only, `add_subdirectory`)
**Compiler:** MSVC 19.43.34809.0 (Visual Studio 17 2022), C++17
**Platform:** Windows 11, Git Bash
**Fixture set:** `fixtures/` + `Reference Materials/` + `docs/validation/fixtures/`
**App location:** `docs/validation/cpp/app/` — its own standalone CMake project, `add_subdirectory`-consuming
`cpp/` exactly the way a real from-source consumer would per §9.5 Step 1, with `picosha2` fetched
independently since it's not part of the public `machine_config` target. First developed and verified
externally at `C:\Users\ChrisParham\Desktop\Practice\machineconfiglibrarytesting\Cpp` for S-01–09/AV-01–08,
per `VALIDATION_PLAN.md`'s "External validation project location" convention. AV-09–11 were written
directly in `cpp/tests/` (see "Mock v1.1 adapter" section below) — like Rust, they cannot live in the
external app at all: this library's dispatcher has no registry to inject a mock into.

---

## Run output (verbatim, 17-scenario run of the in-repo app)

```
[PASS] S-01: all scalar fields match expected values
[PASS] S-02: shapes OK, finite values OK, forward!=inverse, correction_data SHA-256=b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6
[PASS] S-03: machine_name="TM-LPBF-02: AconityMIDI+_OG" | file_version="1.0" | trains=2 | build_plate_x=250.000000 | build_plate_y=250.000000 | wd=670.000000 | rotation[0]=0.000000 | hash=9bc38c92c582a154...
[PASS] S-04: machine_name persisted, file_version and other fields unchanged
[PASS] S-05: correction_data preserved: SHA-256=b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6
[PASS] S-06: 2-laser build OK, center~=2.0, roundtrip OK
[PASS] S-07: OPCUA roundtrip OK: 2 triggers, url="opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer"
[PASS] S-08: 3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK
[PASS] S-09: all public types resolve and are usable from the umbrella header
[PASS] AV-01: error raised naming version '2.0': No adapter registered for File_Version "2.0"
[PASS] AV-02: missing File_Version dispatches OK, file_version=''
[PASS] AV-03: error raised naming version '1.1': No adapter registered for File_Version "1.1"
[PASS] AV-04: error raised for missing Machine/ group: Unable to open the group "Machine": (Symbol table) Object not found
[PASS] AV-05: error raised for corrupt scalar: attribute 'Build_Plate_X_Dimension' has non-numeric string value: not_a_number
[PASS] AV-06: whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK
[PASS] AV-07: empty File_Version dispatches OK, file_version=''
[PASS] AV-08: File_Version survives roundtrip unchanged: '1.0'

17 scenarios: 17 passed, 0 failed
```

## Run output (verbatim, AV-09–11 via Catch2 in `cpp/tests/`)

```
Filters: "v1_1_read_all_categories","v1_1_roundtrip","v1_to_v1_1_forward_migration","v1_1_to_v1_backward_migration","v1_unaffected"
Randomness seeded to: 1612605423
===============================================================================
All tests passed (43 assertions in 5 test cases)
```

Full repo suite reconfirmed green after adding these: **354 assertions in 79 test cases** (up from
311/74 before this validation pass).

---

## Reference hashes (cross-language comparison)

| Field | SHA-256 |
|---|---|
| `correction_data` train 0 (reference_config.h5) | `b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6` |

Matches Python's, Node.js's, and Rust's recorded hash for the same fixture/train exactly — confirms
byte-identical correction-grid parity across all four finished languages. S-07's OPCUA server URL also
matches all three exactly.

---

## App file consolidation (post-hoc)

The standalone app's 36 files (17 `.hpp`/`.cpp` scenario pairs + `common.hpp` + `main.cpp`) were
consolidated into a single `scenarios.hpp`/`scenarios.cpp` pair, matching the same one-file-per-app
decision applied to all five languages' apps. Each scenario kept its own namespace (`s01`, `av05`,
etc.), so `main.cpp`'s `scenarioList` needed zero changes — only its 18 `#include` lines collapsed to
one. `CMakeLists.txt`'s source list shrank from 18 `.cpp` files to 2.

Rebuilding after the merge surfaced a real, pre-existing bug, unrelated to the consolidation itself
(`common.hpp` was not touched — verified via `git diff` before attributing it elsewhere):
`avFixture()`'s `fixturesDir.parent_path()` has the same trailing-separator quirk as Go's
`filepath.Dir()` (§9.4) — a `fixturesDir` ending in `/` makes the final path component empty, so
`parent_path()` had nothing to "drop" and returned the path unchanged, breaking every AV-01–07 fixture
lookup (`AV-01`/`AV-03`/`AV-06` failed outright; `AV-02`/`AV-04`/`AV-05`/`AV-07` "passed" only because
they treat any exception as an acceptable/expected outcome). Fixed with the same technique as Go:
`(fixturesDir / "..").lexically_normal()` instead of `.parent_path()`. Re-verified 17/17 after the fix.

---

## Library bugs found and fixed during this validation pass

Two real, pre-existing gaps were found — both while working through §9.5's own prerequisites, before
a single scenario ran:

1. **No umbrella public header existed.** §8 S-09 and §10 (static tarball) both require
   `#include <machine_config/machine_config.hpp>` to be the only include a consumer needs. That header
   never existed — every real consumer (`cpp/src/main.cpp`, both `examples/*/cpp/main.cpp`) used several
   separate includes. Created `cpp/include/machine_config/machine_config.hpp` (deliberately excluding
   `schema.hpp`, which hard-errors without a consumer-supplied `SCHEMA_DIR` — schema validation stays
   opt-in, matching Python's/Node's convention) and wired the CLI and both examples to it as a
   regression check.
2. **`CMAKE_SOURCE_DIR` used where `PROJECT_SOURCE_DIR` was needed**, in `cpp/CMakeLists.txt` (examples)
   and `cpp/tests/CMakeLists.txt` (`FIXTURES_DIR`/`SCHEMA_DIR`). `CMAKE_SOURCE_DIR` is the top of the
   *whole* CMake project tree — correct only when `cpp/` is built standalone. The instant `cpp/` is
   consumed via `add_subdirectory` from any external project (exactly what §9.5 Step 1's own template
   does, and what this validation app does for real), it resolves to the *external* project's root
   instead, and every path relative to it breaks. Fixed by switching to `PROJECT_SOURCE_DIR`, which
   tracks the nearest enclosing `project()` call and is therefore correct in both contexts. Caught a
   subtly-wrong first attempt (`CMAKE_CURRENT_SOURCE_DIR`, which is anchored to the *current listfile's*
   directory — `cpp/tests/`, not `cpp/`, inside `tests/CMakeLists.txt` specifically) by re-running the
   full suite rather than trusting the fix: it silently broke 57 of 74 test cases before being caught
   and corrected.

No runtime/serialization defects were found — unlike Node.js's pass, which found two real bugs
(`attrFloat` silent-null, `meta.extra` double-write). AV-04/AV-05 already threw descriptive errors from
the start, and the mock's `meta.extra` handling (see below) was written correctly on the first pass,
having already seen the equivalent bug surface in Node's and Rust's implementations earlier in this
validation effort.

---

## Mock v1.1 adapter — design and coverage (AV-09–11)

`cpp/tests/mock_v1_1.hpp` implements `MockV1_1Reader`/`MockV1_1Writer` using the same **delegate-then-patch**
design as Node.js and Rust: the reader calls the real, public `Hdf5AdapterV1_0(path).parse()` for the
whole file (every subcomponent this mock doesn't change comes back correct; the documented differences
come back `nullopt`/empty, never a throw), then patches exactly the 10 documented differences via
HighFive's own `Group`/`File` API. The writer calls the real, public `Hdf5WriterV1_0{config}.write(path)`
to produce a fully valid v1.0-shaped file, then reopens it read-write (`HighFive::File::ReadWrite`) and
applies the same 10 edits in place.

**Goes one step further than Node's/Rust's mocks**, because this library is header-only with no
visibility barrier at all: its low-level attribute helpers (`readStr`, `readFloat`, `readRequiredStr` in
`hdf5.hpp`; `ws`, `wf` in `writer.hpp`) are namespace-scope `inline` functions, not private to a class —
so the patch step reuses those exact primitives instead of re-deriving its own. This matters concretely:
`readRaw()` already contains a documented workaround for an HDF5 1.14 quirk (VarLen strings with
`SpacePadded` strpad metadata mishandled by HighFive's own `read<string>()`) — reimplementing string
reading for the mock would have meant either duplicating that workaround or silently reintroducing the
bug it fixes.

**Verified against the real, vendored `HighFive` source** (`cpp/build/_deps/highfive-src/include/highfive/`,
already fetched by a prior build) before committing to the design, not assumed: `File::ReadWrite`,
`AnnotateTraits::createAttribute`/`deleteAttribute`/`hasAttribute`, and
`NodeTraits::createGroup`/`getGroup`/`exist` are all real, stable public API.

**No dispatch-table injection, unlike Python's `_ADAPTERS` or Node's `_READERS`/`_WRITERS`:** this
library's public dispatcher (`MachineConfigReader`/`MachineConfigWriter`) is a hardcoded
`if (ver != "1.0") throw ...` in `reader.hpp`/`writer.hpp`, not a registry — there is no entry to
temporarily inject a mock into. `cpp/tests/test_adapter_migration.cpp`'s 5 tests therefore call
`MockV1_1Reader`/`MockV1_1Writer` directly rather than through the public facade — the same documented
decision made for Rust in `VALIDATION_PLAN.md` §9.3, applied here for the same reason.

**No typed exception hierarchy:** every error path in this library throws a bare `std::runtime_error`
or `std::out_of_range` — there is no C++ equivalent of Rust's `MachineConfigError` enum for the plain
reader/writer. AV-01/03/04/05 (above) catch `std::exception` and inspect `.what()` for the version
string or field name, closer to Node's generic-`Error` situation than Rust's typed variants.

**Version identifier:** the mock uses `"1.1-mock"` from the very first line, matching every other
language's convention (`docs/contributing.md`'s "Mock fixtures and real version numbers" section).

---

## Behavioral notes for cross-language comparison

**AV-02 / AV-07 (missing / empty File_Version):** identical behavior and identical resulting value to
Python, Node.js, and Rust — `file_version` is the empty string `""` in all four languages when the
attribute is absent or empty, while dispatch still succeeds against the v1.0 adapter.

**AV-04 / AV-05 error type:** bare `std::runtime_error`/`std::out_of_range` (via HighFive), naming the
missing group path or the corrupt attribute and value respectively. Same category as Node's generic
`Error` and Python's `KeyError`/`ValueError` — a descriptive but not library-typed error. Same potential
API improvement noted for the other languages (a typed `MissingRequiredGroup`-equivalent) applies here
too; C++ currently has the least of any finished language in this respect, since even AV-01/AV-03
(unsupported version) use a generic exception rather than Rust's typed `UnsupportedVersion` variant.
