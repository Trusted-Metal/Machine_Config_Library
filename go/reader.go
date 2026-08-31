package machineconfig

import (
	v1_0hdf5 "machine-config-go/capabilities/v1_0/hdf5"
)

// ReaderAdapter is the version-agnostic reader surface — mirrors the three
// operations v1_0hdf5's free functions provide. A thin wrapper struct per
// version implements this by calling those functions with the path it was
// constructed with.
type ReaderAdapter interface {
	Parse(opts ParseOptions) (*MachineConfig, error)
	GetCorrectionData(trainIndex int) (*CorrectionData, error)
	GetInverseCorrectionData(trainIndex int) (*CorrectionData, error)
}

type v1_0ReaderAdapter struct {
	path string
}

func (a v1_0ReaderAdapter) Parse(opts ParseOptions) (*MachineConfig, error) {
	return v1_0hdf5.Parse(a.path, opts.IncludeBinary)
}

func (a v1_0ReaderAdapter) GetCorrectionData(trainIndex int) (*CorrectionData, error) {
	return v1_0hdf5.GetCorrectionData(a.path, trainIndex)
}

func (a v1_0ReaderAdapter) GetInverseCorrectionData(trainIndex int) (*CorrectionData, error) {
	return v1_0hdf5.GetInverseCorrectionData(a.path, trainIndex)
}

// ReaderRegistry maps a File_Version string to the constructor for that
// version's ReaderAdapter. A real registry (DISPATCH_REGISTRY_PLAN.md):
// adding a version means adding an entry here, never editing
// MachineConfigReader itself. This is also what collapses what used to be
// three independent switch statements (one per method below, each
// re-implementing the same version dispatch) into one shared lookup.
type ReaderRegistry map[string]func(path string) ReaderAdapter

var productionReaderRegistry = ReaderRegistry{
	"1.0": func(path string) ReaderAdapter { return v1_0ReaderAdapter{path: path} },
}

// ResolveReader is registry-parameterized so tests can inject a fake entry
// without touching global state — see DISPATCH_REGISTRY_PLAN.md's shared
// testing pattern. Not for application use; MachineConfigReader is the real
// entry point.
func ResolveReader(version string, path string, registry ReaderRegistry) (ReaderAdapter, error) {
	ctor, ok := registry[version]
	if !ok {
		return nil, &UnsupportedFileVersionError{Version: version}
	}
	return ctor(path), nil
}

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
	adapter, err := ResolveReader(fv, r.path, productionReaderRegistry)
	if err != nil {
		return nil, err
	}
	return adapter.Parse(opts)
}

// GetCorrectionData loads the ClearBox Correction_Data grid for train index.
func (r *MachineConfigReader) GetCorrectionData(trainIndex int) (*CorrectionData, error) {
	fv, err := PeekFileVersion(r.path)
	if err != nil {
		return nil, err
	}
	adapter, err := ResolveReader(fv, r.path, productionReaderRegistry)
	if err != nil {
		return nil, err
	}
	return adapter.GetCorrectionData(trainIndex)
}

// GetInverseCorrectionData loads the ClearBox Inverse_Correction_Data grid for train index.
func (r *MachineConfigReader) GetInverseCorrectionData(trainIndex int) (*CorrectionData, error) {
	fv, err := PeekFileVersion(r.path)
	if err != nil {
		return nil, err
	}
	adapter, err := ResolveReader(fv, r.path, productionReaderRegistry)
	if err != nil {
		return nil, err
	}
	return adapter.GetInverseCorrectionData(trainIndex)
}
