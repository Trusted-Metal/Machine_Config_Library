package machineconfig

import (
	v10hdf5 "machine-config-go/capabilities/v1_0/hdf5"
)

// MachineConfigReader reads LPBF machine-config HDF5 files into MachineConfig.
type MachineConfigReader struct {
	path string
}

// NewReader constructs a reader for path.
func NewReader(path string) *MachineConfigReader {
	return &MachineConfigReader{path: path}
}

// ParseOptions controls binary dataset loading.
type ParseOptions struct {
	IncludeBinary bool
}

// Parse opens the file and returns a populated MachineConfig (scalars + nested groups).
// Binary correction grids / fc3 bytes are omitted unless IncludeBinary is set via ParseWithOptions.
func (r *MachineConfigReader) Parse() (*MachineConfig, error) {
	return r.ParseWithOptions(ParseOptions{})
}

// ParseWithOptions peeks File_Version and dispatches to that version's adapter.
func (r *MachineConfigReader) ParseWithOptions(opts ParseOptions) (*MachineConfig, error) {
	fv, err := PeekFileVersion(r.path)
	if err != nil {
		return nil, err
	}
	switch fv {
	case "1.0":
		return v10hdf5.Parse(r.path, opts.IncludeBinary)
	default:
		return nil, &UnsupportedFileVersionError{Version: fv}
	}
}

// GetCorrectionData loads the ClearBox Correction_Data grid for train index.
func (r *MachineConfigReader) GetCorrectionData(trainIndex int) (*CorrectionData, error) {
	fv, err := PeekFileVersion(r.path)
	if err != nil {
		return nil, err
	}
	switch fv {
	case "1.0":
		return v10hdf5.GetCorrectionData(r.path, trainIndex)
	default:
		return nil, &UnsupportedFileVersionError{Version: fv}
	}
}
