# OPCUA Field Promotion + Required-Field Validation — Implementation Plan

**Status:** Required/optional split confirmed (see "The 23 fields" table and "Required/optional
list — confirmed" at the bottom). Phase 0's shared artifacts are done and verified (schema,
reference fixture, `opcua_missing_required.h5` — see Phase 0 below). **Rust is fully done** —
Phase 1, Phase 2, and validation-app coverage all complete and verified (125/125 crate tests,
19/19 validation-app scenarios). Python is next — Phase 1 → Phase 2 → validation-app coverage —
then Node.js → Go → C++ in turn, each language completing all three steps before the next
language starts.

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

- [ ] `python/src/machine_config/models.py` — add fields (`Optional[T] = None`).
- [ ] `python/src/machine_config/capabilities/v1_0/hdf5.py` — add reads in `_parse_opcua`; add
      names to `_KNOWN_CLIENT_KEYS`/`_KNOWN_PIPE_KEYS`/`_KNOWN_TRIGGER_KEYS`; read
      `Trigger_Stop_Ceiling_Layers`.
- [ ] `python/src/machine_config/capabilities/v1_0/writer.py` — add writes via existing `_s`/
      `_f`/`_i`/`_b` helpers.
- [ ] No existing breaking test (confirmed via grep across `python/tests/`).
- [ ] **New tests** — same three categories as Rust.

### Node.js

- [ ] `nodejs/src/models.ts` — add fields (`field: T | null`) to the three interfaces.
- [ ] `nodejs/src/capabilities/v1_0/hdf5.ts` — add `attrStr`/`attrInt`/`attrBool` calls; add names
      to `KNOWN_CLIENT`/`KNOWN_PIPE`/`KNOWN_TRIGGER`; read `Trigger_Stop_Ceiling_Layers`.
- [ ] `nodejs/src/capabilities/v1_0/writer.ts` — add `ws`/`wi`/`wf`/`wb` calls.
- [ ] No existing breaking test (confirmed).
- [ ] **New tests** — same three categories. Reminder: TS object literals must list every
      property unless a field is `?:`, so any hand-built test fixture needs the new fields
      listed explicitly.

### Go

- [ ] `go/internal/models/models.go` — add fields as pointer types (`*int`, `*string`, `*bool`).
- [ ] `go/capabilities/v1_0/hdf5/hdf5.go` — add `readStrAttr`/`readIntAttr`/`readBoolFromIntAttr`
      calls; add names to the inline `clientKnownKeys`/`pipeKnownKeys`/`triggerKnownKeys` maps in
      `parseOpcua`; read `Trigger_Stop_Ceiling_Layers`.
- [ ] `go/capabilities/v1_0/hdf5/writer.go` — add `ws`/`wi`/`wf`/`wb` calls.
- [ ] No existing breaking test (confirmed).
- [ ] **New tests** — same three categories.

### C++

- [ ] `cpp/include/machine_config/models.hpp` — add fields as `std::optional<T>`.
- [ ] `cpp/include/machine_config/capabilities/v1_0/hdf5.hpp` — add `readStr`/`readInt`/
      `readBoolFromInt` calls; add names to the inline initializer-lists passed to
      `collectExtra(...)`; read `Trigger_Stop_Ceiling_Layers`.
- [ ] `cpp/include/machine_config/capabilities/v1_0/writer.hpp` — add `ws`/`wi`/`wf`/`wb` calls.
- [ ] No existing breaking test (confirmed).
- [ ] **New tests** — same three categories.

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
