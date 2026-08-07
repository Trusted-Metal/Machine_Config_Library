# Go — Machine Config Library

**Phase 5 — not yet implemented.** See [IMPLEMENTATION_PLAN.md §Phase 5](../IMPLEMENTATION_PLAN.md) for the full plan.

← [Back to index](../USAGE.md)

---

## Confirmed decisions

All pre-implementation decisions have been made:

| Decision | Choice | Rationale |
|---|---|---|
| **HDF5 binding** | `github.com/scigolib/hdf5` (CGo) | Native HDF5 C library access via CGo; full float64/NaN support confirmed in proto tests |
| **CI — Ubuntu** | `apt-get install -y libhdf5-dev` (HDF5 1.10.x) | Sufficient for scigolib/hdf5; same pattern as C++ |
| **CI — Windows** | MinGW-w64 GCC + vcpkg HDF5 (deferred to follow-on PR) | Reuses vcpkg cache from `cpp.yml` |
| **Module path** | `machine-config-go` | Standalone module inside `go/` |
| **CLI binary name** | `machine-config-go` | Avoids collision with Rust's `machine-config-cli` |
| **JSON Schema** | `github.com/santhosh-tekuri/jsonschema/v6` | Pure-Go, draft 2020-12 support |
| **CLI framework** | `github.com/spf13/cobra` v1.8+ | Standard Go CLI idiom |
| **cross_check.yml** | `"go"` added to `RUNNERS`, `WRITERS`, `COPIERS`, `BINARIES` | Added incrementally as each phase completes |

---

## Important: scigolib/hdf5 prototype validation (do before writing any library code)

`scigolib/hdf5` had write-correctness bugs in versions before v0.14 (NaN not preserved in float64 dataset writes). Before implementing any library code, run the four proto programs in `go/proto/` to confirm the pinned version handles all required operations correctly:

| Proto | What it tests |
|---|---|
| `proto1_read_root_attrs` | Open `reference_config.h5`; read string root attributes |
| `proto2_read_3d_nan_dataset` | Read `Correction_Data` dataset; verify shape [257,257,2] and NaN present |
| `proto3_write_3d_nan_roundtrip` | **Critical**: write 3D float64 with NaN; close; reopen; verify NaN preserved |
| `proto4_attribute_types` | Write and read string, int64, float64 attributes; verify type fidelity |

Run each with `go run .` inside its directory. All four must exit 0 before proceeding. See §5.0 in IMPLEMENTATION_PLAN.md for the full programs.

---

## Planned contents (populated once implemented)

Once implemented, this document will cover:

- Installation (`go get machine-config-go`)
- CGo build requirements (libhdf5-dev on Ubuntu, MinGW+vcpkg on Windows)
- Use case 1 — Parse a machine config file
- Use case 2 — Export to canonical JSON
- Use case 4 — Write a config back to HDF5
- Use case 6 — Generate a synthetic test config (`MockConfigBuilder`)
- Use case 8 — Read OPCUA telemetry configuration
- Use case 9 — Access ClearBox correction arrays (`GetCorrectionData`)
- Use case 10 — Validate a config against the schema
- CLI reference (`machine-config-go --help`)
- Quickstart example (`examples/quickstart/go/main.go`)
- Running the Go test suite (`cd go && go test ./...`)
