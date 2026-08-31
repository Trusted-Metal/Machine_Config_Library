# File_Version Dispatch Registry — Implementation Plan

**Status: done and verified in all 5 languages, 2026-08-31.**

**Scope:** standardize how all five languages dispatch on `File_Version` — reader, writer,
and the optional get/set capabilities facade — onto a real, data-driven registry, before any
v1.1 content lands. This is a precursor to `V1_1_IMPLEMENTATION_PLAN.md`, requested
explicitly so that adding v1.1 (and v1.2, v1.3, …) becomes "add one registry entry" in every
language, not "re-derive how this particular language's ad-hoc dispatch works." No
`File_Version` other than `"1.0"` is registered anywhere by this plan — behavior must be
byte-identical before and after.

**Explicitly not part of this work:** any of v1.1's actual schema/content changes (see
`file_testing/v1_0_to_v1_1.md` and `V1_1_IMPLEMENTATION_PLAN.md`). This plan only changes
*how* a version is looked up, never what it resolves to.

---

## Current state (verified against real code, not docs, 2026-08-31)

| Concern | Python | Node.js | Rust | C++ | Go |
|---|---|---|---|---|---|
| Plain Reader | ✅ `ReaderAdapter` Protocol + `_ADAPTERS` dict (`reader.py:20,27`) | ✅ `ReaderBackend` type + `_READERS` record (`reader.ts:17,32`) | ❌ concrete field `backend: Hdf5AdapterV1_0`, `match` (`reader.rs:16-30`) | ❌ `adapter()` returns concrete `Hdf5AdapterV1_0` by value, `if` chain (`reader.hpp:62-69`) | ❌ **3 separate switches**, one per method, no adapter object at all (`reader.go:34,48,62`) |
| Plain Writer | ✅ `WriterAdapter` Protocol + `_ADAPTERS` dict (`writer.py:16,21`) | ✅ `WriterBackend` type + `_WRITERS` record (`writer.ts:19`) | ❌ `match` (`writer.rs:27-30`) | ❌ `if` chain (`writer.hpp:18-34`) | ❌ `switch` (`writer.go:22-27`) |
| Facade `open` | ✅ `_OPEN` dict (`capabilities/__init__.py:45`) | ✅ `OPEN` record (`capabilities/index.ts:20`) | ❌ no interface exists — `generated.rs` emits only `SetMode`, despite its own header comment claiming "facade trait outlines"; `match` in `mod.rs:24-29` | ✅ **already solved** — `IMachineConfigFile` is generated (`capabilities/generated.hpp:12`) and `openMachineConfig` already returns it polymorphically (`capabilities/file.hpp:17-36`); only the `if (fv != "1.0")` guard (line 21) needs to become a real map | ❌ `type File = v1_0.File` **type alias** (`capabilities/file.go:9`) — cannot point at 2 concrete types once v1.1 exists; `if` guard (`file.go:17,28`) |
| Facade `create` | ❌ hardcoded `if fv == "1.0"` (`capabilities/__init__.py:66-75`) — not table-driven even though `open` is | ❌ same gap (`capabilities/index.ts:47-59`) | ❌ same root cause as `open` | ❌ same root cause as `open`; also returns concrete `MachineConfigFileV1_0` unlike `open` | ❌ same root cause as `open` |

Two findings shape the design below:

1. **C++ already solved the hard part.** `IMachineConfigFile` is generated and
   `openMachineConfig` already returns it polymorphically. Its facade fix is genuinely
   small: one `if` becomes one map.
2. **Rust is missing a generated interface that Python, C++, and Node.js all already
   have.** `tools/generate_capabilities.py` emits a facade Protocol/interface for those
   three languages (`capabilities/generated.py:63`'s `MachineConfigFile(Protocol)`,
   C++'s `IMachineConfigFile`, TS's `generated.ts:73` `interface MachineConfigFile`) but
   only `SetMode` for Rust. Closing this is this plan's one genuinely new piece of
   infrastructure, not new per-version logic.

---

## Design decisions

### Forced vs. chosen — which parts of this plan are unavoidable

"Standardize onto registries" sounds like one uniform decision; it's actually two
different kinds of change bundled together, worth separating before anyone re-derives
this from scratch mid-implementation.

**Forced by the type system — some unifying mechanism is unavoidable the moment a second
version's concrete adapter type differs from the first, regardless of which
implementation path is chosen:**
- Rust: `capabilities/mod.rs`'s `open_machine_config`/`create_machine_config` currently
  return `Result<MachineConfigFileV1_0, CapabilityError>` — a concrete type.
  `reader.rs`'s `MachineConfigReader` holds `backend: Hdf5AdapterV1_0`, also concrete.
  Neither compiles with "just add a match arm returning a different concrete type" once
  v1.1's adapter exists — an enum or a trait is required, not optional.
