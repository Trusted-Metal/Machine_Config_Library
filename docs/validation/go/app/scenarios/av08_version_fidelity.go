package scenarios

// AV-08: File_Version string survives write->read unchanged

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	mc "machine-config-go"
)

func RunAv08VersionFidelity(fixturesDir, _ string) (bool, string) {
	path := filepath.Join(fixturesDir, "reference_config.h5")
	cfg, err := mc.NewReader(path).Parse()
	if err != nil {
		return false, fmt.Sprintf("read failed: %v", err)
	}
	origVersion := strings.TrimSpace(cfg.Meta.FileVersion)

	tmpPath, err := tempH5("mcl_go_av08")
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
	rbVersion := strings.TrimSpace(rb.Meta.FileVersion)

	if rbVersion != "1.0" {
		return false, fmt.Sprintf("file_version after roundtrip: expected '1.0', got %q", rbVersion)
	}
	if rbVersion != origVersion {
		return false, fmt.Sprintf("file_version changed: %q -> %q", origVersion, rbVersion)
	}

	return true, fmt.Sprintf("File_Version survives roundtrip unchanged: %q", rbVersion)
}
