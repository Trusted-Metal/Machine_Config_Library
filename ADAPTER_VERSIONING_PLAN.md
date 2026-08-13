# Adapter & Schema Versioning — Implementation Plan

This plan builds the adapter system end-to-end: base contracts, code generation,
dispatch, unit tests, cross-language parity, and CI enforcement. Each phase has a
clear goal, step-by-step implementation prompts, and a verification gate that must
past before moving to the next phase.

---

## Scope — What This Plan Does and Does Not Do

**This plan validates the adapter machinery. It does not perform a real schema migration.**

The schema remains at its current version (`"1.0"`) throughout. `CURRENT_VERSION`
in every production reader is never changed by this work. Real files read exactly
as they do today — the adapter chain is wired in but dormant for all current files
(since `from_version == CURRENT_VERSION` → empty chain).

**Version attribute**: `File_Version` is the single HDF5 root attribute used for
adapter dispatch. `MachineConfigMeta.file_version` is its Python model counterpart.
There is no separate `schema_version` attribute — the library owns the MCF format
end-to-end, so a second version attribute would be redundant duplication.

Validation is driven by a **synthetic test fixture**: an HDF5 file artificially
tagged with `File_Version = "0.9"` (a fictional old version that never existed
in production). This file only lives under `fixtures/adapters/test/`. The
corresponding adapter spec (`test_v0_9_to_v1_0.yaml`) is also test infrastructure
— it is clearly named and commented so it is never mistaken for a real migration.

When a real schema change occurs in the future, the work is:
1. Write a real spec in `schema/adapters/`
2. Run `python tools/generate_adapters.py`
3. Bump `CURRENT_VERSION` in each reader and update `File_Version` in new files
4. Update each language's registry to include the new adapter
5. Ship — with confidence the system works because this plan already proved it.

---

## Design Decision (Resolved — Option A confirmed)

**Adapters operate on raw JSON (dict / Map / nlohmann::json) in all four languages,
before deserialization into the typed model.**

The alternative — running adapters on the typed model — was explicitly evaluated
and rejected. A typed-model adapter pipeline silently drops fields the current model
does not recognise before the adapter ever runs. For HDF5-stored configs this is
fatal: a field renamed between v0.8 and v0.9 would vanish on read rather than being
migrated. A raw-dict pipeline preserves every attribute in the file, including ones
the current typed model has never seen, and hands them to the adapter intact.

Rationale:
- **Pipeline integrity**: Every attribute in the HDF5 file survives to the adapter
  unchanged. Unknown fields are not silently discarded at the model boundary.
- **Arbitrary version chains**: Adapters can chain 0.5→0.9→1.0 without requiring a
  typed model for each intermediate version.
- **Consistency**: Rust and C++ already use raw JSON. Aligning Python and Node.js
  removes the split strategy.
- **Cross-language parity**: All four languages compare the same JSON structure in
  Phase E tests.
- **Ergonomics cost is low**: Adapter logic is add/rename/remove key operations.
  Type safety adds nothing here.

**Required pipeline in every language (Phase D)**:
```
HDF5 → raw dict (ALL fields preserved) → adapter chain → typed model
```
The HDF5→raw dict step must read ALL group attributes — not only the fields the
current typed model knows about. Each Phase D sub-task therefore has two explicit
steps: (1) implement the HDF5→raw dict parser, then (2) wire adapter dispatch using
that parser. A pipeline that deserialises to the typed model first is incorrect and
must not be used.

**Action required in Phase A**: Update `tools/templates/adapter_python.py.j2` so
`adapt()` takes and returns `dict` (not `MachineConfig`). Update
`tools/templates/adapter_typescript.ts.j2` so `adapt()` takes and returns
`Record<string, unknown>` (not `MachineConfig`). Record this decision as a comment
at the top of `tools/generate_adapters.py`.

---

## Change Type Taxonomy

Adapters express schema changes as a list of typed operations. The generator reads
these from a spec YAML and emits language-specific code for each entry.

### Implemented — exercised by the synthetic test spec in Phase B

| Type | YAML keys | What it does |
|---|---|---|
| `field_add` | `field`, `path`, `default` (optional) | Inserts `field` at `path` with `default` value (`null` if omitted) when the key is absent |
| `field_rename` | `old_field`, `new_field`, `path` | Copies value from `old_field` to `new_field` at `path`, erases source |
| `field_remove` | `field`, `path` | Erases `field` at `path` |
| `field_move` | `field`, `from_path`, `to_path` | Same name, new nested location; reads from `from_path`, writes to `to_path`, erases source |
| `field_move_rename` | `old_field`, `new_field`, `from_path`, `to_path` | New location AND new name atomically; avoids stale-data risk of a two-step approach |

### Reserved — generator emits a hard error until implemented

These types are enumerated now so the spec format is stable and cannot be silently
misused. When a reserved type is first needed: design its YAML keys, add template
branches in all four language templates, add a test case to the synthetic spec, and
remove the hard-error stub. The existing Phase B infrastructure (fixtures, golden
file, cross-check parity) requires no structural change — just add new entries to
the spec and regenerate.

| Type | Description | Why deferred |
|---|---|---|
| `type_coerce` | Same path/name, value encoding changes (units, bool repr, int↔float) | Requires a coercion-function vocabulary in the spec |
| `field_split` | One field becomes two or more | Requires a split-function specification |
| `field_merge` | Two or more fields collapse into one | Inverse of `field_split`; same constraint |
| `container_restructure` | A group of fields wrapped into a new sub-object en masse | Equivalent to many simultaneous `field_move` ops with structural side-effects |
| `conditional` | Change applies only when a predicate on another field is satisfied | Requires a predicate mini-DSL in the spec |

---

## Phase A — Foundation: Base Classes & Contracts

**Goal**: Every language has a concrete, compilable adapter contract. The code
generator's output is syntactically valid and importable in all four languages.
The Python and Node.js templates are updated to the resolved raw-JSON approach.
Nothing is wired to the reader yet.

