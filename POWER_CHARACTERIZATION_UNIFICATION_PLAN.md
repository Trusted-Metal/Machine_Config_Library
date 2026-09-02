# Power Characterization Unification — Implementation Plan

> **Status: complete (2026-09-02) — implementation, cross-check, and documentation all
> done; only the commit remains, left for the user.** Reviewed via
> `power_characterization_mapping.html` (architecture-review artifact) and approved in
> principle (2026-09-02) — this document is the concrete execution plan. Follow-on to
> `V1_1_IMPLEMENTATION_PLAN.md`'s Phase 2, which this plan **supersedes and simplifies**
> for the `ClearBox`/`LightSource` power-conversion fields specifically. Sequencing
> followed the same pattern as Phase 1/2 — one language per pass, fully verified before
> the next (Python → Rust → C++ → Go → Node.js), then a full `tools/cross_check.py` run
> and the §6 documentation updates. See "Next steps" at the end of this document for
> what's left (just the commit).

---

## Python — Done (2026-09-02)

Implemented exactly per §2–§4 below, with every file:line citation in this plan
confirmed accurate against the real pre-change source before editing (no drift from
when this plan was drafted).

- **StableModel** (`models.py`): removed all 4 fields from `ClearBox`/`LightSource`,
  as planned.
- **v1.0 reader** (`v1_0/hdf5.py`): `_parse_clearbox`/`_parse_light_source` now bind the
  raw attrs to locals and forward-derive `power_characterization`, exactly as planned.
  Also updated the **4 hand-written JSON dict-helper sites** (`_config_to_dict`/
  `config_from_dict`'s `_clearbox_to_dict`-equivalent inline dicts) the plan flagged —
  removed the old field entries there too.
- **v1.0 writer** (`v1_0/writer.py`): both `ClearBox`/`LightSource` blocks simplified to
  the unconditional `backward_flat_fields_*` call, as planned.
- **v1.1 reader/writer** (`v1_1/hdf5.py`, `v1_1/writer.py`): removed the now-invalid
  `None` assignments and the fallback-derivation branches; deleted the now-unused
  `forward_power_characterization_*` import from `v1_1/writer.py` entirely, since
  nothing else in that file needed it.
- **`MockConfigBuilder`/`YamlConfigBuilder`** (`builder.py`): both `_mock_clearbox`'s and
  `_mock_train`'s `LightSource`'s hardcoded flat fields replaced with
  `forward_power_characterization_coefficients`/`_points` calls using the same literal
  values as before (`"LINEAR"`/`"50.0,100.0"`; `"LINEAR"`/`"[1,100,10,1000]"`) — the
  mock can never drift from what a real v1.0 read of those same values would produce.
  `YamlConfigBuilder`'s dict-hydration site treated the same way, keeping the YAML spec's
  own key names (`watts_to_volts_algorithm`/`_params`) unchanged for anyone with an
  existing YAML file — only the StableModel construction changed.
- **`create()`**: confirmed no change needed — Python's relies on a real disk round-trip,
  never had an explicit derivation call to remove.
- **Schema**: removed the same 4 properties from both `schema/machine_config_v1.schema.json`
  (canonical) and its `python/src/machine_config/` copy, and from
  `schema/capabilities/models.yaml`. `python tools/generate_capabilities.py --check`
  confirmed zero drift both before and after — exactly the no-op this plan's §1
  predicted, since that generator's templates are hardcoded and never actually read
  `models.yaml`'s field list.

**A real bug found and fixed, beyond the plan's original scope — necessary, not
optional:** `capabilities/v1_0/hdf5.py`'s `to_json()` (used by the CLI's `export-json`
and by `tools/generate_fixtures.py`'s golden-file generator) calls `_config_to_dict`,
which — per a pre-existing, previously-harmless gap already noted in
`PARITY_AUDIT.md`'s "lower-confidence" section — never serialized `power_characterization`
at all for v1.0 objects. Before this plan, that was inert: `power_characterization` was
always `None` for a v1.0 read, so omitting it changed nothing observable. After this
plan's reader change, v1.0 reads **always** populate it — so the same omission became a
live bug, silently dropping real data from every v1.0 file's canonical JSON. Caught by
regenerating `fixtures/reference_output.json` and inspecting it directly, not assumed.
Fixed by adding `_power_characterization_to_dict`/`_power_characterization_from_dict` to
`Hdf5AdapterV1_0` (deliberately a separate copy from `Hdf5AdapterV1_1`'s identically-shaped
methods, not a shared import — matching this file pair's existing, deliberate
"no coupling between version dict-helpers" convention) and wiring them into both
`_clearbox_to_dict`/`_light_source_to_dict` and `config_from_dict`.

**Fixtures regenerated**: `fixtures/reference_output.json`/`.sha256` and
`fixtures/synthetic_2laser.h5`, via `tools/generate_fixtures.py`. Reviewed against
`docs/contributing.md`'s checklist (machine name, config hash length, build plate,
per-train scanner values, thermal lensing flags, SFCF file sizes, no OPCUA fields) — all
confirmed, plus a new check specific to this change: both trains' `power_characterization`
now genuinely present with real derived values, not silently dropped.

