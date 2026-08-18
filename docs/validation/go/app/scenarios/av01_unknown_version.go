package scenarios

// AV-01: Reader rejects unknown File_Version with typed error

import (
	"errors"
	"fmt"

	mc "machine-config-go"
)

func RunAv01UnknownVersion(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "v2_0_unknown.h5")
	_, err := mc.NewReader(fixture).Parse()
	if err == nil {
		return false, "no error raised for File_Version='2.0'"
	}
	var uv *mc.UnsupportedFileVersionError
	if errors.As(err, &uv) {
		if uv.Version == "2.0" {
			return true, fmt.Sprintf("UnsupportedFileVersionError raised, version=%q", uv.Version)
		}
		return false, fmt.Sprintf("UnsupportedFileVersionError raised but version=%q", uv.Version)
	}
	return false, fmt.Sprintf("wrong error type: %v", err)
}
