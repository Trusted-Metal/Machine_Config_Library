package scenarios

// S-04: Write modified config and verify field change survives roundtrip
//
// ID:       S-04
// Action:   Read reference fixture, change machine_name, write to temp, re-read.
// Expected: machine_name change persists.

import (
	"fmt"
	"os"
	"path/filepath"

	mc "machine-config-go"
)

func RunS04WriteModify(fixturesDir, _ string) (bool, string) {
	path := filepath.Join(fixturesDir, "reference_config.h5")
	cfg, err := mc.NewReader(path).Parse()
	if err != nil {
		return false, fmt.Sprintf("read failed: %v", err)
	}

	cfg.Meta.MachineName = "MODIFIED_VALIDATION"
	cfg.Machine.MachineName = "MODIFIED_VALIDATION"

	tmpPath, err := tempH5("mcl_go_s04")
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
	if rb.Meta.MachineName != "MODIFIED_VALIDATION" {
		return false, fmt.Sprintf("meta.machine_name after roundtrip: %q", rb.Meta.MachineName)
	}
	if rb.Machine.MachineName != "MODIFIED_VALIDATION" {
		return false, fmt.Sprintf("machine.machine_name after roundtrip: %q", rb.Machine.MachineName)
	}
	return true, "machine_name modified and survives roundtrip"
}
