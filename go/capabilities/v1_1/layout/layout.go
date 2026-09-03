// Package layout defines File_Version 1.1's on-disk HDF5 layout — group
// paths, train-id formatting, and dataset names.
//
// Deliberately independent of capabilities/v1_0/layout: nothing here
// imports from or refers to v1.0, so v1.0 can change or be removed later
// without affecting v1.1 (see go/version_adapter_isolation_test.go, which
// enforces this statically).
package layout

import "fmt"

const (
	FileVersion = "1.1"

	RootMachine       = "Machine"
	RootOpticalTrains = "Machine/Optical_Trains"

	// RootExtensions holds every v1.1 optional, cross-cutting section
	// (ClearBox, TM_OPCUA). Present only if at least one of them exists —
	// never written as an empty placeholder group.
	RootExtensions = "Extensions"
	// GroupClearBoxRoot is Change 1's relocated ClearBox home: one shared
	// Output_Path/Software_Trigger_Delay (Consolidate) plus one <train_id>
	// subgroup per train that has a ClearBox.
	GroupClearBoxRoot = "Extensions/ClearBox"
	// RootOPCUA is Change 2's relocated OPCUA home (was root-level "OPCUA").
	RootOPCUA    = "Extensions/TM_OPCUA"
	OpcuaClient  = "Extensions/TM_OPCUA/Client"
	OpcuaPipe    = "Extensions/TM_OPCUA/Pipe"
	OpcuaTrigger = "Extensions/TM_OPCUA/Triggers"

	TrainIDPrefix = "Optical_Train_"
	GroupScanner  = "Scanner"
	// Note: no GroupOptionalComponents / GroupClearBox under a train in
	// v1.1 — Optional_Components is removed entirely (Change 1); ClearBox
	// lives under RootExtensions/GroupClearBoxRoot instead.
	GroupLightSource = "Light_Source"
	GroupCollimator  = "Collimator"
	GroupScannerCard = "Scanner_Card"

	// GroupPowerCharacterization is Changes 3/4's structured replacement for
	// Volts_To_Watts_*/Watts_To_Volts_*, used identically under both a
	// per-train ClearBox subgroup and a train's Light_Source group.
	GroupPowerCharacterization     = "Power_Characterization"
	DSDerivationEquationConstants  = "Derivation_Equation_Constants"
	DSCharacterizationPoints       = "Characterization_Points"
	DSCorrectionData               = "Correction_Data"
	DSInverseCorrectionData        = "Inverse_Correction_Data"
	DSScanFieldCorrectionFile      = "scan_field_correction_file"
)

func TrainID(index int) string {
	return fmt.Sprintf("%s%02d", TrainIDPrefix, index+1)
}

func TrainPath(index int) string {
	return RootOpticalTrains + "/" + TrainID(index)
}

func TrainPathByID(tid string) string {
	return RootOpticalTrains + "/" + tid
}

func ScannerPathByID(tid string) string {
	return TrainPathByID(tid) + "/" + GroupScanner
}

func LightSourcePathByID(tid string) string {
	return TrainPathByID(tid) + "/" + GroupLightSource
}

func LightSourcePowerCharacterizationPathByID(tid string) string {
	return LightSourcePathByID(tid) + "/" + GroupPowerCharacterization
}

func CollimatorPathByID(tid string) string {
	return TrainPathByID(tid) + "/" + GroupCollimator
}

func ScannerCardPathByID(tid string) string {
	return TrainPathByID(tid) + "/" + GroupScannerCard
}

// ClearboxPath is the index-based variant, used by the top-level
// GetCorrectionData/GetInverseCorrectionData free functions (mirrors
// v1_0/layout's ClearboxPath).
func ClearboxPath(index int) string {
	return GroupClearBoxRoot + "/" + TrainID(index)
}

func ClearboxPathByID(tid string) string {
	return GroupClearBoxRoot + "/" + tid
}

func ClearboxPowerCharacterizationPathByID(tid string) string {
	return ClearboxPathByID(tid) + "/" + GroupPowerCharacterization
}
