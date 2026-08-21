# Correction Data Facade Access — Implementation Plan

**Status: not started.**

**Origin:** while discussing an unrelated feature, the user asked whether an application using the
`capabilities` facade (as opposed to the plain `MachineConfigReader`) has an ergonomic way to get
the `(257, 257, 2)` correction/inverse-correction grids as a real numeric array. It does not today
— see "Why this is worth doing" below. That surfaced a second, more fundamental question: is the
null↔NaN grid-conversion logic this needs inherently tied to `capabilities/v1_0/`, or does it
belong at the version-independent model layer? Investigated rather than assumed (see "Where this
logic lives today" below) — it belongs at the model layer, and one language already does it that
way.

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

## Where this logic lives today

The null↔NaN conversion (in both directions) already exists in every language — it's not new
logic to write. Checked directly, not assumed:

| Language | Conversion helper | Current location |
|---|---|---|
| C++ | `detail::grid3d_to_json` / `detail::grid3d_from_json` | `models.hpp` — **already version-independent** |
| Rust | `nan_array3_to_nested` | `capabilities/v1_0/hdf5.rs:341` — version-specific |
| Python | `Hdf5AdapterV1_0._nan_array_to_list` | `capabilities/v1_0/hdf5.py:506` — a method on the v1_0 adapter class itself |
| Node.js | `float64ToNested3D` / `nestedToFlat` | `capabilities/v1_0/hdf5.ts:215` / `capabilities/v1_0/writer.ts:141` — version-specific |
| Go | (inline, not factored into a named function) | inside `readFloatGrid` in `capabilities/v1_0/hdf5/hdf5.go` — most tightly coupled of the five |

**Conclusion: this conversion is not inherently version-specific, and per this project's own
StableModel architecture (`docs/contributing.md`), it shouldn't be.** It operates purely on the
stable model's `Grid3D`/`CorrectionData` shape — nothing about it depends on on-disk attribute
names or layout, which is the only thing that actually varies between file versions. C++ already
treats it this way; the other four languages currently don't, for no principled reason (just
historical — it was written inline while building the v1_0 adapter and never relocated).

Keeping it inside `capabilities/v1_0/` isn't broken today (there's only one version), but it means
a hypothetical future `v1_1`/`v2_0` adapter would have to either duplicate this logic or reach into
`v1_0`'s internals — the wrong direction for a "newer version depends on older version" dependency.
Relocating it now, while there's only one call site to update per language, is far cheaper than
retrofitting it once a second version adapter exists.

---

## Phase 1 — extract grid-conversion helpers to the model layer

**Not needed for C++** — already done (`detail::grid3d_to_json`/`grid3d_from_json` in `models.hpp`).
The facade method for C++ (Phase 2) can call these directly, unchanged.

