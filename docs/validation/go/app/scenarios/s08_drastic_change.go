package scenarios

// S-08: Drastic field change to real file, verify adapter pipeline integrity
//
// ID:           S-08
// Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
// Action:       1. Add a third optical train (clone train 1, change train_id)
//               2. Change build_plate_x from 250.0 to 350.0
//               3. Set scan_head_rotation on new train to 90.0
//               4. Clear all correction data on the new train (clearbox = nil)
//               5. Change machine_name to "MODIFIED_ACONITY_VALIDATION"
// Expected:     All five changes persist after write -> read; file_version
//               unchanged at "1.0".

import (
	"fmt"
	"math"
	"os"
	"strings"

	mc "machine-config-go"
)

func RunS08DrasticChange(_ string, realDir string) (bool, string) {
	path, err := findRealFixture(realDir)
	if err != nil {
		return false, err.Error()
	}
	cfg, err := mc.NewReader(path).Parse()
	if err != nil {
		return false, fmt.Sprintf("read failed: %v", err)
	}

	cfg.Meta.MachineName = "MODIFIED_ACONITY_VALIDATION"
	cfg.Machine.MachineName = "MODIFIED_ACONITY_VALIDATION"
	cfg.Machine.BuildPlateX = mc.Float64Ptr(350.0)

	newTrain := cfg.OpticalTrains[1]
	newTrain.TrainID = "Optical_Train_03"
	newTrain.Scanner.ScanHeadRotation = mc.Float64Ptr(90.0)
	newTrain.OptionalComponents.Clearbox = nil
	cfg.OpticalTrains = append(cfg.OpticalTrains, newTrain)

	tmpPath, err := tempH5("mcl_go_s08")
	if err != nil {
		return false, fmt.Sprintf("tempfile: %v", err)
	}
	defer os.Remove(tmpPath)

	if err := mc.NewWriter().Write(cfg, tmpPath); err != nil {
		return false, fmt.Sprintf("write failed: %v", err)
	}
	rb, err := mc.NewReader(tmpPath).Parse()
	if err != nil {
		return false, fmt.Sprintf("readback failed: %v", err)
	}

	if len(rb.OpticalTrains) != 3 {
		return false, fmt.Sprintf("optical_trains: expected 3, got %d", len(rb.OpticalTrains))
	}
	if rb.Machine.BuildPlateX == nil || math.Abs(*rb.Machine.BuildPlateX-350.0) > 0.001 {
		return false, fmt.Sprintf("build_plate_x: got %s", fmtF64Ptr(rb.Machine.BuildPlateX))
	}
	rot := rb.OpticalTrains[2].Scanner.ScanHeadRotation
	if rot == nil || math.Abs(*rot-90.0) > 0.001 {
		return false, fmt.Sprintf("train[2].scan_head_rotation: got %s", fmtF64Ptr(rot))
	}
	if rb.OpticalTrains[2].OptionalComponents.Clearbox != nil {
		return false, "train[2].clearbox should be nil"
	}
	if rb.Meta.MachineName != "MODIFIED_ACONITY_VALIDATION" {
		return false, fmt.Sprintf("machine_name: got %q", rb.Meta.MachineName)
	}
	if strings.TrimSpace(rb.Meta.FileVersion) != "1.0" {
		return false, fmt.Sprintf("file_version changed: %q", rb.Meta.FileVersion)
	}

	return true, "3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK"
}