**Delivers**: `BaseAdapter` (Python), `Adapter` interface wiring (Node.js), adapter
`trait` (Rust), abstract base (C++). Updated Python and TypeScript templates.

---

### A.1 — Python: Define `BaseAdapter` (raw-dict approach) ✅ COMPLETE

**Prompt**:
> `python/src/machine_config/adapters/base.py` currently contains only a comment.
> Define a `BaseAdapter` abstract base class there. It must have two class-level
> string attributes — `from_version` and `to_version` — and one abstract method
> `adapt(self, config: dict) -> dict`. The `dict` represents the raw parsed JSON
> of the full config (before deserialization into `MachineConfig`). Use `abc.ABC`
> and `abc.abstractmethod`. Do not import `MachineConfig`.

**Prompt**:
> Update `python/src/machine_config/adapters/__init__.py` to export `BaseAdapter`
> and define `REGISTRY: dict[tuple[str, str], type[BaseAdapter]] = {}` mapping
> `(from_version, to_version)` to the adapter class. Add a `register(cls)` decorator
> that inserts the class into `REGISTRY` keyed by `(from_version, to_version)` and
> returns the class unchanged.

**Prompt**:
> Update `tools/templates/adapter_python.py.j2` to match the raw-dict contract:
> - Remove `from ..models import MachineConfig`
> - Change `adapt(self, config: MachineConfig) -> MachineConfig:` to
>   `adapt(self, config: dict) -> dict:`
> - The field-transform logic already accesses `config["optical_trains"]` by key,
>   so only the signature and imports change.
> Add a comment at the top of `tools/generate_adapters.py` recording the raw-JSON
> design decision.

**Verify**:
```bash
.venv/Scripts/python.exe -c "from machine_config.adapters import BaseAdapter, REGISTRY, register; print('OK')"
```

---

### A.2 — Node.js: Wire Generated Adapters to the `Adapter` Interface (raw-dict approach) ✅ COMPLETE

**Prompt**:
> `nodejs/src/adapters/base.ts` defines an `Adapter` interface with `name: string`
> and `adapt(config: MachineConfig): MachineConfig`. Change the interface signature
> to operate on the raw parsed object before deserialization:
> `adapt(config: Record<string, unknown>): Record<string, unknown>;`
> Remove the `MachineConfig` import from `base.ts`.

**Prompt**:
> Update `tools/templates/adapter_typescript.ts.j2` to match the raw-dict contract:
> - Remove the `MachineConfig` import
> - Change the generated `adapt()` signature to
>   `adapt(config: Record<string, unknown>): Record<string, unknown>`
> - Update the field-transform logic to access
>   `(config["optical_trains"] as Record<string, unknown>[])` instead of
>   `config.optical_trains`
> - The generated class must still implement `Adapter` and carry a `name` property.

**Prompt**:
> `nodejs/src/adapters/index.ts` already has `registerAdapter` and `getChain`.
> Confirm the generated class export name matches what `index.ts` expects. If
> adapters are not auto-registered on import, add a comment explaining that each
> generated adapter must be manually imported and registered here.

**Verify**:
```bash
cd nodejs && npm run build
```
cd nodejs && npm run build
```
Build must complete with zero TypeScript errors (no adapters generated yet — this
just confirms the base wiring compiles).

---

### A.3 — Rust: Define Adapter Trait ✅ COMPLETE

**Prompt**:
> `rust/src/adapters/mod.rs` is a stub. Define an `Adapter` trait there with
> three required methods and expose it publicly:
> ```rust
> pub trait Adapter {
>     fn from_version(&self) -> &'static str;
>     fn to_version(&self) -> &'static str;
>     fn adapt(&self, config: serde_json::Map<String, serde_json::Value>)
>         -> serde_json::Map<String, serde_json::Value>;
> }
> ```
> **Do not use associated constants** (`const FROM_VERSION: &'static str`).
> Associated constants make traits non-dyn-compatible in Rust, which breaks
> `Vec<Box<dyn Adapter>>` in the registry. Use methods instead — they carry the
> same information and remain object-safe. Each generated struct will still expose
> `pub const FROM_VERSION` and `pub const TO_VERSION` for direct use outside the
> trait, but the trait itself uses methods.

**Prompt**:
> In `rust/src/adapters/registry.rs`, implement a `get_chain(from: &str, to: &str)`
> function that returns a `Vec<Box<dyn Adapter>>` representing the ordered sequence
> of adapters needed to migrate from `from` to `to`. For now it can return an empty
> `Vec` — the real chain registration comes in Phase B. Add a comment noting where
> individual adapters are added as they are generated.

**Prompt**:
> Ensure `rust/src/lib.rs` declares `pub mod adapters;` so the module is compiled.

**Verify**:
```bash
cd rust && cargo check
```
Must produce zero errors.

---

### A.4 — C++: Define Adapter Base Interface ✅ COMPLETE

**Prompt**:
> `cpp/include/machine_config/adapters/adapter.hpp` is currently a stub. Define a
> pure-virtual `IAdapter` interface class there using `nlohmann::json`:
> - Virtual destructor
> - `virtual std::string_view from_version() const noexcept = 0;`
> - `virtual std::string_view to_version() const noexcept = 0;`
> - `virtual nlohmann::json adapt(nlohmann::json config) const = 0;`

**Prompt**:
> In `cpp/include/machine_config/adapters/registry.hpp`, define an `AdapterRegistry`
> class with:
> - `void register_adapter(std::shared_ptr<IAdapter> adapter);`
> - `std::vector<std::shared_ptr<IAdapter>> get_chain(std::string_view from, std::string_view to) const;`
> Implement the registry using `std::vector` of shared_ptr. `get_chain` can return
> an empty vector for now. Include a `static AdapterRegistry& instance()` singleton.

