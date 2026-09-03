# Go Validation Results

**Date:** 2026-08-18
**Library version:** machine-config-go 0.2.0-rc.4 (installed via a `replace` directive on the
local `go/` checkout)
**Go:** go1.26.5 windows/amd64
**Platform:** Windows 11, Git Bash + MSYS2 MinGW64 (CGo build)
**Fixture set:** `fixtures/` + `Reference Materials/` + `docs/validation/fixtures/`
**App location:** `docs/validation/go/app/` — its own standalone Go module (`go.mod` with a
`replace machine-config-go => ../../../../go` directive, not a member of `go/`'s own module),
mirroring how the ported Rust/Node.js apps are their own separate projects rather than being
built as part of the library's own module. It was first developed and verified externally at
`C:\Users\ChrisParham\Desktop\Practice\machineconfiglibrarytesting\Go` for S-01–09/AV-01–08, per
`VALIDATION_PLAN.md`'s "External validation project location" convention. AV-09–11 were written
directly in `go/` (see "Mock v1.1 adapter" section below) — they cannot live in the external app
at all, for the same structural reason as Rust: Go's dispatcher is a hardcoded `switch`, not a
registry, so there is nothing for the external app to import that a real consumer couldn't
also reach.

---

## Environment setup (CGo/HDF5)

`go/internal/h5c/h5c.go` is the entire HDF5 I/O layer via CGo:
```c
#cgo linux pkg-config: hdf5
#cgo linux CFLAGS: -I/usr/include/hdf5/serial
#cgo linux LDFLAGS: -L/usr/lib/x86_64-linux-gnu/hdf5/serial -lhdf5
#cgo windows CFLAGS: -I/mingw64/include
#cgo windows LDFLAGS: -L/mingw64/lib -lhdf5
```

On this Windows dev machine, MSYS2 MinGW64 with HDF5 1.14.6 was already installed at
`C:\msys64` (`mingw-w64-x86_64-hdf5`, verified via `pkg-config --modversion hdf5` → `1.14.6`).
Building and running from Git Bash directly (not inside an actual `mingw64.exe` login shell)
worked without issue by exporting:
```bash
export CGO_ENABLED=1
export CC="C:/msys64/mingw64/bin/gcc.exe"
export PATH="/c/msys64/mingw64/bin:$PATH"
```
The `/mingw64/include`/`/mingw64/lib` paths in the cgo directives resolve correctly regardless
of the invoking shell, because MinGW-w64 gcc resolves its own sysroot relative to its own binary
location (`C:\msys64\mingw64\bin\gcc.exe` → `C:\msys64\mingw64\...`), not the shell's `PATH`.
`go.yml`'s CI recipe (`msys2/setup-msys2@v2` + `cygpath -w /mingw64/bin/gcc.exe` inside
`shell: msys2 {0}`) sets up the identical `/mingw64` root, so this local recipe and CI's are
consistent — nothing in `go.yml` needed correction.

---

## Run output (verbatim, 17-scenario run of the in-repo app)

```
[PASS] S-01: machine_name="TM-LPBF-02: AconityMIDI+_OG" file_version="1.0" hash_len=64
[PASS] S-02: shape=[257 257 2] nan_count=112700 total=132098
[PASS] S-03: machine_name="TM-LPBF-02: AconityMIDI+_OG" file_version="1.0" trains=2 build_plate_x=250 wd=670.00 hash=9bc38c92c582a154...
[PASS] S-04: machine_name modified and survives roundtrip
[PASS] S-05: correction_data preserved: SHA-256=b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6
[PASS] S-06: 2-laser build OK, center~2.0, roundtrip OK
[PASS] S-07: opcua server_url="opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer" preserved through roundtrip
[PASS] S-08: 3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK
[PASS] S-09: all public types resolve and are usable from the module root
[PASS] AV-01: UnsupportedFileVersionError raised, version="2.0"
[PASS] AV-02: missing File_Version dispatches OK, file_version=""
[PASS] AV-03: UnsupportedFileVersionError raised, version="1.1"
[PASS] AV-04: error raised for missing Machine/ group: H5Gopen2(Machine) failed
[PASS] AV-05: error raised for corrupt scalar: attribute "Build_Plate_X_Dimension": non-numeric string "not_a_number"
[PASS] AV-06: whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK
[PASS] AV-07: empty File_Version dispatches OK, file_version=""
[PASS] AV-08: File_Version survives roundtrip unchanged: "1.0"

17 scenarios: 17 passed, 0 failed
```
Same output, re-verified after porting the app into `docs/validation/go/app/` (relative
`replace` path instead of the absolute local dev path).

## Run output (verbatim, AV-09–11 via `go test .` in `go/`)

```
=== RUN   TestV1Unaffected
--- PASS: TestV1Unaffected (0.00s)
=== RUN   TestV1_1ReadAllCategories
--- PASS: TestV1_1ReadAllCategories (0.06s)
=== RUN   TestV1_1Roundtrip
--- PASS: TestV1_1Roundtrip (0.09s)
=== RUN   TestV1ToV1_1ForwardMigration
--- PASS: TestV1ToV1_1ForwardMigration (0.06s)
=== RUN   TestV1_1ToV1BackwardMigration
--- PASS: TestV1_1ToV1BackwardMigration (0.08s)
PASS
ok  	machine-config-go	2.750s
```
Full `go test ./...` (every existing test plus these 5) also re-run green after adding the
mock — see "Mock v1.1 adapter" below for why that matters.

