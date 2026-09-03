# Node.js Validation Results

**Date:** 2026-08-17
**Library version:** machine-config-library 0.2.0-rc.4 (installed via `file:` dependency on the built package)
**Node:** v22.22.0
**Platform:** Windows 11, Git Bash
**Fixture set:** `fixtures/` + `Reference Materials/`
**App location:** `docs/validation/nodejs/app/` (ported in-repo; S-01–09/AV-01–08 were first
developed and verified externally at `machineconfiglibrarytesting/node`, per
`VALIDATION_PLAN.md`'s "External validation project location" convention. AV-09–11
were written directly in-repo, since they import the mock adapter from
`nodejs/tests/mockV1_1.ts` at a fixed relative offset — that only works when the
scenario file and the mock live in the same repo checkout, mirroring exactly how
Python's AV-09–11 scenarios reach `python/tests/test_adapter_migration.py`.)

---

## Run output (verbatim, full 20-scenario in-repo run)

```
[PASS] S-01: all scalar fields match expected values
[PASS] S-02: shapes OK, finite values OK, forward≠inverse, correction_data SHA-256=b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6
[PASS] S-03: machine_name="TM-LPBF-02: AconityMIDI+_OG" | file_version="1.0" | trains=2 | build_plate_x=250 | build_plate_y=250 | wd=670 | rotation[0]=0 | hash=9bc38c92c582a154...
[PASS] S-04: machine_name persisted, file_version and other fields unchanged
[PASS] S-05: correction_data preserved: SHA-256=b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6
[PASS] S-06: 2-laser build OK, center≈2.0, roundtrip OK
[PASS] S-07: OPCUA roundtrip OK: 2 triggers, url='opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer'
[PASS] S-08: 3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK
[PASS] S-09: all public types importable from machine-config-library top-level
[PASS] AV-01: UnsupportedFileVersion raised, version='2.0'
[PASS] AV-02: missing File_Version defaults to '1.0', reads OK, file_version=''
[PASS] AV-03: UnsupportedFileVersion raised, version='1.1'
[PASS] AV-04: Error raised for missing Machine/ group: Error: HDF5: missing group at "Machine"
[PASS] AV-05: TypeError raised for corrupt scalar: TypeError: Attribute "Build_Plate_X_Dimension": cannot parse "not_a_number" as a number
[PASS] AV-06: whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK
[PASS] AV-07: empty File_Version defaults to '1.0', reads OK, file_version=''
[PASS] AV-08: File_Version survives roundtrip unchanged: '1.0'
[PASS] AV-09: v1.0 adapter unaffected by mock v1.1 adapter import; all fields correct
[PASS] AV-10: forward migration OK — ADDITION=null, REMOVAL=null, machine_name='TM-LPBF-02: AconityMIDI+_OG', build_plate x=250 y=250 z=20 preserved
[PASS] AV-11: backward migration OK — ADDITION fields lost (facility_id=null, config_author=null), machine_name='BackwardMigrationTest' preserved, file_version='1.0'

20 scenarios: 20 passed, 0 failed
```

---

## Reference hashes (cross-language comparison)

| Field | SHA-256 |
|---|---|
| `correction_data` train 0 (reference_config.h5) | `b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6` |

Matches Python's recorded hash for the same fixture/train exactly — confirms byte-identical correction-grid parity between the two languages.

---

## Mock v1.1 adapter — design and coverage (AV-09–11)

`nodejs/tests/mockV1_1.ts` implements `MockV1_1Reader`/`MockV1_1Writer` using a
**delegate-then-patch** design, not the reassemble-from-parts approach Python's
mock uses: the reader calls the real, public `Hdf5AdapterV1_0.parse()` on the
whole file (every subcomponent this mock doesn't change comes back correct;
the ~8 fields that changed come back null/empty, never a throw) and then
patches exactly the 10 documented differences by reading their real, mock-v1.1
locations directly. The writer calls the real, public `Hdf5WriterV1_0.write()`
to produce a fully valid v1.0-shaped file, then reopens it read-write and
applies the same 10 edits in place (`delete_attribute`, `create_group`,
`create_attribute`).

This was a deliberate choice over reimplementing or reaching into v1.0's
private parsing internals — it means light_source/collimator/scanner_card/
clearbox/sfcf/opcua are never re-tested here (they're already covered by the
143 tests exercising the real v1.0 adapter), and it can't drift from the real
v1.0 behavior for those fields, since it's calling that exact code, not a
second copy of it. The design was verified against h5wasm's actual runtime
behavior (a standalone spike script, not just its type definitions) before
being adopted — `delete_attribute`/`create_group` on an already-written,
reopened file both work exactly as needed.

**A real bug was found and fixed while building this:** the mock reader's
first version set `meta.facility_id`/`config_author` as typed fields but left
them *also* present in `meta.extra`, because the base v1.0 parser's
`KNOWN_ROOT` set doesn't know these are typed fields and swept them into
`extra` as unknown attributes. Writing that config then wrote `Facility_ID`
twice — once from the typed field logic, once from the `extra` loop — colliding
with an HDF5 "attribute already exists" error (visible in the diagnostic
output even though the tests still passed on the resulting, coincidentally
correct, data). Fixed by stripping the two keys out of `extra` before setting
the typed fields. This is exactly the serialization-safety failure mode
flagged for every language earlier in this validation effort — confirmed here
as a real, not just theoretical, risk.

`nodejs/tests/adapterMigration.test.ts` has 8 Vitest tests covering this mock,
mirroring Python's `test_adapter_migration.py` exactly: adapter-level read,
roundtrip, forward migration, backward migration, v1.0-path-unaffected, plus
two dispatcher-level tests proving the *public* `MachineConfigReader`/`Writer`
route correctly to the mock via temporarily-injected `_READERS`/`_WRITERS`
entries (see "Dispatch injection" note below), and a structural-compatibility
check.

**Dispatch injection:** `reader.ts`/`writer.ts`'s `READERS`/`WRITERS` dispatch
tables were originally unexported. They're now exported as `_READERS`/`_WRITERS`
(underscore-prefixed, matching Python's `_ADAPTERS` convention) solely so tests
can temporarily register a mock adapter — mirroring `pytest`'s `monkeypatch.setitem`,
except Vitest has no automatic revert for plain object mutation, so cleanup is
explicit via `afterEach`.

**Version identifier:** the mock uses `"1.1-mock"` from the very first line,
not `"1.1"` — avoiding the mistake-then-fix cycle Python went through (see
`docs/contributing.md`'s "Mock fixtures and real version numbers" section).

**Visual inspection:** `nodejs/scratch/inspectMigration.mts` (gitignored,
run via `npx tsx` — Node's native TypeScript support can't handle the
constructor-parameter-property shorthand used throughout the real adapter
classes) produces real, inspectable `.h5` files for both directions, mirroring
Python's `scratch/inspect_migration.py`. Run and visually confirmed all 10
changes behave correctly in both directions before this result was recorded.

---

## Bugs found and fixed during this validation run

**AV-05 initially FAILED** — first run reported `[FAIL] AV-05: no error raised for corrupt Build_Plate_X_Dimension`.

- **Root cause:** `attrFloat()` in `nodejs/src/capabilities/v1_0/hdf5.ts` (then lines 138-144) parsed numeric attributes with `Number(v)` and returned `null` whenever that produced `NaN` — silently swallowing a corrupt/non-numeric value instead of raising. Python's reader raises `ValueError` for the identical input (`Build_Plate_X_Dimension = "not_a_number"`); AV-05's own rationale requires failing loudly here, unlike AV-02/AV-07 where either behavior is documented as acceptable.
- **Fix:** `attrFloat()` now throws a `TypeError` naming the attribute when the raw value is present and non-empty but fails to parse as a number, while still returning `null` for genuinely absent/empty attributes (preserving AV-02/AV-07 behavior).
- **Verified:** full Node.js suite (151 tests, including the later AV-09-11 additions) still green after the fix; this validation app confirms AV-05 passing with the new, informative `TypeError`.

**The `meta.extra` double-write bug described above**, found while building the AV-09–11 mock adapter.

## Behavioral notes for cross-language comparison

**AV-02 / AV-07 (missing / empty File_Version):** identical behavior and identical
resulting value to Python — `file_version` is the empty string `""` in both
languages when the attribute is absent or empty, while dispatch still succeeds
against the v1.0 adapter. Full parity with Python's recorded behavior.

**AV-04 error type:** a generic `Error` thrown by `h5wasm` (`"HDF5: missing group
at \"Machine\""`), not a library-defined typed error — same situation as
Python's `KeyError` from h5py. Consumers must catch a generic `Error` for
missing-group failures in both languages. Same potential API improvement noted
for Python (a typed `MissingRequiredGroup`-equivalent) applies here too — Node.js
currently has no such type at all (Python's exists; see `docs/contributing.md`).

**AV-05 error type:** `TypeError`, matching Python's `ValueError` in spirit (a
built-in type-conversion error, not a library-defined type) — same category as
the AV-04 note above.