**Prompt**:
> Update `tools/templates/adapter_cpp.hpp.j2` so the generated file defines a
> concrete class `V{{ spec.from_version | replace('.', '_') }}_to_V{{ spec.to_version | replace('.', '_') }}`
> that inherits from `IAdapter` and overrides all three methods. The `adapt()`
> body contains the existing field-transform logic. The class must be registered
> with `AdapterRegistry::instance()` via a static initializer or a free
> `register_*()` function at the bottom of the header.

**Verify**:
```bash
cmake --build cpp/build --target machine_config_cli
```
Must compile with zero errors. (No adapters generated yet.)

---

### Phase A Gate

All four of the following must pass before Phase B:

| Check | Command |
|---|---|
| Python base import | `.venv/Scripts/python.exe -c "from machine_config.adapters import BaseAdapter, REGISTRY, register"` |
| Node.js build | `cd nodejs && npm run build` |
| Rust check | `cd rust && cargo check` |
| C++ build | `cmake --build cpp/build --target machine_config_cli` |

---

## Phase B — Synthetic Validation Spec & Generator Validation

**Goal**: A synthetic test spec exercises every template branch (all five
implemented change types). The generator produces valid, compilable output in
all four languages. Template bugs are discovered and fixed here.

**Delivers**: `schema/adapters/test_v0_9_to_v1_0.yaml`, 4 generated adapter files,
confirmed compilation in all languages.

> **Why `"0.9"` → `"1.0"`?** The current schema is `"1.0"`. Using a fictional
> old `"0.9"` tag makes it unambiguous that this spec is test infrastructure and
> does not represent a real past or future version. No production code declares
> `"0.9"` as a known version.

---

### B.1 — Write the Synthetic Validation Spec ✅ COMPLETE

**Prompt**:
> Create `schema/adapters/test_v0_9_to_v1_0.yaml`. Add a comment at the top of
> the file: `# SYNTHETIC TEST SPEC — validates adapter machinery only. Not a real migration.`
> The spec must include exactly one entry of each implemented change type so that
> every template branch is exercised:
>
> ```yaml
> # SYNTHETIC TEST SPEC — validates adapter machinery only. Not a real migration.
> from_version: "0.9"
> to_version:   "1.0"
> changes:
>   - type: field_add
>     field: test_added_field
>     path: optical_trains[*]
>     default: null
>   - type: field_rename
>     old_field: test_old_name
>     new_field: test_new_name
>     path: optical_trains[*]
>   - type: field_remove
>     field: test_removed_field
>     path: optical_trains[*]
>   - type: field_move
>     field: test_move_field
>     from_path: optical_trains[*]
>     to_path: optical_trains[*].test_nested
>   - type: field_move_rename
>     old_field: test_move_rename_old
>     new_field: test_move_rename_new
>     from_path: optical_trains[*]
>     to_path: optical_trains[*].test_nested
> ```
>
> Use clearly synthetic field names (`test_*`) that cannot be confused with real
> schema fields. Both `field_move` and `field_move_rename` target a `test_nested`
> sub-object — the adapter must create it when absent. Adjust if any names collide
> with current fields.

---

### B.2 — Run the Generator and Fix Template Bugs ✅ COMPLETE

**Prompt**:
> Run `python tools/generate_adapters.py` from the repo root. The generator will
> pick up `test_v0_9_to_v1_0.yaml` and emit four files. Report any errors.
> Fix all issues in the **templates**, never in generated files directly.
> Common issues to look for:
> - Python: wrong relative import depth, `BaseAdapter` signature mismatch,
>   `dict` vs `MachineConfig` type annotation inconsistency
> - TypeScript: missing `implements Adapter`, wrong `adapt()` signature,
>   `MachineConfig` import still present after A.2 template update
> - Rust: struct not declared, trait impl missing, module not declared in `mod.rs`
> - C++: class not inheriting `IAdapter`, file not included in the build
> - All languages: `field_move` and `field_move_rename` require nested path
>   navigation; the templates must create the destination sub-object when absent
>   before writing the moved field.

**Verify — Python**:
```bash
.venv/Scripts/python.exe -c "from machine_config.adapters.test_v0_9_to_v1_0 import V0_9_to_V1_0; print(V0_9_to_V1_0.from_version)"
```

**Verify — Node.js**:
```bash
cd nodejs && npm run build
```

**Prompt**:
> Fix any Rust compile errors in the generated `rust/src/adapters/test_v0_9_to_v1_0.rs`.
> The generated file must declare `pub struct V0_9ToV1_0;` and implement the
> `Adapter` trait. Add `pub mod test_v0_9_to_v1_0;` to `rust/src/adapters/mod.rs`.

**Verify — Rust**:
```bash
cd rust && cargo check
```

**Prompt**:
> Fix any C++ errors in the generated
> `cpp/include/machine_config/adapters/test_v0_9_to_v1_0.hpp`. The generated class
> must inherit from `IAdapter`. Include it from `registry.hpp` or a dedicated
> `adapters_all.hpp` so it is compiled.

**Verify — C++**:
```bash
cmake --build cpp/build --target machine_config_cli
```

---

### B.3 — Confirm Generator Idempotency ✅ COMPLETE

**Prompt**:
> Add a `test_generator_is_idempotent` test to `python/tests/test_generator.py`.
> The test must: read the SHA-256 hash of each of the four generated files, run
> the generator a second time, then assert every hash is unchanged. This approach
> works regardless of git state and is self-contained.

**Verify**:
```bash
.venv/Scripts/python.exe -m pytest python/tests/test_generator.py::test_generator_is_idempotent -v
```

---

### B.4 — Update the Existing Generator CI Test ✅ COMPLETE

**Prompt**:
> `python/tests/test_generator.py` contains `test_generator_runs_with_empty_spec_dir`
> which asserts `"No adapter specs found"` in stdout. Now that
> `test_v0_9_to_v1_0.yaml` exists, this test will fail because the spec dir is no
> longer empty. Update the test to instead assert:
> - `result.returncode == 0`
> - The four expected output files were created (one per language)
> - The existing template-validity and change-type rendering tests are unchanged
>
> Do this in the same commit as the spec file — do not leave CI broken between commits.