- C++: `reader.hpp`'s `adapter()` returns `Hdf5AdapterV1_0` **by value** — same forcing.
  (`capabilities/file.hpp`'s facade is the one exception: `IMachineConfigFile` already
  exists, so that half of C++ was never forced to change shape, only its dispatch
  mechanism.)
- Go: `capabilities/file.go`'s `type File = v1_0.File` is a type alias — categorically
  cannot name two concrete types. Forced.

**Chosen for consistency/hygiene, not required by any compiler or type system:**
- Go's `reader.go` consolidating 3 independent switch statements into 1 registry — each
  switch, individually, could take a `case "1.1":` with zero type problems (each already
  dispatches to a free function and returns a uniform `(*MachineConfig, error)` /
  equivalent shape). Fixing the duplication is worthwhile, but nothing forces it.
- Python's/Node.js's `create_machine_config`/`createMachineConfig` hardcoded `if` instead
  of a dict — inconsistent with their own sibling `open`/`_OPEN`, but was never going to
  cause a compile/type error either way. Fixing it is hygiene.

**A genuine, still-open design choice within the forced cases**: Rust's and C++'s forced
unification could be satisfied by either a **closed enum** (lighter — no heap allocation,
more idiomatic for a small, deliberately-curated version set that nothing plugs into at
runtime) or the **trait-object / virtual-base-class** approach this plan specifies
(heavier — a `Box<dyn Trait>` / `std::unique_ptr<AbstractBase>` per adapter instance, plus
a new trait/interface to define and keep in sync). An enum-backed `HashMap` is exactly as
legitimate a "real registry" as one holding trait objects — the difference is entirely in
how the *stored values* get a common shape, not in whether a registry data structure
exists at all.

**Why this plan specifies the heavier trait-object approach anyway:** closed enums are
poorly suited to the test-injection pattern the testing section below relies on — adding
a test-only variant to a closed enum means threading `#[cfg(test)]` through every `match`
that consumes it, which is both uglier and weaker coverage than a trait object, where any
type implementing the trait can be boxed and inserted into a test-local registry using the
*exact same* dispatch code path production uses. Given the explicit requirement for
per-language tests that prove correct implementation (not just "it compiles"), trait
objects are the better fit despite the extra weight — a deliberate tradeoff being named
here, not a default reached by inertia. If a future review decides the weight isn't worth
it, the enum form is a legitimate fallback for Rust/C++ specifically — it would not
weaken anything forced above, only the ease of the fake-version-injection tests.

### Discovered during C++ implementation: the generated facade interface must mirror its own language's concrete class, in full — never a narrower copy

While generalizing C++'s `createMachineConfig` to match `open()`'s already-polymorphic
return type, a real, separate gap surfaced: C++'s generated `IMachineConfigFile` only had
4 methods (`fileVersion`, `opticalTrainCount`, `save`, `close`), while `MachineConfigFileV1_0`
itself has ~23. Checking Python's `MachineConfigFile(Protocol)` and Node's `MachineConfigFile`
interface confirmed those are the **full, rich surface** (handle-based:
`file.opticalTrain(i).getScanner()`) — C++'s narrowness wasn't a deliberate design choice
mirrored across languages, it was an incomplete hardcoded template. Root cause, confirmed
by reading `tools/generate_capabilities.py` directly: this "generator" isn't schema-driven
at all — every `render_*` function explicitly discards its `api`/`models` arguments
(`_ = api, models`) and returns a hand-maintained string template. `render_py`/`render_ts`
were kept in sync with the real facade surface; `render_rs`/`render_hpp` were not.

**Resolution, and the principle for Rust/Go's still-pending interface work:** the fix is
not to unify shape across languages (Python/Node use handle objects; Rust/C++/Go's actual
concrete facades are flat/index-based — `file.getScanner(index)` directly, no handle
indirection — and redesigning that is a real facade-API project, unrelated to dispatch).
The fix is: **each language's generated interface must mirror that language's own
concrete v1.0 class in full**, so the version-agnostic path never loses capability
relative to using the concrete class directly. Applied to C++ (below); the same principle
applies when Rust's trait is generated and when Go's interface replaces its type alias —
each should match its *own* current concrete surface, not copy C++'s old gap or Python/
Node's shape. Go's own concrete `v1_0.File` is separately narrower than Rust/C++'s
(missing `SetOpcua`, `SetClearbox`, `GetLightSource`/`SetLightSource`,
`GetCollimator`/`SetCollimator`, `GetScannerCard`/`SetScannerCard`, `GetMachine`/
`SetMachine`, `GetTrain`/`SetTrain`) — a real, pre-existing completeness gap, independent
of dispatch, noted here so it isn't silently backfilled as a side effect of Go's
dispatch-registry work and isn't lost either — worth its own follow-up.

