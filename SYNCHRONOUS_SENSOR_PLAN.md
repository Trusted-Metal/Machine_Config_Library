# Synchronous Sensor Group — Implementation Plan

**Status: Phase 0 and Phase 1 done and verified for all five languages, 2026-08-21.** Design
settled (attribute list, storage shape, cardinality/key convention — see "Design decisions —
confirmed" below). Phase 0's shared schema and fixture artifacts are complete and verified across
all five languages (see Phase 0 below). **Rust: 139/139 tests passing; Python: 350/350 tests
passing; Node.js: 181/181 tests passing; Go: 74/74 tests passing; C++: 533 assertions in 96 test
cases, 0 failures** (Phase 1 model/reader/writer/compound-type support complete for all five — see
Phase 1 below; validation-app coverage deliberately deferred for all five, same rhythm as the OPCUA
rollout). Phase 1 for this feature is complete across the board; the only remaining work is
validation-app coverage, deferred for all five languages (see each language's section). A
cross-check discrepancy found while verifying C++ turned out to be a stale build artifact, not a
real bug — see C++'s section below for the full resolution.

**Revision, 2026-08-21: `Derivation_Equation_Constants.name` moved from a variable-length to a
fixed-length (64-byte) string, in both already-shipped languages.** A pre-implementation check for
Node.js Phase 1 surfaced a real cross-language HDF5 restriction (see "Compound dataset string
convention" below) that made the originally-shipped variable-length convention a dead end for at
least one of the five languages. Resolved before Node.js Phase 1 began, so Node's writer/reader
work could build directly against the corrected convention rather than inheriting a design that
would have needed revisiting. Node.js's Phase 1 (models/reader/writer/tests) was then completed
using the corrected 64-byte fixed-length convention from the start — see the Node.js section below.

**Scope:** add a new `Synchronous_Sensors` collection — a name-keyed group of sensor records —
nested under the existing `ClearBox` group/struct, across all five languages (Rust, Python,
Node.js, Go, C++). Each sensor record carries 18 scalar attributes plus two new HDF5 **dataset**
types (not attributes) never used anywhere in this codebase before: a variable-row-count compound
dataset of named equation constants, and a variable-row-count compound dataset of calibration
points. Source: a ZR800 Oxygen Analyzer example the user supplied (image), refined through
discussion into the shape below.

**Explicitly not part of this work:**
- **No facade-level required-field validation layer** (the OPCUA-`getOpcua()`-equivalent
  `ValidationError`/`details` check) — deferred, not built speculatively. See "Why no facade
  validation layer yet" below for the reasoning; this is a decision, not an oversight.
- **No `File_Version` bump.** Per `docs/contributing.md`'s StableModel rules (§3): *"New optional
  field — add `field: Optional[T] = None` to the relevant dataclass. All existing adapters
  automatically return `None` for it. ✅ Safe."* The policy text is generic to "field," not
  special-cased to scalar-vs-nested-object fields — `synchronous_sensors` is a new optional field
  on `ClearBox` exactly like `trigger_stop_ceiling_layers` was a new optional field on `OpcuaConfig`
  in the OPCUA rollout, which also required no version bump.
- **No changes to `ClearBox`'s existing 20 fields.** This is a pure addition: one new field
  (`synchronous_sensors`) on an otherwise-untouched struct.

---

## Why a map keyed by a free-form label

Confirmed with the user: a `ClearBox` can have more than one Synchronous Sensor (different ports,
different analyzers), and the key naming needs to stay flexible — a user might call an entry
`"Oxygen Sensor"`, `"Oxygen"`, `"O2_Port5"`, or anything else, independent of that sensor's own
`Sensor_Type`/`Sensor_Name`/`Port_ID` attribute values.

This is not a new pattern — `OpcuaConfig.triggers` already works exactly this way. In every
language, the map key is the arbitrary HDF5 sub-group name the file's author chooses (confirmed by
reading the actual field, e.g. Rust's `rust/src/models.rs:372`: *"Key = trigger label (HDF5
sub-group name under `OPCUA/Triggers/`)"*), and it's never required to equal any attribute value
inside that trigger's own group. `Synchronous_Sensors` reuses the identical shape:
`Map<String, SynchronousSensor>` (language-appropriate map type), with the on-disk group name as
the free-form key.

## Why compound datasets, not bare arrays or JSON strings

`Derivation_Equation_Constants` and `Calibration_Points` both need to hold an open-ended number of
named/paired numeric values (a `Log-Linear` sensor has 2 constants, `a`/`b`; a different
`Algorithm_Type` might have 3+, with different names entirely). Three storage shapes were
considered:

1. **A bare positional float64 array** (`[0.4375, -2.75]`) — what the user's original spec named.
   Rejected on reflection: position-only storage means a reader must already know "index 0 is `a`"
   from `Algorithm_Type` alone, and any future algorithm with a different constant count/order
   becomes an ordering hazard instead of an additive change.
2. **A JSON-string attribute** (`'{"a": 0.4375, "b": -2.75}'`) — maximally flexible, but breaks
   from every other on-disk representation in this schema (everything else is typed
   attributes/datasets, not embedded string blobs) and loses direct binary/numeric accessibility
   for any tool reading the HDF5 file natively.
3. **A compound dataset of `{name, value}` rows** (chosen) — self-documenting per row, arbitrary
   length, and adding a constant is adding a row rather than shifting anyone else's index. Reuses
   the identical mechanism `Calibration_Points` already needs (`{input_value, output_value}` rows),
   so the compound-dataset plumbing is built once per language and used twice.

Row-count is per-sensor and varies freely — this is a plain 1-D dataset sized to `len(rows)` per
sensor group, not an HDF5 `H5T_VLEN`-of-compound construct. Confirmed against precedent in three of
the five languages' own HDF5 bindings — see each language's Phase 1 section below.

**Naming note:** `Calibration_Points`' members are named `input_value`/`output_value`, not
`input_unit`/`output_unit` as originally drafted — the stored data is a raw value pair (e.g.
`4 mA → -1`), not a unit of measurement (the actual units, `mA` and `log10(ppm)`, already live on
`Input_Type`/`Sensor_Output_Space`). Calling the members "value" avoids colliding with "unit"
terminology used for something else nearby.

**Unit convention — no per-row unit tag, but the convention must be stated explicitly.** Raised
directly: should each `Calibration_Points` row also carry its own unit string, so a row is
self-describing without relying on sibling attributes? No — a calibration curve is one fixed
function in one fixed unit space; every row for a given sensor is in the same units by
construction, so a per-row unit tag would repeat an identical value on every row while opening the
door to a genuinely invalid state that can't exist today (two rows in the same dataset disagreeing
about units). It would also add two more variable-length-string compound members on top of the
exact part of this work already flagged as the highest cross-language risk (Go has no
string-in-compound support at all yet — see Go's Phase 1 section).

The real ambiguity isn't "should units travel with the row" — it's **which of the sensor's two
existing unit-bearing fields the stored numbers correspond to**, since `Sensor_Output_Space`
(`"log10(ppm)"`) and `Units_Derived_Quantity` (`"ppm"`) describe two different scales for the same
underlying quantity. Resolved by checking the ZR800 example's own numbers against its own
`Algorithm_Equation`, rather than guessing: `log(ppm) = a*mA + b` with `a=0.4375, b=-2.75` gives
`0.4375×4 − 2.75 = -1.0` and `0.4375×20 − 2.75 = 6.0` — an exact match to both calibration points.
This proves the points are recorded in `Sensor_Output_Space` units (log-space), not
`Units_Derived_Quantity`'s linear `ppm` — they're the very two samples the fit constants were
derived from. **Convention (general, not specific to this example): `calibration_points.input_value`
is in whatever unit `Input_Type` implies; `calibration_points.output_value` is in whatever unit
`Sensor_Output_Space` implies** — for whatever those two fields happen to say on a given sensor,
not hardcoded to mA/log10(ppm). This belongs in the schema's property description for
`calibration_points` (Phase 0) and in each language's model doc-comment for the field (Phase 1),
so it's discoverable without re-deriving it from an example every time.

## Compound dataset string convention: fixed-length, 64 bytes (revised 2026-08-21)

`Derivation_Equation_Constants.name` is the only string anywhere in either compound dataset
(`Calibration_Points` is all-`f64`). It was originally shipped in both Rust and Python as a
variable-length (VLEN) string, matching how every other string in this schema is stored. That
convention does not survive contact with Node.js.

**The problem, found before writing any Node.js code, is not "VLEN strings are unreliable" —
they work fine everywhere as attributes or as plain top-level datasets.** It's specifically VLEN
strings as *members of a compound type*, and it's two separate failures stacked together:

1. **Node.js's h5wasm cannot write a non-empty VLEN string inside a compound row at all.** Its own
   error is explicit: *"Writing VLEN strings inside compound types requires pointers in Wasm heap
   and is not currently supported."* A VLEN string is an on-disk pointer to a heap-allocated blob;
   h5wasm's WebAssembly build of libhdf5 has no way to set that pointer up when the string lives
   inside a compound row's memory layout. (Reading a VLEN compound works fine in h5wasm — this is
   write-only, and a *zero-row* compound dataset of this shape also writes fine, since there's no
   actual string to allocate a pointer for.)
2. **Separately, the HDF5 C library itself cannot convert between a fixed-length and a
   variable-length string when they're compound-type members, in either direction.** Confirmed at
   the raw C level (`H5Tcreate`/`H5Tinsert`/`H5Dread`, via Go's cgo, bypassing every higher-level
   binding) — `H5T_path_find(): can't find datatype conversion path`. An exact type match (same
   width, padding, character set) always succeeds; a fixed↔VLEN conversion never does, for any
   language. So even if h5wasm's write gap didn't exist, a file written with VLEN by one language
   could still fail to be read by any other language whose compound type declared fixed-length for
   the same field.

**Empirically verified compatibility, all five languages, before deciding:**

| | Read VLEN | Write VLEN | Read Fixed(64) | Write Fixed(64) |
|---|---|---|---|---|
| Rust (hdf5-metno, `FixedUnicode<64>`) | ✅ | ✅ | ✅ | ✅ |
| Python (h5py) | ✅ | ✅ | ✅ | ✅ |
| Node.js (h5wasm) | ✅ | ❌ | ✅ | ✅ |
| C++ (HighFive) | ✅ | not tried | ✅ | ✅ |
| Go (raw libhdf5 1.14.6, via cgo) | not built | not built | not built (would work) | not built |

Fixed-length is the one representation every tested binding can both read and write for this
field. **Decision: standardize `Derivation_Equation_Constants.name` on a 64-byte fixed-length
UTF-8 string (NULLPAD-padded on disk) in every language.**

**Why 64 bytes specifically.** Real usage today is `a`/`b` (`LINEAR`) or `c0`..`cN`
(`POLYNOMIAL`) — 1-2 characters. 64 bytes is not sized to that; it's sized to leave room for more
descriptive names (`gain_coefficient`, `thermal_drift_factor`) without being a meaningful storage
cost at the row counts this dataset actually has (a handful of rows per sensor). NULLPAD is the
specific padding convention (not `SPACEPAD`) — it's what every binding's normal string-decode path
strips automatically, so reading back a 64-byte field, once trailing pad bytes are removed by
whichever language's own high-level API, always returns the exact string written (e.g. `"slope"`,
not `"slope"` plus padding) — confirmed directly, not assumed, via the round-trip probes below.

