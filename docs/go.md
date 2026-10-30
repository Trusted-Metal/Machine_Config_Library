# Go — Machine Config Library

**Status: in progress (Phase 5).** Binding decision updated after proto gate:

| Topic | Choice |
|---|---|
| **HDF5** | **CGo → system `libhdf5`** via [`go/internal/h5c`](../go/internal/h5c) |
| **Why not scigolib** | Pure-Go `scigolib/hdf5` cannot read **dense attribute storage** (>8 attrs) as written by h5py / production exporters. Proto5 audit: 16/22 groups unreadable. |
| **CI** | Ubuntu `libhdf5-dev` ([`.github/workflows/go.yml`](../.github/workflows/go.yml)); Windows deferred (§5.14) |
| **Module** | `machine-config-go` in `go/` |
| **Facade** | `capabilities` package — open/get/set with `SetMode` (save awaits writer §5.7) |

← [Back to index](../USAGE.md)

---

## Proto gate

```bash
# CGo dense-attr proof (required)
docker run --rm -v "${PWD}:/work" -w /work/go golang:1.24-bookworm \
  bash -c 'apt-get update -qq && apt-get install -y -qq libhdf5-dev pkg-config >/dev/null && \
           CGO_ENABLED=1 go run ./proto/proto_cgo_read_root_attrs/'
```

Historical `go/proto/proto1_*` … `proto5_*` programs use scigolib and are tagged `//go:build ignore` (kept as diagnostics).

---

## Package layout

```
go/
├── models.go              ← MachineConfig tree
├── reader.go              ← MachineConfigReader (CGo)
├── reader_helpers.go
├── capabilities/          ← dispatch + shared SetMode/errors
│   └── v1_0/              ← File_Version 1.0 facade
│       └── layout/        ← on-disk paths (own package; avoids import cycle)
├── internal/h5c/          ← thin libhdf5 wrapper
├── proto/proto_cgo_*      ← CGo gate
└── .github/workflows/go.yml
```

---

## Reader quickstart

```go
import machineconfig "machine-config-go"

cfg, err := machineconfig.NewReader("machine.h5").Parse()
if err != nil { log.Fatal(err) }
fmt.Println(cfg.Meta.MachineName)
fmt.Println(*cfg.OpticalTrains[0].Scanner.WorkingDistance)
```

Facade:

```go
import "machine-config-go/capabilities"

f, err := capabilities.OpenMachineConfig("machine.h5")
sc, _ := f.GetScanner(0)
sc.WorkingDistance = machineconfig.Float64Ptr(680)
_ = f.SetScanner(0, sc, capabilities.Merge)
```

---

## Running the Go test suite

```bash
# From the go/ directory (Linux — requires libhdf5-dev)
CGO_ENABLED=1 go test ./... -count=1

# From the repo root
cd go && CGO_ENABLED=1 go test ./... -count=1
```

```powershell
# PowerShell (Windows — requires MSYS2 MinGW64 HDF5; see go.yml for full env setup)
Push-Location go
$env:CGO_ENABLED = '1'
go test ./... -count=1
Pop-Location
```

111 tests across two packages: `machine-config-go` (root — reader, writer, builder, mock
migration) and `machine-config-go/capabilities` (capability facade and dispatch).

---

## Validation results

20 / 20 scenarios pass on this SDK. See the full table in
[docs/validation/go/PASS_FAIL.md](../docs/validation/go/PASS_FAIL.md) and verbatim output
in [docs/validation/go/results.md](../docs/validation/go/results.md).

Two production code additions were made during the validation pass:
- `go/internal/h5c/h5c.go` gained `OpenRW` (open existing file read/write without
  truncating) and `(g *Group) DeleteAttr` — needed by the mock v1.1 writer's
  delegate-then-patch design for AV-09–11.
- `go/internal/models/models.go`'s `MachineConfigMeta` gained `FacilityID`/`ConfigAuthor`
  pointer fields, explicitly marked test-fixture-only, mirroring Python and Rust.

No runtime or serialization defects were found. One open item was deliberately deferred:
`BuildPlate` is exported and aliased at the module root but never constructed in the real
codebase — flagged during S-09, decision on removal vs retention left for a later pass.

For the master cross-language matrix see [docs/validation/README.md](../docs/validation/README.md).

```bash
docker run --rm -v "${PWD}:/work" -w /work/go golang:1.24-bookworm \
  bash -c 'apt-get update -qq && apt-get install -y -qq libhdf5-dev pkg-config >/dev/null && \
           CGO_ENABLED=1 go test ./... -count=1'
```

Read-only quickstart (dummy 2-train file):

```bash
docker run --rm -v "${PWD}:/work" -w /work/examples/quickstart/go golang:1.24-bookworm \
  bash -c 'apt-get update -qq && apt-get install -y -qq libhdf5-dev pkg-config >/dev/null && \
           CGO_ENABLED=1 go run .'
```

---

## Still TODO (Phase 5)

- §5.7 Writer + `write-hdf5` CLI (unblocks facade `Save`)
- §5.8–5.9 `copy-hdf5` / `correction-hash`
- §5.10 MockConfigBuilder
- §5.11 Schema validation
- §5.12 Quickstart binary — read-only example at `examples/quickstart/go` (save awaits writer)
- §5.13 cross_check integration
- §5.14 Windows CI
