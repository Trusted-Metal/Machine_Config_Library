# Configuration Hash — Implementation Plan

**Status: done and verified, all five languages, 2026-09-15.** Steps 1-7 (Python, Rust, Node.js,
Go, C++, cross-language verification, fixtures) done 2026-09-15; Step 8 (docs) held until
`CAPABILITIES_HASH_INTEGRATION_PLAN.md` finished all five languages, then completed the same day —
see Step 8 below for what changed and why it had to wait. Step 1's first
attempt used a write→re-read→patch design to work around a bug it found (writers silently
materializing default unit values), but that whole approach was **superseded** the same day: the
actual fix is to stop the writers from materializing those defaults at all, which lets every
language's hash go back to the simple "compute once, up front, before writing" design this plan
originally called for. Python was rebuilt this way; Rust, Node.js, Go, and C++ were implemented
this way from the start. All five languages are fully green, and their `configuration-hash` CLI
output is byte-identical against `fixtures/reference_config.h5` (confirmed pairwise across all
five — Node.js needed one fix to get there in Step 3; Rust, Go, and C++ each matched on the first
attempt). Step 6's automated cross-check (`tools/cross_check.py` Phase 5, covering all fixtures,
not just the reference one) additionally caught and fixed a real Python-only bug
(`ClearBox.firmware_version` silently dropped by `hash.py`'s always-v1.0 dict-builder assumption,
only observable on the `v1_1` fixture) and a stale pre-existing assumption in `cross_check.py`
itself (Phase 3's fidelity check, which predates this feature and didn't yet know every writer now
recomputes its hash on purpose). Step 7 regenerated `fixtures/synthetic_2laser.h5` (the only
committed fixture that needed it — everything else is externally-authored and permanently
`is_valid=false` by design) and re-verified all five languages' test suites plus the full
cross-check against the regenerated file.

**Purpose of this document:** committed to the repo (unlike the ephemeral plan-mode file this
was drafted in) so it's a durable, reviewable record the same way `RECOATER_BLADE_TYPE_PLAN.md`
is — picked up step by step, one language (or phase) per implementation session, each step
checked off with what was actually done/verified before moving to the next.

---

## Contents

