package scenarios

// S-07: OPCUA config roundtrip
//
// ID:       S-07
// Action:   Read reference_config_opcua.h5, verify OPCUA present, write ->
//           re-read, verify server_url survives.

import (
	"fmt"
	"os"
	"path/filepath"

	mc "machine-config-go"
)

func RunS07Opcua(fixturesDir, _ string) (bool, string) {
	path := filepath.Join(fixturesDir, "reference_config_opcua.h5")
	cfg, err := mc.NewReader(path).Parse()
	if err != nil {
		return false, fmt.Sprintf("read failed: %v", err)
	}
	if cfg.Opcua == nil || cfg.Opcua.Client.ServerURL == "" {
		return false, "expected opcua client with non-empty server_url"
	}
	origURL := cfg.Opcua.Client.ServerURL

	tmpPath, err := tempH5("mcl_go_s07")
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
	if rb.Opcua == nil || rb.Opcua.Client.ServerURL != origURL {
		return false, "server_url changed after roundtrip"
	}

	return true, fmt.Sprintf("opcua server_url=%q preserved through roundtrip", origURL)
}