**Overflow behavior: rejected with a clear error, not silently truncated.** A name whose UTF-8
encoding exceeds 64 bytes would otherwise be silently cut off by both languages' underlying
mechanisms (Rust's `FixedUnicode<64>::from_str` and numpy's fixed-width `S64` dtype both truncate
by default). Both writers now check this explicitly before writing and raise
(`MachineConfigError::Parse` in Rust, `ValueError` in Python) rather than allow a truncated name
onto disk. Covered by
`write_synchronous_sensor_constant_name_too_long_for_fixed64_errors` (Rust,
`capabilities/v1_0/writer.rs`) and `test_constant_name_too_long_for_fixed64_raises` (Python,
`test_writer_roundtrip.py`).

**Verified before touching production code, not assumed:** a throwaway Python script wrote a
Fixed(64) compound file; a throwaway Rust example (`hdf5::types::FixedUnicode<64>`) read it back
cleanly and also wrote its own Fixed(64) file, which Python then read back cleanly — confirming
real bidirectional interop between the two already-shipped languages before committing to the
change. Both throwaway probes were deleted afterward; only the source and fixture changes below
are permanent.

**What changed to apply this:**
- Rust: `capabilities/v1_0/hdf5.rs`'s `RawEquationConstant.name` is now `FixedUnicode<64>` (was
  `VarLenUnicode`). The model-facing `EquationConstant.name : String` and the read path
  (`r.name.as_str().to_string()`) are unchanged — only the on-disk compound type moved.
- Python: `capabilities/v1_0/hdf5.py`'s `_EQUATION_CONSTANT_DTYPE` now uses
  `h5py.string_dtype(encoding="utf-8", length=64)` (was unbounded `h5py.string_dtype(encoding=
  "utf-8")`). The read path's `.decode("utf-8")` is unchanged — numpy's fixed-width byte-string
  dtype strips trailing NUL padding the same way the VLEN path already returned clean bytes.
- **Both existing fixtures patched in place, surgically, not regenerated from scratch**:
  `fixtures/reference_config_synchronous_sensors.h5` and
  `fixtures/reference_config_opcua_synchronous_sensors.h5` had only their
  `.../Synchronous_Sensors/Oxygen Sensor/Derivation_Equation_Constants` dataset rewritten under the
  new dtype (same two rows, same values, `[("a", 0.4375), ("b", -2.75)]`) — verified via a full
  attribute-tree diff (every other group, dataset, and attribute, including the `Correction_Data`/
  `Inverse_Correction_Data` grids' NaN-bearing float arrays) against a pre-patch backup, confirming
  zero unrelated changes anywhere else in either file.
- **Verification: full `cargo test` — 139 passed, 0 failed** (up from 138 — 1 new overflow-rejection
  test, 0 regressions). **Full `pytest python/tests/` — 350 passed, 0 failed** (up from 349 — 1 new
  overflow-rejection test, 0 regressions), zero warnings.

This decision and its rationale apply to any future language's Phase 1 work (Node.js, Go, C++)
just as much as it already applies to Rust and Python — none of them should introduce a VLEN
compound string for this field.

## Why no facade validation layer yet

Raised directly with the user: should `getClearbox()` (or a new accessor) enforce a required-field
check the way `getOpcua()` does? Answer: **not yet, and this is not the same kind of gap OPCUA had.**
OPCUA's required-field list came from a real CSV attribute reference the user could point to and
correct against. No equivalent reference exists yet for Synchronous Sensors — inventing a
required/optional split now would mean guessing at business rules with no source to verify against,
exactly the kind of guess this project's discipline has repeatedly avoided (see
`OPCUA_FIELD_PROMOTION_PLAN.md`'s own required-field corrections, all resolved against a real
source rather than assumed).

Critically, **deferring this costs nothing structurally.** `CapabilityError`'s `details: Option<Vec<String>>`
field (and its four language-equivalents) was built as generic, reusable infrastructure during the
OPCUA Phase 2 work specifically so *any* future facade check could report multiple named missing
fields — it is not OPCUA-specific plumbing that needs to be rebuilt. Adding a required-field check
for Synchronous Sensors later, once a real required list exists, is a small, self-contained
follow-up phase on top of infrastructure that already exists in all five languages today — not a
new plumbing project. This plan does not build that follow-up phase; it simply doesn't foreclose it.

---

## The attribute list

All 18 scalar attributes are `Option<T>` / `Optional[T]` / `T | null` / `*T` / `std::optional<T>`
in every language's model, matching the established "every field permissive at the low level,
required-ness (if any) is a facade-only concept" philosophy — see "Why no facade validation layer
yet" above. On-disk HDF5 group path: `.../Optional_Components/ClearBox/Synchronous_Sensors/<key>/`.

| Field → on-disk attribute | Type | Notes |
|---|---|---|
| `enabled` → `Enabled` | bool (HDF5 int 0/1) | |
| `sensor_name` → `Sensor_Name` | string | |
| `sensor_output_range_low` → `Sensor_Output_Range_Low` | float64 | split from a single range value — see rationale in conversation; matches existing paired-value convention (`scan_field_x/y/z`, `beam_waist_major/minor`) |
| `sensor_output_range_high` → `Sensor_Output_Range_High` | float64 | |
| `sensor_output_space` → `Sensor_Output_Space` | string | e.g. `"log10(ppm)"` |
| `sensor_model` → `Sensor_Model` | string | |
| `sensor_manufacturer` → `Sensor_Manufacturer` | string | added on user confirmation — present in the source image, omitted from the original typed spec |
| `sensor_scope` → `Sensor_Scope` | string | renamed from "Sensor Scope" on user confirmation, for on-disk naming consistency with every other attribute in this schema |
| `units_derived_quantity` → `Units_Derived_Quantity` | string | |
| `port_id` → `Port_ID` | int | uses each language's generic int-attribute helper (user confirmed), same as every other integer attribute in this schema (OPCUA's `cooldown_period`, `max_fires_per_job`, etc.) — no dedicated 32-bit-specific type or helper, despite the original spec naming Int32 |
| `sensor_type` → `Sensor_Type` | string | |
| `input_type` → `Input_Type` | string | e.g. `"4-20 mA"` |
| `algorithm_type` → `Algorithm_Type` | string | |
| `algorithm_equation` → `Algorithm_Equation` | string | free-text, not parsed/validated |
| `calibration_source` → `Calibration_Source` | string | |
| `calibration_verified` → `Calibration_Verified` | bool (HDF5 int 0/1) | |
| `sample_period` → `Sample_Period` | float64 | |
| `metadata` → `Metadata` | string | free-text |
| `derivation_equation_constants` → `Derivation_Equation_Constants` | **dataset**, compound rows `{name: string, value: float64}` | variable row count per sensor |
| `calibration_points` → `Calibration_Points` | **dataset**, compound rows `{input_value: float64, output_value: float64}` | variable row count per sensor; `input_value` in `Input_Type`'s unit, `output_value` in `Sensor_Output_Space`'s unit — see "Unit convention" under "Why compound datasets" above |

---

## Phase 0 — shared artifacts (once, not per language)

**Status: done and verified, 2026-08-20.**

