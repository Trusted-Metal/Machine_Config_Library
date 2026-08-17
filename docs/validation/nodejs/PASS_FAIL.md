# Node.js — PASS/FAIL Summary

**Date:** 2026-08-17
**Version:** machine-config-library 0.2.0-rc.4
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
| AV-01 | Reader rejects unknown File_Version with typed error | PASS |
| AV-02 | Reader handles absent File_Version predictably | PASS |
| AV-03 | v1.0 reader encountering a v1.1 file fails loudly | PASS |
| AV-04 | Reader returns typed error when required group is absent | PASS |
| AV-05 | Reader handles corrupt required attribute gracefully | PASS (fixed during this run — see `results.md`) |
| AV-06 | Dispatcher normalizes whitespace in File_Version | PASS |
| AV-07 | Dispatcher handles empty string File_Version | PASS |
| AV-08 | File_Version string survives write→read unchanged | PASS |
| AV-09 | Mock v1.1 adapter leaves v1.0 adapter untouched | PASS |
| AV-10 | Forward migration v1.0 → v1.1-mock, all change categories | PASS |
| AV-11 | Backward migration v1.1-mock → v1.0, ADDITION fields lost | PASS |
