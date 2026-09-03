# OPCUA Field Promotion + Required-Field Validation — Implementation Plan

**Status: the entire 5-language rollout is complete.** Required/optional split confirmed (see
"The 23 fields" table and "Required/optional list — confirmed" at the bottom). Phase 0's shared
artifacts are done and verified (schema, reference fixture, `opcua_missing_required.h5` — see
Phase 0 below). **Rust, Python, Node.js, Go, and C++ are all fully done** — Phase 1, Phase 2, and
validation-app coverage all complete and verified for each (Rust: 125/125 crate tests, 19/19
validation-app scenarios; Python: 321/321 tests, 22/22 validation-app scenarios; Node.js: 169/169
tests, 22/22 validation-app scenarios; Go: 59/59 tests, 19/19 validation-app scenarios; C++:
463/463 assertions across 87 test cases, 19/19 validation-app scenarios).

**Note on cross-language CI during this rollout:** `tools/cross_check.py`'s Phase 2 (Read Parity)
will show expected, temporary failures between whichever languages have completed their OPCUA
Phase 1 and whichever haven't — Rust and Python now agree with each other but disagree with
Node.js/Go/C++ until each of those completes its own Phase 1. This is not a regression; it
resolves language by language as the rollout proceeds and fully resolves once all five are done.

**Scope:** promote 22 attributes currently swept into `extra` on `OpcuaClientConfig`/
`OpcuaPipeConfig`/`OpcuaTrigger` to named, typed fields, add one new field
(`trigger_stop_ceiling_layers`) to `OpcuaConfig`, and add a `capabilities`-facade-layer check that
a confirmed subset of these are populated when OPCUA is present at all — across all five
languages (Rust, Python, Node.js, Go, C++). The required-field *check* (Phase 2) also covers
`triggers_enabled` — an already-existing, already-typed `OpcuaConfig` field, not one of the 23
being promoted/added, but the CSV attribute reference marks it "master gate for trigger
automation," required, and there's no reason to scope the validation check more narrowly than
what's actually required just because a field predates this work.

**Explicitly not part of this work:**
- The low-level `MachineConfigReader`/`Writer` stays exactly as permissive as it is today, in
  every language. "Required" is a `capabilities`-facade-only concept (see "Why facade-only"
  below).
- The pre-existing cross-language inconsistency in the *low-level* reader/writer's error types
  (Rust has a rich `thiserror` enum, Python has two independently-subclassed exceptions with no
  common base, Node.js/Go each have one typed error, C++ has none) is a real, separate item.
  Noted, parked, will be scoped on its own later.
- No `File_Version` bump. Verified against `docs/contributing.md`'s documented policy: a new
  version is only required for attribute rename/move/removal, or a field with no v1.0
  representation. None of that applies — every promoted attribute already exists at its current
  name and location on disk; this is purely modeling data that was already there.

---

## Why facade-only enforcement

The low-level reader is deliberately permissive everywhere in this library (`Option<T>` on nearly
every field, unknown attributes preserved in `extra` rather than rejected, no hard requirements).
The `capabilities` layer exists specifically to be the stricter, app-friendly surface on top of
that permissive core, and it already has the right primitive for this: every language's
`open_machine_config`/`OpenMachineConfig`/equivalent returns a `Result<T, CapabilityError>` (not
an exception/panic), `CapabilityError` already carries a closed set of codes
(`UnsupportedVersion`, `UnsupportedInVersion`, `NotPresent`, `ValidationError`, `IoError`,
`InvalidIndex`, `Closed`) consistent across Rust/Python/Node.js/Go (C++'s `code` is currently a
bare `std::string`, not a closed type — noted below as an optional cleanup), and every language
already has a dedicated `.opcua()`/`get_opcua()`/`getOpcua()` accessor at this layer.

Putting the same "is this field populated" check in the low-level reader *too* would duplicate
the check in two places per language — exactly the kind of duplication that silently drifts out
of sync (see the Python `results.md` incident from the file-consolidation work: a real behavior
fix landed in the reader and the downstream doc was never re-run). One enforcement point, at the
layer that already owns validation semantics, is the safer design as well as the cheaper one.

**Consequence, stated plainly:** a consumer using `MachineConfigReader`/`Writer` directly, bypassing
`capabilities`, gets no enforcement — a "required" field is silently `None` for them, the same as
an optional one. This is a deliberate trade-off, not an oversight.

---

## The 23 fields

All verified directly against `fixtures/reference_config_opcua.h5`'s raw HDF5 attributes — every
mapping below is an exact match, no naming exceptions.

| Struct | Field → on-disk attribute | Type | Required when OPCUA present? |
|---|---|---|---|
| `OpcuaClientConfig` | `keep_alive_count` → `Keep_Alive_Count` | int | N |
| | `lifetime_count` → `Lifetime_Count` | int | N |
| | `machine_profile` → `Machine_Profile` | string | **Y** |
| | `queue_policy` → `Queue_Policy` | string | N |
| | `queue_size_data_change` → `Queue_Size_Data_Change` | int | N |
| | `queue_size_events` → `Queue_Size_Events` | int | N |
| | `reconnect_interval` → `Reconnect_Interval` | int | N |
| | `root_node` → `Root_Node` | string | **Y** |
| | `sync_loop_interval_initial` → `Sync_Loop_Interval_Initial` | int | N |
| | `sync_loop_interval_settled` → `Sync_Loop_Interval_Settled` | int | N |
| `OpcuaPipeConfig` | `configure_client` → `Configure_Client` | bool (int 0/1) | **Y** |
| | `inbound_rate_limit` → `Inbound_Rate_Limit` | int | N |
| | `max_inbound_message_size` → `Max_Inbound_Message_Size` | int | N |
| | `min_integrity_level` → `Min_Integrity_Level` | string | N |
| | `pipe_name` → `Pipe_Name` | string | **Y** |
| | `user_access_level` → `User_Access_Level` | string | N |
| `OpcuaTrigger` (per-trigger subgroup) | `case_sensitivity` → `Case_Sensitivity` | string | N |
| | `component` → `Component` | string | N |
| | `cooldown_period` → `Cooldown_Period` | int | N |
| | `event` → `Event` | string | **Y** (checked per-trigger — see Phase 2) |
| | `max_fires_per_job` → `Max_Fires_Per_Job` | int | N |
| | `trigger_label` → `Trigger_Label` | string | N |
| `OpcuaConfig` (new field, on `OPCUA/Triggers` group itself, alongside `Triggers_Enabled`) | `trigger_stop_ceiling_layers` → `Trigger_Stop_Ceiling_Layers` | u32/int | **Y** |

All 23 are `Option<T>` / `Optional[T]` / `T \| null` / `*T` / `std::optional<T>` in every
language's model — uniform regardless of which end up required, since "required" lives only in
Phase 2's facade check, not in the model shape. Note: `OpcuaConfig` has no `extra` bucket today
(only `Client`/`Pipe`/`Trigger` do), so `trigger_stop_ceiling_layers` is a direct read/write, not
a known-keys update.

---

## Phase 0 — shared artifacts (once, not per language)

**Status: done and verified, 2026-08-19** (the `CapabilityError.details` item is deliberately
deferred — it's per-language in practice, so it's done alongside each language's Phase 2 work
instead of as an isolated five-language edit here; see note at the end of this section).