- [x] `schema/machine_config_v1.schema.json` — added `synchronous_sensors` under the existing
      `clearbox` block, following the exact name-keyed-map shape already used for `triggers`
      (`"type": "object"`, `"additionalProperties": { <inline sub-schema> }`, no `$ref`, no
      `patternProperties`). The two dataset fields are plain JSON arrays of objects in the
      sub-schema (`derivation_equation_constants`: array of `{name, value}`; `calibration_points`:
      array of `{input_value, output_value}`) — JSON Schema doesn't distinguish "dataset" vs
      "attribute" on-disk storage, both just serialize as arrays/objects in `to_json()` output.
      Added a `"description"` on `calibration_points` stating the unit convention explicitly
      (`input_value` in `Input_Type`'s unit, `output_value` in `Sensor_Output_Space`'s unit — see
      "Why compound datasets"), and on `synchronous_sensors` itself stating the key is a free-form
      label with no schema meaning — the first uses of the `"description"` annotation keyword
      anywhere in this schema file (a deliberate, standard JSON Schema mechanism, not a new
      convention invented for this).
      **Then synced the two bundled copies**: `python/src/machine_config/machine_config_v1.schema.json`
      copied manually (git-tracked, no regeneration step — confirmed byte-identical to canonical
      via `diff` after copying).
      `nodejs/schema/machine_config_v1.schema.json` regenerated via `npm run build`'s `prebuild`
      step (`scripts/copy-schema.mjs`); confirmed byte-identical to canonical via `diff` afterward.
      **Verified:** `jsonschema.Draft202012Validator.check_schema(...)` confirms the file is itself
      a valid Draft 2020-12 schema. `jsonschema.validate(...)` confirms the *existing*
      `reference_config.h5` output (pre-Phase-1, via `MachineConfigReader(...).to_json()`) still
      validates — the addition is additive-only, nothing existing broke. Since no reader can
      produce a real `synchronous_sensors` payload yet (Phase 1 hasn't happened in any language),
      also hand-built a JSON document matching the intended future `to_json()` shape (the full
      ZR800 example, all 18 fields + both compound-dataset arrays) and confirmed *that* validates
      too — proving the new schema block itself is self-consistent, not just non-breaking.
- [x] **New fixture, not an in-place edit — a real correction to this plan's original design,
      caught by the user before it happened.** The plan as originally written called for editing
      `fixtures/reference_config.h5` in place. That would have silently destroyed the "`ClearBox`
      present, no `Synchronous_Sensors`" case for that file's train 1 — the same case
      `reference_config.h5` vs. `reference_config_opcua.h5` already exists specifically to preserve
      for OPCUA (two separate files, not one file mutated to sometimes-have-OPCUA). Caught mid-work
      (a `git status` on the in-place-edited file made the risk visible before it was ever
      committed) and corrected: `fixtures/reference_config.h5` was reverted to pristine
      (`git checkout`), and a **new** fixture, `fixtures/reference_config_synchronous_sensors.h5`,
      was created instead — a copy of `reference_config.h5` with `Synchronous_Sensors/Oxygen Sensor/`
      added under optical train 1's existing `Optional_Components/ClearBox/` group, mirroring the
      exact `reference_config_opcua.h5` naming/creation convention (a new file for a new optional
      feature, never a mutation of the shared baseline). `"Oxygen Sensor"` is just this new
      fixture's chosen label for the one demo entry — an arbitrary value, not a reserved or
      schema-significant name (per the "Map key" decision below); any other key would be equally
      valid on disk. The group carries the *complete* record for that sensor — all 18 scalar
      attributes plus both compound datasets, not a subset: `Derivation_Equation_Constants` =
      `[("a", 0.4375), ("b", -2.75)]`, `Calibration_Points` = `[(4.0, -1.0), (20.0, 6.0)]`, plus
      every scalar field from the ZR800 example.
      **A fourth fixture — reversed from the earlier "not built" call, on a follow-up question from
      the user.** The original reasoning against a combined fixture was narrowly about
      interaction-bug protection: `OPCUA/` (top-level) and `ClearBox/Synchronous_Sensors/` (nested
      per-optical-train) are structurally unrelated, so a combined file doesn't catch a bug class
      separate fixtures + an in-memory test wouldn't already catch. That reasoning still holds, but
      it wasn't the only reason to want this file — a canonical "every currently-modeled optional
      feature populated at once" fixture has independent value (the honest maximal example; a real
      on-disk target for Phase 1's eventual full-shape schema validation, stronger evidence than a
      hand-built JSON proxy; something Phase 1's tests can read directly instead of only
      reconstructing in memory) that the original evaluation didn't weigh. Built
      `fixtures/reference_config_opcua_synchronous_sensors.h5` as a copy of
      `reference_config_opcua.h5` with the identical `Synchronous_Sensors/Oxygen Sensor/` group
      added under train 1's `ClearBox` — same content, same verification rigor (full
      attribute-tree diff against pristine `reference_config_opcua.h5`: only the new group differs;
      all five languages' current CLIs open it without error). **Honest caveat:** since Phase 1
      hasn't happened in any language, this fixture's schema-validation payoff is deferred to
      Phase 1 — today's readers still can't emit `synchronous_sensors` in `to_json()` regardless of
      what's on disk, so the immediate verification value is the same "silently absorbed, no error"
      check as the other fixture, not a stronger schema proof yet.
      The in-memory build→write→read-back cross-feature test already queued in each language's
      Phase 1 test list is **kept, not replaced** — it independently proves the *Writer* can
      reconstruct this combination from a fresh in-memory model, which reading a pre-built fixture
      alone doesn't prove. Each language's Phase 1 test list now has both: a direct read of
      `reference_config_opcua_synchronous_sensors.h5` (proves the Reader), and the in-memory
      round-trip (proves the Writer) — not redundant, since they exercise different code paths.
      **Arbitrary-key coverage beyond this one fixture entry** is exercised at the unit-test level
      per language (construct a `SynchronousSensor` map in memory with a deliberately
      differently-styled key — e.g. an underscore-joined or multi-word name unlike `"Oxygen Sensor"`
      — round-trip it, confirm the key survives verbatim), not via a second on-disk fixture sensor;
      same "in-memory is enough, no new fixture needed" reasoning as above and as already used for
      the empty-map edge case in each language's Phase 1 test list below.
      **Verified:** re-opened `reference_config_synchronous_sensors.h5` fresh after writing (not the
      same handle) and confirmed all 20 fields read back with the expected values. A full
      attribute-tree diff (every group/dataset, every attribute) against pristine
      `reference_config.h5` shows zero changes anywhere else in the file — only the new group and
      its contents differ. Confirmed `fixtures/reference_config.h5` itself is back to byte-identical
      with its committed version (clean `git status`) — the "no sensors" baseline is intact.
      **Also confirmed all five languages' *current*, Phase-1-unaware readers open both new
      fixtures without error** (each silently absorbs the unknown group, exactly like every other
      unknown-attribute-outside-a-known-keys-list case already established in this codebase):
      Python's `MachineConfigReader(...).parse()`, and each language's CLI `export-json` command
      (Rust `machine-config-cli`, C++ `machine_config_cli`, Node's `cli.ts`, Go's
      `machine-config-cli`) all produced valid JSON output with no error, against both
      `reference_config_synchronous_sensors.h5` and `reference_config_opcua_synchronous_sensors.h5`.
- [x] No new `docs/validation/fixtures/*.h5` fixture — since no required-field validation exists
      yet (see "Why no facade validation layer yet"), there's no "missing required field" case to
      construct. `docs/validation/fixtures/generate_fixtures.py`'s established convention (a new
      optional CLI flag + a dedicated `generate_*()` function, off by default) is noted here for
      when that follow-up phase happens, not used now.
- [x] `CapabilityError.details` — **no action needed.** Already built, in all five languages,
      during the OPCUA rollout's Phase 2. Confirmed present and reusable; see "Why no facade
      validation layer yet."

**Verification (regression check across all five languages, full existing suites, against the
now-pristine `reference_config.h5` plus the new fixture sitting alongside it unreferenced by any
existing test):** Python `pytest python/tests/`: **321 passed, 0 failed** (unchanged from before
this work — schema-only/fixture-only changes, no source changes). Rust `cargo test`: **125 passed,
0 failed**. C++ (`cmake --build cpp/build --config Debug --target machine_config_tests` then
running the binary): **463 assertions in 87 test cases, 0 failures**. Node.js `npx vitest run`
(after `npm run build` regenerated the bundled schema copy): **169 passed, 0 failed** — the one
transient failure before rebuilding (`bundled schema matches canonical`) was the expected,
already-anticipated staleness in the git-ignored bundle, not a real regression; resolved by running
the existing `prebuild` step, not a code change. Go `go test ./... -count=1`: **59 passed, 0
failed**.

**Note for Python's Phase 1** (discovered while building the fixture, worth recording now so it
isn't rediscovered later): reading a compound dataset's variable-length string member back via
plain h5py (`dataset[()]`) returns `bytes`, not `str` — e.g. `Derivation_Equation_Constants`'s
`name` field reads back as `b'a'`/`b'b'`, even though it was written via
`h5py.string_dtype(encoding="utf-8")`. h5py's automatic bytes→str decoding for VLEN-string
attributes/scalar datasets does not automatically extend to VLEN-string *members inside a compound
dataset* — Phase 1's `_parse_clearbox` will need an explicit `.decode("utf-8")` on that field,
not assume it arrives pre-decoded the way a plain string attribute does.

---

## Phase 1 — per-language model / reader / writer / compound-type support

Common shape per language: (a) add a `SynchronousSensor` struct/dataclass/interface with the 18
scalar fields plus two row-type structs (`EquationConstant`/similarly-named `{name, value}`, and
`CalibrationPoint` `{input_value, output_value}`); (b) add `synchronous_sensors` as a name-keyed
map field on the existing `ClearBox` type; (c) extend the existing ClearBox parse/write functions
with a group-enumeration loop, mirroring the loop each language already has for `OpcuaTrigger`;
(d) add compound-dataset read/write support — new for every language, since no compound HDF5 type
is used anywhere in this codebase today.

### Rust

