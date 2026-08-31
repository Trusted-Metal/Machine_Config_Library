# File_Version 1.1 — Implementation Plan

**Status: partially unblocked.** `DISPATCH_REGISTRY_PLAN.md` is now done and verified in
all 5 languages (2026-08-31) — every `File_Version` dispatch point is a real registry, so
adding v1.1 is now a one-line entry per language, not a bespoke dispatch edit. Still
blocked on three specific open items in the migration manifest (see "Blockers" below).
Changes 2 and 5 have no manifest-item dependency and may start in Python immediately.

**Related, deliberately deprioritized:** `PARITY_AUDIT.md` catalogs functional/API parity
gaps across the 5 languages (independent of `File_Version` dispatch, which
`DISPATCH_REGISTRY_PLAN.md` already fixed). By explicit decision (2026-08-31), that audit
is addressed *after* this plan, not before or alongside — noted here so it isn't
rediscovered or accidentally started mid-v1.1-work. Two items in it are worth a glance
during this plan specifically (builder construction-option gaps affecting in-memory
fixture construction, and a `facility_id`/`config_author` serialization note relevant to
adding new fields) — see that file's own "Relationship to V1_1_IMPLEMENTATION_PLAN.md"
section.

**Content spec:** `file_testing/v1_0_to_v1_1.md` — the migration manifest, already fully
designed (5 changes, before/after HDF5 trees, field-by-field tables, StableModel deltas,
derivation rules), hand-verified via `file_testing/reference_config_v1_1_review.h5`. This
plan does not repeat that content — it covers *how* to execute
`docs/contributing.md`'s "Adding a new file version" checklist (layers 2–6) against it,
and *how to test it correctly*, which the manifest itself doesn't specify.

**The 5 changes, one line each** (see the manifest for full detail):
1. ClearBox relocates `Optional_Components/ClearBox` → `/Extensions/ClearBox/<train_id>/`;
   `Output_Path`/`Software_Trigger_Delay` consolidated across trains (new **Consolidate**
   category, hard error on disagreement); 4 attrs dropped; `Firmware_Version` added.
2. `/OPCUA` → `/Extensions/TM_OPCUA/` — pure relocate+rename, zero StableModel impact.
3. ClearBox's `Volts_To_Watts_*` → structured `Power_Characterization/` group (new
   **Transform** category) — new `PowerCharacterization` StableModel type, reuses
   existing `EquationConstant`/`CalibrationPoint`.
4. Light_Source's `Watts_To_Volts_*` → the **same** `Power_Characterization/` shape,
   different HDF5 path, inverted data availability from Change 3.
5. `Tuning_Parameters`/`Tuning_Type` dropped from every Scanner axis group — pure
   removal, no StableModel field removed.

---

## Blockers