### The shared testing pattern (applies to Rust, C++, Go identically)

Python and Node.js already support test-time registry injection for free — their
registries are plain module-level dicts, and tests already do this exact thing for the
mock v1.1 adapters (`python/tests/test_adapter_migration.py:616`:
`monkeypatch.setitem(_reader_mod._ADAPTERS, "1.1-mock", MockV1_1Reader)`, reverted
automatically after the test). Rust/C++/Go have no such precedent, and mutating a shared
global registry from a test is exactly the kind of shared-mutable-state hazard worth
avoiding, especially under parallel test execution (Rust's default).

**Decision: every new dispatch entry point in Rust/C++/Go takes the registry as an
explicit argument, with the public zero-argument API supplying the real one.**

```rust
// Illustrative shape, not final code:
fn resolve_reader(version: &str, registry: &ReaderRegistry) -> Result<Box<dyn ReaderAdapter>> { ... }
pub fn open(path: &Path) -> Result<MachineConfigReader> {
    resolve_reader(&peek_file_version(path)?, &PRODUCTION_READER_REGISTRY)...
}
```

Tests build a small local registry (the real `"1.0"` entry plus one throwaway fake, e.g.
key `"9.9-test"`) and call the two-argument form directly. This proves the dispatch
mechanism is genuinely data-driven — new version in, correct adapter out, unknown version
rejected — without ever touching shared/global state, identically in Rust, C++, and Go.
Go's map-based registry is simple enough that a package-level `var` plus `t.Cleanup` would
also work, but the injectable-parameter form is used for consistency across all three and
because it's the only option that's actually clean in Rust.

### Go's `*File` → `File` return-type change

Once `type File = v1_0.File` becomes `type File interface { ... }`,
`OpenMachineConfig`/`CreateMachineConfig` should return `File` (an interface value), not
`*File` (a pointer to an interface, which is not idiomatic Go and not what the alias
implied before). Callers who wrote `f, err := capabilities.OpenMachineConfig(path)` and
just use `f` are unaffected by type inference; a caller who explicitly annotated
`var f *capabilities.File` would need to drop the `*`. Flag in the CHANGELOG as a minor
source-compatibility note — not expected to affect real callers, but real enough to state
plainly rather than discover silently.

### Rust and Go interface scope

The new `ReaderAdapter`/`WriterAdapter` traits (Rust) and interface (Go) should cover
exactly the current public method surface of `Hdf5AdapterV1_0`/`Hdf5WriterV1_0` /
`v1_0hdf5`'s free functions — no speculative methods. For Rust that's `parse`,
`parse_with_binary`, `get_correction_data`, `get_inverse_correction_data`,
`get_scan_field_correction_bytes`, `get_raw_group`, `to_json` (reader) and `write`
(writer) — confirmed the current full list via `rust/src/capabilities/v1_0/hdf5.rs`'s and
`writer.rs`'s `pub fn` signatures. For Go, `Parse`/`GetCorrectionData`/
`GetInverseCorrectionData` (reader) and `Write` (writer) — confirmed via `reader.go`'s
three methods and `writer.go`'s one. Go's structural typing means the existing
`v1_0hdf5` functions likely satisfy the new interface via a thin wrapper struct with zero
changes to `v1_0hdf5` itself — confirm this during implementation before writing any
adapter shim.

---

## Per-language checklist

### Python (smallest lift)

**Status: done and verified, 2026-08-31.**

- [x] `python/src/machine_config/capabilities/__init__.py` — added
      `_CREATE = {"1.0": MachineConfigFileV1_0.create}` next to the existing `_OPEN`;
      rewrote `create_machine_config` to look up `_CREATE` via `.get(fv)` instead of
      `if fv == "1.0"`, matching `open_machine_config`'s existing shape exactly (same
      `None`-check-then-`err(...)` pattern, identical error message text — the mechanism
      changed, the behavior did not).
- [x] Tests (`python/tests/test_capabilities.py`):
  - [x] `test_create_machine_config_rejects_unregistered_version` — unchanged behavior,
        confirms the error path survives the refactor (`create_machine_config("2.0")` →
        `UnsupportedVersion`).
  - [x] `test_create_machine_config_dispatches_via_registered_version` — via
        `monkeypatch.setitem(_capabilities_mod._CREATE, "9.9-test", lambda version:
        ok(sentinel))`, asserts `create_machine_config("9.9-test")` returns the sentinel,
        proving `_CREATE` is genuinely consulted, not a hardcoded check that happens to
        still work.