**Tests**: updated `test_writer_roundtrip.py` (known-value factories now build
`power_characterization` via the forward functions; field-specific assertions replaced
with `power_characterization` equivalents), `test_v1_1_adapter.py` (`_mock_v1_1_config`
drastically simplified — `MockConfigBuilder` already builds `power_characterization`
natively now, so it's just the v1.0 mock with `file_version` overridden; every assertion
touching a removed field rewritten to check `power_characterization` instead; one test,
`test_v1_0_write_never_invokes_fallback_when_native_present`, removed outright — its
entire premise, a fallback branch to avoid invoking, no longer exists), and
`test_adapter_migration.py` (the unrelated mock-v1.1-architecture test's own
self-contained mock reader/writer/fixtures also constructed `ClearBox`/`LightSource`
directly — same field removals, no `power_characterization` involvement needed since
that test's own concept is `facility_id`/`config_author`, unrelated).

**Verification: 402/402 tests pass** (`python -m pytest python/tests/`), zero
regressions, zero skips. `tools/generate_capabilities.py --check` clean.
`tools/cross_check.py` **not run** — deferred until all 5 languages convert, per this
plan's sequencing note above.

**Noted, not fixed (pre-existing, unrelated to this work)**: `scratch/smoke_test.py`
(gitignored, untracked, not part of the reviewed test suite) fails with
`AttributeError: 'OpticalTrain' object has no attribute 'clearbox'` — it calls
`t0.clearbox` directly instead of `t0.optional_components.clearbox`, which was never a
valid attribute path at any point in this session's work. Confirmed pre-existing and
unrelated to this change; left alone as out of scope.

---

## Rust — Done (2026-09-02)

Implemented exactly per §2–§4 below, with every file:line citation re-verified against
the real pre-change source before editing (a few had shifted slightly from the plan's
original drafting — noted inline where relevant).

- **StableModel** (`models.rs`): removed all 4 fields from `ClearBox`/`LightSource`
  and their doc comments referencing them; also updated two embedded `#[cfg(test)]`
  struct literals in this same file (a `sample_config()`-style helper and a
  `binary_fields_omitted_by_default_and_present_when_set` test's inline `ClearBox`)
  that the plan didn't separately enumerate — found by letting the compiler surface
  every call site rather than relying on grep alone.
- **v1.0 reader** (`v1_0/hdf5.rs`): `parse_clearbox`/`parse_light_source` restructured
  to bind the raw attrs to locals first (previously inlined directly into the struct
  literal with `?`), then forward-derive `power_characterization`, exactly as planned.
- **v1.0 writer** (`v1_0/writer.rs`): both blocks simplified to the unconditional
  `backward_flat_fields_*` call — `backward_flat_fields_coefficients`/`_points` are
  infallible (return a plain tuple, not `Result`), so no `?` needed.
- **v1.1 reader/writer** (`v1_1/hdf5.rs`, `v1_1/writer.rs`): removed the now-invalid
  `None` struct-literal fields and the `match`-based fallback-derivation blocks;
  deleted the now-unused `forward_power_characterization_*` import from `v1_1/writer.rs`
  entirely (confirmed `MachineConfigError` is still used elsewhere in that file, so only
  the one import line needed trimming, not the whole `use` block).
- **`MockConfigBuilder`** (`builder.rs`): both `build_train`'s `LightSource` and
  `mock_clearbox`'s `ClearBox` now build `power_characterization` via
  `forward_power_characterization_coefficients`/`_points` using the same literal values
  as before. Neither surrounding function returns `Result` (`fn mock_clearbox(index: usize) -> ClearBox`,
  plain), so the derivation call is unwrapped via `.expect("mock power characterization
  literals are always valid")` rather than threaded through as an error — appropriate
  here since these are fixed, known-valid literals, not arbitrary input; matches this
  codebase's convention of not adding error handling for scenarios that can't happen.
- **`create()`** (`capabilities/v1_1/file.rs`): confirmed via direct inspection that
  `MockConfigBuilder::new(1).build()` was the *only* input to the explicit derivation
  loop — deleted the loop outright, along with the now-unused
  `forward_power_characterization_*` import, exactly as the plan's resolved open item
  anticipated.
- **Module documentation** (`power_characterization.rs`): the module's own doc comment
  described the old Phase 2 fallback design in detail ("each writer first looks at its
  own native field, and only derives from the other shape when its own is absent...
  both readers stay exactly as simple as every other version's reader"). That's no
  longer accurate — v1.0's reader now calls this module unconditionally as part of
  ordinary parsing, not as a writer-side fallback. Rewritten to describe the current
  architecture accurately, with the old fallback design kept as explicit historical
  context (why the conversion was originally kept out of the readers) rather than
  deleted outright.
- **Schema**: no Rust-specific schema file exists (per `PARITY_AUDIT.md`, schema
  validation is Python/Node.js-only) — the canonical `schema/machine_config_v1.schema.json`
  and `schema/capabilities/models.yaml` edits from Python's pass already cover this.

**Confirmed, not assumed: no equivalent to Python's hidden-JSON-serialization bug.**
Rust's JSON output is `serde_json::to_string(&config)` directly on the struct — no
hand-maintained dict layer to go stale. `power_characterization` already carried
`#[serde(skip_serializing_if = "Option::is_none", default)]` before this change, so it
was always going to serialize correctly the moment the field was populated. Verified
empirically, not just reasoned about: `reader::tests::matches_python_golden_file` (which
diffs Rust's JSON output for the reference fixture against Python's real,
just-regenerated `reference_output.json`) passed on the first run.

**Tests**: `rust/tests/v1_1_adapter_test.rs` rewritten in lockstep with Python's
equivalent rewrite (`mock_v1_1_config` drastically simplified to just the v1.0 mock with
`file_version` overridden; every assertion touching a removed field rewritten to check
`power_characterization`; `test_v1_0_write_never_invokes_fallback_when_native_present`
removed outright, same reasoning as Python's identical removal). `rust/tests/mock_v1_1/mod.rs`
(the unrelated mock-v1.1-architecture test's self-contained mock reader/writer) — same
field removals, no `power_characterization` involvement needed.

**Verification: 200/200 tests pass** (`cargo test --manifest-path rust/Cargo.toml`) —
99 lib + 5 adapter_migration + 18 capabilities + 30 integration + 21
power_characterization + 19 v1_1_adapter (was 20; one removed) + 6
version_adapter_isolation + 2 doctests. Zero regressions, zero warnings.
`tools/cross_check.py` **not run** — deferred per this plan's sequencing note.

---

## C++ — Done (2026-09-02)

Implemented exactly per §2–§4 below, with every file:line citation re-verified against
the real pre-change source before editing.

- **StableModel** (`models.hpp`): removed all 4 fields from `ClearBox`/`LightSource`
  and their doc comments, plus their `to_json`/`from_json` hand-written entries — both
  the struct-literal serializers and the corresponding `from_json` reads. Re-verified
  the Phase 1 "new-fields-silently-dropped" bug class does not recur here: every other
  field in both functions is still listed, nothing else was accidentally dropped while
  editing.
- **v1.0 reader** (`v1_0/hdf5.hpp`): `parseClearBox`/`parseLightSource` restructured to
  bind the raw attrs to locals first (`voltsToWattsAlgorithm`/`Params`,
  `wattsToVoltsAlgorithm`/`Params`), then forward-derive `power_characterization` via
  `forwardPowerCharacterizationCoefficients`/`Points`, exactly as planned. Added the
  `#include "machine_config/power_characterization.hpp"` this file didn't previously need
  (it's header-only, no separate translation unit to link).
- **v1.0 writer** (`v1_0/writer.hpp`): both `ClearBox`/`LightSource` blocks simplified
  from the ternary-fallback pattern to the unconditional
  `backwardFlatFieldsCoefficients`/`Points` call.
- **v1.1 reader/writer** (`v1_1/hdf5.hpp`, `v1_1/writer.hpp`): removed the now-invalid
  `std::nullopt` struct-literal assignments and the ternary fallback-derivation
  expressions (`ls.power_characterization ? ls.power_characterization :
  forwardPowerCharacterizationPoints(...)` and the `ClearBox` equivalent); deleted the
  now-unused `#include "machine_config/power_characterization.hpp"` from
  `v1_1/writer.hpp` entirely, since nothing else in that file called its functions
  (confirmed via grep before removing — `PowerCharacterization` the *type* is still used
  there, but that comes from `models.hpp`, already included).
- **`MockConfigBuilder`** (`builder.hpp`): both `mockClearbox`'s `ClearBox` and the
  `LightSource` mock now build `power_characterization` via
  `forwardPowerCharacterizationCoefficients`/`Points` using the same literal values as
  before (`"LINEAR"`/`"50.0,100.0"`; `"LINEAR"`/`"[1,100,10,1000]"`). No `.expect()`-style
  unwrap needed here (unlike Rust) — the field itself is `std::optional<PowerCharacterization>`,
  and the forward functions already return exactly that type, so the assignment is direct.
- **`create()`** (`capabilities/v1_1/file.hpp`): confirmed via direct inspection that
  `MockConfigBuilder{}.build()` was the *only* input to the explicit derivation loop over
  `config.optical_trains` — deleted the loop outright, exactly as the plan's resolved
  open item anticipated.
- **Module documentation** (`power_characterization.hpp`): same staleness Rust's module
  doc had — described the old Phase 2 fallback design in detail. Rewritten to describe
  the current architecture (v1.0 reader always forward-derives; v1.0 writer always
  backward-derives; v1.1 reads/writes natively), with the old fallback design kept as
  explicit historical context.
- **Schema**: no C++-specific schema file exists (confirmed by grep — no
  `volts_to_watts_algorithm` hits anywhere under `cpp/` before this session's edits
  except the header/test files this plan already lists) — the canonical
  `schema/machine_config_v1.schema.json` and `schema/capabilities/models.yaml` edits
  from Python's pass already cover this.

**Confirmed, not assumed: no equivalent to Python's hidden-JSON-serialization bug.**
C++'s `to_json(ClearBox)`/`to_json(LightSource)` are hand-written but directly reference
`c.power_characterization`/`l.power_characterization` on the struct — there's no separate
dict-hydration layer that could go stale independently, the way Python's
`_config_to_dict` did. Verified empirically: `test_reader.cpp`'s golden-file comparison
against `fixtures/reference_output.json` (Python's real, just-regenerated output) passed
on the first run, as part of the full suite.

**Tests**: `tests/test_v1_1_adapter.cpp` rewritten in lockstep with Python's and Rust's
equivalent rewrites — `mockV1_1Config` drastically simplified to just the v1.0 mock with
`file_version` overridden (`MockConfigBuilder` already builds `power_characterization`
natively now); every assertion touching a removed field rewritten to check
`power_characterization` instead, using two new small helpers (`constantValues`/
`pointValues`) to flatten a `PowerCharacterization`'s constants/points to a
`vector<double>` for comparison, since the flat CSV string no longer exists to compare
against directly; the two "unrecognized Algorithm_Type" tests now construct via
`forwardPowerCharacterizationCoefficients`/`Points` directly instead of setting the
removed scalar fields; `"v1.0 write never invokes the fallback when native data is
present"` removed outright — its entire premise, a fallback branch to avoid invoking, no
longer exists, same reasoning as Python's and Rust's identical removals.
`tests/test_reader.cpp`'s v1.0 `ClearBox` read-assertion updated from checking
`volts_to_watts_algorithm` to `power_characterization->algorithm_type`.
`tests/mock_v1_1.hpp` (the unrelated mock-v1.1-architecture test's self-contained mock
reader/writer, used by `test_adapter_migration.cpp`) — same field removals, no
`power_characterization` involvement needed since that mock's own concept is
`facility_id`/`config_author`, unrelated.

**Verification: 154/154 tests pass, 823 assertions** (`ctest -C Debug` /
`machine_config_tests.exe` directly), via the full solution build (`cmake --build .
--config Debug`), which also confirmed the CLI, `quickstart`, and `full_workflow` example
targets still compile cleanly against the changed StableModel. Zero regressions.
`tools/cross_check.py` **not run** — deferred per this plan's sequencing note.

---

## Go — Done (2026-09-02)

Implemented exactly per §2–§4 below, with every file:line citation re-verified against
the real pre-change source before editing.

- **StableModel** (`internal/models/models.go`): removed all 4 fields from
  `ClearBox`/`LightSource` and their `json:"..."` tags, plus the doc comments referencing
  them. Confirmed `go/models.go`'s root-level type aliases needed no separate edit — pure
  `= models.X` aliases, as the plan anticipated.
- **v1.0 reader** (`capabilities/v1_0/hdf5/hdf5.go`): `parseLightSource`/`parseClearBox`
  restructured to read the raw attrs into locals first, then forward-derive
  `PowerCharacterization` via `ForwardPowerCharacterizationPoints`/`Coefficients` —
  unlike the other 4 languages, these functions return `(*PowerCharacterization, error)`
  in Go, so both call sites now propagate that error the same way every other field in
  these functions already does (`if err != nil { return X{}, err }`); confirmed both
  callers (`parseTrain`, `parseClearBox`'s own caller) already handled a returned error
  correctly, so no further propagation changes were needed up the call chain.
- **v1.0 writer** (`capabilities/v1_0/hdf5/writer.go`): both `ClearBox`/`LightSource`
  blocks simplified from the conditional-fallback pattern to the unconditional
  `BackwardFlatFieldsCoefficients`/`Points` call.
- **v1.1 reader/writer** (`capabilities/v1_1/hdf5/hdf5.go`, `.../writer.go`): removed the
  now-invalid `WattsToVoltsAlgorithm: nil,`/`VoltsToWattsAlgorithm: nil,` struct-literal
  lines and the `if pc == nil { pc, err = ForwardPowerCharacterization*(...) }`
  fallback-derivation blocks for both `ClearBox` and `LightSource` — replaced with a
  direct `pc := ls.PowerCharacterization` / `cb.PowerCharacterization`, used directly.
- **`MockConfigBuilder`** (`builder.go`): both `mockClearbox`'s `ClearBox` and the
  `LightSource` mock now build `PowerCharacterization` via two new small helpers,
  `mustPowerCharacterizationCoefficients`/`Points`, using the same literal values as
  before (`"LINEAR"`/`"50.0,100.0"`; `"LINEAR"`/`"[1,100,10,1000]"`). Added the
  `machine-config-go/internal/models` import this file didn't previously need. The
  helpers `panic` on error rather than threading one through this non-error-returning
  builder — appropriate here since these are fixed, known-valid literals that the forward
  functions can only fail on for a genuinely malformed CSV float, never the case for
  these constants.
- **`Create()`** (`capabilities/v1_1/file.go`): confirmed via direct inspection — no
  change needed. Go's `Create()` hand-builds its mock `OpticalTrain` inline
  (`LightSource{Manufacturer: ..., Model: ..., SerialNumber: ...}`, no `ClearBox` at
  all) and never touched either removed field or `PowerCharacterization` to begin with,
  exactly as the plan's resolved open item anticipated.
- **Module documentation** (`internal/models/power_characterization.go`,
  `capabilities/v1_1/hdf5/migrate.go`): both carried the same staleness Rust's and C++'s
  module docs had — described the old Phase 2 fallback design in detail. Rewritten to
  describe the current architecture (v1.0 reader always forward-derives; v1.0 writer
  always backward-derives; v1.1 reads/writes natively), with the old fallback design kept
  as explicit historical context in both files.
- **Schema**: no Go-specific schema file exists — confirmed via `PARITY_AUDIT.md` (schema
  validation is Python/Node.js-only) and by grep (zero hits for the removed field names
  under `go/` before this session's edits, outside the files this plan already lists).
- **`internal/mockv1_1/reader.go`/`writer.go`**: unlike the other 4 languages' mock v1.1
  adapters, Go's mock *does* reference `internal/models.ClearBox`/`LightSource` directly
  (no dot-import boundary separating it) — confirmed by grep during this session, exactly
  the "matched the grep, confirm rather than assume" case the plan flagged. Removed the 4
  field references (2 in `reader.go`'s `parseLightSource`/`parseClearBox`, 2 in
  `writer.go`'s corresponding write functions) — same reasoning as everywhere else: this
  mock's own concept is `facility_id`/`config_author`, unrelated to power characterization.

**Go has no automated golden-file/cross-language JSON comparison test of its own**
(confirmed by grep — `doc.go` only *documents* that Go's JSON matches
`fixtures/reference_output.json`; the actual byte-for-byte check is
`tools/cross_check.py`, deferred until all 5 languages convert). `TestV1_1AdapterRead`'s
own JSON-marshal assertion (that `power_characterization` and `firmware_version` actually
appear in `encoding/json`'s reflection-driven output, not just in the struct tags) still
passed unchanged — Go's serialization is derived from the same struct fields being
edited, so there's no separate hand-written dict layer that could have gone stale the way
Python's did.

**Tests**: `v1_1_adapter_test.go` rewritten in lockstep with the other 4 languages'
equivalent rewrites — `mockV1_1Config` drastically simplified to just the v1.0 mock with
`FileVersion` overridden (`MockConfigBuilder` already builds `PowerCharacterization`
natively now); added `constantValues`/`pointValues` helpers (mirroring C++'s) to flatten
a `PowerCharacterization`'s constants/points to a `[]float64` for comparison, since the
flat CSV string no longer exists to compare against directly; every assertion touching a
removed field rewritten to check `PowerCharacterization` instead; both "unrecognized
Algorithm_Type" tests now construct via `ForwardPowerCharacterizationCoefficients`/
`Points` directly instead of setting the removed scalar fields (needed local variable
renames — `pc`/`err` → `inputPC`/`err` — to avoid Go's "no new variables on left side of
:=" redeclaration error against the later `pc := result...PowerCharacterization` in the
same scope); `TestV1_1AdapterV1_0WriteNeverInvokesFallbackWhenNativePresent` removed
outright — its entire premise, a fallback branch to avoid invoking, no longer exists,
same reasoning as the other 4 languages' identical removals. The now-fully-unused
`parseCSVToFloats` helper and its now-unused `strconv` import were removed rather than
left as dead code.

**Verification: 132/132 top-level tests pass** (`go test ./... -count=1 -v`, counted via
`--- PASS` lines), across all packages with test files (`machine-config-go`,
`machine-config-go/capabilities`, `machine-config-go/internal/h5c`). `go vet ./...`
clean. Built via the MSYS2 MinGW64 toolchain this repo's own CI
(`.github/workflows/go.yml`) uses (`CGO_ENABLED=1`, `CC=/mingw64/bin/gcc.exe`) — this
session's default environment has `CGO_ENABLED=0` and no system HDF5, so build
constraints exclude the CGo-based `internal/h5c` package entirely unless pointed at the
MSYS2 install explicitly, same as CI's Windows job does. Zero regressions.
`tools/cross_check.py` **not run** — deferred per this plan's sequencing note.

---

## Node.js — Done (2026-09-02)

Implemented exactly per §2–§4 below, with every file:line citation re-verified against
the real pre-change source before editing.

- **StableModel** (`src/models.ts`): removed all 4 fields from the `ClearBox`/
  `LightSource` interfaces and the doc comments referencing them.
- **v1.0 reader** (`capabilities/v1_0/hdf5.ts`): `parseLightSource`/`parseClearBox`
  restructured to forward-derive `power_characterization` via
  `forwardPowerCharacterizationPoints`/`Coefficients(attrStr(a, "..."), attrStr(a,
  "..."))`, with `?? undefined` on the result — needed here for the same reason as the
  `firmware_version` fix mentioned in this plan's §3 Node.js notes:
  `JSON.stringify` keeps a `null`-valued property but drops an `undefined` one, and this
  field's convention is "omitted entirely when absent," not `null`. Added the
  `../../powerCharacterization.js` import this file didn't previously need.
- **v1.0 writer** (`capabilities/v1_0/writer.ts`): both `ClearBox`/`LightSource` blocks
  simplified from the `let`-then-conditional-reassign fallback pattern to a single
  `const [algorithm, params] = backwardFlatFields*(...)` call.
- **v1.1 reader/writer** (`capabilities/v1_1/hdf5.ts`, `.../writer.ts`): removed the
  now-invalid `volts_to_watts_algorithm: null,`/`watts_to_volts_algorithm: null,`
  object-literal entries and the `ls.power_characterization ??
  forwardPowerCharacterization*(...)` fallback expressions for both `ClearBox` and
  `LightSource` — replaced with the field passed straight through
  (`writePowerCharacterization(..., ls.power_characterization)`), since that helper
  already accepts `PowerCharacterization | null | undefined`. Removed the now-unused
  `forwardPowerCharacterizationCoefficients`/`Points` import from `v1_1/writer.ts`
  entirely.
- **`MockConfigBuilder`** (`src/builder.ts`): unlike the other 4 languages, Node's mock
  builder had **never** populated `power_characterization` for its `ClearBox`/
  `LightSource` (the doc comments explicitly said "not populated by the mock builder,
  only real v1.1 fixtures exercise this field") — since the flat fields it *did*
  hardcode are now gone, this was a forced fix, not optional: both blocks now build
  `power_characterization` via `forwardPowerCharacterizationCoefficients`/`Points` using
  the same literal values as before (`'LINEAR'`/`'50.0,100.0'`;
  `'LINEAR'`/`'[1,100,10,1000]'`), bringing Node's mock output in line with the other 4
  languages' for the first time.
- **`create()`** (`capabilities/v1_1/file.ts`): confirmed via direct inspection that
  `new MockConfigBuilder({ nLasers: 1 }).build()` was the *only* input to the explicit
  `.map()` derivation loop over `optical_trains` — deleted the loop outright, along with
  the now-unused `forwardPowerCharacterizationCoefficients`/`Points` import, exactly as
  the plan's resolved open item anticipated.
- **Module documentation** (`src/powerCharacterization.ts`,
  `capabilities/v1_1/hdf5.ts`'s closing comment block): both carried the same staleness
  the other 4 languages' module docs had — described the old Phase 2 fallback design in
  detail. Rewritten to describe the current architecture, with the old fallback design
  kept as explicit historical context. **One near-miss caught by
  `versionAdapterIsolation.test.ts`** (the raw-text-scan isolation test, not a parser):
  an earlier wording of the `v1_1/hdf5.ts` rewrite said "`capabilities/v1_0`'s
  reader/writer always forward/backward-derive it," and the literal text `v1_0` inside a
  `v1_1` file failed the isolation test — exactly the "C++'s near-miss" class of mistake
  this plan's §5 already called out as a risk. Reworded to describe the behavior without
  naming the other version, and the isolation test passed clean.
- **Schema**: `schema/machine_config_v1.schema.json` (canonical) and
  `schema/capabilities/models.yaml` were already fixed by Python's pass — no further
  source edit needed. `nodejs/schema/` (the git-ignored, build-time copy made by
  `scripts/copy-schema.mjs`) was still the **pre-change** canonical schema on disk in
  this working copy — since it's a byte-for-byte `cpSync` mirror with no
  language-specific edits, this was a **stale local build artifact, not a bug**;
  re-running `node scripts/copy-schema.mjs` (equivalent to `npm run prebuild`) refreshed
  it and the one failing test (`schema.test.ts`'s "bundled schema matches canonical")
  went green immediately.

**Confirmed, not assumed: Node.js has no golden-file/cross-language JSON comparison test
of its own** (grep for `reference_output` under `nodejs/tests/` returns nothing) — that
check is `tools/cross_check.py`, deferred until all 5 languages convert, same situation
as Go's. `v1_1Adapter.test.ts`'s own `toJson()` assertion (that `power_characterization`
and `firmware_version` actually appear in the real serialized output, not just in the
type definitions) still passed unchanged.

**Tests**: `tests/v1_1Adapter.test.ts` rewritten in lockstep with the other 4 languages'
equivalent rewrites — `mockV1_1Config` drastically simplified to just the v1.0 mock with
`file_version` overridden (`MockConfigBuilder` already builds `power_characterization`
natively now); every assertion touching a removed field rewritten to check
`power_characterization` instead; both "unrecognized Algorithm_Type" tests now construct
via `forwardPowerCharacterizationCoefficients`/`Points` directly instead of setting the
removed scalar fields; "the writer never invokes the fallback when native data is
present" removed outright — its entire premise, a fallback branch to avoid invoking, no
longer exists, same reasoning as the other 4 languages' identical removals.
`tests/mockV1_1.ts` (the unrelated mock-v1.1-architecture test's self-contained mock
reader/writer, used by `adapterMigration.test.ts`) — same 4 field removals, no
`power_characterization` involvement needed since that mock's own concept is
`facility_id`/`config_author`, unrelated.

**Verification: 235/235 tests pass across all 9 test files** (`npm run test` /
`vitest run`), `npm run typecheck` clean, `npm run build` (full `tsc` compile, not just
`--noEmit`) clean. Zero regressions. `tools/cross_check.py` **not run** — deferred per
this plan's sequencing note; this was the last of the 5 languages, so cross-check is now
unblocked.

---

## Next steps

All 5 languages have landed this change and pass their own full test suites
independently, cross-check is clean, fixtures are regenerated and reviewed, and §6
documentation is updated. One item remains, deliberately left for the user rather than
done automatically:

1. ~~Full `python tools/cross_check.py --verbose` across all 5 languages, all 5
   phases.~~ **Done (2026-09-02)** — see Verification item 3 above.
2. ~~Regenerate/re-review `fixtures/reference_output.json`/`.sha256` and
   `fixtures/synthetic_2laser.h5`.~~ **Done (2026-09-02)** — see Verification item 4
   above. This surfaced real drift: the committed fixture on disk still predated this
   entire plan (last touched 2026-07-30, before Power Characterization Unification
   began) — not a change introduced by this plan, but this was the right moment to
   catch and fix it.
3. ~~§6 Documentation updates.~~ **Done (2026-09-02)** — `docs/migrations/v1_0_to_v1_1.md`,
   `docs/contributing.md`, `V1_1_IMPLEMENTATION_PLAN.md`, and `PARITY_AUDIT.md` all
   updated. See Verification item 5 above.
4. **Commit(s), landed as a plain `feat`** per this plan's "Not a breaking change —
   decided" section — intentionally not done here; the user will review the changed
   files first and take care of the commit themselves.

---

## Context

Phase 2 gave `ClearBox`/`LightSource` a writer-side fallback: write the version's own
native field if present, else derive it from the other version's shape. That kept
**both** representations alive on the StableModel side by side —
`volts_to_watts_algorithm`/`volts_to_watts_params` (v1.0's flat shape) and
`power_characterization` (v1.1's structured shape) — with exactly one populated and the
other `None`, depending on which version a file was read as.

That asymmetry caused a real, confirmed integration bug: `clearbox-tauri`'s own pipeline
only ever read the legacy scalar field. Every v1.0 file worked; every v1.1 file silently
produced an empty value with no compiler or type-level signal that the data had moved,
not disappeared — see `capture_services/mod.rs:611-613` → `pipeline.rs:1521` →
`hdf5/writer.rs:1042-1046`'s hardcoded `"Linear"` fallback in that project for the full
chain.

**The fix is not to keep both fields in sync — it's to stop having two fields.**
`PowerCharacterization` is already a lossless, bidirectional representation of v1.0's
flat shape (proven by the existing round-trip tests in every language's
`power_characterization_test.*`). There is nothing v1.0's `Volts_To_Watts_Algorithm`/
`Volts_To_Watts_Params` (or `LightSource`'s `Watts_To_Volts_*`) can express that
`PowerCharacterization` cannot already hold. Once that's true, keeping a second field
around only gives every consumer a second place to have to remember to check.

This plan removes `ClearBox.volts_to_watts_algorithm`/`volts_to_watts_params` and
`LightSource.watts_to_volts_algorithm`/`watts_to_volts_params` from the StableModel in
all 5 languages, leaving `power_characterization` as the **only** representation of this
concept, for files of either version.

---

## Scope confirmation — this is the only instance of this pattern

Before drafting this plan, checked whether any other v1.0→v1.1 change has the same
shape (a v1.1 addition that semantically *replaces* an existing v1.0 field, left sitting
alongside the field it replaces) — **confirmed it's isolated to this one concept**, two
independent ways:

1. **Code-level signal.** The word "supersedes" appears in exactly one relationship,
   consistently across every language's models file, and nowhere else: `power_characterization`
   supersedes `volts_to_watts_algorithm`/`volts_to_watts_params` (`ClearBox`) and
   `watts_to_volts_algorithm`/`watts_to_watts_params` (`LightSource`). Repo-wide grep for
   `supersede` across all 5 languages' source returns exactly 20 files, all of them part
   of this one relationship (the struct comments plus the v1.1 fallback-logic sites and
   their tests) — nothing else in the codebase is annotated this way.
2. **Manual pass over all 37 field-by-field rows across all 5 Changes** in
   `docs/migrations/v1_0_to_v1_1.md`. Every other StableModel-affecting row is one of two
   unproblematic shapes: genuinely new with no v1.0 analog (`ClearBox.firmware_version` —
   its own comment says "no on-disk source there," nothing to reconcile), or genuinely
   removed with no v1.1 replacement (Change 1's 6 removed ClearBox attrs; Change 5's
   `AxisConfig.tuning_parameters`/`tuning_type`) — asymmetric, but nothing on the other
   side claims to represent the same concept in a new shape, so there's no second field
   to collapse into the first. `Output_Path`/`Software_Trigger_Delay`'s Consolidate and
   every plain Path/Name rename keep exactly one StableModel field throughout.

No other scope adjustment is needed beyond `ClearBox`/`LightSource.power_characterization`.

---

## Not a breaking change — decided

`docs/contributing.md`'s StableModel rules require a "major breaking change review"
before removing a field. This plan **is** that review, and the conclusion is: **commit
this as a plain `feat`, not `feat!`/`BREAKING CHANGE:`.**

Reasoning (2026-09-02): nothing has ever been promoted to the `release` branch — every
version so far is a `main`-branch `rc` prerelease. "Breaking" presumes there's a prior
*released*, stable contract being broken, and none exists yet; the entire project to
date is still pre-first-release iteration. Flagging this as breaking wouldn't just
describe this change — because `main`'s `rc` train and `release` share one version
lineage, it would silently decide that the **first-ever real release** of this library
becomes `1.0.0` (dragging along every unrelated `feat` accumulated in the same train),
the moment this train is eventually promoted. That's a real milestone ("this library is
now stable/production-ready") worth choosing deliberately, later, on its own — not one
that should get set as a side effect of an internal StableModel refactor.

`clearbox-tauri` being actively co-developed alongside this library (not a frozen
integration that would silently break) is exactly why doing this cleanup now, while it's
cheap, is still the right call — it's just not what makes it "breaking" in the semver
sense.

---

## Target shape

```diff
 struct ClearBox {
     ...
-    volts_to_watts_algorithm: Option<String>,
-    volts_to_watts_params: Option<String>,
     firmware_version: Option<String>,
     power_characterization: Option<PowerCharacterization>,
     ...
 }

 struct LightSource {
     ...
-    watts_to_volts_algorithm: Option<String>,
-    watts_to_volts_params: Option<String>,
     power_characterization: Option<PowerCharacterization>,
     ...
 }
```

`PowerCharacterization` itself, `EquationConstant`, and `CalibrationPoint` are
**unchanged** — this plan only removes the redundant fields, it doesn't touch the
structure that replaces them.

### How responsibility shifts, per adapter

This is the part worth being precise about — it's not symmetric:

| Adapter | Today (Phase 2) | After this plan |
|---|---|---|
| v1.0 reader | Populates the flat scalar fields directly from disk. Never touches `power_characterization`. | **Gains** forward derivation: parse the flat fields, call `forward_power_characterization_coefficients`/`_points`, populate `power_characterization`. This is the only field left to populate. |
| v1.0 writer | Writes its own scalar fields if present; falls back to `backward_flat_fields_*` only if both are `None` and `power_characterization` is present. | **Simplifies** to unconditional: there's no scalar field to check anymore, so always call `backward_flat_fields_coefficients`/`_points` on `power_characterization` and write the result. |
| v1.1 reader | Reads `Power_Characterization` natively into `power_characterization`; separately sets the scalar fields to `None` (no on-disk source). | **Simplifies**: drop the now-invalid `None` assignments for the removed fields. The native read of `power_characterization` is unchanged. |
| v1.1 writer | Writes `power_characterization` if present; falls back to `forward_power_characterization_*` from the scalar fields only if `power_characterization` is `None`. | **Simplifies** to unconditional: there's no scalar field to fall back to anymore, so just write whatever is in `power_characterization` (including `None` — schema-complete-empty-group convention is unchanged). The `forward_power_characterization_*` call and its import can be deleted from the writer entirely. |

Net effect: all the derivation logic collapses onto v1.0's adapters (which now do the
work v1.1's writer used to do as a fallback), and v1.1's adapters get **simpler** than
they are today, not just different.

---

## 1 — Schema

- `schema/capabilities/models.yaml`: remove `volts_to_watts_algorithm`/
  `volts_to_watts_params` from `ClearBox`; remove `watts_to_volts_algorithm`/
  `watts_to_volts_params` from `LightSource`.
- `schema/machine_config_v1.schema.json` (canonical) and its known copies
  (`python/src/machine_config/machine_config_v1.schema.json`; confirm at implementation
  time whether any other language bundles a copy — `nodejs/schema/` is generated at
  build time via `scripts/copy-schema.mjs`, not hand-edited): remove the same 4
  properties from the `clearbox`/`light_source` definitions. Both are
  `additionalProperties: false`, so this must be a real removal, not just relaxing a
  `required` list.
- `tools/generate_capabilities.py`: **no action needed — verified, not assumed.**
  Researched directly (2026-09-02): this tool is current, actively-maintained
  infrastructure, not tech debt — introduced 2026-08-12 (`feat: refactor to support
  stable interfaces`) and last touched 2026-08-31, the commit immediately before this
  branch's v1.1 work began, as part of standardizing the facade layer across languages
  (`DISPATCH_REGISTRY_PLAN.md`'s effort). It generates `capabilities/generated.{ts,py,rs,hpp}`
  for **4 of 5 languages** — Python, TypeScript, Rust, and C++ — reading its output list
  directly from `main()`. **Go is deliberately not covered**; its `capabilities/` layer
  is hand-authored, consistent with `docs/contributing.md`'s adapter table and with how
  every bit of Go's capabilities work this session was done by hand, no codegen
  involved.
  - **Important correction to an earlier assumption in this plan**: Rust does get a
    full generated `MachineConfigFile` trait today (`render_rs`, `generate_capabilities.py:174-239`)
    — the complete get/set surface, not just `SetMode`. The "Rust only gets `SetMode`"
    note that shaped this plan's first draft was stale, describing a gap that
    `e75cefc` (2026-08-31) already closed before this branch started.
  - **Why re-running it is a genuine no-op here, not just an expected one**: every
    `render_*` function explicitly discards its `models`/`api` parameters
    (`_ = api, models`) and renders a fully hardcoded template — the tool loads
    `models.yaml` only to confirm it parses and that `bindings/*.yaml` exist, it never
    actually reads field names out of it to drive generation. Combined with the facade
    being whole-struct (`get_clearbox`/`set_clearbox` take/return the entire `ClearBox`,
    never an individual field), removing `volts_to_watts_algorithm`/`volts_to_watts_params`/
    `watts_to_volts_algorithm`/`watts_to_volts_params` from `models.yaml` cannot change
    this tool's output at all. `python tools/generate_capabilities.py --check` should be
    run once after the schema edit purely to confirm zero drift, not because a change is
    expected.

---

## 2 — StableModel field removal, per language

| Language | File | Fields to remove |
|---|---|---|
| Python | `python/src/machine_config/models.py` | `ClearBox.volts_to_watts_algorithm`, `.volts_to_watts_params`; `LightSource.watts_to_volts_algorithm`, `.watts_to_volts_params` |
| Rust | `rust/src/models.rs` | same 4, plus their `#[serde(...)]` attributes |
| C++ | `cpp/include/machine_config/models.hpp` | same 4 struct members, plus their `to_json`/`from_json` hand-written entries — confirm the existing `to_json` doesn't silently drop other fields the way it once did for `firmware_version`/`power_characterization` during Phase 1 (real bug then; re-check now while editing this exact function) |
| Go | `go/internal/models/models.go` | `ClearBox.VoltsToWattsAlgorithm`, `.VoltsToWattsParams`; `LightSource.WattsToVoltsAlgorithm`, `.WattsToVoltsParams`, plus their `json:"...,omitempty"` tags. Also check `go/models.go`'s root-level type aliases — expected to be pure aliases needing no separate edit, confirm. |
| Node.js | `nodejs/src/models.ts` | same 4 from the `ClearBox`/`LightSource` interfaces |

---

## 3 — Reader / writer changes, per language

Exact current call sites, verified against the repo as of this plan's drafting:

### Python

- Reader (`capabilities/v1_0/hdf5.py:535-536`, `ClearBox`; `:481-482`, `LightSource` —
  both verified): replace
  `volts_to_watts_algorithm=self._read_str(a, "Volts_To_Watts_Algorithm")` /
  `volts_to_watts_params=self._read_str(a, "Volts_To_Watts_Params")` with a single
  `power_characterization=forward_power_characterization_coefficients(alg, params)` call
  (read `alg`/`params` into locals first, same two attrs, then discard them — nothing
  else in `ClearBox` needs them). `LightSource:481-482` mirrors this with
  `forward_power_characterization_points` on `Watts_To_Volts_Algorithm`/`Params`.
- Also remove the four matching dict round-trip sites already present in this file —
  these are the hand-written JSON dict helpers, not the HDF5 reader, and all four need
  the same treatment: `_config_to_dict`'s `"volts_to_watts_algorithm": cb.volts_to_watts_algorithm` /
  `"volts_to_watts_params": ...` (`hdf5.py:888-889`) and `"watts_to_volts_algorithm": ls.watts_to_volts_algorithm` /
  `"watts_to_volts_params": ...` (`hdf5.py:845-846`); `config_from_dict`'s
  `volts_to_watts_algorithm=cb_d.get("volts_to_watts_algorithm")` /
  `volts_to_watts_params=cb_d.get(...)` (`hdf5.py:1220-1221`) and
  `watts_to_volts_algorithm=ls.get("watts_to_volts_algorithm")` /
  `watts_to_volts_params=ls.get(...)` (`hdf5.py:1150-1151`).
- Writer (`capabilities/v1_0/writer.py:292-296`, `ClearBox`; `:254`, `LightSource` —
  both verified): today —
  ```python
  algorithm, params = cb.volts_to_watts_algorithm, cb.volts_to_watts_params
  if algorithm is None and params is None and cb.power_characterization is not None:
      algorithm, params = backward_flat_fields_coefficients(cb.power_characterization)
  ```
  becomes unconditionally `algorithm, params = backward_flat_fields_coefficients(cb.power_characterization)`.
  `LightSource:254`'s `algorithm, params = ls.watts_to_volts_algorithm, ls.watts_to_volts_params`
  gets the same treatment with `backward_flat_fields_points`.
- v1.1 writer (`capabilities/v1_1/writer.py:330-337` and `396-403`): delete the
  `if pc is None: pc = forward_power_characterization_points(ls.watts_to_volts_algorithm, ls.watts_to_volts_params)`
  and the equivalent `cb.volts_to_watts_algorithm`/`_params` branch — replace with
  `pc = ls.power_characterization` / `pc = cb.power_characterization`, used directly.
  Remove the now-unused `forward_power_characterization_coefficients`/`_points` import
  from this file (`writer.py:39-42`) if nothing else in it still needs them.
- v1.1 reader (`capabilities/v1_1/hdf5.py`): confirm and remove any explicit
  `volts_to_watts_algorithm=None` / `watts_to_volts_algorithm=None` assignments in the
  `ClearBox`/`LightSource` construction blocks (Python dataclasses require every
  non-defaulted field, so these almost certainly exist today).

### Rust

- Reader (`capabilities/v1_0/hdf5.rs:730-731`, `ClearBox`; `:661-662`, `LightSource` —
  both verified): replace
  `volts_to_watts_algorithm: read_str(grp, "Volts_To_Watts_Algorithm")?` /
  `volts_to_watts_params: read_str(grp, "Volts_To_Watts_Params")?` with
  `power_characterization: forward_power_characterization_coefficients(alg.as_deref(), params.as_deref())?`
  (bind `alg`/`params` locally from the same two `read_str` calls first).
  `LightSource:661-662` mirrors this with `Watts_To_Volts_*` /
  `forward_power_characterization_points` — add the
  `use crate::power_characterization::{forward_power_characterization_coefficients, forward_power_characterization_points};`
  import to this file (currently only imported in `writer.rs`).
- Writer (`capabilities/v1_0/writer.rs:383-390`, `ClearBox`; `:336-339`, `LightSource` —
  both verified): today —
  ```rust
  let (algorithm, params) =
      if cb.volts_to_watts_algorithm.is_none() && cb.volts_to_watts_params.is_none() {
          backward_flat_fields_coefficients(cb.power_characterization.as_ref())
      } else {
          (cb.volts_to_watts_algorithm.clone(), cb.volts_to_watts_params.clone())
      };
  ```
  becomes `let (algorithm, params) = backward_flat_fields_coefficients(cb.power_characterization.as_ref());`.
  `LightSource:336-339`'s identically-shaped `if ls.watts_to_volts_algorithm.is_none() && ...`
  gets the same treatment with `backward_flat_fields_points`.
- v1.1 writer (`capabilities/v1_1/writer.rs:478-488` and `559-569`): delete the
  `match ls.power_characterization.as_ref() { Some(pc) => Some(pc), None => { derived_pc = forward_power_characterization_points(...); ... } }`
  pattern (and the `ClearBox` equivalent) — replace with a direct
  `let pc = ls.power_characterization.as_ref();` / `cb.power_characterization.as_ref()`.
  Remove the now-unused `forward_power_characterization_coefficients`/`_points` import
  (`writer.rs:23-25`) if nothing else needs it, and the `derived_pc` locals.
- v1.1 reader (`capabilities/v1_1/hdf5.rs`): remove any explicit
  `volts_to_watts_algorithm: None, volts_to_watts_params: None,` struct-literal fields
  in `ClearBox`/`LightSource` construction (required in Rust struct literals unless
  `..Default::default()` is used — confirm which pattern this file uses).
- `rust/src/error.rs`: no change expected — this plan adds no new error variants.

### C++

- Reader (`capabilities/v1_0/hdf5.hpp:509-511` for `ClearBox`; `:467-468` for
  `LightSource` — both verified): replace
  `cb.volts_to_watts_algorithm = readStr(grp, "Volts_To_Watts_Algorithm"); cb.volts_to_watts_params = readStr(grp, "Volts_To_Watts_Params");`
  with `cb.power_characterization = forwardPowerCharacterizationCoefficients(alg, params);`
  (bind `alg`/`params` from the same two `readStr` calls). `LightSource:467-468` mirrors
  this with `forwardPowerCharacterizationPoints`.
- Writer (`capabilities/v1_0/writer.hpp:311-317` for `ClearBox`; `:269-271` for
  `LightSource` — both verified): today —
  ```cpp
  auto [algorithm, params] =
      (!cb.volts_to_watts_algorithm && !cb.volts_to_watts_params)
          ? backwardFlatFieldsCoefficients(cb.power_characterization)
          : std::make_pair(cb.volts_to_watts_algorithm, cb.volts_to_watts_params);
  ```
  becomes `auto [algorithm, params] = backwardFlatFieldsCoefficients(cb.power_characterization);`.
  `LightSource:269-271`'s identically-shaped ternary gets the same treatment.
- v1.1 writer (`capabilities/v1_1/writer.hpp:348-352` and `408-412`): delete the
  ternary `cb.power_characterization ? cb.power_characterization : forwardPowerCharacterizationCoefficients(cb.volts_to_watts_algorithm, cb.volts_to_watts_params)`
  pattern (and `LightSource`'s) — replace with plain `auto pc = cb.power_characterization;`
  / `ls.power_characterization`.
- v1.1 reader (`capabilities/v1_1/hdf5.hpp`): remove any explicit
  `cb.volts_to_watts_algorithm = std::nullopt;` (or equivalent) initialization for the
  removed fields.
- `models.hpp`'s hand-written `to_json`/`from_json`: remove the 4 fields' entries —
  **re-verify no regression of the Phase 1 "new-fields-silently-dropped" bug class**
  while editing this function, since that's exactly the kind of hand-written
  serialization code where it happened before.

### Go

- Reader (`capabilities/v1_0/hdf5/hdf5.go:537-538` for `ClearBox`; `:462-463` for
  `LightSource` — both verified): replace
  `VoltsToWattsAlgorithm: readStrAttr(g, "Volts_To_Watts_Algorithm"), VoltsToWattsParams: readStrAttr(g, "Volts_To_Watts_Params"),`
  with a `PowerCharacterization` field populated via
  `ForwardPowerCharacterizationCoefficients(alg, params)` (this function currently
  returns `(*PowerCharacterization, error)` — the reader's construction site will need
  to handle the error return, unlike today's direct-assignment pattern; check how the
  v1.1 reader already handles this same call for the pattern to follow). `LightSource:462-463`
  mirrors this with `ForwardPowerCharacterizationPoints`.
- Writer (`capabilities/v1_0/hdf5/writer.go:731-733` for `ClearBox`; `:626` for
  `LightSource` — both verified): today —
  ```go
  cbAlgorithm, cbParams := cb.VoltsToWattsAlgorithm, cb.VoltsToWattsParams
  if cbAlgorithm == nil && cbParams == nil {
      cbAlgorithm, cbParams = BackwardFlatFieldsCoefficients(cb.PowerCharacterization)
  }
  ```
  becomes `cbAlgorithm, cbParams := BackwardFlatFieldsCoefficients(cb.PowerCharacterization)`.
  `LightSource:626-628` (`algorithm, params := ls.WattsToVoltsAlgorithm, ls.WattsToVoltsParams`
  / `if algorithm == nil && params == nil { ... BackwardFlatFieldsPoints(...) }`) is the
  identically-shaped fallback and gets the same unconditional treatment.
- v1.1 writer (`capabilities/v1_1/hdf5/writer.go:787-794` and `941-948`): delete the
  `pc := ls.PowerCharacterization; if pc == nil { pc, err = ForwardPowerCharacterizationPoints(...) }`
  pattern (and `ClearBox`'s) — replace with `pc := ls.PowerCharacterization` /
  `cb.PowerCharacterization`, used directly, dropping the now-unneeded error handling
  for that call specifically.
- v1.1 reader (`capabilities/v1_1/hdf5/hdf5.go`): remove any explicit
  `VoltsToWattsAlgorithm: nil,` struct-literal fields for the removed fields in
  `ClearBox`/`LightSource` construction.
- `go/internal/mockv1_1/reader.go` / `writer.go`: these implement the **mock** v1.1
  adapter used by `DISPATCH_REGISTRY_PLAN.md`'s AV-09–11 tests — check for any
  references to the removed fields (unlikely, since the mock's own concept is
  `facility_id`/`config_author`, unrelated) but confirm rather than assume, since they
  matched the `VoltsToWattsAlgorithm` grep during this plan's drafting.

### Node.js

- Reader (`capabilities/v1_0/hdf5.ts:462-463` for `ClearBox`; `:339-340` for
  `LightSource` — both verified): replace
  `volts_to_watts_algorithm: attrStr(a, "Volts_To_Watts_Algorithm"), volts_to_watts_params: attrStr(a, "Volts_To_Watts_Params"),`
  with `power_characterization: forwardPowerCharacterizationCoefficients(alg, params) ?? undefined,`
  (note the `?? undefined` — same lesson as the `firmware_version` bug fixed this
  session: `forwardPowerCharacterizationCoefficients` returns `null` when both inputs
  are absent, and that must become `undefined` here too, or this reintroduces the exact
  same class of bug on a different field). `LightSource:339-340` mirrors this with
  `forwardPowerCharacterizationPoints`.
- Writer (`capabilities/v1_0/writer.ts:357-360` for `ClearBox`; `:311-312` for
  `LightSource` — both verified): today —
  ```ts
  let voltsAlgorithm = cb.volts_to_watts_algorithm;
  let voltsParams = cb.volts_to_watts_params;
  if (voltsAlgorithm == null && voltsParams == null) {
    [voltsAlgorithm, voltsParams] = backwardFlatFieldsCoefficients(cb.power_characterization);
  }
  ```
  becomes `const [voltsAlgorithm, voltsParams] = backwardFlatFieldsCoefficients(cb.power_characterization);`.
  `LightSource:311-312`'s identically-shaped `let wattsAlgorithm = ls.watts_to_volts_algorithm; let wattsParams = ls.watts_to_volts_params;`
  gets the same treatment with `backwardFlatFieldsPoints`.
- v1.1 writer (`capabilities/v1_1/writer.ts:334-337` and `390-393`): delete the
  `const pc = ls.power_characterization ?? forwardPowerCharacterizationPoints(...)`
  pattern (and `ClearBox`'s) — replace with `const pc = ls.power_characterization;` /
  `cb.power_characterization`.
- v1.1 reader (`capabilities/v1_1/hdf5.ts`): remove any explicit
  `volts_to_watts_algorithm: null,` object-literal entries in `ClearBox`/`LightSource`
  construction (TypeScript will error at compile time on any leftover reference to the
  removed interface members — a useful forcing function, but confirm every site is
  found, since a `null` assigned to a since-removed optional property is still a type
  error worth catching deliberately rather than via a wall of compiler errors).

---

## 4 — `create()` facades and `MockConfigBuilder`

Every language's `MockConfigBuilder` currently hardcodes **both** fields with real
values, verified this session:

| Language | File:line | Current default |
|---|---|---|
| Python | `builder.py:95-96` (ClearBox), `173-174` (LightSource) | `"LINEAR"` / `"50.0,100.0"`; `"LINEAR"` / `"[1,100,10,1000]"` |
| Rust | `builder.rs:261-263` (ClearBox), `153-155` (LightSource) | same values |
| C++ | `builder.hpp:114-116` (ClearBox), `171-173` (LightSource) | same values |
| Go | `builder.go:233-236` (ClearBox); LightSource's equivalent nearby | same values |
| Node.js | `builder.ts:129-131` (ClearBox), `208-210` (LightSource) | same values |

Since the removed fields won't exist, every one of these literals becomes a compile
error (Rust/Go/C++/TS) or a `TypeError`-on-construction (Python) — a forcing function,
not optional cleanup. Replace each with the equivalent `power_characterization:
PowerCharacterization { algorithm_type: Some("LINEAR"), algorithm_equation: Some("W = a*V + b"), derivation_equation_constants: [...], characterization_points: [...] }`
(coefficients-shape for `ClearBox`, points-shape for `LightSource`) — or, simpler and
recommended: call `forward_power_characterization_coefficients("LINEAR", "50.0,100.0")`
/ `forward_power_characterization_points("LINEAR", "[1,100,10,1000]")` from inside the
builder itself, so the mock's derived shape can never drift from what a real v1.0 read
would actually produce.

Python's `builder.py:451-455` (`ls.get("watts_to_volts_algorithm")`-style dict hydration
for `YamlConfigBuilder` or similar) needs the same treatment — confirm exact call site
and purpose during implementation.

**`create()` — resolved, not just likely (2026-09-02):** this only affects Rust, C++,
and Node.js — it is explicitly **not** a Python or Go concern, for opposite reasons.
Python's `create()` relies on a real disk round-trip, so it never needed an explicit
derivation call in the first place (nothing to delete). Go's `Create()` hand-builds its
mock inline without ever touching `ClearBox`/`LightSource`'s power fields at all
(confirmed again this session — still zero changes needed).

For the three affected languages, read each `create()` directly to confirm what feeds
the derivation call, rather than assuming:

- Rust — `capabilities/v1_1/file.rs:60-81`: `create()` calls
  `MockConfigBuilder::new(1).build()` (line 68), then explicitly derives
  `cb.power_characterization`/`ls.power_characterization` (lines 72-81) from that same
  builder's `volts_to_watts_algorithm`/`params` and `watts_to_volts_algorithm`/`params`.
- C++ — `capabilities/v1_1/file.hpp:66-78`: identical shape — `MockConfigBuilder b;`
  (line 66), then explicit `forwardPowerCharacterizationCoefficients`/`Points` calls
  (lines 74, 78) reading that same `b`'s output.
- Node.js — `capabilities/v1_1/file.ts:96-115`: identical shape —
  `new MockConfigBuilder({ nLasers: 1 }).build()` (line 96), then explicit
  `forwardPowerCharacterizationCoefficients`/`Points` calls (lines 103-115) reading that
  same `mock`'s output.

In all three, `MockConfigBuilder`'s own output is the **only** input to the derivation
call — there is no other caller or code path feeding it. Once `MockConfigBuilder` builds
`power_characterization` natively (§4 above), these calls become dead code with the data
already present on the struct they're building it from — **delete them outright**, not
speculatively: drop the `forward_power_characterization_*` calls and their imports, and
let `create()` use the builder's `ClearBox`/`LightSource` as-is.

---

## 5 — Tests

### Remove (no longer applicable)

Every language's `v1_1_adapter_test.*` has explicit coverage of the Phase 2
fallback-from-legacy-fields direction — e.g. "v1.1 writer derives `power_characterization`
from `volts_to_watts_algorithm` when native field is absent." That code path no longer
exists (there's nothing to be absent from) — remove these specific test cases rather
than leaving them to fail or vacuously pass. Search each file for the fallback-specific
assertions (they're the ones constructing a `ClearBox`/`LightSource` with the legacy
scalar fields set and `power_characterization: None`, then checking the *written* file
has a derived `Power_Characterization` group).

### Update (assertions reference removed fields)

- Any read-assertion checking `clearbox.volts_to_watts_algorithm == "LINEAR"` (or
  equivalent) against a v1.0 fixture becomes
  `clearbox.power_characterization.algorithm_type == "LINEAR"`, plus (new coverage, not
  previously exercised on the read side) asserting the fully-derived structure —
  `algorithm_equation`, named `derivation_equation_constants`, empty
  `characterization_points`.
- Any write-assertion round-tripping a hand-built `ClearBox`/`LightSource` through the
  scalar fields needs its test fixture rewritten to set `power_characterization`
  instead.
- `test_builder.*` / `builder_test.*`: update to expect `power_characterization` in the
  builder's output instead of the two scalar fields.
- `test_capabilities.*` / `capabilities.test.ts`: if any facade get/set round-trip test
  exercises `SetClearbox`/`GetClearbox` with the old fields populated, update.

### Confirm unaffected (pure function tests)

- `power_characterization_test.*` / `test_power_characterization.*`: these test the
  shared module's pure functions directly with raw strings, not through the StableModel
  — should need no changes. Run them anyway as part of each language's full suite to
  confirm.
- `test_version_adapter_isolation.*` / `versionAdapterIsolation.test.ts`: no expected
  impact, but re-run — any new comment text added during this work should stay
  version-neutral, per the lesson from Phase 2 (C++'s near-miss, Node's clean pass).

### Fixtures

- `fixtures/reference_output.json` / `.sha256` (the Python-golden cross-language
  fixture): **must be regenerated** via `tools/generate_fixtures.py` once Python's
  reader change lands, since the JSON shape for `clearbox`/`light_source` changes (no
  more `volts_to_watts_algorithm`/`params` keys at all; `power_characterization` now
  populated for every v1.0 fixture, not just `None`). Re-run
  `docs/contributing.md`'s "Human review checklist" — it will need new line items for
  the now-always-present `power_characterization` block, since today's checklist
  predates this field's existence in v1.0 output entirely.
- `fixtures/synthetic_2laser.h5` (`MockConfigBuilder` output): regenerate after the
  builder change above; confirm its structure still matches whatever the checklist
  expects.
- `fixtures/reference_config_v1_1.h5` / `reference_config_v1_1_migrated.h5`: **no
  on-disk change** — v1.1's on-disk shape never had these fields to begin with, this
  plan only changes the in-memory StableModel and v1.0's on-disk-to-memory mapping.
  Re-run cross-language JSON comparisons against them anyway (Phase 1/2/4 of
  `cross_check.py`) since the JSON *shape* read from these files also loses the
  always-`None` legacy keys.

### Cross-language coordination

Because Python is the read-parity anchor in `tools/cross_check.py`, its reader change
alone will make Phase 2 (read parity) fail against every other language until they
land the identical change — same sequencing risk `V1_1_IMPLEMENTATION_PLAN.md` handled
by doing Python first, verifying its own suite, then proceeding language by language,
and only running the full cross-language check once all 5 are done. Follow the same
order here: Python → Rust → C++ → Go → Node.js (or whatever order the team prefers),
each verified independently before starting the next, full `cross_check.py --verbose`
only at the end.

---

## 6 — Documentation

- `docs/migrations/v1_0_to_v1_1.md`: the Overview table's claim "the old scalar fields
  are not removed... simply `None` for v1.1 files" becomes false — rewrite to state
  they're removed from the StableModel entirely, with `power_characterization` as the
  sole representation for both versions. Change 3/4's field-by-field tables and
  derivation-rule prose should simplify accordingly — there's no more "superseded but
  kept" framing needed.
- `docs/contributing.md`'s StableModel rules: add the distinction this whole exercise
  surfaced — "new orthogonal concept" (genuinely add a field, keep old ones) vs.
  "Transform of an existing concept's on-disk shape" (the existing field should
  usually be *replaced*, not left alongside a new one, precisely because leaving both
  is what caused this). Point at this plan as the worked example.
- `PARITY_AUDIT.md`: no new entry needed — the Node float-formatting Tier 2 item still
  applies unchanged to `backwardFlatFieldsCoefficients`/`Points` under the new
  unconditional-call design; add a one-line note that it's now reachable from v1.0's
  writer on *every* write, not just the Phase 2 fallback case, which raises its
  practical likelihood slightly but doesn't change the fix.
- `V1_1_IMPLEMENTATION_PLAN.md`: add a cross-reference in its closing status noting this
  plan supersedes Phase 2's dual-field compromise for these specific fields.
- **Downstream, out of this repo's scope but worth flagging explicitly**:
  `clearbox-tauri`'s `capture_services/mod.rs`/`pipeline.rs`/`hdf5/writer.rs` chain will
  need updating to read `power_characterization` once this ships — its own hardcoded
  `"Linear"` fallback should also be reconsidered while that work happens, independent
  of anything in this repo.

---

## Verification

1. **Done.** Per language, per change: that language's full existing suite green
   immediately after its own conversion, before moving to the next language.
2. **Done.** New/updated tests (§5) green, per language.
3. **Done (2026-09-02).** Full `tools/cross_check.py --verbose`, all 5 languages, all 5
   phases — clean: 25 schema validations, 50 read-parity comparisons, 25 write-interop
   checks (20 parity + 5 fidelity), 50 binary-copy checks, 100 correction-hash
   comparisons, all passing. One environment-only false alarm on the first run, not a
   code bug: Phase 3's `f"{writer_lang}-writes → ..."`-style tag strings (a literal `→`
   in the Python source, same character `V1_1_IMPLEMENTATION_PLAN.md`'s own Phase 1
   cross-check run hit) raised `UnicodeEncodeError: 'charmap' codec` when `print()`ed to
   this session's non-UTF-8 Windows console, masking the real per-writer results behind
   a generic `{lang}-writer` exception tag — fixed the same way that plan's run was
   fixed, by setting `PYTHONUTF8=1`/`PYTHONIOENCODING=utf-8` for `cross_check.py`'s own
   process, not just the subprocesses it spawns.
4. **Done (2026-09-02).** `fixtures/reference_output.json`/`.sha256` and
   `fixtures/synthetic_2laser.h5` regenerated via `tools/generate_fixtures.py` and
   re-reviewed per `docs/contributing.md`'s checklist — all items confirmed (machine
   name, 64-hex config hash, build plate 250/250/20, both trains' scanner
   offsets/rotation/thermal-lensing flags, both SFCF file sizes, no OPCUA fields), plus
   the new check this plan added: both trains' `power_characterization` now genuinely
   present with real values for both `clearbox` and `light_source`, in both fixture
   files. Re-ran Python (402/402), Rust (all `matches_python_golden_file` tests
   included), C++ (154/154), and Node.js (235/235) against the regenerated fixtures — all
   still green, confirming no language's own test suite depended on the previously
   on-disk (pre-unification) fixture content in a way this regeneration would break.
5. **Done (2026-09-02).** `docs/migrations/v1_0_to_v1_1.md`, `docs/contributing.md`,
   `V1_1_IMPLEMENTATION_PLAN.md`, and `PARITY_AUDIT.md` all updated per §6 — see each
   file's own 2026-09-02 edits for specifics.
6. **Not done — deliberately left for the user.** Commit(s) to be landed as a plain
   `feat` per the "Not a breaking change — decided" section above, once the user has
   reviewed the changes.

---

## Open items

- [x] **Resolved (2026-09-02).** `tools/generate_capabilities.py`'s coverage — confirmed
      current, active infrastructure (not tech debt), covers Python/TS/Rust/C++
      (Go deliberately excluded, hand-authored), and confirmed its templates are fully
      hardcoded rather than schema-driven — this plan's field removal requires no change
      to it at all. See §1.
- [x] **Resolved (2026-09-02).** Exact `LightSource` reader/writer line numbers —
      verified directly for all 5 languages, no longer inferred from `ClearBox`'s
      symmetry. See §3's per-language sections.
- [x] **Resolved (2026-09-02).** `create()`'s explicit `forward_power_characterization_*`
      calls (Rust/C++/Node) — read each `create()` directly; confirmed `MockConfigBuilder`'s
      output is the only input in all three, so the calls are safely, fully deletable.
      Confirmed not a Python or Go concern at all (opposite reasons: Python never needed
      the call, Go's mock never touched these fields). See §4.
- [x] **Resolved (2026-09-02).** Semver/commit-message question — plain `feat`, not
      `feat!`, since nothing has ever been promoted to `release`; see "Not a breaking
      change — decided" above.
- [x] **Resolved (2026-09-02).** Scope check — confirmed the only instance of the
      dual-representation pattern in the current v1.0→v1.1 diff; see "Scope
      confirmation" above.
