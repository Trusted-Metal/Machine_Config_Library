package scenarios

// AV-03: v1.0 reader encountering a v1.1 file fails loudly

import (
	"errors"
	"fmt"

	mc "machine-config-go"
)

func RunAv03FutureVersion(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "v1_1_simulated.h5")
	_, err := mc.NewReader(fixture).Parse()
	if err == nil {
		return false, "no error raised for File_Version='1.1'"
	}
	var uv *mc.UnsupportedFileVersionError
	if errors.As(err, &uv) {
		if uv.Version == "1.1" {
			return true, fmt.Sprintf("UnsupportedFileVersionError raised, version=%q", uv.Version)
		}
		return false, fmt.Sprintf("UnsupportedFileVersionError raised but version=%q", uv.Version)
	}
	return false, fmt.Sprintf("wrong error type: %v", err)
}
