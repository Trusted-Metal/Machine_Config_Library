package machineconfig

import (
	"strings"

	v1_0hdf5 "machine-config-go/capabilities/v1_0/hdf5"
	v1_1hdf5 "machine-config-go/capabilities/v1_1/hdf5"
)

// WriterAdapter is the version-agnostic writer surface — mirrors
// v1_0hdf5.Write's shape.
type WriterAdapter interface {
	Write(cfg *MachineConfig, path string) error
}

type v1_0WriterAdapter struct{}

func (v1_0WriterAdapter) Write(cfg *MachineConfig, path string) error {
	return v1_0hdf5.Write(cfg, path)
}

type v1_1WriterAdapter struct{}

func (v1_1WriterAdapter) Write(cfg *MachineConfig, path string) error {
	return v1_1hdf5.Write(cfg, path)
}

// WriterRegistry maps a File_Version string to that version's WriterAdapter.
// A real registry (DISPATCH_REGISTRY_PLAN.md): adding a version means adding
// an entry here, never editing MachineConfigWriter itself.
type WriterRegistry map[string]WriterAdapter

var productionWriterRegistry = WriterRegistry{
	"1.0": v1_0WriterAdapter{},
	"1.1": v1_1WriterAdapter{},
}

// ResolveWriter is registry-parameterized so tests can inject a fake entry
// without touching global state — see DISPATCH_REGISTRY_PLAN.md's shared
// testing pattern. Not for application use; MachineConfigWriter is the real
// entry point.
func ResolveWriter(version string, registry WriterRegistry) (WriterAdapter, error) {
	adapter, ok := registry[version]
	if !ok {
		return nil, &UnsupportedFileVersionError{Version: version}
	}
	return adapter, nil
}

// MachineConfigWriter serialises a MachineConfig to an HDF5 file.
type MachineConfigWriter struct{}

// NewWriter returns a MachineConfigWriter.
func NewWriter() *MachineConfigWriter { return &MachineConfigWriter{} }

// Write serialises cfg to path, creating or overwriting the file.
// Dispatches by cfg.Meta.FileVersion; defaults to "1.0" when empty.
func (w *MachineConfigWriter) Write(cfg *MachineConfig, path string) error {
	fv := strings.TrimSpace(cfg.Meta.FileVersion)
	if fv == "" {
		fv = "1.0"
	}
	return w.writeResolved(cfg, path, fv)
}

// WriteAs is like Write, but writes as targetVersion regardless of
// cfg.Meta.FileVersion — lets a caller upgrade/downgrade without mutating
// the model just to express intent (e.g. reading a v1.0 file and writing it
// as v1.1 no longer requires setting cfg.Meta.FileVersion = "1.1" first).
// Never mutates cfg itself; only the on-disk File_Version changes.
func (w *MachineConfigWriter) WriteAs(cfg *MachineConfig, path string, targetVersion string) error {
	fv := strings.TrimSpace(targetVersion)
	if fv == "" {
		fv = "1.0"
	}
	return w.writeResolved(cfg, path, fv)
}

// writeResolved always builds a copy of cfg (never mutates the caller's
// value): sets Meta.FileVersion to fv, then computes and stamps
// Configuration_Hash fresh from that copy's content — a caller-supplied
// value is never trusted or passed through, since anything else goes stale
// the instant any other field changes. Computed once, up front; no
// re-read, no patch, no fallback-on-error (see hash.go).
func (w *MachineConfigWriter) writeResolved(cfg *MachineConfig, path string, fv string) error {
	adapter, err := ResolveWriter(fv, productionWriterRegistry)
	if err != nil {
		return err
	}
	corrected := *cfg
	corrected.Meta.FileVersion = fv
	hash, err := ComputeConfigurationHash(&corrected)
	if err != nil {
		return err
	}
	corrected.Meta.ConfigurationHash = hash
	return adapter.Write(&corrected, path)
}
