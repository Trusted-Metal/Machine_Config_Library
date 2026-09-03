# Scanner Invert Flags — Implementation Record

**Status: done and verified, all five languages, 2026-08-21.**

**Scope:** add four new optional boolean attributes directly on `Scanner` (not nested under
`AxisConfig`, despite the "Actual"/"Commanded" naming echoing that struct — confirmed with the
user these are flat fields on `Scanner` itself, matching the existing flat X/Y-suffixed field
pattern already used there, e.g. `scan_head_offset_x`/`scan_head_offset_y`):

| Field → on-disk attribute |
|---|
| `invert_actual_x` → `Invert_Actual_X` |
| `invert_actual_y` → `Invert_Actual_Y` |
| `invert_commanded_x` → `Invert_Commanded_X` |
| `invert_commanded_y` → `Invert_Commanded_Y` |

No `File_Version` bump — pure additive optional field, same rule as every other addition in this
schema (`docs/contributing.md`: *"New optional field — Safe."*).

---

## The design question, and how it was resolved

The initial ask was "same as `enabled` on `SynchronousSensor`" — but `enabled` is
`Optional<bool>`/`bool | null`, serialised as `null` when absent. That turned out not to be what
was wanted here. Resolved through direct discussion into a genuinely different, simpler contract
than any other bool field in this schema:

- **API/model layer**: a plain `bool` in every language — never `Optional`/nullable. Always
  `true` or `false`; an application calling the library directly gets a real boolean, full stop.
- **Read**: attribute `1` → `true`; attribute `0` → `false`; attribute absent → `false`. The API
  never distinguishes "explicitly recorded as false" from "never recorded" — both are just `false`.
- **Write — to *any* output, HDF5 or JSON**: only emit the attribute/key when `true`. `false`
  never appears anywhere it's written back out, not as `0`, not as `null` — it's simply absent.

**The one deliberate, confirmed asymmetry**: reads are fully faithful (an attribute explicitly
stored as `0` reads back as `false`, correctly, in the same session). Writes are lossy for
`false` specifically — writing a config back out (to a new `.h5` file or to JSON) drops any
`false`-valued flag entirely, indistinguishable from one that was never set. A read→write→read
round-trip still produces the correct *value* (`false`), but the fact that it was ever explicitly
recorded does not survive. User-confirmed as the intended behavior, not an oversight.

This is different from `MachineConfig.opcua` (whole-feature-optional, omitted when absent) and
from `OpcuaConfig.triggers`/`ClearBox.synchronous_sensors` (map-shaped, omitted-when-empty or
always-`{}`) — this is the first and only *scalar* field in the schema where the value `false`
itself, not just absence, controls whether it's serialised at all.

---

## Per-language implementation

### Rust

- [x] `rust/src/models.rs` — `Scanner` gained four `bool` fields (not `Option<bool>`) with
      `#[serde(default, skip_serializing_if = "std::ops::Not::not")]` — the standard serde idiom
      for "skip when false", needing no custom predicate function.
- [x] `rust/src/capabilities/v1_0/hdf5.rs` — `parse_scanner` reads via the existing
      `read_bool_from_int` helper, collapsed with `.unwrap_or(false)`.
- [x] `rust/src/capabilities/v1_0/writer.rs` — new `wb_if_true(grp, key, val: bool)` helper
      (distinct from the existing `wb`, which always writes something — `wb_if_true` writes
      nothing at all when `false`), called from `write_scanner`.
- [x] Construction sites fixed: `models.rs`'s own sample-data test helper, `builder.rs`'s
      `MockConfigBuilder` scanner (plain `bool` fields with no default require every
      struct-literal site to specify them — the compiler enforced this, no site was missed).
