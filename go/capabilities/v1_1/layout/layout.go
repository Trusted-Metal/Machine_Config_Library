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

	// RootExtensions holds every v1.1 optional, cross-cutting section: every extension is an opaque
	// tree, ClearBox included. Present only if at least one exists — never written as an empty
	// placeholder group.
	RootExtensions = "Extensions"

	TrainIDPrefix = "Optical_Train_"
	GroupScanner  = "Scanner"
	// Note: no GroupOptionalComponents / GroupClearBox under a train in
	// v1.1 — Optional_Components is removed entirely (Change 1); ClearBox
	// lives under Extensions/ClearBox as an ordinary extension tree.
	GroupLightSource = "Light_Source"
	GroupCollimator  = "Collimator"
	GroupScannerCard = "Scanner_Card"

	// GroupPowerCharacterization is Changes 3/4's structured replacement for
	// Volts_To_Watts_*/Watts_To_Volts_*, under a train's Light_Source group.
	GroupPowerCharacterization    = "Power_Characterization"
	DSDerivationEquationConstants = "Derivation_Equation_Constants"
	DSCharacterizationPoints      = "Characterization_Points"
	DSScanFieldCorrectionFile     = "scan_field_correction_file"
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
