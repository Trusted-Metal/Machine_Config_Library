# Go — Machine Config Library

**Phase 5 — not yet implemented.**

This file is a placeholder. Go support will be added when Phase 5 begins. The library
dependency strategy (CGo bindings vs. pure-Go reader vs. WASM) should be decided before
implementation starts — see [IMPLEMENTATION_PLAN.md](../IMPLEMENTATION_PLAN.md) for context.

← [Back to index](../USAGE.md)

---

## Planned contents

Once implemented, this document will follow the same structure as the other language docs:

- Installation
- Use case 1 — Parse a machine config file
- Use case 2 — Export to canonical JSON
- Use case 4 — Write a config back to HDF5
- Use case 6 — Generate a synthetic test config
- Use case 8 — Read OPCUA telemetry configuration
- Use case 9 — Access ClearBox correction arrays
- Use case 10 — Validate a config against the schema
- CLI reference
- Quickstart example
- Full workflow example
- Running the Go test suite

---

## Pre-implementation decision checklist

Before writing any Go code, decide:

- [ ] **HDF5 binding**: CGo (`gonum/hdf5`) vs. pure-Go partial reader vs. h5wasm via WASM runtime
- [ ] **CI dependency**: if CGo, Ubuntu's apt ships HDF5 1.10 (may be too old); Windows needs vcpkg or pre-built binaries
- [ ] **Module path**: e.g. `github.com/<org>/machine-config-library/go`
- [ ] **CLI binary name**: `machine-config-go` to avoid collision with the Rust `machine-config-cli`
- [ ] **cross_check.yml**: add Go build steps and `go` to `--langs` flag once the CLI binary exists