---

## A real Go stdlib gotcha found while writing AV-01–08

`filepath.Dir()` on Windows does not behave like Rust's `Path::parent()` when the input has a
trailing separator: `filepath.Dir("C:\...\fixtures\")` returns `"C:\...\fixtures"` unchanged
(not its parent), because `Split()` sees an empty final path element after the trailing
separator and has nothing to drop. `filepath.Dir("C:\...\fixtures")` (no trailing slash)
correctly returns the parent. Verified directly with a scratch `go run`. Since `go.yml`
invokes the app with `"$PWD/fixtures/"` (trailing slash), the first version of `avFixture()`
in `docs/validation/go/app/scenarios/common.go` silently produced a doubled path
(`.../fixtures/docs/validation/fixtures/...`) and every AV-0x fixture lookup failed with an
HDF5 open error. Fixed by using `filepath.Join(fixturesDir, "..", "docs", "validation",
"fixtures", name)` instead of `filepath.Dir(fixturesDir)` — `Join`'s own `Clean()` resolves
`".."` correctly regardless of a trailing separator in the input.

---

## Behavioral notes for cross-language comparison

**AV-02 / AV-07 (missing / empty File_Version):** identical behavior and identical resulting
value to Python, Node.js, Rust, and C++ — `file_version` is the empty string `""` in all five
languages when the attribute is absent or empty, while dispatch still succeeds against the
v1.0 adapter. Verified directly: Go's `PeekFileVersion` (`go/file_version.go`) defaults to
`"1.0"` for **dispatch** when the attribute is missing/empty/whitespace-only (after
`strings.TrimSpace`), but the model's `Meta.FileVersion` field is populated separately, straight
from the raw HDF5 attribute value inside `capabilities/v1_0/hdf5/hdf5.go`'s `parse()` — which is
`""` when the attribute doesn't exist. This was checked against all four other languages'
`results.md` before concluding it wasn't a bug: Python's own doc states the identical rationale
("dispatch succeeds and the model's `file_version` field is `""`"), and Rust/C++ independently
recorded the same `""` result.

**AV-04 error type:** a plain, untyped `error` — `fmt.Errorf("H5Gopen2(%s) failed", path)` from
`internal/h5c`, wrapped once more on the way up. Same category as Python's `KeyError`, Node's
generic `Error`, and C++'s `runtime_error`/`out_of_range` — a descriptive but not library-typed
error. Rust is the outlier here with a typed `MachineConfigError::MissingGroup(String)`. Same
potential API improvement noted for the other three languages (a typed
`MissingRequiredGroup`-equivalent) would apply here too, if ever pursued.

**AV-05 error type:** also a plain `error`, but with a genuinely descriptive message —
`capabilities/v1_0/hdf5/helpers.go`'s `readFloatAttr` explicitly detects a non-numeric string
value and returns `fmt.Errorf("attribute %q: non-numeric string %q", key, s)` rather than
letting a generic HDF5 type-mismatch failure surface. Confirmed this *before* writing the AV-05
scenario, not after a failing run.

---

## Public type export surface (S-09) — one open item, deliberately left unresolved

`go/models.go` re-exports the model surface via Go type aliases. All 18 aliased types plus
`MachineConfigReader`/`MachineConfigWriter`/`MockConfigBuilder`/`ParseOptions`/
`UnsupportedFileVersionError` (already at the module root — no alias needed, since
`reader.go`/`writer.go`/`builder.go` declare `package machineconfig` directly) resolve cleanly
with no `internal/...` import required.

**`BuildPlate`** (`go/internal/models/models.go`) is a real, exported, aliased type — but
nothing in the actual codebase ever constructs one. `Machine` carries build-plate dimensions as
its own flat `BuildPlateX/Y/Z` fields (matching Rust/C++/Node's convention), not a nested
`BuildPlate` value. `reader_test.go`'s `TestReadMachineBuildPlate` — despite its name — asserts
against `cfg.Machine.BuildPlateX`, not a `BuildPlate` struct instance. S-09
(`docs/validation/go/app/scenarios/s09_type_exports.go`) type-annotates it (`var buildPlate
mc.BuildPlate`) without constructing it, matching this plan's own precedent for Rust's
`OpcuaConfig` in the same situation — but whether `BuildPlate` should be kept as reserved public
surface or removed as dead code was explicitly raised with the user and left **undecided**, not
silently resolved either way.

---

## Mock v1.1 adapter — design and coverage (AV-09–11)

`go/internal/mockv1_1/mockv1_1.go` implements `MockV1_1Reader`/`MockV1_1Writer` using the same
**delegate-then-patch** design as Rust and Node: the reader calls the real, public
`capabilities/v1_0/hdf5.Parse`, then patches exactly the 10 documented differences by reading
their real mock-v1.1 locations directly; the writer calls the real, public
`capabilities/v1_0/hdf5.Write`, then reopens the file read-write and patches the same 10
differences in place. "Unchanged" subcomponents (light source, collimator, scanner card,
clearbox, SFCF, OPCUA) are never re-tested here — already covered by the pre-existing suite
exercising the real v1.0 adapter.

**No dispatch-table injection, unlike Python's `_ADAPTERS` or Node's `_READERS`/`_WRITERS`:**
Go's public dispatcher (`MachineConfigReader`/`MachineConfigWriter`, and the `capabilities`
stable-facade layer) is a hardcoded `switch fv { case "1.0": ...; default: ... }` in
`reader.go`/`writer.go`/`capabilities/file.go` — there is no entry to temporarily inject a mock
into. `go/mock_v1_1_test.go`'s 5 tests therefore call `MockV1_1Reader`/`MockV1_1Writer` directly
rather than through the public facade. AV-09's actual rationale — "adding v1.1 doesn't require
modifying the v1.0 adapter" — is satisfied by the mock living in its own package
(`go/internal/mockv1_1`) with zero edits to `capabilities/v1_0/`, plus the full pre-existing
suite staying green after it was added (`TestV1Unaffected`, and every other test in the module).

