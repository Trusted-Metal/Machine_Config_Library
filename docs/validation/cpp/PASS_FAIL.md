# C++ — PASS/FAIL Summary

**Date:** 2026-08-18
**Version:** machine_config 0.1.0
**Result:** 20 / 20 PASS

| ID | Title | Result |
|---|---|---|
| S-01 | Read reference fixture, all scalar fields | PASS |
| S-02 | Read correction grids, shape and finite values | PASS |
| S-03 | Read real AconityMIDI fixture | PASS |
| S-04 | Write modified config and verify field change survives roundtrip | PASS |
| S-05 | Full binary roundtrip with correction hash verification | PASS |
| S-06 | Build synthetic config with MockConfigBuilder and verify fields | PASS |
| S-07 | OPCUA config roundtrip | PASS |
| S-08 | Drastic field change to real file, verify adapter pipeline | PASS |
| S-09 | Public type export surface | PASS |
| AV-01 | Reader rejects unknown File_Version | PASS |
| AV-02 | Reader handles absent File_Version predictably | PASS |
| AV-03 | v1.0 reader encountering a v1.1 file fails loudly | PASS |
| AV-04 | Reader returns an error when required group is absent | PASS |
| AV-05 | Reader handles corrupt required attribute gracefully | PASS |
| AV-06 | Dispatcher normalizes whitespace in File_Version | PASS |
| AV-07 | Dispatcher handles empty string File_Version | PASS |
| AV-08 | File_Version string survives write→read unchanged | PASS |
| AV-09 | Mock v1.1 adapter leaves v1.0 adapter untouched | PASS |
| AV-10 | Forward migration v1.0 → v1.1-mock, all change categories | PASS |
| AV-11 | Backward migration v1.1-mock → v1.0, ADDITION fields lost | PASS |

AV-09–11 run via Catch2 (`cpp/tests/test_adapter_migration.cpp`) in `cpp/` (not the standalone
app — see `results.md` for why: this library's dispatcher has no registry to inject a mock
into). All other scenarios run via `docs/validation/cpp/app/`.