- [x] `schema/machine_config_v1.schema.json` — added all 23 new properties under the existing
      `client`/`pipe`/trigger sub-schemas, plus `trigger_stop_ceiling_layers` at the `opcua` level.
      All added as `["type", "null"]`, none added to any `required` array — consistent with every
      field being `Option<T>` in the model and "required" being a facade-only concept (see "Why
      facade-only enforcement" above).
      **Verified:** `jsonschema.Draft202012Validator.check_schema(...)` confirms the file is
      itself a valid schema; `jsonschema.validate(...)` confirms the *existing*
      `reference_config_opcua.h5` output (pre-new-field, via `MachineConfigReader(...).parse()`
      → `to_json()`) still validates against the updated schema — the additions are additive only,
      nothing existing broke.

      **Detour, resolved:** editing only the canonical file broke
      `python/tests/test_schema.py::test_bundled_schema_matches_canonical` — Python and Node.js
      each bundle a copy of this file inside their own package
      (`python/src/machine_config/machine_config_v1.schema.json`,
      `nodejs/schema/machine_config_v1.schema.json`), needed because a pip/npm-installed package
      has no access to the monorepo's `schema/` directory at runtime. Confirmed this duplication is
      *not* a general 5-language pattern and not a pre-refactor artifact: Rust and Go have no schema
      validation feature at all (checked only centrally via `tools/cross_check.py`, which reads the
      canonical file directly); C++'s `schema.hpp` takes `SCHEMA_DIR` as a build-time path and reads
      the canonical file at runtime, no embedded copy. Only Python/Node duplicate the bytes, purely
      as a consequence of being installable outside the monorepo. Also confirmed this is orthogonal
      to `File_Version`/`capabilities/v1_X/` versioning: `SCHEMA_VERSION` (`schema.py`) is bumped
      "only for breaking output changes" to the StableModel's JSON shape, independent of how many
      on-disk `File_Version`s exist — a new `File_Version` needs its own `capabilities/v1_X/`
      adapter per language, not necessarily a new schema file.

      The two bundles aren't actually symmetric, discovered while investigating: Node's copy
      (`nodejs/schema/`) is git-**ignored** and regenerated by `scripts/copy-schema.mjs` via the
      `prebuild` npm script, which CI always runs (`npm run build`) before `npm test` — so it
      structurally cannot go stale in CI, only in a local dev shell that runs `vitest` without
      `npm run build` first. Python's copy is a plain git-**tracked** file with no regeneration
      step at all — it only stays correct because a human edits it and
      `test_bundled_schema_matches_canonical` catches the times they forget (exactly what happened
      here). Fixed by copying canonical → both bundles, and adding a Node.js test
      (`nodejs/tests/schema.test.ts`, new `bundled schema matches canonical` case) mirroring
      Python's — a real gap for the local-dev-without-build case, though a narrower one than
      Python's given Node's build-time regeneration already covers CI. Re-ran both full suites
      afterward: 315/315 Python, 152/152 Node.js. Python's *lack* of an equivalent build-time
      regeneration step (vs. Node's `prebuild`) is a legitimate follow-up but is its own,
      separate concern — not pursued here since it's outside this plan's scope.
- [x] `fixtures/reference_config_opcua.h5` — added `Trigger_Stop_Ceiling_Layers = 3` (int) to the
      `OPCUA/Triggers` group, alongside the existing `Triggers_Enabled`.
      **Verified:** re-opened the file fresh after writing (not relying on the same handle) and
      dumped `OPCUA/Triggers`' attrs and both trigger subgroups' attrs — the new attribute is
      present with the expected value, both `Chamber Oxygen Level` and `Laser Emission Interlock`
      are byte-for-byte unchanged. Also confirmed today's Python reader still parses the file
      without error (the new attribute has nowhere typed to land yet, so it's silently absorbed —
      `OpcuaConfig` has no `extra` bucket, and this attribute isn't in any known-keys list, so it
      simply isn't surfaced anywhere; this is expected and matches every other language's
      identical "unknown attribute → ignored, not an error" behavior for group-level attrs outside
      `Client`/`Pipe`/`Trigger`'s known-keys mechanism).
- [x] New shared fixture `docs/validation/fixtures/opcua_missing_required.h5` — added a
      `generate_opcua_missing_required()` step to `docs/validation/fixtures/generate_fixtures.py`
      (new `--opcua-source` CLI flag, off by default so the script's existing non-OPCUA callers are
      unaffected) that copies `reference_config_opcua.h5` (**after** the step above) and deletes
      all seven required attributes:
      - `OPCUA/Client`: `Machine_Profile`, `Root_Node`
      - `OPCUA/Pipe`: `Configure_Client`, `Pipe_Name`
      - `OPCUA/Triggers/Laser Emission Interlock`: `Event` — deleted from **one** trigger only,
        deliberately, so the test can also confirm the *other* trigger (which still has `Event`)
        is not flagged.
      - `OPCUA/Triggers` (group level): `Trigger_Stop_Ceiling_Layers` **and** `Triggers_Enabled`
      **Verified:** a full attribute-tree diff (every group/dataset, every attribute, via
      `h5py.File.visititems`) between the new fixture and `reference_config_opcua.h5` shows zero
      added/removed groups or datasets, and *exactly* the seven listed attributes removed with no
      other attribute changed anywhere — confirmed `Chamber Oxygen Level`'s `Event` is untouched
      (`'SensorEvents'`) while `Laser Emission Interlock`'s is gone. Also confirmed today's Python
      reader parses this fixture cleanly (permissive by design — no error), with
      `triggers_enabled` reading back `None` and the per-trigger `Event` asymmetry visible exactly
      as intended, proving Phase 2 will have real, correctly-shaped data to validate against.
      Regenerating also touched the other seven pre-existing AV fixtures with incidental HDF5
      binary/timestamp differences (no content change) — reverted those via `git checkout` so only
      `generate_fixtures.py` and the new fixture are part of this change.
- [ ] `CapabilityError` (shared shape) — add `details: Option<Vec<String>>` / `list[str] | None` /
      `string[] | null` / `[]string` (Go) / `std::optional<std::vector<std::string>>` (C++),
      default absent for every existing error path. Optional cleanup while in this struct: give
      C++'s `code` a closed type instead of a bare `std::string`, matching the other four.
      **Deferred to each language's Phase 2 step**, not done here — this is the one Phase 0 item
      that's per-language in practice (five separate edits, not one shared artifact), so it's more
      natural to do it in the same pass as that language's `.opcua()` validation-check work rather
      than as an isolated round-trip through all five languages' error types now.

---

## Phase 1 — per-language model / reader / writer (ready now, not blocked)

### Rust

**Status: done and verified, 2026-08-19.**

- [x] `rust/src/models.rs` — added the 10/6/6/1 fields to `OpcuaClientConfig`/`OpcuaPipeConfig`/
      `OpcuaTrigger`/`OpcuaConfig`. Matched the existing sibling-field convention exactly: plain
      `Option<T>`, no `skip_serializing_if`/`#[serde(default)]` (none of the pre-existing Option
      fields on these four structs use either).
- [x] `rust/src/capabilities/v1_0/hdf5.rs` — one `read_str`/`read_int`/`read_bool_from_int` call
      per field; all 22 promoted names added to `KNOWN_CLIENT_KEYS`/`KNOWN_PIPE_KEYS`/
      `KNOWN_TRIGGER_KEYS`; `Trigger_Stop_Ceiling_Layers` read alongside `Triggers_Enabled`.
- [x] `rust/src/capabilities/v1_0/writer.rs` — one `ws`/`wi`/`wb` call per field, all existing
      helpers, no new ones added.
- [x] **Fixed existing test** — `hdf5.rs`'s OPCUA trigger test asserted
      `trigger.extra.contains_key("Trigger_Label")`; changed to assert
      `trigger.trigger_label == Some("Laser Emission Interlock".into())` **and**
      `!trigger.extra.contains_key("Trigger_Label")`.
      **Found and fixed the same bug a second time:** the plan only named this one test, but
      `rust/src/reader.rs` (the plain `MachineConfigReader` public facade) had a byte-for-byte
      duplicate of the same test with the same stale assertion — caught immediately by running
      the OPCUA test group before touching anything else, not by re-reading the plan. Fixed
      identically.
- [x] **Updated existing test** — `models.rs`'s `present_opcua_serialises_as_object` now
      hand-builds all 23 new fields with real, distinct, non-`None` values (taken from the same
      values verified against the fixture in Phase 0), so `assert_eq!(config, roundtripped)`
      actually exercises the new code paths; added explicit spot-check assertions on a few of them
      post-roundtrip too.
- [x] **New tests**, all in `capabilities/v1_0/hdf5.rs` / `capabilities/v1_0/writer.rs` (the
      authoritative copy — where `KNOWN_*_KEYS` and the parsing/writing logic actually live):
  - [x] `opcua_fixture_promoted_fields_have_real_values` — every one of the 22 promoted fields
        checked against `reference_config_opcua.h5`'s real values (re-verified via h5py before
        writing), plus `trigger_stop_ceiling_layers`, plus an explicit loop over every
        `KNOWN_*_KEYS` constant asserting none of them still land in `extra`.
  - [x] `opcua_missing_required_fixture_parses_gracefully` — parses `opcua_missing_required.h5`
        without error; all seven deliberately-removed fields read back `None`; the per-trigger
        `Event` asymmetry confirmed (`Laser Emission Interlock` → `None`, `Chamber Oxygen Level` →
        still `Some("SensorEvents")`); a few untouched fields spot-checked to confirm the rest of
        the file is unaffected.
  - [x] `roundtrip_trigger_stop_ceiling_layers_none_and_some` — dedicated test covering both the
        `Some(3)` case (real fixture) and the `None` case (mutated in memory, re-written, re-read)
        through a temp file — the one field with no `extra` bucket to fall back on if the
        write/read pairing were mismatched.
  - [x] Also extended the pre-existing `roundtrip_opcua_fields` test (both the
        `capabilities::v1_0::writer` copy and the plain `writer.rs`/`reader.rs` facade copies,
        which the plan didn't call out individually but which needed the same treatment for
        consistency) with equality assertions on all 22 promoted fields across a real write→read
        cycle.

**Verification:** `cargo check --lib --tests` clean, zero warnings from `cargo build`. Full
`cargo test` run: 77 lib unit tests + 5 + 8 + 30 integration tests (`tests/*.rs`) + 2 doc-tests =
122 tests, **0 failures**. The 13 OPCUA-specific tests across all five files (`models.rs`,
`reader.rs`, `writer.rs`, `capabilities/v1_0/hdf5.rs`, `capabilities/v1_0/writer.rs`) individually
confirmed passing before the full-suite run.

### Python

**Status: done and verified, 2026-08-19.**

- [x] `python/src/machine_config/models.py` — added all 23 fields (`Optional[T] = None`) to
      `OpcuaClientConfig`/`OpcuaPipeConfig`/`OpcuaTrigger`/`OpcuaConfig`, placed after each
      class's existing non-default fields and before `extra` (dataclass field-ordering rule:
      defaults must follow non-defaults — all 23 already have `= None`, so no ordering conflict).
- [x] `python/src/machine_config/capabilities/v1_0/hdf5.py` — added `self._read_str`/`_read_int`/
      `_read_bool_from_int` calls per field in `_parse_opcua`; added all 22 promoted names to
      `_KNOWN_CLIENT_KEYS`/`_KNOWN_PIPE_KEYS`/`_KNOWN_TRIGGER_KEYS`; read
      `Trigger_Stop_Ceiling_Layers` alongside `Triggers_Enabled`.
- [x] `python/src/machine_config/capabilities/v1_0/writer.py` — added `self._s`/`_i`/`_b` calls
      per field in `_write_opcua`, all existing helpers, no new ones added.
- [x] **Found two additional construction/serialisation sites the plan didn't name** — Python
      hand-writes its JSON round-trip (no serde-equivalent auto-derive), so beyond the three files
      the plan called out, `hdf5.py` also has:
  - `_opcua_to_dict` (the `to_json()` path) — needed all 23 fields added to its dict output.
  - `_opcua_from_dict` (module-level, the `config_from_dict()`/`from_json()` path) — needed all
    23 fields added to its dict-to-dataclass reconstruction.

    Missing either one would have made `.parse()` return the new fields correctly while
    `to_json()`/`config_from_dict()` silently dropped them — found by grepping every
    `OpcuaClientConfig(`/`OpcuaPipeConfig(`/`OpcuaTrigger(`/`OpcuaConfig(` construction site
    across `python/src/` up front, the same discipline that caught Rust's duplicate
    `reader.rs`/`writer.rs` tests, rather than trusting the plan's file list as exhaustive.
- [x] **The plan's "no existing breaking test" claim was wrong** — found and fixed a real
      breaking test: `python/tests/test_opcua_roundtrip.py::test_roundtrip_json_opcua` hand-built
      an `OpcuaTrigger` with `extra={"Event": "SensorEvents"}` and asserted
      `t.extra["Event"] == "SensorEvents"` after a full write→read→`to_json()`→`config_from_dict()`
      cycle. Now that `Event` is a typed field, it no longer lands in `extra` — the assertion
      raised `KeyError: 'Event'`, caught by running the file before touching anything else, not
      by trusting the plan's grep. Fixed to construct via `event="SensorEvents"` and assert
      `t.event == "SensorEvents"` **and** `"Event" not in t.extra`.
- [x] **New tests**, all in `python/tests/test_opcua_roundtrip.py` (added an
      `opcua_missing_required_reader` fixture to `conftest.py` alongside the existing
      `opcua_reader`):
  - [x] `test_promoted_fields_have_real_values` — every one of the 22 promoted fields checked
        against `reference_config_opcua.h5`'s real values, plus `trigger_stop_ceiling_layers`,
        plus explicit `extra == {}` assertions on both structs and both triggers.
  - [x] `test_missing_required_fixture_parses_gracefully` — parses `opcua_missing_required.h5`
        without error; all seven deliberately-removed fields read back `None`; the per-trigger
        `Event` asymmetry confirmed (`Laser Emission Interlock` → `None`,
        `Chamber Oxygen Level` → still `"SensorEvents"`); a few untouched fields spot-checked.
  - [x] `test_roundtrip_trigger_stop_ceiling_layers_none_and_some` — both the `3` case (real
        fixture) and the `None` case (mutated in memory, re-written, re-read) through a temp file.

**Verification:** full `pytest python/tests/` run: **318 passed, 0 failed** (up from 315 — one
existing test fixed, three new tests added), zero warnings. The 13 OPCUA-specific tests in
`test_opcua_roundtrip.py` individually confirmed passing before the full-suite run, along with
standalone smoke tests of the reader, the JSON to-dict/from-dict cycle, and a real write→read
cycle against the fixture.

### Node.js

**Status: done and verified, 2026-08-19.**

- [x] `nodejs/src/models.ts` — added all 23 fields (`field: T | null`) to
      `OpcuaClientConfig`/`OpcuaPipeConfig`/`OpcuaTrigger`/`OpcuaConfig`. None of these interfaces
      use `?:` optional-property syntax (they use `T | null` on required properties), so — same as
      Rust's compiler-enforced discipline — every existing object-literal construction site missing
      one of the new required properties became a real `tsc` error (`TS2740`/`TS2741`) the moment
      the interfaces changed, confirming every site was found rather than assumed.
- [x] `nodejs/src/capabilities/v1_0/hdf5.ts` — added `attrStr`/`attrInt`/`attrBool` calls per
      field in `parseOpcua`; added all 22 promoted names to `KNOWN_CLIENT`/`KNOWN_PIPE`/
      `KNOWN_TRIGGER`; read `Trigger_Stop_Ceiling_Layers` alongside `Triggers_Enabled`.
- [x] `nodejs/src/capabilities/v1_0/writer.ts` — added `ws`/`wi`/`wb` calls per field in
      `writeOpcua`, all existing helpers, no new ones added.
- [x] **Confirmed no hidden third site, unlike Python** — Python hand-writes a JSON
      to-dict/from-dict layer that needed separate updates (see Python's section above); Node.js's
      equivalent (`asJson()` in `capabilities/v1_0/file.ts`) is a pure type-cast
      (`config as unknown as Json`), not a field-by-field mapper, so it picks up new `MachineConfig`
      fields automatically. Verified by reading it directly rather than assuming parity with
      Python's architecture.
- [x] **No existing breaking test found** — this claim, unlike Python's, held up: grepped
      `nodejs/tests/` for every promoted on-disk attribute name and for `.extra[` lookups; every
      OPCUA test reads properties off an already-parsed fixture rather than hand-constructing
      `OpcuaTrigger`/`OpcuaClientConfig`/etc. object literals with `extra: {...}` the way Python's
      broken test did, so none could have this failure mode.
- [x] **New tests**, all in `nodejs/tests/reader.test.ts` and `nodejs/tests/writer.test.ts` (the
      existing homes of the OPCUA `describe` blocks, extended alongside them; added an
      `OPCUA_MISSING_REQUIRED` fixture path + parsed config to `reader.test.ts`'s shared
      `beforeAll`):
  - [x] `reader.test.ts` — every one of the 22 promoted fields checked against
        `reference_config_opcua.h5`'s real values across 4 new tests (client, pipe,
        `trigger_stop_ceiling_layers`, both triggers), each also asserting `extra` is `{}`.
  - [x] `reader.test.ts` — 3 new tests parsing `opcua_missing_required.h5`: all seven
        deliberately-removed fields read back `null`; the per-trigger `event` asymmetry confirmed
        (`Laser Emission Interlock` → `null`, `Chamber Oxygen Level` → still `'SensorEvents'`); a
        few untouched fields spot-checked.
  - [x] `writer.test.ts` — extended the existing OPC-UA roundtrip `describe` block with 4 new
        tests covering all 22 promoted fields surviving a real write→read cycle, plus a dedicated
        `describe` block round-tripping `trigger_stop_ceiling_layers` both as `3` (real fixture)
        and as `null` (built via an object spread that clears it, then re-written/re-read).

**Verification:** `npx tsc --noEmit` clean across the whole project (src + tests); `npm run build`
clean, zero warnings. Full `npx vitest run`: **166 passed** (up from 152 — 14 new tests, 0 fixed
since nothing was broken), 0 failures. `reader.test.ts` and `writer.test.ts` individually confirmed
passing before the full-suite run, along with a standalone smoke test of the built reader against
the real fixture.

### Go

**Status: done and verified, 2026-08-19.**

- [x] `go/internal/models/models.go` — added all 23 fields as pointer types (`*int`, `*string`,
      `*bool`) to `OpcuaClientConfig`/`OpcuaPipeConfig`/`OpcuaTrigger`/`OpcuaConfig`.
- [x] `go/capabilities/v1_0/hdf5/hdf5.go` — added `readStrAttr`/`readIntAttr`/`readBoolFromIntAttr`
      calls per field in `parseOpcua`; added all 22 promoted names to the inline
      `clientKnownKeys`/`pipeKnownKeys`/`triggerKnownKeys` maps; read
      `Trigger_Stop_Ceiling_Layers` alongside `Triggers_Enabled`.
- [x] `go/capabilities/v1_0/hdf5/writer.go` — added `ws`/`wi`/`wb` calls per field in
      `writeOpcua` (via the existing `strOrEmpty` helper for the `*string` fields, since `ws`
      takes a plain `string`, not a pointer — the same pattern already used for `ID`/`Signal`/
      etc.), all existing helpers, no new ones added.
- [x] **Confirmed no hidden third site** — Go's capabilities facade (`GetOpcua()` in
      `capabilities/v1_0/file.go`) returns the typed `OpcuaConfig` struct directly via
      `api.Snapshot()` (a deep-copy), the same shape as Rust/Node.js, not Python's hand-written
      JSON-dict layer — confirmed by reading `file.go` directly rather than assuming architectural
      parity with any one other language.
