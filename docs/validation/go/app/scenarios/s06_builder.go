package scenarios

// S-06: Build synthetic config with MockConfigBuilder and verify fields
//
// ID:       S-06
// Action:   Build a 2-laser config -> verify fields -> save to temp -> re-read.
// Expected: 2 trains, rotations 0/180, machine_name non-empty,
//           correction_data centre cell ~2.0 (Gaussian peak), roundtrip OK.

import (
	"fmt"
	"math"
	"os"

	mc "machine-config-go"
)

func RunS06Builder(_, _ string) (bool, string) {
	cfg := mc.NewMockConfigBuilder().Build()

	if len(cfg.OpticalTrains) != 2 {
		return false, fmt.Sprintf("optical_trains count: %d", len(cfg.OpticalTrains))
	}

	r0 := cfg.OpticalTrains[0].Scanner.ScanHeadRotation
	if r0 == nil || math.Abs(*r0) > 0.001 {
		return false, fmt.Sprintf("train[0].scan_head_rotation: %s", fmtF64Ptr(r0))
	}
	r1 := cfg.OpticalTrains[1].Scanner.ScanHeadRotation
	if r1 == nil || math.Abs(*r1-180.0) > 0.001 {
		return false, fmt.Sprintf("train[1].scan_head_rotation: %s", fmtF64Ptr(r1))
	}
	if cfg.Meta.MachineName == "" {
		return false, "machine_name is empty"
	}

	cb := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	if cb == nil || cb.CorrectionData == nil {
		return false, "clearbox or correction_data is nil"
	}
	grid := *cb.CorrectionData
	center := grid[128][128][0]
	if center == nil || math.Abs(*center-2.0) > 0.01 {
		return false, fmt.Sprintf("correction_data center: expected ~2.0, got %s", fmtF64Ptr(center))
	}

	tmpPath, err := tempH5("mcl_go_s06")
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
	if len(rb.OpticalTrains) != 2 {
		return false, fmt.Sprintf("readback trains: %d", len(rb.OpticalTrains))
	}
	if rb.Meta.MachineName != cfg.Meta.MachineName {
		return false, "machine_name changed after roundtrip"
	}

	return true, "2-laser build OK, center~2.0, roundtrip OK"
}