**Status: done and verified, 2026-08-21.**

- [x] `rust/src/models.rs` — added `SynchronousSensor` struct (18 `Option<T>` fields) plus
      `EquationConstant`/`CalibrationPoint` row structs, and added
      `pub synchronous_sensors: IndexMap<String, SynchronousSensor>` to `ClearBox` — `IndexMap`,
      matching `OpcuaConfig.triggers`'s existing choice. Confirmed no hidden third serialization
      site — `ClearBox` has no hand-written `Serialize`/`Deserialize` impl, just
      `#[derive(Serialize, Deserialize)]`.
  - [x] **Found and fixed a real cross-language regression the plan didn't anticipate**: adding
        `synchronous_sensors` with no `skip_serializing_if` broke
        `capabilities::v1_0::hdf5::tests::matches_python_golden_file` (and its `reader.rs`
        duplicate) — both compare Rust's JSON output for `fixtures/reference_config.h5` against a
        Python-generated golden file that predates this feature. Since that fixture's `ClearBox`
        always serializes (unlike `OpcuaConfig`, which is skipped whole when absent),
        `"synchronous_sensors": {}` appeared in Rust's output with nothing on the Python side to
        match. Fixed with `#[serde(skip_serializing_if = "IndexMap::is_empty", default)]` — not a
        test workaround but the semantically correct choice: keeps "no group on disk" and "no key
        in JSON" symmetric, and keeps every fixture that doesn't use this feature byte-identical to
        before it existed. Caught by running the full suite before adding new tests, not assumed.
- [x] `rust/src/capabilities/v1_0/hdf5.rs` — extended `fn parse_clearbox` with a
      `grp.group("Synchronous_Sensors")` → `.member_names()?` → `.group(&name)?` loop, the same
      mechanism `parse_opcua`'s trigger loop uses; defaults to an empty `IndexMap` when the group
      doesn't exist at all (no separate "absent" state — see `models.rs`'s doc comment). New
      `parse_synchronous_sensor` reads the 18 scalars plus both compound datasets via
      `ds.read_raw::<RawEquationConstant>()`/`RawCalibrationPoint`, defaulting to an empty `Vec`
      if a dataset itself is absent (same permissive-reader discipline extended from attributes to
      datasets).
- [x] `rust/src/capabilities/v1_0/writer.rs` — extended `fn write_clearbox` with a
      `sensors_grp.create_group(name)` loop per sensor, only creating the `Synchronous_Sensors`
      group at all when the map is non-empty (so a `ClearBox` with zero sensors is byte-identical
      on disk to before this field existed — no empty placeholder group). New
      `write_synchronous_sensor` writes the 18 scalars via the existing `ws`/`wf`/`wi`/`wb`
      helpers, then both compound datasets via
      `grp.new_dataset_builder().with_data(&Array1::from(rows)).create(name)?` — confirmed this
      creates a valid zero-length dataset when `rows` is empty, not assumed (see zero-row test
      below).
- [x] **Compound-type infrastructure**: `RawEquationConstant`/`RawCalibrationPoint`
      (`#[derive(hdf5::H5Type, Clone, Debug)] #[repr(C)]`) added as `pub(crate)` types inside
      `capabilities/v1_0/hdf5.rs` (imported into `writer.rs` via `use super::hdf5::{...}`) —
      distinct from the model-facing `EquationConstant`/`CalibrationPoint` in `models.rs`, exactly
      because `String` doesn't implement `H5Type`; `RawEquationConstant.name` is
      `hdf5::types::FixedUnicode<64>` (originally shipped as `VarLenUnicode`, revised 2026-08-21 —
      see "Compound dataset string convention" above), converted to/from `String` at the model
      boundary (`r.name.as_str().to_string()` / `c.name.parse()`). `RawCalibrationPoint` (all `f64`) is kept
      as its own type anyway, for symmetry and to keep `models.rs` free of any HDF5-specific derive
      — confirmed this split is what keeps the adapter/model layering intact (see the architecture
      discussion earlier in this conversation): `models.rs` never needs to know an HDF5 compound
      type exists at all.
- [x] **Found and fixed two hidden duplicate-test sites the plan didn't name**, exactly the class
      of gap flagged before starting: `rust/src/reader.rs` and `rust/src/writer.rs` (the plain
      public `MachineConfigReader`/`MachineConfigWriter` facade) each carry their own
      `#[cfg(test)] mod tests` with a same-named `roundtrip_clearbox_scalar_fields` (`writer.rs`)
      and `parse_reference_clearbox_and_sfcf_metadata_without_binary` (`reader.rs`) distinct from
      the `capabilities/v1_0` copies — confirmed by reading, not assumed, before starting (the
      production parsing/writing logic itself is *not* duplicated — `reader.rs`/`writer.rs` are
      thin delegating wrappers around `Hdf5AdapterV1_0`/`Hdf5WriterV1_0` — only these test files
      independently exercise ClearBox through the public facade). Extended both, and added two new
      facade-level tests (`parse_reference_has_no_synchronous_sensors`,
      `parse_synchronous_sensor_fixture_through_public_facade`) so the new feature is proven
      through all three layers (internal adapter, plain public facade, `capabilities` facade), not
      just one.
- [x] **New crate dependency-shape decision, confirmed correct**: `EquationConstant`/
      `CalibrationPoint`'s raw counterparts derive `H5Type` + `#[repr(C)]`. Confirmed `String`
      does **not** implement `H5Type` (no `impl H5Type for String` anywhere in
      `hdf5-metno-types-0.11.0`) before writing any code, not discovered after a compile failure.
- [x] **New tests** (13 new test functions across 4 files, plus 2 existing tests extended):
  - [x] `capabilities/v1_0/hdf5.rs`: `reference_fixture_has_no_synchronous_sensors`,
        `synchronous_sensor_fixture_has_real_values` (all 18 scalars + both compound datasets,
        exact values in order, against the real ZR800 example; also re-derives the unit-convention
        proof — both calibration points satisfy `algorithm_equation` exactly in log-space),
        `combined_opcua_and_synchronous_sensors_fixture_has_both`.
  - [x] `capabilities/v1_0/writer.rs`: extended `roundtrip_clearbox_scalar_fields` to use
        `reference_config_synchronous_sensors.h5` instead of `reference_config.h5`; new
        `roundtrip_synchronous_sensor_compound_datasets_exact_values_in_order`,
        `roundtrip_empty_synchronous_sensors_map_stays_empty_not_absent` (using
        `reference_config.h5`, which already has zero sensors),
        `roundtrip_synchronous_sensor_with_zero_row_compound_datasets` — **a distinct edge case
        from the empty-map test**, refined during implementation: the empty-map test proves zero
        *sensors*; this one proves a sensor that exists but whose two datasets have zero *rows*,
        which is the actual "zero-row compound dataset creation" case the plan named (the plan's
        original phrasing had conflated the two) — `roundtrip_synchronous_sensor_arbitrary_differently_styled_key`,
        `opcua_and_synchronous_sensor_coexist_through_writer` (proves the Writer side of the
        cross-feature guarantee).
  - [x] `rust/src/reader.rs`: `parse_reference_has_no_synchronous_sensors`,
        `parse_synchronous_sensor_fixture_through_public_facade`.
  - [x] `rust/src/writer.rs`: extended its own `roundtrip_clearbox_scalar_fields` (the facade
        duplicate found above) with sensor assertions.
  - [x] `rust/tests/capabilities_test.rs`: `clearbox_synchronous_sensors_empty_when_fixture_has_none`,
        `clearbox_synchronous_sensors_present_and_named_on_dedicated_fixture`,
        `combined_fixture_has_opcua_and_synchronous_sensors_through_facade` (proves the Reader
        side of the cross-feature guarantee through the `capabilities` facade specifically — a
        third layer, alongside `hdf5.rs`'s internal-adapter version and this same fixture's
        coverage).
- [ ] `docs/validation/rust/app/src/scenarios.rs` — **deferred, not part of this pass.** Matches
      the established project rhythm: validation-app coverage was requested and delivered as its
      own separate step for every language during the OPCUA rollout, after that language's model/
      reader/writer work, not bundled into it. Left unchecked here on that basis, not an oversight.

**Verification:** `cargo build --all-targets` clean, zero warnings (including `cargo doc
--no-deps --lib`, checked separately since intra-doc links aren't validated by a normal build).
Full `cargo test`: **138 tests, 0 failures** (up from 125 — 13 new test functions, 0 regressions).
Every new/extended test individually re-confirmed passing in the full run, including the two
hidden duplicate-test sites and the golden-file serialization fix. **Revised 2026-08-21:** 139
tests, 0 failures, after the `FixedUnicode<64>` change and its new overflow-rejection test — see
"Compound dataset string convention" above.

### Python

**Status: done and verified, 2026-08-21.**

- [x] `python/src/machine_config/models.py` — added `SynchronousSensor` dataclass (18
      `Optional[T]` fields, no defaults — matching `OpcuaTrigger`/`ClearBox`'s own convention of
      requiring every field explicitly rather than defaulting) plus `EquationConstant`/
      `CalibrationPoint` row dataclasses, and added `synchronous_sensors: dict[str,
      SynchronousSensor]` to `ClearBox` — **no default**, correcting the plan's original
      `field(default_factory=dict)` suggestion: `OpcuaConfig.triggers` (the closest analog) and
      every existing `ClearBox` field both require explicit specification at every construction
      site, so this field does too, for consistency.
- [x] `python/src/machine_config/capabilities/v1_0/hdf5.py` — extended `_parse_clearbox` with a
      `if "Synchronous_Sensors" in grp:` → `.keys()` loop, the same mechanism `_parse_opcua`'s
      trigger loop uses; defaults to an empty dict when the group doesn't exist at all (no
      separate "absent" state). New `_parse_synchronous_sensor` reads the 18 scalars plus both
      compound datasets via the new `_EQUATION_CONSTANT_DTYPE`/`_CALIBRATION_POINT_DTYPE`
      structured dtypes, defaulting to an empty list if a dataset itself is absent.
  - [x] **Found and fixed the exact `bytes`-vs-`str` gap flagged in Phase 0**: reading a compound
        dataset's variable-length string member back via `dataset[()]` returns `bytes`
        (`b'a'`), not `str` — confirmed directly against the real fixture before writing the
        parser, not assumed. `_parse_synchronous_sensor` explicitly decodes
        `row["name"].decode("utf-8")`.
  - [x] **`_EQUATION_CONSTANT_DTYPE`'s `name` field revised 2026-08-21** from an unbounded
        `h5py.string_dtype(encoding="utf-8")` to a fixed-length
        `h5py.string_dtype(encoding="utf-8", length=64)` — see "Compound dataset string
        convention" above. The `.decode("utf-8")` read path is unchanged; numpy's fixed-width
        byte-string dtype already strips trailing NUL padding before `.decode()` sees it. The
        writer now explicitly rejects (rather than silently truncates) a name whose UTF-8 encoding
        exceeds 64 bytes.