**Verify**:
```bash
.venv/Scripts/python.exe -m pytest python/tests/test_generator.py -v
```

---

### Phase B Gate

| Check | Expected |
|---|---|
| `schema/adapters/test_v0_9_to_v1_0.yaml` exists with synthetic comment | ✓ |
| `test_generator.py` passes (updated for non-empty spec dir) | ✓ |
| Python adapter imports cleanly | ✓ |
| Node.js build passes | ✓ |
| Rust `cargo check` passes | ✓ |
| C++ builds | ✓ |
| `git diff` after second generator run | empty |

---

## Phase C — Synthetic Test Fixtures

> **Prerequisite (completed before C.1)**: The Python reader and writer were updated
> so that `schema_version` is stored as a root HDF5 attribute on write and read back
> on parse (falling back to `SCHEMA_VERSION` for legacy files). This enables the
> fixture generator to write `"0.9"` as a real HDF5 attribute that the dispatch layer
> will later detect. All 336 existing Python tests continue to pass unchanged.

**Goal**: Two test-only fixture files anchor all adapter tests: an HDF5 file
artificially tagged as `schema_version = "0.9"` (fictional), and a golden JSON
recording the expected state after the adapter upgrades it to `"1.0"`.
These files live under `fixtures/adapters/test/` — the path makes their
test-only purpose self-evident.

**Delivers**: `fixtures/adapters/test/reference_synthetic_v0_9.h5`,
`fixtures/adapters/test/expected_after_upgrade.json`.

---

### C.1 — Create the Synthetic v0.9 Fixture HDF5 ✅ COMPLETE

**Implementation**: `tools/generate_adapter_fixtures.py` — `generate_v0_9()` copies
`fixtures/reference_config.h5` with `shutil.copy2`, then uses h5py directly to stamp
`schema_version = "0.9"` as a root attribute and inject the 4 synthetic pre-migration
attributes onto every optical train group. Bypasses the typed writer to preserve fields
the model doesn't know about. 9 tests in `python/tests/test_adapters.py::TestSyntheticV09Fixture` pass.

---

### C.2 — Generate the Golden "After Upgrade" JSON ✅ COMPLETE

**Implementation**: `tools/generate_adapter_fixtures.py` — `generate_golden_json()`
reads the v0.9 HDF5 via `_read_raw_dict()` (h5py + numpy-to-Python conversion,
ALL attributes preserved), applies `V0_9_to_V1_0().adapt()` directly, sets
`meta.schema_version = "1.0"` (reflecting what the Phase D dispatch will do), and
writes `fixtures/adapters/test/expected_after_upgrade.json` as UTF-8 JSON.

Manual review confirmed all 5 change types correct:
- `test_added_field` present and `null` on each train ✓
- `test_new_name` present; `test_old_name` absent ✓
- `test_removed_field` absent ✓
- `test_move_field` absent at top level; present at `test_nested.test_move_field` ✓
- `test_move_rename_old` absent; `test_nested.test_move_rename_new` present ✓
- `meta.schema_version` is `"1.0"` ✓

13 tests in `python/tests/test_adapters.py::TestGoldenAfterUpgrade` pass.
Full Python suite: 358 tests pass.

---

### Phase C Gate ✅ COMPLETE

| File | Status |
|---|---|
| `fixtures/adapters/test/reference_synthetic_v0_9.h5` | ✅ exists — schema_version=0.9, synthetic fields present on both trains |
| `fixtures/adapters/test/expected_after_upgrade.json` | ✅ exists — schema_version=1.0, all 5 change types reflected |
| `python/tests/test_adapters.py` | ✅ 22 tests pass (9 C.1 + 13 C.2) |

---

## Phase D — Dispatch Layer

**Goal**: Each language's reader detects `meta.file_version` and applies the
correct adapter chain before deserializing into the model. `CURRENT_VERSION` is
**not changed** — it remains `"1.0"` in all four languages. The dispatch mechanism
is inert for all real production files (chain is empty when
`from_version == CURRENT_VERSION`). It only activates for the synthetic `"0.9"`
test fixture.

**Version attribute**: `File_Version` HDF5 attr / `meta.file_version` Python field
is the single dispatch key. There is no separate `schema_version`.

**Delivers**: Version detection + chain dispatch wired in all four readers,
dormant for all current files.

> **Critical implementation note**: Every sub-task below has a mandatory first
> step — implement the HDF5→raw dict parser — before wiring the adapter dispatch.
> Do NOT skip this step or substitute a typed-model serialisation round-trip.
> See the Design Decision section for why.

---

### D.1 — Python Dispatch ✅ COMPLETE

**Implementation**:
- `_to_native(v)` helper and `_hdf5_to_raw_dict(f)` added to `reader.py` — reads
  ALL root attrs + all optical train group attrs into a plain dict, preserving
  every attribute regardless of whether the current model knows it.
- `adapters/__init__.py` gains `get_chain(from_version, to_version)` using the
  `REGISTRY`; imports `V0_9_to_V1_0` eagerly so `@register` fires.
- `_parse()` dispatch: reads `File_Version` from HDF5, calls `get_chain`, and if
  the chain is non-empty builds the raw dict, runs all adapters, stamps
  `File_Version = CURRENT_VERSION`, then builds `MachineConfig` from the adapted
  attrs. If the chain is empty (all real files) the existing parse path is taken
  unchanged.
- `_parse_train()` gains an optional `attrs: dict | None` parameter; when set
  (adapter path) the adapted dict is used instead of `f[base_path].attrs`.
- Old duplicate `_hdf5_attrs_extra` removed; `MachineConfigMeta.schema_version`
  field removed; `File_Version` is the single version field in both HDF5 and model.

