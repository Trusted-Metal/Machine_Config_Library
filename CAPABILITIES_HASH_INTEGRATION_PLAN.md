# Capabilities-Facade Hash Integration — Implementation Plan

**Status: fully complete, 2026-09-15 — all 5 languages plus documentation.** Along the way: a
cross-language generator discovery that changed Step 1's actual scope (see "Generator discovery"
below), and a finding (Step 3, reconfirmed independently in Steps 4 and 5) that the
registry-driven parity test drafted for both Python and Node.js only actually applies to Python.
`cpp/build`, which was red at the end of the Rust-step generator regeneration (Step 5 not yet
started), is green again: both `machine_config_cli` and `machine_config_tests` build clean in both
Debug and Release, all tests pass. `docs/clearbox-tauri-integration.md` §6, deferred from Step 1,
was rewritten as the closing action (tracked as `CONFIGURATION_HASH_PLAN.md` Step 8, since that's
the plan document that originally owned it) — see the closing note below for what it now says.

**Purpose of this document:** committed to the repo (unlike an ephemeral plan-mode file) so it's a
durable, reviewable record the same way `CONFIGURATION_HASH_PLAN.md` and
`RECOATER_BLADE_TYPE_PLAN.md` are — picked up step by step, one language per implementation
session, each step checked off with what was actually done/verified before moving to the next.

---

## Context

`CONFIGURATION_HASH_PLAN.md` (all 5 languages done, Steps 1-5) added a real, computed SHA-256
`configuration_hash` and a reader-computed `is_valid` flag — but only at the `MachineConfigReader`/
`MachineConfigWriter` layer in each language. That work was verified thoroughly (per-language test
suites, cross-language CLI spot checks against `fixtures/reference_config.h5`) and that verification
was accurate as far as it went.

What it missed: **every language also has a second, parallel public API** — the "capabilities"
stable-facade (`open_machine_config`/`create_machine_config`/`MachineConfigFileV1_0`/
`MachineConfigFileV1_1` in Rust and Python; the equivalent `File`/`MachineConfigFileV1_0` shapes in
Go, Node.js, C++) — a session-style get/set-by-field API, distinct from `MachineConfigReader`/
`MachineConfigWriter`, with its own `open()`/`Open()` and `save()`/`Save()` implementations. Two
real consuming applications were investigated as part of this discovery, and **both use the
capabilities facade exclusively — neither uses `MachineConfigReader`/`MachineConfigWriter` at
all**:

- **clearbox-tauri** (`C:\Users\ChrisParham\Desktop\Repo\clearbox-tauri`, Rust/Tauri): depends on
  the `machine-config` crate via git rev (`src-tauri/Cargo.toml:70`), consumed only through
  `machine_config::open_machine_config()` (`src-tauri/src/machine_config/reader.rs:30-140`). No
  `machine_config::reader::MachineConfigReader` import anywhere in the app.
- **mcf_editor.py** (`C:\Users\ChrisParham\Desktop\Practice\machineconfiglibrarytesting\python\mcf_editor.py`,
  PySide6 desktop GUI): imports only `open_machine_config`, `create_machine_config`,
  `MachineConfigWriter` is never imported; opens via `open_machine_config(path)` (line ~707).

(A third candidate, RDT-CORE, was also investigated and ruled out — it's TypeScript/Node-only, has
no Rust component, and never reads a machine-config HDF5 file back after writing one, so it isn't
affected by anything in this document.)

## Critical finding: the capabilities facade bypasses both `MachineConfigReader` and, in most
## languages, `MachineConfigWriter` too

Confirmed by direct inspection of `capabilities/v1_0/file.*` and `capabilities/v1_1/file.*` in all
five languages (v1.0 and v1.1 checked separately — they are not always consistent with each other
within the same language, see the table below).

**Read side (`open()`/`Open()`), all 5 languages, both versions: always bypassed.** Every facade's
`open()` calls the raw per-version adapter's `parse()` directly (e.g. Rust:
`Hdf5AdapterV1_0::open(&path)?.parse_with_binary()?`; Python: `Hdf5AdapterV1_0(str(path)).parse()`;
Node: `new Hdf5AdapterV1_0(path).parse(...)`; Go: `v1_0hdf5.Parse(path, true)`; C++:
`v1_0::Hdf5AdapterV1_0 reader(path)` + its own parse) — **never** `MachineConfigReader::parse()`,
which is the only place any language's `is_valid` actually gets computed. Grepping `is_valid`/
`IsValid` across every `capabilities/v1_0/file.*` and `capabilities/v1_1/file.*` file in all five
languages returns zero matches. **No capabilities-facade consumer, in any language, can see
`is_valid` today — it is never computed on this path, not merely hard to reach.**

**Write side (`save()`/`Save()`): inconsistent, both across languages and across versions within
the same language.**

| Language | v1.0 `save()`                          | v1.1 `save()`                          |
|----------|------------------------------------------|------------------------------------------|
| Rust     | ✅ `MachineConfigWriter::new(&self.config).write(&out)` (`capabilities/v1_0/file.rs:368-369`) | ❌ `Hdf5WriterV1_1::new(&self.config).write(&out)` directly (`capabilities/v1_1/file.rs:331-332`) — bypasses `MachineConfigWriter` |
| Python   | ❌ `Hdf5WriterV1_0(config).write(out)` directly (`capabilities/v1_0/file.py:200`) | ❌ `Hdf5WriterV1_1(config).write(out)` directly (`capabilities/v1_1/file.py:200`) |
| Node.js  | ❌ `new Hdf5WriterV1_0(...).write(...)` directly (`capabilities/v1_0/file.ts:344`) | ❌ `new Hdf5WriterV1_1(...).write(...)` directly (`capabilities/v1_1/file.ts:351`) |
| Go       | ❌ `v1_0hdf5.Write(f.config, out)` directly (`capabilities/v1_0/file.go:233`) | ❌ `v1_1hdf5.Write(f.config, out)` directly (`capabilities/v1_1/file.go:241`) |
| C++      | ✅ `MachineConfigWriter{config_}.write(out.string())` (`capabilities/v1_0/file.hpp:268`) | ✅ `MachineConfigWriter{config_}.write(out.string())` (`capabilities/v1_1/file.hpp:273`) |

