# File_Version 1.1 — Implementation Plan

**Status: Phase 0 and Phase 1 both complete, all 5 languages (2026-09-01).**
`DISPATCH_REGISTRY_PLAN.md` is done and verified in all 5 languages (2026-08-31) — every
`File_Version` dispatch point is a real registry, so adding v1.1 was a one-line entry per
language, not a bespoke dispatch edit. All four manifest blockers are resolved
(2026-08-31, see "Blockers" below). Phase 0 is fully complete, including both fixtures
(see Phase 0 below for details, including two real bugs found and fixed along the way —
a fixture-generation ASCII/UTF-8 charset bug, and C++'s JSON-serialization gap). Phase 1
is now complete across Python, Rust, C++, Go, and Node.js (see Phase 1 below for each
language's details) — every adapter is self-contained (machine-enforced by a per-language
version-adapter-isolation guard test), implements all 5 manifest changes, and has a
matching test suite using the same two shared fixtures. A cross-language message-format
inconsistency in the manifest itself (Consolidate-conflict detail wrapped in parentheses
vs. the stated em-dash standard, in the Go/C++ bullets) was caught and fixed along the
way — C++'s implementation self-corrected it, Go's didn't and had to be fixed after the
fact, Node.js's implementation used the corrected text from the start. **A real
architectural gap was found after Phase 1 shipped**: reading a natively-authored v1.1
file and writing it straight to v1.0 (no migration function involved, just the ordinary
public `parse()`/`write()` path) silently drops `Volts_To_Watts_*`/`Watts_To_Volts_*`,
because only `Hdf5AdapterV1_0`'s reader ever derived `power_characterization` — v1.1's
reader never derived the reverse. **Phase 2 fixes this — done in all 5 languages
(2026-09-01).** The design changed once before Python's
implementation landed: an initial draft (still visible, marked stale, in that section
below) proposed symmetric derivation in both *readers*; review found this only works
for models that came from an actual file read (missing `create()`, hand-built mocks,
etc.) and read as unwanted mutual awareness between the two versions' adapters, so the
shipped design instead puts a derive-only-as-fallback step in both *writers*, leaving
both readers untouched from Phase 1. See Phase 2's "Design decision" subsection for the
full comparison, and Python's write-up there for exactly what was built, a design
detour that was implemented then reverted mid-pass, a real bug caught by a test before
being called done (the writer's `target_version` parameter silently stamping the wrong
on-disk `File_Version` without a corresponding fix), and what's deliberately deferred
(fixture regeneration, manifest update) until closer to when the remaining languages
start. **Rust and C++ (each mirrors Python's design, plus the same one real
cross-language difference in `create()` — see each language's write-up) were both
blocked mid-verification by an unrelated Windows security issue in this environment
preventing execution of freshly-built `.exe` files** — both compiled/linked clean the
whole time, but neither's test suite could actually run. Once the user resolved the
security issue separately, both were re-run: **Rust 201/201 tests pass**; **C++ 826
assertions / 155 test cases pass**, after fixing one real bug the first C++ run
surfaced — the version-adapter-isolation guard test correctly flagged two Phase 2 doc
comments that incidentally mentioned the *other* version's token in prose (a plan
filename, a file path), not an actual code coupling; reworded to be version-neutral,
matching the convention Phase 1's own migrate-function comments already followed. See
C++'s write-up for the full detail. **Go is also done** (no security-issue blocker hit
here — `go test` was never subject to the same freshly-built-`.exe` block, since Go's
test binaries are compiled and run internally by the `go test` toolchain rather than
via CTest's separate discovery step): `go build`/`go vet ./...` clean, `go test ./...`
133/133 tests pass across the module. One more real cross-language finding here, in the
opposite direction from Rust's/C++'s: Go's `Create()` needed **zero** changes for Phase
2, since it was never built on a migrate function or `MockConfigBuilder` to begin with
— see Go's write-up for the detail. **Node.js is done too — the final language,
completing Phase 2 across all 5.** Its version-adapter-isolation guard test is a
raw-text scan like C++'s, not parser-based — but having already been burned by C++'s
false positive, every new Phase 2 comment was written version-neutral from the start,
and the guard test passed clean on the first run, no fix needed. `create()` needed the
same explicit-derivation treatment as Rust's/C++'s (no real disk round trip). One more
real, pre-existing finding surfaced here: Node's float-to-string formatting
(`String(value)`, e.g. `"1"`) already differed from the other 4 languages'
always-include-a-decimal-point convention (e.g. `"1.0"`) before Phase 2 ever started —
confirmed by reading the existing Phase 1 test expectations, not discovered by a
failure — and was deliberately preserved rather than "corrected," since fixing a
pre-existing Phase 1 formatting difference is out of Phase 2's scope; worth a
`PARITY_AUDIT.md` note. `npx tsc --noEmit` clean, `npx vitest run` 236/236 tests pass
across 9 files, after fixing one real bug the first run surfaced (an assertion expecting
`undefined` where a real disk round trip correctly normalizes to `null` — the same class
of finding every other language's Phase 2 rewrite hit). See Node's write-up for the full
detail. **Phase 2 is now complete in all 5 languages.** The Verification section's
cross-cutting closeout work is now done except one item: the manifest is promoted to
`docs/migrations/v1_0_to_v1_1.md` (with a new "API impact" section covering the Phase 2
writer-API changes), `docs/contributing.md` is updated (category table, CHANGELOG-entry
step), and the CHANGELOG format decision is resolved (`feat` commits, semantic-release
stays automated, no hand-authored table) — see the Verification section below. Still
outstanding: the full `cross_check.py` run, and the still-deferred fixture regeneration
noted in Python's Phase 2 write-up (now overdue, since every language is done).

**Done (2026-09-01):** the Node float-formatting difference is now recorded in
`PARITY_AUDIT.md`'s Tier 2 (real capability/consistency gaps), including why a real fix
isn't a one-file patch to `powerCharacterization.ts` — Node's plain `String(value)` is
used for every other float field across the codebase too, so decimal-forcing only the new
v1.1 module would trade today's uniform-but-cross-language-inconsistent state for a new
*internal* Node inconsistency instead. Closing it out for real means porting the
`format_f64_like_python`-style helper already written for Rust/C++/Go repo-wide across
Node's writers, touching existing v1.0 float-serialization call sites and their tests —
deliberately kept out of Phase 2's scope. By explicit decision (2026-09-01), left as
tracked audit debt rather than folded into this plan.

**Related, deliberately deprioritized:** `PARITY_AUDIT.md` catalogs functional/API parity
gaps across the 5 languages (independent of `File_Version` dispatch, which
`DISPATCH_REGISTRY_PLAN.md` already fixed). By explicit decision (2026-08-31), that audit
is addressed *after* this plan, not before or alongside — noted here so it isn't
rediscovered or accidentally started mid-v1.1-work. Two items in it are worth a glance
during this plan specifically (builder construction-option gaps affecting in-memory
fixture construction, and a `facility_id`/`config_author` serialization note relevant to
adding new fields) — see that file's own "Relationship to V1_1_IMPLEMENTATION_PLAN.md"
section.

**Content spec:** `docs/migrations/v1_0_to_v1_1.md` — the migration manifest, already fully
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

- [x] **Exact error type/name, Output_Path/Software_Trigger_Delay Consolidate conflict**
      (blocked Change 1). **Resolved (2026-08-31)** and written into
      `docs/migrations/v1_0_to_v1_1.md`'s Change 1 "Consolidate conflict rule" section. Each
      language follows its own existing validation-error convention (confirmed by reading
      each one, not assumed) rather than a new shared exception type — Rust: new
      `MachineConfigError::ConsolidateConflict { field, detail }` variant
      (`rust/src/error.rs`, struct-style matching `UnitMismatch`); Python: plain
      `raise ValueError(...)`, no new class, matching `v1_0/hdf5.py:230,248`; Node.js:
      plain `throw new Error(...)`, matching `v1_0/hdf5.ts`'s inline-throw convention; Go:
      plain `fmt.Errorf(...)`, matching `capabilities/v1_0/hdf5/*.go`; C++: plain
      `throw std::runtime_error(...)`, matching `v1_0/hdf5.hpp`'s Rule-8 validation
      throws. Message content is standardized across all five so tests can assert on the
      same substrings despite the differing type machinery — see the manifest for the
      exact template.
- [x] **Exact error type/name, unrecognized `Algorithm_Type` during migration** (blocked
      Changes 3 and 4). **Resolved (2026-08-31)**, same per-language approach as above —
      written into `docs/migrations/v1_0_to_v1_1.md`'s Change 3 "Unrecognized `Algorithm_Type`"
      bullet (applies identically to Change 4's `Watts_To_Volts_Algorithm`). Rust:
      `MachineConfigError::UnrecognizedAlgorithmType(String)`, tuple-style matching
      `UnsupportedVersion(String)`; other four languages: plain error/exception, no new
      type, standardized message text.
- [x] **Decision: should `Algorithm_Type` be a closed enum (`Linear`/`Polynomial`
      only) or an open string**, for natively-authored (non-migration) v1.1 files?
      **Resolved (2026-08-31): open string.** No enum constraint in
      `schema/capabilities/models.yaml`; `Linear`/`Polynomial` remain the only values the
      migration path itself ever produces or recognizes (see the two error-name blockers
      above), but the StableModel field itself stays an open string so future algorithm
      types don't require a second regen cycle to add.
- [x] **Backward-migration derivation rules for Changes 3/4.** **Resolved (2026-08-31)**
      and written into `docs/migrations/v1_0_to_v1_1.md`'s Change 3 and Change 4 sections
      (new "Backward migration (v1.1 → v1.0) — confirmed" subsections). Summary:
      backward migration only *re-serializes* already-structured, already-named data —
      it has no equivalent to forward's "unrecognized `Algorithm_Type`" failure mode.
      `Algorithm_Type` copies straight across in both directions. Change 3's
      `Derivation_Equation_Constants` rows must be explicitly re-sorted by name (`b`
      before `a`; `c0 < c1 < c2 …`) before joining as CSV, since compound-dataset rows
      are named but not positionally guaranteed on disk; Change 4's
      `Characterization_Points` need no such sort since its rows aren't named and
      on-disk order is already correct. If the needed dataset is empty (e.g. a
      natively-authored v1.1 record that only populated the other
      `Power_Characterization` dataset), write the corresponding v1.0 params string as
      blank (`""`) rather than erroring — confirmed as absence, not ambiguity, distinct
      from the Output_Path/Software_Trigger_Delay disagreement and forward's
      unrecognized-`Algorithm_Type` hard-error cases.

Non-blocked work (safe to start immediately, any time): Changes 2 and 5 have no open
manifest items and no StableModel impact.

---

## Phase 0 — shared artifacts (once, not per language)

- [x] Resolve the two error-name blockers and the Algorithm_Type-enum decision above;
      update `docs/migrations/v1_0_to_v1_1.md` in place. **Done (2026-08-31).** The
      `STATUS: DRAFT` banner stays as-is — a few unrelated open items remain in that
      file's "Open items" list (e.g. `Input_Type` placeholder values, `/Extensions`
      unit-style metadata), so not everything is closed yet.
- [x] `schema/capabilities/models.yaml` — add `firmware_version` to `ClearBox`.
      **Done (2026-08-31)**, with one correction to this bullet's original premise:
      investigation confirmed this file does **not** drive the capabilities facade's
      generated getters/setters — `tools/generate_capabilities.py` loads it but every
      renderer discards it (`_ = api, models`); the facade's `get_model()`/`set_model()`
      are generic JSON/dict passthroughs in Rust, Python, and Node (confirmed via
      `rust/src/capabilities/merge.rs`, `python/.../merge.py`, `nodejs/.../merge.ts`),
      so new StableModel fields flow through the facade automatically regardless of this
      file. It's live documentation only (12 days old, part of an in-progress "stable
      interfaces" refactor per commit `f972630` — not dead code, just not wired up yet).
      Given that, `PowerCharacterization` was **not** added here — this file has no
      array/nested-object type and `SynchronousSensor` (a prior, structurally identical
      addition) was never added either; a scope note now documents why. `firmware_version`
      alone was added since it's a flat scalar matching the file's existing shape.
- [x] Run `tools/generate_capabilities.py --check` — confirmed a true no-op (0 diff), as
      expected given the above.
- [x] Hand-edit each language's plain `models.*` to add the same StableModel fields.
      **Done (2026-08-31)** — `rust/src/models.rs`, `python/src/machine_config/models.py`,
      `nodejs/src/models.ts`, `go/internal/models/models.go` (+ re-exported from
      `go/models.go`), `cpp/include/machine_config/models.hpp`. Added: new
      `PowerCharacterization` type (`algorithm_type`, `algorithm_equation`, `input_type`,
      `units_derived_quantity`, `derivation_equation_constants: Vec<EquationConstant>`,
      `characterization_points: Vec<CalibrationPoint>`) in all 5; `ClearBox.firmware_version`
      and `ClearBox.power_characterization` / `LightSource.power_characterization` in all 5.
      Also updated every existing `ClearBox`/`LightSource` construction site that needed it
      (Rust and Node.js require every field explicitly; Python/Go/C++ don't, due to
      dataclass defaults / Go zero-values / C++ aggregate defaults).
      **Design decision made along the way:** new fields are omitted from JSON when absent
      (not emitted as `null`) — matches the existing `synchronous_sensors`/`correction_data`
      precedent (byte-identical v1.0 output). Implemented via `#[serde(skip_serializing_if
      = "Option::is_none")]` (Rust), optional TS keys never set (Node — this was initially
      done wrong as always-present `null`, caught by a failing schema-validation test, then
      fixed), `json:"...,omitempty"` (Go), `std::optional` left unset (C++, deferred to its
      `to_json`, see Phase 1 note below), and simply not referenced by the existing
      hand-written `_clearbox_to_dict` (Python — already correct, untouched).
      **Follow-up for Phase 1, not yet done:** Python's `_clearbox_to_dict`/
      `_clearbox_from_dict` (`capabilities/v1_0/hdf5.py`) and C++'s `to_json`/`from_json`
      for `ClearBox`/`LightSource` (`models.hpp`) are hand-written per-field (unlike Rust's
      derive-based serde or Node's structural typing) — they silently drop the two new
      fields today, which is correct for v1.0 (always absent) but means the v1.1 adapter
      work must explicitly add these two fields to both functions, or v1.1's real values
      will never appear in JSON output despite being on the struct.
      **Also discovered along the way:** `schema/machine_config_v1.schema.json` (a real,
      `ajv`/`jsonschema`-enforced contract, distinct from the inert `models.yaml` above) also
      needed the same two fields added to its `clearbox`/`light_source` definitions — its
      `additionalProperties: false` on `clearbox` was what caught the Node.js null-vs-omit
      bug. Kept in sync across `schema/`, `nodejs/schema/`, and
      `python/src/machine_config/` (the latter two are copies of the canonical root file).
      Confirmed via git history this is the right file to extend in place — no separate
      `machine_config_v1_1.schema.json` — its `title` (`"MachineConfig"`) and the precedent
      of `SynchronousSensor`'s earlier addition (commit `715615b`, same file, no new file)
      both confirm "v1" here tracks the stable JSON contract's own version, not File_Version;
      a v1.0 and a v1.1 read are meant to validate against the exact same schema.
      **Verification:** full test suites green with zero regressions — Rust 154/154,
      Python 360/360, Node.js 191/191 (`tsc --noEmit` clean), Go all packages `ok` (via
      `go build`/`go test ./...` with `CGO_ENABLED=1` and MSYS2 mingw64 on `PATH`), C++ 591
      assertions / 113 cases green.
- [ ] Two real fixtures needed, distinct purposes — do not reuse one for both:
  - [x] A **natively-authored v1.1 fixture**, fully populated in every field including
        both `Power_Characterization` groups' `Characterization_Points` (this is what
        `file_testing/reference_config_v1_1_review.h5` already is, by design — its own
        docstring calls out the "populate normally-empty datasets for structural review"
        deviations specifically so every field has real values to inspect). **Promoted
        (2026-08-31)** to `fixtures/reference_config_v1_1.h5` (structure spot-checked:
        `File_Version` 1.1, `Extensions/{ClearBox,TM_OPCUA}` present) for use by
        `test_v1_1_read` and `test_v1_1_roundtrip` below.
  - [x] A **genuine migration output** — the actual result of running each language's
        v1.0→v1.1 migration path against a real v1.0 fixture
        (`fixtures/reference_config_opcua_synchronous_sensors.h5`), with **no** synthetic
        overrides. **Done (2026-08-31)**, as a byproduct of Python's Phase 1 adapter
        (see below) — generated via the real `MachineConfigReader` →
        `migrate_v1_to_v1_1` → `MachineConfigWriter` path, saved to
        `fixtures/reference_config_v1_1_migrated.h5`. Verified: `Characterization_Points`
        empty for ClearBox, `Derivation_Equation_Constants` empty for Light_Source,
        `Firmware_Version` blank — matches the manifest's forward-migration rules
        exactly, deliberately different from the natively-authored review fixture above.
        Reuse this same file for the other four languages' tests — don't regenerate it
        per language. **Regenerated (2026-09-01)** once `migrate_v1_to_v1_1` was deleted
        in Phase 2 — same source fixture, now via `MachineConfigReader(...).parse()` →
        `MachineConfigWriter(config, target_version="1.1").write(...)` instead. Content
        differs from the original only in `Input_Type`/`Units_Derived_Quantity` (now
        blank, not `"0-10 V"`/`"Volts"`/`"Watts"`), per Phase 2's blank-not-hardcoded
        fix — everything else verified unchanged.

## Phase 1 — per-language adapter (Python leads, then Rust → C++ → Go, Node.js flexible)

Common shape per language, once Phase 0 lands: (a) `capabilities/v1_1/{file,layout,hdf5,
writer}.*`, mirroring the `v1_0/` folder's structure per `docs/contributing.md`'s table;
(b) one registry entry per dispatch point — now genuinely one line each, thanks to
`DISPATCH_REGISTRY_PLAN.md`; (c) the migration functions (forward and backward) living
wherever that language's existing convention puts version-to-version migration logic.

**Guardrail, per `docs/contributing.md`:** the shared capabilities root
(`file.hpp`/`__init__.py`/`index.ts`/`mod.rs`/`file.go`) stays a dispatcher — peek
`File_Version`, call the matching adapter. The only change any of these five files
should need for v1.1 is the one-line registry entry from (b) above; no v1.1-specific
facade logic or on-disk-layout knowledge belongs there. Worth checking for explicitly in
review, since it's an easy rule to violate by accident once migration logic needs to live
somewhere and the registry file is right there.

**Second guardrail, added during Python's implementation (2026-08-31): `capabilities/
v1_1/*` must not import or call into `capabilities/v1_0/*`, even for pieces that are
byte-identical between the two versions today** (Collimator, Scanner_Card, sfcf, OPCUA's
inner Client/Pipe/Triggers shape, train-attribute read/write). The temptation is real —
Python's v1.1 adapter and v1.0's adapter genuinely share a lot of code today — but taking
a shortcut and delegating to v1.0's methods would make v1.0 a permanent runtime dependency
of v1.1, unable to change or be removed later without checking every later version leaning
on it, compounding with every version after that. Pay the small duplication cost instead:
each version's adapter is 100% self-contained, including its own copies of the small
static type-conversion helpers (`_read_str`/`_read_float`/etc.). The only place both
versions are legitimately referenced together is the three shared dispatch registries
(`reader.py`, `writer.py`, `capabilities/__init__.py`) — that's the registry's actual job,
not a coupling between the adapters. (The pre-existing mock adapter in
`python/tests/test_adapter_migration.py` did exactly this delegation before being fixed
alongside this work — see that file's own module-level note.) Apply the same discipline
in Rust/C++/Go/Node.js — do not let v1.1 import from `capabilities/v1_0`.

**This guardrail is now machine-enforced in all 5 languages (2026-08-31), not just written
down.** Each language got a new static-scan guard test that fails if any file under a real
`capabilities/vX_Y/` adapter folder — or that language's test-only mock v1.1 adapter —
references a different version's token (`vX_Y`) than its own:
- Python: `python/tests/test_version_adapter_isolation.py` (AST-based import scan).
- Rust: `rust/tests/version_adapter_isolation_test.rs` (comment/string-stripped token scan,
  no new dependency).
- Go: `go/version_adapter_isolation_test.go` (`go/parser`, `ImportsOnly` mode — doc-comment
  prose mentioning another version is never mistaken for a real import).
- Node.js: `nodejs/tests/versionAdapterIsolation.test.ts` (raw-text regex scan).
- C++: `cpp/tests/test_version_adapter_isolation.cpp` (Catch2, raw-text token scan,
  registered in `cpp/tests/CMakeLists.txt`).

All 4 languages' pre-existing test-only mock v1.1 adapters (`rust/tests/mock_v1_1/`,
`go/internal/mockv1_1/`, `cpp/tests/mock_v1_1.hpp`, `nodejs/tests/mockV1_1.ts`) were also
rewritten to be fully self-contained, matching Python's — they'd previously all
independently converged on a "delegate the whole file to the real v1.0 adapter, then patch
the ~10 documented mock differences in place" design (different from Python's original
per-method-delegation mock, but the same underlying coupling). Fixing this meant
reimplementing each language's entire v1.0-equivalent reader/writer natively inside the
mock — a large, mechanical duplication, not a small edit — because the mocks model what a
real adapter looks like (per `docs/migrations/mock_v1_0_to_v1_1.md`) and should be held to
the same standard a real future version will be. Each guard test's own development caught
at least one real leftover cross-version token (stray prose/comment references) before
passing clean. Public APIs of all 4 mocks were preserved exactly, so no calling test file
needed changes. **Verification, all 5 languages, zero regressions:** Python 378/378 (was
376), Rust 160/160 (was 154), Go all packages `ok` (91 pre-existing + 1 new guard test),
Node.js 192/192 (was 191), C++ 600 assertions/114 cases (was 591/113).

Order rationale: Python first (this repo's reference implementation — other languages'
adapters and tests are transliterated from it), then Rust (works out any migration-logic
patterns before C++/Go copy them), then C++, then Go. Node.js is table-based and
low-risk like Python — it can slot in anywhere after Python without materially changing
overall risk.

- [x] **Python**: `python/src/machine_config/capabilities/v1_1/{file,layout,hdf5,writer}.py`
      + registry entries (`reader.py:_ADAPTERS`, `writer.py:_ADAPTERS`,
      `capabilities/__init__.py:_OPEN` and `_CREATE` — the latter now exists thanks to
      `DISPATCH_REGISTRY_PLAN.md`). **Done (2026-08-31).**
      - Fully self-contained per the second guardrail above — `Hdf5AdapterV1_1`/
        `Hdf5WriterV1_1` have their own copies of every helper, including the ones
        identical to v1.0's today.
      - `migrate_v1_to_v1_1`/`migrate_v1_1_to_v1` live in `capabilities/v1_1/hdf5.py`
        (this repo has no other precedent for where migration functions belong, so
        colocating with the version they produce/consume seemed most natural).
        Forward: derives `PowerCharacterization` from `Volts_To_Watts_*`/
        `Watts_To_Volts_*` per the manifest's Change 3/4 rules (named-constant
        convention for ClearBox, direct point-pairing for Light_Source, hard error
        on an unrecognized `Algorithm_Type`). Backward: re-serializes, re-sorting
        `Derivation_Equation_Constants` by name before joining as CSV (rows aren't
        positionally guaranteed on disk), blank (not an error) when the needed
        dataset is empty. Output_Path/Software_Trigger_Delay Consolidate-conflict
        checking lives in `Hdf5WriterV1_1` itself (write-time, not migration-time) —
        it applies uniformly whether the model came from a migration or was authored
        directly as v1.1, per the manifest's "Write ... including v1.0→v1.1 migration"
        framing.
      - Real on-disk quirk found and handled: `Watts_To_Volts_Params` is stored
        bracket-wrapped (`"[1,102.57,...]"`) in every real fixture, unlike
        `Volts_To_Watts_Params`'s plain `"50.5,107.5"` — the manifest's own prose
        example omitted the brackets. The CSV-parsing helper strips them
        unconditionally (a no-op for the unbracketed case).
      - `test_v1_1_adapter.py` (new, 16 tests): the full `docs/contributing.md`
        7-test template plus the extra hard-error tests this plan called for
        (`test_v1_to_v1_1_output_path_disagreement_raises`,
        `..._software_trigger_delay_disagreement_raises`,
        `..._unrecognized_algorithm_type_raises` for both ClearBox and Light_Source),
        plus dedicated tests for the constant-reorder and missing-data-writes-blank
        backward rules, plus a cross-wiring guard in `test_v1_1_roundtrip` (deliberately
        different ClearBox/Light_Source `power_characterization` values in one object).
        Dispatcher tests use the real `"1.1"` registry entry directly — no
        monkeypatching needed, unlike the mock's temporary injection.
      - Fixed alongside this work: `python/tests/test_adapter_migration.py`'s
        pre-existing `MockV1_1Reader`/`MockV1_1Writer` depended on
        `capabilities.v1_0.hdf5.Hdf5AdapterV1_0`/`.writer.Hdf5WriterV1_0` for
        "unchanged" subcomponents — violated the same independence guardrail. Made
        fully self-contained; all 8 of that file's tests still pass unchanged.
      - **Verification:** 376/376 Python tests pass (360 pre-existing + 16 new),
        zero regressions.
- [x] **Rust**: `rust/src/capabilities/v1_1/{file,layout,hdf5,writer}.rs` + registry
      entries in `reader.rs`, `writer.rs`, `capabilities/mod.rs` (now genuine registry
      inserts, not new `match` arms, per `DISPATCH_REGISTRY_PLAN.md`). **Done
      (2026-08-31).**
      - Fully self-contained per the second guardrail — confirmed by
        `rust/tests/version_adapter_isolation_test.rs`'s
        `v1_1_adapter_does_not_reference_other_versions_if_present` test, which was a
        no-op skip before this and is now genuinely active and passing.
        `MachineConfigFileV1_1` added to `capabilities/mod.rs`'s `OpenRegistry`/
        `CreateRegistry` and `lib.rs`'s curated re-export list, alongside V1_0.
      - `migrate_v1_to_v1_1`/`migrate_v1_1_to_v1` live in `capabilities/v1_1/hdf5.rs`,
        matching Python's placement. Same derivation rules as Python (Consolidate check
        at write time in `Hdf5WriterV1_1`, not migration time; bracket-stripping for
        `Watts_To_Volts_Params`; backward re-sort-by-name for
        `Derivation_Equation_Constants`; blank-not-error on missing data).
      - Error variants added to `rust/src/error.rs` ahead of the adapter work:
        `MachineConfigError::ConsolidateConflict { field, detail }` (struct-style,
        matching `UnitMismatch`) and `UnrecognizedAlgorithmType(String)` (tuple-style,
        matching `UnsupportedVersion`), per the manifest's resolved error-naming
        decisions.
      - **Real fixture-generation bug found and fixed (2026-08-31), not a design
        divergence.** `fixtures/reference_config_v1_1.h5`'s
        `Power_Characterization/Derivation_Equation_Constants` had an `H5T_CSET_ASCII`
        `name` field, while `SynchronousSensor`'s own compound dataset in the *same
        file* is `H5T_CSET_UTF8` (confirmed via `h5py`'s low-level `get_cset()`: 0 vs
        1) — despite the manifest's explicit intent that both share the identical
        convention. Root-caused to `file_testing/build_v1_1_review_file.py`'s
        `EQ_CONST_DTYPE = np.dtype([("name", "S64"), ...])` — a plain numpy `S64`
        dtype defaults to ASCII cset in the HDF5 type h5py derives from it, unlike
        `h5py.string_dtype(encoding="utf-8", length=64)`, which the real v1.0/v1.1
        writers both correctly use for this exact dataset. Confirmed the real
        writers were never wrong: Python's `capabilities/v1_1/hdf5.py` has the
        identical UTF-8 dtype as `v1_0/hdf5.py`'s SynchronousSensor constant, and
        `fixtures/reference_config_v1_1_migrated.h5` (generated by the real
        migration path, not this script) was already correctly UTF-8. Only the
        standalone review-fixture script had the bug. **Fixed at the source**: the
        script now uses the correct UTF-8 dtype (with a comment explaining why a
        plain `S64` is the wrong choice here); `fixtures/reference_config_v1_1.h5`
        regenerated and re-promoted — verified byte-for-byte identical to the old
        one in every parsed value (only the charset metadata changed). Rust's
        initial workaround (a second `RawPowerEquationConstant`/`FixedAscii<64>` row
        type, needed only because `hdf5-metno` enforces charset strictly at the
        compound-member level, unlike the other 4 languages' bindings) was then
        **removed** — `RawEquationConstant`/`FixedUnicode<64>` is now shared by both
        `Power_Characterization` and `SynchronousSensor`, matching original intent.
        All 176 Rust tests and all 378 Python tests still pass after both the
        fixture fix and the Rust simplification.
      - New test file `rust/tests/v1_1_adapter_test.rs` — the same 16 tests as Python's
        suite, using the shared `fixtures/reference_config_v1_1.h5` and
        `reference_config_v1_1_migrated.h5` fixtures (no per-language regeneration).
      - Fixed an existing test that would have gone flaky once `"1.1"` was added:
        `capabilities::mod::tests::supported_file_versions_reflects_production_registry`
        asserted exact `HashMap`-iteration-order equality against a single-entry
        registry — now sorts before comparing.
      - **Verification:** 176/176 Rust tests pass (was 160), zero warnings, zero
        regressions. Independently re-verified (not just trusting the report): reran
        `cargo build`/`cargo test` myself, confirmed zero real `capabilities::v1_0`
        references in the new module (only doc-comment mentions and an unrelated local
        variable name), and independently confirmed the ASCII/UTF-8 fixture finding via
        a separate `h5py` inspection before accepting it.
- [x] **C++**: `cpp/include/machine_config/capabilities/v1_1/{file,layout,hdf5,writer}.hpp`
      + registry entries. **Done (2026-08-31).**
      - Fully self-contained (header-only, all `inline`, matching v1_0's style) —
        confirmed by `cpp/tests/test_version_adapter_isolation.cpp`, which was a no-op
        skip before this and is now genuinely active (14/14 assertions passing).
        Duplicated, not shared: not just the read/write helpers but also
        `v1_0/compound_types.hpp`'s `EquationConstantRow`/`CalibrationPointRow` —
        those live at **global scope** (required by `HIGHFIVE_REGISTER_TYPE`'s
        macro expansion), so reusing the same names for v1.1's copy would have been
        a redefinition/ODR error the moment any translation unit included both
        versions (which `reader.hpp`/`writer.hpp`/`capabilities/file.hpp` all do,
        by registry necessity). Named the v1.1 copies `V1_1EquationConstantRow`/
        `V1_1CalibrationPointRow` instead — same fix already applied once to
        `cpp/tests/mock_v1_1.hpp`'s `MockV1_1EquationConstantRow`/
        `MockV1_1CalibrationPointRow` for the identical reason.
      - Registries wired in `reader.hpp`, `writer.hpp`, `capabilities/file.hpp`.
      - Real, correctly-UTF-8 `Power_Characterization` constants confirmed working
        with zero charset workaround — the fixture bug from Rust's Phase 1 was
        already fixed at the source before this work started, and the new
        `V1_1EquationConstantRow` compound type declares `HighFive::CharacterSet::Utf8`
        matching `v1_0/compound_types.hpp`'s existing convention directly, no
        divergence needed.
      - Backward-migration CSV float formatting uses `std::setprecision(17)`
        (exact double round-trip guarantee) rather than a shortest-round-trip
        algorithm like Python's/Rust's default formatting — verified against the
        real fixture's non-terminating-decimal `Watts_To_Volts_Params` values,
        which lost precision at `setprecision(15)` and round-tripped exactly at 17.
      - One real, minor build-config fix needed: `/bigobj` added to the
        `machine_config` interface target's MSVC compile options
        (`cpp/CMakeLists.txt`) — a single translation unit now pulling in both
        versions' adapter templates (e.g. the CLI's umbrella header) exceeded
        MSVC's default object-file section limit. No behavior change.
      - New test file `cpp/tests/test_v1_1_adapter.cpp` (16 test cases), registered
        in `cpp/tests/CMakeLists.txt`, using the same shared
        `fixtures/reference_config_v1_1.h5`/`reference_config_v1_1_migrated.h5`
        fixtures as Python/Rust.
      - **Real bug caught during independent verification, not by the agent:** the
        agent's own report claimed C++'s v1.1 facade "builds its own JSON conversion
        fresh," implying the Phase 0 JSON follow-up didn't apply to C++ — checking
        this directly showed the opposite. `Hdf5AdapterV1_1::toJson()` calls
        `nlohmann::json j = parse();`, which invokes `models.hpp`'s shared,
        version-agnostic ADL `to_json(nlohmann::json&, const MachineConfig&)` —
        legitimately shared infrastructure, not a v1_0-specific file, so no
        independence-guardrail issue — but that function's `ClearBox`/`LightSource`
        overloads had never been updated with `firmware_version`/
        `power_characterization` (the exact Phase 0 follow-up), and
        `PowerCharacterization` had no `to_json`/`from_json` overloads at all. Net
        effect: any `MachineConfigReader(v1_1_file).toJson()` call was silently
        dropping both new fields. Fixed directly in `models.hpp`: added
        `PowerCharacterization`'s `to_json`/`from_json` (same shape as
        `SynchronousSensor`'s, reusing `EquationConstant`/`CalibrationPoint`'s
        existing overloads), and added the two fields to `ClearBox`'s/`LightSource`'s
        `to_json`/`from_json`, omitted when absent (matching the byte-identical-v1.0-
        output convention used everywhere else in this file, e.g.
        `synchronous_sensors`). Verified the fix directly against a real fixture via
        the CLI's `export-json` subcommand — `firmware_version`/`power_characterization`
        now appear correctly, not just via the test suite passing.
      - **Verification:** 723 assertions / 130 test cases pass (was 600/114) both
        before and after the JSON fix (the fix closed a real gap the existing tests
        didn't happen to exercise, so the pass count didn't change — a reminder that
        "tests pass" isn't the same as "no bugs remain"), zero regressions, full
        solution build (including CLI/examples) succeeds. Independently re-verified
        myself throughout: reran the full build and test suite, confirmed zero
        `v1_0` references anywhere in the new files (not even in comments), reran
        the guard test in isolation (14/14), confirmed the `/bigobj` and
        `setprecision(17)` choices directly in the diff rather than only trusting
        the agent's description of them, and found + fixed the JSON gap above by
        checking the agent's specific claim instead of accepting it at face value.
- [x] **Go**: `go/capabilities/v1_1/` (`file.go` plus `layout/`, `hdf5/` packages, per
      `docs/contributing.md`'s table) + registry entries. **Done (2026-09-01)**,
      mirroring `v1_0`'s real package split exactly (not `internal/mockv1_1`'s flat
      shape, which is test-only).
      - Independence guardrail confirmed by `go/version_adapter_isolation_test.go`
        (`go/parser`, `ImportsOnly` mode — real imports only, doc-comment mentions
        don't trip it): zero real imports of `capabilities/v1_0/*` anywhere under
        `capabilities/v1_1/`.
      - **Go-specific nuance, correctly identified and applied**: not everything
        needed duplicating. `internal/h5c` (the cgo/HDF5 C-binding layer — its
        `EquationConstantRow`/`CreateEquationConstantsDataset`/
        `ReadEquationConstantsDataset`, already hardcoding `H5T_CSET_UTF8`) and
        `internal/models` are legitimately shared, version-agnostic infrastructure,
        not v1_0-owned — reused directly, avoiding both the compound-type ODR
        collision Rust/C++ had to route around and any charset concern. Only
        `v1_0/hdf5/helpers.go`'s scalar attribute-read helpers (genuinely v1_0-owned)
        were duplicated fresh into `v1_1/hdf5/helpers.go`.
      - `v1_1.File`'s facade correctly implements only Go's existing narrower
        `capabilities.File` interface (missing several setters compared to the other
        4 languages' facades, a known separate pre-existing gap tracked in
        `PARITY_AUDIT.md`) — not backfilled here, correctly out of scope.
      - **Real, verified JSON-serialization check** (asked for explicitly, given the
        C++ lesson above): `encoding/json` is reflection/struct-tag driven in Go, not
        hand-written per-field functions, so the `omitempty`-tagged fields Phase 0
        already added just work — but this was *verified*, not assumed: a test
        `json.Marshal`s a real parsed `MachineConfig` and asserts
        `firmware_version`/`power_characterization` keys are actually present in the
        output, independently re-confirmed by me reading that test's body directly
        rather than trusting the report alone.
      - **Real cross-language inconsistency caught in review, in the manifest
        itself, not in Go's code originally**: the Consolidate-conflict message
        manifest bullets for Go and C++ used parentheses
        (`"trains disagree (...)"`), contradicting the manifest's own stated
        cross-language standard (em-dash, stated one paragraph above) and the actual
        Rust/Python implementations. C++'s implementation caught this itself and
        followed the stated standard over the specific bullet; Go's implementation
        followed the flawed bullet literally, producing a real, live inconsistency
        between Go's error message and the other 4 languages'. Fixed in both
        `docs/migrations/v1_0_to_v1_1.md` (corrected bullet, with a note explaining what
        happened) and `go/capabilities/v1_1/hdf5/writer.go` (both `Output_Path`/
        `Software_Trigger_Delay` message sites) — all 5 languages now agree exactly
        on the standardized message form.
      - New test file `go/v1_1_adapter_test.go` (16 tests, package
        `machineconfig_test`), using the shared `fixtures/reference_config_v1_1.h5`/
        `reference_config_v1_1_migrated.h5` fixtures.
      - Fixed an existing test that would have gone stale: `capabilities/file_test.go`'s
        `TestSupportedFileVersionsReflectsRegistry` had `["1.0"]` hardcoded.
      - **Verification:** all packages `ok`, 108 tests total (was 92: 64+21+7 →
        80+21+7), zero regressions. Independently re-verified myself: reran
        `go build ./...`/`go vet ./...`/`go test ./...` (with `CGO_ENABLED=1` and
        MSYS2 mingw64 on `PATH`) after fixing the message-format bug above, confirmed
        zero real `capabilities/v1_0` imports, and read the JSON-verification test's
        actual body rather than trusting the summary of it.
- [x] **Node.js**: `nodejs/src/capabilities/v1_1/{file,layout,hdf5,writer}.ts` + registry
      entries. **Done (2026-09-01)** — the final language for Phase 1.
      - Fully self-contained — confirmed by `nodejs/tests/versionAdapterIsolation.test.ts`
        (0 real `v1_0` references in any new file, verified directly by me, not just the
        agent's report). Correctly avoided the message-format mistake Go's implementation
        made (parentheses instead of the manifest's stated em-dash standard) — Node's
        `Consolidate conflict` and `Unrecognized Algorithm_Type` messages match the other
        4 languages exactly on the first attempt.
      - `create()` built via `migrateV1ToV1_1` over a mock v1.0 config, matching the
        pattern established in Python/Rust/C++/Go (reuse the migration function rather
        than a second, independent mock-data path).
      - **JSON-serialization verified directly** (asked for explicitly, given the C++
        lesson): Node's `toJson()` is `JSON.stringify(config, ...)` directly on the parsed
        object with no separate hand-written serialization layer, so the optional
        `firmware_version`/`power_characterization` TypeScript fields flow through
        automatically — confirmed by a dedicated test asserting the real fixture's
        `.toJson()` output string actually contains both field names, independently
        re-read by me to confirm it tests what it claims.
      - New test file `nodejs/tests/v1_1Adapter.test.ts` (19 tests: the 16 mirrored from
        Python plus the JSON check and 2 registry/interface checks), using the shared
        `fixtures/reference_config_v1_1.h5`/`reference_config_v1_1_migrated.h5` fixtures.
      - **Verification:** 211/211 tests pass (was 192, 8 files up from 7), zero
        regressions, `npx tsc --noEmit` clean. Independently re-verified myself: reran
        typecheck and full suite, confirmed zero real `v1_0` references, reran the guard
        test standalone, read the JSON test's actual body, and confirmed the
        bracket-stripping CSV parser (`replace(/[[\]]/g, "")`) is genuinely present.

**Phase 1 is now complete across all 5 languages (2026-09-01).** Every language's v1.1
adapter is self-contained (enforced by that language's version-adapter-isolation guard
test), implements all 5 manifest changes, and has a matching ~16-19 test suite using the
same two shared fixtures. Two real bugs were caught and fixed during this phase that
weren't specific to any one language's adapter logic: the `Power_Characterization`
fixture's ASCII/UTF-8 charset bug (root-caused to `build_v1_1_review_file.py`, not any
adapter), and C++'s `models.hpp` JSON serialization silently dropping the two new fields
(a real, live bug, not a documentation gap). A cross-language message-format
inconsistency in the manifest itself (Go/C++ bullets contradicting the stated em-dash
standard) was also caught and corrected. Remaining before this plan is fully closed out:
just the Verification section's `cross_check.py` run across all 5 languages with the
v1.1 fixtures — the manifest promotion, `docs/contributing.md` category-table update, and
CHANGELOG format decision are done (2026-09-01), see the Verification section below.

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
      verbatim; Change 5 has nothing to reverse). Changes 3/4 now have their rule
      written (manifest's "Backward migration" subsections, resolved 2026-08-31) —
      cover all five changes, including the two new missing-data cases (empty
      `Derivation_Equation_Constants` / empty `Characterization_Points` → blank
      params string, not an error) and the Change 3 constant re-sort-by-name
      requirement before joining as CSV.
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

## Phase 2 — clean-up: writer-side fallback derivation, remove migrate functions

**Status: Python done (2026-09-01); Rust, C++, Go, Node.js not started.** Raised after
Phase 1 shipped, during a review of why separate `migrate_v1_to_v1_1`/`migrate_v1_1_to_v1`
functions exist at all given the StableModel-plus-adapters architecture. Confirmed as a
real, demonstrable gap, not a style preference — see "Root cause" below.

**Design superseded once, before any implementation started — read this first.** The
original draft of this section (below, mostly unchanged) proposed *readers* deriving the
other shape eagerly, at parse time ("Option A"). Before any language implemented that,
review surfaced a real problem with it and the design changed to what's actually built
today ("Option C": *writers* derive, only as a fallback, and neither reader changes at
all). See "Design decision: writer-side fallback (Option C), not reader-side eager
derivation (Option A)" below for the two reasons this changed — it materially affects
where the diff lands in each language, so implementers should read that section, not just
skim past it as history.

### The acceptance criterion this phase exists to satisfy

Stated by the user (2026-09-01), and adopted here verbatim as the correctness spec for
this phase's tests: *"If you ask the API for a V1.1 file and a V1.0 file's data was
loaded, then you get a V1.1 file as much as there is data to present and nothing more.
Similarly if you ask the API for a V1.0 file and a V1.1 file's data was loaded, you get
a V1.0 file as much as there is data to present and nothing more."* Loss when moving
between a richer and a simpler shape is expected and acceptable — the user is expected
to understand this and fill gaps back in if needed — but it must be the *only* source of
loss. Loss caused by the migration mechanism itself (today's bug — see below) is not
acceptable.

**This must not be specific to File_Version 1.0/1.1.** Whatever mechanism fixes this has
to keep working, unmodified, when a hypothetical v1.2/v1.3 arrives — no new per-version-
pair function should ever be needed just because the version count went up. See "Why
this generalizes" below for how the design satisfies that.

### Root cause, confirmed against real code (not assumed)

Every language's StableModel (`models.*`) keeps **both** representations of this concept
as permanent, independent fields — this was already true before Phase 1 and is not
changing: `ClearBox.volts_to_watts_algorithm`/`volts_to_watts_params` and
`LightSource.watts_to_volts_algorithm`/`watts_to_volts_params` (the flat, V1.0 shape)
sit alongside `ClearBox.power_characterization`/`LightSource.power_characterization`
(the structured, V1.1 shape) — e.g. `python/src/machine_config/models.py:145-146,158-160`
and `:286-290`.

The bug: **only one reader ever populates the field it doesn't own natively.** Confirmed
by reading the real parse methods, not the migrate functions:
- `Hdf5AdapterV1_0.parse()` populates the flat fields; `power_characterization` is left
  at its default (`None`) — v1.0 files have no on-disk source for it, correctly.
- `Hdf5AdapterV1_1.parse()` populates `power_characterization`, but **explicitly sets
  the flat fields to `None`** (confirmed: `python/src/machine_config/capabilities/v1_1/
  hdf5.py:567-568`, `volts_to_watts_algorithm=None, volts_to_watts_params=None` — not an
  oversight, a deliberate "no on-disk source in v1.1" comment, correct under the old
  migrate-function model but wrong under the acceptance criterion above).

Net effect, demonstrable today: read a natively-authored v1.1 file (its
`power_characterization` fully populated), pass the resulting `MachineConfig` straight
to `Hdf5WriterV1_0` — no migration function involved, just the ordinary public
`parse()`/`write()` path the user described — and `Volts_To_Watts_Algorithm`/
`Volts_To_Watts_Params` come out blank on disk, even though the data to produce them
(`power_characterization.algorithm_type`/`.derivation_equation_constants`) was fully
present in memory. That's silent, mechanism-caused loss, not the expected
richer-to-simpler kind — exactly what the acceptance criterion forbids.

### Design decision: writer-side fallback (Option C), not reader-side eager derivation (Option A)

**What's actually built (Option C), stated first:** neither reader changes at all —
`Hdf5AdapterV1_0.parse()`/`Hdf5AdapterV1_1.parse()` are byte-identical to what Phase 1
shipped. Instead, each **writer** writes its own native field if present, and only
derives from the *other* shape as a fallback when its own native field is `None`:
`Hdf5WriterV1_0` writes `volts_to_watts_*`/`watts_to_volts_*` directly if set, else
derives them from `power_characterization`; `Hdf5WriterV1_1` writes
`power_characterization` directly if set, else derives it from the flat fields. A
present native value always wins — the fallback only ever fires when there is nothing
native to write.

**Why not the reader (the original draft below described exactly that — Option A),
found during review before any language implemented it:**

1. **Option A only works for models that came from an actual `parse()` call.** Anything
   built without going through a real read — a hand-built model, a test mock, `create()`
   (which builds a mock v1.0 config purely in memory, with no file involved at all) —
   would silently skip the derivation, since there's no reader in that path to have run
   it. Option C's fallback lives at the one place every path to disk must go through
   regardless of how the `MachineConfig` was produced: the writer. Concretely, this
   let `create()` shrink to "build a plain v1.0-shaped mock, hand it to
   `Hdf5WriterV1_1`" — no special-cased manual derivation call needed, unlike what
   Option A would have required there (see the Python write-up below for the actual
   before/after).
2. **A separate, real objection surfaced during review**: even though neither design
   creates the guard-test-forbidden kind of coupling (no adapter imports another
   version's code either way — confirmed for both), Option A meant *both* readers
   reference StableModel concepts whose only on-disk source is the other version, which
   read as unwanted mutual awareness. Option C doesn't reduce the *total* amount of
   cross-shape knowledge (the same four shared-module functions get called from
   somewhere in each version's adapter pair either way) — but it confines all of it to
   the writers, leaving both readers exactly as simple as Phase 1 left them: "read only
   your own on-disk shape."

The one thing Option A had that Option C doesn't: a model is fully bi-populated
immediately after *any* `parse()` call, whether or not you ever call `write()`. Nothing
in the acceptance criterion above requires that — it's stated entirely in terms of
`write()` behavior — so this was judged not worth the two costs above.

**Everything below this point in "Design" describes Option A's mechanics and was written
before this decision — kept for the parts that are still accurate (the shared-module
placement, the never-raises rule, the blank-not-hardcoded rule, generalization to future
versions), but wherever it says "reader," the shipped behavior is "writer, as a
fallback." The per-language checklist and test list further down have already been
corrected for Option C — trust those over this paragraph's stale specifics.**

- **Lenient, never throws**: if the flat `Algorithm_Type` isn't `LINEAR`/`POLYNOMIAL`,
  don't error — copy `algorithm_type` verbatim (it's an open string on the StableModel
  regardless, per the already-resolved enum-vs-string blocker), split `Params` into
  positionally-named constants (`"0"`, `"1"`, `"2"`, ... instead of `b`/`a`/`c0`/`c1`),
  and leave `algorithm_equation` blank (nothing to mechanically generate for an unknown
  form). Round trip stays lossless even for unrecognized types: writing back to v1.0
  re-sorts by the numeric suffix and reproduces the original CSV order exactly, the same
  way the already-existing name-based sort does for `b`/`a`/`c0`/`c1`.
- **`UnrecognizedAlgorithmType` is removed**, all 5 languages, once this lands (confirm
  no other call sites first — it was introduced solely for this path in Phase 1). This
  reverses that Phase 1 blocker decision; see "Why the hard error goes away" below.
- **`Input_Type` and `Units_Derived_Quantity` are left blank (`None`) on derivation**,
  not hardcoded (`"0-10 V"` / `"Volts"` / `"Watts"`) — confirmed by the user (2026-09-01):
  a hardcoded guess is a special case that would need re-litigating for every future
  version; blank is honest about "not derivable from what's on disk" and consistent with
  the "blank over ambiguity" rule the backward-migration functions already use.
- **Change 1 (Consolidate) is unaffected by this phase — do not touch it.** It was
  already correctly writer-intrinsic (the hard error lives in `Hdf5WriterV1_1` at write
  time, checking per-train values that are still independent fields on the StableModel —
  there is no shape conversion here, so nothing needs to move). It stays a genuine hard
  error, not blank-on-ambiguity, because it's a *conflict* (two present, disagreeing
  values), not *incompleteness* (a value that's merely absent) — see "Why the hard error
  goes away" for why that distinction, not "always vs. never error," is the actual rule.

### Why keep both StableModel fields, instead of collapsing to one

The obvious-looking alternative is to delete `volts_to_watts_algorithm`/`_params` and
`watts_to_volts_algorithm`/`_params` outright, leaving `power_characterization` as the
*only* representation — every version's reader/writer would convert at its own boundary
and nothing else in the system would ever see the flat shape. Considered and rejected,
for two concrete reasons, not just inertia:

1. **Byte-fidelity regression on the plain v1.0 path.** Today, a pure v1.0-read →
   v1.0-write round trip (v1.1 never involved) is exact — `Volts_To_Watts_Params`'s raw
   string passes through completely untouched, nothing ever parses or reformats it. If
   that field stops existing and V1.0 always round-trips through `PowerCharacterization`
   instead, *every* v1.0 file becomes subject to floating-point reformatting fidelity —
   the exact problem C++'s Phase 1 work needed `setprecision(17)` to solve, but as a
   concern for the ordinary, v1.1-unrelated v1.0 path, in all 5 languages, not just the
   v1.1-conversion path where it's unavoidable anyway. `test_v1_unaffected` (Testing
   section above) exists specifically to guarantee zero behavior change to plain v1.0
   handling — collapsing to one field would put that guarantee at risk for no gain.
2. **Bigger blast radius for no functional benefit.** The flat fields predate v1.1
   entirely; removing them means changing the ClearBox/LightSource shape itself across
   all 5 languages — every construction site, builder, and JSON schema entry Phase 0
   already wired up for these (pre-existing) fields, not just new v1.1 code.

Keeping both fields, converted through one shared module, already delivers the
abstraction that matters: `power_characterization` is *the* canonical representation of
this concept — every consumer that wants "what does this ClearBox's power derivation
look like" should read that field, not the flat one — and both versions' readers reach
it through the exact same shared conversion functions rather than two independent
implementations. The flat fields remain on the model purely as v1.0's wire format,
always kept in sync automatically; they are not a second, competing source of truth a
caller needs to reconcile by hand.

### Why the hard error goes away here but not for Consolidate

Two different situations were being treated the same way in Phase 1, and shouldn't be:

- **Incompleteness** (Algorithm_Type unrecognized, Input_Type/Units_Derived_Quantity not
  derivable): there's no contradiction, just less information than the richer shape can
  hold. Best-effort-and-blank is correct — this is what changes in this phase.
- **Conflict** (Consolidate: two trains' `Output_Path` values both present and
  disagreeing): there is no "as much data as there is" answer, because both values are
  equally present and mutually exclusive — silently picking one would be a real
  correctness bug, not a documented limitation. This stays a hard error and is out of
  scope for this phase.

### Why this generalizes to future versions (not a V1.0/1.1 special case)

The derivation functions are keyed by **on-disk shape**, not by version pair, and they
operate purely on StableModel types (`PowerCharacterization`, `EquationConstant`,
`CalibrationPoint`) with no HDF5 I/O and no knowledge of "1.0" or "1.1" as strings. There
are exactly two shapes today — flat (opaque string + CSV string) and structured
(`PowerCharacterization`) — so exactly one pair of conversion functions per StableModel
field pair (Change 3's ClearBox pair, Change 4's LightSource pair; 4 functions total).
If a future version reuses the flat shape, its reader calls the same functions with zero
new code. If a future version introduces a **third** shape, that's exactly one new pair
of functions for that shape — never a new function per *other-version* pair, which is
what keeps this from becoming O(N²) as versions accumulate. A version whose on-disk
format matches the structured shape directly (as v1.1's does) needs no conversion
function at all, same as today.

**Structural requirement this implies:** the shared module must **not** live inside
`capabilities/v1_0/` or `capabilities/v1_1/` — either placement would make one version a
runtime dependency of the other the moment both readers import it, exactly the coupling
the version-adapter-isolation guardrail (Phase 1) exists to prevent. It belongs at the
same neutral level as `models.*` itself, since it only transforms StableModel types:

- Python: `python/src/machine_config/power_characterization.py`
- Rust: `rust/src/power_characterization.rs` (new `mod power_characterization;` in
  `lib.rs`, sibling to `mod models;`)
- Node.js: `nodejs/src/powerCharacterization.ts`
- Go: `go/internal/models/power_characterization.go` (colocated with the already-shared
  `internal/models` package — confirm during implementation whether `capabilities/v1_0`
  and `capabilities/v1_1` already import `internal/models` directly; if not, add that
  import rather than duplicating)
- C++: `cpp/include/machine_config/power_characterization.hpp` (header-only, sibling to
  `models.hpp`)

Each language's version-adapter-isolation guard test must be extended to treat this new
file the same way it already (implicitly) treats `models.*` — as legitimate shared
infrastructure importable from any version, not a violation. Add an explicit case to
each guard test asserting this (don't just rely on the scan happening to pass).

### Function contracts (language-agnostic; adapt naming per language's convention)

```
forward_power_characterization(algorithm: str|None, params: str|None, shape: "coefficients"|"points") -> PowerCharacterization|None
    # None if both inputs are None (nothing to derive).
    # "coefficients" (Change 3 / ClearBox): split params by comma -> floats;
    #   name them b/a (LINEAR), c0/c1/... (POLYNOMIAL), or 0/1/2/... (anything else);
    #   generate algorithm_equation only for LINEAR/POLYNOMIAL, else None.
    # "points" (Change 4 / LightSource): split params by comma -> floats, pair
    #   consecutively into (input_value, output_value); LINEAR gets a generated
    #   equation, everything else (including POLYNOMIAL, unchanged from Phase 1's
    #   existing rule) leaves it None.
    # input_type, units_derived_quantity: always None.
    # Never raises.

backward_flat_fields(pc: PowerCharacterization|None, shape: "coefficients"|"points") -> (algorithm_type: str|None, params: str|None)
    # None/None if pc is None.
    # "coefficients": re-sort derivation_equation_constants by name (existing
    #   _sorted_constants_by_name rule, extended to sort "0","1","2",... numerically
    #   alongside b/a/c0/c1/...) before joining as CSV; blank ("") if empty, not None
    #   and not an error — matches Phase 1's existing "absence, not ambiguity" rule.
    # "points": flatten characterization_points pairwise, already-correct on-disk
    #   order, no sort needed (unchanged from Phase 1).
```

This is Phase 1's existing `_forward_power_characterization_from_volts_to_watts`/
`_watts_to_volts`, `_backward_volts_to_watts_params`/`_watts_to_volts_params`,
`_sorted_constants_by_name`, `_generate_power_equation`, and `_parse_csv_floats`
(`python/src/machine_config/capabilities/v1_1/hdf5.py`, and each other language's
equivalent) **relocated** to the new shared module and **modified**: drop the
`raise`/`throw` on unrecognized `algorithm_type`, drop the hardcoded `input_type`/
`units_derived_quantity` values, extend the sort key to handle positional fallback
names. No new algorithm — this is moving and lightly editing code that already exists
and is already tested, not writing new derivation logic from scratch.

### Per-language change checklist (Option C — corrected from the Option A draft above)

For each of Python, Rust, C++, Go, Node.js:

1. Create the new shared module at the path listed above; move the 5-ish helper
   functions into it (adjusted per the contracts above, renamed to
   `forward_power_characterization_coefficients`/`_points` and
   `backward_flat_fields_coefficients`/`_points` per Python's implementation — see
   Python's write-up below for exact signatures); delete them from
   `capabilities/v1_1/hdf5.*`.
2. **Both readers: no changes.** `capabilities/v1_0/hdf5.*` and `capabilities/v1_1/hdf5.*`
   stay byte-identical to what Phase 1 shipped — this is the point of Option C.
3. `capabilities/v1_0/writer.py` (and equivalent): in the ClearBox/LightSource write
   methods, write the native flat field if present (`is not None`); if both are `None`
   *and* `power_characterization` is present, derive via
   `backward_flat_fields_coefficients`/`_points` and write that instead.
4. `capabilities/v1_1/writer.py` (and equivalent): symmetric — write native
   `power_characterization` if present; if `None` and either flat field is present,
   derive via `forward_power_characterization_coefficients`/`_points` and write that
   instead.
5. Delete `migrate_v1_to_v1_1`/`migrate_v1_1_to_v1` entirely from
   `capabilities/v1_1/hdf5.*` (or wherever each language placed them).
6. `capabilities/v1_1/file.py`'s `create()` (and each language's equivalent): simplify to
   building a plain v1.0-shaped mock config and handing it straight to
   `Hdf5WriterV1_1`, setting `meta.file_version = "1.1"` first so the on-disk attribute
   is correct — no explicit derivation call needed at all; the writer's own fallback
   (step 4) does it automatically, the same as it would for any other v1.0-sourced
   model. This is a bigger simplification than Option A's version of this step, which
   still needed `create()` to call the forward-derivation function manually (see "Why
   not the reader," reason 1, above).
7. Update every existing test that imports or calls `migrate_v1_to_v1_1`/
   `migrate_v1_1_to_v1` directly (`test_adapter_migration.py`-style tests and each
   language's `v1_1_adapter_test.*`) to instead exercise the real `parse()`/`write()`
   path via the writer's new `target_version` parameter (step 9 below) — this is also a
   genuine strengthening, since it now tests the same code path a real caller uses, not
   a function no public API entry point actually calls the way the test does.
8. Regenerate `fixtures/reference_config_v1_1_migrated.h5` — its `Input_Type`/
   `Units_Derived_Quantity` values change from the old hardcoded placeholders to blank.
   Re-verify structure the same way Phase 0 did (spot-check `File_Version`, confirm the
   two Power_Characterization groups' new blank fields, confirm everything else
   unchanged) before re-promoting. **Done (2026-09-01)** — regenerated via
   `MachineConfigReader(...).parse()` → `MachineConfigWriter(config,
   target_version="1.1").write(...)`, the new writer-API replacement for the deleted
   migrate function, against the same source fixture as before
   (`reference_config_opcua_synchronous_sensors.h5`). Verified: `File_Version` `1.1`,
   both `Power_Characterization` groups' `Input_Type`/`Units_Derived_Quantity` blank,
   ClearBox/Light_Source's constants-vs-points population correctly inverted (2/0 vs
   0/5 rows), everything else unchanged.
9. Update `docs/migrations/v1_0_to_v1_1.md`'s Change 3/4 sections (the "Addition" rows for
   `Algorithm_Equation`/`Input_Type`/`Units_Derived_Quantity`, and the "Unrecognized
   Algorithm_Type during migration" bullets) to describe blank-not-hardcoded and
   best-effort-not-error. This is a real behavior change to already-shipped
   documentation, not just an implementation-notes update — needs the same care as the
   original manifest content. **Done (2026-09-01)**, alongside the manifest's promotion
   to `docs/migrations/` — see that file's "superseded" notes under Changes 3–4.
10. Add the optional explicit target-version parameter described below to the writer
    facade.

### Writer: optional explicit target-version parameter

Once both writers can derive from either shape (steps 3–4 above), "upgrade" and
"downgrade" stop being distinct operations — they're just "read, then write somewhere
that resolves to a different adapter," exactly the `parse()`/`write()` workflow
originally proposed at the start of this discussion. The one rough edge left: the writer
facade currently decides *which* adapter to use purely by inspecting
`config.meta.file_version` (confirmed: `python/src/machine_config/writer.py:33`,
`version = (config.meta.file_version or "1.0").strip() or "1.0"`, same shape in the
other 4 languages per `DISPATCH_REGISTRY_PLAN.md`) — so today, writing a v1.0-sourced
model as v1.1 requires **mutating the config the reader handed back**
(`config.meta.file_version = "1.1"`) just to express the caller's intent, before
construction. That overloads one field for two different things — "what version is
this data" (a fact the reader set) and "what version do I want written" (a request the
caller is making) — and requires modifying an object that logically shouldn't need to
change just to make a write call.

Fix: give the writer facade an optional parameter that overrides the inferred version,
defaulting to today's behavior when omitted, so nothing existing breaks:

```python
class MachineConfigWriter:
    def __init__(self, config: MachineConfig, target_version: str | None = None) -> None:
        self.config = config
        version = (target_version or config.meta.file_version or "1.0").strip() or "1.0"
        adapter_cls = _ADAPTERS.get(version)
        if adapter_cls is None:
            raise UnsupportedFileVersion(version)
        # Each adapter stamps config.meta.file_version verbatim as the on-disk
        # File_Version attribute — if target_version overrides the adapter choice, the
        # config handed to the adapter must reflect that too, or the file would claim
        # the wrong version on disk. A copy, not a mutation of the caller's object:
        resolved_config = (
            config if version == (config.meta.file_version or "1.0").strip()
            else dataclasses.replace(config, meta=dataclasses.replace(config.meta, file_version=version))
        )
        self._backend = adapter_cls(resolved_config)
        self.file_version = version
```

- No dispatch-mechanism changes needed — `_ADAPTERS`/`_WRITERS` (the real registry from
  `DISPATCH_REGISTRY_PLAN.md`) is looked up by the same key as always; only what decides
  the key changes.
- **Real bug caught while implementing this, not just a hypothetical**: without the
  `resolved_config` copy above, using `target_version` would silently write the *wrong*
  `File_Version` attribute on disk — every adapter's `_write_root_attrs` stamps
  `meta.file_version` verbatim, so overriding *which adapter* runs without also
  correcting what it thinks its own version is produces a file whose on-disk attribute
  contradicts the adapter that wrote it (e.g. `Hdf5WriterV1_1` running but stamping
  `"1.0"`). Caught by writing the test below before declaring this done, not by
  inspection — a reminder to actually run the target-version-override test against the
  written file's on-disk attribute, not just against a re-parsed in-memory object.
- `self.config` stays the caller's original, unmutated object; only the copy handed to
  the backend differs. `write()` itself never mutates anything.
- Per-language equivalents: Rust (`MachineConfigWriter::new(config, target_version:
  Option<&str>)` or a builder-style `.with_target_version(...)`), Go
  (`NewMachineConfigWriter(config, opts ...WriterOption)` or a second parameter,
  matching whatever optional-parameter idiom `DISPATCH_REGISTRY_PLAN.md` already
  established for that language), Node.js (`new MachineConfigWriter(config,
  targetVersion?: string)`), C++ (an optional parameter or overload on
  `MachineConfigWriter`'s constructor). **Check each language's writer for the same
  verbatim-stamping bug** — this is not Python-specific, it follows from any adapter
  design that reads its output version from the config it was handed.

### Tests to add or change

**Corrected for Option C** — the two tests below that assumed reader-side derivation
(`test_v1_0_read_derives_power_characterization`, `test_v1_1_read_derives_flat_fields`)
don't apply anymore, since neither reader changes; replaced by writer-focused
equivalents. Names below match what Python actually implemented
(`python/tests/test_v1_1_adapter.py`, `python/tests/test_power_characterization.py`);
other languages should follow the same names for cross-language consistency, per this
repo's usual convention.

- [x] **`test_power_characterization.py`** (new file, pure unit tests, no HDF5 I/O) —
      the shared module's four public functions tested directly: both-`None`-returns-
      `None`, LINEAR/POLYNOMIAL naming and equation generation, blank
      `input_type`/`units_derived_quantity`, never-raises + positional-fallback-naming
      for unrecognized algorithm types, blank-not-error on empty constants/points,
      re-sort-by-name (including the new numeric positional-fallback case and an
      arbitrary-name alphabetical fallback), and forward-then-backward round-trip
      fidelity for the unrecognized-type case. 24 tests.
- [x] **`test_v1_0_write_never_invokes_fallback_when_native_present`** (new — replaces
      the planned `test_v1_0_roundtrip_unaffected_by_derivation`, made stronger given
      Option C's actual risk surface) — writes a v1.0 mock with a deliberately
      odd-but-valid `Volts_To_Watts_Params` string (`"50.50,107.500"`, which *would*
      change if parsed-and-rejoined through floats) and confirms it survives a write+read
      cycle byte-for-byte. Proves `Hdf5WriterV1_0`'s fallback never fires when the native
      field is present — the real risk this phase adds to the plain v1.0 path, now that
      the writer (not just the reader) has new code on it.
- [x] **`test_roundtrip_v1_0_to_v1_1_to_v1_0`** — the acceptance criterion, made
      concrete: read the v1.0 fixture, write as v1.1 (`target_version="1.1"`), read that
      back, write as v1.0 again, read that back; asserts the final `MachineConfig`
      matches the original for every field v1.0 can represent. Uses the fixture's actual
      `Algorithm_Type` values, not a special-cased LINEAR/POLYNOMIAL-only fixture.
- [x] **`test_roundtrip_v1_1_to_v1_0_to_v1_1`** — mirror direction, starting from the
      natively-authored v1.1 fixture; asserts `algorithm_type` and constant/point values
      survive, and that v1.1-only fields (`firmware_version`, `input_type`,
      `units_derived_quantity`, `characterization_points` on the ClearBox side) come back
      `None`/empty — documented richer-to-simpler loss, not a bug.
- [x] **`test_v1_to_v1_1`** (existing test, rewritten) — no longer calls
      `migrate_v1_to_v1_1` directly; now writes the real v1.0 fixture as v1.1 via
      `target_version` and reads it back, asserting the same per-manifest values as
      before (Consolidate carried through, Addition fields `None`, Removal fields
      `None`, Change 3/4's derived structured data). `test_v1_to_v1_1_writes_and_reads_back`
      (a near-duplicate once both tests require a real write+read) was folded into this
      one rather than kept separate.
- [x] **`test_v1_to_v1_1_unrecognized_algorithm_type_best_effort`** (replaces
      `..._raises`, both the ClearBox and `..._for_light_source` Light_Source variants)
      — writes a v1.0 mock with `Volts_To_Watts_Algorithm`/`Watts_To_Volts_Algorithm` set
      to neither `LINEAR` nor `POLYNOMIAL`, via a real `target_version="1.1"` write;
      asserts no exception, `algorithm_type` copied verbatim, constants/points parsed
      and positionally named, `algorithm_equation is None`, and (ClearBox variant only)
      that writing back to v1.0 reproduces the original CSV string exactly.
- [x] Existing forward-migration assertions expecting `input_type == "0-10 V"` /
      `"Volts"` and `units_derived_quantity == "Watts"` — none needed updating in
      Python's suite; those specific literal-value assertions only ever existed in the
      now-deleted migrate-function tests, not elsewhere.
- [x] **`test_v1_1_to_v1`** (existing test, rewritten) — no longer calls migrate
      functions; now writes the *natively-authored* v1.1 fixture as v1.0 via
      `target_version` (a stronger source than round-tripping a v1.0 fixture up and back
      down, since it exercises the fallback against genuinely-native structured data),
      and confirms the same per-manifest field-survival assertions as before.
      `test_v1_1_to_v1_reorders_constants_by_name`,
      `..._missing_derivation_constants_writes_blank`, and
      `..._missing_characterization_points_writes_blank` were similarly rewritten to
      write-then-read through `target_version="1.0"` instead of calling
      `migrate_v1_1_to_v1` directly, using the new `_mock_v1_1_config` test helper (see
      Python's write-up below) to build v1.1-shaped in-memory data without touching disk.
- [x] `test_v1_to_v1_1_output_path_disagreement_raises` /
      `..._software_trigger_delay_disagreement_raises` — confirmed unaffected, simplified
      slightly (write the v1.0 mock directly via `target_version="1.1"`, no intermediate
      migrate call was ever needed here even before this phase, since Consolidate was
      already writer-intrinsic).
- [x] Per-language version-adapter-isolation guard test — **no change needed for
      Python**, verified rather than assumed: the guard test only flags import strings
      matching a `vX_Y` token, and `machine_config.power_characterization` contains no
      such token, so it structurally cannot false-positive regardless of which versions
      import it. Confirmed by the full suite passing with the guard test included, not
      by inspection alone. Other languages should verify the same is true of their own
      guard test's matching logic before assuming this item is free there too.
- [x] **`test_writer_target_version_overrides_meta`** — read the v1.0 fixture
      (`config.meta.file_version == "1.0"`, untouched), construct the writer with
      `target_version="1.1"`, confirm the written file's on-disk `File_Version` is
      `"1.1"` **and** that `config.meta.file_version` is still `"1.0"` afterward. Mirror
      in the opposite direction starting from the v1.1 fixture. This test is what caught
      the verbatim-stamping bug described above — it failed on the first implementation
      attempt (before the `resolved_config` fix) because it checks the *written file's*
      on-disk attribute via a fresh read, not just the in-memory object passed in.
- [x] **`test_writer_defaults_to_meta_file_version`** — construct the writer with no
      `target_version` argument; confirms behavior is identical to today's (pre-Phase-2)
      writer construction.
- [x] Full existing Python suite green — 403/403 (was 378 before this phase's file:
      +24 in the new `test_power_characterization.py`, plus net new tests in
      `test_v1_1_adapter.py` after the migrate-function tests were rewritten/consolidated
      rather than simply added alongside).

### Per-language implementation status

- [x] **Python**: **Done (2026-09-01).**
      - New file `python/src/machine_config/power_characterization.py` — the four
        public functions (`forward_power_characterization_coefficients`,
        `forward_power_characterization_points`, `backward_flat_fields_coefficients`,
        `backward_flat_fields_points`) plus private helpers
        (`_parse_csv_floats`, `_generate_power_equation`, `_sorted_constants_by_name` —
        the last extended with a numeric-positional-fallback sort bucket and an
        alphabetical fallback for arbitrary natively-authored names). Imports only
        `machine_config.models` — confirmed no `v1_0`/`v1_1` reference anywhere in it.
      - `capabilities/v1_0/hdf5.py`, `capabilities/v1_1/hdf5.py`: **untouched**, exactly
        as Phase 1 left them — confirmed via `git diff` showing zero changes to either
        file, not just asserted.
      - `capabilities/v1_0/writer.py`: `_write_clearbox`/`_write_light_source` write
        the native flat fields if present, else fall back to
        `backward_flat_fields_coefficients`/`_points`.
      - `capabilities/v1_1/writer.py`: `_write_clearbox`/`_write_light_source` write
        native `power_characterization` if present, else fall back to
        `forward_power_characterization_coefficients`/`_points`.
      - `capabilities/v1_1/hdf5.py`: `migrate_v1_to_v1_1`/`migrate_v1_1_to_v1` and their
        private helpers deleted entirely (roughly 400 lines) — replaced with a short
        comment pointing to `power_characterization.py`.
      - `capabilities/v1_1/__init__.py`: migrate function exports removed from
        `__all__`.
      - `capabilities/v1_1/file.py`'s `create()`: simplified — builds a plain v1.0-shaped
        mock, sets `meta.file_version = "1.1"`, hands it straight to `Hdf5WriterV1_1`.
        No explicit derivation call; the writer's own fallback handles it.
      - `python/src/machine_config/writer.py`: `MachineConfigWriter` gained the optional
        `target_version` parameter, plus the `resolved_config` fix for the
        verbatim-File_Version-stamping bug described above.
      - New test files `python/tests/test_power_characterization.py` (24 tests) and a
        substantially rewritten `python/tests/test_v1_1_adapter.py` (per the "Tests"
        list above).
      - **Design change made and reverted during this pass, not just planned in the
        abstract**: implementation started under the original Option A (reader-side)
        draft — `capabilities/v1_0/hdf5.py`'s `_parse_clearbox`/`_parse_light_source`
        were actually edited to call forward derivation, and a
        `_power_characterization_to_dict`/`_from_dict` pair plus dict-serialization
        wiring were added to work around a gap Option A would have introduced in the
        get/set facade (`config_from_dict`/`_clearbox_to_dict` silently dropping the
        newly-reader-populated `power_characterization`). All of this was reverted
        (via targeted `Edit`, not a destructive `git checkout`, since the changes were
        uncommitted work) once Option C was adopted instead — confirmed via `git diff`
        showing zero net changes to `capabilities/v1_0/hdf5.py` after the revert.
      - **Found, out of scope, not fixed**: that same get/set facade gap
        (`config_from_dict`/`_clearbox_to_dict`/`_light_source_to_dict` in
        `capabilities/v1_0/hdf5.py` don't serialize/hydrate `firmware_version` or
        `power_characterization` at all) is a **pre-existing** gap dating to Phase 0/1,
        not something Option C introduces (Option C's readers never populate
        `power_characterization`, so there's nothing new for that path to drop) — but a
        caller explicitly `set_model()`-ing either field on a v1.0 file through the
        generic dict-based facade would still silently lose it. Left alone deliberately,
        matching this plan's own scope discipline; worth a follow-up note in
        `PARITY_AUDIT.md` alongside the other known facade gaps.
      - **Not done in this pass**: `fixtures/reference_config_v1_1_migrated.h5`
        regeneration (its `Input_Type`/`Units_Derived_Quantity` values still reflect the
        old hardcoded placeholders — **still outstanding**, see the Verification section)
        and the `docs/migrations/v1_0_to_v1_1.md` manifest update (at the time, still
        documented the old hard-error and hardcoded-placeholder behavior — **since
        resolved (2026-09-01)** when the manifest was promoted from `file_testing/` and
        annotated with superseded-on-the-writer-path notes under Changes 3–4, plus a new
        "API impact" section). Both were checklist items 8–9 above, deferred rather than
        skipped at the time — flagging here so they weren't lost track of before the
        other 4 languages started, since the fixture in particular is shared across all 5
        languages' test suites.
      - **Verification**: 403/403 Python tests pass (was 378), zero regressions.
        Independently re-verified: reran the full suite after each structural change
        (not just at the end), confirmed via `git diff` that both readers are
        byte-identical to Phase 1, confirmed the version-adapter-isolation guard test's
        matching logic directly rather than assuming the new module was safe, and caught
        the target-version File_Version-stamping bug via a failing test rather than by
        reasoning about the code in the abstract.
- [x] **Rust**: **Done (2026-09-01).** Was blocked on an unrelated Windows security
      issue in this environment preventing execution of freshly-built `.exe` files
      (`cargo test`/direct invocation both failed with OS error 5, "Access is denied")
      — resolved by the user separately (not a code problem); `cargo test` then run
      and its output actually inspected, not just re-attempted.
      - New file `rust/src/power_characterization.rs` — same four public functions as
        Python (`forward_power_characterization_coefficients`/`_points`,
        `backward_flat_fields_coefficients`/`_points`), same private helpers relocated
        from the deleted migrate functions (`parse_csv_floats`, `generate_power_equation`,
        `sort_key_for_constant_name`/`sorted_constants_by_name` — extended with the same
        numeric-positional-fallback and alphabetical-fallback sort buckets as Python's —
        and `format_f64_like_python`, needed here because Rust's `f64::to_string()`
        doesn't match Python's `str(float)` output). Declared in `lib.rs` as
        `pub mod power_characterization`. Imports only `crate::models` and
        `crate::error` — confirmed no `v1_0`/`v1_1` token anywhere in it (the guard
        test's own matching logic only flags `vX_Y`-shaped tokens after `::` or on a
        `use`/`mod` line, and `power_characterization` isn't shaped like one — verified
        by reading `version_adapter_isolation_test.rs`'s matcher directly, not assumed).
      - `capabilities/v1_0/hdf5.rs`, `capabilities/v1_1/hdf5.rs`: **untouched** — no
        edits made to either reader.
      - `capabilities/v1_0/writer.rs`: `write_clearbox`/`write_light_source` write the
        native flat fields if present, else fall back to
        `backward_flat_fields_coefficients`/`_points`.
      - `capabilities/v1_1/writer.rs`: `write_clearbox`/`write_light_source` write
        native `power_characterization` if present, else fall back to
        `forward_power_characterization_coefficients`/`_points` (fallible — a genuinely
        malformed CSV float still propagates `Err`, an orthogonal, pre-existing failure
        mode distinct from the removed unrecognized-algorithm-type hard error).
      - `capabilities/v1_1/hdf5.rs`: `migrate_v1_to_v1_1`/`migrate_v1_1_to_v1` and their
        private helpers deleted (~270 lines) — replaced with a short comment pointing to
        `power_characterization.rs`. `capabilities/v1_1/mod.rs`'s `pub use` updated to
        drop both.
      - `error.rs`: `MachineConfigError::UnrecognizedAlgorithmType` variant deleted
        entirely (confirmed via grep it was referenced nowhere else in `src/` or
        `tests/` before removing it).
      - `rust/src/writer.rs`: `MachineConfigWriter` gained
        `with_target_version(config, target_version)` as a second constructor
        alongside the existing `new(config)` — Rust has no default parameters, so unlike
        Python's single-constructor-plus-optional-arg shape, this is a second associated
        function, not a modified `new`. Includes the same verbatim-`File_Version`-
        stamping fix as Python's `resolved_config`: `write()` clones `config` and
        corrects `meta.file_version` before constructing the backend, but only when
        `target_version` actually differs from the config's own version, so the common
        (no-override) path never pays for a clone.
      - **Real cross-language design difference found, not just translated
        mechanically**: `capabilities/v1_1/file.rs`'s `create()` does **not** round-trip
        through a real HDF5 write/read the way Python's does — it builds the mock
        config and returns it held in memory directly. That means the writer's Phase 2
        fallback never runs for this path in Rust, unlike Python (where `create()`
        could be simplified to rely on it). Rust's `create()` therefore still calls
        `forward_power_characterization_coefficients`/`_points` explicitly, per
        ClearBox/LightSource, on the mock — the opposite simplification direction from
        Python's write-up above. Flagged explicitly so this isn't mistaken for a missed
        translation later.
      - New test file `rust/tests/power_characterization_test.rs` (24 tests, mirroring
        Python's `test_power_characterization.py` one-for-one) and a rewritten
        `rust/tests/v1_1_adapter_test.rs` (same test names as Python's rewrite:
        `test_v1_to_v1_1_unrecognized_algorithm_type_best_effort`,
        `test_v1_0_write_never_invokes_fallback_when_native_present`,
        `test_roundtrip_v1_0_to_v1_1_to_v1_0`/`_v1_1_to_v1_0_to_v1_1`,
        `test_writer_target_version_overrides_meta`/`_defaults_to_meta_file_version`,
        etc. — all using `MachineConfigWriter::with_target_version` instead of a
        deleted migrate function).
      - **Verification: complete.** `cargo test` run in full once the security block
        cleared: **201/201 tests pass, 0 failed** — 99 lib unit tests, 5 mock-adapter
        migration tests, 18 capabilities-facade tests, 30 integration tests, 21 new
        `power_characterization_test.rs` tests, 20 rewritten `v1_1_adapter_test.rs`
        tests, 6 version-adapter-isolation guard tests, 2 doc-tests. No fixes needed on
        this pass — the assertion adjustments anticipated above (e.g. the
        disk-round-trip blank-string normalization Python's suite needed) turned out to
        already be correctly handled here, since this rewrite was done after Python's
        finding was already known and accounted for up front.
- [x] **C++**: **Done (2026-09-01).** Was blocked on the same Windows security issue as
      Rust above (confirmed the same root cause, different symptom: `catch_discover_tests`'s
      post-build step, which must run the just-built test binary to enumerate its
      `TEST_CASE`s for CTest, failed with `Result: operation not permitted` rather than
      Rust's OS error 5) — resolved by the user separately; the test binary was then run
      directly and its output actually inspected.
      - New header `cpp/include/machine_config/power_characterization.hpp` — same four
        public functions as Python/Rust (`forwardPowerCharacterizationCoefficients`/
        `Points`, `backwardFlatFieldsCoefficients`/`Points`, the latter pair returning
        `std::pair<std::optional<std::string>, std::optional<std::string>>`), same
        private helpers relocated from the deleted migrate functions
        (`pcParseCsvFloats`, `pcGeneratePowerEquation`,
        `pcSortedConstantsByName` — extended with the same numeric-positional-fallback
        and alphabetical-fallback sort buckets as Python/Rust — and
        `pcFormatDoubleForCsv`, which keeps Phase 1's `std::setprecision(17)` choice
        verified back then against this project's real fixtures, not the
        shortest-round-trip formatting Python/Rust use). Header-only, `inline`
        functions, namespace `machine_config` (top-level, matching `models.hpp` — not
        nested under `capabilities::v1_0`/`v1_1`). Confirmed safe against the guard
        test's actual matching regex (`[vV](\d+)_(\d+)`, scanning only files under
        `include/machine_config/capabilities/v1_0`, `v1_1`, and the test-only mock) by
        reading `test_version_adapter_isolation.cpp` directly — this new header lives
        outside all three scanned locations, so it can't be flagged regardless of what
        it's named.
      - `capabilities/v1_0/hdf5.hpp`, `capabilities/v1_1/hdf5.hpp`: **untouched** — no
        edits made to either reader.
      - `capabilities/v1_0/writer.hpp`: `writeClearBox`/`writeLightSource` write the
        native flat fields if present, else fall back to
        `backwardFlatFieldsCoefficients`/`Points` (structured bindings over the
        returned `std::pair`).
      - `capabilities/v1_1/writer.hpp`: `writeClearBox`/`writeLightSource` compute
        `pc` as native `power_characterization` if present, else
        `forwardPowerCharacterizationCoefficients`/`Points`, then pass that (unchanged)
        into the existing `writePowerCharacterization` helper — this helper itself
        needed no changes, since Phase 1's version already treated a `nullopt` `pc`
        correctly (schema-complete-but-blank groups).
      - `capabilities/v1_1/hdf5.hpp`: `migrateV1ToV1_1`/`migrateV1_1ToV1` and their
        private helpers deleted (~300 lines) — replaced with a short comment pointing
        to `power_characterization.hpp`.
      - `cpp/include/machine_config/writer.hpp`: `MachineConfigWriter` gained a second,
        two-argument constructor (`MachineConfigWriter(const MachineConfig&,
        std::string targetVersion)`) alongside the existing single-argument one — C++
        has no named/keyword optional-parameter idiom as clean as Python's here, so
        (matching Rust's `with_target_version` choice) this is an overload, not a
        modified single constructor. Includes the same verbatim-`File_Version`-stamping
        fix as Python's/Rust's: `write()` copies `cfg_` and corrects
        `meta.file_version` before constructing the backend, but only when
        `targetVersion_` actually differs from the config's own version.
      - **Real cross-language design difference, same shape as Rust's**:
        `capabilities/v1_1/file.hpp`'s `create()` does **not** round-trip through a
        real HDF5 write/read the way Python's does — like Rust, it builds the mock
        config and holds it in memory directly, so the writer's Phase 2 fallback never
        runs for this path. `create()` therefore still calls
        `forwardPowerCharacterizationCoefficients`/`Points` explicitly, per
        ClearBox/LightSource, exactly mirroring the fix already made in Rust's
        `MachineConfigFileV1_1::create`.
      - New test file `cpp/tests/test_power_characterization.cpp` (26 `TEST_CASE`s,
        mirroring Python's/Rust's shared-module tests) and a rewritten
        `cpp/tests/test_v1_1_adapter.cpp` (same coverage as Python's/Rust's rewrites —
        best-effort-not-raises tests for both ClearBox and Light_Source, the
        writer-never-invokes-fallback-when-native-present regression test, both
        acceptance-criterion round-trip tests, and the writer-targetVersion tests — all
        using `MachineConfigWriter`'s new two-argument constructor instead of a deleted
        migrate function). Both registered in `cpp/tests/CMakeLists.txt`.
      - **Real bug caught on the first actual test run, not by inspection**: the
        version-adapter-isolation guard test failed with 3 violations —
        `capabilities/v1_0/writer.hpp` flagged for containing the token `v1_1` (twice)
        and `capabilities/v1_1/hdf5.hpp` flagged for containing `v1_0` (once). Root
        cause: the guard test's regex (`[vV](\d+)_(\d+)`) matches *any* text shaped like
        a version token, including inside doc comments that reference the *other*
        version incidentally rather than importing its code — specifically, two Phase 2
        comments in `v1_0/writer.hpp` cited "V1_1_IMPLEMENTATION_PLAN.md" by name (the
        literal string contains `V1_1`), and one Phase 2 comment in `v1_1/hdf5.hpp`
        referenced "`capabilities/v1_0/writer.hpp`" by path. Not a real coupling
        (confirmed: no actual `#include`/namespace reference to the other version in
        either file, only prose), but a real violation of the guard test's own rule as
        written, and correctly caught by it — comments describing *why* a design choice
        was made are exactly the kind of thing this rule is meant to force to stay
        version-neutral, the same discipline Phase 1's `migrateV1ToV1_1`'s own doc
        comments already followed (deliberately writing "the previous version"/"this
        File_Version" instead of literal `v1_0`/`v1_1"). Fixed by rewording: dropped the
        literal plan filename from the `v1_0/writer.hpp` comments, and replaced the
        literal path in `v1_1/hdf5.hpp` with "the previous version's writer." Re-ran the
        full suite clean afterward — this was the only fix needed.
      - **Verification: complete.** Full solution rebuilt after the fix
        (`machine_config_tests`, `machine_config_cli`, `full_workflow`, `quickstart` —
        the same "full solution build succeeds" bar Phase 1 used) and the test binary
        run directly: **all tests passed — 826 assertions in 155 test cases, 0
        failures.** (An HDF5-DIAG diagnostic block prints to stderr at startup — this is
        HDF5's own internal logging from a test that deliberately exercises a
        missing-group error path, not a test failure; the final summary line is the
        authoritative result.)
- [x] **Go**: **Done (2026-09-01).**
      - New file `go/internal/models/power_characterization.go` — same four public
        functions as Python/Rust/C++ (`ForwardPowerCharacterizationCoefficients`/`Points`,
        `BackwardFlatFieldsCoefficients`/`Points`, the latter pair returning
        `(algorithmType, params *string)`), same private helpers relocated from the
        deleted migrate functions (`parsePCCSVFloats`, `generatePCPowerEquation`,
        `pcSortedConstantsByName`/`pcConstantSortKey` — extended with the same
        numeric-positional-fallback and alphabetical-fallback sort buckets as the other
        3 languages — and `formatPCFloat`). Placed in `internal/models` rather than a
        new top-level package — confirmed both `capabilities/v1_0/hdf5` and
        `capabilities/v1_1/hdf5` already dot-import this package
        (`. "machine-config-go/internal/models"`), so the new functions are callable
        unqualified from both writers with no new import needed, and confirmed
        `go/version_adapter_isolation_test.go` only walks `capabilities/v1_0`,
        `capabilities/v1_1`, and `internal/mockv1_1` — `internal/models` is outside all
        three, so this file can never be flagged regardless of content. Also confirmed
        (unlike C++'s regex-based full-text scan) that Go's guard test parses only
        actual `import` declarations via `go/parser`'s `ImportsOnly` mode, so — unlike
        the false positives hit during C++'s verification — doc-comment prose
        mentioning the other version's token here needed no special wording at all.
      - `capabilities/v1_0/hdf5/writer.go`: `writeLightSource`/`writeClearBox` write
        the native flat fields if present, else fall back to
        `BackwardFlatFieldsCoefficients`/`Points`.
      - `capabilities/v1_1/hdf5/writer.go`: `writeLightSource`/`writeClearBoxPerTrain`
        compute `pc` as native `PowerCharacterization` if present, else
        `ForwardPowerCharacterizationCoefficients`/`Points` (fallible — a genuinely
        malformed CSV float still returns an error, an orthogonal, pre-existing failure
        mode distinct from the removed unrecognized-algorithm-type hard error), then
        pass that into the existing `writePowerCharacterization` helper unchanged.
      - `capabilities/v1_1/hdf5/migrate.go`: `MigrateV1ToV1_1`/`MigrateV1_1ToV1` and
        their private helpers deleted (~330 lines) — replaced with a short doc comment
        pointing to `internal/models/power_characterization.go`.
      - `go/writer.go`: `MachineConfigWriter` gained a `WriteAs(cfg, path,
        targetVersion)` method alongside the existing `Write(cfg, path)` — Go's
        `MachineConfigWriter` is already stateless (`struct{}`, `cfg` passed per call,
        not held), which made this simpler than the other 3 languages' constructor
        changes: no stored-reference lifetime concerns, just a second method. Includes
        the same verbatim-`File_Version`-stamping fix as the other languages: `WriteAs`
        copies `*cfg` and corrects `Meta.FileVersion` before constructing the backend,
        but only when `targetVersion` actually differs from the config's own version.
      - **Real cross-language finding — the opposite direction from Rust's/C++'s**:
        `capabilities/v1_1/file.go`'s `Create()` needed **zero changes**. Unlike
        Python/Rust/C++, Go's `Create()` was never built on `MockConfigBuilder` or a
        migrate function at all — it's a fully hand-authored `MachineConfig` literal
        with no `ClearBox` and a `LightSource` that never sets
        `WattsToVoltsAlgorithm`/`Params` in the first place (confirmed by reading the
        literal directly, not assumed). With nothing to derive from and no
        `PowerCharacterization` concept touched anywhere in this path, there was
        nothing for Phase 2 to fix here — a third distinct `create()`/`Create()`
        situation across the 4 languages implemented so far (Python: relies on the
        writer's fallback via a real disk round-trip; Rust/C++: in-memory only, needed
        explicit derivation calls; Go: doesn't touch this concept at all).
      - New test file `go/power_characterization_test.go` (24 tests, mirroring the
        other 3 languages' shared-module tests) and a rewritten
        `go/v1_1_adapter_test.go` (same coverage as the other rewrites — best-effort
        tests for both ClearBox and Light_Source instead of the deleted hard-error
        tests, the writer-never-invokes-fallback-when-native-present regression test,
        both acceptance-criterion round-trip tests, and the `WriteAs` tests — all using
        `MachineConfigWriter.WriteAs` instead of a deleted migrate function).
      - **Verification: complete.** `go build ./...` and `go vet ./...` both clean.
        `go test ./...` — **all packages `ok`, 0 failures**: 133 tests total across the
        module (105 in the root `machineconfig_test` package — including the new
        `power_characterization_test.go` and the rewritten `v1_1_adapter_test.go` — 21
        in `capabilities`, 7 in `internal/h5c`). Run with
        `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH; $env:CGO_ENABLED = "1"` per
        this project's established CGO/HDF5 toolchain requirement on Windows.
- [x] **Node.js**: **Done (2026-09-01) — the final language for Phase 2.**
      - New file `nodejs/src/powerCharacterization.ts` — same four public functions as
        the other 4 languages (`forwardPowerCharacterizationCoefficients`/`Points`,
        `backwardFlatFieldsCoefficients`/`Points`, the latter pair returning a
        `[algorithmType, params]` tuple), same private helpers relocated from the
        deleted migrate functions (`parseCsvFloats`, `generatePowerEquation`,
        `sortedConstantsByName` — extended with the same numeric-positional-fallback
        and alphabetical-fallback sort buckets as the other languages). Placed at
        `src/powerCharacterization.ts` (sibling to `models.ts`), outside
        `src/capabilities/` — confirmed
        `nodejs/tests/versionAdapterIsolation.test.ts` only scans files under
        `src/capabilities/<vX_Y>/` plus the one named mock file, so this module is
        structurally exempt regardless of content.
      - **Real near-miss, caught before it became a bug, unlike C++'s equivalent**:
        this guard test is a **raw-text scan** (regex over the whole file, comments and
        strings included) — the same shape as C++'s, not Python's/Rust's/Go's
        parser-based approach. Having already been burned by this exact pattern during
        C++'s verification, every Phase 2 comment added to `capabilities/v1_0/writer.ts`
        and `capabilities/v1_1/{writer,hdf5,index}.ts` was deliberately written
        version-neutral from the start ("Phase 2 clean-up (see the migration
        implementation plan)" instead of naming the plan file, "the previous version's
        writer" instead of a literal path) — confirmed by running the guard test
        immediately after writing those files, which passed on the first attempt, not
        after a fix.
      - `capabilities/v1_0/writer.ts`: `writeLightSource`/`writeClearBox` write the
        native flat fields if present, else fall back to
        `backwardFlatFieldsCoefficients`/`Points`.
      - `capabilities/v1_1/writer.ts`: `writeLightSource`/`writeClearBox` compute `pc`
        as native `power_characterization` if present, else
        `forwardPowerCharacterizationCoefficients`/`Points`, then pass that into the
        existing `writePowerCharacterization` helper unchanged.
      - `capabilities/v1_1/hdf5.ts`: `migrateV1ToV1_1`/`migrateV1_1ToV1` and their
        private helpers deleted (~255 lines) — replaced with a short comment pointing
        to `../../powerCharacterization.js`. `capabilities/v1_1/index.ts`'s re-exports
        updated to drop both.
      - `nodejs/src/writer.ts`: `MachineConfigWriter` gained an optional second
        constructor parameter, `targetVersion?: string` — TypeScript's native optional
        parameters made this the closest to Python's single-constructor shape of any
        of the 4 non-Python languages (no second constructor/method needed, unlike
        Rust/C++/Go). Includes the same verbatim-`File_Version`-stamping fix as the
        other languages: the constructor builds a corrected shallow copy of `config`
        (spreading `meta` with `file_version` overridden) before constructing the
        backend, but only when `targetVersion` actually differs from the config's own
        version.
      - **Real cross-language finding, same shape as Rust's/C++'s**:
        `capabilities/v1_1/file.ts`'s `create()` does **not** round-trip through a real
        HDF5 write/read the way Python's does — like Rust/C++, it builds the mock
        config and holds it in memory directly (`asJson(config)`, a plain cast, no I/O),
        so the writer's Phase 2 fallback never runs for this path. `create()` therefore
        still calls `forwardPowerCharacterizationCoefficients`/`Points` explicitly, per
        ClearBox/LightSource, mirroring the fix already made in Rust's/C++'s
        `create()`/`Create()`.
      - **Real, pre-existing cross-language inconsistency found and deliberately left
        alone, not fixed**: Node's existing float-to-string formatting for backward
        derivation is plain `String(value)` (e.g. `"1"`), not the other 4 languages'
        always-include-a-decimal-point convention (e.g. `"1.0"`) — confirmed by reading
        the pre-existing (Phase 1) test expectation
        (`volts_to_watts_params).toBe('1,2')`, not `'1.0,2.0'`) before writing new
        tests, not after a failure. This predates Phase 2 entirely and is out of its
        scope to fix (a Phase 1 finding, not a Phase 2 one) — preserved exactly as-is in
        the new shared module and in every rewritten test, rather than silently
        "corrected" to match the other languages. Worth a note in `PARITY_AUDIT.md` as
        a real, live formatting difference a caller round-tripping between languages
        could observe.
      - New test file `nodejs/tests/powerCharacterization.test.ts` (21 tests, mirroring
        the other 4 languages' shared-module tests, using Node's own `String(value)`
        formatting convention in its expectations) and a rewritten
        `nodejs/tests/v1_1Adapter.test.ts` (same coverage as the other rewrites —
        best-effort tests for both ClearBox and Light_Source instead of the deleted
        hard-error tests, the writer-never-invokes-fallback-when-native-present
        regression test, both acceptance-criterion round-trip tests, and the
        `targetVersion` tests — all using `MachineConfigWriter`'s new optional
        parameter instead of a deleted migrate function).
      - **Real bug caught on the first actual test run, not by inspection** (same
        finding as every other language's rewrite, for the same reason): one assertion
        expected `firmware_version` to read back as `undefined` after a v1.0→v1.1 write
        — true only of the deleted migrate function's direct in-memory field
        assignment, not of a real disk round trip (h5wasm normalizes an absent/empty
        string attribute to `null` on read). Fixed the assertion to `toBeNull()`,
        matching every other language's identical finding.
      - **Verification: complete.** `npx tsc --noEmit` (main source) and
        `npx tsc -p tests/tsconfig.json --noEmit` (tests) both clean.
        `npx vitest run` — **9 test files, 236/236 tests pass, 0 failures** (21 new in
        `powerCharacterization.test.ts`, 23 in the rewritten `v1_1Adapter.test.ts`,
        plus every pre-existing file including `versionAdapterIsolation.test.ts`, which
        passed clean on the first run, not after a fix).

---

## Verification

1. Per language, per change: that language's suite green, immediately after Phase 1. **Done.**
2. All tests above, green, per language. **Done** — see each language's Phase 1/Phase 2
   write-ups above for the exact numbers.
3. Full `python tools/cross_check.py --verbose` (all 5 languages, all 5 phases) with the
   v1.1 fixtures from Phase 0 included alongside the existing v1.0 fixtures. **Done
   (2026-09-01).** `tools/cross_check.py`'s `FIXTURES` dict didn't have the v1.1 fixtures
   registered at all — added `"v1_1"` (`reference_config_v1_1.h5`) and `"v1_1_migrated"`
   (`reference_config_v1_1_migrated.h5`) entries. Note for future reference: Phase 3
   (write interop) and Phase 3.5 (binary copy) are hardcoded to `FIXTURES["reference"]`
   only (pre-existing tool design, not new) — adding fixtures to the dict extends Phases
   1, 2, and 4's coverage, not 3/3.5. All 5 phases pass, all 5 languages, 0 failures,
   after one real bug found and fixed (below) and two environment-only false alarms:
   - **Real bug found and fixed**: Phase 2 (read parity) flagged Node vs. every other
     language disagreeing on `reference_config_v1_1_migrated.h5` —
     `optional_components.clearbox.firmware_version` present as `null` in Node's JSON,
     absent entirely from Python/Rust/C++/Go's. Root cause:
     `nodejs/src/capabilities/v1_1/hdf5.ts`'s ClearBox parser had
     `firmware_version: attrStr(a, "Firmware_Version")`, which returns `null` (not
     `undefined`) for a blank attribute — `JSON.stringify` keeps a `null`-valued property
     but drops an `undefined` one, so the key stayed present, unlike the other 4
     languages' `skip_serializing_if`/`omitempty`/`has_value()`/`is not None` guards.
     The sibling `power_characterization` field on the very next line already correctly
     used `undefined` — a straightforward one-line copy-paste-shaped miss, not a
     conceptual gap. **Fixed** (`?? undefined`, matching that sibling line) and rebuilt;
     `nodejs/tests/v1_1Adapter.test.ts`'s assertion for this field, which an earlier
     Phase 2 fix had changed to `.toBeNull()`, is reverted to `.toBeUndefined()` — that
     earlier "fix" was unknowingly asserting the buggy behavior as correct. This is
     exactly what `docs/contributing.md` means by "the cross-check catches bugs that
     per-language tests cannot": Node's own suite was internally consistent and still
     green throughout, since nothing in it compared against another language's output.
     Node's full suite re-verified after both fixes: 236/236, 0 regressions.
   - **Environment-only, not code bugs** (both specific to this sandbox, not the
     library): Go's CLI first failed every invocation with exit code `3221225781`
     (`STATUS_DLL_NOT_FOUND`) — cross_check.py's subprocess `env` doesn't inherit the
     MSYS2 `mingw64/bin` directory `libhdf5`'s DLL needs at runtime unless the *parent*
     process's `PATH` already has it; fixed by running cross_check.py itself with that
     prepended, same requirement Go's own `go test` needed earlier in this plan. Separately,
     Phase 3/3.5's `f"{writer_lang}-writes → ..."`-style tag strings (a literal `→` in the
     Python source) raised `UnicodeEncodeError: 'charmap' codec` when `print()`ed to a
     non-UTF-8 Windows console — fixed by setting `PYTHONUTF8=1` for cross_check.py's own
     process (subprocess calls already set this for their children; the parent process's
     own stdout was the gap). Neither is a `PARITY_AUDIT.md`-worthy finding — both are
     purely this environment's console/PATH configuration.
   - **Confirmed, not just assumed**: `PARITY_AUDIT.md`'s Tier 2 Node float-formatting
     item did **not** surface here, and structurally cannot with `cross_check.py` as it
     exists today. That bug only lives in the backward/forward CSV-string derivation
     functions (`backwardFlatFieldsCoefficients`/`Points`,
     `forwardPowerCharacterizationCoefficients`/`Points`), which only run when a
     writer's *own* native field is absent and it must fall back to deriving from the
     other version's shape. Phase 2 (read parity) never invokes them at all — readers
     stay untouched under Option C, they read whatever's actually on disk. Phase 3/3.5
     (write interop / binary copy) are hardcoded to `FIXTURES["reference"]` (a v1.0
     fixture) and always round-trip at the *same* `File_Version` the fixture already
     has — every writer's CLI `write`/`write-hdf5` command has no target-version
     override (that only exists on `MachineConfigWriter` itself, not yet plumbed through
     any CLI), so native fields are always present and fallback derivation never fires.
     If `cross_check.py` (or a CLI) is ever extended to exercise the acceptance
     criterion directly — writing a v1.0-sourced config out as v1.1, or vice versa —
     this bug would become live for the first time and would need fixing before that
     work could pass.
4. **Done (2026-09-01).** `file_testing/v1_0_to_v1_1.md` (and its diagrams file) promoted
   to `docs/migrations/v1_0_to_v1_1.md`/`v1_0_to_v1_1_diagrams.md`, with a new "API impact"
   section added covering the Phase 2 writer-API changes (target-version override,
   migrate-function removal) — see that section for the per-language signatures. The two
   new change categories (Consolidate, Transform) are folded into `docs/contributing.md`'s
   category table.
5. **Resolved (2026-09-01).** By explicit decision, this repo's existing
   semantic-release-generated `CHANGELOG.md` (from `.releaserc.json`) needs no
   hand-authored `## [1.1.0]` section — land the work as `feat` commits with descriptive
   bodies and let the release-notes-generator produce the entry. `docs/contributing.md`'s
   step 6 rewritten to match: the migration manifest's own "API impact" section is the
   consumer-facing detail, not a hand-authored CHANGELOG table.