**Tests** (5 new in `test_adapters.py::TestDispatch`):
- `test_v0_9_fixture_file_version_upgraded` — dispatch fires, result is `"1.0"` ✓
- `test_real_file_file_version_unchanged` — chain empty, still `"1.0"` ✓
- `test_v0_9_fixture_returns_valid_machineconfig` — valid model, 2 trains ✓
- `test_real_file_returns_valid_machineconfig` — valid model ✓
- `test_v0_9_optical_train_data_intact` — real beam data survives adapter pass ✓

Full Python suite: **363 tests pass**.

---

### D.2 — Node.js Dispatch ✅ COMPLETE

**Implementation**:
- `hdf5ToRawDict(f: h5wasm.File)` added to `reader.ts` — snapshots all root attrs
  and all optical-train group attrs into a plain-JS dict (same structure as Python).
- `adaptedAttrStr`, `adaptedAttrNum`, `adaptedAttrBool` helpers added — parallel
  to the existing `attrStr`/`attrFloat`/`attrBool` h5wasm helpers, but read from a
  plain `Record<string, unknown>` produced by adapter output.
- `CURRENT_VERSION = "1.0"` constant added.
- `adapters/index.ts` gains `getChainFor(fromVersion, toVersion)` with a separate
  version-pair-keyed registry; `V0_9_to_V1_0` is imported and registered at
  module load time.
- `parseOpticalTrain` gets optional `adaptedAttrs: Record<string, unknown> | null`
  parameter; when non-null, uses `adaptedAttrStr`/`adaptedAttrNum`/`adaptedAttrBool`
  closures instead of h5wasm attrs — sub-group parsing (Scanner, etc.) unchanged.
- `parseFile` dispatch: reads `File_Version`, calls `getChainFor`, if chain non-empty
  builds raw dict, runs adapters, stamps `File_Version = CURRENT_VERSION`, builds
  typed model from adapted dicts. Empty chain takes the original path unchanged.
- `parseMeta` had `schema_version: "v1"` hardcoded — removed. `MachineConfigMeta`
  `schema_version` field removed from `models.ts`. `SCHEMA_VERSION` unused import
  removed from `builder.ts`; `schema_version` assignment removed from `build()`.

**Tests** (5 new in `tests/reader.test.ts` — `MachineConfigReader — adapter dispatch`):
- `v0.9 fixture is upgraded to file_version "1.0"` ✓
- `real fixture (already v1.0) has file_version "1.0" unchanged` ✓
- `v0.9 fixture parses to a valid MachineConfig with 2 optical trains` ✓
- `beam_waist_major survives the v0.9 → v1.0 upgrade intact` ✓
- `real fixture parses successfully with no adapters applied` ✓

Full Node.js suite: **135 tests pass**.

---

### D.3 — Rust Dispatch ✅ COMPLETE

**Implementation**:
- `hdf5_to_raw(f: &H5File) -> Result<Map<String, Value>>` added to `reader.rs` —
  snapshots all root attrs and all optical-train group attrs via the existing
  `read_raw`/`raw_to_json` pipeline into a plain serde_json map.
- `json_str`, `json_required_str`, `json_float`, `json_int`, `json_bool_from_int`,
  `collect_extra_from_json` helpers added — parallel to the HDF5 helpers, working
  on `&Map<String, Value>` produced by adapter output.
- `CURRENT_VERSION = "1.0"` replaces `EXPECTED_FILE_VERSION`; `SCHEMA_VERSION`
  constant removed entirely. `adapters` crate import added.
- `parse_inner` dispatch: reads `File_Version`, calls `adapters::registry::get_chain`,
  if chain non-empty builds raw map, runs adapters, stamps `File_Version = CURRENT_VERSION`,
  builds `MachineConfigMeta` from adapted map. Empty chain uses the existing HDF5 path.
- `parse_train` gets `adapted_attrs: Option<&Map<String, Value>>` parameter; four
  one-liner dispatch closures (`s`, `flt`, `u`, `b`) select between JSON helpers
  and HDF5 helpers. Sub-group parsing (Scanner, etc.) always uses HDF5.
- `check_file_version` method removed; unknown-version warning moved inside
  `parse_inner` (fires only when chain is empty and version is not current).
- `adapters/registry.rs` wires `V0_9ToV1_0` for `"0.9" → "1.0"`.
- `MachineConfigMeta.schema_version` field removed from `models.rs` and all
  construction sites (`builder.rs`, `models.rs` test helper, `reader.rs` inline test).
- `fixtures/reference_output.json` golden file updated to drop `schema_version`.

**Tests** (5 new in `tests/integration_test.rs`):
- `test_dispatch_v09_file_version_upgraded` — chain fires, result is `"1.0"` ✓
- `test_dispatch_real_file_version_unchanged` — chain empty, still `"1.0"` ✓
- `test_dispatch_v09_returns_two_trains` — valid model, 2 trains ✓
- `test_dispatch_v09_beam_waist_major_intact` — real beam data survives ✓
- `test_dispatch_real_file_parses_successfully` — no adapter applied ✓

Full Rust suite: **46 unit + 20 integration = 66 tests pass**.

---

### D.4 — C++ Dispatch ✅ COMPLETE

**Changes made**:
- `cpp/include/machine_config/models.hpp`: removed `schema_version` field from `MachineConfigMeta`, its `to_json` and `from_json` overloads.
- `cpp/include/machine_config/builder.hpp`: removed `cfg.meta.schema_version = "v1"` assignment.
- `cpp/include/machine_config/adapters/registry.hpp`: added `#include "test_v0_9_to_v1_0.hpp"`, implemented `get_chain` to return `V0_9_to_V1_0` for `"0.9" → "1.0"`.
- `cpp/include/machine_config/reader.hpp`: removed `SCHEMA_VERSION` constant; renamed `EXPECTED_FILE_VERSION` → `CURRENT_VERSION = "1.0"`; added `hdf5ToRaw()` static method; added JSON extraction helpers (`jsonStr`, `jsonRequiredStr`, `jsonFloat`, `jsonBoolFromInt`); updated `parseInner()` to dispatch via `AdapterRegistry`; added `parseMetaFromJson()` for the adapted path; updated `parseTrains()` and `parseTrain()` to accept and use adapted JSON.
- `cpp/tests/test_reader.cpp`: replaced `schema_version == "v1"` assertions with `file_version == "1.0"`; added 5 dispatch tests (49–53).
- `cpp/tests/test_models.cpp`, `cpp/tests/test_writer.cpp`: removed `schema_version` field assignments and JSON key assertions.