A "❌" here means: saving a file through that language/version's capabilities facade writes
whatever `configuration_hash` is already sitting in the in-memory session (untouched from the
original file, or whatever a caller explicitly set) — **not** a hash freshly recomputed from the
content actually being written. This is a correctness bug, not a visibility gap: it silently
violates the Configuration Hash feature's central promise ("the stored hash always reflects the
file's actual content") for every consumer on a ❌ path. Concretely:

- **mcf_editor.py** (Python, v1.0 and v1.1 both ❌): edit a field, hit Save — the on-disk
  `Configuration_Hash` attribute does not change to reflect the edit.
- **clearbox-tauri** (Rust, v1.0 only, since it only creates/writes v1.0 files today per
  `docs/clearbox-tauri-integration.md`): currently fine in practice, but silently would not be
  fine if/when it starts writing v1.1 files.

## Generator discovery (2026-09-15): `is_valid()` unavoidably touches 4 languages at once, not 1

Discovered while starting Step 1 (Rust): `rust/src/capabilities/generated.rs` — where the
`MachineConfigFile` trait lives — carries a "DO NOT EDIT, generated by
`tools/generate_capabilities.py`" banner. This matters immediately for `is_valid()` specifically:
`open_machine_config()` returns `Box<dyn MachineConfigFile>`, and clearbox-tauri (this plan's
primary motivating real consumer) only ever holds that boxed trait object, never the concrete
`MachineConfigFileV1_0`/`V1_1` types. A Rust trait object can only expose methods the trait itself
declares — so for clearbox-tauri to ever call `is_valid()`, it **must** be added to the
`MachineConfigFile` trait, not just to the concrete structs. There is no way around this by adding
an inherent method or a separate extension trait instead.

Investigating `tools/generate_capabilities.py` turned up two things that changed this step's scope:

1. **One script, four outputs, one call.** `main()` writes `nodejs/src/capabilities/generated.ts`,
   `python/.../generated.py`, `rust/.../generated.rs`, and `cpp/.../generated.hpp` unconditionally,
   every run. Go is not part of this generator at all — its facade interface is hand-written
   separately. So "add `is_valid` to the Rust trait" is inescapably "add it to 4 languages'
   generated interfaces simultaneously," not 1.
2. **`schema/capabilities/api.yaml`/`models.yaml` are not actually consulted.** Every `render_*`
   function (`render_ts`/`render_py`/`render_rs`/`render_hpp`) opens with `_ = api, models` —
   an explicit "parameter intentionally unused" — and each language's interface is instead a
   hardcoded string-literal template baked directly into the script. The YAML files exist but are
   dead weight for code generation today (not something to fix as part of this plan — out of
   scope, noted here only because it changes *how* the edit is made: editing
   `generate_capabilities.py`'s template strings directly, not the YAML).

**Risk differs sharply by language**, because each generated interface uses a different mechanism:

| Language | Interface kind | Effect of an unimplemented new method |
|----------|----------------|----------------------------------------|
| Rust | `trait MachineConfigFile` | Compile error (`E0046`) everywhere it's implemented, including a test-only fake in `capabilities/mod.rs` |
| TypeScript | `interface MachineConfigFile` | Compile error (`tsc`) on any class declaring `implements MachineConfigFile` |
| C++ | `class IMachineConfigFile` with pure virtual methods | The derived class becomes abstract — cannot be instantiated, a hard build break everywhere `MachineConfigFileV1_0`/`V1_1` is constructed |
| Python | `class MachineConfigFile(Protocol)` | **Nothing** — `Protocol` is structural typing, checked only by an optional static type-checker, never enforced at runtime. Python's tests kept passing with zero changes. |

**Decision, confirmed with the user before proceeding**: add a real, working (not a stub/
`NotImplementedError`) `is_valid()`/`isValid()` method to all four generator-covered languages'
concrete classes now, as a mechanical side effect of unblocking Rust — every language keeps
building and every existing test suite stays green. This does **not** mean Python's/Node.js's/
C++'s own steps below are done: only the method itself was added early; each language's `save()`-
side fix, live-ness tests, and doc updates remain scoped to that language's own dedicated session,
exactly as originally planned. Go is unaffected by this generator entirely, so its own step is
untouched and starts fresh whenever it's picked up.

While adding it, one further simplification: the method returns a plain `bool` (not
`Option<bool>`/`Optional[bool]`/`boolean | undefined` as earlier drafts of this plan sketched) in
every language. Unlike the `MachineConfigMeta.is_valid` *field* from `CONFIGURATION_HASH_PLAN.md`
(which starts unset and only becomes `Some`/`Some` once a `MachineConfigReader` actually parses
something), this is a *method*, always called against an already-loaded, already-populated session
— there is no "not yet known" state to represent, so the nullable wrapper was unnecessary
complexity.

## `is_valid()` is a live method, decided (2026-09-15) — not a cached field anywhere

Two designs were considered for the read side: (a) compute `is_valid` once at `open()` time and
cache it on the struct/dict, mirroring how `MachineConfigReader::parse()` already works, or
(b) compute it fresh, on demand, every time a caller asks. **Decided: (b), live/lazy, uniformly
across all 5 languages.**

Why this rules out the "no new accessor needed" reasoning that (a) would have allowed for Rust/Go/
C++: the capabilities facade — unlike `MachineConfigReader::parse()`, a one-shot, immutable
read — is a **mutable session** (`set_meta`/`set_scanner`/etc. all mutate `self.config`/`self._data`
in place). A struct field computed once at `open()` time would silently go stale the instant any
`set_*` call ran, and calling it via `get_meta().is_valid` would then be actively misleading (`true`
long after the in-memory session no longer matches its own stored hash). So every language needs a
real **method**, not a field read — `is_valid()` (Rust/Python convention) / `isValid()`
(Node.js/C++ convention) / `IsValid()` (Go convention, exported), matching each language's existing
casing — that recomputes
`compute_configuration_hash`/`computeConfigurationHash`/`ComputeConfigurationHash` against whatever
the session currently holds, and compares it to that same current session's `configuration_hash`,
every single call. This is true for Rust/Go/C++ too, even though `get_meta()` already structurally
carries an `is_valid` field on `MachineConfigMeta` (added in `CONFIGURATION_HASH_PLAN.md` Steps
2/4/5) — that field stays permanently `None`/`nullopt`/unset on every config the capabilities
facade ever touches; it is simply the wrong mechanism for a mutable session, not merely
"available but redundant."

A useful simplification falls out of this: since nothing needs to be precomputed, **`open()`/
`Open()` needs no hash-related change at all** for the read side — no wrapping, no field-setting.
The per-language read-side work is entirely: add one `is_valid()`/`isValid()` method to
`MachineConfigFile`/the equivalent interface, implemented identically in shape across all 5
languages regardless of which of the two facade architectures (typed-struct vs. dict/handle-based)
that language uses:

```
is_valid() -> bool:
    current = <the session's current config, in whatever shape compute_configuration_hash expects>
    return compute_configuration_hash(current) == current.meta.configuration_hash
```

- **Typed-struct facades (Rust, Go, C++)**: `current` is just `self.config`/`f.config`/`config_` —
  already the right shape, no conversion needed.
- **Dict/handle-based facades (Python, Node.js)**: `current` must be reconstructed from
  `self._data`/`this.data` first — **resolved during Step 1** (see "Generator discovery" above):
  Python needs a real reconstruction (`config_from_dict(self._data)`, a dict → dataclass
  conversion), while Node.js needs none at all — `asJson(config)` (`capabilities/v1_0/file.ts`) is
  a pure type-cast (`config as unknown as Json`) with zero runtime transformation, so `this.data`
  already *is* the original `MachineConfig` object structurally; `computeConfigurationHash(this.data
  as unknown as MachineConfig)` works directly. Either way, `is_valid`/`isValid` stays **out of**
  the schema-shaped dict `meta().getModel()`/`get_model()` returns (same passive-exclusion category
  as `facility_id`/`config_author`) — it is only ever reachable via the new method, never as a key
  in that dict. This resolves the original "fold into the dict vs. distinct method" question in
  favor of the distinct method, for both languages, definitively.

**A consequence for the write-side fix that must not be missed**: `save()`/`Save()` computes a
fresh hash and writes it to the *output file* — but it must **also** write that same fresh hash
back into the *session's own in-memory config* (`self._data["meta"]["configuration_hash"]` /
`self.config.Meta.ConfigurationHash` / etc.), not just the bytes on disk. If it doesn't, calling
`is_valid()` immediately after `save()` — without reopening — would read back `false` (the session
still holds its old, pre-save hash) even though the file just written is genuinely valid. This is
the direct, mechanical payoff of choosing "live": the whole point of a live check is that
`save()` → `is_valid()` should read `true` without a round-trip through disk, and that only works
if `save()` updates the session state it's supposed to be checking.