- [x] `python/src/machine_config/capabilities/v1_0/writer.py` — extended `_write_clearbox` with a
      `sensors_grp.require_group(name)` loop per sensor, only creating the `Synchronous_Sensors`
      group at all when the map is non-empty (so a `ClearBox` with zero sensors is byte-identical
      on disk to before this field existed). New `_write_synchronous_sensor` writes the 18
      scalars via the existing `_s`/`_f`/`_i`/`_b` helpers, then both compound datasets via
      `np.array([...], dtype=_EQUATION_CONSTANT_DTYPE)` /
      `grp.create_dataset(name, data=...)` — confirmed this produces a valid zero-length dataset
      when the row list is empty, not assumed (verified directly with a throwaway script before
      relying on it in a test).
- [x] **The hidden third site — confirmed present, exactly as flagged.** `_clearbox_to_dict` and
      the reverse-reconstruction block in `_train_from_dict` both updated, following the same
      pattern already used for OPCUA's `triggers` key.
  - [x] **Cross-language consistency decision, not in the original plan**: unlike
        `_opcua_to_dict`'s `triggers` key (always included, even as `{}`), `_clearbox_to_dict`
        **omits** `synchronous_sensors` entirely when the dict is empty. Matches the fix already
        applied to Rust's serde output for the identical reason (Rust's `matches_python_golden_file`
        test — see Rust's Phase 1 section) — keeps every fixture that doesn't use this feature
        byte-identical in JSON shape to before the feature existed, and keeps Rust and Python
        agreeing on JSON shape from day one rather than introducing a fresh cross-language
        disagreement. Reverse direction (`cb_d.get("synchronous_sensors", {})`) already tolerates
        the key being absent, matching the existing `triggers` precedent.
  - [x] Verified both directions for real, not just via unit tests: `to_json()` output for
        `reference_config_synchronous_sensors.h5` validates against the schema (the first *real*
        schema-validation evidence for this feature — Phase 0 could only hand-build a JSON proxy,
        since no reader could produce this shape yet); a full `to_json()` → `config_from_dict()`
        round-trip reproduces the exact same `SynchronousSensor` object; the empty-sensor-map case
        confirmed to omit the key entirely from real output.
- [x] **`__init__.py`'s curated export list also needed the three new types** — not named in the
      plan, found by checking how `ClearBox`/`OpcuaTrigger` are exposed at the top level before
      assuming the new types would be too (`from .models import *` is not used; the list is
      hand-maintained). Added `SynchronousSensor`/`EquationConstant`/`CalibrationPoint` to both the
      import block and `__all__`.
- [x] **Found and fixed a class-scoped-fixture deprecation warning this project has already
      documented avoiding elsewhere.** The first draft of `test_reader.py`'s new
      `TestSynchronousSensor` class defined its `sensor` fixture as an instance method
      (`@pytest.fixture(scope="class") def sensor(self, sensors_reader):`), triggering
      `PytestRemovedIn10Warning` — the exact anti-pattern `test_writer_roundtrip.py`'s and
      `test_opcua_roundtrip.py`'s own header comments already call out avoiding ("module-level
      fixtures... to avoid the PytestRemovedIn10Warning about instance-method fixtures"). Fixed by
      moving it to a plain module-level function fixture, matching the rest of the suite.