**`h5c.go` needed two new primitives that Rust/Node get for free.** Rust's `hdf5-metno` crate
and Node's underlying HDF5 library already expose `open_rw`/`delete_attr` as part of a
full-featured third-party binding. Go's `internal/h5c` is the project's own minimal, hand-written
CGo wrapper — it only had `Open` (read-only) and `Create` (truncating write), and no way to
delete an attribute, because nothing in the real v1.0 reader/writer ever needed either. Two
small, generic primitives were added to support delegate-then-patch:
- `OpenRW(path string) (*File, error)` — opens an existing file `H5F_ACC_RDWR` without
  truncating it.
- `(g *Group) DeleteAttr(name string) error` — `H5Adelete`, no-op if the attribute is absent.

This was raised explicitly before implementing: the alternative (build the mock's v1.1-mock
file entirely from scratch using only pre-existing primitives, avoiding any change to `h5c.go`)
would have skipped exercising the real v1.0 *writer* for every unchanged subcomponent — which is
the actual point of the delegate-then-patch design, not an incidental detail. The user chose to
extend `h5c.go` with the two primitives rather than take that shortcut.

**Production model changes (additive, non-breaking):** `MachineConfigMeta` gained two new
`omitempty` pointer fields, `FacilityID *string` and `ConfigAuthor *string`
(`go/internal/models/models.go`), explicitly commented as test-fixture-only and never populated
by the real v1.0 read path — mirroring `python/src/machine_config/models.py`'s
`facility_id`/`config_author` and `rust/src/models.rs`'s identical fields (marked "TEST FIXTURE
for the mock v1.1 adapter" there too). `Machine.BuildPlateRadius`/`BuildPlateRadiusUnit` already
existed and were already fully wired into the real v1.0 read/write path (unlike Rust, where the
equivalent field is comparatively less exercised) — no change needed there.

**Version identifier:** the mock uses `"1.1-mock"` from the very first line, matching every
other language's convention (`docs/contributing.md`'s "Mock fixtures and real version numbers"
section).

**The 10 documented differences** (identical manifest to Rust/Node — see
`docs/migrations/mock_v1_0_to_v1_1.md`):
| # | Category | v1.0 | v1.1-mock |
|---|---|---|---|
| 1 | ADDITION | (absent) | `Facility_ID` (root attr) |
| 2 | ADDITION | (absent) | `Config_Author` (root attr) |
| 3 | REMOVAL | `Machine/Gas_Flow_Direction` | (absent) |
| 4 | REMOVAL | `Machine/Recoat_Direction` | (absent) |
| 5 | NAME | `Machine/Machine_Name` | `Machine/Machine_Label` |
| 6 | NAME (scanner) | `.../Scanner/Working_Distance` | `.../Scanner/Focal_Distance` |
| 7 | PATH | `Machine/Build_Plate_Z_Dimension` | `Machine/Dimensions/Build_Plate_Z_Dimension` |
| 8 | PATH | `Machine/Build_Plate_Corner_Radius` | `Machine/Dimensions/Build_Plate_Corner_Radius` |
| 9 | NAME+PATH | `Machine/Build_Plate_X_Dimension` | `Machine/Dimensions/Width` |
| 10 | NAME+PATH | `Machine/Build_Plate_Y_Dimension` | `Machine/Dimensions/Height` |

---

## Not yet done — consistent with all four other languages, not Go-specific

Wiring the standalone validation app into CI was checked directly against `rust.yml`,
`cpp.yml`, `nodejs.yml`, and `python.yml` — none of them reference their respective
`docs/validation/*/app/` at all. This is an existing gap across the whole project, so `go.yml`
was deliberately left unchanged rather than making Go the one language wired in while the other
four are not.

Also not pursued this pass, matching what's still open for the other languages: independent
verification of the Windows CGo build by a from-scratch CI run (only local, already-installed
MSYS2 HDF5 was used here), and a real `npx semantic-release` run of any of this.
