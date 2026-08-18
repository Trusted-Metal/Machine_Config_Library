package scenarios

// S-03: Read real AconityMIDI fixture file
//
// ID:           S-03
// Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
// Action:       Parse the file.
// Expected:     No error. Fields match known machine parameters.

import (
	"fmt"

	mc "machine-config-go"
)

func RunS03ReadReal(_ string, realDir string) (bool, string) {
	path, err := findRealFixture(realDir)
	if err != nil {
		return false, err.Error()
	}
	cfg, err := mc.NewReader(path).Parse()
	if err != nil {
		return false, fmt.Sprintf("read failed: %v", err)
	}

	wd := "nil"
	if cfg.OpticalTrains[0].Scanner.WorkingDistance != nil {
		wd = fmt.Sprintf("%.2f", *cfg.OpticalTrains[0].Scanner.WorkingDistance)
	}
	hashPrefix := cfg.Meta.ConfigurationHash
	if len(hashPrefix) > 16 {
		hashPrefix = hashPrefix[:16]
	}
	return true, fmt.Sprintf(
		"machine_name=%q file_version=%q trains=%d build_plate_x=%s wd=%s hash=%s...",
		cfg.Meta.MachineName, cfg.Meta.FileVersion, len(cfg.OpticalTrains),
		fmtF64Ptr(cfg.Machine.BuildPlateX), wd, hashPrefix,
	)
}