**Verification:** `pytest python/tests/`: **360 passed, 0 failed** (up from 358 — 2 new
test functions, 0 regressions). **Found, not fixed (pre-existing, unrelated):**
`scratch/smoke_test.py` fails with `AttributeError: 'OpticalTrain' object has no
attribute 'clearbox'` — it accesses `t0.clearbox` directly instead of
`t0.optional_components.clearbox`. Confirmed via `git diff --stat
scratch/smoke_test.py` (no output — file untouched by this change) that this is stale
drift in the smoke-test script itself, unrelated to the dispatch-registry refactor. Not
fixed here — out of this plan's scope.

### Node.js (smallest lift)

**Status: done and verified, 2026-08-31.**

- [x] `nodejs/src/capabilities/index.ts` — added `export const CREATE: Record<string,
      (version: string) => Result<MachineConfigFile, CapabilityError>>` next to the
      existing (unexported, unchanged) `OPEN`; rewrote `createMachineConfig` to look up
      `CREATE[fv]` instead of `if (fv !== '1.0')`, matching `openMachineConfig`'s
      existing shape exactly (same `if (!creator)` / `err(...)` pattern, identical error
      message text). `CREATE` is exported with a header comment matching `reader.ts`'s
      `_READERS` / `writer.ts`'s `_WRITERS` convention — note the actual naming here is
      `CREATE` (no underscore prefix, since it's a new binding in a module that doesn't
      otherwise use the underscore convention for `OPEN`), not the `CAPABILITIES_CREATE`
      name this plan originally sketched.
- [x] Tests (`nodejs/tests/capabilities.test.ts`):
  - [x] `createMachineConfig rejects an unregistered version` — unchanged behavior,
        confirms the error path survives the refactor (`createMachineConfig('2.0')` →
        `UnsupportedVersion`).
  - [x] `createMachineConfig dispatches via a registered version` — directly assigns
        `CREATE['9.9-test'] = () => ok(sentinel)`, asserts `createMachineConfig('9.9-test')`
        returns the sentinel, `delete`s the entry in a `finally` block (matching
        `adapterMigration.test.ts`'s existing `_READERS`/`_WRITERS` injection style,
        since Vitest has no `monkeypatch`-style auto-revert).

**Verification:** `npx vitest run` (full suite): **191 passed, 0 failed** (up from 189 —
2 new test functions, 0 regressions), across all 6 test files. `npx tsc --noEmit`: clean,
zero type errors. `npm run build`: clean.

### C++ (leverages the existing `IMachineConfigFile` interface)

**Status: done and verified, 2026-08-31.**

- [x] `cpp/include/machine_config/adapters.hpp` (**new file**) — `ReaderAdapter` /
      `WriterAdapter` abstract base classes, pure virtual methods mirroring
      `Hdf5AdapterV1_0`'s / `Hdf5WriterV1_0`'s exact current public surface (`parse`,
      `parseWithBinary`, `toJson`, `getRawGroup`, `getCorrectionData`,
      `getInverseCorrectionData`, `getScanFieldCorrectionBytes` / `write`). Its own header,
      not folded into `reader.hpp`/`writer.hpp`, specifically to avoid a circular
      `#include` — the concrete adapters need to inherit from it while
      `reader.hpp`/`writer.hpp` need to include the concrete adapters to build their
      production registries.
- [x] `cpp/include/machine_config/capabilities/v1_0/hdf5.hpp` — `Hdf5AdapterV1_0 : public
      ReaderAdapter`, `override` added to all 7 methods.
- [x] `cpp/include/machine_config/capabilities/v1_0/writer.hpp` — `Hdf5WriterV1_0 : public
      WriterAdapter`, `override` added to `write`.
- [x] `cpp/include/machine_config/reader.hpp` — `ReaderRegistry` (version →
      `std::function<unique_ptr<ReaderAdapter>(path)>`), `productionReaderRegistry()`,
      registry-parameterized `resolveReader(version, path, registry)` (throws the same
      `std::runtime_error` message as before on an unregistered version).
      `MachineConfigReader::adapter()` now returns `unique_ptr<ReaderAdapter>` via
      `resolveReader`, constructed fresh per call exactly as before (no new caching, no
      lifetime change).
- [x] `cpp/include/machine_config/writer.hpp` — same shape: `WriterRegistry`,
      `productionWriterRegistry()`, `resolveWriter(version, cfg, registry)`.
      `MachineConfigWriter::write()` unchanged in every other respect (same
      trim/default-to-"1.0" logic, same exception message).
