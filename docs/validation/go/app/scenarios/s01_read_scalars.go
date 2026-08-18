package scenarios

// S-01: Read reference fixture, all scalar fields
//
// ID:       S-01
// Action:   Parse reference_config.h5, check machine_name/file_version/hash.
// Expected: No error; file_version == "1.0"; configuration_hash is 64 chars.

import (
	"fmt"
	"path/filepath"

	mc "machine-config-go"
)

func RunS01ReadScalars(fixturesDir, _ string) (bool, string) {
	path := filepath.Join(fixturesDir, "reference_config.h5")
	cfg, err := mc.NewReader(path).Parse()
	if err != nil {
		return false, fmt.Sprintf("read failed: %v", err)
	}
	if cfg.Meta.MachineName == "" {
		return false, "machine_name is empty"
	}
	if cfg.Meta.FileVersion != "1.0" {
		return false, fmt.Sprintf("file_version = %q", cfg.Meta.FileVersion)
	}
	if len(cfg.Meta.ConfigurationHash) != 64 {
		return false, fmt.Sprintf("configuration_hash len = %d", len(cfg.Meta.ConfigurationHash))
	}
	return true, fmt.Sprintf("machine_name=%q file_version=%q hash_len=%d",
		cfg.Meta.MachineName, cfg.Meta.FileVersion, len(cfg.Meta.ConfigurationHash))
}