- [Context](#context)
- [Critical finding: byte-identical parity with RDT-CORE is not achievable](#critical-finding-byte-identical-parity-with-rdt-core-is-not-achievable-and-must-not-be-the-goal)
- [The algorithm, precisely](#the-algorithm-precisely)
- [Prerequisite: stop defaulting unit attributes](#prerequisite-all-five-languages-stop-defaulting-unit-attributes-when-unspecified)
- [Decisions made](#decisions-made-flag-at-review-if-youd-rather-go-another-way)
- [Step 1 — Python (reference implementation)](#step-1--python-reference-implementation)
- [Step 2 — Rust](#step-2--rust)
- [Step 3 — Node.js](#step-3--nodejs)
- [Step 4 — Go](#step-4--go)
- [Step 5 — C++](#step-5--c)
- [Step 6 — Cross-language verification](#step-6--cross-language-verification)
- [Step 7 — Fixtures](#step-7--fixtures)
- [Step 8 — Documentation cleanup](#step-8--documentation-cleanup)
- [On step granularity](#on-step-granularity)

---

## Context

`meta.configuration_hash` (the `Configuration_Hash` root HDF5 attribute) is currently a pure
opaque passthrough everywhere in this library — every language's writer stores whatever string
is already in the in-memory `MachineConfig` being written, and every reader returns whatever
string was already on disk. Nothing computes it. `docs/clearbox-tauri-integration.md`'s §6
"Configuration Hash Validation" already documents a `reader.compute_hash()` method as if it
exists — confirmed via full-repo search that it does not; that doc section is aspirational and
wrong today.

The goal: make `configuration_hash` a real, computed value. The writer computes it from the
config being written; the reader recomputes it from what it just read and compares, setting a
non-fatal `is_valid` signal — **the file must always remain fully readable regardless of match**,
per explicit instruction. A single shared helper function per language is called by both the
writer (to generate) and the reader (to recompute-and-compare).

The user's team has an existing Node.js producer of these files, `RDT-CORE`
(`C:\Users\ChrisParham\Desktop\Repo\RDT-CORE`), whose current algorithm was given as the
"ground-truth" starting point to port:

```typescript
private calculateHdf5StructureHash(hdf5Structure: Record<string, any>, machine: any): string {
  const configForHash = {
    machine: { id: machine.id, machine_name: machine.machine_name, serial_number: machine.serial_number, manufacturer: machine.manufacturer, model: machine.model },
    structure: hdf5Structure,
  };
  const normalizedConfig = this.deepSortForHash(configForHash); // recursively sorts object keys
  const configString = JSON.stringify(normalizedConfig);        // no whitespace
  return crypto.createHash('sha256').update(configString).digest('hex');
}
```

---

## Critical finding: byte-identical parity with RDT-CORE is not achievable, and must not be the goal

Read RDT-CORE's actual write path in full
(`RDT-CORE/api/endpoints/machines/service-observations/helpers/machine-config-generator.helper.ts`).
Its hash (`calculateHdf5StructureHash`) is computed over an in-memory `hdf5Structure` map where
each attribute carries `{value, unit, data_type, source, component_id, component_type,
document_data?}`. But its own attribute-writing code (`writeHdf5Structure`, ~lines 905-980) only
ever persists **`value`** (as the attribute) and **`unit`** (as a sibling `<name>_unit` attribute)
to the actual `.h5` file for ordinary scalars — `data_type`/`source`/`component_id`/
`component_type` are used only in-memory (dtype selection, attribute-resolution bookkeeping) and
**never survive to disk**. There is also no code anywhere in RDT-CORE that re-reads a `.h5` file
and recomputes/verifies this hash — it's a one-way, write-time-only change-detection fingerprint
for RDT-CORE's own caching (`handleConfigurationVersioning`).

**Consequence**: no reader — not this library in any language, not even RDT-CORE's own Node.js
code — can ever reconstruct RDT-CORE's original hash input from a persisted file, because the
input included data that isn't there anymore. Byte-identical parity with RDT-CORE's stored value
is impossible by construction, not a porting-difficulty problem.

**Adopted approach**: build a new, MCF-native configuration-hash algorithm — same *shape* as
RDT-CORE's (deep-sorted-keys canonical JSON → SHA-256) but computed over MCF's own typed
`MachineConfig`, whose only real requirement is **self-consistency across MCF's own 5 language
implementations** — exactly the same kind of guarantee `correction-hash` (a pre-existing, unrelated
feature) already provides for binary correction grids, verified continuously by
`tools/cross_check.py`. A file MCF's own writer produced will read back `is_valid = true`. A file
authored by anything else (RDT-CORE, the real `fixtures/reference_config.h5`, hand-edited files)
will very likely read back `is_valid = false` — **that is expected and correct, not a bug**: it
means "no other party can forge apparent validity under MCF's own scheme," and "was this
MCF-touched file tampered with since MCF itself last touched it" is the actual, achievable
question this feature answers.

---

## The algorithm, precisely

1. Take the `include_binary=false` canonical representation of the **whole** `MachineConfig`
   (same shape `to_json()`/`export-json` already produce) — scalars/metadata only, never
   `correction_data`/`inverse_correction_data`/`raw_bytes` (a separate, dedicated mechanism —
   `correction-hash` — already exists for binary-grid integrity; duplicating that here would be
   slow and redundant).
2. Delete two keys from `meta`: `configuration_hash` (can't hash itself) and `export_date`
   (changes on every re-export even when nothing configuration-wise changed — RDT-CORE's own
   algorithm excludes its equivalent root `Export_Date` attribute for the same reason).
3. Recursively sort all object keys (arrays keep their existing element order) — this is *not*
   about matching RDT-CORE; it's what makes the result independent of any one language's
   particular field-declaration order, which matters more here than it did for RDT-CORE since we
   have 5 independently-maintained codebases, not one.
4. Serialize compactly (no whitespace) with **explicit, spelled-out formatting rules** — this is
   the single biggest real risk in the whole feature (see below) — then SHA-256 the UTF-8 bytes,
   lowercase hex.

### Why step 4 needs its own explicit rules, not "just use the native JSON serializer"

SHA-256 is byte-sensitive; "structurally equal JSON" (all today's cross-language tests actually
check) is not good enough. Concretely, for a whole-numbered float like `250.0`: JS/Go emit
`"250"`, Python/Rust/C++ emit `"250.0"` — the majority of real fixture values are round numbers,
so left unfixed, hashes would diverge language-to-language on nearly every real file. Two more
default-behavior traps: Go's `encoding/json` HTML-escapes `<>&` unless `SetEscapeHTML(false)` is
set (already worked around in this repo's `export-json` command —
`go/cmd/machine-config-cli/main.go`, reuse the same fix here); Python's `json.dumps` defaults to
`ensure_ascii=True` (must pass `ensure_ascii=False` to match the other four languages' behavior).

Key ordering, by contrast, is *already free* in two languages and needs real code in the other
three: `serde_json::Value` (Rust) and `nlohmann::json` (C++) are both confirmed plain/sorted-map
based here (`preserve_order` is not enabled anywhere in `Cargo.lock`/`Cargo.toml`; C++'s
`ExtraAttrs = nlohmann::json`, not `ordered_json`) — so converting to that language's own JSON
value type and stripping two keys already yields sorted output. Node (plain object, JS key
insertion order) and Go (`encoding/json` on a struct preserves declared field order) both need an
explicit recursive sort step, ported from RDT-CORE's own `deepSortForHash` shape.

---

## Prerequisite (all five languages): stop defaulting unit attributes when unspecified

**This section supersedes the "write → re-read → patch" design Step 1 initially landed on.**
Discovered while writing Step 1's tests, and resolved through discussion before continuing to
the other four languages — read this before implementing any language's step below.

### The problem

Every language's writer has a set of `*_unit` fields that, when the in-memory value is
`None`/`null`/`nullopt`, get **silently replaced with a hardcoded default** on the way to disk —
e.g. Python's `capabilities/v1_0/writer.py`: `grp.attrs["Build_Plate_X_Dimension_unit"] = bp.x_unit
or "mm"`. The in-memory config never actually had `"mm"` in it — the writer materializes that
value out of nowhere at write time. A hash computed from the pre-write config (`None`) therefore
permanently disagrees with a hash computed from what a reader later finds on disk (`"mm"`) —
`is_valid` would spuriously read `false` for every file MCF's own writer ever produced, wherever
any such field was left unspecified. This is what Step 1's test suite caught.

The complete inventory (Python's writer; confirmed identical field set in both `v1_0` and `v1_1`,
and this same pattern is expected to exist in each of the other four languages' writers too —
confirm the equivalent list for each language while implementing its step, don't assume it's
letter-for-letter identical in every codebase):

| Struct | Field | Default |
|---|---|---|
| `Machine.build_plate` (`BuildPlate`) | `x_unit`, `y_unit`, `z_unit`, `corner_radius_unit` | `"mm"` |
| `OpticalTrain` (top-level) | `beam_waist_major_unit`, `beam_waist_minor_unit` | `"μm"` |
| | `beam_waist_offset_z_unit`, `build_plane_offset_major_unit`, `build_plane_offset_minor_unit`, `collimator_focal_length_unit` *(train-level mirror of `Collimator.focal_length_unit`, a separate field)*, `rayleigh_length_major_unit`, `rayleigh_length_minor_unit`, `thermal_lensing_focal_plane_shift_unit`, `thermal_lensing_threshold_unit` | `"mm"` |
| | `major_axis_angle_unit` | `"degrees"` |
| `Scanner` | `working_distance_unit`, `scan_field_x_unit`, `scan_field_y_unit`, `scan_field_z_unit`, `scan_head_offset_x_unit`, `scan_head_offset_y_unit`, `scan_head_offset_z_unit` | `"mm"` |
| | `scan_head_rotation_unit` | `"degrees"` |
| `LightSource` | `wavelength_unit` | `"nm"` |
| | `power_max_nominal_unit`, `power_max_actual_unit`, `power_min_actual_unit`, `power_min_nominal_unit` | `"W"` |
| | `power_bit_resolution_unit` | `"bits"` |
| `Collimator` | `focal_length_unit` | `"mm"` |
| `ScannerCard` | `sample_period_unit` | `"μs"` |

**31 fields total.** Confirmed *not* affected, for contrast: `AxisConfig`'s three `_unit` fields
(`actual_bit_resolution_unit`, `commanded_bit_resolution_unit`, `range_of_motion_unit`) already
use a different, null-safe helper (Python's `_s()`: `None` → `""` on write, `""` → `None` on
read) — no hardcoded default is ever materialized, so these three already round-trip `None` as
`None` with no drift. No numeric or boolean field anywhere in the schema has this
default-materializing pattern — it is specific to these 31 string `_unit` fields.

### The decision: remove the defaulting; make all 31 fields behave like `AxisConfig`'s three already do

Write `""`/`None`/`null`/`nullopt` through when unspecified — the same null-safe pattern already
used for `AxisConfig`'s fields — instead of materializing a default. This is **not** just a
hash-computation convenience; it's independently justified by this codebase's own **Rule 8** (unit
locking, `docs/schema.md`'s Conventions: *"all `*_unit` fields have their values locked at parse
time... raises an error rather than silently returning the wrong unit"*). Since each of these
fields already has exactly one legitimate value system-wide (enforced by the reader), the
"default" was never really *data* — it was the reader's own locked constant, materialized early
for no real benefit. Writing it in only when a caller actually supplied a real value is more
honest about what was actually specified, and removes the entire class of pre-write/post-write
disagreement — not just the one field (`build_plate_radius_unit`) that happened to surface it.

**Consequence for the hash design**: this removes the need for the "write → re-read → patch"
approach entirely. Once no field silently changes value between the in-memory config and what's
on disk, the hash can go back to being computed **once, up front, before writing** — exactly the
simple design this plan originally called for. Every "write-then-reread-then-patch" instruction
elsewhere in this document (Step 1's implementation, and the guidance originally written for
Steps 2-5) is superseded by this — do not implement the reread/patch version.

**Accepted tradeoff, confirmed with the user**: a numeric value that *is* set, paired with a unit
left unspecified, no longer gets a sensible default — it becomes a real value with a blank unit,
rather than e.g. `250.0` / `"mm"`. Accepted as a reasonable consequence of removing the
default-materialization behavior generally, not something to special-case around.

**Scope**: this is a genuine change to the writer's on-disk output in every language — not an
internal implementation detail scoped to hashing — so it must be implemented identically in all
five languages' writers as part of each language's step below, not only where the hash feature
happens to touch. No consumer inside this codebase depends on the old defaulted behavior
(confirmed for Python — see Step 1's rework notes for the exact regression-test check performed;
repeat the equivalent check for each other language before assuming it's equally safe there).

---

## Decisions made (flag at review if you'd rather go another way)

- **Field name**: `is_valid`, per the user's own wording, typed nullable/optional
  (`Option<bool>` / `Optional[bool]` / `boolean | undefined` / `*bool` / `std::optional<bool>`) —
  `None` until a `MachineConfigReader` actually computes it; never set by a writer or by
  `MockConfigBuilder`/`YamlConfigBuilder`.
- **`is_valid` is reader-computed, in-memory-only** — added as a real field on
  `MachineConfigMeta` in every language, but **excluded** from `to_json()`/canonical JSON and from
  the JSON schema (never written to HDF5, never part of the interop contract) — same category of
  exclusion as `facility_id`/`config_author` already get, though the *mechanism* differs by
  language since those two are `None` in every real (non-test) read while `is_valid` will be
  populated on every real read: Rust needs `#[serde(skip)]` + `#[serde(default)]` (not
  `skip_serializing_if`, which wouldn't fire once genuinely populated); Node must extend the
  existing JSON.stringify replacer (`omitFalseInvertFlags` in `capabilities/*/hdf5.ts`) to also
  drop `is_valid`, since Node's `toJson()` stringifies the live runtime object directly with no
  intermediate dict-builder to simply not populate; Python/C++/Go's existing patterns (don't add
  to the dict-builder / don't wire into `to_json`/`from_json` / `json:"-"` tag) work as clean,
  passive exclusions with no extra code.
- **Writer always overwrites `Configuration_Hash` unconditionally** on every write, computed
  fresh from the config's actual content — a caller-supplied value is never trusted or passed
  through, since anything else goes stale the instant any other field changes. This is a real,
  deliberate behavior change from today's pure-passthrough writer.
- **Superseded by the "Prerequisite" section above**: Step 1's first attempt worked around the
  unit-defaulting problem with a "write → re-read → patch the attribute" dance (plus a
  fallback-on-error, since re-reading through the full validating reader could fail for reasons
  unrelated to hashing). That approach is **no longer part of this plan** — once the writers stop
  materializing unit defaults (the Prerequisite fix), the pre-write in-memory config and the
  post-write on-disk content agree on every field, so the hash can simply be **computed once, up
  front, from the config being written**, exactly as originally intended. Do not implement the
  reread/patch version in any language, including a reworked Python.
- **C++**: the library (`machine_config`, a header-only `INTERFACE` target) does not currently
  expose `picosha2` to its own consumers — only `machine_config_cli`/`machine_config_tests` get
  its include dir (confirmed in `cpp/CMakeLists.txt`; `machine_config`'s own `INTERFACE` include
  dirs don't have it). Rather than changing this repo's packaging story, **vendor a small,
  self-contained SHA-256 implementation directly inside the new `hash.hpp`** (no new dependency,
  no CMake/packaging change) — keep `picosha2` exactly as-is today, CLI/test-only.
- **New CLI subcommand**: `configuration-hash <path.h5>`, mirroring `correction-hash`'s exact
  per-language registration shape. Prints `stored: <hex-or-empty>`, `recomputed: <hex>`,
  `valid: true|false`; add a `--quiet` flag that prints only the bare recomputed hex digest (for
  scripting/cross-check subprocess parsing, matching how `correction-hash` already prints one bare
  line). Exit code 1 on mismatch (useful for CI/scripting) — this is a CLI-layer convenience only;
  **the library API itself never raises/errors on mismatch**, matching the explicit
  non-fatal requirement.
- **New shared module per language** (version-agnostic — operates on the already-parsed
  `MachineConfig` StableModel, so it lives beside `MachineConfigReader`/`MachineConfigWriter`, not
  inside any `capabilities/vX_Y/` folder): `python/src/machine_config/hash.py`, `rust/src/hash.rs`,
  `nodejs/src/hash.ts`, `go/hash.go` (package root), `cpp/include/machine_config/hash.hpp`. Each
  exports one function, `compute_configuration_hash(config) -> str` (or the language's naming
  convention), doing exactly the 4 algorithm steps above — called by both the writer (generate)
  and the reader (recompute + compare).

---

## Step 1 — Python (reference implementation)

**Status: done and reworked, verified 2026-09-14.** First implemented the same day with the
write→re-read→patch design (kept below as a historical record); superseded, same day, once its
root cause was traced to the writer's own unit-defaulting behavior — and the rework itself is now
also complete and verified.

### What was originally implemented, and why it was reworked (historical record)

- [x] `python/src/machine_config/hash.py` (new) — `compute_configuration_hash(config: MachineConfig) -> str`.
      Reused `Hdf5AdapterV1_0._config_to_dict(config, include_binary=False)` (`capabilities/v1_0/hdf5.py:697`)
      rather than duplicating it — confirmed it and its helper methods (`_train_to_dict`,
      `_opcua_to_dict`, etc.) never touch `self.path`/do any I/O, so a placeholder-path instance
      (`Hdf5AdapterV1_0(Path("."))`, module-level singleton `_DICT_BUILDER`) is enough to reuse it
      safely, with a comment explaining why. Deletes `d["meta"]["configuration_hash"]` and
      `d["meta"]["export_date"]`, then `json.dumps(d, sort_keys=True, ensure_ascii=False,
      default=str, separators=(",", ":"))` → `hashlib.sha256(...).hexdigest()`. Confirmed
      `sort_keys=True` already recurses through arbitrarily nested dicts — no separate manual
      deep-sort function needed in Python. **This part stays as-is; not affected by the rework.**
- [x] `python/src/machine_config/models.py` — added `is_valid: Optional[bool] = None` on
      `MachineConfigMeta`, with a comment distinguishing it from the `facility_id`/`config_author`
      "TEST FIXTURE" fields directly above it. **Stays as-is.**
- [x] `python/src/machine_config/cli.py` — new `configuration-hash` command mirroring
      `correction_hash` (`cli.py:255-278`): prints `stored:`/`recomputed:`/`valid:` lines, `--quiet`
      flag for bare-digest output, exits 1 on mismatch. **Stays as-is.**
- [x] `python/src/machine_config/writer.py`/`reader.py` — implemented as write → re-read via
      `MachineConfigReader(path).parse()` → compute → patch `Configuration_Hash` in place via
      `h5py.File(path, "a")`, wrapped in `try/except Exception` (falling back to hashing the
      pre-write config) because re-reading through the full validating reader made `write()`
      transitively enforce unrelated reader-side validation (`test_unit_mismatch_raises_value_error`
      started failing inside `write()` instead of at its own explicit read call). **This is exactly
      the design being replaced — see the Prerequisite section above for the root cause (unit
      defaulting) and the Rework checklist below for what replaces it.**

### Rework — completed

- [x] `python/src/machine_config/capabilities/v1_0/writer.py` and `v1_1/writer.py` — removed all
      31 `<field> or "<default>"` fallbacks (all 6 structs: `BuildPlate`, `OpticalTrain`
      top-level, `Scanner`, `LightSource`, `Collimator`, `ScannerCard`; confirmed via `grep 'or "'`
      returning zero matches in both files afterward) and replaced each with the same null-safe
      `self._s(<field>)` write already used for `AxisConfig`'s three unit fields. `AxisConfig`
      itself untouched.
- [x] `python/src/machine_config/writer.py` (`MachineConfigWriter`) — reverted to the simple
      pre-write design: `compute_configuration_hash(...)` is computed once in `__init__` from a
      config that already reflects the resolved `file_version`, folded into one final
      `dataclasses.replace(...)` copy alongside it, and `write()` is back to a single
      `self._backend.write(path)` call. The re-read, the `h5py.File(path, "a")` patch step, and
      the `try/except Exception` fallback are all deleted — `import h5py` removed from this file
      too (no longer needed here).
- [x] `python/src/machine_config/reader.py` — unaffected by the rework, unchanged from Step 1's
      first pass (recompute + compare + set `is_valid`, never raises).
- [x] Confirmed directly: `python/tests/test_reader.py::test_unit_mismatch_raises_value_error`
      passes on its own (`pytest ... -v`, isolated run) — `write()` no longer calls into
      `MachineConfigReader` at all, so the transitive-validation interaction is structurally gone,
      not just avoided by the removed try/except.

### Tests (Python)

- [x] New `test_unset_unit_fields_round_trip_as_none_not_a_default` in `test_hash.py` — one
      representative field per affected struct (`build_plate.corner_radius_unit`,
      `major_axis_angle_unit`, `scanner.working_distance_unit`, `light_source.wavelength_unit`,
      `collimator.focal_length_unit`, `scanner_card.sample_period_unit`) left `None`, written,
      read back, each asserted still `None` — the scenario that had zero prior coverage.
- [x] Confirmed (re-checked post-rework, not just trusted from before): no existing test needed
      updating — `test_reader.py`'s unit assertions check `reference_config` (real, explicitly-set
      values) and `test_writer_roundtrip.py`'s `scalar_rt` fixture explicitly sets all 31 fields
      already; neither exercises the defaulting path.
- [x] `python/tests/test_hash.py` — all 8 tests re-verified passing after the rework (7 from
      Step 1's first pass + the 1 new Prerequisite test above). `test_hash_stable_across_pure_roundtrip`
      and `test_correction_grid_patch_after_write_does_not_invalidate_hash` no longer depend on a
      prior write to "settle" defaults, as anticipated.
- [x] Full suite: `.venv/Scripts/python.exe -m pytest python/tests/ -v` — **411 passed, 0 failed,
      0 skipped** (410 after Step 1's first pass + 1 new test this rework added; no test needed
      removing).
- [x] Manual CLI re-verification: `configuration-hash` against `fixtures/reference_config.h5`
      produces the **identical** `recomputed` hash as before the rework (confirms this real
      fixture never relied on the defaulting behavior in the first place, exactly as predicted);
      against a fresh MCF-writer output, `stored == recomputed`, `valid: true`, exit 0.

**Python is fully done — both the hash feature and its prerequisite fix. Proceeding to Step 2
(Rust) next.**

---

## Step 2 — Rust

**Status: done and verified, 2026-09-14.** Implemented directly with the Prerequisite fix
included from the start (no rework needed — Python's rework had already happened before this
step began, so this step never implemented the superseded re-read/patch design).

- [x] `rust/src/hash.rs` (new) — `pub fn compute_configuration_hash(config: &MachineConfig) ->
      String`. Clones the config, clears the three binary fields
      (`ClearBox::correction_data`/`inverse_correction_data`, `ScanFieldCorrectionFile::raw_bytes`)
      to `None` regardless of what the caller passed in (so a `parse_with_binary()`-sourced config
      hashes identically to a `parse()`-sourced one — covered by `hash::tests::hash_ignores_binary_grid_data`),
      `serde_json::to_value(&config)`, removes `value["meta"]["configuration_hash"]` and
      `["export_date"]`, then `serde_json::to_string(&value)` directly — **no manual
      walk-and-render pass was needed**. This deviates from the plan's speculation: empirically
      verified (`serde_json::to_string(&serde_json::json!(250.0_f64))` → `"250.0"`, not `"250"`;
      confirmed via a throwaway `cargo run` scratch check) that `serde_json`'s own f64 rendering
      (via `ryu`) already always includes a decimal point for whole-valued floats, so
      `Number::is_f64()`-driven forcing would have been redundant. Key ordering is likewise free,
      as predicted (`serde_json::Value`'s map is plain/sorted here, `preserve_order` not enabled).
      `sha2::Sha256::digest(bytes)`, lowercase hex via `format!("{digest:x}")`.
- [x] `rust/src/models.rs:454` area — added `pub is_valid: Option<bool>,` on `MachineConfigMeta`
      with `#[serde(skip, default)]`, doc comment explaining the exclusion mechanism.
- [x] Updated every exhaustive `MachineConfigMeta { .. }` struct-literal site: `models.rs`'s
      `sample_config()`, `builder.rs`, `capabilities/v1_0/hdf5.rs`, `capabilities/v1_1/hdf5.rs`,
      `tests/mock_v1_1/mod.rs` — 3 found via `cargo build` compiler errors, 2 via direct
      inspection; verified exhaustive via a clean `cargo build` and `cargo test --no-run` (zero
      errors).
- [x] **Prerequisite for this language**: confirmed via `grep -n 'unwrap_or("'` that Rust's
      equivalent of Python's 31 `field or "<default>"` fallbacks is the
      `.as_deref().unwrap_or("<default>")` idiom, in the exact same 31 conceptual locations, split
      across `capabilities/v1_0/writer.rs` (26 call sites) and `v1_1/writer.rs` (26 call sites,
      some wrapped across multiple lines by rustfmt). Changed every one from
      `unwrap_or("mm"/"degrees"/"nm"/"W"/"bits"/"μm"/"μs")` to `unwrap_or("")`, leaving the
      pre-existing, already-correct `unwrap_or("")` call sites (ordinary optional strings,
      `AxisConfig`'s three unit fields, v1.1's `Firmware_Version`/`PowerCharacterization` fields)
      untouched. Verified via
      `grep -nE 'unwrap_or\("(mm|degrees|nm|W|bits|μm|μs)"\)' rust/src/capabilities/**/writer.rs`
      → zero matches in both files, mirroring exactly how Python's Step 1 rework was verified.
- [x] `rust/src/writer.rs` (`MachineConfigWriter::write`) — now unconditionally clones
      `self.config`, sets `file_version`, computes `compute_configuration_hash(&corrected)` and
      assigns it to `corrected.meta.configuration_hash`, then resolves and writes once. Removed
      the old conditional (clone only when `target_version` differs) — always computing the hash
      requires a clone regardless, so the branch collapsed into one unconditional path. No
      re-read, no patch, no fallback-on-error.
- [x] `rust/src/reader.rs` — both `MachineConfigReader::parse()` and `parse_with_binary()` now
      route their result through a new private `with_validated_hash(config)` helper: recomputes
      via `compute_configuration_hash`, sets `config.meta.is_valid = Some(recomputed == stored)`,
      returns the config unconditionally (never errors on mismatch).
- [x] `rust/src/main.rs` — new `Command::ConfigurationHash { path: PathBuf, quiet: bool }` variant
      + match arm mirroring `CorrectionHash`'s registration shape: prints `stored:`/`recomputed:`/
      `valid:` lines (or, with `--quiet`, only the bare recomputed hex digest), exits 1 on a
      non-quiet mismatch. Library API (`hash::compute_configuration_hash`,
      `MachineConfigReader::parse`) never raises — the exit-1 behavior lives only in the CLI match
      arm.

### Tests (Rust)

- [x] New test `writer::tests::unset_unit_fields_round_trip_as_none_not_a_default` — Prerequisite
      fix regression guard, one representative field per affected struct (`build_plate_radius_unit`,
      `major_axis_angle_unit`, `scanner.working_distance_unit`, `light_source.wavelength_unit`,
      `collimator.focal_length_unit`, `scanner_card.sample_period_unit`), built via
      `MockConfigBuilder::new(1).build()` with each field explicitly set to `None`, written, read
      back, asserted still `None` — mirrors Python's equivalent test's shape exactly.
- [x] New `#[cfg(test)]` module in `hash.rs` (5 tests): `same_content_same_hash` (stable + 64
      lowercase hex chars), `hash_ignores_its_own_stored_value`, `hash_ignores_export_date`,
      `hash_changes_when_a_scalar_field_changes`, `hash_ignores_binary_grid_data` (parse vs.
      parse_with_binary of the same file hash identically).
- [x] `rust/src/writer.rs`'s `roundtrip_meta_fields` test — dropped its now-invalid
      `configuration_hash` equality assertion (the writer no longer passes it through unchanged);
      added a new `roundtrip_configuration_hash_is_real_and_valid` test asserting the round-tripped
      file's hash is 64 lowercase hex chars, differs from the original externally-authored
      fixture's stored value, and reads back `is_valid == Some(true)`. (`sample_config()` in
      `models.rs` is only used by `models.rs`'s own unit tests, not by any `writer.rs` test, so no
      separate `sample_config()`-based roundtrip test existed to update here.)
- [x] Not touched: `rust/tests/mock_v1_1/mod.rs`'s fictional `-mock` fixtures, beyond the
      mechanical `is_valid: None,` struct-literal addition forced by Rust's exhaustiveness — no
      real hash-computation change for that test version.
- [x] Full suite: `cargo build` clean, `cargo test --no-run` zero errors, then
      `cargo test --no-fail-fast` — **207 passed, 0 failed, 0 ignored** (106 lib unit tests + 5 +
      18 + 30 + 21 + 19 + 6 integration tests + 2 doc-tests). Confirmed the
      `matches_python_golden_file` tests (both the `capabilities::v1_0::hdf5` and `reader` copies)
      still pass unmodified — this feature does not touch `fixtures/reference_output.json`'s
      content, as predicted.
- [x] Manual CLI spot check: `cargo run -- configuration-hash ../fixtures/reference_config.h5`
      printed `stored: 9bc38c92c582a15439fe74990c04bcf75705ca6ac499ef4332460b923400e431`,
      `recomputed: d6e7a73f873dbcc936078df73b55ba29bca9e71005e6b19d2883bc9a6584202d`, `valid: false`,
      exit 1 — **byte-identical** to Python's `machine-config configuration-hash` output on the
      same file (confirmed side-by-side), demonstrating the two languages' algorithms already
      agree even before the Node/Go/C++ steps exist. A `copy-hdf5`-produced self-written copy of
      the same content then read back `stored == recomputed`, `valid: true`, exit 0.

**Rust is fully done. Proceeding to Step 3 (Node.js).**

---

## Step 3 — Node.js

**Status: done and verified, 2026-09-14.**

- [x] `nodejs/src/hash.ts` (new) — `export function computeConfigurationHash(config:
      MachineConfig): string`. Builds its own compact JSON text directly (`renderValue`/
      `renderNumber`, recursive) rather than delegating to `JSON.stringify` — a plain JS object's
      enumeration order is insertion order, not sorted, so key sorting has to happen in this
      hand-rolled renderer, unlike Python/Rust/C++ where it's free. `sanitizeForHash()` builds the
      `include_binary=false` shape (binary correction-grid fields removed from every optical train
      regardless of whether they were populated) and also drops each `Scanner`'s four `invert_*`
      keys when exactly `false` (see "discrepancy found and fixed" below). `meta.configuration_hash`
      /`meta.export_date`/`meta.is_valid` are deleted before rendering. `createHash('sha256')`
      (`node:crypto`, already used by `correction-hash`).
  - **Float-vs-integer rendering, and why the plan's "read the schema" idea was rejected**: JS has
      no runtime int/float distinction (`250` and `250.0` are the identical value), so every
      numeric leaf is rendered with a forced trailing `.0` when whole *except* a hardcoded
      `INTEGER_FIELD_NAMES` set (26 names: OPCUA's `bfs_max_depth`/`publish_interval`/etc.,
      ClearBox's `data_port`/`server_port`/etc., `Actual_Bit_Resolution`/`Commanded_Bit_Resolution`,
      `file_size`, `trigger_stop_ceiling_layers`). The plan originally suggested reading
      `schema/machine_config_v1.schema.json` at runtime to derive this set — **rejected during
      review**: that schema is itself hand-maintained (not generated from the models; confirmed via
      `README.md` — "the schema and data model are defined once" — and no generator script exists
      anywhere in the repo), and a concrete instance of it drifting from other hand-maintained
      descriptions of itself was found in the process (`docs/schema.md` claims OPCUA fields are
      "not part of the JSON schema... does not appear in export-json output", but the schema file
      *does* define `opcua` as a top-level property and Python's `_config_to_dict` *does* include it
      in canonical JSON — the doc is stale, the schema and code agree with each other). Reading the
      schema at runtime would only relocate the drift risk, not remove it. **Decision**: hardcode
      `INTEGER_FIELD_NAMES` in `hash.ts` (derived once from the schema's own `"type": "integer"`
      declarations, confirmed zero name collisions with any `"type": "number"` leaf), and add a
      dedicated test (`hash.test.ts`'s "schema cross-check" describe block) that walks
      `getSchema()` and asserts the hardcoded set matches the schema's integer declarations exactly
      — so a future mismatch between the two is caught by CI instead of silently producing a wrong
      hash. This set does not cover dynamically-named passthrough `extra` attributes (no fixed key
      name to allowlist, and Node's `attrExtra` already collapses int64/float64 attribute values to
      a plain `number` with no provenance retained) — a pre-existing limitation of `toJson()`'s own
      output, not newly introduced by this hash, so out of scope here.
- [x] `nodejs/src/models.ts` — added `is_valid?: boolean;` to `MachineConfigMeta` (optional, no
      existing object-literal construction site needed updating).
- [x] `nodejs/src/capabilities/v1_0/hdf5.ts` and `v1_1/hdf5.ts` — extended the existing
      `omitFalseInvertFlags` `JSON.stringify` replacer in each file to also drop `is_valid`
      unconditionally (kept the function name — only the body grew a `key === "is_valid"` check).
- [x] **Prerequisite for this language**: confirmed via `grep -n '_unit ?? "'` that Node's
      equivalent of Python's 31 fallbacks is the `field ?? '<default>'` idiom passed into the `ws()`
      helper — and that `ws()` itself already does `val ?? ""` internally, so the fix is simply to
      delete the `?? "<default>"` half of each call site, letting `ws()`'s own null-coalescing take
      over (confirmed against precedent: plain optional strings like `gas_flow_direction`/`id`
      already round-trip this way with no `??` at all). Found and fixed all 31 in
      `capabilities/v1_0/writer.ts` (hand-edited) and all 31 in `v1_1/writer.ts` (identical pattern,
      applied via a small Python regex script for speed — `_unit \?\? "[^"]*"\)` → `_unit)` —
      spot-checked the result). Verified via `grep -c '_unit ?? "'` → 0 in both files.
- [x] `nodejs/src/writer.ts` — `MachineConfigWriter`'s constructor now unconditionally builds a
      `versionResolvedConfig` (file_version applied), computes
      `computeConfigurationHash(versionResolvedConfig)`, and builds `resolvedConfig` with that hash
      spliced into `meta` before constructing the version adapter — replacing the old
      conditional-copy-only-on-override logic. No re-read, no patch, no fallback-on-error.
- [x] `nodejs/src/reader.ts` — `MachineConfigReader.parse()` now recomputes the hash from the
      adapter's parsed result and returns a new object with `meta.is_valid` set to the comparison —
      never errors on mismatch.
- [x] `nodejs/src/cli.ts` — new `configuration-hash <file>` command (with a `--quiet` flag)
      mirroring `correction-hash`'s registration shape: prints `stored:`/`recomputed:`/`valid:`
      lines, sets `process.exitCode = 1` on a non-quiet mismatch (library API never raises).

**Discrepancy found and fixed during manual cross-language verification**: the first working
version of `hash.ts` produced a *different* recomputed hash from Python's and Rust's for the same
file (`reference_config.h5`). Root-caused by diffing the two languages' canonical pre-hash JSON
text directly (dumped both to disk, pretty-printed with `sort_keys=True`, `diff`'d): Node's
`MachineConfigReader.parse()` returns the raw model object, where `Scanner.invert_actual_x/y` and
`invert_commanded_x/y` are always-present real booleans — but Python's `_config_to_dict` (and
Rust's `#[serde(skip_serializing_if = "std::ops::Not::not")]`) omit these four keys entirely when
`false`, as part of the *canonical dict/struct shape itself*, not merely as a final
`JSON.stringify`-time cosmetic step. Node already had this same omission rule
(`omitFalseInvertFlags`), but it lived only inside `toJson()`'s `JSON.stringify` replacer — and
this hash's hand-rolled renderer never goes through `JSON.stringify`, so it silently skipped the
rule. Fixed by moving the same omission into `sanitizeForHash()` (delete the key from a shallow
`Scanner` copy when `=== false`) so the hash is computed over the exact same canonical shape
`toJson()` produces, per the plan's own definition of the algorithm. After the fix, `hash.ts`'s own
temporarily-exported `canonicalizeForHash()` was diffed again against Python's canonical text for
`reference_config.h5` — byte-identical (both 10,334 characters, zero diff) — and kept as a
permanent (not just debug) export, since it's useful for exactly this kind of cross-language
troubleshooting in the future.

### Tests (Node.js)

- [x] New test (`writer.test.ts`, "unset unit fields round trip as null, not a default" describe
      block) covering the Prerequisite fix: one representative field per affected struct
      (`build_plate_radius_unit`, `major_axis_angle_unit`, `scanner.working_distance_unit`,
      `light_source.wavelength_unit`, `collimator.focal_length_unit`,
      `scanner_card.sample_period_unit`) built via `MockConfigBuilder`, explicitly set to `null`,
      written, read back, asserted still `null` — mirrors Python's/Rust's equivalent test's shape.
- [x] New `nodejs/tests/hash.test.ts`: 6 black-box tests (stable + 64-char lowercase hex; ignores
      its own stored value; ignores `export_date`; ignores `is_valid`; changes when a scalar field
      changes; ignores binary grid data — `parse()` vs. `parse({includeBinary: true})` hash
      identically) plus the schema cross-check block described above (2 tests: exact-match against
      every `"type": "integer"` leaf name, and no-overlap against every `"type": "number"` leaf
      name) — 8 tests total.
- [x] Updated `nodejs/tests/writer.test.ts`'s roundtrip test (renamed
      `'configuration_hash is real and valid after roundtrip'`) — asserts 64 lowercase hex chars,
      differs from `reference`'s original stored value, and `is_valid === true`.
- [x] `npx tsc --noEmit` clean. `npx vitest run` — **245 passed, 0 failed** across 10 test files
      (includes the 8 new `hash.test.ts` tests and the 1 new Prerequisite test added to
      `writer.test.ts`).
- [x] Manual CLI spot check (after `npm run build`): `node dist/cli.js configuration-hash
      ../fixtures/reference_config.h5` printed `stored: 9bc38c92c582a15439fe74990c04bcf75705ca6ac49
      9ef4332460b923400e431`, `recomputed: d6e7a73f873dbcc936078df73b55ba29bca9e71005e6b19d2883bc9a
      6584202d`, `valid: false`, exit 1 — **byte-identical to Python's and Rust's own output on the
      same file**, confirmed after the invert-flags fix above. A `copy-hdf5`-produced self-written
      copy then read back `stored == recomputed == d6e7a73f...` (matching Rust's own self-written
      result exactly), `valid: true`, exit 0.

**Node.js is fully done. Proceeding to Step 4 (Go).**

---

## Step 4 — Go

**Status: done and verified, 2026-09-14.**

- [x] `go/hash.go` (new) — `func ComputeConfigurationHash(cfg *MachineConfig) (string, error)` and
      `func CanonicalizeForHash(cfg *MachineConfig) ([]byte, error)` (the latter exposed, not just
      an internal detail, for exactly the kind of cross-language diffing Step 3's discrepancy
      needed — see below). `sanitizeForHash()` clears `ClearBox.CorrectionData`/
      `InverseCorrectionData` and `ScanFieldCorrectionFile.RawBytes` on a copy, regardless of
      whether they were populated. `json.Marshal(sanitized)` → `json.Unmarshal` into a generic
      `map[string]any` → delete `meta.configuration_hash`/`meta.export_date` → a hand-rolled
      recursive `renderValue`/`renderNumber` pass writes the final canonical bytes directly (object
      keys sorted via `sort.Strings`, arrays left in order, numbers rendered per the float-vs-integer
      rule below). `crypto/sha256` (stdlib, already used by `correction-hash`).
  - **`Meta.IsValid`'s `json:"-"` tag turned out to make Go the cleanest of all four languages so
      far for is_valid exclusion**: because the tag is unconditional at the struct-definition level,
      `json.Marshal` never includes it regardless of the field's actual value — no active
      `delete(meta, "is_valid")` call was even needed in `hash.go` (Python/Rust/Node.js all needed
      some form of active exclusion; Go's needed none).
  - **Scanner's four `Invert_*` flags also needed no special handling** — unlike Node.js (Step 3),
      where the same omission rule had to be copied into the hash's sanitizer by hand because
      Node's hash bypasses `JSON.stringify` (and thus its `omitFalseInvertFlags` replacer)
      entirely. Go's `Scanner.InvertActualX bool \`json:"invert_actual_x,omitempty"\`` (etc.) already
      omits itself when `false` as a plain `encoding/json` struct-tag rule — and this hash's first
      step genuinely is `json.Marshal` on the real typed struct, so the omission applies for free,
      with no equivalent bug to find. (This is the general reason Go turned out lower-risk than
      Node.js for this feature: every structural decision — field naming, omission, nesting — comes
      from one `json.Marshal` call on the authoritative typed model, not a hand-built approximation
      of its shape.)
  - **Float-vs-integer rendering — verified empirically, not assumed**: `json.Marshal(250.0)`
      produces `"250"`, identical to `json.Marshal(250)` — confirmed via a throwaway `go run`
      scratch check — so Go needs the same forced-`.0` fixup Node.js needs, *not* the "comes for
      free" behavior Rust's `serde_json` turned out to have. However, Go's model already has real
      type information Node's TypeScript types can't carry (`*int` vs `*float64` fields declared
      directly in `internal/models/models.go`) — the ambiguity is introduced only by the
      `json.Marshal → json.Unmarshal` round trip (the *first* Marshal call already collapses
      `250.0` to the text `"250"`, before any Unmarshal happens, so the round trip itself is what
      erases the distinction, not the generic map it lands in). Given that, hardcoding a
      Node.js-style leaf-key-name allowlist in Go would have thrown away information Go's own
      compiler already tracks. Instead, `go/integer_field_names.go` derives the set once via
      `reflect` over `models.MachineConfig`'s own struct tags and field types (walking
      structs/slices/arrays/maps/pointers, classifying each leaf field by its Go `Kind()`) — the
      model's own type declarations are the single source of truth, so unlike Node.js's hardcoded
      set, there is no second copy that can drift and therefore no separate cross-check test is
      needed (a light smoke test exists instead — see Tests below). One real bug caught during this
      derivation: `ScanFieldCorrectionFile.RawBytes []byte` was initially misclassified as
      integer-typed (`byte` is `uint8`, an unsigned integer `Kind()`) — but `encoding/json` always
      marshals `[]byte` as a base64 *string*, never a numeric array, so `stripContainers` was
      special-cased to stop at a `[]byte`-shaped slice rather than unwrap it to `uint8`. After the
      fix, the derived set has exactly 26 names — the same 26 names as Node.js's hardcoded
      `INTEGER_FIELD_NAMES` (cross-checked manually), independently arrived at.
- [x] `go/internal/models/models.go` — added `IsValid *bool \`json:"-"\`` on `MachineConfigMeta`.
      Go struct literals aren't exhaustive; confirmed via a clean build that no existing
      construction site needed updating.
- [x] **Prerequisite for this language**: confirmed via `grep -n '"mm"\|"degrees"\|"nm"\|"W"\|"bits"\|"μm"\|"μs"'`
      that Go's equivalent of Python's 31 fallbacks is `strOrDefault(ptr, "<default>")` — a small
      per-file helper distinct from the already-correct `strOrEmpty(ptr)` used for ordinary optional
      strings (including `AxisConfig`'s three unit fields, matching precedent). Replaced all 31
      `strOrDefault(X, "<default>")` call sites with `strOrEmpty(X)` in each of
      `capabilities/v1_0/hdf5/writer.go` and `capabilities/v1_1/hdf5/writer.go` (32 total matches
      per file before the fix: 31 call sites + 1 function definition; applied via a small Python
      regex script — `strOrDefault\(([^,]+), "[^"]*"\)` → `strOrEmpty(\1)` — for speed, then deleted
      the now-fully-unused `strOrDefault` function from both files). Verified via
      `grep -rn strOrDefault capabilities/` → zero matches, and a clean `go build ./...`.
- [x] `go/writer.go` — `Write` and `WriteAs` now both delegate to a new private `writeResolved`
      that *always* builds a `corrected := *cfg` copy (previously `Write` skipped the copy entirely
      when there was no version override), computes `ComputeConfigurationHash(&corrected)`, and
      sets `corrected.Meta.ConfigurationHash` before calling the adapter — once, up front, no
      re-read, no patch, no fallback-on-error.
- [x] `go/reader.go` — `ParseWithOptions` now recomputes the hash from the adapter's parsed result
      and sets `cfg.Meta.IsValid` to the comparison before returning; `Parse()` (calls
      `ParseWithOptions`) inherits this automatically. Never errors on mismatch.
- [x] `go/cmd/machine-config-cli/main.go` — new `configuration-hash` case (`cmdConfigurationHash`,
      with a `--quiet` flag) mirroring `cmdCorrectionHash`'s registration shape: prints
      `stored:`/`recomputed:`/`valid:` lines, calls `os.Exit(1)` on a non-quiet mismatch (library
      API never raises).

### Tests (Go)

- [x] New test `TestUnsetUnitFieldsRoundTripAsNilNotADefault` (`writer_test.go`) — Prerequisite fix
      regression guard, one representative field per affected struct (`BuildPlateRadiusUnit`,
      `MajorAxisAngleUnit`, `Scanner.WorkingDistanceUnit`, `LightSource.WavelengthUnit`,
      `Collimator.FocalLengthUnit`, `ScannerCard.SamplePeriodUnit`), built via
      `NewMockConfigBuilder()` with `NLasers = 1` and each field explicitly set to `nil`, written,
      read back, asserted still `nil` — mirrors the other languages' equivalent test's shape.
- [x] New `go/hash_test.go` (7 tests): `TestComputeConfigurationHashStableAndWellFormed`,
      `IgnoresItsOwnStoredValue`, `IgnoresExportDate`, `IgnoresIsValid`,
      `ChangesWhenAScalarFieldChanges`, `IgnoresBinaryGridData` (`ParseWithOptions` with/without
      `IncludeBinary` hash identically), plus `TestIntegerFieldNamesSmokeTest` (asserts a handful of
      known integer-typed and known float-typed key names classify correctly — a sanity check on
      the reflection derivation itself, not a cross-check against a second hand-maintained source,
      since none exists here).
- [x] Updated `writer_test.go`'s `TestWriterRoundtripMetaFields` — replaced the direct
      `ConfigurationHash` equality assertion with: 64 lowercase hex chars, differs from the
      externally-authored original's stored value, and `Meta.IsValid` is non-nil and `true`.
- [x] Full suite (`export PATH="/c/msys64/mingw64/bin:$PATH"; CGO_ENABLED=1 go test ./... -count=1
      -v` from `go/`) — **all packages pass**, including the new `hash_test.go` tests and the
      updated/new `writer_test.go` tests; zero unexpected failures elsewhere.
- [x] Manual CLI spot check: built `machine-config-cli` and ran `configuration-hash
      ../fixtures/reference_config.h5` — printed `stored: 9bc38c92c582a15439fe74990c04bcf75705ca6ac4
      99ef4332460b923400e431`, `recomputed: d6e7a73f873dbcc936078df73b55ba29bca9e71005e6b19d2883bc9a
      6584202d`, `valid: false`, exit 1 — **byte-identical to Python's, Rust's, and Node.js's own
      output on the same file, on the first attempt** (no discrepancy this time, unlike Step 3). A
      `copy-hdf5`-produced self-written copy then read back `stored == recomputed ==
      d6e7a73f873d...` (matching every other language's own self-written result exactly),
      `valid: true`, exit 0.

**Go is fully done. Proceeding to Step 5 (C++).**

---

## Step 5 — C++

**Status: done and verified, 2026-09-14.** This turned out to be the lowest-friction language of
all five — every question the plan flagged as needing empirical resolution came back "already
correct, do nothing extra."

- [x] **Empirically verified first, as planned**: compiled a throwaway standalone check
      (`nlohmann::json(250.0).dump()`) via MSVC (`cl.exe` through `vcvars64.bat`, since this repo's
      C++ toolchain is Visual Studio 2022, not MinGW) — printed `"250.0"`, not `"250"`. Confirmed:
      `nlohmann::json`'s own double formatting already forces a trailing `.0` on a whole-valued
      double, exactly like Rust's `serde_json` and *unlike* Node.js/Go (both of which needed a
      hand-rolled numeric renderer for this). **No float-fixup code was needed anywhere in C++.**
- [x] `cpp/include/machine_config/hash.hpp` (new) — `canonicalizeForHash(const MachineConfig&)` /
      `computeConfigurationHash(const MachineConfig&)`. `nlohmann::json j = config;` (the real
      `to_json` ADL call) already yields a plain (not `ordered_json`) sorted-by-construction value;
      `detail::stripBinaryFieldsForHash(j)` erases `correction_data`/`inverse_correction_data`
      (inside each train's `optional_components.clearbox`, when not null) and `raw_bytes` (inside
      `scan_field_correction_file`, when not null) directly on the JSON tree — no C++ struct clone
      needed, unlike Rust/Go/Node.js, since nlohmann::json objects are trivially mutable by key.
      `j["meta"].erase("configuration_hash")`/`.erase("export_date")`, then `.dump()`. SHA-256 is a
      ~100-line vendored `detail::Sha256` (FIPS 180-4, single header, not constant-time — this is a
      content fingerprint, not a secret) — confirmed via `CMakeLists.txt` that `machine_config`'s
      own `INTERFACE` target still does not expose `picosha2` to consumers (unchanged from before
      this step), so vendoring stayed the right call per the existing Decision.
  - **`is_valid` needed zero active exclusion code** — the cleanest outcome of all five languages.
      `models.hpp`'s `to_json(json&, const MachineConfigMeta&)` is a hand-written explicit
      initializer list (not derived from struct-field enumeration), already alphabetically sorted,
      and already never mentions `facility_id`/`config_author` — adding `is_valid` to the struct
      without adding it to that initializer list is the entire fix. Rust needed a `#[serde(skip)]`
      tag, Node.js needed an active `JSON.stringify` replacer entry, Go's exclusion was free via a
      `json:"-"` tag (close to this) — C++ needed *nothing at all* beyond not writing the line.
  - **Scanner's four `Invert_*` flags also needed no special handling** — same reasoning as Go
      (Step 4): `to_json(json&, const Scanner&)` already reads `if (s.invert_actual_x)
      j["invert_actual_x"] = true;` (omit unless true), and this hash's first step genuinely is the
      real `to_json` ADL call, so the omission applies before this header ever touches the tree —
      no equivalent to Node.js's Step 3 bug (whose hash bypassed `JSON.stringify` and, with it,
      `JSON.stringify`'s own replacer).
- [x] `cpp/include/machine_config/models.hpp` — added `std::optional<bool> is_valid;` to
      `MachineConfigMeta`; confirmed via a clean build that no construction site needed updating
      (C++ aggregate/brace-init of this struct isn't used anywhere outside `to_json`/`from_json`
      themselves, which don't need to change either).
- [x] **Prerequisite for this language**: confirmed via `grep -n 'unit\.value_or('` that C++'s
      equivalent of Python's 31 fallbacks is `.value_or("<default>")` directly on the
      `std::optional<std::string>` field (34 total matches per file — 31 real defaults + 3 that
      were already `.value_or("")`, i.e. `AxisConfig`'s three unit fields, matching precedent).
      Replaced all 34 with `.value_or("")` in both `capabilities/v1_0/writer.hpp` and
      `v1_1/writer.hpp` (applied via a small Python regex script — idempotent on the 3 already-empty
      ones — then spot-checked, including the two `\xce\xbc` (μ) UTF-8-escaped defaults). Verified
      via a follow-up grep for the 5 default literals (`"mm"`/`"degrees"`/`"nm"`/`"W"`/`"bits"`) on
      a `_unit.value_or(...)` call → zero matches in both files.
- [x] `cpp/include/machine_config/writer.hpp` — `write()` now unconditionally builds a `corrected`
      copy (previously only copied when `targetVersion_` overrode the adapter choice), computes
      `computeConfigurationHash(corrected)`, and sets `corrected.meta.configuration_hash` before
      resolving and writing — once, up front, no re-read, no patch, no fallback-on-error.
- [x] `cpp/include/machine_config/reader.hpp` — both `parse()` and `parseWithBinary()` now route
      through a new private static `withValidatedHash(MachineConfig)` helper: recomputes via
      `computeConfigurationHash`, sets `config.meta.is_valid` to the comparison, returns
      unconditionally. Never errors on mismatch.
- [x] `cpp/src/main.cpp` — new `configuration-hash` CLI11 subcommand (with a `--quiet` flag)
      mirroring `correction-hash`'s registration shape: prints `stored:`/`recomputed:`/`valid:`
      lines, returns `1` from `main` on a non-quiet mismatch (library API never raises).
- [x] `cpp/CMakeLists.txt` (the library's): unchanged, as planned — SHA-256 is vendored inline, not
      a new dependency. `cpp/tests/CMakeLists.txt` (a different file) *did* need one line — the new
      `test_hash.cpp` registered in the test source list, which the Tests section below already
      called for; the "no CMakeLists.txt change" note was about the library's own build config, not
      the tests'.

### Tests (C++)

- [x] New test `UnsetUnitFieldsRoundTripAsNulloptNotADefault` (`test_writer.cpp`) — Prerequisite fix
      regression guard, one representative field per affected struct (`build_plate_radius_unit`,
      `major_axis_angle_unit`, `scanner.working_distance_unit`, `light_source.wavelength_unit`,
      `collimator.focal_length_unit`, `scanner_card.sample_period_unit`) built via
      `MockConfigBuilder` (`laser_count = 1`) with each field explicitly set to `std::nullopt`,
      written, read back, asserted still without a value — mirrors the other languages' equivalent
      test's shape.
- [x] New `cpp/tests/test_hash.cpp` (registered in `tests/CMakeLists.txt`; 7 test cases):
      `ComputeConfigurationHashStableAndWellFormed`, `IgnoresItsOwnStoredValue`, `IgnoresExportDate`,
      `IgnoresIsValid`, `ChangesWhenAScalarFieldChanges`, `IgnoresBinaryGridData` (`parse()` vs.
      `parseWithBinary()` hash identically), plus
      `ReaderSetsIsValidTrueForSelfWrittenFileAndFalseForReference` — the permanent regression guard
      (this repo's own convention) proving `reference_config.h5` reads back `is_valid == false`
      forever, since it's externally-authored.
- [x] Updated `test_writer.cpp`'s `RoundtripAllScalarFields` — replaced the direct
      `configuration_hash` equality assertion with: 64 chars, differs from the externally-authored
      original's stored value, and `meta.is_valid` is present and `true`.
- [x] Rebuilt both configs fresh: `cmake --build cpp/build --config Debug --target
      machine_config_tests` and `--config Release` — both compiled clean. Ran both
      `cpp/build/tests/{Debug,Release}/machine_config_tests.exe` directly — **both report "All
      tests passed (842 assertions in 162 test cases)"**, identical between configs, 0 failures.
- [x] Manual CLI spot check (both configs built): `machine_config_cli configuration-hash
      ../fixtures/reference_config.h5` printed `stored: 9bc38c92c582a15439fe74990c04bcf75705ca6ac49
      9ef4332460b923400e431`, `recomputed: d6e7a73f873dbcc936078df73b55ba29bca9e71005e6b19d2883bc9a
      6584202d`, `valid: false`, exit 1 — **byte-identical to Python's, Rust's, Node.js's, and Go's
      own output on the same file, on the first attempt** (same as Go in Step 4 — no discrepancy to
      chase this time). A `copy-hdf5`-produced self-written copy then read back `stored ==
      recomputed == d6e7a73f873d...` (matching every other language's own self-written result
      exactly), `valid: true`, exit 0.

**C++ is fully done — all five languages' `configuration-hash` implementations are now complete
and mutually cross-verified by hand against the same fixture. Proceeding to Step 6 (cross-language
verification via `tools/cross_check.py`).**

---

## Step 6 — Cross-language verification

**Status: done and verified, 2026-09-15.** Found and fixed two real, pre-existing issues along the
way — one a genuine latent bug in Python's Step 1 work, one a stale assumption in `cross_check.py`
itself — both exposed by the new phase, neither introduced by it.

- [x] Rebuilt all five languages fresh: `cargo build --release` (Rust), `npm run build` (Node.js),
      `cmake --build build --config Release --target machine_config_cli` +
      `--config Debug --target machine_config_cli` (C++), `CGO_ENABLED=1 go build -o
      bin/machine-config-cli.exe ./cmd/machine-config-cli` with MinGW64 on `PATH` (Go). Python
      needs no build (editable install).
- [x] `tools/cross_check.py` — new `phase_configuration_hash` (Phase 5) + `_configuration_hash_output`
      helper. Deliberately does **not** use `configuration-hash --quiet` (which only prints the bare
      digest) — instead runs the CLI without `--quiet` and parses all three `stored:`/`recomputed:`/
      `valid:` lines from stdout, **ignoring the process's exit code entirely**: the CLI contract
      exits 1 whenever `valid: false`, which is the *expected*, permanent state for every
      externally-authored fixture on every language — not a crash. A real failure is instead
      detected by the three expected lines being absent from stdout. For each fixture, compares
      both the recomputed hash string and the `valid` conclusion across all active languages against
      the Python anchor — not asserting universal validity (an externally-authored fixture is
      expected to be invalid everywhere; that *shared* invalid verdict is what's being verified).
      Registered as Phase 5 in `main()`'s results list, with a matching `--skip-configuration-hash`
      flag.
- [x] Ran the full cross-check — **first run surfaced 2 real failures**, both root-caused with
      direct evidence (not assumptions) before fixing:
  - **Phase 5 itself failed for the `v1_1` fixture only**: Python's recomputed hash disagreed with
    all 4 other languages (which agreed with each other). Root cause: `python/src/machine_config/hash.py`
    always builds its dict via `Hdf5AdapterV1_0` regardless of the config's actual `file_version`,
    on the documented assumption that v1.0's and v1.1's `_config_to_dict()` "must already produce
    identical output for identical MachineConfig content." Verified that assumption directly by
    diffing both adapters' dict output field-by-field, on the same real parsed config, across
    **all 7 available fixtures** (not just the failing one) — the *only* discrepancy anywhere was
    `ClearBox.firmware_version` (a real StableModel field per the schema, `schema/machine_config_v1.schema.json:316`,
    unconditionally — not scoped to v1.1), silently omitted by v1.0's `_clearbox_to_dict`. Fixed by
    adding the same `if cb.firmware_version is not None: d["firmware_version"] = cb.firmware_version`
    line v1.1's `_clearbox_to_dict` already has (`python/src/machine_config/capabilities/v1_0/hdf5.py`).
    Re-verified zero remaining diffs across all 7 fixtures after the fix. New regression test:
    `python/tests/test_hash.py::test_v1_0_and_v1_1_dict_builders_agree_on_firmware_version`.
  - **Phase 3 (Write Interop)'s pre-existing "fidelity" check failed for all 5 writer languages** —
    not a Configuration Hash bug at all, but a stale assumption in `cross_check.py` predating this
    feature: fidelity compares a writer's round-tripped output against the *original* canonical
    JSON, which still carries its old, externally-authored `configuration_hash`. Every writer now
    unconditionally recomputes that field (the entire point of this feature), so a difference there
    is expected and correct, not a defect. Fixed with a new `_without_configuration_hash()` helper
    that strips `meta.configuration_hash` before the fidelity diff only (parity checks, which
    compare reader outputs against each other rather than against the stale original, were
    correctly left untouched and continued to pass throughout).
  - Confirmed via Python's full test suite (`pytest`, 412 passed, up from 411) that the
    `firmware_version` fix caused no regressions elsewhere.
- [x] Re-ran the full cross-check after both fixes: `PYTHONIOENCODING=utf-8 python
      tools/cross_check.py --verbose` (MinGW64 on `PATH`) — **all 6 active phases passed** (Phase 1
      schema validation skipped — `jsonschema` not installed in this environment, a pre-existing,
      unrelated gap).
- [x] Manual spot check: ran `configuration-hash` in all 5 languages against
      `fixtures/reference_config.h5` — all report the identical `recomputed` hash and `valid: false`
      (already covered exhaustively by Phase 5's own automated comparison, run here as a direct
      sanity check too). Separately created one self-written file (`copy-hdf5` via Python) and ran
      `configuration-hash` against it in all 5 languages — all five report the identical `stored`/
      `recomputed` value and `valid: true` (Phase 5 only exercises pre-existing, externally-authored
      fixtures, which are always invalid, so this covers the complementary "freshly-written → valid"
      case Phase 5 doesn't).

**All checks pass. Proceeding to Step 7 (Fixtures).**

---

## Step 7 — Fixtures

**Status: done and verified, 2026-09-15.**

- [x] Confirmed no regeneration needed for any existing static fixture (`reference_config.h5` and
      siblings) — ran `tools/generate_fixtures.py`, which always regenerates
      `reference_output.json`/`.sha256` from `reference_config.h5` alongside `synthetic_2laser.h5`
      (no flag to run them independently). Verified by parsed-value comparison (not raw text —
      git's CRLF normalization made a first naive `diff` look alarming even though nothing had
      changed) that `reference_output.json` and `.sha256` are byte-for-byte/hash-for-hash identical
      before and after: `reference_config.h5` itself is only ever read, never written, by this
      script, and its `to_json()` shape doesn't change (`is_valid` stays passively excluded, and
      `configuration_hash` in that output is the file's own stored value, not a recomputed one).
- [x] Regenerated `synthetic_2laser.h5` via `tools/generate_fixtures.py`.
- [x] **Correction to this step's original wording**: diffing `synthetic_2laser.h5`'s own
      `export-json` output before vs. after (not `reference_output.json`, which is derived from a
      *different* fixture entirely and was never expected to move — the original bullet named the
      wrong file) showed **8** changed fields, not 2. Investigated each one before accepting the
      diff: `machine.id`, both trains' `scan_field_correction_file.document_id`, `meta.export_date`,
      and both trains' `scan_field_correction_file.valid_as_of_date` are `uuid.uuid4()`/"now"-based
      in `MockConfigBuilder` (`python/src/machine_config/builder.py`) — inherently non-deterministic
      on *every* regeneration, with or without this feature, confirmed by reading the builder's own
      source rather than assuming. The other 2 are exactly what this step predicted:
      `machine.build_plate_radius_unit` (`"mm"` → `null`) and `meta.configuration_hash` (a real,
      self-consistent value). Also confirmed via repo-wide grep that nothing hardcodes the fixture's
      old random UUIDs, old hash, or old `"mm"` unit value, so none of the 6 incidental changes
      could have silently broken a brittle assertion anywhere.
- [x] `fixtures/synthetic_2laser.h5` is shared by all five languages' test suites (not just
      Python's), so **re-ran all five** after regeneration, not only Python's: Python (`pytest`,
      412 passed), Rust (`cargo test`, all crates 0 failed), Node.js (`vitest run`, 245 passed), Go
      (`go test ./...`, all packages ok), C++ (both Debug and Release `machine_config_tests`
      rebuilt fresh, 842 assertions / 162 test cases passing in both).
- [x] Re-ran the full cross-check (`tools/cross_check.py --verbose`) once more after regeneration —
      all 6 active phases still pass, and Phase 5's `synthetic` row correctly flipped from
      `valid=false` (the fixture's pre-feature stored hash) to `valid=true` (its freshly-regenerated,
      self-consistent one) across all 5 languages.

**Proceeding to Step 8 (Documentation cleanup) — held per the sequencing note there until
`CAPABILITIES_HASH_INTEGRATION_PLAN.md`'s Rust step is done.**

---

## Step 8 — Documentation cleanup

**Status: done, 2026-09-15.** Originally held (sequencing note below, kept for the record) until
`CAPABILITIES_HASH_INTEGRATION_PLAN.md`'s Rust step was done; by the time this step was actually
picked up, all five languages of that plan were complete, which changed *which* API this section
needed to document — see the note below the original sequencing note.

**Original sequencing note (2026-09-14):** `docs/clearbox-tauri-integration.md` is written for
clearbox-tauri, which integrates exclusively through the capabilities facade
(`open_machine_config`/`MachineConfigFileV1_0`), not `MachineConfigReader` — and that facade
didn't compute `is_valid` at all at the time this note was written (see
`CAPABILITIES_HASH_INTEGRATION_PLAN.md`'s "Critical finding"). Rewriting §6 then would have
described an API clearbox-tauri doesn't call, so the doc would still have been misleading (or at
best moot) for its actual reader. Do Steps 6-7 below first (they're independent of the
capabilities gap), then the capabilities plan's Rust step, then come back here.

**What actually landed differs from this step's original checklist item in one way, precisely
because of the finding above**: the original wording below said to document
`config.meta.is_valid` (the `MachineConfigReader`-computed field). That field stays permanently
unset on every config the capabilities facade touches (confirmed in
`CAPABILITIES_HASH_INTEGRATION_PLAN.md`) — and clearbox-tauri only ever uses the capabilities
facade — so documenting it would have reintroduced the exact same "describes an API the real
consumer doesn't call" problem the sequencing note above was written to avoid. §6 instead
documents `MachineConfigFile::is_valid()`, the live method that plan added to the facade.

- [x] `docs/clearbox-tauri-integration.md` §6 "Configuration Hash Validation" — rewritten: removed
      the fictional `reader.compute_hash()`/`assert_eq!`-hard-fails framing (contradicted the
      non-fatal design and referenced a method/constructor that was never real), replaced with the
      actual pattern clearbox-tauri's real code path supports — `open_machine_config()` returns a
      `MachineConfigFile` session, call `is_valid()` on it (live, recomputed every call, not cached
      at `open()`), log/report a `false` result rather than fail, and treat `false` immediately
      after an edit as expected (not alarming) since nothing recomputes the hash until the next
      `save()`. Added a one-line callout explaining why this section's API (`open_machine_config`)
      differs from §2's own Phase-1 sketch (`MachineConfigReader`) — clearbox-tauri's real
      `reader.rs` uses the former, confirmed by direct inspection during
      `CAPABILITIES_HASH_INTEGRATION_PLAN.md`'s investigation, not the migration guide's original
      suggestion.
- [x] Did not hand-author a `CHANGELOG.md` entry — landed as `feat:`/plan-doc commits;
      semantic-release generates the changelog from commit messages.
- [x] Updated this file's Status line (top of document) to `done and verified, all five languages,
      2026-09-15`, following the closed-out style of `SCANNER_INVERT_FLAGS_PLAN.md`/
      `RECOATER_BLADE_TYPE_PLAN.md`.

---

## On step granularity

Steps 1-5 (one per language) match the granularity `RECOATER_BLADE_TYPE_PLAN.md` used
successfully for a smaller change. This feature carries more per-language risk than that one did
(a genuinely new cross-language algorithm, not just a new passthrough field) — concretely, the
float-rendering fixup (Rust/Node/Go/C++) and the C++ SHA-256/`nlohmann` verification are real,
non-mechanical implementation work, not just "copy the pattern from the last field." If any
single language's step turns out to need more back-and-forth than expected once underway (e.g.
the float-fixup logic needs iteration, or cross-language hash values don't match on the first
attempt), treat that as a reason to pause and confirm before continuing within that same step,
rather than a reason to have pre-split it further — the risk is concentrated in *getting the
algorithm right per language*, which doesn't shrink by chopping the step into smaller prompts
ahead of time. One prompt per language/phase step (as listed above: 5 language steps + verification
+ fixtures + docs = 8 total) is the recommended granularity; treat Step 2 (Rust) and Step 5 (C++)
as the two most likely to need mid-step check-ins, given Rust's exhaustive-struct-literal
mechanics and C++'s upfront `nlohmann` empirical-verification requirement.