- [x] `cpp/include/machine_config/capabilities/file.hpp` — `OpenRegistry`/
      `productionOpenRegistry()`/`openMachineConfigWithRegistry()`/`openMachineConfig()`
      (registry replaces the `if` chain, same error codes/messages); identical shape for
      `CreateRegistry`/`productionCreateRegistry()`/`createMachineConfigWithRegistry()`/
      `createMachineConfig()`. `supportedFileVersions()` now derives from
      `productionOpenRegistry()`'s keys instead of the literal `{"1.0"}`.
  - [x] **`createMachineConfig`'s return type changed to `Result<shared_ptr<IMachineConfigFile>>`**,
        matching `open()` — made safe only because `IMachineConfigFile` was completed
        first (see finding above); confirmed zero behavior change by re-running the two
        existing tests that exercise `create()`'s rich methods through `auto`
        (`CapabilityCreateSetMetaSaveReopen`, `CapabilityGetCorrectionDataWorksOnCreateBasedInstanceWithoutTouchingDisk`)
        unmodified — both still pass, proving no capability was lost.
- [x] `tools/generate_capabilities.py`'s `render_hpp` — expanded `IMachineConfigFile` from
      4 to 24 methods (the full `MachineConfigFileV1_0` surface); regenerated
      `generated.hpp`. `render_rs`/`render_py`/`render_ts` untouched.
- [x] `cpp/include/machine_config/capabilities/v1_0/file.hpp` — `override` added to all 20
      newly-covered methods (`getMeta`/`setMeta` through `getOpcua`/`setOpcua`); zero
      method bodies changed.
- [x] Tests (`cpp/tests/test_capabilities.cpp`, Catch2):
  - [x] `FakeReaderAdapter`/`FakeWriterAdapter`/`FakeMachineConfigFile` — minimal stub
        implementations (bodies never exercised; the tests only prove a version string
        routes to the fake's *type*, via `dynamic_cast`, not the real "1.0" adapter).
  - [x] `writeFileVersionOnly(version, tag)` helper — writes only the root `File_Version`
        attribute (mirrors `test_reader.py`'s `test_unknown_file_version_does_not_use_v1_layout`
        precedent), enough for `peekFileVersion()`/`openMachineConfig` tests without a
        full valid fixture.
  - [x] `ReaderRegistryRejectsUnregisteredVersion`, `ReaderRegistryDispatchesViaInjectedTestAdapter`
  - [x] `WriterRegistryRejectsUnregisteredVersion`, `WriterRegistryDispatchesViaInjectedTestAdapter`
  - [x] `CapabilityOpenRejectsUnregisteredVersion`, `CapabilityOpenDispatchesViaInjectedRegistryEntry`
  - [x] `CapabilityCreateRejectsUnregisteredVersion`, `CapabilityCreateDispatchesViaInjectedRegistryEntry`

**Verification:** `cmake --build cpp/build --config Release --target machine_config_tests`
clean (pre-existing "compiler doesn't support deprecating using statements" warnings only,
unrelated). Full suite (`machine_config_tests.exe`, all 7 test files in one binary):
**591 assertions, 113 test cases, 0 failures** (+8 test cases from this change). The
HDF5-DIAG stderr output during the run is expected — the two "rejects unregistered
version" tests deliberately open minimal/malformed files, HDF5 logs its internal error
verbosely, and the code correctly catches it into an `Err` Result.

### Rust (the one language needing new generated infrastructure)

**Status: done and verified, 2026-08-31.**

- [x] `tools/generate_capabilities.py`'s `render_rs` — extended to emit a `MachineConfigFile`
      trait into `rust/src/capabilities/generated.rs`; regenerated. Mirrors
      `MachineConfigFileV1_0`'s own concrete surface exactly (confirmed by reading it
      directly first) — **not** identical to C++'s completed `IMachineConfigFile`:
      `get_clearbox` returns `Result<Option<ClearBox>, CapabilityError>` (Rust already
      models "absent" as `Ok(None)`, not a separate `has_optional_components` +
      `Err(NotPresent)` the way C++ does), and Rust has `set_train` where C++ has none.
      Confirms the "mirror your own language's concrete class, never another
      language's shape" principle from the finding above in practice, not just in theory.
- [x] `rust/src/capabilities/v1_0/file.rs` — `MachineConfigFileV1_0` gained a **separate**
      `impl MachineConfigFile for MachineConfigFileV1_0` block that delegates to the
      existing inherent methods (zero method bodies touched). Safe because Rust always
      resolves an inherent method over a trait method of the same name on the same type —
      confirmed this is not accidental recursion, it's standard, documented Rust method
      resolution.
- [x] `rust/src/reader.rs` — hand-written `ReaderAdapter` trait (7 methods, mirrors
      `Hdf5AdapterV1_0` exactly) + `ReaderRegistry`
      (`HashMap<&'static str, fn(&Path) -> Result<Box<dyn ReaderAdapter>>>`) +
      `std::sync::LazyLock` production singleton + registry-parameterized `resolve_reader`.
      `MachineConfigReader.backend` is now `Box<dyn ReaderAdapter>`, constructed fresh per
      `open()` call exactly as before (no new caching).
