package capabilities

import (
	"os"
	"path/filepath"

	machineconfig "machine-config-go"
)

// File is the File_Version 1.0 stable model facade (in-memory session).
type File struct {
	config      *machineconfig.MachineConfig
	path        string
	fileVersion string
	closed      bool
}

// OpenMachineConfig opens path and returns a v1.0 facade session.
func OpenMachineConfig(path string) (*File, *Error) {
	cfg, err := machineconfig.NewReader(path).ParseWithOptions(machineconfig.ParseOptions{IncludeBinary: true})
	if err != nil {
		return nil, errf(ErrIo, err.Error())
	}
	fv := cfg.Meta.FileVersion
	if fv == "" {
		fv = "1.0"
	}
	if fv != "1.0" {
		return nil, errf(ErrUnsupportedVersion, "No adapter for File_Version \""+fv+"\"")
	}
	return &File{config: cfg, path: path, fileVersion: fv}, nil
}

// CreateMachineConfig builds a minimal in-memory v1.0 config (1 train).
// Full MockConfigBuilder parity lands with §5.10; this skeleton is enough for facade tests.
func CreateMachineConfig(version string) (*File, *Error) {
	if version == "" {
		version = "1.0"
	}
	if version != "1.0" {
		return nil, errf(ErrUnsupportedVersion, "create() unsupported for File_Version \""+version+"\"")
	}
	cfg := &machineconfig.MachineConfig{
		Meta: machineconfig.MachineConfigMeta{
			SchemaVersion:     "v1",
			MachineName:       "MockMachine",
			Manufacturer:      "MockCo",
			Model:             "MockMIDI+",
			SerialNumber:      "MOCK-001",
			FileVersion:       "1.0",
			ExportDate:        "2026-01-01T00:00:00.000Z",
			ConfigurationHash: "0000000000000000000000000000000000000000000000000000000000000000",
			Extra:             map[string]any{},
		},
		Machine: machineconfig.Machine{
			MachineName:  "MockMachine",
			Manufacturer: "MockCo",
			Model:        "MockMIDI+",
			SerialNumber: "MOCK-001",
			BuildPlateX:  machineconfig.Float64Ptr(250),
			BuildPlateY:  machineconfig.Float64Ptr(250),
			BuildPlateZ:  machineconfig.Float64Ptr(20),
		},
		OpticalTrains: []machineconfig.OpticalTrain{{
			TrainID: "Optical_Train_01",
			Scanner: machineconfig.Scanner{
				Manufacturer:    "MockScanner",
				Model:           "Mock",
				SerialNumber:    "S-1",
				WorkingDistance: machineconfig.Float64Ptr(670),
			},
			LightSource: machineconfig.LightSource{Manufacturer: "MockLS", Model: "L", SerialNumber: "L-1"},
			Collimator:  machineconfig.Collimator{Manufacturer: "MockCol", Model: "C", SerialNumber: "C-1"},
			ScannerCard: machineconfig.ScannerCard{Manufacturer: "MockCard", Model: "SP-ICE-3", SerialNumber: "SC-1"},
		}},
	}
	return &File{config: cfg, fileVersion: "1.0"}, nil
}

// SupportedFileVersions lists registered adapter keys.
func SupportedFileVersions() []string { return []string{"1.0"} }

func (f *File) assertOpen() *Error {
	if f.closed {
		return errf(ErrClosed, "MachineConfigFile session is closed")
	}
	return nil
}

func (f *File) FileVersion() string { return f.fileVersion }

func (f *File) OpticalTrainCount() (int, *Error) {
	if err := f.assertOpen(); err != nil {
		return 0, err
	}
	return len(f.config.OpticalTrains), nil
}

func (f *File) GetMeta() (machineconfig.MachineConfigMeta, *Error) {
	if err := f.assertOpen(); err != nil {
		return machineconfig.MachineConfigMeta{}, err
	}
	return snapshot(f.config.Meta), nil
}

func (f *File) SetMeta(model machineconfig.MachineConfigMeta, mode SetMode) *Error {
	if err := f.assertOpen(); err != nil {
		return err
	}
	merged, err := applySetMode(f.config.Meta, model, mode)
	if err != nil {
		return errf(ErrValidation, err.Error())
	}
	f.config.Meta = merged
	return nil
}

func (f *File) GetScanner(index int) (machineconfig.Scanner, *Error) {
	if err := f.assertOpen(); err != nil {
		return machineconfig.Scanner{}, err
	}
	if index < 0 || index >= len(f.config.OpticalTrains) {
		return machineconfig.Scanner{}, errf(ErrInvalidIndex, "optical train index out of range")
	}
	return snapshot(f.config.OpticalTrains[index].Scanner), nil
}

func (f *File) SetScanner(index int, model machineconfig.Scanner, mode SetMode) *Error {
	if err := f.assertOpen(); err != nil {
		return err
	}
	if index < 0 || index >= len(f.config.OpticalTrains) {
		return errf(ErrInvalidIndex, "optical train index out of range")
	}
	merged, err := applySetMode(f.config.OpticalTrains[index].Scanner, model, mode)
	if err != nil {
		return errf(ErrValidation, err.Error())
	}
	f.config.OpticalTrains[index].Scanner = merged
	return nil
}

func (f *File) GetOpcua() (machineconfig.OpcuaConfig, *Error) {
	if err := f.assertOpen(); err != nil {
		return machineconfig.OpcuaConfig{}, err
	}
	if f.config.Opcua == nil {
		return machineconfig.OpcuaConfig{}, errf(ErrNotPresent, "OPCUA group is not present")
	}
	return snapshot(*f.config.Opcua), nil
}

func (f *File) GetClearbox(index int) (machineconfig.ClearBox, *Error) {
	if err := f.assertOpen(); err != nil {
		return machineconfig.ClearBox{}, err
	}
	if index < 0 || index >= len(f.config.OpticalTrains) {
		return machineconfig.ClearBox{}, errf(ErrInvalidIndex, "optical train index out of range")
	}
	cb := f.config.OpticalTrains[index].OptionalComponents.Clearbox
	if cb == nil {
		return machineconfig.ClearBox{}, errf(ErrNotPresent, "clearbox is not present")
	}
	return snapshot(*cb), nil
}

// Save writes the session. Path is required for create()-d files.
// NOTE: full MachineConfigWriter lands in §5.7; Save currently returns ErrValidation
// until the writer is implemented. Merge/set APIs are still usable in-memory.
func (f *File) Save(path string) *Error {
	if err := f.assertOpen(); err != nil {
		return err
	}
	out := path
	if out == "" {
		out = f.path
	}
	if out == "" {
		return errf(ErrValidation, "save() requires a path for create()-d files")
	}
	_ = filepath.Clean(out)
	_ = os.DevNull
	return errf(ErrValidation, "Go MachineConfigWriter not implemented yet (§5.7); in-memory set* works")
}

func (f *File) Close() { f.closed = true }