- [x] Tests (2 new, in `capabilities/v1_0/writer.rs`'s test module):
      `invert_flags_absent_from_fixture_read_as_false_and_omitted_from_json`,
      `invert_flags_write_only_true_values_survive_as_real_hdf5_attributes` (raw `hdf5::File`
      inspection via `attr_names()` — proves the `false`-valued attributes are genuinely absent
      from the written file, not just read back as `false`).

**Verification:** `cargo test` — **141 passed, 0 failed** (up from 139).

### Python

- [x] `python/src/machine_config/models.py` — `Scanner` gained four `bool = False` dataclass
      fields (defaults required since they follow already-defaulted `z_axis`/`focus` fields).
- [x] `python/src/machine_config/capabilities/v1_0/hdf5.py` — `_parse_scanner` reads via
      `_read_bool_from_int(...) or False`; `_scanner_to_dict` builds its dict as a local variable
      and conditionally adds each `invert_*` key only `if s.invert_actual_x: d["invert_actual_x"]
      = True` (etc.); the reverse `_scanner_from_dict`-equivalent reads via `s.get("invert_actual_x",
      False)`.
- [x] `python/src/machine_config/capabilities/v1_0/writer.py` — `_write_scanner` writes each
      attribute conditionally (`if s.invert_actual_x: grp.attrs["Invert_Actual_X"] = 1`) — no
      write call at all when `False`.
- [x] Tests (4 new): `test_reader.py`'s `TestScanner` class gained
      `test_invert_flags_absent_from_fixture_read_as_false` and
      `test_invert_flags_omitted_from_json_when_false`; `test_writer_roundtrip.py` gained a new
      `TestScannerInvertFlags` class with `test_defaults_to_false_and_omitted_from_json` and
      `test_only_true_values_survive_as_real_hdf5_attributes` (raw `h5py` inspection of
      `f.attrs`, mirroring Rust's raw-attribute-presence check).

**Verification:** `pytest` — **354 passed, 0 failed** (up from 350).

### Node.js

**The one language requiring a genuinely different mechanism**, because it has no separate
model-to-dict serialisation layer — `toJson()` is a direct `JSON.stringify(config)`. A plain
object property that's ever assigned `false` would always serialise as `false`; there is no way
to get both "the API returns a real `false`" and "JSON omits it" from the same plain property.

- [x] `nodejs/src/models.ts` — `Scanner` gained four plain `boolean` fields (not `boolean | null`,
      the one deliberate departure from this file's established "`T | null` over `?:`"
      convention for exactly this reason).
- [x] `nodejs/src/capabilities/v1_0/hdf5.ts` — `parseScanner` reads via `attrBool(a, "Invert_Actual_X")
      ?? false` (etc.). New module-level `omitFalseInvertFlags` — a `JSON.stringify` **replacer
      function** (`(key, value) => ...`) that returns `undefined` for any of the four `invert_*`
      keys when the value is exactly `false`, letting `JSON.stringify` drop the key. `toJson()`
      now calls `JSON.stringify(config, omitFalseInvertFlags, indent)` instead of passing `null`
      as the replacer. This keeps `Scanner.invert_actual_x` a genuine, always-real `boolean` at
      the model/API layer — the replacer only affects the one JSON-serialisation call site.
- [x] `nodejs/src/capabilities/v1_0/writer.ts` — new `wbIfTrue(grp, key, val: boolean)` helper,
      writes nothing at all when `false`.
- [x] `nodejs/src/builder.ts` — `mockScanner`'s object literal updated (required fields, no
      `?:`, so `tsc` enforced this).
- [x] Tests (4 new): `reader.test.ts`'s scanner `describe` block gained two `it`s (defaults, and
      JSON omission via a real `toJson()` call — not just inspecting the in-memory object, since
      the replacer is the whole point); `writer.test.ts` gained a new `describe('MachineConfigWriter
      — Scanner invert_* flags', ...)` block, including a raw `h5wasm` attribute-presence check.

**Verification:** `npx tsc --noEmit` clean, `npx vitest run` — **185 passed, 0 failed** (up from 181).

### Go

**The simplest of the five** — Go's standard `encoding/json` `omitempty` tag already treats a
plain `bool`'s zero value (`false`) as empty and omits it, which is exactly the desired rule with
zero custom serialisation code.

- [x] `go/internal/models/models.go` — `Scanner` gained four plain `bool` fields (not `*bool`,
      the same deliberate departure as every other language) with `json:"invert_actual_x,omitempty"`
      (etc.).
- [x] `go/capabilities/v1_0/hdf5/hdf5.go` — `parseScanner` reads via the existing
      `readBoolFromIntAttr` helper, collapsed with `iax != nil && *iax`.
- [x] `go/capabilities/v1_0/hdf5/writer.go` — new `wbIfTrue(g, key, v bool) error` helper
      (distinct from the existing `wb`, which takes `*bool` and always writes something).