- [x] `rust/src/capabilities/v1_0/hdf5.rs` — `impl ReaderAdapter for Hdf5AdapterV1_0`,
      delegating, same pattern as the facade.
- [x] `rust/src/writer.rs` — hand-written `WriterAdapter` trait. **One real design
      subtlety**: `Hdf5WriterV1_0<'a>` holds `&'a MachineConfig` (a lifetime parameter),
      so the trait object needs a matching lifetime bound
      (`Box<dyn WriterAdapter + 'a>`), and the constructor-registry's function type needs
      an explicit higher-ranked bound: `type ConstructFn = for<'a> fn(&'a MachineConfig)
      -> Box<dyn WriterAdapter + 'a>`. Also, `Hdf5WriterV1_0::write` is generic
      (`<P: AsRef<Path>>`), which is not object-safe — `WriterAdapter::write` takes a
      concrete `&Path` instead; `MachineConfigWriter::write` stays generic for callers
      and converts once via `path.as_ref()` before reaching the trait object. Verified
      this compiles and passes on the first `cargo build` after writing it — the lifetime
      reasoning held.
- [x] `rust/src/capabilities/v1_0/writer.rs` — `impl WriterAdapter for Hdf5WriterV1_0<'a>`,
      delegating (`path: &Path` satisfies the inherent generic `write<P: AsRef<Path>>` via
      the reflexive `&Path: AsRef<Path>` impl).
- [x] `rust/src/capabilities/mod.rs` — `OpenRegistry`/`CreateRegistry` (both
      `HashMap<&'static str, fn(...) -> Result<Box<dyn MachineConfigFile>,
      CapabilityError>>`), `LazyLock` production singletons, registry-parameterized
      `resolve_open`/`resolve_create`. `open_machine_config`/`create_machine_config` both
      now return `Box<dyn MachineConfigFile>` (previously `create_machine_config` already
      did — no facade-completeness gap existed in Rust the way it did in C++, since
      Rust's facade trait didn't exist at all until this change). `supported_file_versions()`
      changed from `&'static [&'static str]` to `Vec<&'static str>` (an owned collection
      is the only option once the registry is a `HashMap`, not a literal array) — confirmed
      both call sites (`rust/tests/capabilities_test.rs`, `docs/validation/rust/app/src/scenarios.rs`)
      only ever call `.contains(&"1.0")`, which behaves identically on both types.
- [x] `rust/src/lib.rs` — added `MachineConfigFile` to the crate-root re-export list
      (`docs/contributing.md`/VALIDATION_PLAN.md §8 S-09's discipline: no public type
      should require a sub-module path).
- [x] Tests:
  - [x] Full existing suite re-run unmodified first, confirming zero regressions before
        adding anything — 145 tests, all passing. **Correction to this plan's original
        assumption**: `rejects_unknown_file_version`-equivalent tests exist in
        `writer.rs`'s own `#[cfg(test)]` module (`rejects_unknown_file_version`) and in
        `rust/tests/integration_test.rs` (`test_unknown_file_version_does_not_use_v1_layout`,
        `test_writer_rejects_unknown_file_version`) for both reader and writer — not
        "reader.rs and writer.rs" as originally sketched; `reader.rs` itself had none
        before this change.
  - [x] `reader.rs`: `reader_registry_rejects_unregistered_version` (new, filling the gap
        just noted), `reader_registry_dispatches_via_injected_test_adapter` (local
        registry, fake panics if actually used — reaching the assertion without touching
        the real fixture already proves dispatch).
  - [x] `writer.rs`: `writer_registry_rejects_unregistered_version`,
        `writer_registry_dispatches_via_injected_test_adapter` (same shape).
  - [x] `capabilities/mod.rs` (**new `#[cfg(test)]` module — none existed here before**):
        `open_registry_rejects_unregistered_version`, `open_registry_dispatches_via_injected_test_file`,
        `create_registry_rejects_unregistered_version`, `create_registry_dispatches_via_injected_test_file`
        (fake's `file_version()` returns a sentinel string, asserted directly — simpler
        than C++'s `dynamic_cast`, since Rust doesn't need RTTI for this), and
        `supported_file_versions_reflects_production_registry`.
  - [x] **One real compile error found and fixed**: `Result<T, E>::unwrap_err()` requires
        `T: Debug`, and `Box<dyn Trait>` doesn't implement `Debug` for an arbitrary trait.
        All four "rejects unregistered version" tests switched from `.unwrap_err()` to
        `matches!(result, Err(...))`, which needs no such bound.

