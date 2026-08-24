# Correction Data Facade Access — Implementation Plan

**Status: done and verified for all five languages.** Phase 1 (extraction) and Phase 2
(facade `getCorrectionData`/`getInverseCorrectionData` methods) are both complete.
See "Correction to this plan" below for a correction found during Phase 1, and the
Node.js-specific codegen note under Phase 2 for a second correction found while
implementing it.

**Origin:** while discussing an unrelated feature, the user asked whether an application using the
`capabilities` facade (as opposed to the plain `MachineConfigReader`) has an ergonomic way to get
the `(257, 257, 2)` correction/inverse-correction grids as a real numeric array. It does not today
— see "Why this is worth doing" below. That surfaced a second, more fundamental question: is the
null↔NaN grid-conversion logic this needs inherently tied to `capabilities/v1_0/`, or does it
belong at the version-independent model layer? Investigated rather than assumed (see "Where this
logic lives today" below) — it belongs at the model layer, and now every language does it that way.

---

## Correction to this plan (found while implementing Phase 1)

The original version of this plan claimed C++ was "already done" — that `detail::grid3d_to_json` /
`detail::grid3d_from_json` in `models.hpp` meant C++ needed no Phase 1 work. **That was wrong.**
Those two functions only convert `Grid3D` (the model's own nested-`optional<double>` type) to and
from `nlohmann::json` — a trivial structural step. The actual NaN-detecting conversion — flat
`CorrectionData` buffer (real `NaN`, straight off the HDF5 dataset) ↔ nested `Grid3D`
(`nullopt`-for-NaN) — lived in `correctionDataToGrid3D` (`capabilities/v1_0/hdf5.hpp`, a static
method on `Hdf5AdapterV1_0`) and `gridToFlat` (`capabilities/v1_0/writer.hpp`, a free function in
that header). Both were just as version-specific as the other four languages' equivalents. This was
caught by re-reading the actual code rather than trusting the earlier summary, and fixed as part of
this Phase 1 pass — see the C++ row in "Where this logic lives today" and the C++ entry in Phase 1
below, both now corrected.

---

## Why this is worth doing

Every language's facade (`MachineConfigFileV1_0::open()`) already loads the correction grids into
memory at open time — confirmed directly, not assumed:

- Rust: `Hdf5AdapterV1_0::open(...).parse_with_binary()` (`rust/src/capabilities/v1_0/file.rs`)
- Python: `Hdf5AdapterV1_0.parse()` always loads grids unconditionally, no flag
  (`python/src/machine_config/capabilities/v1_0/hdf5.py:512-515`)
- Node.js: `reader.parse({ includeBinary: true })` (`nodejs/src/capabilities/v1_0/file.ts:53`)
- Go: `v1_0hdf5.Parse(path, true)` (`go/capabilities/v1_0/file.go:23`)
- C++: `reader.parseWithBinary()` (`cpp/include/machine_config/capabilities/v1_0/file.hpp:23`)

So the data is not *missing* from the facade — `clearbox().get_model()["correction_data"]` already
has it in every language. The gap is **format**: the facade's model path returns the JSON-shaped
nested list with `null` standing in for `NaN` (`Grid3D` / `Array<Array<Array<number|null>>>` /
etc.), while `MachineConfigReader::get_correction_data()` returns a real numeric array with true
`NaN` — directly usable for math, without an application having to walk three levels of nesting
converting `null`→`NaN` by hand first.

**Design decision, confirmed with the user:** implement this by converting the *already-loaded*
in-memory grid back into a raw array, not by re-delegating to the Reader and re-opening the file
from disk. Two reasons: (1) no redundant I/O for the common `open()`-from-disk case, and (2) a
`create()`-based facade instance has no path to re-read from at all, so delegate-by-reopening
would not work for it — convert-in-memory works uniformly for both.

## Where this logic lives today (as of Phase 1 completion)

The null↔NaN conversion (in both directions) already existed in every language before this
plan — it wasn't new logic to write, only new *placement*.

| Language | Conversion helper | Location |
|---|---|---|
| C++ | `detail::correctionDataToGrid3D` / `detail::gridToFlat` | `models.hpp` — moved here in Phase 1 (previously in `capabilities/v1_0/hdf5.hpp` and `writer.hpp` respectively) |
| Rust | `nan_array3_to_nested` / `nested_to_array3` | `models.rs` — moved here in Phase 1 (previously in `capabilities/v1_0/hdf5.rs` and `writer.rs`) |
| Python | `nan_array_to_nested` / `nested_to_array` | `models.py` — moved here in Phase 1 (previously `Hdf5AdapterV1_0._nan_array_to_list` and `Hdf5WriterV1_0._correction_list_to_array`) |
| Node.js | `float64ToNested3D` / `nestedToFlat` | `models.ts` — moved here in Phase 1 (previously in `capabilities/v1_0/hdf5.ts` and `writer.ts`) |
| Go | `NestedGridFromFlat` / `FlatFromNestedGrid` | `internal/models/models.go` — moved here in Phase 1 (previously inline in `readFloatGrid`, `capabilities/v1_0/hdf5/hdf5.go`, and in `flattenGrid`, `capabilities/v1_0/hdf5/writer.go`) |

**Conclusion (unchanged): this conversion is not inherently version-specific, and per this
project's own StableModel architecture (`docs/contributing.md`), it shouldn't be.** It operates
purely on the stable model's `Grid3D`/`CorrectionData` shape — nothing about it depends on on-disk
attribute names or layout, which is the only thing that actually varies between file versions.

Keeping it inside `capabilities/v1_0/` wasn't broken (there was only one version), but it meant a
hypothetical future `v1_1`/`v2_0` adapter would have had to either duplicate this logic or reach
into `v1_0`'s internals — the wrong direction for a "newer version depends on older version"
dependency. Relocating it now, while there was only one call site to update per language, was far
cheaper than retrofitting it once a second version adapter exists.

---

## Phase 1 — extract grid-conversion helpers to the model layer ✅ done, all 5 languages

- [x] **Rust**: moved `nan_array3_to_nested` (`capabilities/v1_0/hdf5.rs`) and `nested_to_array3`
      (`capabilities/v1_0/writer.rs`) into `models.rs` as `pub(crate)` free functions, placed
      directly after the `CorrectionData` struct. Both call sites resolve unchanged via the
      existing `use crate::models::*;` glob import in each file — no call-site edits needed beyond
      deleting the old definitions. `ndarray::Array3` import dropped from both `hdf5.rs` (now
      unused) and added to `models.rs`.
- [x] **Python**: moved `_nan_array_to_list` (was a `@staticmethod` on `Hdf5AdapterV1_0` in
      `capabilities/v1_0/hdf5.py`) and `_correction_list_to_array` (was a `@staticmethod` on
      `Hdf5WriterV1_0` in `capabilities/v1_0/writer.py`) into `models.py` as module-level functions
      `nan_array_to_nested` / `nested_to_array`. Added `import numpy as np` to `models.py` (not
      previously a dependency there); both adapter modules now import the two functions by name
      from `machine_config.models`.
- [x] **Node.js**: moved `float64ToNested3D` (`capabilities/v1_0/hdf5.ts:215`) and `nestedToFlat`
      (`capabilities/v1_0/writer.ts:141`) into `models.ts` as exported free functions (renamed the
      local shape constant `CORRECTION_SHAPE` → `DEFAULT_CORRECTION_SHAPE` to avoid colliding with
      anything already in `models.ts`). Both call sites import them from `../../models.js`.
- [x] **Go** — the one language where this logic wasn't factored into a named function at all:
      extracted the pure conversion out of `readFloatGrid` (`capabilities/v1_0/hdf5/hdf5.go`) and
      `flattenGrid` (`capabilities/v1_0/hdf5/writer.go`) into two new functions in
      `internal/models/models.go`: `NestedGridFromFlat(flat []float64, shape [3]int) [][][]*float64`
      and `FlatFromNestedGrid(data *[][][]*float64) []float64`. `readFloatGrid` and `flattenGrid`
      now do only HDF5 I/O (open dataset, read dims, read flat buffer) and delegate the actual
      NaN↔nil conversion to the model layer; `flattenGrid` itself was deleted as a pure pass-through
      once its only remaining job was calling `FlatFromNestedGrid`. `math` import dropped from both
      `hdf5.go` and `writer.go` (now unused there). Both files dot-import
      `machine-config-go/internal/models`, so the relocated names resolve unqualified.
- [x] **C++** — turned out, contrary to the original version of this plan (see "Correction to this
      plan" above), to need the same treatment as the other four: moved `correctionDataToGrid3D`
      (was a `static` method on `Hdf5AdapterV1_0` in `capabilities/v1_0/hdf5.hpp`) and `gridToFlat`
      (was a free function in `capabilities/v1_0/writer.hpp`) into `models.hpp`'s existing `detail`
      namespace, right after the `CorrectionData` struct and alongside `grid3d_to_json` /
      `grid3d_from_json`. Added `<cmath>` and `<limits>` to `models.hpp` (needed by `std::isnan` /
      `std::numeric_limits::quiet_NaN`); dropped the now-unused `<cmath>` from `hdf5.hpp` and
      `<limits>` from `writer.hpp`. Call sites updated to `detail::correctionDataToGrid3D(...)` /
      `detail::gridToFlat(...)`.
- [x] Full existing test suite run per language after the move — pure relocation, **zero test
      assertions changed**, only definitions moved and call sites/imports updated:
  - Rust: 90 lib tests + 30 integration tests + 2 doc-tests, all passing (unchanged counts).
  - Python: 354 tests passing (unchanged count).
  - Node.js: 185 tests passing (unchanged count, `vitest run`).
  - Go: all packages pass (`machine-config-go`, `machine-config-go/capabilities`,
    `machine-config-go/internal/h5c`; no test files in the touched packages themselves).
  - C++: 559 assertions in 100 test cases, all passing (unchanged counts).
- [x] Cross-language verification: rebuilt every binary (Rust release, Go CLI at `go/bin/` — the
      path `tools/cross_check.py` actually invokes, C++ Debug tests + Release CLI) and ran
      `tools/cross_check.py --langs python,rust,nodejs,go,cpp`. All active phases passed: Read
      Parity (30 comparisons), Write Interoperability (20 parity + 5 fidelity checks), Binary Copy
      Round-trip (50 checks), Correction Data Hashes (60 comparisons). Schema validation phase
      skipped in that run only because `jsonschema` wasn't on the `PATH`-shadowed interpreter used
      alongside the MinGW64 prefix required for the Go/C++ binaries — already covered independently
      by the Python and Node.js suites' own schema tests (both included in the "unchanged counts"
      above).
  - Caught and fixed one process pitfall while doing this, matching a pattern from earlier work on
    this repo: the first cross-check run reported the Go CLI crashing (`STATUS_DLL_NOT_FOUND`) on
    every fixture. Root cause was environmental, not a code regression — `tools/cross_check.py`
    looks for the Go binary specifically at `go/bin/machine-config-cli.exe`, and a stale copy from
    4 days earlier was sitting there; a fresh build had been placed at `go/machine-config-cli.exe`
    (wrong path) instead. Rebuilding directly to `go/bin/machine-config-cli.exe` resolved it — same
    "always rebuild the exact binary path the checker uses" lesson as the earlier C++
    stale-`Release`-binary false alarm recorded in `SYNCHRONOUS_SENSOR_PLAN.md`.

## Phase 2 — facade `getCorrectionData()` / `getInverseCorrectionData()` ✅ done, all 5 languages

- [x] **Placement, confirmed:** on `MachineConfigFileV1_0` directly, taking a `train_index` —
      exactly mirrors `MachineConfigReader::get_correction_data(train_index)`'s existing signature.
      (Rejected alternative: on the per-train handle with no index. Consistent with the Reader won
      out over consistent with the rest of the facade's handle-based navigation.)
- [x] **Return type, confirmed and implemented:** each language's facade method returns *exactly*
      what that language's Reader method already returns today — **no type changes to any Reader,
      anywhere**:
  - Rust: `CorrectionData` (`{ data: Vec<f64>, shape: [usize; 3] }`) — **not** `Array3<f64>`. Caught
    and corrected during the design discussion: an earlier draft of this plan said `Array3<f64>`,
    but `reader.rs::get_correction_data` and `capabilities/v1_0/hdf5.rs::get_correction_data` both
    return `Result<CorrectionData>` — `Array3` is only a transient intermediate the adapter builds
    from the HDF5 dataset and immediately flattens via `.into_raw_vec_and_offset().0`. Keeping
    `CorrectionData` (rather than changing the Reader to return `Array3<f64>` to match a
    hypothetical facade type) avoids exposing `ndarray` as a version constraint on every downstream
    crate — the exact reasoning already documented on `CorrectionData` itself in `models.rs`,
    now extended to the facade instead of re-litigated for it. A caller who wants `Array3<f64>`
    reconstructs it in one zero-copy call: `Array3::from_shape_vec(cd.shape, cd.data)`.
  - Python: `np.ndarray`, shape `(257, 257, 2)` — matches, as expected.
  - Node.js: `{ data: Float64Array, shape: [number, number, number] }` — matches, as expected.
  - Go: `*CorrectionData` (`{ Data []float64; Shape [3]int }`) — matches, as expected.
  - C++: `CorrectionData` struct (flat `std::vector<double>` + `std::array<size_t, 3>`) — matches,
    as expected.
- [x] **Rust** (`rust/src/capabilities/v1_0/file.rs`): added `get_correction_data`/
      `get_inverse_correction_data(&self, index) -> Result<CorrectionData, CapabilityError>`, each
      calling `get_clearbox(index)` (reusing its existing `InvalidIndex`/`NotPresent` handling) then
      a small private `grid_to_correction_data` helper that calls the Phase 1 model-layer
      `nested_to_array3` and flattens via `.into_raw_vec_and_offset().0`.
- [x] **Python** (`python/src/machine_config/capabilities/v1_0/file.py`): added a private
      `_clearbox_dict(index) -> Result[dict, CapabilityError]` helper (index + presence validation,
      shared by both new methods) and `get_correction_data`/`get_inverse_correction_data(index) ->
      Result[Any, CapabilityError]`, calling the Phase 1 model-layer `nested_to_array` on the
      clearbox dict's `correction_data`/`inverse_correction_data` key.
- [x] **Node.js** (`nodejs/src/capabilities/v1_0/file.ts`): added a private `clearboxJson(index):
      Result<Json, CapabilityError>` helper and `getCorrectionData`/`getInverseCorrectionData
      (trainIndex): Result<CorrectionData, CapabilityError>` (synchronous — no `Promise`, since it
      converts already-loaded data with no I/O, unlike the Reader's async version of the same
      method), calling the Phase 1 model-layer `nestedToFlat`.
  - **Correction found while implementing this (not anticipated in the design discussion):** the
    top-level `openMachineConfig`/`createMachineConfig` entry points return `Result<MachineConfigFile,
    CapabilityError>` — the generated **interface** type, not the concrete `MachineConfigFileV1_0`
    class. Adding the two methods only to the concrete class would have made them unreachable through
    the actual public entry point most callers use. Fixed by treating this as part of the codegen
    contract: added `getCorrectionData`/`getInverseCorrectionData` to `MachineConfigFile` in
    `tools/generate_capabilities.py`'s `render_ts` (imported `CorrectionData` from `../models.js`),
    regenerated `nodejs/src/capabilities/generated.ts`, and confirmed
    `python tools/generate_capabilities.py --check` (the same check `.github/workflows/python.yml`
    runs in CI) still passes. Also moved the `CorrectionData` interface itself from
    `capabilities/v1_0/hdf5.ts` (version-specific) into `models.ts` (model layer) — it was the last
    consumer-facing type not already following the Phase 1 pattern, and the generated interface
    needed to import it from a version-independent location. `hdf5.ts` now re-exports it from
    `models.ts` for source compatibility with existing importers (`reader.ts`, `index.ts`).
  - Checked and confirmed **not** needed for the other four: Rust and Go have no generated
    interface at all (return concrete types from their top-level `open`/`create` functions); C++'s
    `IMachineConfigFile` only declares `fileVersion`/`opticalTrainCount`/`save`/`close` (it never
    included `getScanner`-style methods either, so this is pre-existing, consistent behavior, not a
    gap this feature introduced); Python's `open_machine_config`/`create_machine_config` return the
    concrete `MachineConfigFileV1_0` type already, not its `Protocol`.
- [x] **Go** (`go/capabilities/v1_0/file.go`): added `GetCorrectionData`/`GetInverseCorrectionData
      (index int) (*machineconfig.CorrectionData, *api.Error)`, calling `f.GetClearbox(index)` then
      the Phase 1 model-layer `models.FlatFromNestedGrid` (imported `machine-config-go/internal/models`
      directly, since Go's top-level `machineconfig` package only re-exports *types* from that
      package as aliases, not functions).
- [x] **C++** (`cpp/include/machine_config/capabilities/v1_0/file.hpp`): added
      `getCorrectionData`/`getInverseCorrectionData(std::size_t index) const -> Result<CorrectionData>`,
      calling `getClearbox(index)` then the Phase 1 model-layer `detail::gridToFlat`.
- [x] Tests per language — same four cases in every language: (1) facade result matches the
      Reader's result exactly for `reference_config.h5` train 0, using a NaN-aware comparison helper
      in each language (`assert_correction_data_eq` / `np.testing.assert_array_equal` /
      `toEqual` on `Array.from(...)` / `correctionDataEqual` ×2) since plain equality on real grid
      data — which always contains NaN — is never bit-for-bit `==` in any of these languages; (2)
      shape is `(257, 257, 2)` and at least one NaN cell is present, read through the facade; (3) a
      fixture with train 0's ClearBox stripped (built by parsing `reference_config.h5`, clearing
      `optional_components.clearbox`, and re-writing to a temp file — no stock fixture lacks a
      ClearBox) returns `NotPresent` from both new methods; (4) a `create()`-based instance (no path,
      never touches disk) still returns valid `(257, 257, 2)`-shaped data — **except Go**, where
      `Create()`'s mock config has no `OptionalComponents` at all (unlike the other four languages'
      mock builders, which default to including a ClearBox) and the Go facade has no `SetClearbox` to
      add one after the fact; that test instead asserts `NotPresent` comes back immediately with no
      disk access, which is the accurate behavior for that language's `Create()`, and still
      demonstrates the no-I/O guarantee.
  - Rust: 4 new tests in `rust/tests/capabilities_test.rs` → 18/18 passing in that file (up from 14).
  - Python: 4 new tests in `python/tests/test_capabilities.py` → 15/15 passing in that file (up from 11);
    358 passing overall (up from 354).
  - Node.js: 4 new tests in `nodejs/tests/capabilities.test.ts` → 15/15 passing in that file (up from 11);
    189 passing overall (up from 185).
  - Go: 4 new tests in `go/capabilities/file_test.go`; full `go test ./...` passing.
  - C++: 4 new `TEST_CASE`s in `cpp/tests/test_capabilities.cpp` → 578 assertions in 104 test cases
    overall (up from 559/100).
- [x] Cross-language verification: rebuilt every binary at the exact path `tools/cross_check.py`
      invokes (Rust release, `go/bin/machine-config-cli.exe`, C++ Debug tests + Release CLI) and ran
      `tools/cross_check.py --langs python,rust,nodejs,go,cpp` — all active phases passed (Read
      Parity 30 comparisons, Write Interop 20+5 checks, Binary Copy Round-trip 50 checks, Correction
      Data Hashes 60 comparisons). Confirms Phase 2 changed no on-disk behavior anywhere — expected,
      since these are pure additive facade methods with no writer/reader path changes.

---

## Design decisions — confirmed

- **Not v1_0-specific, extracted to the model layer.** Confirmed with the user: the null↔NaN
  conversion depends only on the stable model's `Grid3D` shape, not on-disk layout, so it belongs
  at the model layer, reusable by any future version adapter without duplication or a
  newer-depends-on-older layering problem. **Now true for all five languages, including C++** (see
  "Correction to this plan" above — C++ was not actually an exception, unlike the original claim).
- **Convert already-loaded in-memory data, don't delegate to the Reader by reopening the file.**
  Confirmed with the user: works uniformly for `open()`- and `create()`-based facade instances
  (the latter has no path to reopen), and avoids redundant I/O for the former. Still the design for
  Phase 2, unaffected by Phase 1's implementation.
- **Phase 2 placement: on `MachineConfigFileV1_0` directly, with a `train_index` parameter.**
  Confirmed with the user, over the alternative of putting it on the per-train handle with no
  index. Chosen to mirror the Reader's existing `get_correction_data(train_index)` signature
  exactly, rather than the facade's own handle-based navigation style. Implemented identically in
  all five languages.
- **Phase 2 return type: exactly what each language's Reader already returns today — no Reader
  changes.** Confirmed with the user, including catching and fixing this plan's own error about
  Rust (it said `Array3<f64>`; the Reader actually returns `CorrectionData`, with `Array3` only a
  transient step inside the adapter). No Reader in any language was touched during implementation.
- **Phase 1 (extraction) is a pure refactor — no behavior change expected, anywhere.** Verified:
  every language's existing test suite passed with unchanged assertion/test counts, and a full
  5-language `cross_check.py` run (read parity, write interop, binary copy round-trip, correction
  hashes) passed with no diffs.
- **Phase 2 is purely additive — no behavior change to any existing method, anywhere.** Verified
  the same way: every language's full existing test suite still passes unchanged, plus 4 new tests
  per language for the new methods, plus a full 5-language `cross_check.py` run with no diffs.
