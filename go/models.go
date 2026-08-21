package machineconfig

import "machine-config-go/internal/models"

// StrPtr returns a pointer to s.
func StrPtr(s string) *string { return &s }

// Float64Ptr returns a pointer to f.
func Float64Ptr(f float64) *float64 { return &f }

// IntPtr returns a pointer to i.
func IntPtr(i int) *int { return &i }

// BoolPtr returns a pointer to b.
func BoolPtr(b bool) *bool { return &b }

type (
	CorrectionData          = models.CorrectionData
	MachineConfig           = models.MachineConfig
	MachineConfigMeta       = models.MachineConfigMeta
	BuildPlate              = models.BuildPlate
	Machine                 = models.Machine
	OpticalTrain            = models.OpticalTrain
	Scanner                 = models.Scanner
	AxisConfig              = models.AxisConfig
	LightSource             = models.LightSource
	Collimator              = models.Collimator
	ScannerCard             = models.ScannerCard
	OptionalComponents      = models.OptionalComponents
	ClearBox                = models.ClearBox
	SynchronousSensor       = models.SynchronousSensor
	EquationConstant        = models.EquationConstant
	CalibrationPoint        = models.CalibrationPoint
	ScanFieldCorrectionFile = models.ScanFieldCorrectionFile
	OpcuaConfig             = models.OpcuaConfig
	OpcuaClientConfig       = models.OpcuaClientConfig
	OpcuaPipeConfig         = models.OpcuaPipeConfig
	OpcuaTrigger            = models.OpcuaTrigger
)