**Result**: 69/69 C++ tests pass, including all 5 new dispatch tests.

---

### Phase D Gate — 4-Language Battle Test

Executed after D.4 is complete. All four steps must pass before declaring Phase D done.

#### Step 1 — Each language reads the synthetic v0.9 fixture

`File_Version = "0.9"` → adapter must fire → output shows `file_version = "1.0"`.

```bash
# Python
.venv/Scripts/python.exe -c "
from machine_config import MachineConfigReader
cfg = MachineConfigReader('fixtures/adapters/test/reference_synthetic_v0_9.h5').parse()
print('file_version:', cfg.meta.file_version)   # expects 1.0
print('trains:', len(cfg.optical_trains))        # expects 2
"

# Node.js
node -e "
const { MachineConfigReader } = require('./nodejs/dist/index.js');
new MachineConfigReader('fixtures/adapters/test/reference_synthetic_v0_9.h5')
  .parse().then(c => {
    console.log('file_version:', c.meta.file_version);  // expects 1.0
    console.log('trains:', c.optical_trains.length);     // expects 2
  });
"

# Rust (run from repo root)
cd rust && cargo run -- export-json ../fixtures/adapters/test/reference_synthetic_v0_9.h5 \
  | python -c "import sys,json; d=json.load(sys.stdin); print('file_version:', d['meta']['file_version'])"

# C++ (run from repo root)
./cpp/build/machine_config_cli export-json fixtures/adapters/test/reference_synthetic_v0_9.h5 \
  | python -c "import sys,json; d=json.load(sys.stdin); print('file_version:', d['meta']['file_version'])"
```

Expected: all four print `file_version: 1.0`.

---

#### Step 2 — Each language reads the real `reference_config.h5`

`File_Version = "1.0"` → no adapter fires → output valid and unchanged.

```bash
# Python
.venv/Scripts/python.exe -c "
from machine_config import MachineConfigReader
cfg = MachineConfigReader('fixtures/reference_config.h5').parse()
print('file_version:', cfg.meta.file_version)   # expects 1.0
print('trains:', len(cfg.optical_trains))
"

# Node.js
node -e "
const { MachineConfigReader } = require('./nodejs/dist/index.js');
new MachineConfigReader('fixtures/reference_config.h5')
  .parse().then(c => console.log('file_version:', c.meta.file_version));
"

# Rust
cd rust && cargo run -- export-json ../fixtures/reference_config.h5 \
  | python -c "import sys,json; d=json.load(sys.stdin); print('file_version:', d['meta']['file_version'])"

# C++
./cpp/build/machine_config_cli export-json fixtures/reference_config.h5 \
  | python -c "import sys,json; d=json.load(sys.stdin); print('file_version:', d['meta']['file_version'])"
```

Expected: all four print `file_version: 1.0`, no warnings.

---

#### Step 3 — Cross-language round-trip

Python writes a v1.0 file; all other languages read it and return the same `machine_name`.

```bash
# Write
.venv/Scripts/python.exe -c "
from machine_config.builder import MockConfigBuilder
from machine_config.writer import MachineConfigWriter
cfg = MockConfigBuilder().build()
MachineConfigWriter(cfg).write('/tmp/cross_test.h5')
print('wrote:', cfg.meta.machine_name)
"

# Node.js reads
node -e "
const { MachineConfigReader } = require('./nodejs/dist/index.js');
new MachineConfigReader('/tmp/cross_test.h5')
  .parse().then(c => console.log('Node.js OK:', c.meta.machine_name));
"

# Rust reads
cd rust && cargo run -- export-json /tmp/cross_test.h5 \
  | python -c "import sys,json; d=json.load(sys.stdin); print('Rust OK:', d['meta']['machine_name'])"

# C++ reads
./cpp/build/machine_config_cli export-json /tmp/cross_test.h5 \
  | python -c "import sys,json; d=json.load(sys.stdin); print('C++ OK:', d['meta']['machine_name'])"
```

Expected: all four print the same `machine_name`.

---

#### Step 4 — Schema validation of the adapted output

```bash
.venv/Scripts/python.exe tools/cross_check.py \
  fixtures/adapters/test/reference_synthetic_v0_9.h5
```

Expected: no schema violations reported.

---

#### Gate pass criteria

- All 4 languages: `file_version = "1.0"` for the v0.9 fixture (Step 1)
- All 4 languages: `file_version = "1.0"` for the real fixture, no warnings (Step 2)
- All 4 languages: identical `machine_name` from the Python-written file (Step 3)
- `cross_check.py` reports zero violations (Step 4)

## ✅ Phase D Gate — PASSED

**Executed**: 2026-08-12  
**Command**: `python tools/cross_check.py --verbose`  
**Changes made**:
- `tools/cross_check.py`: added `adapter_v09` fixture (`fixtures/adapters/test/reference_synthetic_v0_9.h5`) to `FIXTURES` dict so all phases exercise adapter dispatch.
- Rebuilt Rust release binary (`cargo build --release`) to pick up D.3 dispatch changes.
- Rebuilt Node.js dist (`npm run build`) to pick up D.2 dispatch changes.

**Results**:
- Phase 1 (Schema Validation): 16/16 PASS — 4 languages × 4 fixtures
- Phase 2 (Read Parity): 24/24 PASS — all 4 languages agree on all 4 fixtures (including v0.9 → v1.0 upgrade)
- Phase 3 (Write Interop): 12 parity + 4 fidelity PASS — 4 writers × 4 readers
- Phase 3.5 (Binary Copy): 32/32 PASS — 4 writers × 4 readers × 2 directions
- Phase 4 (Correction Hashes): 48/48 PASS — 4 languages × 4 fixtures × 2 directions
- **All 5 active phases passed.**

