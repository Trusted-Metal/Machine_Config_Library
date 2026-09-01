# Cross-Language Parity Audit

**Status: documented, not yet started.** Deprioritized behind `V1_1_IMPLEMENTATION_PLAN.md`
by explicit decision (2026-08-31) — the v1.0→v1.1 adapter work proceeds first; this audit
is the record of what to come back to once that's underway or done, not a call to action
right now.

**Scope:** functional/API parity across Python, Node.js, Rust, C++, and Go — does each
language expose the same capabilities, produce the same JSON shape, and behave the same
on the same input file? This is distinct from `DISPATCH_REGISTRY_PLAN.md` (done, all 5
languages) — that plan made *how* each language dispatches on `File_Version` consistent;
this audit is about whether the *capabilities reachable once dispatched* are consistent.

**Method:** four parallel research passes, each reading full source files (not sampling)
across all 5 languages — capabilities-facade + plain-reader/writer API surface, StableModel
field-by-field parity, builder/mock parity + leftover hardcoded version strings, and
`VALIDATION_PLAN.md` scenario-coverage parity. Every finding below was independently
verified by reading the actual current file, not inferred from a plan document's claims.

---

## Tier 1 — Real bugs

These produce actually-wrong behavior on real inputs today, not just reduced capability.

- [ ] **C++ binary export is non-functional end-to-end.** Three compounding, independently
      confirmed issues:
  1. `Hdf5AdapterV1_0::toJson()` (`cpp/include/machine_config/capabilities/v1_0/hdf5.hpp:241-244`)
     declares `bool /*include_binary*/` — the parameter is unused — and its body always
     calls `parse()`, never `parseWithBinary()`. The comment directly above it
     ("Correction grids excluded unless include_binary=true") is stale.
  2. The CLI never exposes the option at all: `cpp/src/main.cpp:59` calls `reader.toJson()`
     with no arguments. Python's `cli.py`, Rust's `main.rs`, Node's `cli.ts`, and Go's
     `cmd/machine-config-cli/main.go` all implement a working include-binary flag; C++'s
     CLI has none.
  3. Independently, `to_json(json&, const ScanFieldCorrectionFile&)`
     (`cpp/include/machine_config/models.hpp:529-530`) is a stub —
     `if (s.raw_bytes.has_value()) j["raw_bytes"] = nullptr; // placeholder; replaced in §4.11`
     — so even a caller who manually calls `parseWithBinary()` and serializes directly gets
     `null`. `from_json` (`models.hpp:533-542`) never assigns `raw_bytes` at all, so reading
     a JSON file with a real base64 `raw_bytes` (e.g. from Python) silently drops it, no
     error. (`correction_data`/`inverse_correction_data` don't share this third bug — only
     issues #1/#2 block grids; raw_bytes is blocked by all three.)
  - **Impact:** no C++ caller can get correction grids or raw `.fc3` bytes via JSON, from
    the CLI or the API, in or out.

- [ ] **Rust's `raw_bytes` isn't base64-encoded.** The shared schema
      (`schema/machine_config_v1.schema.json:351`) requires `raw_bytes` as
      `{"type":["string","null"],"contentEncoding":"base64"}`. Python
      (`capabilities/v1_0/hdf5.py:953-955`, explicit `base64.b64encode`) and Node.js
      (`capabilities/v1_0/hdf5.ts:496-498`, explicit `.toString("base64")`) comply; Go's
      `RawBytes []byte` (`go/internal/models/models.go:383`) is base64-encoded
      automatically by `encoding/json`. Rust does not: `rust/src/models.rs:141` declares
      plain `Option<Vec<u8>>` with no `serde_bytes`/base64 annotation, and `to_json`
      (`capabilities/v1_0/hdf5.rs:433-439`) calls `serde_json::to_string(&config)`
      directly — serde's default renders `Vec<u8>` as a JSON array of integers (e.g.
      `[137,80,...]`). No `base64` crate exists anywhere in `rust/src`.
  - **Impact:** a Rust `export-json --include-binary` file fails schema validation and
    can't be parsed as bytes by a consumer expecting the documented string format. Not
    caught today because the cross-language golden-file test
    (`matches_python_golden_file`, `hdf5.rs:1269`) never exercises `include_binary=true`.