**Known caveat, accepted deliberately, not an oversight**: "live" means `is_valid()` reads `false`
for the *entire* span between the first `set_*` call after `open()`/`save()` and the next
successful `save()` — not because anything is wrong, but simply because nothing recomputes the
hash until you save. This is technically correct (the in-memory content genuinely doesn't match
the recorded hash) but it conflates two questions a consumer likely cares about separately:
"was this file tampered with before I opened it" (integrity) vs. "do I have unsaved edits right
now" (dirty-state) — during any edit session, `is_valid()` answers both identically (`false`),
which risks reading as a false alarm if a UI surfaces it as a warning rather than an expected
"you have unsaved changes" state. Decided to accept this rather than freeze `is_valid()` at
`open()`/`save()` time instead (the alternative considered), because freezing reintroduces the
exact staleness problem "live" exists to solve, and because `is_valid()` is still *always correct*
by construction, which a frozen value cannot claim. Mitigation, to make sure this doesn't surprise
a real integration the way the original capabilities-facade gap itself did: (a) document this
explicitly wherever `is_valid()` is introduced per language (docstrings/doc comments, and
`docs/clearbox-tauri-integration.md` §6 for Rust) — "false right after an edit is expected, not
alarming"; (b) a consumer that wants to distinguish "tampered" from "mid-edit" should pair
`is_valid()` with its own dirty-tracking if it has one (this plan doesn't add a new `is_dirty()` —
out of scope — but doesn't preclude an app layering one on top).

## Decisions (made 2026-09-15, except where still flagged open)

- **`is_valid()`/`isValid()` is a live method, computed fresh on every call, uniformly across all 5
  languages** — see the section above. No caching, no struct field populated by `open()`, no
  per-language-architecture split in the final design (the split only affects *how* each language's
  method reconstructs "the session's current config" internally, not *whether* it needs the method
  or *when* it computes).
- **`is_valid` (the `MachineConfigMeta` struct/dataclass field from `CONFIGURATION_HASH_PLAN.md`)
  stays permanently unset on every config the capabilities facade touches.** It remains exactly
  what it always was — the mechanism `MachineConfigReader::parse()` uses for its own one-shot,
  immutable read, where "compute once at parse time" is correct because there's no session to go
  stale. The capabilities facade doesn't reuse it; it gets its own method instead. No conflict
  between the two designs, because they solve different problems (one-shot read vs. mutable
  session) — neither needs to change to accommodate the other.
- **Write-side fix, everywhere it's ❌ (Python v1.0/v1.1, Node.js v1.0/v1.1, Go v1.0/v1.1 — Rust
  v1.1 done as of Step 1, see below)**: change `save()`/`Save()` to compute the hash fresh and
  stamp it onto **both** the config
  actually handed to the per-version writer **and** the session's own retained config (`self.config`/
  `self._data`/etc.) — the latter is new, required by "live" (see above): without it, `is_valid()`
  called right after `save()` would incorrectly read `false` until the next `open()`. Prefer routing
  through `MachineConfigWriter`/`MachineConfigWriter::new(...)` the way the already-correct paths do
  (Rust v1.0, C++ both versions) wherever the facade already holds a real typed `MachineConfig` it
  can hand off cleanly (Rust v1.1 clearly qualifies); where that's awkward given the facade's own
  internal shape (Python/Node.js/Go), call `compute_configuration_hash`/`computeConfigurationHash`/
  `ComputeConfigurationHash` directly and set `meta.configuration_hash` on the session before
  invoking the raw per-version writer — investigate each language's exact internal shape at
  implementation time to decide which approach fits more cleanly.