- [x] No construction-site fixes needed — Go struct literals silently zero-value missing fields
      (unlike Rust's compiler-enforced struct literals), so this was a purely additive change at
      every existing call site.
- [x] Tests (4 new): `reader_test.go` gained `TestScannerInvertFlagsAbsentFromFixtureReadAsFalse`
      and `TestScannerInvertFlagsOmittedFromJSONWhenFalse`; `writer_test.go` gained
      `TestWriterRoundtripInvertFlagsDefaultFalseAndOmittedFromJSON` and
      `TestWriterOnlyTrueInvertFlagsSurviveAsRealHDF5Attributes` (raw `h5c.Group.HasAttr`
      inspection).

**Verification:** `go test ./... -count=1` — **78 passed, 0 failed** (up from 74). Requires
`CGO_ENABLED=1` and MSYS2's real MinGW64 toolchain on `PATH` in this environment — see this repo's
other `*_PLAN.md` files for the recurring note on this environment's `gcc` resolution quirk.

### C++

- [x] `cpp/include/machine_config/models.hpp` — `Scanner` gained four `bool` fields (default
      `{false}`, not `std::optional<bool>`).
- [x] `Scanner`'s `to_json`/`from_json` (same file) — `to_json` conditionally assigns
      `if (s.invert_actual_x) j["invert_actual_x"] = true;` (etc.) after building the rest of the
      object; `from_json` reads via `j.value("invert_actual_x", false)`.
- [x] `cpp/include/machine_config/capabilities/v1_0/hdf5.hpp` — `parseScanner` reads via the
      existing `readBoolFromInt(...).value_or(false)`.
- [x] `cpp/include/machine_config/capabilities/v1_0/writer.hpp` — new template helper
      `wbIfTrue(Loc&, key, bool)`, mirroring the existing `wb` template but writing nothing at all
      when `false`.
- [x] Tests (4 new test cases, 26 new assertions): `test_reader.cpp` gained
      `ScannerInvertFlagsAbsentFromFixtureReadAsFalse` and
      `ScannerInvertFlagsOmittedFromJSONWhenFalse`; `test_writer.cpp` gained
      `RoundtripInvertFlagsDefaultFalseAndOmittedFromJSON` and
      `OnlyTrueInvertFlagsSurviveAsRealHDF5Attributes` (raw `HighFive::Group::hasAttribute`
      inspection).

**Verification:** `cmake --build build --config Debug --target machine_config_tests` — **559
assertions in 100 test cases, 0 failures** (up from 533/96).

---

## Schema

- [x] `schema/machine_config_v1.schema.json` — added `invert_actual_x`/`invert_actual_y`/
      `invert_commanded_x`/`invert_commanded_y` under `scanner`'s `properties`, each
      `{"type": "boolean", "description": "Omitted entirely (not present as false) unless true."}`
      — plain `"boolean"`, not `["boolean", "null"]`, since `null` is never a real value here; not
      added to any `required` list, since these are never required.
- [x] Synced both bundled copies byte-for-byte: `python/src/machine_config/machine_config_v1.schema.json`
      (manual copy) and `nodejs/schema/machine_config_v1.schema.json` (regenerated via `npm run
      build`'s `prebuild` step). Confirmed identical via `diff` after syncing.
- [x] Re-validated the schema itself is well-formed Draft 2020-12
      (`jsonschema.Draft202012Validator.check_schema`).

---

## Cross-language verification

Full test suites, all five languages, all green (see per-language sections above for exact
counts). Beyond each language's own suite:

- **`tools/cross_check.py --langs python,rust,nodejs,go,cpp`** (Phase 2, Read Parity): all five
  languages agree on all three shared fixtures (`reference_config.h5`, `reference_config_opcua.h5`,
  `synthetic_2laser.h5`) — 30/30 comparisons. None of these fixtures has any `Invert_*` attribute
  set, so this specifically confirms the "absent → `false`, omitted from JSON" path across all
  five with no regressions.
- **Real true/false/absent mix, checked directly** (none of the three shared fixtures exercise a
  `true` value, so this was checked separately): a copy of `reference_config.h5` was patched via
  `h5py` to set `Invert_Actual_X = 1` and `Invert_Commanded_X = 1` directly (leaving
  `Invert_Actual_Y`/`Invert_Commanded_Y` untouched, i.e. absent). All five CLIs'
  `export-json` output was compared directly: Python, Rust, Node.js, Go, and C++ all produced
  identically-shaped output — `invert_actual_x`/`invert_commanded_x` present as `true`,
  `invert_actual_y`/`invert_commanded_y` absent from the JSON object entirely (not `false`, not
  `null`).
- **Environment note, recorded once, applies to every verification step above**: this repo's
  default shell has no working `gcc` on `PATH`, and `cross_check.py`'s C++/Go binary discovery
  prefers a `Release` build over `Debug` when both exist — a *stale* `Release` binary from days
  earlier was mistakenly picked up once during this work and looked like a real cross-language
  bug in unrelated OPCUA code before being traced to the stale binary. Both CLI configs
  (`Debug`/`Release` for C++; the single Go binary) were rebuilt fresh before the final
  verification pass above, specifically to avoid repeating that mistake.

---

## Documentation

- [x] This file.
- [ ] `docs/{rust,python,nodejs,go,cpp}.md` — not yet checked for a place enumerating `Scanner`'s
      fields explicitly; update if found (same follow-up item `SYNCHRONOUS_SENSOR_PLAN.md` left
      open for its own fields, still outstanding as of this writing).
