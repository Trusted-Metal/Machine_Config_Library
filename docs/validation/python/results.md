# Python Validation Results

**Date:** 2026-08-19 (refreshed — see "Doc/behavior drift found and fixed" below)
**Library version:** machine-config-library 0.2.0-rc.4 (wheel)
**Python:** 3.12.2 (installed via `python -m build python/` → wheel)
**Platform:** Windows 11, Git Bash
**Fixture set:** `fixtures/` + `Reference Materials/` + `docs/validation/fixtures/`
**App location:** `docs/validation/python/app/` — AV-09–11 run in-app (Python's dispatcher is a
registry, `_ADAPTERS`, not a hardcoded match/switch, so there is a real seam to inject the mock
adapter into and exercise it through the public `MachineConfigReader`/`MachineConfigWriter` facade,
unlike Rust/Go/C++). `python/tests/test_adapter_migration.py` additionally carries its own
pytest-native suite (8 tests, run via `pytest python/tests/` in CI) — broader than the app's 3
AV-09–11 scenarios, since it also covers dispatcher-injection and mock-protocol-conformance cases
that only make sense to test directly against the registry, not through the app's scenario shape.

---

## Doc/behavior drift found and fixed (2026-08-19)

This file was dated 2026-08-14 and recorded a 17-scenario run (before AV-09–11 existed in the app
at all). A cross-language audit — re-running all five languages' apps fresh and diffing against
their recorded `results.md` output — found Python was the **only** language where live behavior no
longer matched what was recorded; Rust, Go, Node.js, and C++ all matched their docs byte-for-byte.

Root cause, confirmed via `git log -S'or "1.0"' -- python/src/machine_config/capabilities/v1_0/hdf5.py`:
commit `7ef1c36` (2026-08-17) made two deliberate correctness improvements to
`Hdf5AdapterV1_0._parse()` in the same diff hunk, and this file was never re-run afterward to catch
either one:

1. `file_version=str(f.attrs.get("File_Version", ""))` became
   `file_version=str(f.attrs.get("File_Version", "")).strip() or "1.0"` — `meta.file_version` now
   defaults to `"1.0"` when the attribute is absent/empty, consistent with `peek_file_version`'s
   dispatch decision (previously the two disagreed: dispatch said `"1.0"`, the model said `""`).
2. `Hdf5AdapterV1_0.parse()` gained a `try/except KeyError as exc: raise MissingRequiredGroup(str(exc))`
   wrapper — the bare `KeyError` AV-04 used to raise is now a typed `MissingRequiredGroup(KeyError)`
   (it still subclasses `KeyError`, so existing `except KeyError` consumer code keeps working
   unchanged, but a consumer can now also catch the more specific type).

Both are real, positive changes — not regressions — but neither was reflected below until now.
**Practical implication for future validation passes:** treat `results.md` as generated output that
must be re-run whenever the adapter it describes changes, not hand-maintained prose kept in sync by
memory alongside code.

---

## Run output (verbatim, full 20-scenario in-repo app run)