- [x] **New tests** (found real construction sites beyond the plan's file list, matching the
      "grep before trusting the plan's list" discipline established in the OPCUA rollout):
  - [x] Fixed all 5 real `ClearBox(...)` construction sites (only 3 were named in the plan):
        `hdf5.py`'s `_parse_clearbox`/reverse-reconstruction block (named), plus
        `builder.py`'s `_mock_clearbox` and **two** sites in `test_writer_roundtrip.py`
        (`_clearbox()` and the `nan_cb` fixture's `cb_obj`) — neither of the last two was named in
        the plan.
  - [x] `python/tests/conftest.py`: added `REFERENCE_SENSORS_H5`/`REFERENCE_OPCUA_SENSORS_H5`
        paths and `sensors_reader`/`opcua_sensors_reader` session fixtures, matching the existing
        convention exactly.
  - [x] `test_reader.py`: `test_synchronous_sensors_empty_when_fixture_has_none` (in the existing
        `TestClearBox`); new `TestSynchronousSensor` class — all 18 scalars, both compound
        datasets' exact values in order, the unit-convention log-space proof, and a combined
        OPCUA+sensors fixture check (proves the Reader through the plain public facade).
  - [x] `test_writer_roundtrip.py`: extended `TestClearBoxAttributeRoundtrip` (via the shared
        `_clearbox()` helper, now including one fully-populated sensor) with sensor-presence and
        compound-dataset-exact-order assertions; new `TestSynchronousSensorEdgeCases` class —
        empty-map-stays-empty (not absent), zero-row compound datasets for a sensor that *does*
        exist (**a distinct edge case from the empty-map case**, refined during implementation:
        the plan's original phrasing had conflated the two, exactly as it did for Rust), and an
        arbitrary differently-styled key.
  - [x] `test_opcua_roundtrip.py`: `test_opcua_and_synchronous_sensor_coexist_through_writer` —
        proves the Writer side of the cross-feature guarantee (parses the real OPCUA-only fixture,
        adds a sensor in memory, writes, confirms both survive re-reading).
- [ ] `docs/validation/python/app/scenarios.py` — **deferred, not part of this pass**, same
      established rhythm as Rust (validation-app coverage requested and delivered as its own
      separate step, after model/reader/writer work, throughout the OPCUA rollout).

**Verification:** full `pytest python/tests/`: **349 passed, 0 failed** (up from 321 — 28 new test
functions across 4 files, 0 regressions), zero warnings (including after fixing the class-scoped-
fixture deprecation warning found above). Clean import under `python -W error`. Real, not just
hand-built-proxy, schema validation: `MachineConfigReader(...).to_json()` for a fully-populated
sensor validates against `schema/machine_config_v1.schema.json`; a `to_json()` →
`config_from_dict()` round-trip reproduces the sensor object exactly; the empty-sensor-map fixture
confirmed to omit the key from real output, matching Rust's choice. **Revised 2026-08-21:** 350
passed, 0 failed, after the fixed-length `_EQUATION_CONSTANT_DTYPE` change and its new
overflow-rejection test — see "Compound dataset string convention" above.

### Node.js

**Status: done and verified, 2026-08-21.**

- [x] `nodejs/src/models.ts` — added `SynchronousSensor` interface (18 `T | null` properties,
      non-optional per this codebase's established convention of `T | null` over `?:`) plus
      `EquationConstant`/`CalibrationPoint` row interfaces, and added
      `synchronous_sensors?: Record<string, SynchronousSensor>` to `ClearBox`.
      **Naming correction from this plan's original draft**: the field is `synchronous_sensors`
      (snake_case), not `synchronousSensors` — every other `models.ts` field is snake_case to match
      the canonical JSON output shared across all five languages (per the file's own header
      comment), and the plan's earlier pseudocode was inconsistent with that established
      convention. Since `ClearBox` uses non-optional `T | null` properties, adding this required
      field made every existing object-literal construction site missing it a real `tsc` error
      (`TS2741`) — confirmed and fixed at the two real sites (`capabilities/v1_0/hdf5.ts`'s
      `parseClearBox`, `builder.ts`'s `mockClearBox`); `npx tsc --noEmit` across the whole project
      is the proof no site was missed, not a grep.
  - [x] **Real bug found and fixed, 2026-08-21 (before starting Go's Phase 1, while grounding in
        the existing code): the field was originally shipped as a required
        `synchronous_sensors: Record<string, SynchronousSensor>` (no `?:`), following the
        `OpcuaConfig.triggers` precedent (always present, even as `{}`).** That's the wrong
        precedent for this field — Rust and Python deliberately *omit* `synchronous_sensors` from
        JSON entirely when a `ClearBox` has no sensors (see this plan's Rust/Python Phase 1
        sections), matching `MachineConfig.opcua?`'s shape, not `triggers`'s. Confirmed empirically,
        not assumed: running both CLIs on `reference_config.h5`, Python's `ClearBox` JSON had no
        `synchronous_sensors` key at all, while Node's had `"synchronous_sensors": {}` — a real
        `tools/cross_check.py` Phase 2 (Read Parity) failure waiting to happen the moment Go/C++
        joined the CI matrix, not a cosmetic inconsistency. Fixed by making the field optional
        (`synchronous_sensors?:`) and having `parseSynchronousSensors` (`hdf5.ts`) collapse an
        empty result to `undefined` rather than `{}` — whether the group is absent entirely or
        present with zero children — and having `writeClearBox` (`writer.ts`) treat the field as
        possibly-`undefined` (`cb.synchronous_sensors ?? {}`). `builder.ts`'s `mockClearBox` no
        longer sets the field at all (omitted → `undefined`, matching the new contract). All three
        affected test assertions (one in `reader.test.ts`, one in `writer.test.ts`) were corrected
        from asserting `toEqual({})` to `toBeUndefined()`. Verified for real, not just via Node's
        own suite: `tools/cross_check.py --langs python,rust,nodejs` Phase 2 (Read Parity) passes
        clean after the fix, and a direct `DeepDiff` of Node's vs. Python's `export-json` output on
        `reference_config.h5` shows zero differences.
- [x] `nodejs/src/capabilities/v1_0/hdf5.ts` — extended `parseClearBox` with a
      `parseSynchronousSensors` helper (`sensorsEnt.keys()` loop over `Synchronous_Sensors/<name>`
      sub-groups), the identical mechanism `parseOpcua`'s trigger loop uses; defaults to an empty
      record when the group doesn't exist at all. New `readEquationConstants`/
      `readCalibrationPoints` read each compound dataset via `.value`, defaulting to `[]` if the
      dataset itself is absent. **Confirmed directly, not assumed, before writing this code**: a
      throwaway `tsx` probe (`h5wasm/node`) proved `create_dataset` with an explicit
      `dtype: [["name","S64"],["value","<d"]]` + `Map`/SoA `data` writes and reads correctly for
      both non-empty and zero-row cases, and that reading returns Array-of-Structures rows
      (`[["a",0.4375],["b",-2.75]]`) in declared-member order — confirming this plan's earlier
      correction to the (wrong) claim that Node.js has "the most direct compound-dataset support of
      the five" (see "Compound dataset string convention" above).
- [x] `nodejs/src/capabilities/v1_0/writer.ts` — extended `writeClearBox` with a sensor loop
      (`Object.keys(cb.synchronous_sensors)`, only creating the `Synchronous_Sensors` group when
      non-empty — same "no empty placeholder" discipline as Rust/Python). New
      `writeEquationConstants`/`writeCalibrationPoints`/`writeSynchronousSensor` helpers.
      `Derivation_Equation_Constants` is written with the explicit fixed-length `dtype`
      (`S${EQUATION_CONSTANT_NAME_MAX_BYTES}`, i.e. `S64`); `Calibration_Points` uses an explicit
      all-numeric dtype too (no string member, so no restriction, but kept explicit for
      consistency).
  - [x] **Overflow behavior confirmed empirically before relying on it**: the same throwaway probe
        showed `h5wasm` does **not** throw when a name exceeds 64 bytes — it silently truncates on
        write (a 100-character name came back as exactly 64 characters on read). `writeEquationConstants`
        now explicitly checks each name's UTF-8 byte length and throws a plain `Error` before
        calling `create_dataset` if it exceeds 64 bytes, matching Rust's (`MachineConfigError::Parse`)
        and Python's (`ValueError`) explicit-rejection behavior — this is not an assumption ported
        from the other languages, it's a real, separately-confirmed h5wasm behavior.
- [x] **No hidden third site** — confirmed `asJson()` (`capabilities/v1_0/file.ts:32-36`) is a
      pure type-cast (`config as unknown as Json`), and a full grep of `nodejs/src` for
      `ClearBox|clearbox` found no separate to-dict/from-dict function anywhere — the same
      "no hidden site" finding the OPCUA rollout already confirmed for this language.
- [x] Tests (12 new test functions across 2 files): `nodejs/tests/reader.test.ts` — extended the
      `ClearBox` describe block with a `synchronous_sensors` omitted-(undefined)-not-`{}` assertion
      on `reference_config.h5` (corrected 2026-08-21 alongside the bug fix above); new
      `SynchronousSensor` describe block against `reference_config_synchronous_sensors.h5` (not
      `reference_config.h5`) covering all 18 scalar fields against the real ZR800 example, both
      compound datasets' exact values in order, the log-space unit-convention proof (`a*mA + b`
      matches both calibration points), and a combined-fixture check against
      `reference_config_opcua_synchronous_sensors.h5` proving OPCUA and the sensor map are both
      present (proves the Reader).
      `nodejs/tests/writer.test.ts` — extended the reference-roundtrip block with an
      omitted-(undefined)-not-`{}` assertion; new `SynchronousSensor roundtrip` describe block: the real ZR800
      example roundtrips with exact compound-dataset values in order; a sensor that exists but has
      zero-row compound datasets (distinct from the empty-*map* case — proves 0-length compound
      dataset creation/read, not just an absent-group default); an arbitrary, differently-styled
      key (`HUMIDITY_SENSOR_2`) survives verbatim; OPCUA and a newly-added sensor coexist through
      the writer (proves the Writer side of the cross-feature guarantee, parsing
      `reference_config_opcua.h5` and adding a sensor purely in memory); and the 64-byte
      overflow-rejection test.
- [ ] `docs/validation/nodejs/app/scenarios.mts` — **deferred, not part of this pass**, matching the
      established rhythm already used for Rust's and Python's Phase 1 (validation-app coverage
      delivered as its own separate step, after model/reader/writer work, throughout the OPCUA
      rollout). This file does not exist yet in the repo at all — confirmed via search, not assumed
      — so there is nothing to extend or leave broken in the meantime.

**Verification:** `npx tsc --noEmit` clean across the whole project. `npm run build` clean
(regenerates the bundled schema copy). Full `npx vitest run`: **181 passed, 0 failed** (up from
169 — 12 new test functions across 2 files, 0 regressions).

### Go

**Status: done and verified, 2026-08-21.** Was "the largest single per-language effort in this
plan" going in, since `go/internal/h5c` had zero compound-type support — confirmed true: this was
the only language requiring genuinely new C-level (cgo) infrastructure rather than an extension of
an existing library feature.

- [x] **New H5T_COMPOUND infrastructure in `go/internal/h5c/h5c.go`'s cgo preamble** — first use of
      `H5Tcreate(H5T_COMPOUND, ...)`/`H5Tinsert` anywhere in this codebase. Two C struct typedefs
      (`h5c_equation_constant_t`, `h5c_calibration_point_t`) plus two factory functions
      (`h5c_create_equation_constant_type`/`h5c_create_calibration_point_type`) that build the
      on-disk compound type via `H5Tinsert` at each member's `HOFFSET`. `h5c_equation_constant_t`'s
      `name` member is a **64-byte fixed-length `H5T_STRING`, `H5T_STR_NULLPAD`, `H5T_CSET_UTF8`**
      — not the VLEN-string mechanism this file already uses for scalar attributes (reusing that
      would recreate the fixed↔VLEN conversion failure documented in "Compound dataset string
      convention" above) — matching Rust's `FixedUnicode<64>`, Python's
      `h5py.string_dtype(encoding="utf-8", length=64)`, and Node's explicit `S64` dtype exactly.
  - [x] Go-level wrappers: `EquationConstantRow`/`CalibrationPointRow` structs, and
        `(*Group).CreateEquationConstantsDataset`/`ReadEquationConstantsDataset`/
        `CreateCalibrationPointsDataset`/`ReadCalibrationPointsDataset` — narrowly-scoped to these
        two row shapes, matching this wrapper's existing purpose-built-not-generic style. A
        zero-length row slice creates a valid zero-row dataset (no write call), the same
        convention `CreateUint8Dataset` already uses for empty data.
  - [x] **Name-length validation happens in Go, not C**: `CreateEquationConstantsDataset` checks
        each row's UTF-8 byte length against `EquationConstantNameMaxBytes` (64) before building
        the write buffer, and returns an error — not a silent truncation — for an oversized name,
        matching Rust's/Python's/Node's explicit-rejection behavior.
  - [x] **New low-level unit tests, independent of and prior to the model-level tests** (7 test
        functions, `go/internal/h5c/compound_test.go`): exact-value round-trip and zero-row
        round-trip for both compound types; a name of exactly 64 bytes succeeds; a name of 65 bytes
        is rejected with a "does not fit" error, not truncated; and — the strongest test in this
        set — **reading the real, already-committed `reference_config_synchronous_sensors.h5`
        fixture directly** (written by Python, patched in place to the fixed-64-byte convention)
        confirms genuine cross-language interop, not just internal self-consistency.
  - [x] **Environment note, not a code issue**: this repo's Bash tool has no working `gcc` on PATH
        by default (`/mingw64/bin` resolves to Git-Bash's own minimal mingw runtime, which has no
        compiler) — the real MinGW64 toolchain with HDF5 1.14.6 headers/libs lives at
        `C:\msys64\mingw64\bin`. `CGO_ENABLED=1` plus prepending that directory to `PATH` is
        required for every `go build`/`go test`/CLI invocation in this environment; this isn't a
        project misconfiguration, `#cgo windows CFLAGS/LDFLAGS` in `h5c.go` already point at
        `/mingw64/include`/`/mingw64/lib` correctly for a properly-configured MSYS2 shell.
- [x] `go/internal/models/models.go` — added `SynchronousSensor` struct (18 pointer-typed fields)
      plus `EquationConstant`/`CalibrationPoint` row structs, and added
      `SynchronousSensors map[string]SynchronousSensor` to `ClearBox` with
      **`json:"synchronous_sensors,omitempty"`** — `omitempty` on a Go map omits the key for both
      `nil` and zero-length maps, which is exactly the Rust/Python "omit when empty" behavior (see
      the Node.js bug fix above, found and corrected in this same session, for why the naive
      always-present-record choice is wrong for this specific field). Also added `SynchronousSensor`/
      `EquationConstant`/`CalibrationPoint` as top-level type aliases in `go/models.go`, matching
      every other sub-type's existing alias.
- [x] `go/capabilities/v1_0/hdf5/hdf5.go` — extended `parseClearBox` with `parseSynchronousSensors`,
      a `sensorsGrp.SubGroupNames()` loop mirroring the `OpcuaTrigger` loop, using the identical
      `sg, err := sensorsGrp.OpenGroup(name); if err != nil { continue }` defensive pattern the
      trigger loop already uses — which already handles the `SubGroupNames()`-returns-everything
      caution this plan originally flagged, since a non-group child simply fails `OpenGroup` and is
      skipped, no extra filtering needed. New `parseSynchronousSensor` reads the 18 scalars via the
      existing `readStrAttr`/`readFloatAttr`/`readIntAttr`/`readBoolFromIntAttr` helpers, plus both
      compound datasets via the new h5c methods, defaulting to an empty (non-nil) slice if a
      dataset is itself absent.
- [x] `go/capabilities/v1_0/hdf5/writer.go` — extended `writeClearBox` with a sensor loop, only
      creating the `Synchronous_Sensors` group at all when the map is non-empty (matching
      Rust's/Python's/Node's choice, deliberately different from `OPCUA/Triggers`, which is always
      created). New `writeSynchronousSensor` writes the 18 scalars via the existing `ws`/`wf`/`wi`/
      `wb`/`strOrEmpty` helpers, then both compound datasets via the new h5c methods.
- [x] Tests (15 new test functions across 3 files): `go/reader_test.go` —
      `TestSynchronousSensorsEmptyWhenFixtureHasNone`, `TestSynchronousSensorFixtureHasRealValues`
      (all 18 scalars + both compound datasets' exact values in order + the log-space
      unit-convention proof, against the real ZR800 example), `TestCombinedOpcuaAndSynchronousSensorsFixtureHasBoth`.
      `go/writer_test.go` — `TestWriterRoundtripSynchronousSensorCompoundDatasetsExactValuesInOrder`,
      `TestWriterRoundtripSynchronousSensorWithZeroRowCompoundDatasets` (distinct from the empty-map
      case — a sensor that exists but has zero-row datasets),
      `TestWriterRoundtripSynchronousSensorArbitraryDifferentlyStyledKey`,
      `TestWriterOpcuaAndSynchronousSensorCoexist` (proves the Writer side of the cross-feature
      guarantee), `TestWriterRejectsEquationConstantNameOver64Bytes`. Plus the 7 h5c-level tests
      listed above. `go/capabilities/file_test.go`'s lack of any ClearBox-facade test at all
      (confirmed via grep, matching this plan's original note) was left as-is — not part of this
      pass, and no worse than before it.
- [ ] `docs/validation/go/app/scenarios/scenarios.go` — **deferred, not part of this pass**, matching
      the established rhythm used for Rust's, Python's, and Node's Phase 1 (validation-app coverage
      delivered as its own separate step, after model/reader/writer work, throughout the OPCUA
      rollout).

**Verification:** `go build ./...` clean. Full `go test ./... -count=1`: **74 passed, 0 failed**
(up from 59 — 15 new test functions across 3 files, 0 regressions). Cross-language parity confirmed
for real, not just via Go's own suite: `tools/cross_check.py --langs python,rust,nodejs,go` Phase 2
(Read Parity) passes across all four languages; a direct `DeepDiff` of the Go CLI's `export-json`
output against Python's, for both `reference_config_synchronous_sensors.h5` and
`reference_config_opcua_synchronous_sensors.h5`, shows zero differences.

### C++

**Status: done and verified, 2026-08-21.**

- [x] **New shared header, not folded into `hdf5.hpp`**: `cpp/include/machine_config/capabilities/v1_0/compound_types.hpp`
      — the on-disk `EquationConstantRow`/`CalibrationPointRow` structs plus their
      `HighFive::CompoundType` factory functions and `HIGHFIVE_REGISTER_TYPE` registrations, included
      by both `hdf5.hpp` (reader) and `writer.hpp` (writer), which are independent siblings that
      don't include each other. **Deliberately declared at global scope, not inside
      `namespace machine_config`**: `HIGHFIVE_REGISTER_TYPE` expands to an explicit specialization
      of `HighFive::create_datatype<T>` written with HighFive's fully-qualified name, and C++
      requires an explicit specialization to be declared in a namespace enclosing the template's own
      — `machine_config` and `HighFive` are unrelated sibling namespaces, so the registration has to
      live outside both (matching HighFive's own `compound_types.cpp` example, which also registers
      at global scope). `EquationConstantRow.name` is `char[64]`, matching the compound-type
      convention (`FixedLengthStringType(64, NullPadded, Utf8)`) — see "Compound dataset string
      convention" above.
  - [x] **Verified via a throwaway probe before writing any production code** (`cpp/_probe.cpp`,
        wired into `CMakeLists.txt` temporarily, then fully removed): write→read round-trip for
        both compound types via the `HIGHFIVE_REGISTER_TYPE` + `grp.createDataSet(name,
        std::vector<T>)` convenience path (not raw `write_raw`/`read_raw`); a zero-row
        `std::vector<T>{}` creates and reads back a valid zero-length dataset; and — the strongest
        check — reading the real, already-committed `reference_config_synchronous_sensors.h5`
        fixture directly (written by Python, patched to the fixed-64-byte convention) through this
        exact mechanism, confirming genuine cross-language interop, not just internal
        self-consistency. One real bug caught by this probe before it reached production code: an
        unhandled `HighFive::Exception` (from an initially-wrong relative fixture path) triggered a
        silent-looking hang — actually the Windows Error Reporting dialog blocking on an unhandled
        exception, not an infinite loop — fixed by wrapping the probe in `try/catch` and correcting
        the path.
- [x] `cpp/include/machine_config/models.hpp` — added `SynchronousSensor` struct (18
      `std::optional<T>` fields, declared before `ClearBox` since it's used as the map's value
      type) plus `EquationConstant`/`CalibrationPoint` row structs, and added
      `std::map<std::string, SynchronousSensor> synchronous_sensors;` to `ClearBox`.
  - [x] **Correction to this plan's original draft, caught before implementing (2026-08-21,
        immediately after the identical mistake was found and fixed in Node.js's already-shipped
        code): the direct, unconditional `{"triggers", o.triggers}`-style nlohmann assignment this
        plan called for is the wrong pattern for this field.** Rust and Python deliberately *omit*
        `synchronous_sensors` from JSON entirely when a `ClearBox` has no sensors — matching
        `MachineConfig::opcua`'s `if (m.opcua.has_value()) j["opcua"] = *m.opcua;` shape
        (`models.hpp` line ~965), not `OpcuaConfig::triggers`'s always-present shape. `ClearBox`'s
        `to_json` now does `if (!c.synchronous_sensors.empty()) j["synchronous_sensors"] = ...;`,
        and `from_json` clears the map first and only populates it `if
        (j.contains("synchronous_sensors"))`. Caught by design-review before writing the code, not
        by a failing test — the Node.js incident made the risk visible in advance for this
        language.
- [x] `cpp/include/machine_config/capabilities/v1_0/hdf5.hpp` — extended `parseClearBox` with
      `parseSynchronousSensors`, a `sensors_grp.listObjectNames()` loop, the identical mechanism the
      `OpcuaTrigger` loop already uses (confirmed against this file directly, not assumed). New
      `parseSynchronousSensor` reads the 18 scalars via the existing `readStr`/`readFloat`/
      `readInt`/`readBoolFromInt` helpers, plus both compound datasets via `grp.getDataSet(name)
      .read(rows)` against the registered compound types, defaulting to an empty (non-nil) vector if
      a dataset is itself absent. New `fixedBufToString(const char*, size_t)` helper trims a fixed
      `char[64]` buffer at its first NUL byte — **not** `std::string(buf)`, which would read past
      the 64-byte window if a name fills the field exactly (no trailing NUL to stop at).
- [x] `cpp/include/machine_config/capabilities/v1_0/writer.hpp` — extended `writeClearBox` with a
      sensor loop, only creating the `Synchronous_Sensors` group at all when the map is non-empty
      (matching Rust's/Python's/Node's/Go's choice, deliberately different from `OPCUA/Triggers`,
      which is always created). New `writeSynchronousSensor` writes the 18 scalars via the existing
      `ws`/`wf`/`wi`/`wb` helpers (each optional field passed as `.value_or("")`/directly, matching
      this file's own established convention — no `strOrEmpty` wrapper needed here since `wf`/`wi`/
      `wb` already accept `std::optional<T>` natively), then both compound datasets via new
      `writeEquationConstants`/`writeCalibrationPoints` helpers.
  - [x] **Overflow behavior**: `writeEquationConstants` checks each name's UTF-8 byte length
        (`std::string::size()`) against `EquationConstantMaxNameBytes` and throws
        `std::runtime_error` — this codebase's established error-signaling convention (confirmed via
        `reader.hpp`/`writer.hpp`'s existing `throw std::runtime_error(...)` sites) — before
        building the write buffer, rather than truncating.
- [x] Tests (9 new test cases across 3 files): `cpp/tests/test_reader.cpp` —
      `SynchronousSensorsEmptyWhenFixtureHasNone`, `SynchronousSensorFixtureHasRealValues` (all 18
      scalars + both compound datasets' exact values in order + the log-space unit-convention proof,
      against the real ZR800 example), `CombinedOpcuaAndSynchronousSensorsFixtureHasBoth`.
      `cpp/tests/test_writer.cpp` — extended `RoundtripAllScalarFields` with an empty-not-absent
      assertion; new `RoundtripSynchronousSensorCompoundDatasetsExactValuesInOrder`,
      `RoundtripSynchronousSensorWithZeroRowCompoundDatasets` (distinct from the empty-map case — a
      sensor that exists but has zero-row datasets), `RoundtripSynchronousSensorArbitraryDifferentlyStyledKey`,
      `RoundtripOpcuaAndSynchronousSensorCoexist` (proves the Writer side of the cross-feature
      guarantee), `WriterRejectsEquationConstantNameOver64Bytes`. `cpp/tests/test_capabilities.cpp` —
      new `CapabilityClearboxSynchronousSensors`, extending the existing `getClearbox()` facade
      coverage to the sensor map too.
- [ ] `docs/validation/cpp/app/src/scenarios/scenarios.cpp` — **deferred, not part of this pass**,
      matching the established rhythm used for every other language's Phase 1 (validation-app
      coverage delivered as its own separate step, after model/reader/writer work, throughout the
      OPCUA rollout).

**Verification:** `cmake --build build --config Debug --target machine_config_tests` clean. Full
suite: **533 assertions in 96 test cases, 0 failures** (up from 463/87 — 9 new test cases, 0
regressions). Cross-language parity confirmed for real: a direct `DeepDiff` of the C++ CLI's
`export-json` output against Python's, isolated to the `ClearBox` subtree, for both
`reference_config_synchronous_sensors.h5` and `reference_config_opcua_synchronous_sensors.h5`, shows
zero differences.

**False alarm, resolved, 2026-08-21 — not a real bug.** Running the full `tools/cross_check.py`
across all five languages initially appeared to surface an OPCUA field-promotion bug in C++
(`Keep_Alive_Count`, `Queue_Policy`, `Case_Sensitivity`, etc. landing in `extra` instead of their
named fields on `reference_config_opcua.h5`). Investigated rather than assumed: `models.hpp`'s
`to_json`/`from_json` for `OpcuaClientConfig`/`OpcuaPipeConfig`/`OpcuaTrigger` were already correct
— confirmed by running `machine_config_cli.exe export-json` directly and inspecting the raw output,
which showed every field correctly named and `extra: {}`. The actual cause: `tools/cross_check.py`'s
`_cpp_bin()` prefers a `Release` build over `Debug` when both exist, and a **stale `Release/
machine_config_cli.exe` from 2026-08-17 — four days old, predating this entire investigation** —
was sitting alongside the fresh Debug build made during this pass, and got picked up instead.
Rebuilding the Release config (`cmake --build build --config Release --target machine_config_cli`)
and re-running the cross-check confirmed it: **all five languages agree on all 3 fixtures (30/30
comparisons), 0 failures.** No code change was needed — the lesson is procedural (rebuild every
config a verification tool might select, not just the one used for the test suite), not a defect
in this feature or in the OPCUA work.

---

## Phase 2 — facade validation (not scheduled)

Deliberately not planned in detail here — see "Why no facade validation layer yet" above. When a
real required-field reference exists, this phase is: extend `getClearbox()` (or add a dedicated
per-sensor accessor) with the same 4-step mechanism `getOpcua()` already uses (single-instance
checks + a per-sensor loop for anything checked per-entry, combined into one `ValidationError`
with `details` populated only when non-empty) — mechanically identical to OPCUA's Phase 2, just
against a different field list. No new shared infrastructure would be needed to start that phase.

---

## Validation-app coverage

**Status: not started.**

- [ ] One new scenario per language (or an extension of the existing ClearBox-reading scenario),
      opening **`reference_config_synchronous_sensors.h5`** (the dedicated new fixture from Phase 0
      — not `reference_config.h5`, which deliberately has no sensors) and confirming: the
      `synchronous_sensors` map contains the fixture's demo entry under whatever key Phase 0
      assigned it (`"Oxygen Sensor"` — asserted because it's what's actually on disk, not because
      that string is otherwise significant), all 18 scalar fields match the fixture's real values,
      and both compound datasets read back with the exact expected rows in order. A companion
      assertion, opening plain `reference_config.h5`, confirms `synchronous_sensors` there reads
      back empty, not absent — proving the feature is genuinely optional through the public API,
      not just in unit tests.
- [ ] Extend the existing write→read roundtrip scenario (wherever each language currently proves
      `ClearBox` survives a roundtrip) with sensor-map assertions, reading from
      `reference_config_synchronous_sensors.h5` — proving the low-level roundtrip works through the
      public `Reader`/`Writer` API too, not just unit tests, matching the established "prove it
      end-to-end" pattern from the OPCUA rollout's validation-app phase.
- [ ] S-09-equivalent (public type-export surface) needs a one-line addition per language if
      `SynchronousSensor` becomes independently importable/exported as a top-level type (check each
      language's existing S-09 scenario for its current type list).
- [ ] No AV-##-style validation-error scenario — there's no validation error to construct without
      Phase 2.
- [ ] Follow `VALIDATION_PLAN.md` §8's documented per-scenario format (`### AV-NN: <title>` heading
      + fenced `ID:`/`Title:`/`Category:`/`Layer:`/`Precondition:`/`Action:`/`Expected:`/`Rationale:`
      block, confirmed verbatim from the AV-10/AV-11 entries) if this gets a numbered catalog entry
      — next available number is AV-14 (AV-12/13 are already used by the OPCUA rollout).

---

## Documentation pass (after implementation, not blocking it)

- [ ] `docs/{rust,python,nodejs,go,cpp}.md` — check for any place enumerating `ClearBox`'s fields
      explicitly; update if so.
- [ ] `VALIDATION_PLAN.md` §8 — add the new scenario(s) to the catalog if numbered per the format
      above.
- [ ] Each language's `docs/validation/<lang>/results.md`/`PASS_FAIL.md` — update once implemented
      and passing.

---

## Design decisions — confirmed

- **Fixture layout: 4 files, not 1 (as originally planned).** `fixtures/reference_config.h5` stays
  pristine (no sensors, the "optional feature absent" baseline); `fixtures/reference_config_opcua.h5`
  stays pristine too (OPCUA present, no sensors); `fixtures/reference_config_synchronous_sensors.h5`
  carries the ZR800 demo entry with no OPCUA; `fixtures/reference_config_opcua_synchronous_sensors.h5`
  carries both at once — the canonical "every currently-modeled optional feature populated"
  example. The original plan called for an in-place edit of `reference_config.h5` (caught and
  reverted before it was committed) and, after that, an initial call to skip the fourth/combined
  file entirely (reversed on a follow-up question — see Phase 0's "New fixture, not an in-place
  edit" and "A fourth fixture" items for the full reasoning behind each correction). Both the
  fixture-only Reader-side test and the in-memory Writer-side round-trip test are kept in each
  language's Phase 1 test list — not redundant, since they exercise different code paths.
- **Sensor_Manufacturer**: included (user confirmed; present in the source image, omitted from the
  originally-typed field list).
- **Sensor_Scope**: on-disk name is `Sensor_Scope` (user confirmed the rename from "Sensor Scope",
  for consistency with every other attribute name in this schema).
- **Cardinality**: a `ClearBox` may have zero or more Synchronous Sensors (user confirmed).
- **Map key**: a free-form label chosen by the file's author, not required to equal any attribute
  value inside the sensor's own group (user confirmed, after discussion) — same shape as
  `OpcuaTrigger`'s existing key convention. **Reaffirmed:** `"Oxygen Sensor"` (the fixture's demo
  key) and every other example key mentioned in this document are just illustrations — the schema
  places no constraint on the key's content or style, and the fixture/tests must not be read as
  implying otherwise.
- **Sensor record completeness**: the `SynchronousSensor` structure (18 scalar attributes + 2
  compound datasets) is meant to encapsulate the complete, current data set for one sensor — not a
  partial/summary view. Any fixture or test entry populates all of it, not a subset.
- **Calibration_Points member names**: `input_value`/`output_value`, not `input_unit`/`output_unit`
  as originally drafted (see "Why compound datasets" above for the reasoning; presented to the
  user as part of the worked example, accepted).
- **Derivation_Equation_Constants storage shape**: upgraded from a bare float64 array (as
  originally drafted) to a compound `{name, value}` dataset, matching `Calibration_Points`'s shape
  (see "Why compound datasets" above; presented to the user as part of the worked example,
  accepted).
- **Calibration_Points unit convention**: no per-row unit tag; `input_value` is in `Input_Type`'s
  unit, `output_value` is in `Sensor_Output_Space`'s unit — confirmed against the ZR800 example by
  checking both calibration points satisfy `Algorithm_Equation` exactly in log-space, not linear
  `ppm` (see "Why compound datasets" above). To be stated explicitly in the schema property
  description and each language's model doc-comment during Phase 0/1, not left implicit.
- **Facade validation layer**: deferred, not built now (see "Why no facade validation layer yet").
- **Port_ID width**: uses each language's generic int-attribute helper (user confirmed) — same as
  every other integer attribute in this schema, not a dedicated 32-bit-specific type, despite the
  original spec naming Int32.