- [ ] **Exact error type/name, Output_Path/Software_Trigger_Delay Consolidate conflict**
      (blocks Change 1). Follow each language's own existing error-handling convention —
      **not** `schema/capabilities/errors.yaml`. That file generates `CapabilityError`,
      the Result-based error type for the *optional get/set facade* only
      (`UnsupportedVersion`, `NotPresent`, etc.) — a different surface from the plain
      reader/writer's errors, which is what a migration/write-time hard error actually
      is. The manifest's own precedent is the right model: a new variant on each
      language's plain error type, e.g. Rust's `MachineConfigError` enum
      (`rust/src/error.rs:24`'s `UnitMismatch` is the precedent to match); Python
      following `capabilities/v1_0/hdf5.py:230,248`'s existing convention of raising
      `ValueError` for validation failures (no new exception class required unless
      wanted).
- [ ] **Exact error type/name, unrecognized `Algorithm_Type` during migration** (blocks
      Changes 3 and 4). Same reasoning and same per-language convention as above.
- [ ] **🔶 Decision: should `Algorithm_Type` be a closed enum (`Linear`/`Polynomial`
      only) or an open string**, for natively-authored (non-migration) v1.1 files? Blocks
      Changes 3 and 4's `schema/capabilities/models.yaml` entry — changing this after
      5 languages' generated code references it means a second regen cycle.
- [ ] **New — backward-migration derivation rules for Changes 3/4 are not yet written.**
      The manifest documents forward (v1.0→v1.1) derivation rules for both changes in
      detail, but `docs/contributing.md`'s test template requires a `test_vX_Y_to_vprev`
      (backward) test too, and the manifest is currently silent on it. Change 3's
      `Algorithm_Type` + `Derivation_Equation_Constants` look mechanically reversible
      into `Volts_To_Watts_Algorithm`/`_Params` (inverse of the forward parse); Change
      4's `Characterization_Points` similarly look reversible into
      `Watts_To_Volts_Params`. Confirm this is actually the intended backward behavior
      (not assumed here) and add it to the manifest before writing
      `test_v1_1_to_v1` for these two changes — Changes 1, 2, and 5 already have
      unambiguous backward behavior (Consolidate expands 1→N trivially; OPCUA is a
      verbatim move either direction; Change 5's removal has nothing to reverse).

Non-blocked work (safe to start immediately, any time): Changes 2 and 5 have no open
manifest items and no StableModel impact.

---

## Phase 0 — shared artifacts (once, not per language)

- [ ] Resolve the two error-name blockers and the Algorithm_Type-enum decision above;
      update `file_testing/v1_0_to_v1_1.md` in place; flip its `STATUS: DRAFT` banner
      once all items are closed.
- [ ] `schema/capabilities/models.yaml` — add `firmware_version` to `ClearBox`; add a new
      `PowerCharacterization` model (`algorithm_type`, `algorithm_equation`, `input_type`,
      `units_derived_quantity`, `derivation_equation_constants`,
      `characterization_points`); reference it as an optional field from both `ClearBox`
      and `LightSource`. Confirmed this file mirrors the StableModel 1:1 (e.g.
      `AxisConfig`'s `tuning_parameters`/`tuning_type` are already listed there exactly
      as in `rust/src/models.rs`) — it drives the capabilities facade's generated
      getters/setters, so skipping it means plain read/write works but the optional
      facade silently doesn't expose the new fields.
- [ ] Run `tools/generate_capabilities.py` — regenerates all 5 languages' `generated.*`
      in one pass.
- [ ] Hand-edit each language's plain `models.*` to add the same StableModel fields —
      these are hand-written, not generated (confirmed: `EquationConstant`/
      `CalibrationPoint` are already hand-authored and reused per the manifest).
- [ ] Two real fixtures needed, distinct purposes — do not reuse one for both:
  - [ ] A **natively-authored v1.1 fixture**, fully populated in every field including
        both `Power_Characterization` groups' `Characterization_Points` (this is what
        `file_testing/reference_config_v1_1_review.h5` already is, by design — its own
        docstring calls out the "populate normally-empty datasets for structural review"
        deviations specifically so every field has real values to inspect). Promote it
        (or a cleaned-up copy) to `fixtures/` for use by `test_v1_1_read` and
        `test_v1_1_roundtrip` below.
  - [ ] A **genuine migration output** — the actual result of running each language's
        v1.0→v1.1 migration path against a real v1.0 fixture
        (`fixtures/reference_config_opcua_synchronous_sensors.h5`), with **no** synthetic
        overrides. This will have empty `Characterization_Points` for ClearBox, empty
        `Derivation_Equation_Constants` for Light_Source, and `firmware_version = None`
        — all per the manifest's actual forward-migration rules, deliberately different
        from the review file. Used by `test_v1_to_v1_1` below. **Do not port the review
        script's synthetic-population logic** (`SYNTHETIC_FIRMWARE_VERSION`, the
        least-squares-fit/evaluate-at-sample-points helpers) into the real migration
        adapter — those exist solely so the review file has inspectable values, and are
        not part of the manifest's actual migration behavior.

