# Go — PASS/FAIL Summary

**Date:** 2026-08-18
**Version:** machine-config-go 0.2.0-rc.4
**Result:** 17 / 17 PASS (app) + 5 / 5 PASS (AV-09–11, `go/` test tree)

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
| AV-04 | Reader returns an error when required group is absent | PASS |
| AV-05 | Reader handles corrupt required attribute gracefully | PASS |
| AV-06 | Dispatcher normalizes whitespace in File_Version | PASS |
| AV-07 | Dispatcher handles empty string File_Version | PASS |
| AV-08 | File_Version string survives write→read unchanged | PASS |
| AV-09 | Mock v1.1 adapter leaves v1.0 adapter untouched | PASS |
| AV-10 | Forward migration v1.0 → v1.1-mock, all change categories | PASS |
| AV-11 | Backward migration v1.1-mock → v1.0, ADDITION fields lost | PASS |

AV-09–11 run via `go test .` (`go/mock_v1_1_test.go`) in `go/` (not the standalone app — see
`results.md` for why, mirroring Rust's/Node's precedent). All other scenarios run via
`docs/validation/go/app/`.

**Open, not yet resolved:** `BuildPlate` (`go/internal/models/models.go`) is exported and
aliased at the module root but never constructed anywhere in the real codebase — flagged in
S-09 and `results.md`, deliberately left undecided pending a separate decision.

**Not yet done, consistent with all four other languages:** none of Python/Node/Rust/C++/Go
have their validation app wired into CI yet (`rust.yml`/`cpp.yml`/`nodejs.yml`/`python.yml`
have no references to their respective `docs/validation/*/app/`) — this is an existing,
consistent gap across the whole project, not something specific to Go.
