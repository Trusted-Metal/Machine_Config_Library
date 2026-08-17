# Python Validation Results

**Date:** 2026-08-14
**Library version:** machine-config-library 0.2.0-rc.4 (wheel)
**Python:** 3.x (installed via `python -m build python/` → wheel)
**Platform:** Windows 11, Git Bash
**Fixture set:** `fixtures/` + `Reference Materials/`

---

## Run output (verbatim)

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
[PASS] AV-02: missing File_Version defaults to '1.0', reads OK, file_version=''
[PASS] AV-03: UnsupportedFileVersion raised, version='1.1'
[PASS] AV-04: KeyError raised for missing Machine/ group: "Unable to synchronously open object (object 'Machine' doesn't exist)"
[PASS] AV-05: ValueError raised for corrupt scalar: could not convert string to float: 'not_a_number'
[PASS] AV-06: whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK
[PASS] AV-07: empty File_Version defaults to '1.0', reads OK, file_version=''
[PASS] AV-08: File_Version survives roundtrip unchanged: '1.0'

17 scenarios: 17 passed, 0 failed
```

---

## Reference hashes (record on first passing run)

| Field | SHA-256 |
|---|---|
| `correction_data` train 0 (reference_config.h5) | `b3b95bf5d5e73119ad8c632e6f5b44ba3beada6f054e04797afd6f193f2cebc6` |

---

## Behavioral notes for cross-language comparison

**AV-02 (missing File_Version):** `peek_file_version` defaults to `"1.0"` for
dispatch when the attribute is absent, but `meta.file_version` is written from
the raw HDF5 attribute value (which is absent → `""`). Result: dispatch succeeds
and the model's `file_version` field is `""`. Other languages may expose `"1.0"`
or raise an error — record their actual value for comparison.

**AV-07 (empty File_Version):** Same behaviour as AV-02. `peek_file_version`
strips and returns `"1.0"` on empty string; model preserves the raw `""`.

**AV-04 error type:** `KeyError` from h5py (not a library-defined typed error).
Consumers must catch `KeyError` for missing-group failures. If a typed error is
preferred, this is a potential API improvement.