---

## Phase E — Unit Tests

**Goal**: Each language has dedicated unit tests that verify all five implemented
change types independently of the dispatch layer and file I/O. Tests operate
directly on dict/map inputs — no HDF5 file needed.

**Delivers**: Adapter unit tests in all four language test suites.

---

### E.1 — Python: `python/tests/test_adapters.py` ✅ COMPLETE

**Changes made**: Added 8 pure-unit test functions and `REGISTRY`/`get_chain`/`V0_9_to_V1_0` imports to the existing `python/tests/test_adapters.py` (which already held Phase C fixture and Phase D.1 dispatch tests).

**Result**: 35/35 tests pass (8 new E.1 unit tests + 27 pre-existing tests).

---

### E.2 — Node.js: `nodejs/tests/adapters.test.ts` ✅ COMPLETE

**Changes made**: Created `nodejs/tests/adapters.test.ts` with 8 pure-unit tests using Vitest, importing `V0_9_to_V1_0`, `fromVersion`, `toVersion` from the adapter and `getChainFor` from the registry. All tests operate on inline `Record<string, unknown>` objects with no HDF5 I/O.

**Result**: 8/8 tests pass.

---

### E.3 — Rust: unit tests in `rust/src/adapters/test_v0_9_to_v1_0.rs` ✅ COMPLETE

**Changes made**: Added a `#[cfg(test)] mod tests` block to the end of `rust/src/adapters/test_v0_9_to_v1_0.rs` with 7 unit tests using `serde_json::json!`. All tests operate on inline JSON maps with no HDF5 I/O.

**Result**: 7/7 tests pass.

---

### E.4 — C++: `cpp/tests/test_adapters.cpp` ✅ COMPLETE

**Changes made**: Created `cpp/tests/test_adapters.cpp` with 7 Catch2 unit tests. Added `test_adapters.cpp` to `cpp/tests/CMakeLists.txt`.

**Result**: 7/7 tests pass.

---

### Phase E Gate

| Language | Command | Expected |
|---|---|---|
| Python | `.venv/Scripts/python.exe -m pytest python/tests/test_adapters.py -v` | 8 passed |
| Node.js | `cd nodejs && npx vitest run tests/adapters.test.ts` | 8 passed |
| Rust | `cd rust && cargo test adapters` | 7 passed |
| C++ | `ctest --test-dir cpp/build -R test_adapters` | 7 passed |

---

## Phase F — Integration & Cross-Language Parity Tests

**Goal**: End-to-end verification. Each language's reader auto-upgrades the
synthetic `"0.9"` file and produces output matching the golden fixture. Real
production files are verified to pass through with zero adapter activity.
All four languages produce identical output (parity).

**Delivers**: Integration tests per language + Phase 5 in `cross_check.py`.

---

### F.1 — Python Integration Test ✅ COMPLETE

**Design note**: The test adapter's `test_*` fields are synthetic — intentionally
not in the typed schema — so they cannot be asserted through `MachineConfig`
or `export-json`. Field-level adapter correctness is fully covered by the E.1
unit tests operating on raw dicts. F.1 verifies what the **typed reader layer**
can prove: the version stamp is applied, real schema fields survive unchanged,
and real files pass through with zero adapter activity.

**Result**: All F.1 assertions were already present in `TestDispatch` from Phase D.1 — no new code required. 5/5 tests pass:

| Test | Covers |
|---|---|
| `test_v0_9_fixture_file_version_upgraded` | version stamp applied |
| `test_v0_9_fixture_returns_valid_machineconfig` | 2 trains present |
| `test_v0_9_optical_train_data_intact` | beam fields survive adapter |
| `test_real_file_file_version_unchanged` | no adapter fires on real file |
| `test_real_file_returns_valid_machineconfig` | real file parses correctly |

---

### F.2 — Node.js Integration Test ✅ COMPLETE

**Design note**: Same scope as F.1 — typed-layer assertions only.

**Result**: Fully covered by the `MachineConfigReader — adapter dispatch` describe block added in Phase D.2 (`nodejs/tests/reader.test.ts`). No new code required. 87/87 tests pass.

| Test | Covers |
|---|---|
| `v0.9 fixture is upgraded to file_version "1.0"` | version stamp |
| `real fixture (already v1.0) has file_version "1.0" unchanged` | no adapter fires |
| `beam_waist_major survives the v0.9 → v1.0 upgrade intact` | real fields survive |
| `real fixture parses successfully with no adapters applied` | passthrough |

---

### F.3 — Rust Integration Test ✅ COMPLETE

**Design note**: Same scope as F.1 — typed-layer assertions only.

**Result**: Fully covered by the dispatch tests added in Phase D.3 (`rust/tests/integration_test.rs`). No new code required. 5/5 dispatch tests pass.

| Test | Covers |
|---|---|
| `test_dispatch_v09_file_version_upgraded` | version stamp |
| `test_dispatch_real_file_version_unchanged` | no adapter fires |
| `test_dispatch_v09_returns_two_trains` | train count |
| `test_dispatch_v09_beam_waist_major_intact` | real fields survive |
| `test_dispatch_real_file_parses_successfully` | passthrough |

---

### F.4 — C++ Integration Test ✅ COMPLETE

**Design note**: Same scope as F.1 — typed-layer assertions only.

**Result**: Fully covered by the dispatch tests added in Phase D.4 (`cpp/tests/test_reader.cpp`). No new code required. 5/5 dispatch tests pass.

| Test | Covers |
|---|---|
| `dispatch: v0.9 fixture is upgraded to file_version 1.0` | version stamp |
| `dispatch: real fixture file_version unchanged` | no adapter fires |
| `dispatch: v0.9 fixture returns 2 optical trains` | train count |
| `dispatch: beam_waist_major survives upgrade` | real fields survive |
| `dispatch: real fixture parses successfully` | passthrough |