**Verification:** `cargo build --all-targets` clean, zero warnings, both before and after
the test additions. `cargo clippy --all-targets`: 4 pre-existing warnings, all in files/lines
untouched by this change (`builder.rs:105,125`, `capabilities/v1_0/hdf5.rs:292,293` — an
unrelated boolean-reading helper elsewhere in a file this change did touch —, `error.rs:89`);
zero new warnings from any added code. Full `cargo test`: **154 passed, 0 failed** (up from
145 — 9 new test functions, 0 regressions) across lib unit tests + 3 integration test
binaries + doc-tests.

### Go (the deepest structural change)

**Status: done and verified, 2026-08-31.**

- [x] `go/capabilities/file.go` — `type File = v1_0.File` (a type alias, which cannot name
      two concrete types) replaced with a hand-written `File interface` (12 methods,
      mirroring `v1_0.File`'s own current surface exactly — kept the plan's own decision
      not to add Go to `generate_capabilities.py`, consistent with `go/capabilities/`
      being otherwise hand-authored). `*v1_0.File` satisfies it with **zero changes** to
      `v1_0/file.go` — confirmed, Go's structural typing made this automatic exactly as
      expected.
  - [x] `OpenMachineConfig` / `CreateMachineConfig` now return `File` (interface value),
        not `*File` (pointer-to-interface) — the return-type change flagged in "Design
        decisions" above.
  - [x] `OpenRegistry` / `CreateRegistry` (`map[string]func(...) (File, *Error)`) replace
        both `if` guards; `ResolveOpen` / `ResolveCreate` are the registry-parameterized
        resolvers. `SupportedFileVersions()` now derives from `productionOpenRegistry`'s
        keys instead of the literal `[]string{v1_0.FileVersion}`.
- [x] `go/reader.go` — new `ReaderAdapter` interface (`Parse`/`GetCorrectionData`/
      `GetInverseCorrectionData`) + `v1_0ReaderAdapter` (unexported, holds just `path`,
      calls the existing `v1_0hdf5.*` free functions). `ReaderRegistry`
      (`map[string]func(path string) ReaderAdapter`) + `ResolveReader` replace what were
      **three independent switch statements**, one per `MachineConfigReader` method, each
      separately re-implementing the same version check — now all three call the one
      shared `ResolveReader`/`productionReaderRegistry`. This is the actual fix for the
      duplication the plan called out, not just a relocation of it.
- [x] `go/writer.go` — same shape: `WriterAdapter` interface (`Write`), `v1_0WriterAdapter`
      (stateless — `Write` already takes `cfg`/`path` as arguments, so no per-instance
      state was needed, unlike the reader's path-holding wrapper), `WriterRegistry`
      (`map[string]WriterAdapter`, direct values rather than constructors — simpler,
      sufficient since nothing needs constructing), `ResolveWriter`.
- [x] Tests — all new registry types/resolvers exported (`ResolveReader`, `ResolveWriter`,
      `ResolveOpen`, `ResolveCreate`, and the registry types), since every existing test
      file in this codebase (`reader_test.go`, `writer_test.go`, `capabilities/file_test.go`)
      uses Go's external `_test` package convention (`package machineconfig_test`,
      `package capabilities_test`), which can only reach exported identifiers — confirmed
      this before deciding names, rather than assuming package-internal tests were an option.
  - [x] `go/reader_test.go`: `TestReaderRegistryRejectsUnregisteredVersion`,
        `TestReaderRegistryDispatchesViaInjectedAdapter` (fake's `Parse` returns a
        sentinel `MachineConfigMeta.MachineName`, asserted directly).
  - [x] `go/writer_test.go`: `TestWriterRegistryRejectsUnregisteredVersion`,
        `TestWriterRegistryDispatchesViaInjectedAdapter` (fake sets a `*bool` flag when
        `Write` is called — simpler than a sentinel return value here, since `Write`
        returns only `error`).
  - [x] `go/capabilities/file_test.go`: `TestOpenRegistryRejectsUnregisteredVersion`,
        `TestOpenRegistryDispatchesViaInjectedFile`, `TestCreateRegistryRejectsUnregisteredVersion`,
        `TestCreateRegistryDispatchesViaInjectedFile` (12-method `fakeFile` stub, matching
        the `File` interface — `FileVersion()` returns a sentinel, everything else is a
        trivial stub since only the sentinel is ever asserted), `TestSupportedFileVersionsReflectsRegistry`.
  - [x] Existing `TestUnknownFileVersionDoesNotUseV1Layout` (`reader_test.go`) and
        `TestWriterRejectsUnknownFileVersion` (`writer_test.go`) — the pre-existing,
        real end-to-end rejection tests the original sketch had in mind — re-run
        unmodified and still pass.

