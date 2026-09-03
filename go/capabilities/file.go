package capabilities

import (
	v1_0 "machine-config-go/capabilities/v1_0"
	v1_1 "machine-config-go/capabilities/v1_1"
	machineconfig "machine-config-go"
)

// File is the active session for a stable-model MachineConfig file. Every
// version adapter (v1_0.File today; any future v1_1.File, ...) implements
// this — it mirrors v1_0.File's current public surface exactly
// (DISPATCH_REGISTRY_PLAN.md). Note this is narrower than Rust's/C++'s
// completed facade interfaces (missing SetOpcua, SetClearbox,
// GetLightSource/SetLightSource, GetCollimator/SetCollimator,
// GetScannerCard/SetScannerCard, GetMachine/SetMachine, GetTrain/SetTrain) —
// that's a real, separate, pre-existing gap in v1_0.File itself, not
// something to silently backfill here; see the plan for why.
//
// Was previously `type File = v1_0.File`, a type alias directly to the
// concrete struct — that only ever worked because exactly one version
// existed. A type alias can name only one concrete type, so it could not
// survive a second version; this interface is what replaces it.
type File interface {
	FileVersion() string
	OpticalTrainCount() (int, *Error)
	GetMeta() (machineconfig.MachineConfigMeta, *Error)
	SetMeta(model machineconfig.MachineConfigMeta, mode SetMode) *Error
	GetScanner(index int) (machineconfig.Scanner, *Error)
	SetScanner(index int, model machineconfig.Scanner, mode SetMode) *Error
	GetOpcua() (machineconfig.OpcuaConfig, *Error)
	GetClearbox(index int) (machineconfig.ClearBox, *Error)
	GetCorrectionData(index int) (*machineconfig.CorrectionData, *Error)
	GetInverseCorrectionData(index int) (*machineconfig.CorrectionData, *Error)
	Save(path string) *Error
	Close()
}

// OpenRegistry maps a File_Version string to the constructor for that
// version's File. A real registry (DISPATCH_REGISTRY_PLAN.md): adding a
// version means adding an entry here, never editing OpenMachineConfig
// itself.
type OpenRegistry map[string]func(path string) (File, *Error)

var productionOpenRegistry = OpenRegistry{
	"1.0": func(path string) (File, *Error) {
		f, err := v1_0.Open(path)
		if err != nil {
			return nil, err
		}
		return f, nil
	},
	"1.1": func(path string) (File, *Error) {
		f, err := v1_1.Open(path)
		if err != nil {
			return nil, err
		}
		return f, nil
	},
}

// ResolveOpen is registry-parameterized so tests can inject a fake entry
// without touching global state — see DISPATCH_REGISTRY_PLAN.md's shared
// testing pattern. Not for application use; OpenMachineConfig is the real
// entry point.
func ResolveOpen(version string, path string, registry OpenRegistry) (File, *Error) {
	ctor, ok := registry[version]
	if !ok {
		return nil, errf(ErrUnsupportedVersion, "No adapter for File_Version \""+version+"\"")
	}
	return ctor(path)
}

// OpenMachineConfig peeks File_Version and dispatches to that version's adapter.
func OpenMachineConfig(path string) (File, *Error) {
	fv, err := machineconfig.PeekFileVersion(path)
	if err != nil {
		return nil, errf(ErrIo, err.Error())
	}
	return ResolveOpen(fv, path, productionOpenRegistry)
}

// CreateRegistry is [OpenRegistry]'s counterpart for Create().
type CreateRegistry map[string]func(version string) (File, *Error)

var productionCreateRegistry = CreateRegistry{
	"1.0": func(version string) (File, *Error) {
		f, err := v1_0.Create(version)
		if err != nil {
			return nil, err
		}
		return f, nil
	},
	"1.1": func(version string) (File, *Error) {
		f, err := v1_1.Create(version)
		if err != nil {
			return nil, err
		}
		return f, nil
	},
}

// ResolveCreate is [ResolveOpen]'s counterpart for Create().
func ResolveCreate(version string, registry CreateRegistry) (File, *Error) {
	ctor, ok := registry[version]
	if !ok {
		return nil, errf(ErrUnsupportedVersion, "create() unsupported for File_Version \""+version+"\"")
	}
	return ctor(version)
}

// CreateMachineConfig builds a new in-memory session for the given File_Version.
func CreateMachineConfig(version string) (File, *Error) {
	if version == "" {
		version = v1_0.FileVersion
	}
	return ResolveCreate(version, productionCreateRegistry)
}

// SupportedFileVersions lists registered adapter keys. Derived from the
// registry's own keys, not a separately-maintained literal.
func SupportedFileVersions() []string {
	versions := make([]string, 0, len(productionOpenRegistry))
	for v := range productionOpenRegistry {
		versions = append(versions, v)
	}
	return versions
}