## Phase 1 — per-language adapter (Python leads, then Rust → C++ → Go, Node.js flexible)

Common shape per language, once Phase 0 lands: (a) `capabilities/v1_1/{file,layout,hdf5,
writer}.*`, mirroring the `v1_0/` folder's structure per `docs/contributing.md`'s table;
(b) one registry entry per dispatch point — now genuinely one line each, thanks to
`DISPATCH_REGISTRY_PLAN.md`; (c) the migration functions (forward and backward) living
wherever that language's existing convention puts version-to-version migration logic.

Order rationale: Python first (this repo's reference implementation — other languages'
adapters and tests are transliterated from it), then Rust (works out any migration-logic
patterns before C++/Go copy them), then C++, then Go. Node.js is table-based and
low-risk like Python — it can slot in anywhere after Python without materially changing
overall risk.

- [ ] **Python**: `python/src/machine_config/capabilities/v1_1/{file,layout,hdf5,writer}.py`
      + registry entries (`reader.py:_ADAPTERS`, `writer.py:_ADAPTERS`,
      `capabilities/__init__.py:_OPEN` and `_CREATE` — the latter now exists thanks to
      `DISPATCH_REGISTRY_PLAN.md`).
- [ ] **Rust**: `rust/src/capabilities/v1_1/{file,layout,hdf5,writer}.rs` + registry
      entries in `reader.rs`, `writer.rs`, `capabilities/mod.rs` (now genuine registry
      inserts, not new `match` arms, per `DISPATCH_REGISTRY_PLAN.md`).
- [ ] **C++**: `cpp/include/machine_config/capabilities/v1_1/{file,layout,hdf5,writer}.hpp`
      + registry entries.
- [ ] **Go**: `go/capabilities/v1_1/` (`file.go` plus `layout/`, `hdf5/` packages, per
      `docs/contributing.md`'s table) + registry entries. Note `go/internal/mockv1_1/`
      already demonstrates this folder shape for the mock adapter — real v1.1 mirrors
      its structure minus the "-mock" suffix and test-only placement.
- [ ] **Node.js**: `nodejs/src/capabilities/v1_1/{file,layout,hdf5,writer}.ts` + registry
      entries.

For each language, run its full existing suite immediately after Phase 1 lands, before
adding the new tests below — isolates "did I break v1.0" from "did I get v1.1 right."

---

## Testing (per language, following `docs/contributing.md`'s 7-test template, made
concrete for this manifest)

Template names below match `docs/migrations/mock_v1_0_to_v1_1.md`'s existing naming
exactly (`test_v1_1_read`, etc.) — the same shape, applied to real content instead of
the mock's Addition/Removal/Name/Path/Name+Path set.

- [ ] **`test_v1_1_read`** — reads the natively-authored v1.1 fixture (Phase 0). Asserts,
      by value, every change category: Change 1's hoisted `Output_Path`/
      `Software_Trigger_Delay` (same value read for every train), all 4 dropped fields
      absent/`None`, `firmware_version` populated; Change 2's `opcua` populated
      identically to a v1.0 OPCUA fixture; Change 3's `ClearBox.power_characterization`
      fully populated (`algorithm_type`, generated `algorithm_equation`, both compound
      datasets non-empty); Change 4's `LightSource.power_characterization` likewise;
      Change 5's `tuning_parameters`/`tuning_type` absent on every axis.
- [ ] **`test_v1_1_roundtrip`** — write → read → write → read an in-memory v1.1
      `MachineConfig`, no drift. **Populate ClearBox's and Light_Source's
      `power_characterization` with deliberately different values** (different
      `algorithm_type`, different constant names/values) in the same test object — this
      is the direct test for the cross-wiring risk flagged during planning (same struct,
      two HDF5 paths per train; a swapped read/write assignment would only be caught if
      the two instances are actually distinguishable). A test using identical values for
      both would not catch this class of bug.