- [x] **No existing breaking test found** — grepped `go/` for every promoted on-disk attribute
      name and for `.Extra[` map lookups; no test constructs an `OpcuaTrigger`/etc. literal with a
      soon-to-be-promoted key inside `Extra`, so none could have Python's failure mode. Ran the
      three pre-existing OPCUA tests (`TestOpcuaAbsentAndPresent`, `TestOpcuaDetailedFields`,
      `TestWriterRoundtripOPCUA`) before touching anything else to confirm.
- [x] **New tests**, added to the existing homes in `go/reader_test.go` and `go/writer_test.go`:
  - [x] Extended `TestOpcuaDetailedFields` with every one of the 22 promoted fields checked
        against `reference_config_opcua.h5`'s real values (client, pipe, both triggers), each also
        asserting `Extra` is empty.
  - [x] `TestOpcuaMissingRequiredFixtureParsesGracefully` (new) — parses
        `opcua_missing_required.h5` (via a new `validationFixturesDir(t)` helper alongside the
        existing `fixturesDir(t)`) without error; all seven deliberately-removed fields read back
        `nil`; the per-trigger `Event` asymmetry confirmed; a few untouched fields spot-checked.
  - [x] Extended `TestWriterRoundtripOPCUA` with equality assertions on all 22 promoted fields
        across a real write→read cycle.
  - [x] `TestWriterRoundtripTriggerStopCeilingLayersNilAndSome` (new) — both the `3` case (real
        fixture, via the existing `roundtrip(t, src)` helper) and the `nil` case (mutated in
        memory, re-written, re-read) through a temp file.

