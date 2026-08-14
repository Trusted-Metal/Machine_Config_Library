package machineconfig

import (
	"strings"

	v1_0hdf5 "machine-config-go/capabilities/v1_0/hdf5"
)

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
	switch fv {
	case "1.0":
		return v1_0hdf5.Write(cfg, path)
	default:
		return &UnsupportedFileVersionError{Version: fv}
	}
}