- **Should `is_valid()`/`open()`/`save()` ever fail loudly on a hash mismatch?** No — matches
  `CONFIGURATION_HASH_PLAN.md`'s explicit, hard requirement: a mismatch is informational
  (`is_valid() == false`), never a thrown/returned error. Carry that invariant through unchanged.
- **New: a standing, registry-driven dict-builder parity test — Python only, confirmed by
  investigation, not assumed** — added 2026-09-15, prompted directly by the
  `CONFIGURATION_HASH_PLAN.md` Step 6 `firmware_version` bug (a real, shipped instance of exactly
  the failure mode this guards against). Originally drafted assuming Node.js needed this too
  (both are "dict-based facades" in the read-side sense) — **corrected during Step 3**: Node.js's
  `hash.ts` hashes whatever `MachineConfig` object it's given directly, with no per-version
  dict-builder in its own path at all (confirmed via `grep`, not assumed), so it turns out to
  belong with Rust/Go/C++ for *this specific* concern, not with Python. Rust/Go/C++/Node.js all
  hash the live, shared `MachineConfig` object directly (via `serde_json`/`encoding/json`+
  reflection/`nlohmann::json` ADL/a generic object walk respectively), so a new field on the model
  is automatically included in every version's hash input with no per-version code to keep in
  sync — this bug class is structurally impossible for all four (independently reconfirmed for Go
  in Step 4 and for C++ in Step 5). Python alone hand-maintains a `_config_to_dict()` **per
  version**, which is exactly what let `firmware_version` silently diverge between v1.0's and
  v1.1's dict-builders until a real cross-language hash mismatch surfaced it by accident. See
  "Tests" below for the concrete test shape — it must enumerate the actual version registry
  (`supported_file_versions()`, backed by Python's real `_OPEN` dict), not name `v1_0`/`v1_1`
  literally, so it automatically extends to cover a future v1.2 the day it's registered, with no
  test-file changes required.

## Per-language steps

Each step: (a) confirm the v1.1 facade's exact shape isn't different from what's summarized above
(don't assume — this document already found one real inconsistency, Rust v1.0 vs. v1.1, by
checking rather than assuming symmetry); (b) add the live `is_valid()`/`isValid()` method (no
`open()` change needed — see the live-method section above); (c) fix `save()`/`Save()` to compute a
fresh hash — on **both** the written file and the retained session config — where currently
bypassed; (d) tests, including the new registry-driven parity test for Python/Node.js; (e) update
`docs/clearbox-tauri-integration.md` §6 at the same time if that language's step is Rust (it's
already flagged as stale in `CONFIGURATION_HASH_PLAN.md` Step 8, but fixing the underlying gap here
is a natural time to also fix the doc that describes it).

### Step 1 — Rust

**Status: done and verified, 2026-09-15.** Scope grew beyond Rust alone partway through — see
"Generator discovery" above — but the actual Rust-specific work (trait, both facades' `save()`,
tests, docs) is fully complete.

- [x] Edited `tools/generate_capabilities.py`'s `render_rs` (and, necessarily, `render_ts`/
      `render_py`/`render_hpp` — one script, four outputs) to add `is_valid`/`isValid` to each
      generated interface, then ran it to regenerate all four `generated.*` files.
- [x] Added `pub fn is_valid(&self) -> bool` as an inherent method on `MachineConfigFileV1_0` and
      `MachineConfigFileV1_1` (`capabilities/v1_0/file.rs`, `capabilities/v1_1/file.rs`):
      `compute_configuration_hash(&self.config) == self.config.meta.configuration_hash`, fresh
      every call. Delegated to it from each type's `impl MachineConfigFile for ...` block (Rust
      always prefers the inherent method over the trait method of the same name, so the
      delegation just forwards). Also had to add a matching (unreachable-bodied) `is_valid()` to
      the test-only `FakeMachineConfigFile` in `capabilities/mod.rs` — the only other trait
      implementer in the codebase, found via `grep -rn "impl MachineConfigFile for"`, not assumed.
- [x] `capabilities/v1_1/file.rs` `save()`: replaced the direct `Hdf5WriterV1_1::new(&self.config)
      .write(&out)` with `MachineConfigWriter::new(&self.config).write(&out)` (imported from
      `crate::writer`, the version-agnostic dispatcher already used by `v1_0/file.rs`'s own
      `save()` — confirmed this does not violate this file's "deliberately independent of v1_0"
      module-doc comment, since `crate::writer` is a shared, version-dispatching module, not
      `v1_0`-specific, and `v1_0/file.rs` already depends on it the same way). Also stamps
      `self.config.meta.configuration_hash = compute_configuration_hash(&self.config)` *before*
      calling the writer, so the session's own retained config reflects the fresh hash
      immediately — required by the live-method design (see above).