```
[PASS] S-01: all scalar fields match expected values
[PASS] S-02: shapes OK, finite values OK, forward≠inverse, correction_data SHA-256=b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6
[PASS] S-03: machine_name='TM-LPBF-02: AconityMIDI+_OG' | file_version='1.0' | trains=2 | build_plate_x=250.0 | build_plate_y=250.0 | wd=670.0 | rotation[0]=0.0 | hash=9bc38c92c582a154...
[PASS] S-04: machine_name persisted, file_version and other fields unchanged
[PASS] S-05: correction_data preserved: SHA-256=b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6
[PASS] S-06: 2-laser build OK, center≈2.0, roundtrip OK
[PASS] S-07: OPCUA roundtrip OK: 2 triggers, url='opc.tcp://172.17.20.240:62541/TM_OPCUA_DevTemplate_V0.1/TelemetryServer'
[PASS] S-08: 3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK
[PASS] S-09: all public types importable from machine_config top-level
[PASS] AV-01: UnsupportedFileVersion raised, version='2.0'
[PASS] AV-02: missing File_Version defaults to '1.0', reads OK, file_version='1.0'
[PASS] AV-03: UnsupportedFileVersion raised, version='1.1'
[PASS] AV-04: MissingRequiredGroup raised for missing Machine/ group: 'Required HDF5 group missing: "Unable to synchronously open object (object \'Machine\' doesn\'t exist)"'
[PASS] AV-05: ValueError raised for corrupt scalar: could not convert string to float: 'not_a_number'
[PASS] AV-06: whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK
[PASS] AV-07: empty File_Version defaults to '1.0', reads OK, file_version='1.0'
[PASS] AV-08: File_Version survives roundtrip unchanged: '1.0'
[PASS] AV-09: v1.0 adapter unaffected by mock v1.1 adapter import; all fields correct
[PASS] AV-10: forward migration OK — ADDITION=None, REMOVAL=None, machine_name='TM-LPBF-02: AconityMIDI+_OG', build_plate x=250.0 y=250.0 z=20.0 preserved
[PASS] AV-11: backward migration OK — ADDITION fields lost (facility_id=None, config_author=None), machine_name='BackwardMigrationTest' preserved, file_version='1.0'

20 scenarios: 20 passed, 0 failed
```

## Run output (verbatim, `pytest python/tests/test_adapter_migration.py -v`)

```
python\tests\test_adapter_migration.py::test_v1_1_read PASSED            [ 12%]
python\tests\test_adapter_migration.py::test_v1_1_roundtrip PASSED       [ 25%]
python\tests\test_adapter_migration.py::test_v1_to_v1_1 PASSED           [ 37%]
python\tests\test_adapter_migration.py::test_v1_1_to_v1 PASSED           [ 50%]
python\tests\test_adapter_migration.py::test_v1_unaffected PASSED        [ 62%]
python\tests\test_adapter_migration.py::test_dispatcher_v1_to_v1_1 PASSED [ 75%]
python\tests\test_adapter_migration.py::test_dispatcher_v1_1_to_v1 PASSED [ 87%]
python\tests\test_adapter_migration.py::test_mock_adapters_satisfy_protocol PASSED [100%]

8 passed in 0.84s
```

---

## Reference hashes (cross-language comparison)

| Field | SHA-256 |
|---|---|
| `correction_data` train 0 (reference_config.h5) | `b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6` |

Matches Rust's, Node.js's, and C++'s recorded hash for the same fixture/train exactly.

---

## Behavioral notes for cross-language comparison

**AV-02 / AV-07 (missing / empty File_Version) — Python now diverges from the other four
languages.** `peek_file_version` defaults to `"1.0"` for dispatch, and (as of commit `7ef1c36`,
2026-08-17) `meta.file_version` is now **also** `"1.0"` when the attribute is absent or empty — the
two were made internally consistent. Rust, Node.js, Go, and C++ all still return the raw, empty
`""` for the model's `file_version` field in this case (verified fresh against all four on
2026-08-19). Neither behavior is "wrong" per VALIDATION_PLAN.md's rule for these two scenarios
("either outcome is acceptable... behavior must simply be documented and consistent") — but the
"consistent across languages" half of that rule is currently **not** met, and that's worth a
deliberate decision (bring the other four in line with Python's `"1.0"`, or revert Python to `""`)
rather than leaving it as an unnoticed four-against-one split.

**AV-04 error type:** as of the same commit, a typed `MissingRequiredGroup(KeyError)` — no longer a
bare `KeyError`. It still subclasses `KeyError`, so existing `except KeyError` consumer code is
unaffected, but consumers can now catch the more specific type. Rust is still the only language
with a from-scratch typed error here (`MachineConfigError::MissingGroup`); Node.js and C++ remain
on generic `Error`/`runtime_error`. Python moved from that generic-error group into a middle
ground (typed, but still catchable as the generic ancestor) that's worth naming explicitly rather
than leaving grouped with Node/C++'s "no typed error at all" bucket.