- [ ] **`test_v1_to_v1_1`** — forward migration from the real v1.0 fixture
      (`reference_config_opcua_synchronous_sensors.h5`) using the genuine migration
      output fixture from Phase 0 (no synthetic overrides). Asserts: hoisted
      `Output_Path`/`Software_Trigger_Delay` match the real fixture's actual values
      (confirmed today: `/recordings/`, `3000`); 4 dropped fields `None`;
      **`firmware_version is None`** (no v1.0 source — must not be a synthetic
      placeholder, see Phase 0's fixture note); Change 3's `characterization_points ==
      []` (empty, not omitted — schema-complete-but-empty, per the manifest's explicit
      rule); Change 4's `derivation_equation_constants == []` (the mirror-image empty
      case); Change 5's fields absent.
  - [ ] **`test_v1_to_v1_1_output_path_disagreement_raises`** — a v1.0 fixture (or
        in-memory model) with trains whose `Output_Path` deliberately disagrees; asserts
        the specific hard error from the resolved blocker above, naming the conflicting
        trains/values per the manifest's stated error contract. This is new — the
        existing mock template has no Consolidate-category precedent to copy.
  - [ ] **`test_v1_to_v1_1_software_trigger_delay_disagreement_raises`** — same shape,
        the other Consolidate field.
  - [ ] **`test_v1_to_v1_1_unrecognized_algorithm_type_raises`** — a v1.0 fixture with
        `Volts_To_Watts_Algorithm`/`Watts_To_Volts_Algorithm` set to neither `LINEAR`
        nor `POLYNOMIAL`; asserts the hard error from the resolved blocker.
- [ ] **`test_v1_1_to_v1`** — backward migration. Changes 1/2/5 are unambiguous
      (Consolidate expands the one shared value to every train; OPCUA moves back
      verbatim; Change 5 has nothing to reverse). **Changes 3/4 are blocked on the
      manifest gap noted above** — write this part only after that's resolved; until
      then, this test may cover Changes 1/2/5 only, explicitly noted as partial in a
      comment, not silently incomplete.
- [ ] **`test_v1_unaffected`** — the full existing v1.0 suite/golden-file comparison,
      confirming zero behavior change to the v1.0 path after v1.1 is added. This is the
      same check run after Phase 1 above, formalized as a named regression test.
- [ ] **`test_dispatcher_v1_to_v1_1` / `test_dispatcher_v1_1_to_v1`** — full public API
      via `MachineConfigReader`/`MachineConfigWriter`, now exercising the *real*
      registry entry from `DISPATCH_REGISTRY_PLAN.md` directly — no monkeypatching
      needed, unlike the mock's temporary injection, since `"1.1"` is a permanent
      registry entry once this ships.
- [ ] **`test_adapters_satisfy_protocol`** — `isinstance`/trait-bound check that the new
      v1.1 reader/writer satisfy `ReaderAdapter`/`WriterAdapter` (Rust's version of this
      check is now meaningful for the first time, since those traits only exist once
      `DISPATCH_REGISTRY_PLAN.md` ships).

---

## Verification

1. Per language, per change: that language's suite green, immediately after Phase 1.
2. All tests above, green, per language.
3. Full `python tools/cross_check.py --verbose` (all 5 languages, all 5 phases) with the
   v1.1 fixtures from Phase 0 included alongside the existing v1.0 fixtures.
4. Move `file_testing/v1_0_to_v1_1.md` → `docs/migrations/v1_0_to_v1_1.md`; fold the two
   new change categories (Consolidate, Transform) into `docs/contributing.md`'s category
   table.
5. CHANGELOG entry — confirm format first: this repo's `CHANGELOG.md` is currently
   semantic-release-generated from conventional commits, not the hand-authored
   `## [1.1.0]` table `docs/contributing.md`'s own example shows. Resolve which is
   actually wanted before writing one by hand.
