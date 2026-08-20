package v1_0

import (
	"strings"

	machineconfig "machine-config-go"
	"machine-config-go/capabilities/internal/api"
	v1_0hdf5 "machine-config-go/capabilities/v1_0/hdf5"
	"machine-config-go/capabilities/v1_0/layout"
)

const FileVersion = layout.FileVersion

// File is the File_Version 1.0 stable model facade.
type File struct {
	config      *machineconfig.MachineConfig
	path        string
	fileVersion string
	closed      bool
}

func Open(path string) (*File, *api.Error) {
	cfg, err := v1_0hdf5.Parse(path, true)
	if err != nil {
		return nil, api.Errf(api.ErrIo, err.Error())
	}
	return &File{config: cfg, path: path, fileVersion: FileVersion}, nil
}

func Create(version string) (*File, *api.Error) {
	if version == "" {
		version = FileVersion
	}
	if version != FileVersion {
		return nil, api.Errf(api.ErrUnsupportedVersion, "create() unsupported for File_Version \""+version+"\"")
	}
	cfg := &machineconfig.MachineConfig{
		Meta: machineconfig.MachineConfigMeta{
			SchemaVersion:     "v1",
			MachineName:       "MockMachine",
			Manufacturer:      "MockCo",
			Model:             "MockMIDI+",
			SerialNumber:      "MOCK-001",
			FileVersion:       FileVersion,
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
			TrainID: layout.TrainID(0),
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
	return &File{config: cfg, fileVersion: FileVersion}, nil
}

func (f *File) assertOpen() *api.Error {
	if f.closed {
		return api.Errf(api.ErrClosed, "MachineConfigFile session is closed")
	}
	return nil
}

func (f *File) FileVersion() string { return f.fileVersion }

func (f *File) OpticalTrainCount() (int, *api.Error) {
	if err := f.assertOpen(); err != nil {
		return 0, err
	}
	return len(f.config.OpticalTrains), nil
}

func (f *File) GetMeta() (machineconfig.MachineConfigMeta, *api.Error) {
	if err := f.assertOpen(); err != nil {
		return machineconfig.MachineConfigMeta{}, err
	}
	return api.Snapshot(f.config.Meta), nil
}

func (f *File) SetMeta(model machineconfig.MachineConfigMeta, mode api.SetMode) *api.Error {
	if err := f.assertOpen(); err != nil {
		return err
	}
	merged, err := api.ApplySetMode(f.config.Meta, model, mode)
	if err != nil {
		return api.Errf(api.ErrValidation, err.Error())
	}
	f.config.Meta = merged
	return nil
}

func (f *File) GetScanner(index int) (machineconfig.Scanner, *api.Error) {
	if err := f.assertOpen(); err != nil {
		return machineconfig.Scanner{}, err
	}
	if index < 0 || index >= len(f.config.OpticalTrains) {
		return machineconfig.Scanner{}, api.Errf(api.ErrInvalidIndex, "optical train index out of range")
	}
	return api.Snapshot(f.config.OpticalTrains[index].Scanner), nil
}

func (f *File) SetScanner(index int, model machineconfig.Scanner, mode api.SetMode) *api.Error {
	if err := f.assertOpen(); err != nil {
		return err
	}
	if index < 0 || index >= len(f.config.OpticalTrains) {
		return api.Errf(api.ErrInvalidIndex, "optical train index out of range")
	}
	merged, err := api.ApplySetMode(f.config.OpticalTrains[index].Scanner, model, mode)
	if err != nil {
		return api.Errf(api.ErrValidation, err.Error())
	}
	f.config.OpticalTrains[index].Scanner = merged
	return nil
}

// GetOpcua returns the OPCUA config, or an ErrValidation error if OPCUA is
// present but missing one or more required fields. Collects every missing
// field at once (in Details) rather than failing on the first one — see
// OPCUA_FIELD_PROMOTION_PLAN.md's "Why facade-only enforcement". The
// low-level reader/writer stay fully permissive; this is the one place
// "required" is enforced.
func (f *File) GetOpcua() (machineconfig.OpcuaConfig, *api.Error) {
	if err := f.assertOpen(); err != nil {
		return machineconfig.OpcuaConfig{}, err
	}
	if f.config.Opcua == nil {
		return machineconfig.OpcuaConfig{}, api.Errf(api.ErrNotPresent, "OPCUA group is not present")
	}
	opcua := f.config.Opcua

	var missing []string
	if opcua.Client.MachineProfile == nil {
		missing = append(missing, "Machine_Profile")
	}
	if opcua.Client.RootNode == nil {
		missing = append(missing, "Root_Node")
	}
	if opcua.Pipe.ConfigureClient == nil {
		missing = append(missing, "Configure_Client")
	}
	if opcua.Pipe.PipeName == nil {
		missing = append(missing, "Pipe_Name")
	}
	if opcua.TriggersEnabled == nil {
		missing = append(missing, "Triggers_Enabled")
	}
	if opcua.TriggerStopCeilingLayers == nil {
		missing = append(missing, "Trigger_Stop_Ceiling_Layers")
	}
	for name, trigger := range opcua.Triggers {
		if trigger.Event == nil {
			missing = append(missing, name+".Event")
		}
	}

	if len(missing) > 0 {
		msg := "OPCUA is present but missing required field(s): " + strings.Join(missing, ", ")
		return machineconfig.OpcuaConfig{}, api.Errf(api.ErrValidation, msg, missing...)
	}

	return api.Snapshot(*opcua), nil
}

func (f *File) GetClearbox(index int) (machineconfig.ClearBox, *api.Error) {
	if err := f.assertOpen(); err != nil {
		return machineconfig.ClearBox{}, err
	}
	if index < 0 || index >= len(f.config.OpticalTrains) {
		return machineconfig.ClearBox{}, api.Errf(api.ErrInvalidIndex, "optical train index out of range")
	}
	cb := f.config.OpticalTrains[index].OptionalComponents.Clearbox
	if cb == nil {
		return machineconfig.ClearBox{}, api.Errf(api.ErrNotPresent, "clearbox is not present")
	}
	return api.Snapshot(*cb), nil
}

func (f *File) Save(path string) *api.Error {
	if err := f.assertOpen(); err != nil {
		return err
	}
	out := path
	if out == "" {
		out = f.path
	}
	if out == "" {
		return api.Errf(api.ErrValidation, "save() requires a path for create()-d files")
	}
	if err := v1_0hdf5.Write(f.config, out); err != nil {
		return api.Errf(api.ErrIo, err.Error())
	}
	f.path = out
	return nil
}

func (f *File) Close() { f.closed = true }