- [ ] **Rust**: move `nan_array3_to_nested` (and the reverse — check whether a mirroring
      "nested→Array3" helper already exists in `writer.rs` under a different name, e.g. near
      `nested_to_array3` mentioned in earlier work; relocate that one too if so) out of
      `capabilities/v1_0/hdf5.rs` into `models.rs`, as free functions (not methods), matching
      C++'s `detail`-namespace placement. Update the one call site in `capabilities/v1_0/hdf5.rs`
      (and the writer's call site) to call the relocated function via `crate::models::...`.
- [ ] **Python**: move `_nan_array_to_list` (currently a `@staticmethod` on `Hdf5AdapterV1_0`) out
      of `capabilities/v1_0/hdf5.py` into `models.py` as a free function; same for whatever
      list→ndarray helper `writer.py` uses on the way back out (check
      `_correction_list_to_array` in `capabilities/v1_0/writer.py`, mentioned in earlier work, for
      the reverse direction). Update both call sites.
- [ ] **Node.js**: move `float64ToNested3D` (`capabilities/v1_0/hdf5.ts:215`) and `nestedToFlat`
      (`capabilities/v1_0/writer.ts:141`) into `models.ts` as exported free functions. Update both
      call sites' imports.
- [ ] **Go**: this is the one language where the logic isn't factored into a named function at
      all today — extract it from inline code in `readFloatGrid`
      (`capabilities/v1_0/hdf5/hdf5.go`) into a properly named, tested function in
      `internal/models/models.go` (e.g. `NestedGridFromFlat`/`FlatFromNestedGrid`, matching this
      package's existing naming style), and do the same for the write-direction flattening
      currently inline in `writer.go`'s `flattenGrid`-equivalent (verify exact current name before
      writing this).
- [ ] For each language: run the full existing test suite after the move — this is a pure
      relocation, not a behavior change, so **zero test assertions should need to change**, only
      import paths/call sites. Any test failure here means the extraction accidentally changed
      behavior and needs to be fixed before moving on, not worked around.

## Phase 2 — facade `get_correction_data()` / `get_inverse_correction_data()`

Once Phase 1 lands (or immediately for C++, which needs no Phase 1 work), add a facade-level
method per language that converts the already-loaded in-memory grid using the (now
version-independent) helper — no disk I/O, works for both `open()`- and `create()`-based
instances.

- [ ] Decide exact placement and signature per language's existing facade idiom before writing
      code — this repo's five facades don't share one binding style (Rust/C++ use
      `Result<T, CapabilityError>` return types; Python/Node use the same `Result`/`ok`/`err`
      helpers; Go uses `*api.Error`). Candidates to settle before implementing, not assumed:
  - On `MachineConfigFileV1_0` directly, taking a train index (mirrors
    `MachineConfigReader::get_correction_data(train_index)`'s existing signature exactly) — matches
    the Reader's shape most closely, easiest migration for existing Reader-based code switching to
    the facade.
  - On the per-train handle (`_TrainHandle`/`TrainHandle`/train "optical_train(i)" result), no
    index needed since the handle already has one — more idiomatic given the facade's existing
    handle-based navigation (`optical_train(i).get_scanner()`, etc.), but a bigger API-shape
    decision to confirm before committing to it across five languages.
- [ ] Return type: match each language's existing raw-array convention from
      `MachineConfigReader::get_correction_data` (Rust: `ndarray::Array3<f64>`/flat `Vec<f64>` +
      shape, matching whatever that method already returns exactly; Python: `np.ndarray`; Node.js:
      flat `Float64Array` + shape tuple; Go: `*CorrectionData` (`{Data []float64; Shape [3]int}`,
      the type already used by the Reader's own method); C++: `CorrectionData` struct, same as the
      Reader.
- [ ] Add both correction and inverse-correction accessors, matching the Reader's existing pair.
- [ ] Tests per language: a facade-opened file's `get_correction_data()` returns a real `NaN`
      (not converted to any sentinel) at the same border cells the Reader-level test already
      knows are NaN for `reference_config.h5` (reuse that fixture, matching existing Reader test
      expectations — do not re-derive which cells are NaN by hand). A second test on a
      `create()`-based (never-saved, no path) facade instance confirms the method still works
      without touching disk — this is the specific case that rules out the
      delegate-to-Reader-by-reopening design and must actually be exercised, not just reasoned
      about.

---

## Design decisions — confirmed

- **Not v1_0-specific, extracted to the model layer.** Confirmed with the user: the null↔NaN
  conversion depends only on the stable model's `Grid3D` shape, not on-disk layout, so it belongs
  at the model layer (matching C++'s existing `detail::grid3d_to_json`/`from_json`), reusable by
  any future version adapter without duplication or a newer-depends-on-older layering problem.
- **Convert already-loaded in-memory data, don't delegate to the Reader by reopening the file.**
  Confirmed with the user: works uniformly for `open()`- and `create()`-based facade instances
  (the latter has no path to reopen), and avoids redundant I/O for the former.
- **Phase 1 (extraction) is a pure refactor — no behavior change expected, anywhere.** Existing
  test suites must pass unchanged after the move; any assertion needing to change indicates the
  extraction broke something, not that a test was stale.
- **C++ needs no Phase 1 work** — it already has the version-independent helper Phase 1 exists to
  give the other four languages.
