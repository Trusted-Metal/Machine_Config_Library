# Validation Master Summary

**Branch:** SD-1684
**Library version:** machine-config-library 0.2.0-rc.4
**Last updated:** 2026-08-18

One row per scenario. See each language's `PASS_FAIL.md` for the full verdict list
and `results.md` for verbatim output.

---

## Summary

| Language | Scenarios | Passed | Status |
|---|---|---|---|
| Python | 20 | 20 | ✅ Complete |
| Node.js | 20 | 20 | ✅ Complete |
| Rust | 20 | 20 | ✅ Complete |
| Go | 20 | — | ⬜ Not started |
| C++ | 20 | 20 | ✅ Complete |

---

## Scenario Matrix

| ID | Title | Python | Node.js | Rust | Go | C++ |
|---|---|---|---|---|---|---|
| S-01 | Read reference fixture, all scalar fields | ✅ | ✅ | ✅ | ⬜ | ✅ |
| S-02 | Read correction grids, shape and finite values | ✅ | ✅ | ✅ | ⬜ | ✅ |
| S-03 | Read real AconityMIDI fixture | ✅ | ✅ | ✅ | ⬜ | ✅ |
| S-04 | Write modified config, verify roundtrip | ✅ | ✅ | ✅ | ⬜ | ✅ |
| S-05 | Full binary roundtrip with correction hash | ✅ | ✅ | ✅ | ⬜ | ✅ |
| S-06 | Build synthetic config with MockConfigBuilder | ✅ | ✅ | ✅ | ⬜ | ✅ |
| S-07 | OPCUA config roundtrip | ✅ | ✅ | ✅ | ⬜ | ✅ |
| S-08 | Drastic field change to real file | ✅ | ✅ | ✅ | ⬜ | ✅ |
| S-09 | Public type export surface | ✅ | ✅ | ✅ | ⬜ | ✅ |
| AV-01 | Reader rejects unknown File_Version | ✅ | ✅ | ✅ | ⬜ | ✅ |
| AV-02 | Reader handles absent File_Version | ✅ | ✅ | ✅ | ⬜ | ✅ |
| AV-03 | v1.0 reader rejects v1.1 file | ✅ | ✅ | ✅ | ⬜ | ✅ |
| AV-04 | Typed error for missing required group | ✅ | ✅ | ✅ | ⬜ | ✅ |
| AV-05 | Typed error for corrupt scalar field | ✅ | ✅ | ✅ | ⬜ | ✅ |
| AV-06 | Dispatcher normalizes whitespace in version | ✅ | ✅ | ✅ | ⬜ | ✅ |
| AV-07 | Dispatcher handles empty string version | ✅ | ✅ | ✅ | ⬜ | ✅ |
| AV-08 | File_Version survives write→read unchanged | ✅ | ✅ | ✅ | ⬜ | ✅ |
| AV-09 | Mock v1.1 adapter leaves v1.0 adapter untouched | ✅ | ✅ | ✅ | ⬜ | ✅ |
| AV-10 | Forward migration v1.0 → v1.1, all change categories | ✅ | ✅ | ✅ | ⬜ | ✅ |
| AV-11 | Backward migration v1.1 → v1.0, ADDITION fields lost | ✅ | ✅ | ✅ | ⬜ | ✅ |

---

## Per-language detail

- [Python PASS_FAIL](python/PASS_FAIL.md) · [results](python/results.md)
- [Node.js PASS_FAIL](nodejs/PASS_FAIL.md) · [results](nodejs/results.md)
- [Rust PASS_FAIL](rust/PASS_FAIL.md) · [results](rust/results.md)
- Go — not started
- [C++ PASS_FAIL](cpp/PASS_FAIL.md) · [results](cpp/results.md) (static tarball verification still pending — see merge gate below)

---

## Merge gate

SD-1684 is ready to merge when all cells in the matrix show ✅ and the C++ static
tarball is verified. See [VALIDATION_PLAN.md](../../VALIDATION_PLAN.md) §2 for the
full gate criteria.