- [ ] **`x_axis`/`y_axis` nullability split.** Python (`models.py:204-205`), Rust
      (`models.rs:303-304`), C++ (`models.hpp:288-289`), and Go (`models.go:218-219`) all
      make `x_axis`/`y_axis` plain, non-optional `AxisConfig` — structurally required.
      TypeScript (`nodejs/src/models.ts:49-50`) types both `AxisConfig | null`, identical
      to `z_axis`/`focus`, and this is live behavior: `capabilities/v1_0/hdf5.ts:310-311`
      does `x_axis: xEnt ? parseAxis(...) : null`. Confirmed the other side reacts
      differently: Python's `grp["X_Axis"]` (`capabilities/v1_0/hdf5.py:408`) raises
      `KeyError` when the subgroup is missing; Rust's `require_group(grp, "X_Axis")?`
      (`capabilities/v1_0/hdf5.rs:598`) propagates an `Err`.
  - **Impact:** a file missing `Scanner/X_Axis` reads successfully in Node
    (`x_axis: null`) but raises in Python/Rust (C++/Go's identical non-optional shape
    implies the same, though their exact call sites weren't traced). The shared schema
    (`schema/machine_config_v1.schema.json:111-142`) types `x_axis`/`y_axis` as
    `["object","null"]` — agreeing with TS, not the four stricter languages. Worth
    deciding which side is "correct" before this causes a real support issue.

---

## Tier 2 — Real capability gaps

One language can do less than the others, without necessarily producing wrong output.

- [ ] **Go's facade is missing 7 methods** the other 4 have: `SetOpcua`, `SetClearbox`,
      `GetLightSource`/`SetLightSource`, `GetCollimator`/`SetCollimator`,
      `GetScannerCard`/`SetScannerCard`, `GetMachine`/`SetMachine`, `GetTrain`/`SetTrain`.
      (`go/capabilities/v1_0/file.go`) — already flagged during `DISPATCH_REGISTRY_PLAN.md`,
      reconfirmed accurate here.

- [ ] **Go's plain reader is missing `ToJson`/`GetRawGroup` entirely** — not merely
      unexposed on the public `MachineConfigReader` wrapper, genuinely unimplemented
      anywhere in the 918-line adapter file (`go/capabilities/v1_0/hdf5/hdf5.go`). Go's
      `MachineConfigReader` (`go/reader.go`) exposes only 4 methods total: `Parse`,
      `ParseWithOptions`, `GetCorrectionData`, `GetInverseCorrectionData`.
      `GetScanFieldCorrectionBytes` also has no dedicated accessor — the model captures
      raw bytes when `Parse(path, true)` is used, but there's no single-purpose getter
      like Rust/C++/Python provide.

- [ ] **Go's `Create()` facade path never builds a ClearBox**, unlike the other four
      languages' `Create()`. `go/capabilities/v1_0/file.go:38-71` hand-constructs a
      `MachineConfig` literal inline rather than calling Go's own `MockConfigBuilder`
      (`go/builder.go`, which defaults `IncludeClearbox: true` and is otherwise fine —
      confirmed used correctly elsewhere, e.g. `go/builder_test.go`,
      `go/internal/mockv1_1/mockv1_1.go:300`). Every other language's `Create()` does
      call its own `MockConfigBuilder` with `include_clearbox` defaulting `true`
      (`python/.../v1_0/file.py:49`, `rust/.../v1_0/file.rs:60`,
      `nodejs/.../v1_0/file.ts:77`, `cpp/.../v1_0/file.hpp:48-49`).
  - **Impact:** a Go `Create()`-based session can never exercise `GetClearbox`/
    `GetCorrectionData` with real data the way the other 4 languages' `Create()` can.

- [ ] **`MockConfigBuilder` construction-option surface diverges sharply.** Full
      comparison:

  | Option | Python | Node.js | Rust | C++ | Go |
  |---|---|---|---|---|---|
  | laser count | `n_lasers=2` kwarg | `nLasers=2` opt | `new(laser_count)` **required, no default** | `laser_count=2` field | `NLasers=2` field |
  | build_plate_z | kwarg, 20.0 | opt, 20.0 | field, 20.0 | field, 20.0 | **absent — hardcoded**, `builder.go:61` |
  | manufacturer/model/serial | kwargs w/ defaults | opts w/ defaults | fields w/ defaults | fields w/ defaults | **absent — hardcoded**, `builder.go:41-56`, cannot be overridden even by mutation |
  | `include_clearbox` | kwarg, True | opt, true | field, true | field, true | field, true |
  | `file_version` | kwarg, `"1.0"` | opt, `'1.0'` | **absent**, hardcoded `builder.rs:57` | **absent**, hardcoded `builder.hpp:59` | **absent**, hardcoded `builder.go:44` |
  | `working_distance_unit` | kwarg, `"mm"` | opt, `'mm'` | **absent**, hardcoded `builder.rs:112` | **absent**, hardcoded `builder.hpp:140` | **absent**, hardcoded throughout |

  Python and Node.js have full, matching 10-option constructors. Rust/C++ expose 8 of
  those as post-construction-mutable public fields but never expose `file_version`/
  `working_distance_unit`. Go has only 5 knobs total and, uniquely, cannot vary
  `manufacturer`/`model`/`serial_number`/`build_plate_z` *at all*.
  - **Impact:** "the same nominal build" produces a structurally different config in Go
    than in the other four for those fields. Only Python/Node.js can build a mock config
    at an explicit non-`"1.0"` version directly; Rust/C++/Go require reaching past the
    builder into the returned struct's public `meta.file_version` field afterward — worth
    keeping in mind once v1.1 adapters need mock configs at both versions.
  - Related, lower-confidence note: Rule-8 (unit-locking) test coverage looks asymmetric —
    Python's builder can parameterize `working_distance_unit` specifically to drive this
    test (`builder.py:255`); Rust/C++ implement Rule 8 but can't parameterize the unit
    through their builders, only via direct model mutation; Go has no "Rule 8" reference
    under that name at all. Flagged for follow-up, not independently confirmed as a live
    bug.

- [ ] **Python always eagerly loads binary correction/`.fc3` data on every `parse()`** —
      no metadata-only fast path. `MachineConfigReader` (`python/src/machine_config/reader.py:32-48`)
      is a blind `__getattr__` proxy onto `Hdf5AdapterV1_0`, whose `parse()`
      (`capabilities/v1_0/hdf5.py:508-509,599`) unconditionally reads both correction
      grids and raw `.fc3` bytes. Rust/C++ split this into `parse()`/`parse_with_binary()`;
      Node (`ReadOptions.includeBinary`, default `false`) and Go (`ParseOptions.IncludeBinary`)
      gate it behind an opt-in flag. Python alone always pays the I/O cost.

- [ ] **Node's plain reader has no dedicated raw-bytes accessor.** `MachineConfigReader`
      (`nodejs/src/reader.ts:36-74`) exposes 5 methods (`parse`, `toJson`,
      `getCorrectionData`, `getInverseCorrectionData`, `getRawGroup`) — no
      `getScanFieldCorrectionBytes` anywhere, confirmed absent from both the
      `ReaderBackend` type and the underlying `Hdf5AdapterV1_0` class
      (`nodejs/src/capabilities/v1_0/hdf5.ts:674-747`). The raw bytes are reachable only
      indirectly via `parse({includeBinary:true})`, and even then arrive pre-encoded as a
      base64 **string** (`hdf5.ts:496-499`), not a byte buffer from a dedicated accessor.

- [ ] **OPCUA-repair asymmetry.** Rust's `set_opcua` (`rust/src/capabilities/v1_0/file.rs:349-358`)
      and C++'s `setOpcua` (`cpp/.../file.hpp:251-258`) only check that the OPCUA block is
      *present* before allowing a write — they do not re-run the required-field
      validation their `get_opcua`/`getOpcua` perform. Python's `opcua()`
      (`python/.../v1_0/file.py:142-187`) and Node's `opcua()` (`nodejs/.../v1_0/file.ts:243-284`)
      are the *only* entry point to an OPCUA handle, and that single method enforces full
      required-field validation before ever returning a settable handle.
  - **Impact:** if an OPCUA block exists but is missing a required field (e.g.
    `Root_Node`), a Rust/C++ caller can still call `set_opcua`/`setOpcua` to repair it; a
    Python/Node caller cannot reach `set_model` at all in that state —`opcua()` returns
    `Err(ValidationError)` first, with no facade path to fix an invalid block. Go's
    situation is a strict superset of this (no `SetOpcua` exists at all).

- [ ] **Error taxonomy incomplete in 2 languages.** Schema
      (`schema/capabilities/errors.yaml:5-19`) defines 7 codes: `UnsupportedVersion`,
      `UnsupportedInVersion`, `NotPresent`, `ValidationError`, `IoError`, `InvalidIndex`,
      `Closed`. Python/Node/Rust all match exactly. Go
      (`go/capabilities/internal/api/api.go:16-23`) has only 6 — `UnsupportedInVersion` is
      entirely missing, confirmed via repo-wide grep that the string never appears in Go
      source. C++ (`cpp/include/machine_config/capabilities/errors.hpp:7-14`) has no
      enum/const set at all — `code` is a bare `std::string`, zero compile-time typo
      protection, and (confirmed by grep) `"UnsupportedInVersion"` is never produced
      anywhere in `cpp/` either, matching Go's gap in practice despite the structurally
      open type.

- [ ] **`Closed` handled inconsistently across languages.** Rust
      (`file.rs:70-76`, `Err(CapabilityError::Closed)`) and Go (`v1_0/file.go:75-80`,
      `api.Errf(api.ErrClosed,...)`) construct and return it as a normal error value.
      Python's and Node's `"Closed"`/`'Closed'` exist only in the type declaration and are
      never constructed anywhere (confirmed by full-repo grep) — `assert_open`/
      `assertOpen` instead `raise SessionClosedError()` / `throw new SessionClosedError()`,
      bypassing the `Result` wrapper entirely. C++ is worse still: `assertOpen()`
      (`file.hpp:283`) throws a bare `std::runtime_error` with no dedicated exception type
      and no structured code at all.
  - **Impact:** Rust/Go callers handle "session closed" through the same `match`/
    `if err != nil` path as every other error; Python/Node callers need a separate
    `try/except`; C++ callers can't even type-match it.

- [ ] **Schema validation only exists in Python and Node.js.** Rust/C++/Go have no
      bundled schema copy and no validation feature at all. Pre-existing, unrelated to
      versioning, but a real equal-capability gap.

- [ ] **Node's float formatting for `power_characterization` params doesn't match the
      other 4 languages' Python-mimicking convention.** Python's `str(c.value)`/
      `str(p.input_value)` (`power_characterization.py:212,232-233`) naturally prints a
      decimal point for whole numbers (`"1.0"`) because that's just how Python renders
      floats. Rust, C++, and Go don't get that for free, so each deliberately wrote a
      small helper to reproduce it — `format_f64_like_python`
      (`rust/src/power_characterization.rs:150`), `pcFormatDoubleForCsv`
      (`cpp/include/machine_config/power_characterization.hpp`), `formatPCFloat`
      (`go/internal/models/power_characterization.go`). Node's `powerCharacterization.ts`
      calls plain `String(value)` (`nodejs/src/powerCharacterization.ts:214,233`)
      instead, which drops the decimal point on whole numbers (`"1"`, not `"1.0"`).
      Confirmed pre-existing, not introduced by Phase 2 clean-up — it matches Node's
      existing float-formatting convention used elsewhere in the codebase, not something
      new to this module; deliberately preserved rather than silently changed when
      `powerCharacterization.ts` was written (`V1_1_IMPLEMENTATION_PLAN.md`'s Node
      Phase 2 write-up).
  - **Impact:** produces a different, though still schema-valid, on-disk string for the
    same in-memory value — no crash, no schema-validation failure (the schema types the
    field as a plain string, no format constraint). The real risk is silent: any future
    byte-level cross-language golden-file comparison of `Watts_To_Volts_Params`/
    `Volts_To_Watts_Params`-derived CSVs (the same pattern `matches_python_golden_file`-
    style tests already use elsewhere) would show a spurious mismatch for whole-number
    values written by Node vs. any other language.
  - **Scope note:** a real fix is *not* a one-file patch to `powerCharacterization.ts` —
    Node's plain `String(value)` is used for every other float field across the
    codebase too, so decimal-forcing only the new v1.1 module would introduce a fresh
    *internal* inconsistency (new fields get `"1.0"`, existing v1.0 fields keep `"1"`)
    that's arguably worse than today's uniform-but-cross-language-inconsistent state.
    Closing this out for real means porting the helper repo-wide across Node's writers,
    touching existing v1.0 float-serialization call sites and their golden-file/
    round-trip tests — bigger and riskier than the discovery context, deliberately not
    undertaken as part of Phase 2 clean-up.

---

## Tier 3 — Documentation/tracking debt

Functional scenario coverage in `VALIDATION_PLAN.md`'s sense is confirmed **equal across
all 5 languages** — this tier is about the tracking documents lagging completed work, not
about a real code gap.

- [ ] **S-10** is implemented and passing in all 5 validation apps
      (`run_s10`/`runS10`/`RunS10FacadeExportSurface`/`s10::run`, confirmed substantive
      and wired into every `main`) but recorded in **none** of the 5 `results.md`/
      `PASS_FAIL.md` files, and absent from `docs/validation/README.md`'s scenario
      matrix. Breaches `VALIDATION_PLAN.md` §2's own completion gate ("All happy-path
      scenarios pass and are recorded for all five languages").
- [ ] **AV-12/AV-13** (from `OPCUA_FIELD_PROMOTION_PLAN.md`, not originally part of
      `VALIDATION_PLAN.md`) exist and pass in all 5 apps but are absent from every
      tracking doc, including `VALIDATION_PLAN.md` itself. `OPCUA_FIELD_PROMOTION_PLAN.md`'s
      own trailing "Documentation pass" checklist has this unchecked.
- [ ] `docs/validation/README.md` (last updated 2026-08-18) is stale: states C++'s
      "static tarball verification still pending" (line 57), but `VALIDATION_PLAN.md`
      §10 Step 5 and §13's C++ checklist both say CI verification and CD artifact
      production are done.
- [ ] `VALIDATION_PLAN.md` §2 is internally inconsistent with its own §10 — the
      Completion Gate line says the C++ artifact-producing step is "still open — see §10
      Step 5," but §10 Step 5's heading now reads "CI verification: done. CD artifact
      production: done." Likely an unedited leftover from before packaging finished.
- [ ] None of the 5 validation apps are wired into CI — already self-documented as an
      existing gap in `VALIDATION_PLAN.md` §13 and Go's own `results.md`. Equal across
      all 5, not a parity issue, just still outstanding.
- [ ] **Confirmed still-open, now actionable**: `DISPATCH_REGISTRY_PLAN.md`'s own
      documented follow-on — upgrading Rust/C++/Go's AV-09–11 mock-adapter tests to route
      through the real registries (`ResolveReader`, `ResolveOpen`, etc.) now that they
      exist, rather than instantiating the mock adapter directly. Verified all three
      still call the mock directly today; the plan itself already says this retrofit is
      "not started here," so this is confirmation, not a new finding.

---

## Confirmed — not gaps

Recorded so these aren't re-audited later.

- **Scanner invert flags** (`invert_actual_x/y`, `invert_commanded_x/y`): plain
  non-optional bool, default `false`, omitted-unless-true, confirmed identical in all 5
  current model files (`SCANNER_INVERT_FLAGS_PLAN.md` had flagged Node/C++/Go as
  unconfirmed — now confirmed correct: `nodejs/src/models.ts:61-64`,
  `cpp/include/machine_config/models.hpp:298-301,793-796,826-829`,
  `go/internal/models/models.go:229-232`).
- **Extra/passthrough fields**: present on exactly the same four types everywhere
  (`MachineConfigMeta`, `OpcuaClientConfig`, `OpcuaPipeConfig`, `OpcuaTrigger`), absent
  from `ClearBox`/`SynchronousSensor` in all 5.
- **Collection empty-vs-absent semantics**: `ClearBox.synchronous_sensors`
  (omit-key-when-empty) and `OpcuaConfig.triggers` (always `{}`) behave identically
  across all 5, including Go's explicit nil-map/nil-slice avoidance
  (`go/capabilities/v1_0/hdf5/hdf5.go:812,593,599`).
- **Go's root-level type aliases** (`go/models.go:17-39`): complete 1:1 match to
  `go/internal/models`, nothing missing. (Minor non-functional note: `internal/models`'s
  standalone `BuildPlate` struct is never wired to `Machine`, which inlines flat fields
  like Rust/C++/TS — harmless dead code, not a parity issue.)
- **CLI tools**: clean, no hardcoded version-specific behavior found in any of the 5.
- **JSON Schema `File_Version` field**: no `enum`/`const` restriction in any of the 3
  schema copies that exist — a future `"1.1"` needs zero schema edits on this front.
- **Facade design-shape difference (handle-based vs. flat)**: Python/Node's
  handle-based facade (`file.opticalTrain(i).getScanner()`) vs. Rust/C++/Go's flat,
  index-based facade (`file.get_scanner(index)`) is a deliberate, pre-existing
  architectural difference, not a bug. Python/Node's handle model reaches full
  method-level parity with Rust/C++ despite the different shape — confirmed both have a
  `set_train`-equivalent (whole-train replace via the train handle's `set_model`) that
  C++ doesn't even have.

---

## Lower-confidence, not independently confirmed

- **`MachineConfigMeta.facility_id`/`config_author` serialization-mechanism
  inconsistency.** Python's `_config_to_dict` and C++'s hand-written `to_json`/`from_json`
  for `MachineConfigMeta` (`models.hpp:1001-1026`) structurally omit these fields
  entirely — they cannot leak no matter what's in memory. Rust
  (`skip_serializing_if="Option::is_none"`) and Go (`omitempty`) instead rely on a
  generic derive/serialization attribute. Safe today only because no real (non-mock,
  non-test) adapter populates them, per `docs/contributing.md`'s "Mock fixtures and real
  version numbers" section — no live trigger confirmed, but worth tightening before a
  real v1.1 reuses these field names for something with different semantics (see
  `docs/contributing.md`'s own collision-resolution steps for exactly this scenario).

---

## Relationship to `V1_1_IMPLEMENTATION_PLAN.md`

Two items are worth keeping in mind *while* doing the v1.1 work, even though this audit
as a whole is deprioritized behind it:

1. The builder construction-option gaps (Tier 2) mean Go/Rust/C++ can't easily build a
   mock config at a specific `File_Version` the way Python/Node can — relevant the moment
   v1.1 test fixtures need in-memory construction rather than reading a committed `.h5`.
2. The `facility_id`/`config_author` serialization-mechanism note (bottom section) is
   exactly the class of risk `docs/contributing.md`'s "mock fixtures and real version
   numbers" section already anticipates for a real version reusing a mock's field names —
   worth a quick check when v1.1's actual new fields (`firmware_version`,
   `power_characterization`) are added, even though this specific note is about different
   field names.

Nothing else here blocks or is blocked by v1.1 — the two plans are independent otherwise.
