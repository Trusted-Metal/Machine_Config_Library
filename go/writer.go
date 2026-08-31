package machineconfig

import (
	"strings"

	v1_0hdf5 "machine-config-go/capabilities/v1_0/hdf5"
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

// WriterRegistry maps a File_Version string to that version's WriterAdapter.
// A real registry (DISPATCH_REGISTRY_PLAN.md): adding a version means adding
// an entry here, never editing MachineConfigWriter itself.
type WriterRegistry map[string]WriterAdapter

var productionWriterRegistry = WriterRegistry{
	"1.0": v1_0WriterAdapter{},
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
	adapter, err := ResolveWriter(fv, productionWriterRegistry)
	if err != nil {
		return err
	}
	return adapter.Write(cfg, path)
}
