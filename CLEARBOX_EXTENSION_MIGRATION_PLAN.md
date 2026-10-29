# ClearBox Extension Migration Plan

Status: Phase 0 (contract 0.1.1 tables) and Phase 1 (core makes ClearBox opaque) are implemented in all five
languages, uncommitted; see the Phase 1 status note in section 6. Phases 2 and 3 are not started.

Goal: extract ClearBox out of core's typed `MachineConfig` model and into a standalone
`mcf-ext-clearbox` package — a pure pack/unpack codec over the extension contract, the same
shape `mcf-ext-opcua` already is for `TM_OPCUA` — so that `rdt-core` and `clearbox-tauri` both
consume ClearBox data through one stable, versioned interface instead of each depending on
core's internal model directly.

This is a planning document only. It records the target architecture, the phases to get there,
what is already true today vs. what has to change, and the open questions that need an answer
before Phase 0 work starts.

## Contents

- [1. Why this mirrors mcf-ext-opcua, and where it differs](#1-why-this-mirrors-mcf-ext-opcua-and-where-it-differs)
- [2. Current state](#2-current-state)
- [3. Target end state](#3-target-end-state)
- [4. The File_Version 1.0 question — resolved](#4-the-file_version-10-question--resolved)
- [5. Blocking prerequisite: compound datasets in the contract](#5-blocking-prerequisite-compound-datasets-in-the-contract)
- [6. Phases](#6-phases)
- [7. Scope boundary: LightSource.power_characterization stays in core](#7-scope-boundary-lightsourcepower_characterization-stays-in-core)
- [8. Consumers](#8-consumers)
- [9. Open questions](#9-open-questions)

---

## 1. Why this mirrors mcf-ext-opcua, and where it differs

`docs/EXTENSION_CONTRACT.md` already names the destination: `ClearBox` is listed in "Known
extensions" as "not yet extracted ... planned as `mcf-ext-clearbox` once compound datasets are
supported", and `mcf-ext-opcua`'s own `DESIGN.md` states core's typed `opcua` *and* ClearBox
models "are deprecated while the extensions land and removed at its next major release." This
plan is the concrete path to that already-stated destination — it does not invent a new target.

Where ClearBox differs from OPC UA, concretely:

- OPC UA was already an opaque, core-agnostic group (`OPCUA/` at root in v1.0, `Extensions/TM_OPCUA/`
  in v1.1) — core never had a typed `opcua` *write* path tied into `MachineConfig` the way it has a
  typed `ClearBox` struct today. ClearBox is still a first-class typed field of
  `OpticalTrain.optional_components` in every language. Removing it is a StableModel change, not
  just a new codec sitting next to an already-opaque tree.
  (`rust/src/models.rs`, `nodejs/src/models.ts`, equivalents in Python/Go/C++.)
- OPC UA's data is scalar attributes and groups only. ClearBox's `Power_Characterization` and
  `Synchronous_Sensors` use **compound datasets** (`EquationConstant`, `CalibrationPoint` rows),
  which the extension contract cannot carry yet — this is the explicit blocker named in
  `docs/EXTENSION_CONTRACT.md`'s "Not yet done" section (see §5).
- ClearBox's correction grids (`Correction_Data`, `Inverse_Correction_Data`, each a `257×257×2`
  `f64` grid) and the `.fc3` raw bytes are large binary payloads today loaded eagerly
  (`docs/EXTENSION_CONTRACT.md` notes compression/chunking is also not yet done) — size matters
  more here than it ever did for OPC UA's config-sized trees.

## 2. Current state

- Core (all 5 languages) models `ClearBox`, `PowerCharacterization`, `SynchronousSensor`,
  `EquationConstant`, `CalibrationPoint` as typed StableModel structs
  (`rust/src/models.rs` lines ~146-270 and equivalents).
- `ClearBox` is in `MODELLED_EXTENSION_NAMES` (`rust/src/capabilities/v1_1/layout.rs`:
  `MODELLED_EXTENSION_NAMES: &[&str] = &["ClearBox"]`) — the extension contract explicitly
  forbids any host from writing that name into `extensions`; core owns it today.
- v1.0 reads/writes it at `Optical_Trains/<id>/Optional_Components/ClearBox/` (one full copy per
  train, including duplicated `Output_Path`/`Software_Trigger_Delay`).
- v1.1 reads/writes it at `Extensions/ClearBox/` (shared `Output_Path`/`Software_Trigger_Delay`) +
  `Extensions/ClearBox/<train_id>/` (per-train fields + `Power_Characterization/`), per
  `docs/migrations/v1_0_to_v1_1.md` Change 1 — including the hard-error "Consolidate conflict"
  rule when trains disagree on the two shared values.
- `clearbox-tauri` reads ClearBox today through core's public facade as the typed
  `optical_trains[i].optional_components.clearbox` field (`docs/clearbox-tauri-integration.md`).
- `rdt-core` (NestJS backend in `rdt-core/api`) both reads and writes MCF files today via
  `machine-config-library` (v0.2.0-rc.9) and `mcf-ext-opcua` (v1.0.0-rc.2), both installed from
  GitHub release tarball URLs in `api/package.json`. ClearBox touchpoints (see §8):
  - **Write (DB -> .h5):** `machine-config-library-mapper.helper.ts` `buildClearBox()` builds the typed
    `LibClearBox` (incl. `correction_data`, `inverse_correction_data`, `power_characterization`) from
    an internal v1.0-shaped `hdf5Structure`; `machine-config-generator.helper.ts` writes it with
    `MachineConfigWriter(config, targetVersion)`, where `targetVersion` is the machine's configured
    `mcf_version` (so rdt-core really does emit both 1.0 and 1.1 files).
  - **Read (.h5 -> DB):** `machine-config-reader.helper.ts` calls `parse({ includeBinary: true })`;
    `config-import-matching.service.ts`, `config-import.service.ts` and `CLEARBOX_FIELD_MAP` in
    `config-import-field-map.ts` read `train.optional_components.clearbox`.
  - **OPC UA already follows the target pattern:** `opcua-extension/machine-opcua-extension.codec.ts`
    maps DB rows <-> `mcf-ext-opcua` model and packs/unpacks `config.extensions['TM_OPCUA']`.
  - **Hash:** rdt-core computes `configuration_hash` itself from `hdf5Structure` (+ an `opcua`
    payload); ClearBox is already inside that hash independent of the MCF model.

## 3. Target end state

- Core stops modeling `ClearBox` (and its nested `PowerCharacterization`/`SynchronousSensor`/
  `EquationConstant`/`CalibrationPoint` types, *except* where `LightSource` still needs them —
  see §7) as a typed StableModel field, in every language, at the next major release.
- `ClearBox` is removed from `MODELLED_EXTENSION_NAMES`; core carries it the same way it already
  carries `TM_OPCUA` — an opaque `ExtensionTree` under `MachineConfig.extensions["ClearBox"]`.
- A new repo, `mcf-ext-clearbox`, provides `pack`/`unpack` between that tree and a typed
  `ClearBoxConfig` domain model — pure, no HDF5, no core dependency, declaring the extension
  contract's tree types structurally (same pattern as `mcf-ext-opcua`'s §2.4 "Distribution").
  Public interface shape mirrors `mcf-ext-opcua`'s exactly:
  `EXTENSION_NAME = 'ClearBox'`, `REQUIRES_CONTRACT`, `supportedSchemaVersions()`,
  `peekSchemaVersion()`, `unpack()`, `pack()`.
- Both `rdt-core` and `clearbox-tauri` depend on `mcf-ext-clearbox` (Node and Rust respectively)
  instead of (or alongside, for `clearbox-tauri`) reading `machine-config`'s typed `ClearBox`
  field. See §8.

## 4. The File_Version 1.0 question — resolved

Per your framing: **versions of MCF before ClearBox was migrated to `Extensions/` should still
have ClearBox handled for them — by core, not by the new extension package.** This is exactly how
`TM_OPCUA` already works today for v1.0's `OPCUA/` root group
(`docs/EXTENSION_CONTRACT.md`: *"File_Version 1.0 has no Extensions/ group. Core exposes the
legacy OPCUA/ group as extensions["TM_OPCUA"], writes a TM_OPCUA tree back to OPCUA/, and rejects
any other name for 1.0."*). The plan extends that identical mechanism to `ClearBox`:

- Core's v1.0 **reader** synthesizes `extensions["ClearBox"]` directly from
  `Optical_Trains/<id>/Optional_Components/ClearBox/`, producing the *same tree shape* v1.1
  already writes on disk (`Extensions/ClearBox/` shared attrs + `Extensions/ClearBox/<train_id>/`
  per-train subtree) — taking `Output_Path`/`Software_Trigger_Delay` from the **first optical train
  that has a ClearBox** (trains are not expected to disagree, so this is a tie-break, not a policy;
  the manifest's hard-error rule is not replayed on read, so a stray disagreeing v1.0 file still
  opens), and dropping the same fields the migration already drops
  (`Correction_Grid_Domain_Shape`, `Inverse_Grid_Domain_Shape`, `Custom_Video_Format`,
  `Selected_Camera`, `Show_Console`, `Video_Output` — see that manifest's field table, rows 4/5/8/
  12/15/17) so `mcf-ext-clearbox` never has to know which source version a tree came from.
- Core's v1.0 **writer** does the inverse: given an `extensions["ClearBox"]` tree, fan it back out
  to each train's `Optional_Components/ClearBox/` subtree, re-duplicating the two shared attrs.
- `mcf-ext-clearbox` never looks at `File_Version` and never special-cases v1.0 — by the time a
  tree reaches it, v1.0 and v1.1 are indistinguishable, same as `TM_OPCUA` today. Trees core
  synthesizes this way carry no schema-version attribute, so they come through
  `mcf-ext-clearbox`'s **legacy adapter** (same concept as `mcf-ext-opcua` §5's "Legacy (no
  version attribute)" adapter) — which in ClearBox's case is actually the *common* case for v1.0
  files, not a historical corner case.
- Net effect: core keeps permanent, first-class v1.0 ClearBox support — it just stops doing it
  through a public typed struct.
- **This is more work than TM_OPCUA's equivalent, not the same work.** v1.0's `OPCUA/` -> `TM_OPCUA` is a
  verbatim subtree move. v1.0 ClearBox -> tree is a real transform (per-train fan-in, Consolidate check,
  dropped fields, `Volts_To_Watts_*` -> `Power_Characterization`). The v1.1 writer already implements
  exactly that from a typed `ClearBox`. Cheapest path: keep a **core-internal** (non-exported) typed
  ClearBox used only by the v1.0 adapter and the existing v1.1 logic, and add one converter
  typed <-> tree, rather than writing the v1.0 mapping a second time. The public StableModel still loses
  `ClearBox`. Decision needed (§9).
- Tree is keyed by `train_id` (`Optical_Train_NN`, `layout.train_id(index)`), so the v1.0 synthesis and
  the extension must agree on ids; the extension model should key trains by that string, not by index.

## 5. Blocking prerequisite: compound datasets in the contract

`docs/EXTENSION_CONTRACT.md`'s "Not yet done" section is explicit that this blocks ClearBox's
extraction:

> Compound datasets: ClearBox's `Power_Characterization` and `Synchronous_Sensors` are compound
> datasets (`{name: S64 fixed-length string, value: f64}` and `{input_value, output_value}` f64
> pairs), which the contract cannot carry yet. Required before ClearBox moves out of core.

Today's `ExtensionTree`/`Dataset` shape (`docs/EXTENSION_CONTRACT.md` §"Tree") only supports
single-dtype numeric/utf8 arrays — not the named-row compound shapes
`EquationConstant{name, value}` and `CalibrationPoint{input_value, output_value}` that
`Power_Characterization` and `Synchronous_Sensors` both use. This has to land in
`extension-contract/` (Rust) and `nodejs/src/extensions/` **before** Phase 1 below, as its own
reviewed contract change (version bump per `EXTENSION_CONTRACT_VERSION`'s additive-within-major
rule), with conformance fixtures the same way the rest of the tree shape is pinned today.

### Hard constraint: do not break `mcf-ext-opcua`

The change must be purely additive. Concretely:

- **Existing types unchanged.** `ExtensionDataset` (numeric only) and the three existing tree keys stay as
  they are; `mcf-ext-opcua` copies these types structurally and converts at the boundary. Compound data is
  a **new, optional** sibling (working name `tables?: Record<string, ExtensionTable>`, rows = named
  fields with per-field dtype incl. fixed-length `utf8`, e.g. `{name: utf8, value: f64}`), omitted from
  JSON and from `emptyExtensionTree()` when empty. Not a widening of `datasets` into a union.
- **Digest unchanged for existing trees.** A tree with no tables must hash byte-identically, so the pinned
  conformance digest `8d23637b...fb890` and every OPC UA digest stay valid. Encode tables only when
  non-empty, behind a reserved marker (e.g. `u32 0xFFFFFFFF`, then count and tables), never as a new
  always-present count. Add a second pinned digest for a tree that has tables.
- **Version range.** `mcf-ext-opcua` declares `REQUIRES_CONTRACT = '^0.1'`, which in semver means
  `>=0.1.0 <0.2.0`. Bumping the contract to `0.2.0` would put OPC UA outside its declared range even
  though nothing broke. Ship compound support as `0.1.x` (additive), have `mcf-ext-clearbox` require
  `>=0.1.N <0.2`, and document that within `0.1` changes are additive-only. Note core currently exports
  `EXTENSION_CONTRACT_VERSION` but has no `assertContractCompatible` helper, so today the range is
  declared but not enforced; any enforcement added later must accept the above.
- **Hosts that don't know tables.** Reading a file with a compound dataset must not fail for a host
  running an older core; define behavior (error with node path vs. skip) and make sure it only affects
  files that actually contain compound data, never OPC UA-only files.
- **Gate:** all existing extension conformance tests, fixtures (`extension_sample_*_v1_1.h5`,
  `extension_sample_tree.json`) and the OPC UA cross-language digests pass unmodified before and after.

Also still open per the same section, lower priority but worth deciding now vs. deferring:
dataset compression/chunking (ClearBox grids are ~1 MB each, `.fc3` is ~1.1 MB/train — currently
read eagerly and uncompressed) and the session-facade `extensions` getter.

## 6. Phases

**Phase 0 — contract: compound datasets (blocking, in `Machine_Config_Library`).**
*Status: implemented in all five languages as contract `0.1.1`. Optional `tables`
map, digest unchanged for trees without tables, existing fixtures and pinned digests untouched; see
`docs/EXTENSION_CONTRACT.md` "Tables". Rust, Go and C++ use the HDF5 C API for the row bytes, so
arbitrary compound shapes work rather than a closed set. Every language asserts the same pinned
digest and reads the Node- and Rust-written fixtures; `export-json` agrees across Node, Rust, Go and
Python (C++ is covered by its digest and golden-JSON tests; its CLI was not run). Not yet committed
or reviewed.*
Add a compound/row-dataset kind to the extension contract (Rust `extension-contract` crate + Node
`nodejs/src/extensions/`), with conformance fixtures proving `EquationConstant`/`CalibrationPoint`
round-trip through it. No consumer-visible change yet.

**Phase 1 — core: ClearBox becomes opaque (breaking release, all 5 languages).**
*Status: implemented in all five languages (working tree, not committed). Decisions taken: `OpticalTrain.optional_components`
is removed entirely; the get/set clearbox facade methods and `get_correction_data` / `get_inverse_correction_data` are
removed; the CLI `correction-hash` stays and reads the grid from the tree; v1.0-only per-train fields ride along as
extra per-train attrs; for v1.0 the first train supplies `Output_Path` / `Software_Trigger_Delay` (no Consolidate
conflict error); `MODELLED_EXTENSION_NAMES` is gone (core models no extension). Shim: `rust/src/clearbox_legacy.rs`,
`nodejs/src/clearboxLegacy.ts`, `python/src/machine_config/clearbox_legacy.py`,
`go/internal/models/clearbox_legacy.go`, `cpp/include/machine_config/clearbox_legacy.hpp`. Consequences: every
configuration_hash of a file with ClearBox changed (it now hashes the extension digest; `reference_config.h5` is
`4fd9e246...e6bd`, `reference_config_v1_1.h5` is `dd87533b...eb4a`); the correction-grid hash is unchanged
(`b3b95bf5...cebc6` forward train 0); `fixtures/reference_output.json` is regenerated with extension dataset
`data` elided and every language's golden comparison strips `data` on both sides; default `export-json` now
carries the ClearBox grids (about 14 MB), so `tools/cross_check.py` hashes large dataset `data` before diffing and
Rust enables serde_json `float_roundtrip` (without it JSON round trips lost one ulp). Examples, the five
`docs/validation` apps and the language docs are updated; all cross_check phases pass for the five CLIs.*
- Remove `ClearBox`/`PowerCharacterization`(ClearBox's use)/`SynchronousSensor`/
  `EquationConstant`/`CalibrationPoint` from the StableModel, per
  `docs/contributing.md` §3's "Remove a field — forbidden without a major breaking change review"
  — this *is* that review.
- Drop `ClearBox` from `MODELLED_EXTENSION_NAMES`.
- v1.1 reader/writer: map `Extensions/ClearBox/...` straight to/from `ExtensionTree`, no typed
  layer in between (mechanical — the layout doesn't change, only who interprets it).
- v1.0 reader/writer: synthesize/fan-out the same tree shape, per §4.
- Update every fixture/conformance test that currently asserts on the typed `ClearBox` field
  (`fixtures/reference_config*.h5`, `test_models.cpp`, `test_v1_1_adapter.cpp`, Rust's
  `roundtrip_clearbox_scalar_fields`, etc.) to assert on the extension tree instead.
- `docs/EXTENSION_CONTRACT.md`: move `ClearBox` out of "still modelled in core", update the
  "Known extensions" table's repo link once Phase 2 exists, close out the corresponding "Not yet
  done" bullets.

**Phase 2 — new repo: `mcf-ext-clearbox`.**
Structure mirrors `mcf-ext-opcua` (`DESIGN.md`, `nodejs/`, `rust/`, `fixtures/`, release tooling).
Node + Rust first (see §8 for why that covers both named consumers; Python/Go/C++ ports deferred,
same as `mcf-ext-opcua` today).
- `EXTENSION_NAME = 'ClearBox'`, `REQUIRES_CONTRACT` pinned to the contract version Phase 0 ships.
- Domain model: today's Rust `ClearBox`/`PowerCharacterization`/`SynchronousSensor`/
  `EquationConstant`/`CalibrationPoint` structs are the starting point for the spec-form types;
  apply the same domain-model cleanups `mcf-ext-opcua` applied to OPC UA's spec (§"Domain model
  (deliberately diverges from the on-disk spec)" in its `DESIGN.md`) wherever ClearBox's on-disk
  encoding has the same kind of quirk (e.g. `0`/`1` int flags → `bool`, as `show_console` and
  `enabled`/`calibration_verified` already are in the current model).
- Legacy adapter: trees with no schema-version attribute (**every** tree core synthesizes from a
  v1.0 file, per §4, plus any already-migrated v1.1 file written before a schema-version attribute
  existed) map through unmodified field-for-field — there is no "flat triggers → rules" style
  reshaping needed here the way there was for OPC UA, since ClearBox's legacy and current layouts
  are structurally identical once both arrive as extension trees.
- Decide the starting schema version number now (open question, §9).

**Phase 3 — integration.**
- `clearbox-tauri`: depend on `mcf-ext-clearbox`'s Rust crate alongside the already-`git`-dependent
  `machine-config` crate (see `docs/clearbox-tauri-integration.md`'s existing dependency table).
  Update its facade (`src-tauri/src/machine_config/`) to read
  `config.extensions["ClearBox"]` + `mcf_ext_clearbox::unpack(...)` instead of
  `optical_trains[i].optional_components.clearbox`.
- `rdt-core`: add `mcf-ext-clearbox` (GitHub release tarball URL, same as `mcf-ext-opcua`) and a
  `machine-clearbox-extension.codec.ts` modelled on `machine-opcua-extension.codec.ts`
  (DB/`hdf5Structure` <-> `ClearBoxConfig`, `pack`/`unpack`, dynamic `import()` for ESM). Replace
  `buildClearBox()` in the mapper helper, set `config.extensions['ClearBox']` in the generator next to
  `TM_OPCUA`, and switch the import path (`config-import-matching.service.ts`,
  `config-import.service.ts`, `CLEARBOX_FIELD_MAP`) from `optional_components.clearbox` to the unpacked
  model. Keep `configuration_hash` computed from `hdf5Structure` exactly as today so existing hashes
  don't change.
- Update `docs/clearbox-tauri-integration.md` and `docs/architecture-overview.md` to describe the
  new boundary (mirrors how those docs already describe the OPC UA boundary).

## 7. Scope boundary: LightSource.power_characterization stays in core

`PowerCharacterization` is shared by two unrelated call sites today (`docs/migrations/
v1_0_to_v1_1.md` Change 3/4): `ClearBox.power_characterization` (Volts→Watts) and
`LightSource.power_characterization` (Watts→Volts, the inverse conversion). `LightSource` is not
part of `Optional_Components`/ClearBox and is not in scope for this extraction —
`docs/EXTENSION_CONTRACT.md`'s "Not yet done" bullet only names ClearBox's own datasets. Phase 1
must keep a `PowerCharacterization`/`EquationConstant`/`CalibrationPoint` typed path alive in core
for `LightSource`'s use while removing it everywhere else — **not** a wholesale deletion of those
three types. This needs explicit care during Phase 1 since Rust today genuinely shares one struct
across both.

## 8. Consumers

| Repo | Language | Current ClearBox usage | Change needed |
|---|---|---|---|
| `clearbox-tauri` | Rust | Typed `optional_components.clearbox` in `capture_services/mod.rs` (`laser_correction_data_from_config`, clearbox settings, `power_characterization` via the public `machine_config::power_characterization::backward_flat_fields_coefficients`) and `machine_config/types.rs`; OPC UA through `config.extensions` and a serde-JSON bridge to `mcf_ext_opcua::ExtensionTree` (`machine_config/opcua.rs`) | Pinned to MCF git rev `b485b817`, so nothing changes until it bumps. Contract 0.1.1 is safe to take: no struct literals of `ExtensionTree` (only `default()` and serde), and the JSON omits `tables` when empty, so the OPC UA bridge is byte-identical. Phase 1/3 then switches ClearBox reads to `mcf-ext-clearbox` `unpack` over `extensions["ClearBox"]`, and core must keep `power_characterization::*` public |
| `rdt-core` | Node/TS (NestJS, `api/`) | Reads and writes typed `ClearBox` incl. correction grids and `power_characterization`, for both 1.0 and 1.1 targets (§2) | Pinned to release tarballs, so unaffected until it bumps (`tables?` is optional on the TypeScript `ExtensionTree`, and the OPC UA codec casts through `unknown`). Phase 3: add `mcf-ext-clearbox`; new ClearBox codec mirroring the OPC UA codec; replace mapper `buildClearBox()` and import-side reads |
| `clifile` | Python | Declares `machine-config-library` (wheel `v0.2.0-rc.8`) in `pyproject.toml`, but no source file imports it | None; effectively an unused dependency |
| `docs/validation/*/app` | all five | In-repo apps assert on typed ClearBox (S-08 etc.) and the Rust one is a path dependency | Update with Phase 1 (they have no `ExtensionTree` struct literals, so contract 0.1.1 itself breaks nothing) |

This is why Phase 2 only needs Node + Rust ports, same reasoning `mcf-ext-opcua` already used
("It does not yet have Python, Go or C++ ports, so hosts in those languages can round-trip the
`TM_OPCUA` tree opaquely but cannot decode it into a typed model until those ports exist" —
identical situation applies to ClearBox and these consumers).

## 9. Decisions and open questions

### Decided

- **Shim lives in core (§4).** Core's v1.0 adapter shims ClearBox into `extensions["ClearBox"]` the way it
  already does for `TM_OPCUA`. The tree is always the v1.1 layout, so the extension never sees
  `File_Version`. Whether core reuses its existing v1.0 parsing internally to build that tree is an
  implementation detail, not a design question; reuse it.
- **Eager grids accepted.** Extension datasets are read eagerly; no lazy/skip option is required first.
- **v1.0 output stays.** rdt-core keeps emitting v1.0 files because older consumers depend on their exact
  shape. v1.0 writing must therefore stay byte-for-byte equivalent in layout to today's (see below).
- **Contract stays `0.1.x`** for compound support, so `mcf-ext-opcua`'s `^0.1` remains valid.
- **Unsupported compound members fail loudly** (non-1-D, variable-length strings, nested/array/enum/bool,
  big-endian), never skipped, because core rewrites files from its model and a skipped node would be
  deleted on save. They can be added later as additive kinds when a real extension needs one. Array
  members are covered by a Rust test; Node cannot create one with h5wasm.
- **No Python, Go or C++ extension codec** is planned. Table support, however, was added to the core
  contract of all five languages (Phase 0), so ClearBox can become an opaque tree in every core and
  the cross-language model stays identical.

### New issue from the v1.0 decision: v1.0-only fields

rdt-core's `buildClearBox()` currently writes fields that v1.1 deliberately drops (`selected_camera`,
`custom_video_format`, `video_output`, `show_console`, `correction_grid_domain_shape`,
`inverse_grid_domain_shape`) and per-train `output_path`/`software_trigger_delay`. If the tree is
v1.1-shaped, a v1.0 file generated after the migration would silently lose those fields. Need to decide
how they travel: (a) carry them as per-train attributes in the tree that only the v1.0 writer emits and
the v1.1 writer rejects or ignores, set by the host only when targeting 1.0; or (b) core's v1.0 writer
fills them with the same defaults/values rdt-core uses today. Verify against what the older consumers
actually read before choosing. Add a golden test: generate a v1.0 file before and after the migration
from the same inputs and diff the ClearBox groups.

### Still open
1. **Starting schema version for `mcf-ext-clearbox`.** `mcf-ext-opcua` started at `1.2` because
   that's what its prototype already wrote on disk, so no existing file needed migrating. Does an
   equivalent "already written somewhere" version exist for ClearBox, or does this start fresh at
   `1.0`/`1.1`?
2. **Disagreeing trains (decided: first train wins).** Trains should never disagree on
   `Output_Path`/`Software_Trigger_Delay`; on the v1.0 read shim core uses the first train with a
   ClearBox. Remaining check: confirm the existing v1.1 *write* hard error (manifest Change 1) stays
   as is, since the v1.0 write fan-out replicates the shared values to every train and cannot create a
   disagreement. Optional: scan real v1.0 files once to see whether any disagree at all.
3. **Phase 1 scope for Python, Go and C++ core — resolved.** Table support was added to all three
   (option b), so Phase 1 removes typed ClearBox from every language and the cross-language model and
   `tools/cross_check.py` stay uniform. Phase 1 therefore includes the v1.0 shim and writer fan-out in
   Python, Go and C++ as well, which is the larger share of the work.

---

Next concrete step once these are answered: Phase 1 (core makes ClearBox opaque, all five languages),
starting with the v1.1 mapping and the v1.0-only-fields decision above.
