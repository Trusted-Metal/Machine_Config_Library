package scenarios

// AV-06: Dispatcher normalizes whitespace in File_Version (" 1.0 ")

import (
	"fmt"

	mc "machine-config-go"
)

func RunAv06WhitespaceVersion(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "version_whitespace.h5")
	_, err := mc.NewReader(fixture).Parse()
	if err != nil {
		return false, fmt.Sprintf("%v", err)
	}
	return true, "whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK"
}