---

### F.5 — `cross_check.py` Phase 5: Adapter Parity ✅ COMPLETE

**Changes made**: Added `phase_adapter_parity()` to `tools/cross_check.py`, `--skip-adapter-parity` flag, and wired the phase into `main` as Phase 5.

**Result**: `All 6 active phase(s) passed.`
- 4 languages × `file_version == "1.0"` verified
- 3 parity diffs (python vs rust/nodejs/cpp) all empty

---

### Phase F Gate ✅ COMPLETE

All F.1–F.5 requirements were satisfied by existing D.1–D.4 dispatch tests plus the new F.5 Phase 5 in `cross_check.py`.

| Check | Result |
|---|---|
| Python integration tests | ✅ 5 tests in `TestDispatch` (D.1) |
| Node.js integration tests | ✅ 4 tests in `adapter dispatch` describe (D.2) |
| Rust integration tests | ✅ 5 `test_dispatch_*` tests (D.3) |
| C++ integration tests | ✅ 5 `dispatch:` TEST_CASEs (D.4) |
| `cross_check.py` Phase 5 | ✅ 4 languages agree; all `file_version == "1.0"` |

---

## Phase G — CI Hardening

**Goal**: The adapter system is enforced in CI. Stale generated files and
broken adapter chains are caught automatically on every PR.

---

### G.1 — Generator Idempotency Check

**Prompt**:
> Add a step to `.github/workflows/cross_check.yml` (after the Python install step)
> that runs:
> ```bash
> python tools/generate_adapters.py
> git diff --exit-code -- nodejs/src/adapters/ rust/src/adapters/ python/src/machine_config/adapters/ cpp/include/machine_config/adapters/
> ```
> If the diff is non-empty, fail the job with a message: "Generated adapter files
> are out of sync with their specs. Run `python tools/generate_adapters.py` and
> commit the result."

---

### G.2 — Add Adapter Tests to Existing CI Workflows

**Prompt**:
> In `.github/workflows/python.yml`, confirm that `pytest python/tests/` already
> covers `test_adapters.py`. If not, add it explicitly.
>
> In `.github/workflows/nodejs.yml`, confirm `npm test` covers `adapters.test.ts`.
>
> In `.github/workflows/rust.yml`, confirm `cargo test` runs the `adapter_dispatch`
> integration test. If tests are filtered, ensure adapter tests are included.
>
> In `.github/workflows/cpp.yml`, confirm the CTest run includes
> `test_adapters` and `test_adapter_dispatch`.

---

### G.3 — Add Phase 5 to Cross-Language CI

**Prompt**:
> In `.github/workflows/cross_check.yml`, confirm the step that invokes
> `python tools/cross_check.py` does not pass `--skip-adapter-parity`. If it did
> in a temporary workaround, remove that flag now.

---

### Phase G Gate

Create a PR with a trivial whitespace change to `schema/adapters/v1_0_to_v1_1.yaml`
(do not run the generator). The CI must:
1. Fail the generator idempotency check ✓
2. Pass all other checks ✓

Revert the whitespace change, re-run the generator, push — CI must go fully green.

---

## Adding Future Schema Versions (Steady-State Process)

Once this plan is complete, every future real schema change follows this checklist:

1. **Classify the change**: additive fields (minor bump) or breaking restructure (major bump)?
2. **Create `schema/machine_config_vN.schema.json`**: new file, never edit the old one
3. **Write `schema/adapters/vX_Y_to_vX_Z.yaml`**: cover all changed fields; use the
   appropriate change type from the *Change Type Taxonomy* section above; use only
   implemented types — the generator errors on reserved types
4. **Run `python tools/generate_adapters.py`**: commit all 4 generated files together
5. **Update `CURRENT_VERSION` in each reader**: one-line change × 4 files — this is
   the only production change; everything else is already proven by this plan
6. **Update `get_chain` / registry** in each language to include the new adapter
7. **Run `tools/generate_adapter_fixtures.py`**: regenerate test fixtures if needed
8. **Run the full test suite**: `python tools/cross_check.py --verbose`
9. **Bump library semver**: minor for non-breaking, major for breaking
10. **Update `CHANGELOG.md`**: record schema version → library version mapping

---

## Test Suite Additions Summary

| Suite | File | Phase | Command |
|---|---|---|---|
| Adapter unit tests (Python) | `python/tests/test_adapters.py` | E.1 | `.venv/Scripts/python.exe -m pytest python/tests/test_adapters.py -v` |
| Adapter dispatch integration (Python) | `python/tests/test_adapters.py` | F.1 | `.venv/Scripts/python.exe -m pytest python/tests/test_adapters.py -v -k dispatch` |
| Adapter unit tests (Node.js) | `nodejs/tests/adapters.test.ts` | E.2 | `cd nodejs && npx vitest run tests/adapters.test.ts` |
| Adapter dispatch integration (Node.js) | `nodejs/tests/reader.test.ts` | F.2 | `cd nodejs && npx vitest run tests/reader.test.ts` |
| Adapter unit tests (Rust) | `rust/src/adapters/test_v0_9_to_v1_0.rs` | E.3 | `cd rust && cargo test adapters` |
| Adapter dispatch integration (Rust) | `rust/tests/adapter_dispatch.rs` | F.3 | `cd rust && cargo test --test adapter_dispatch` |
| Adapter unit tests (C++) | `cpp/tests/test_adapters.cpp` | E.4 | `ctest --test-dir cpp/build -R test_adapters` |
| Adapter dispatch integration (C++) | `cpp/tests/test_adapter_dispatch.cpp` | F.4 | `ctest --test-dir cpp/build -R test_adapter_dispatch` |
| Cross-language adapter parity | `tools/cross_check.py` Phase 5 | F.5 | `.venv/Scripts/python.exe tools/cross_check.py --verbose` |