**Verification:** built and tested with `CGO_ENABLED=1` and MSYS2 MinGW64's `gcc`/`g++` on PATH
(the same toolchain `go.yml`'s Windows CI leg uses for HDF5 bindings). `go build ./...` and
`go vet ./...` both clean. Full `go test ./... -v -count=1`: **56 passed, 0 failed** across both
packages (`machine-config-go` and `machine-config-go/capabilities`). The OPCUA-specific tests
individually confirmed passing before the full-suite run.

### C++

**Status: done and verified, 2026-08-20.**

- [x] `cpp/include/machine_config/models.hpp` — added all 23 fields as `std::optional<T>` to
      `OpcuaClientConfig`/`OpcuaPipeConfig`/`OpcuaTrigger`/`OpcuaConfig`. Unlike Rust/Node.js's
      compiler-enforced struct literals, C++ default-constructs missing members (`std::optional`
      defaults to empty) — adding fields can never cause a compile error at any existing
      construction site, so the "compiler catches every site" discipline used for those two
      languages doesn't apply here; every site had to be found manually (see below).
- [x] **Found a hidden third site, exactly like Python's** — `models.hpp` also hand-writes
      `to_json`/`from_json` overloads for all four OPCUA structs (nlohmann::json's ADL-based
      serialization, not an auto-derive), analogous to Python's `_opcua_to_dict`/`_opcua_from_dict`.
      Found by grepping for `to_json(nlohmann::json&, const Opcua` up front, the same discipline
      that caught Python's and Rust's respective surprises, rather than trusting the plan's
      three-file list as exhaustive. Updated all 8 functions (`to_json`/`from_json` × 4 structs)
      with all 23 fields, using the existing `detail::opt_to_j`/`detail::j_to_opt<T>` templates —
      already generic over any `T`, so no new serialization helpers were needed. Followed the
      pre-existing alphabetical key ordering convention within each `to_json`'s initializer list.
- [x] `cpp/include/machine_config/capabilities/v1_0/hdf5.hpp` — added `readStr`/`readInt`/
      `readBoolFromInt` calls per field in `parseOpcua`; added all 22 promoted names to the inline
      initializer-lists passed to `collectExtra(...)`; read `Trigger_Stop_Ceiling_Layers`
      alongside `Triggers_Enabled`.
- [x] `cpp/include/machine_config/capabilities/v1_0/writer.hpp` — added `ws`/`wi`/`wb` calls per
      field in `writeOpcua`, all existing helpers, no new ones added.
- [x] **No existing breaking test found** — grepped `cpp/tests/` for every promoted on-disk
      attribute name and for `.extra` lookups referencing them; the one test that hand-builds an
      `OpcuaConfig`/`OpcuaTrigger` in memory (`RoundtripOpcuaFromScratch`) does so via field
      assignment, not an aggregate literal, and never touches `extra` for a soon-to-be-promoted
      key, so it couldn't have Python's failure mode. Ran the 12 pre-existing OPCUA-related test
      cases before touching anything else to confirm.
- [x] **New tests**, added to `cpp/tests/test_reader.cpp` and `cpp/tests/test_writer.cpp` (the
      existing homes of the OPCUA test cases, extended alongside them):
  - [x] `OpcuaPromotedFieldsHaveRealValues` (new, in `test_reader.cpp`) — every one of the 22
        promoted fields checked against `reference_config_opcua.h5`'s real values (client, pipe,
        both triggers), each also asserting `extra` is empty.
  - [x] `OpcuaMissingRequiredFixtureParsesGracefully` (new, in `test_reader.cpp`) — parses
        `opcua_missing_required.h5` without error; all seven deliberately-removed fields read
        back `nullopt`; the per-trigger `event` asymmetry confirmed; a few untouched fields
        spot-checked. Required adding a new `VALIDATION_FIXTURES_DIR` compile definition to
        `cpp/tests/CMakeLists.txt` (mirroring the existing `FIXTURES_DIR`/`SCHEMA_DIR`) since
        `cpp/tests/` had never before reached outside `fixtures/`/`schema/` into
        `docs/validation/fixtures/` — every other language already had this path available via
        their own test-fixture conventions.
  - [x] Extended `RoundtripWithOpcua` with equality assertions on all 22 promoted fields across a
        real write→read cycle.
  - [x] `RoundtripTriggerStopCeilingLayersNilAndSome` (new, in `test_writer.cpp`) — both the `3`
        case (real fixture) and the `nullopt` case (mutated in memory, re-written, re-read)
        through a temp file.

**Verification:** reconfigured CMake (`cmake -S cpp -B cpp/build`, required for the new
`VALIDATION_FIXTURES_DIR` definition to take effect) and rebuilt with MSVC 19.43
(`cmake --build cpp/build --config Debug --target machine_config_tests`), clean. Full test binary
run: **425 assertions in 82 test cases, 0 failures**. The 15 OPCUA-related test cases (135
assertions) individually confirmed passing before the full-suite run.

---

## Phase 2 — facade validation + structured error

For each language, extend the existing `.opcua()`/`get_opcua()`/`getOpcua()` accessor:

