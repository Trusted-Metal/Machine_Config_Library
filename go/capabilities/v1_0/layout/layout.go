package layout

import "fmt"

const (
	FileVersion                   = "1.0"
	RootMachine                   = "Machine"
	RootOpticalTrains             = "Machine/Optical_Trains"
	RootOPCUA                     = "OPCUA"
	TrainIDPrefix                 = "Optical_Train_"
	GroupScanner                  = "Scanner"
	GroupLightSource              = "Light_Source"
	GroupCollimator               = "Collimator"
	GroupScannerCard              = "Scanner_Card"
	GroupOptionalComponents       = "Optional_Components"
	GroupClearBox                 = "ClearBox"
	DSCorrectionData              = "Correction_Data"
	DSInverseCorrectionData       = "Inverse_Correction_Data"
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

func ClearboxPath(index int) string {
	return TrainPath(index) + "/" + GroupOptionalComponents + "/" + GroupClearBox
}

func ClearboxPathByID(tid string) string {
	return TrainPathByID(tid) + "/" + GroupOptionalComponents + "/" + GroupClearBox
}