- [x] `capabilities/v1_0/file.rs` `save()`: was already routing through `MachineConfigWriter`, but
      confirmed (by reading `MachineConfigWriter::write()`'s implementation) it clones `self.config`
      internally and never mutates the caller's copy — so `self.config.meta.configuration_hash`
      was **not** being updated on the session, exactly the gap this plan's Decisions section
      flagged. Fixed with the same one-line stamp-before-write pattern as v1.1's fix above.
- [x] Tests (`tests/capabilities_test.rs`, 6 new): `is_valid_false_for_externally_authored_reference_fixture`,
      `is_valid_true_for_self_written_file`, `is_valid_flips_live_across_an_unsaved_edit_then_a_save`
      (the live-ness + save→live round-trip test this whole design exists to satisfy — edits via
      `set_scanner`, checks `false` with no save, then `true` immediately after `save()` with no
      reopen, then confirms a completely fresh `open_machine_config()` session agrees),
      `v1_0_save_still_routes_through_real_hash_computation` (regression guard for the
      already-correct path), and `v1_1_save_computes_real_hash_and_updates_session` (targets the
      exact v1.1 bug directly, using `fixtures/reference_config_v1_1.h5` through the
      version-agnostic `open_machine_config()` dispatcher, confirming it correctly routes a v1.1
      file to the v1.1 facade).
- [x] Full suite: `cargo build` clean, `cargo test --no-fail-fast` — all crates 0 failed (106 lib
      unit tests unaffected; `capabilities_test.rs` up to 23 tests, from 17; `version_adapter_
      isolation_test.rs`'s 6 tests confirm the `MachineConfigWriter` import doesn't violate v1.0/
      v1.1 module isolation).
- [x] Update `docs/clearbox-tauri-integration.md` §6 — **done, 2026-09-15**, once all 5 languages
      were complete (tracked and detailed under `CONFIGURATION_HASH_PLAN.md` Step 8, the plan
      document that originally owned this doc). Deferred here mid-Step-1 specifically to update
      this plan document first, per direct request; picked back up after Step 5.

### Step 2 — Python

**Status: done and verified, 2026-09-15.** `is_valid()` itself had already landed early, as a
mechanical side effect of Step 1 (see "Generator discovery" above); this step completed the
remaining `save()`-side fix and the standing parity test.

- [x] Added `is_valid(self) -> bool` to `MachineConfigFileV1_0`/`V1_1`
      (`capabilities/v1_0/file.py`, `capabilities/v1_1/file.py`) — landed during Step 1.
- [x] `capabilities/v1_0/file.py` `save()`: computes `compute_configuration_hash(config)` on the
      reconstructed `config` and stamps it onto **both** `config.meta.configuration_hash` (before
      `Hdf5WriterV1_0(config).write(out)`) **and** `self._data["meta"]["configuration_hash"]` (the
      session's own retained dict), so `is_valid()` reads `True` immediately after `save()` without
      requiring a fresh `open()`.
- [x] `capabilities/v1_1/file.py` `save()`: same fix, mirrored exactly.
- [x] New standing test `test_registry_driven_dict_builder_parity_across_all_versions`
      (`tests/test_capabilities.py`): iterates `supported_file_versions()` — not `["1.0", "1.1"]`
      literally — and asserts every entry has a matching dict-builder class in a small test-local
      `_DICT_BUILDER_CLASSES` map; if a future version is registered in `capabilities/__init__.py`
      without a matching update here, the test fails loudly (naming the missing version) rather
      than silently skipping it. Builds one `MachineConfig` from `reference_config_v1_1.h5` (the
      fixture confirmed to exercise `firmware_version`, the exact field that diverged before), runs
      every registered version's `_config_to_dict()` over it, and asserts they all agree.
- [x] 5 more tests in `tests/test_capabilities.py`: `is_valid_false_for_externally_authored_
      reference_fixture`, `is_valid_true_for_self_written_file`,
      `is_valid_flips_live_across_an_unsaved_edit_then_a_save` (the live-ness + save→live
      round-trip test this whole design exists to satisfy, mirroring Rust's equivalent exactly),
      `v1_0_save_still_routes_through_real_hash_computation`, and
      `v1_1_save_computes_real_hash_and_updates_session` (using `reference_config_v1_1.h5` through
      `open_machine_config()`'s version-agnostic dispatch).
- [x] Full suite: `pytest` — **418 passed** (up from 412; +6 new tests: the 5 above plus the parity
      test), 0 failed.

### Step 3 — Node.js

**Status: done and verified, 2026-09-15.** `isValid()` itself had already landed early, as a
mechanical side effect of Step 1 (see "Generator discovery" above) — Node.js's generated
`MachineConfigFile` is a real TypeScript `interface`, so this one *did* need a real implementation
immediately to keep `tsc`/`npm run build` green (confirmed: it failed to compile before this was
added). This step completed the remaining `save()`-side fix, and investigated (rather than assumed)
whether the registry-driven parity test applies here.

- [x] Resolved the `asJson(config)` question definitively: it's `config as unknown as Json` — a
      pure type-cast, zero runtime transformation (`capabilities/v1_0/file.ts:35-37`). `this.data`
      is therefore structurally the *same object* as the original `MachineConfig`, not a
      schema-shaped copy the way Python's `self._data` is.
- [x] Added `isValid(): boolean` to `MachineConfigFileV1_0`/`V1_1` — landed during Step 1.
- [x] `capabilities/v1_0/file.ts` `save()`: computes `computeConfigurationHash(config)` and stamps
      it onto `config.meta.configuration_hash` *before* `new Hdf5WriterV1_0(config).write(out)` —
      and because `config` (`this.data as unknown as MachineConfig`) is the exact same object as
      `this.data` (not a copy, confirmed above), this single assignment updates the session's own
      retained state too, automatically. Simpler than Python's fix, which genuinely needed two
      separate writes (`config.meta.configuration_hash` and `self._data["meta"][...]`) because
      `config_from_dict(self._data)` really does build a distinct object there.
- [x] `capabilities/v1_1/file.ts` `save()`: same fix, mirrored exactly.
- [x] **Investigated whether the registry-driven parity test applies to Node.js — it doesn't, and
      this is now recorded rather than assumed.** `hash.ts` hashes whatever `MachineConfig` object
      it's handed directly via its own generic `sanitizeForHash`/`renderValue` walk (confirmed via
      `grep -n "Hdf5AdapterV1_0\|Hdf5AdapterV1_1\|configToDict" src/hash.ts` → zero matches except
      one comment). There is no per-version, hand-maintained dict-builder in Node.js's hash path at
      all for two versions' output to ever disagree about — unlike Python, where `hash.py`
      literally hardcodes `Hdf5AdapterV1_0` as *the* dict-builder regardless of a config's actual
      version. The `firmware_version`-class bug is structurally impossible here, the same way it's
      impossible for Rust/C++ (which hash the real struct via `serde_json`/`nlohmann::json` ADL) —
      Node.js turns out to belong in that group, not Python's. No test written for this reason; a
      test asserting nothing meaningful would just be noise.
- [x] Tests (`tests/capabilities.test.ts`, new `describe('isValid()', ...)` block, 5 tests):
      `is false for the externally-authored reference fixture`, `is true for a self-written file`,
      `flips live across an unsaved edit, then back after save, without reopening` (the live-ness +
      save→live round-trip test this whole design exists to satisfy, mirroring Rust's/Python's
      equivalent), `v1.0 save() still routes through real hash computation`, and `v1.1 save()
      computes a real hash and updates session state` (using `reference_config_v1_1.h5` through
      `openMachineConfig()`'s version-agnostic dispatch).
- [x] Full suite: `tsc --noEmit` clean, `npm run build` clean, `npx vitest run` — **250 passed**
      (up from 245; +5 new tests), 0 failed, across all 10 test files.

### Step 4 — Go

**Status: done and verified, 2026-09-15.** Unlike Rust/Python/Node.js/C++, Go's facade interface
(`capabilities.File`, `go/capabilities/file.go`) is hand-written, not generated by
`tools/generate_capabilities.py` (Go was never part of that script) — so this step's interface
change was fully self-contained to Go from the start, no cross-language surprise like Step 1's.

- [x] Added `IsValid() bool` to the `capabilities.File` interface (`capabilities/file.go`) and
      implemented it identically on `v1_0.File`/`v1_1.File` (`capabilities/v1_0/file.go`,
      `capabilities/v1_1/file.go`): `machineconfig.ComputeConfigurationHash(f.config) ==
      f.config.Meta.ConfigurationHash` — fresh, every call. A plain `bool`, not `(bool, bool)` or
      `*bool` as originally sketched — matches the plain-`bool` decision made during Step 1's
      generator work, since there's no "not yet known" state for this method (see "Generator
      discovery" above). An internal hashing error (should not happen in practice) is reported as
      `false` rather than propagated, matching the non-fatal design throughout. No `Open()` change
      needed: `f.config` is already a `*MachineConfig`, so it's always current regardless of
      intervening `SetScanner`/`SetMeta`/etc. calls — but confirmed the existing `GetMeta().IsValid`
      struct field is *not* a usable substitute (it stays permanently `nil` on every config this
      facade touches, same as every other language's facade, since `Open()` calls
      `v1_0hdf5.Parse`/`v1_1hdf5.Parse` directly rather than `machineconfig.NewReader(...).Parse()`,
      which is the only place that field is ever set).
- [x] Found and fixed a test-only implementer this interface change broke: `fakeFile` in
      `capabilities/file_test.go` (a `DISPATCH_REGISTRY_PLAN.md`-style stub proving
      `ResolveOpen`/`ResolveCreate` are genuinely registry-driven) — found via `go vet ./...`
      failing with "does not implement capabilities.File (missing method IsValid)", not assumed;
      confirmed via `grep -rn "capabilities.File\b"` that this was the only other implementer.
- [x] `capabilities/v1_0/file.go` `Save()`: was calling `v1_0hdf5.Write(f.config, out)` directly,
      writing whatever `Meta.ConfigurationHash` was already in memory — computes
      `ComputeConfigurationHash(f.config)` and sets `f.config.Meta.ConfigurationHash` immediately
      before the write. Since `f.config` is a pointer, this single assignment also updates the
      session's own retained state — confirmed this holds (not assumed), via the new live-ness test
      below.
- [x] `capabilities/v1_1/file.go` `Save()`: same fix — Go's v1.1 facade had the identical bypass
      Rust's did (unlike Rust, where only v1.1 needed this; Go's v1.0 was *also* broken before this
      step, per the original ❌/❌ row in this plan's "Critical finding" table).
- [x] Confirmed (not assumed) the registry-driven parity test does not apply to Go, the same way
      Step 3 confirmed it for Node.js: `grep -n "v1_0\|v1_1\|Hdf5" hash.go` → zero matches. Go's
      `ComputeConfigurationHash` hashes the shared `*MachineConfig` struct directly via
      `json.Marshal`, with no per-version dict-builder in its own path — the `firmware_version`-class
      bug is structurally impossible here, same as Rust/C++/Node.js. No test written for this
      reason.
- [x] Found and fixed one existing stale test while running the suite: `TestSaveRoundTrip/reference_
      roundtrip` (`capabilities/file_test.go`) asserted `configuration_hash` survived the roundtrip
      unchanged — exactly the passthrough assumption this whole plan invalidates. Updated to assert
      64 hex chars, differs from the externally-authored original, and `IsValid()` is `true`.
- [x] 4 new tests (`capabilities/file_test.go`): `TestIsValidFalseForExternallyAuthoredReferenceFixture`,
      `TestIsValidTrueForSelfWrittenFile`, `TestIsValidFlipsLiveAcrossAnUnsavedEditThenASave` (the
      live-ness + save→live round-trip test this whole design exists to satisfy, mirroring Rust's/
      Python's/Node.js's equivalent), and `TestV1_1SaveComputesRealHashAndUpdatesSession` (targets
      the v1.1 bug directly, using `reference_config_v1_1.h5` through `OpenMachineConfig()`'s
      version-agnostic dispatch).
- [x] Full suite: `CGO_ENABLED=1 go test ./... -count=1` (MinGW64 on `PATH`) — all packages `ok`,
      0 failed, including the fixed `TestSaveRoundTrip` and all 4 new tests.

### Step 5 — C++

**Status: done and verified, 2026-09-15.** Unlike Python/Node.js, C++'s generated
`IMachineConfigFile` is a true abstract base with `isValid()` declared pure virtual (`= 0`), so
regenerating `generated.hpp` alone (done during Step 1) had left `MachineConfigFileV1_0`/`V1_1` —
and, discovered only once the test target was rebuilt, the test-only `FakeMachineConfigFile` in
`tests/test_capabilities.cpp` too — unable to be instantiated (`C2259`). This step's real content
was finishing that interface fix, confirming both versions' `save()` needed the same session-state
stamp every other language needed, and adding the new test coverage.

- [x] Regenerated `cpp/include/machine_config/capabilities/generated.hpp` with `virtual bool
      isValid() const = 0;` added to `IMachineConfigFile` — landed during Step 1.
- [x] Added `bool isValid() const override` to `MachineConfigFileV1_0`/`V1_1`
      (`capabilities/v1_0/file.hpp`, `capabilities/v1_1/file.hpp`): `computeConfigurationHash(config_)
      == config_.meta.configuration_hash`, fresh every call. Added `#include
      "machine_config/hash.hpp"` to both files (not previously included directly — only reachable
      transitively via `writer.hpp` before). No `open()` change needed (see the live-method design
      above). This unblocked `machine_config_cli`, confirmed via a clean rebuild.
- [x] Found and fixed the second abstract-class break this interface change caused, only visible
      once the *test* target was rebuilt (not the library/CLI): `FakeMachineConfigFile`
      (`tests/test_capabilities.cpp:83`), a `DISPATCH_REGISTRY_PLAN.md`-style stub proving
      `openMachineConfigWithRegistry`/`createMachineConfigWithRegistry` are genuinely
      registry-driven, also implements `IMachineConfigFile` and lacked `isValid()`. Added `bool
      isValid() const override { return false; }`, matching this fake's existing convention (every
      other stub method returns a fixed, unexercised value) and mirroring Go's identical fix to
      `fakeFile.IsValid()` in Step 4.
- [x] `capabilities/v1_0/file.hpp` `save()` and `v1_1/file.hpp` `save()`: confirmed (not assumed) —
      by reading `MachineConfigWriter::write()`'s implementation, same check made for Rust's v1.0 in
      Step 1 — that it clones `config_` internally and never mutates the caller's copy, so despite
      already routing through `MachineConfigWriter` (the ✅/✅ row in this plan's original "Critical
      finding" table), *neither* version was updating the session's own retained
      `config_.meta.configuration_hash` before this step. Fixed both with the same one-line
      stamp-before-write pattern used in every other language:
      `config_.meta.configuration_hash = computeConfigurationHash(config_);` immediately before
      `MachineConfigWriter{config_}.write(out.string());`.
- [x] Confirmed (not assumed) the registry-driven parity test does not apply to C++, the same way
      Steps 3 and 4 confirmed it for Node.js and Go: `grep` for `v1_0`/`v1_1`/`Hdf5AdapterV1_0`/
      `Hdf5AdapterV1_1` in `include/machine_config/hash.hpp` → zero matches. C++'s
      `computeConfigurationHash` hashes the shared `MachineConfig` struct directly via
      `nlohmann::json` ADL (`to_json` free functions), with no per-version dict-builder in its own
      path — the `firmware_version`-class bug is structurally impossible here, same as
      Rust/Node.js/Go. No test written for this reason.
- [x] 5 new tests (`tests/test_capabilities.cpp`): `CapabilityIsValidFalseForExternallyAuthored
      ReferenceFixture`, `CapabilityIsValidTrueForSelfWrittenFile`,
      `CapabilityIsValidFlipsLiveAcrossAnUnsavedEditThenASave` (the live-ness + save→live
      round-trip test this whole design exists to satisfy, mirroring every other language's
      equivalent — edits via `setScanner`, checks `false` with no save, then `true` immediately
      after `save()` with no reopen, then confirms a completely fresh `MachineConfigFileV1_0::open()`
      session agrees), `CapabilityV1_0SaveStillRoutesThroughRealHashComputation`, and
      `CapabilityV1_1SaveComputesRealHashAndUpdatesSession` (using
      `fixtures/reference_config_v1_1.h5` through `MachineConfigFileV1_1::open()` directly, since
      this test file — unlike Rust's/Python's/Node.js's/Go's — had never previously exercised the
      v1.1 facade at all; added `MachineConfigFileV1_1` and a `REFERENCE_V1_1` constant to the file
      for this purpose).
- [x] Full suite: `cmake --build build --config Debug --target machine_config_tests` and the same
      for `Release` both build clean; `machine_config_cli` also rebuilt clean in Release to confirm
      the whole build, not just tests, is green. Both `build/tests/Debug/machine_config_tests.exe`
      and `build/tests/Release/machine_config_tests.exe` run with **all tests passed (880 assertions
      in 167 test cases)** (up from 842/162; +5 new tests), 0 failed.
- [x] Update `docs/clearbox-tauri-integration.md` §6 — **done, 2026-09-15**, the closing action for
      this entire plan. Deferred from Step 1 (paused there specifically to update this plan document
      mid-session, per direct request) and picked back up once all 5 languages were complete. Full
      detail tracked in `CONFIGURATION_HASH_PLAN.md` Step 8, which originally owned this doc update:
      §6 now documents `MachineConfigFile::is_valid()` (the capabilities-facade method, matching
      clearbox-tauri's real `open_machine_config()`-based integration) rather than
      `config.meta.is_valid` (the `MachineConfigReader`-only field the doc's original checklist item
      had assumed, before this plan's own "Critical finding" section established that field stays
      permanently unset on the facade path clearbox-tauri actually uses).

## Tests (per language, mirroring the shape below)

- Open a file via the capabilities facade whose stored hash matches its content (e.g. a file this
  session's own `save()` just wrote) → `is_valid()`/`isValid()` is `true`.
- Open `fixtures/reference_config.h5` via the capabilities facade → `is_valid()` is `false` (same
  permanent regression guard `CONFIGURATION_HASH_PLAN.md` established at the `MachineConfigReader`
  layer — externally-authored, expected invalid forever).
- **Live-ness, the test this whole design decision exists to satisfy**: open a valid, self-written
  file (`is_valid()` starts `true`) → mutate any field via the session's `set_*`/`Set*` API,
  *without saving* → `is_valid()` must now read `false`, in the same session, with no `save()`/
  reopen in between. This is the test a cached-at-`open()`-time field could never pass.
- **The save→live round trip**: continuing from the same session, `save()` → call `is_valid()`
  again *without reopening* → must read `true`, and the stored hash must differ from what it was
  before the edit. This is the test that catches both the original write-side bug (stale hash on
  save) and a regression of the "`save()` must also update session state" requirement specifically
  — a version of this test that reopens the file fresh instead of checking the live session would
  pass even if `save()` forgot to update `self.config`/`self._data`, so reopening is not a
  substitute for this check, only a complement to it.
- Open a file via the facade, mutate a field, `save()` to a new path, then open that new path fresh
  (via the facade again, in a *new* session) → `is_valid()` is `true` and the stored hash differs
  from the original file's stored hash.
- For the two language/version combinations kept intentionally correct already (Rust v1.0, C++
  both) — a simple regression test asserting `save()` still routes through the real hash
  computation and still updates session state, so a future refactor can't silently reintroduce
  either bypass.
- **New, Python only (confirmed not applicable to Node.js — see Step 3) — registry-driven
  dict-builder parity** (the standing guard against a repeat of the `firmware_version` bug):
  enumerate `supported_file_versions()` (not a hardcoded `["1.0", "1.1"]` list — the point is that
  this test must not need editing when a version is added), build one `MachineConfig` with every
  known StableModel field populated (reuse or extend `MockConfigBuilder`), run every registered
  version's `_config_to_dict()` over that same config, and assert all
  outputs are equal. Should fail loudly, by construction, the moment any two registered versions'
  builders disagree on any field — exactly the failure Step 6 of `CONFIGURATION_HASH_PLAN.md`
  found by accident.

## Verification

- Each language's own test suite green after its step.
- Manual spot check per language: open `fixtures/reference_config.h5` via the capabilities facade
  specifically (not `MachineConfigReader`) and confirm `is_valid()` reads `false`; round-trip a
  file through the facade's own `open()` → mutate → confirm `is_valid()` flips to `false` live →
  `save()` → confirm `is_valid()` flips back to `true` live (no reopen) → reopen fresh and confirm
  it still reads `true` there too.
- Re-read `docs/clearbox-tauri-integration.md` once all 5 languages are done and confirm it no
  longer describes an unreachable or fictional API, and that it correctly describes `is_valid()`
  as something to call after any edit, not just once after opening. **Done, 2026-09-15** — §6 now
  documents `open_machine_config()`/`MachineConfigFile::is_valid()` with the live-call pattern and
  the post-edit-`false`-is-expected caveat.

## On step granularity

One language per implementation session, same as `CONFIGURATION_HASH_PLAN.md` — held up well in
practice across all 5 languages. In hindsight, Python turned out to be the one language genuinely
worth the extra care flagged for it: (1) `is_valid()` needs an explicit dict→model reconstruction
step (`config_from_dict`) its facade doesn't otherwise need, and its `save()` fix genuinely needed
two separate writes (config *and* `self._data`) because that reconstruction produces a distinct
object — and (2) it's the only language that needed the new registry-driven dict-builder parity
test at all. Node.js was originally grouped with Python on both counts by assumption;
investigating rather than assuming during Step 3 showed neither applied (its `asJson` cast means
one write suffices, and its hash path has no per-version dict-builder to diverge in the first
place). Go turned out closest to Rust: a real interface change, but one that stayed entirely
self-contained (Go isn't part of the shared generator at all), a pointer-based config that made
the "session state" requirement close to automatic, and a second independent confirmation (via
`grep`, not assumption) that the parity-test concern doesn't apply outside Python. C++ turned out
to hold its own small surprise despite looking like the safest step going in (its "Critical
finding" table showed ✅/✅ for `save()` already routing through `MachineConfigWriter`, unlike every
other language's at least one ❌): that table only ever tracked *which writer gets called*, not
*whether the session's own retained config gets the fresh hash stamped back onto it* — and it
turned out neither C++ version did, the exact same "session state" gap Rust's v1.0 had in Step 1,
just not visible from the original ✅/✅ framing. A second, unrelated surprise: unblocking the
*library* build (`MachineConfigFileV1_0`/`V1_1` implementing the new pure-virtual `isValid()`) and
unblocking the *test* build were two separate fixes, not one — `FakeMachineConfigFile` in
`tests/test_capabilities.cpp` is itself an `IMachineConfigFile` implementer, invisible to a
`machine_config_cli`-only rebuild, and only surfaced once `machine_config_tests` was rebuilt
specifically. A good example of why this plan keeps re-verifying language-specific assumptions per
step, including ones a prior step's own summary table seemed to have already settled, rather than
carrying them forward from the original draft.

This closes every per-language step in this plan, and, as of 2026-09-15, its one remaining
documentation item too: `docs/clearbox-tauri-integration.md` §6, deferred at the end of Step 1
specifically to update this plan document first, was rewritten once all 5 languages were done
(tracked in full under `CONFIGURATION_HASH_PLAN.md` Step 8, the plan that originally owned that
doc). It now calls `MachineConfigFile::is_valid()` — the capabilities-facade method this plan
added, matching clearbox-tauri's real `open_machine_config()`-based integration, not
`MachineConfigReader` — after opening *and* after any edit (not just once after opening, since
it's live), and documents `false` right after an edit as expected/informational, not alarming, per
the caveat section above. This plan has no further open items.