**Environment note, worth recording for future sessions in this repo**: Go's HDF5 layer
is CGo (`go/internal/h5c`), so `go build`/`go test` need a real C toolchain + HDF5 headers/
libs, not just the Go toolchain. This machine has neither on `PATH` by default
(`CGO_ENABLED=0` is the Go default here, and plain `gcc` isn't found even with it
force-enabled) — but MSYS2's MinGW64 toolchain *is* installed, just not wired into the
shell's `PATH`, matching what `.github/workflows/go.yml` already uses in CI. Working
invocation, confirmed:
```bash
export CGO_ENABLED=1
export CC="C:\\msys64\\mingw64\\bin\\gcc.exe"
export CXX="C:\\msys64\\mingw64\\bin\\g++.exe"
export PATH="/c/msys64/mingw64/bin:$PATH"
go test ./... -count=1
```

**Verification:** `go vet ./...` clean (exit 0, no output). Full `go test ./...
-count=1 -v`: **91 passed, 0 failed** (up from 82 — 9 new test functions, 0
regressions), across `machine-config-go`, `machine-config-go/capabilities`, and
`machine-config-go/internal/h5c`.

---

## Order of implementation

Python and Node.js first (smallest, lowest-risk, confirms the "registry + test injection"
shape works end-to-end before the harder languages). **Python done** — confirmed the
monkeypatch-based injection pattern works exactly as designed, zero surprises, zero
behavior change. **Node.js done** — same shape, same low risk, confirmed; the only
deviation from the plan's original sketch was the registry's exact name (`CREATE`, not
`CAPABILITIES_CREATE`), noted in Node's checklist above. **C++ done** — the facade half
was nearly free as expected (existing `IMachineConfigFile` interface), but surfaced a
real, separate completeness gap in that same interface (see finding above) that had to be
fixed first before `createMachineConfig` could safely be generalized to match `open()`;
the reader/writer half needed genuinely new abstract-base infrastructure
(`adapters.hpp`), more than originally sketched but well-contained. **Rust done** — the new generated trait (this language's one genuinely new piece of
infrastructure) was generated as the *full* flat surface from the start, matching its own
`MachineConfigFileV1_0` exactly rather than copying C++'s now-fixed old gap — confirmed it
even differs in shape from C++'s completed interface where Rust's own concrete class
genuinely differs (`get_clearbox`'s `Option`-based absence, `set_train`'s presence). The
`WriterAdapter` trait needed one genuinely new piece of reasoning C++ didn't (a
lifetime-bound trait object, `Box<dyn WriterAdapter + 'a>`, since `Hdf5WriterV1_0<'a>`
borrows rather than owns its config) — resolved correctly on the first build. **Go
done, last** — the type alias really did have to become an interface (forced, as
predicted), and it really did satisfy `*v1_0.File` with zero changes to that file
(confirmed, not assumed). The 12-method surface stayed exactly Go's own, deliberately
narrower than Rust/C++'s completed interfaces — no attempt made to backfill Go's own
missing `SetOpcua`/`SetClearbox`/etc., consistent with the finding above. The one
practical obstacle was environmental, not architectural: this machine's shell has no C
toolchain on `PATH` by default, and Go's HDF5 layer is CGo — resolved by locating the
already-installed MSYS2 MinGW64 toolchain and wiring it in explicitly (see Go's
verification note above), the same toolchain `.github/workflows/go.yml` already uses in
CI.

**All 5 languages are now done.** Every `File_Version` dispatch point (reader, writer,
optional capabilities facade) in Python, Node.js, C++, Rust, and Go is a real,
data-driven registry, individually verified with a registry-injection test proving
genuine dispatch (not a relocated hardcoded check), with zero regressions in any
language's existing suite. `V1_1_IMPLEMENTATION_PLAN.md` is unblocked on this
precursor.

Run that language's full existing test suite immediately after its own conversion, before
starting the next language — a regression here would be a v1.0 regression, silent until
`cross_check.py` runs, and cheaper to catch per-language than at the end.

## Verification

1. Each language's full existing suite, green, immediately after its own conversion.
2. The new registry-extensibility tests above, green, in every language.
3. `python tools/cross_check.py --verbose` (all 5 languages, all 5 phases) — must match a
   pre-refactor baseline run exactly. Record the before/after pass counts here once run.
4. Manual confirmation: `supported_file_versions()`/`SupportedFileVersions()`/
   `supportedFileVersions()` still return exactly `["1.0"]` in every language — this
   refactor changes the mechanism, not the registered content.

## Follow-on (not part of this plan)

`VALIDATION_PLAN.md` §9.3 documents that Rust/C++/Go's existing AV-09–11 mock-v1.1 tests
call their mock adapters directly because there was no dispatch-table to inject into.
Once this plan ships, that limitation no longer holds. Revisiting those tests to go
through the real public facade is a natural follow-on — not required by this plan, and
not started here.