1. Confirm OPCUA is present (existing `NotPresent` check, unchanged).
2. Check the 6 single-instance required fields directly: `client.machine_profile`,
   `client.root_node`, `pipe.configure_client`, `pipe.pipe_name`, and the two `OpcuaConfig`-level
   fields `triggers_enabled` and `trigger_stop_ceiling_layers`. Collect the names of any that are
   `None`/`null`/`nullopt`. (`triggers_enabled` already exists and is already `Option<bool>` in
   every language today — Phase 1 doesn't touch it, only this check does.)
3. **`event` is different — it's per-trigger, not single-instance.** Iterate every entry in
   `triggers`; for each one whose `event` is absent, add an identifying entry to the missing list
   (e.g. `"<trigger name>.Event"`, not just `"Event"` — the consumer needs to know *which*
   trigger is incomplete, not just that one is). This is the one required field that needs a loop
   instead of a single check, and it's the reason the `opcua_missing_required.h5` fixture
   deliberately removes `Event` from only one of the two triggers.
4. If the combined list from steps 2–3 is non-empty, return
   `Err(CapabilityError { code: ValidationError, message: "...", details: Some([...]) })` instead
   of `Ok(...)` — every missing field reported together, not fail-fast on the first one.

**Tests per language (template — see the Rust subsection below for the completed reference):**
- [ ] `.opcua()` → `Ok` on `reference_config_opcua.h5` (all required fields present, on both
      triggers).
- [ ] `.opcua()` → `Err(ValidationError)` on `opcua_missing_required.h5`, `details` names
      *exactly* the seven missing items — including the trigger-qualified `Event` entry for
      `Laser Emission Interlock` only, **not** for `Chamber Oxygen Level` (which still has it).
- [ ] An optional-but-absent field (e.g. `keep_alive_count`) must **not** appear in `details` —
      dedicated assertion, since it's the check most likely to silently regress if the
      required-list logic inverts by mistake.

### Rust

**Status: done and verified, 2026-08-19.**

- [x] `rust/src/capabilities/errors.rs` — `CapabilityError::ValidationError` changed from a
      single-`String` tuple variant to a struct variant:
      `ValidationError { message: String, details: Option<Vec<String>> }`. Added a
      `CapabilityError::validation_error(message)` convenience constructor for the (still common)
      no-details case, so the four pre-existing construction sites didn't have to spell out
      `details: None` by hand. **All four pre-existing call sites updated**
      (`capabilities/merge.rs` ×3, `capabilities/v1_0/file.rs`'s `save()` ×1) — the compiler caught
      every one via `E0533` the moment the variant's shape changed, confirming nothing was missed.
      `Display` updated to match the new shape (`ValidationError { message, .. } => write!(f,
      "{message}")`).
- [x] `rust/src/capabilities/v1_0/file.rs`'s `get_opcua()` — implemented exactly the 4-step
      mechanism above: the 6 single-instance checks, the per-trigger `Event` loop producing
      `"<trigger name>.Event"`-qualified entries, and the combined-list `ValidationError` with
      `details` populated only when non-empty. Missing-field names use the on-disk HDF5 attribute
      spelling (`"Machine_Profile"`, not `"machine_profile"`) — matches the plan's own
      `"<trigger name>.Event"` example casing and gives a consumer a name they can look up directly
      against the raw file. `set_opcua()` was left untouched — the plan scopes this check to the
      read accessor only.
- [x] **New tests**, all in `rust/tests/capabilities_test.rs` (the existing home of the
      `opcua_not_present_vs_present` facade test, extended alongside them):
  - [x] `opcua_required_fields_present_on_reference_fixture` — `.get_opcua()` → `Ok` on
        `reference_config_opcua.h5`, plus spot-checks on two of the fields the check depends on.
  - [x] `opcua_missing_required_fields_reports_all_seven_at_once` — `.get_opcua()` →
        `Err(ValidationError)` on `opcua_missing_required.h5`; `details` (order-independent,
        compared via sorted `Vec`) names exactly the seven expected items, including
        `"Laser Emission Interlock.Event"` and explicitly **not** any `"Chamber Oxygen Level"`
        entry.
  - [x] `opcua_optional_field_never_appears_in_missing_details` — found the plan's suggested check
        (`keep_alive_count` absent from `details` on the missing-required fixture) would have been
        a tautology, since that fixture never removes any optional field, so absence from
        `details` would trivially hold either way. Made it a real test instead: parsed the
        missing-required fixture, cleared `keep_alive_count` in memory too (now both a required
        *and* an optional field are genuinely absent), re-wrote it to a temp file via
        `MachineConfigWriter`, and confirmed `details` still names exactly the same 7 items — not
        8 — proving the optional field's absence truly never reaches `details`.

**Verification:** `cargo check --lib --tests` clean, zero warnings from `cargo build`. Full
`cargo test`: 77 lib unit tests + 11 capabilities-facade tests (was 8; +3 new) + 5 + 30 other
integration tests + 2 doc-tests = **125 tests, 0 failures**. All 4 OPCUA-facade tests in
`capabilities_test.rs` individually confirmed passing before the full-suite run.

### Python

**Status: done and verified, 2026-08-19.**

- [x] `python/src/machine_config/capabilities/errors.py` — added `details: Optional[list[str]] =
      None` to the `CapabilityError` dataclass, and a matching optional `details` parameter to the
      `capability_error(code, message, details=None)` helper. Every existing call goes through
      that helper (confirmed via grep — no call site constructs `CapabilityError(...)` directly
      outside `errors.py` itself), so all pre-existing call sites needed zero changes — the new
      parameter's default kept them working unchanged.
- [x] `python/src/machine_config/capabilities/v1_0/file.py`'s `opcua()` — implemented the same
      4-step mechanism as Rust, adapted to Python's facade architecture: unlike Rust's `OpcuaConfig`
      dataclass, Python's facade operates on the **dict** representation produced by
      `_config_to_dict()` (`self._data["opcua"]`), not a parsed dataclass — so the check reads
      `opcua_data["client"].get("machine_profile")` etc. rather than attribute access, but is
      otherwise identical: 6 single-instance checks, the per-trigger `event` loop producing
      `f"{name}.Event"`-qualified entries, and a combined-list `ValidationError` with `details`
      populated only when non-empty. Missing-field names use the same on-disk HDF5 attribute
      spelling as Rust (`"Machine_Profile"`), for cross-language consistency. `set_opcua`-equivalent
      mutation (via `_NodeHandle.set_model`) was left untouched, matching Rust's scoping decision.
- [x] **New tests**, all in `python/tests/test_capabilities.py` (the existing home of
      `test_opcua_not_present_vs_present`, extended alongside them):
  - [x] `test_opcua_required_fields_present_on_reference_fixture` — `.opcua()` → `Ok` on
        `reference_config_opcua.h5`, plus spot-checks on two of the fields the check depends on.
  - [x] `test_opcua_missing_required_fields_reports_all_seven_at_once` — `.opcua()` →
        `Err(ValidationError)` on `opcua_missing_required.h5`; `details` (compared as a `set`, so
        order-independent) equals exactly the seven expected items, including
        `"Laser Emission Interlock.Event"` and explicitly **not** any `"Chamber Oxygen Level"`
        entry.
  - [x] `test_opcua_optional_field_never_appears_in_missing_details` — same non-tautology fix
        applied as Rust: `opcua_missing_required.h5` never removes any optional field, so checking
        an optional field's absence from `details` there would trivially pass either way. Instead
        parsed the fixture with the low-level `MachineConfigReader`, cleared `keep_alive_count` in
        memory too, re-wrote it via `MachineConfigWriter` to a temp file, and confirmed `details`
        still names exactly the same 7 items on the re-opened facade — not 8.

**Verification:** full `pytest python/tests/` run: **321 passed, 0 failed** (up from 318), zero
warnings. All 3 new OPCUA-facade tests plus the pre-existing `test_opcua_not_present_vs_present`
individually confirmed passing before the full-suite run; a standalone smoke test against both
fixtures (`reference_config_opcua.h5` → `Ok`, `opcua_missing_required.h5` → `Err` naming exactly
the seven expected fields) was also run directly before the pytest pass.

*(Node.js's Phase 2 section was originally drafted here but misfiled under Phase 1 above during
that pass — moved to its correct place after this section, in language-completion order:
Rust → Python → Node.js → Go.)*

### Node.js

**Status: done and verified, 2026-08-19.**

- [x] `nodejs/src/capabilities/errors.ts` — added an optional `details?: string[]` field to the
      `CapabilityError` interface, and a matching optional `details` parameter to the
      `capabilityError(code, message, details?)` helper. Every existing call goes through that
      helper (confirmed via grep — no call site constructs a `CapabilityError` object literal
      directly outside `errors.ts` itself), so all pre-existing call sites needed zero changes.
- [x] `nodejs/src/capabilities/v1_0/file.ts`'s `opcua()` — implemented the same 4-step mechanism
      as Rust/Python: 6 single-instance checks (reading `this.data['opcua']` cast to the existing
      `OpcuaModel` type for typed dot-access, the same dict-like-data architecture Python uses —
      confirmed by reading `asJson()`/`this.data` directly rather than assuming), the per-trigger
      `event` loop producing `` `${name}.Event` `` entries, and a combined-list `ValidationError`
      with `details` populated only when non-empty. Missing-field names use the same on-disk HDF5
      attribute spelling as Rust/Python. Renamed a local variable to `current` (not `model`) to
      avoid shadowing the pre-existing `setModel(model, ...)` parameter name in the same method
      scope. `setModel`-equivalent mutation was left untouched, matching the other languages'
      scoping decision.
- [x] **New tests**, all in `nodejs/tests/capabilities.test.ts` (the existing home of the
      `'opcua NotPresent vs present'` test, extended alongside them; added a
      `FIXTURE_OPCUA_MISSING_REQUIRED` path constant and imported `MachineConfigWriter`):
  - [x] `'opcua() Ok when all required fields present on reference_opcua fixture'` — plus
        spot-checks on two of the fields the check depends on.
  - [x] `'opcua() reports all seven missing required fields at once'` — `details` (compared as a
        `Set`, order-independent) equals exactly the seven expected items, including
        `'Laser Emission Interlock.Event'` and explicitly **not** any `'Chamber Oxygen Level'`
        entry.
  - [x] `'opcua() never reports an optional field, even when genuinely absent'` — same
        non-tautology fix applied as Rust/Python: `opcua_missing_required.h5` never removes any
        optional field, so parsed it with the low-level `MachineConfigReader`, cleared
        `keep_alive_count` in memory too, re-wrote it via `MachineConfigWriter` to a temp file
        (cleaned up in a `finally` block, matching this file's existing `mkdtempSync`/`rmSync`
        convention), and confirmed `details` still names exactly the same 7 items on the re-opened
        facade — not 8.

**Verification:** `npx tsc --noEmit` clean; `npm run build` clean, zero warnings. Full
`npx vitest run`: **169 passed** (up from 166), 0 failures. `capabilities.test.ts` individually
confirmed passing (11/11) before the full-suite run; a standalone smoke test against both fixtures
(`reference_config_opcua.h5` → `ok: true`, `opcua_missing_required.h5` → `ok: false` naming
exactly the seven expected fields) was also run directly against the built `dist/` output before
the vitest pass.

### Go

**Status: done and verified, 2026-08-20.**

- [x] `go/capabilities/internal/api/api.go` — added a `Details []string` field to the `Error`
      struct, and changed `Errf(code, msg)` to `Errf(code ErrorCode, msg string, details ...string)`
      — a variadic third parameter, since Go has no default-argument syntax. Every pre-existing
      2-argument call site (confirmed via grep across `go/capabilities/`) keeps compiling unchanged;
      variadic with zero args yields a `nil` `Details` slice. `go/capabilities/types.go`'s local
      `errf` wrapper updated to match and forward the variadic.
- [x] `go/capabilities/v1_0/file.go`'s `GetOpcua()` — implemented the same 4-step mechanism as the
      other languages: reads `f.config.Opcua` directly (the typed struct, same architecture as
      Rust/Node.js — confirmed by reading `file.go` directly, not assumed), 6 single-instance
      checks, a `for name, trigger := range opcua.Triggers` loop producing `name+".Event"` entries,
      and a combined-list `ErrValidation` built via `api.Errf(api.ErrValidation, msg, missing...)`
      with `Details` populated only when non-empty. Missing-field names use the same on-disk HDF5
      attribute spelling as the other three languages.
- [x] **New tests**, added to `go/capabilities/file_test.go` (the existing home of
      `TestInvalidIndexAndOpcua`/`TestOpcuaStructuredFields`, extended alongside them; added a
      `validationFixturesDir(t)` helper alongside the existing `fixturesDir(t)`):
  - [x] `TestOpcuaRequiredFieldsPresentOnReferenceFixture` — `GetOpcua()` returns no error on
        `reference_config_opcua.h5`, plus spot-checks on two of the fields the check depends on.
  - [x] `TestOpcuaMissingRequiredFieldsReportsAllSevenAtOnce` — `GetOpcua()` returns
        `ErrValidation` on `opcua_missing_required.h5`; `Details` (compared via a lookup map, so
        order-independent) names exactly the seven expected items, including
        `"Laser Emission Interlock.Event"` and explicitly **not** any `"Chamber Oxygen Level"`
        entry.
  - [x] `TestOpcuaOptionalFieldNeverAppearsInMissingDetails` — same non-tautology fix applied as
        the other languages: `opcua_missing_required.h5` never removes any optional field, so
        parsed it with the low-level `NewReader`, cleared `Client.KeepAliveCount` in memory too,
        re-wrote it via `NewWriter` to a temp file, and confirmed `Details` still names exactly
        the same 7 items on the re-opened facade — not 8.
- [x] **Found and fixed a real bug in the new test code itself** (not the library): the optional-
      field test's first draft reused the identifier `err` across two different declarations —
      `cfg, err := machineconfig.NewReader(src).Parse()` (return type `error`, the standard
      interface) followed later by `f, err := capabilities.OpenMachineConfig(tmp)` (return type
      `*Error`, a concrete pointer). Reusing `err` let Go's mixed `:=` rules assign the concrete
      `*Error` into the already-`error`-typed variable — so a successful (nil `*Error`) open still
      produced a non-nil `error` interface value (the classic "nil pointer wrapped in a non-nil
      interface" gotcha), and the test failed with `t.Fatal(err)` printing `<nil>` instead of
      passing. Caught immediately by running the new tests before moving on, not by trusting them
      once compiled; fixed by renaming the second variable to `capErr`.

**Verification:** built and tested with the same `CGO_ENABLED=1` + MSYS2 MinGW64 `gcc`/`g++`
toolchain as Phase 1. `go build ./...` and `go vet ./...` both clean. Full
`go test ./... -v -count=1`: **59 passed, 0 failed** (up from 56). The five OPCUA-facade tests in
`go/capabilities/file_test.go` individually confirmed passing — including catching and fixing the
test-code bug above — before the full-suite run.

### C++

**Status: done and verified, 2026-08-20.**

- [x] **Correction to an earlier finding in this same pass: `CapabilityError` is not dead code —
      it was undocumented drift, and it's now fixed.** Initially found zero construction sites of
      `capabilities::CapabilityError`/`capabilityError(...)` anywhere except `errors.hpp` itself,
      and concluded it was unused/orphaned, since the real facade (`file.hpp`'s `getOpcua()`/etc.)
      returns `Result<T>` (`capabilities/result.hpp`), which stored its own private
      `code_`/`message_` strings directly rather than holding a `CapabilityError`. Before acting on
      that conclusion, re-checked `VALIDATION_PLAN.md` and found it explicitly documents
      `CapabilityError` as belonging to the stable-facade API by design (§9.x, the same section
      covering C++'s error-handling decisions) — meaning the implementation had silently drifted
      from the documented design (`Result<T>` was built with its own inline fields instead of
      using `CapabilityError` as intended), not that the struct was intentionally left as a
      decoy. Confirmed against all four other languages before fixing (`rust/src/capabilities/result.rs`'s
      `Result<T, E = CapabilityError>`, `python/.../result.py`'s `Err[E]` holding `error: E`,
      `nodejs/.../result.ts`'s `{ ok: false; error: E }`, `go/capabilities/v1_0/file.go`'s
      `(OpcuaConfig, *api.Error)`): in every one of them, the value a caller actually holds after a
      failure is **one real error object** with `.code`/`.message`/`.details` as direct fields —
      never three independent getter calls. C++ had drifted furthest from that shape.
- [x] `cpp/include/machine_config/capabilities/errors.hpp` — added
      `std::vector<std::string> details` to `CapabilityError` (plus `#include <vector>`), and a
      matching optional third parameter to `capabilityError(code, message, details = {})`.
- [x] `cpp/include/machine_config/capabilities/result.hpp` — restructured `Result<T>` and the
      `Result<void>` specialization to hold one real `CapabilityError error_` member instead of
      separate `code_`/`message_`/`details_` strings; added `const CapabilityError& error() const`
      — the accessor that actually matches the other four languages' shape — with `errorCode()`/
      `errorMessage()`/`errorDetails()` kept as delegating convenience shortcuts so every
      pre-existing call site (confirmed via grep across `cpp/include/`) keeps compiling unchanged.
      `Err(code, message, details = {})` now builds the `CapabilityError` internally.
- [x] `cpp/include/machine_config/capabilities/v1_0/file.hpp`'s `getOpcua()` — implemented the
      same 4-step mechanism as the other four languages: reads `*config_.opcua` directly (the
      typed struct, same architecture as Rust/Node.js/Go), 6 single-instance checks, a
      `for (const auto& [name, trigger] : opcua.triggers)` loop producing `name + ".Event"`
      entries, and a combined-list `Result<OpcuaConfig>::Err("ValidationError", msg, missing)`
      with `details` populated only when non-empty. Missing-field names use the same on-disk HDF5
      attribute spelling as the other languages. Added `#include <vector>` to `file.hpp`.
- [x] **Found and fixed a second, related bug**: `cpp/include/machine_config/capabilities/file.hpp`'s
      `openMachineConfig()` re-wraps `MachineConfigFileV1_0::open()`'s error into a
      `Result<shared_ptr<IMachineConfigFile>>`, and that re-wrap only forwarded `errorCode()`/
      `errorMessage()`, silently dropping `errorDetails()`. Fixed to forward all three. **Not
      reachable through any real call today** — `open()`'s only two error paths
      (`UnsupportedVersion`, `IoError`) never populate `details` — so this is disclosed as
      currently-inert insurance against a real class of bug, not something the existing test
      suite could have caught by accident, and is tested via an isolated reconstruction of the
      exact re-wrap expression (see below) rather than a misleadingly-labeled "integration" test.
- [x] **New tests**, added to `cpp/tests/test_capabilities.cpp` (the existing home of
      `CapabilityOpcuaNotPresentVsPresent`, extended alongside them; added an
      `OPCUA_MISSING_REQUIRED` path constant using the `VALIDATION_FIXTURES_DIR` macro Phase 1
      already added to `cpp/tests/CMakeLists.txt`, plus `#include "machine_config/writer.hpp"`
      and `#include <set>`):
  - [x] `CapabilityOpcuaRequiredFieldsPresentOnReferenceFixture` — `getOpcua()` returns `ok()` on
        `reference_config_opcua.h5`, plus spot-checks on two of the fields the check depends on.
  - [x] `CapabilityOpcuaMissingRequiredFieldsReportsAllSevenAtOnce` — `getOpcua()` returns
        `errorCode() == "ValidationError"` on `opcua_missing_required.h5`; `errorDetails()`
        (compared as a `std::set`, so order-independent) equals exactly the seven expected items,
        including `"Laser Emission Interlock.Event"` and explicitly **not** any
        `"Chamber Oxygen Level"` entry.
  - [x] `CapabilityOpcuaOptionalFieldNeverAppearsInMissingDetails` — same non-tautology fix
        applied as the other four languages: `opcua_missing_required.h5` never removes any
        optional field, so parsed it with the low-level `MachineConfigReader`, cleared
        `client.keep_alive_count` in memory too, re-wrote it via `MachineConfigWriter` to a temp
        file, and confirmed `errorDetails()` still names exactly the same 7 items on the re-opened
        facade — not 8.
  - [x] `CapabilityResultErrorReturnsRealCapabilityError` (new) — confirms `result.error()` is a
        genuine `CapabilityError` whose `.code`/`.message`/`.details` match `errorCode()`/
        `errorMessage()`/`errorDetails()` exactly, using the real `opcua_missing_required.h5`
        failure.
  - [x] `CapabilityErrorRewrapPreservesDetails` (new) — since no real caller can reach the
        re-wrap bug above, this manually constructs a `Result<shared_ptr<MachineConfigFileV1_0>>::Err`
        with non-empty `details`, applies the identical re-wrap expression now used in
        `openMachineConfig()`, and confirms `details` survives — a faithful test of the fixed
        line's logic, explicitly not a claim that it's exercised end-to-end today.

**Verification:** rebuilt with the same MSVC 19.43 toolchain as Phase 1, clean, after each of the
three edits (`CapabilityError`, `Result<T>`, the re-wrap) — confirming the internal restructuring
never broke any pre-existing call site before adding new tests. All 20 OPCUA/Result-related test
cases (173 assertions) confirmed passing after adding the 5 new ones. Full test binary run:
**463 assertions in 87 test cases, 0 failures** (up from 425/82 at the start of Phase 2).

---

## Validation-app coverage

Two new scenarios, added identically to all five apps' already-consolidated `scenarios.*` file
(same shape as AV-04/AV-05 — crafted fixture, no mock-adapter injection needed, so unlike
AV-09–11 these aren't blocked by any language's hardcoded-dispatcher limitation):

- [ ] **AV-12** — open `reference_config_opcua.h5` via the facade, confirm `.opcua()`/
      `get_opcua()`/`getOpcua()` returns `Ok` with the new typed fields readable.
- [ ] **AV-13** — open `opcua_missing_required.h5` via the facade, confirm it returns
      `Err(ValidationError)` with `details` naming exactly the right fields.

**Why the apps at all:** nothing today exercises the `capabilities` facade — the existing S-07
OPCUA scenario goes through the plain `MachineConfigReader`/`Writer` only. Since the required-field
check is a public-API guarantee, proving it via a real external-consumer-style app is the same
category of protection S-09 already provides for other public-surface regressions.

**In-repo ported apps only, not the original external copies.** The external copies
(`machineconfiglibrarytesting/*`) served a one-time purpose — prove the library builds for a fresh
consumer before anything was ported in — and haven't been kept in sync since (the external Go
copy, for example, was deliberately left untouched during the file-consolidation pass). Nothing
in the project depends on them staying current. The in-repo ported apps are what gets rebuilt and
reverified going forward.

- [ ] Extend the existing **S-07** scenario (all 5 languages) with assertions on a few of the
      newly-promoted fields via the plain `Reader`/`Writer`, proving the low-level round-trip
      works through the public API too, not just in unit tests.
- [ ] **S-09 does not need changes** — no new top-level type is introduced, only new fields on
      already-exported ones.

**Sequencing note:** implemented per language, immediately after that language's own Phase 1 +
Phase 2, rather than batched after all five languages finish — same "prove it end-to-end before
replicating" reasoning already used for Phase 1 → Phase 2 ordering. The one piece left to trail
behind is the documentation pass below, which is genuinely fine to defer since it only records
already-passing results.

### Rust

**Status: done and verified, 2026-08-19.**

- [x] `docs/validation/rust/app/src/scenarios.rs` — added `run_av12`/`run_av13`, wired into
      `docs/validation/rust/app/src/main.rs`'s `scenario_list` (now 19 scenarios, up from 17).
      New imports: `machine_config::capabilities::open_machine_config`,
      `machine_config::capabilities::errors::CapabilityError` — this app had never touched the
      `capabilities` module before.
  - [x] `run_av12` — opens `reference_config_opcua.h5` (the fully-populated fixture) via
        `open_machine_config`, calls `.get_opcua()`, confirms `Ok` and prints five of the
        newly-promoted/new fields to prove they're actually readable, not just present.
  - [x] `run_av13` — opens `opcua_missing_required.h5` via `open_machine_config`, confirms
        `.get_opcua()` returns `Err(CapabilityError::ValidationError { details: Some(_), .. })`,
        and asserts `details` (sorted, order-independent) equals exactly the seven expected
        entries — including `"Laser Emission Interlock.Event"` and, implicitly, the absence of
        any `"Chamber Oxygen Level"` entry (that trigger keeps `Event`, so it's never in the set
        to begin with).
- [x] **S-07 extended** (`run_s07`) — added roundtrip assertions for `machine_profile`,
      `root_node`, `pipe_name`, `trigger_stop_ceiling_layers`, and (on the existing
      `Chamber Oxygen Level` trigger check) `event`/`trigger_label`, proving the Phase 1 fields
      survive a real write→read cycle through the plain public `MachineConfigReader`/`Writer`
      API, not just via the crate's own unit tests.
- [x] S-09 unchanged, per plan (no new top-level type introduced).

**Verification:** `cargo build` in `docs/validation/rust/app/` — clean, zero warnings (the two
new imports were flagged unused until AV-12/13 were added, confirming they're both actually
exercised). Ran the built binary against the real fixture set
(`mcl_rust_validation.exe fixtures "Reference Materials"`):

```
[PASS] S-07: OPCUA roundtrip OK: 2 triggers, url="opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer", machine_profile=Some("Aconity")
[PASS] AV-12: get_opcua() Ok: machine_profile=Some("Aconity"), root_node=Some("MachineFleet"), pipe_name=Some("\\\\.\\pipe\\opc_ua_client_pipe"), triggers_enabled=Some(true), trigger_stop_ceiling_layers=Some(3)
[PASS] AV-13: ValidationError with details=["Configure_Client", "Laser Emission Interlock.Event", "Machine_Profile", "Pipe_Name", "Root_Node", "Trigger_Stop_Ceiling_Layers", "Triggers_Enabled"]

19 scenarios: 19 passed, 0 failed
```

Re-ran the full `rust/` crate's `cargo test` afterward as a regression check: still 125/125
passing, confirming the validation-app changes (a separate standalone Cargo workspace consuming
`machine-config` as a path dependency) didn't require or trigger any change to the library itself.

### Python

**Status: done and verified, 2026-08-19.**

- [x] `docs/validation/python/app/scenarios.py` — added `run_av12`/`run_av13`, wired into
      `docs/validation/python/app/main.py`'s `SCENARIOS` list (now 22 scenarios, up from 20 —
      Python's app already included AV-09–11, unlike Rust, since Python's dispatcher is a real
      registry). New imports: `machine_config.capabilities.open_machine_config` — this app had
      never touched the `capabilities` module before.
  - [x] `run_av12` — opens `reference_config_opcua.h5` via `open_machine_config`, calls
        `.opcua()`, confirms `Ok` and prints five of the newly-promoted/new fields (read from the
        returned node's `get_model()` dict) to prove they're actually readable, not just present.
  - [x] `run_av13` — opens `opcua_missing_required.h5` via `open_machine_config`, confirms
        `.opcua()` returns `Err` with `code == "ValidationError"`, and asserts `details` (compared
        as a `set`, order-independent) equals exactly the seven expected entries — including
        `"Laser Emission Interlock.Event"` and, implicitly, the absence of any
        `"Chamber Oxygen Level"` entry.
- [x] **S-07 extended** (`run_s07`) — added roundtrip assertions for `machine_profile`,
      `root_node`, `pipe_name`, `trigger_stop_ceiling_layers`, and (on the existing
      `Chamber Oxygen Level` trigger check) `event`/`trigger_label`, proving the Phase 1 fields
      survive a real write→read cycle through the plain public `MachineConfigReader`/`Writer`
      API, not just via `pytest`.
- [x] S-09 unchanged, per plan (no new top-level type introduced).

**Verification:** ran the app directly against the real fixture set (this app runs against the
editable-installed `machine_config` package, so no separate build step was needed —
`PYTHONIOENCODING=utf-8 python docs/validation/python/app/main.py fixtures "Reference Materials"`;
the `PYTHONIOENCODING` override is only needed for this Windows console's cp1252 default and is
pre-existing behavior unrelated to this change — S-02's `≠` character needs it too):

```
[PASS] S-07: OPCUA roundtrip OK: 2 triggers, url='opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer', machine_profile='Aconity'
[PASS] AV-12: opcua() Ok: machine_profile='Aconity', root_node='MachineFleet', pipe_name='\\\\.\\pipe\\opc_ua_client_pipe', triggers_enabled=True, trigger_stop_ceiling_layers=3
[PASS] AV-13: ValidationError with details=['Configure_Client', 'Laser Emission Interlock.Event', 'Machine_Profile', 'Pipe_Name', 'Root_Node', 'Trigger_Stop_Ceiling_Layers', 'Triggers_Enabled']

22 scenarios: 22 passed, 0 failed
```

Re-ran the full `pytest python/tests/` suite afterward as a regression check: still 321/321
passing, confirming the validation-app changes didn't require or trigger any change to the
library itself. Also imported `scenarios.py` under `python -W error` to confirm no warnings.

### Node.js

**Status: done and verified, 2026-08-19.**

- [x] `docs/validation/nodejs/app/scenarios.mts` — added `runAv12`/`runAv13`, wired into
      `docs/validation/nodejs/app/main.mts`'s `SCENARIOS` list (now 22 scenarios, up from 20 —
      Node's app already included AV-09–11, like Python, since Node's dispatcher is also a real
      registry, not Rust/C++'s hardcoded match). New import: `openMachineConfig` from
      `machine-config-library` (the app's `file:../../../../nodejs` dependency, resolved via a
      symlink into `nodejs/dist` — confirmed the symlink exists and rebuilt with `npm run build`
      before running the app, rather than assuming a stale `dist/` would pick up the change).
  - [x] `runAv12` — opens `reference_config_opcua.h5` via `openMachineConfig`, calls `.opcua()`,
        confirms `.ok` and prints five of the newly-promoted/new fields (read from the returned
        handle's `getModel()`) to prove they're actually readable, not just present.
  - [x] `runAv13` — opens `opcua_missing_required.h5` via `openMachineConfig`, confirms `.opcua()`
        returns a non-`ok` result with `error.code === 'ValidationError'`, and asserts `details`
        (compared as a `Set`, order-independent) equals exactly the seven expected entries —
        including `'Laser Emission Interlock.Event'` and, implicitly, the absence of any
        `'Chamber Oxygen Level'` entry.
- [x] **S-07 extended** (`runS07`) — added roundtrip assertions for `machine_profile`,
      `root_node`, `pipe_name`, `trigger_stop_ceiling_layers`, and (on the existing
      `Chamber Oxygen Level` trigger check) `event`/`trigger_label`, proving the Phase 1 fields
      survive a real write→read cycle through the plain public `MachineConfigReader`/`Writer`
      API, not just via `vitest`.
- [x] S-09 unchanged, per plan (no new top-level type introduced).

**Verification:** rebuilt `nodejs/` (`npm run build`, clean) so the app's symlinked dependency
picked up every prior change, then ran the app directly via `npx tsx main.mts` (no separate
compile step needed — `tsx` runs the `.mts` file directly):

```
[PASS] S-07: OPCUA roundtrip OK: 2 triggers, url='opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer', machine_profile='Aconity'
[PASS] AV-12: opcua() Ok: machine_profile='Aconity', root_node='MachineFleet', pipe_name='\\.\pipe\opc_ua_client_pipe', triggers_enabled=true, trigger_stop_ceiling_layers=3
[PASS] AV-13: ValidationError with details=["Configure_Client","Laser Emission Interlock.Event","Machine_Profile","Pipe_Name","Root_Node","Trigger_Stop_Ceiling_Layers","Triggers_Enabled"]

22 scenarios: 22 passed, 0 failed
```

`npx tsc --noEmit` clean in the app directory both after extending S-07 and after adding AV-12/13.
Re-ran the full `npx vitest run` suite in `nodejs/` afterward as a regression check: still
169/169 passing, confirming the validation-app changes (a separate npm package consuming
`machine-config-library` as a `file:` dependency) didn't require or trigger any change to the
library itself.

### Go

**Status: done and verified, 2026-08-20.**

- [x] `docs/validation/go/app/scenarios/scenarios.go` — added `RunAv12OpcuaFacadeOk`/
      `RunAv13OpcuaFacadeValidation`, wired into `docs/validation/go/app/main.go`'s scenario list
      (now 19 scenarios, up from 17 — Go's app has no AV-09–11 at all, like Rust/C++, since Go's
      dispatcher has no registry seam either; those live in `go/`'s own test tree). New import:
      `machine-config-go/capabilities` — this app had never touched the `capabilities` package
      before. Added `strPtrEqual`/`intPtrEqual`/`strPtrOrNil`/`intPtrOrNil`/`boolPtrOrNil` helpers
      to `docs/validation/go/app/scenarios/common.go` alongside the existing `fmtF64Ptr`, needed
      because Go has no built-in nil-safe pointer comparison/formatting.
  - [x] `RunAv12OpcuaFacadeOk` — opens `reference_config_opcua.h5` via `capabilities.OpenMachineConfig`,
        calls `.GetOpcua()`, confirms no error and prints five of the newly-promoted/new fields to
        prove they're actually readable, not just present.
  - [x] `RunAv13OpcuaFacadeValidation` — opens `opcua_missing_required.h5` via
        `capabilities.OpenMachineConfig`, confirms `.GetOpcua()` returns an error with
        `Code == capabilities.ErrValidation`, and asserts `Details` (compared via a lookup map, so
        order-independent) equals exactly the seven expected entries — including
        `"Laser Emission Interlock.Event"` and, implicitly, the absence of any
        `"Chamber Oxygen Level"` entry.
- [x] **S-07 extended** (`RunS07Opcua`) — added roundtrip assertions for `MachineProfile`,
      `RootNode`, `PipeName`, and `TriggerStopCeilingLayers`, proving the Phase 1 fields survive a
      real write→read cycle through the plain public `Reader`/`Writer` API, not just via
      `go test`. Go's `RunS07Opcua` was already the thinnest of the five languages' S-07
      (no trigger-level checks at all pre-existing), so no trigger fields were extended here
      either — consistent with what was already there, not a new gap introduced.
- [x] S-09 unchanged, per plan (no new top-level type introduced).

**Verification:** built with the same `CGO_ENABLED=1` + MSYS2 MinGW64 `gcc`/`g++` toolchain as
Phase 1/2, via the app's own `replace machine-config-go => ../../../../go` directive (always
builds against local source directly, no separate link step needed). Ran the built binary against
the real fixture set, with `/mingw64/bin` also on `PATH` at runtime (needed for `libhdf5-310.dll`):

```
[PASS] S-07: opcua server_url="opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer" machine_profile=Aconity preserved through roundtrip
[PASS] AV-12: GetOpcua Ok: machine_profile=Aconity root_node=MachineFleet pipe_name=\\.\pipe\opc_ua_client_pipe triggers_enabled=true trigger_stop_ceiling_layers=3
[PASS] AV-13: ValidationError with details=[Configure_Client Laser Emission Interlock.Event Machine_Profile Pipe_Name Root_Node Trigger_Stop_Ceiling_Layers Triggers_Enabled]

19 scenarios: 19 passed, 0 failed
```

`go vet ./...` clean both in the main module and (implicitly, via the same build) the app.
Re-ran the full `go test ./... -v -count=1` suite in `go/` afterward as a regression check: still
59/59 passing, confirming the validation-app changes (a separate Go module) didn't require or
trigger any change to the library itself. Removed the built binary afterward (not a tracked
artifact).

### C++

**Status: done and verified, 2026-08-20.**

- [x] `docs/validation/cpp/app/src/scenarios/scenarios.cpp` — added `av12`/`av13` namespaces,
      wired into `docs/validation/cpp/app/src/main.cpp`'s `scenarioList` (now 19 scenarios, up
      from 17 — C++'s app has no AV-09–11 at all, like Rust/Go, since C++'s dispatcher is a
      hardcoded `if (ver != "1.0") throw` with no registry seam either; those live in `cpp/tests/`
      instead). Declared in `docs/validation/cpp/app/src/scenarios/scenarios.hpp`. Added a small
      `scenarios::optOrNil<T>(const std::optional<T>&)` template to `common.hpp` — needed because
      unlike Go's typed pointer-or-nil helpers, C++ had no existing convention for rendering an
      unset `std::optional` in a scenario detail string.
  - [x] `av12::run` — opens `reference_config_opcua.h5` via `MachineConfigFileV1_0::open()`
        (not `openMachineConfig()` — the version-agnostic `IMachineConfigFile` interface in
        `generated.hpp` only exposes `fileVersion()`/`opticalTrainCount()`/`save()`/`close()`;
        `getOpcua()` lives on the concrete v1.0 type, the same as every other per-component
        getter this app never routes through the dispatcher for), calls `.getOpcua()`, confirms
        `.ok()` and prints five of the newly-promoted/new fields to prove they're actually
        readable, not just present.
  - [x] `av13::run` — opens `opcua_missing_required.h5` (also via `MachineConfigFileV1_0::open()`)
        via `avFixture()`, confirms `.getOpcua()` is not `.ok()` and `.errorCode() ==
        "ValidationError"`, and asserts `.errorDetails()` (compared as a `std::set`,
        order-independent) equals exactly the seven expected entries — including `"Laser
        Emission Interlock.Event"` and, implicitly, the absence of any `"Chamber Oxygen Level"`
        entry.
- [x] **S-07 extended** (`s07::run`) — added roundtrip assertions for `machine_profile`,
      `root_node`, `pipe_name`, and `trigger_stop_ceiling_layers`, proving the Phase 1 fields
      survive a real write→read cycle through the plain public `MachineConfigReader`/`Writer`
      API, not just via Catch2. Matches Go's S-07 extension in scope (no trigger-level field
      additions), since C++'s pre-existing S-07 was already trigger-name/signal/subsystem-level
      only, not per-field.
- [x] S-09 unchanged, per plan (no new top-level type introduced).

**Verification:** rebuilt the standalone app (`cmake --build docs/validation/cpp/app/build
--config Debug --target validation_app`) — its own CMake project, `add_subdirectory`-consuming
`cpp/` exactly as a real from-source consumer would, so this exercised the real Phase 1/2 changes
through a fresh, independent build tree, not just `cpp/build`. Ran the built binary against the
real fixture set (`validation_app.exe fixtures "Reference Materials"`):

```
[PASS] S-07: OPCUA roundtrip OK: 2 triggers, url="opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer"
[PASS] AV-12: getOpcua Ok: machine_profile=Aconity root_node=MachineFleet pipe_name=\\.\pipe\opc_ua_client_pipe triggers_enabled=true trigger_stop_ceiling_layers=3
[PASS] AV-13: ValidationError with details={'Configure_Client', 'Laser Emission Interlock.Event', 'Machine_Profile', 'Pipe_Name', 'Root_Node', 'Trigger_Stop_Ceiling_Layers', 'Triggers_Enabled'}

19 scenarios: 19 passed, 0 failed
```

Re-ran the full `cpp/build` Catch2 suite afterward as a regression check (`cmake --build cpp/build
--config Debug --target machine_config_tests`, then running `machine_config_tests.exe` directly):
still 463/463 assertions passing in 87 test cases, confirming the validation-app changes (a
separate standalone CMake project) didn't require or trigger any change to the library itself.

**This completes the entire 5-language OPCUA field-promotion rollout** — Rust, Python, Node.js,
Go, and C++ are all fully done: Phase 1, Phase 2, and validation-app coverage complete and
verified for each.

---

## Documentation pass (after implementation, not blocking it)

- [ ] `docs/{rust,python,nodejs,go,cpp}.md` — check for any place that enumerates the OPCUA model
      fields explicitly; update if so.
- [ ] `VALIDATION_PLAN.md` §8 — add AV-12/AV-13 to the scenario catalog.
- [ ] Each language's `docs/validation/<lang>/results.md`/`PASS_FAIL.md` — update once AV-12/13
      are implemented and passing.

---

## Required/optional list — confirmed

Source: a CSV attribute reference the user supplied. Flagged and resolved before finalizing: the
CSV's `Attribute` name column and its `Use Case`/`Default`/`Variants`/`Dtype`/`Units`/`Required`
columns were frequently detached from each other (e.g. the row literally named
`Trigger_Stop_Ceiling_Layers` carried `Configure_Client`'s real metadata one row down, and the
row named `Inbound_Rate_Limit` carried `Configure_Client`'s). Every field below was instead
anchored to its real value from `fixtures/reference_config_opcua.h5` (already dumped directly via
h5py earlier in this effort) before accepting a `Required` value from the CSV. Two of the
CSV-derived guesses were corrected by the user (`component`: Y→N, `event`: N→Y), one gap
(`sync_loop_interval_settled`, unrecoverable from the CSV) was resolved directly by the user as N,
and `triggers_enabled` — an already-existing field, not one of the 23 in scope for promotion —
was added to the required list on the user's review, matched in the CSV to "master gate for
trigger automation," Required=Y.

```
REQUIRED when OPCUA present:
  - machine_profile               (OpcuaClientConfig)
  - root_node                     (OpcuaClientConfig)
  - configure_client              (OpcuaPipeConfig)
  - pipe_name                     (OpcuaPipeConfig)
  - event                         (OpcuaTrigger — checked per-trigger, see Phase 2)
  - trigger_stop_ceiling_layers   (OpcuaConfig — new field)
  - triggers_enabled              (OpcuaConfig — pre-existing field, not part of the promotion,
                                    only the validation check)

OPTIONAL:
  - keep_alive_count, lifetime_count, queue_policy, queue_size_data_change,
    queue_size_events, reconnect_interval, sync_loop_interval_initial,
    sync_loop_interval_settled                                    (OpcuaClientConfig)
  - inbound_rate_limit, max_inbound_message_size, min_integrity_level,
    user_access_level                                             (OpcuaPipeConfig)
  - case_sensitivity, component, cooldown_period, max_fires_per_job,
    trigger_label                                                 (OpcuaTrigger)
```

The `Required when OPCUA present?` column in "The 23 fields" table above only covers the 23
fields being promoted/added — `triggers_enabled` doesn't appear there since it's out of scope for
Phase 1, but it is folded into Phase 0's fixture-generation step and Phase 2's mechanism
description above. Nothing further is blocked — both phases are ready to implement.
