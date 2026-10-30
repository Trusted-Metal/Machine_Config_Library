// Layout constants and path helpers for the mock v1.1 on-disk shape.
//
// Duplicated verbatim from machine-config-go/capabilities/v1_0/layout
// (byte-for-byte identical except this file adds GroupDimensions, the one
// new group the mock's PATH/NAME+PATH changes introduce) rather than
// imported — per the architectural rule that no version's adapter, mock or
// real, may import another version's adapter code. See the package doc in
// mockv1_1.go for the full rationale.
package mockv1_1

import "fmt"

const (
	RootMachine             = "Machine"
	RootOpticalTrains       = "Machine/Optical_Trains"
	RootOPCUA               = "OPCUA"
	TrainIDPrefix           = "Optical_Train_"
	GroupScanner            = "Scanner"
	GroupLightSource        = "Light_Source"
	GroupCollimator         = "Collimator"
	GroupScannerCard        = "Scanner_Card"
	GroupOptionalComponents = "Optional_Components"
	GroupClearBox           = "ClearBox"
	// GroupDimensions is new in mock v1.1: Machine/Dimensions/ (changes 7-10
	// in docs/migrations/mock_v1_0_to_v1_1.md). No v1.0 equivalent.
	GroupDimensions           = "Dimensions"
	DSCorrectionData          = "Correction_Data"
	DSInverseCorrectionData   = "Inverse_Correction_Data"
	DSScanFieldCorrectionFile = "scan_field_correction_file"
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

func ClearboxPath(index int) string {
	return TrainPath(index) + "/" + GroupOptionalComponents + "/" + GroupClearBox
}

func ClearboxPathByID(tid string) string {
	return TrainPathByID(tid) + "/" + GroupOptionalComponents + "/" + GroupClearBox
}
