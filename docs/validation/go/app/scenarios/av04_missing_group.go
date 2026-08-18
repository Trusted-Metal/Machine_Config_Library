package scenarios

// AV-04: Reader returns an error when required group (Machine/) is absent

import (
	"fmt"

	mc "machine-config-go"
)

func RunAv04MissingGroup(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "missing_machine_group.h5")
	_, err := mc.NewReader(fixture).Parse()
	if err == nil {
		return false, "no error raised for missing Machine/ group"
	}
	return true, fmt.Sprintf("error raised for missing Machine/ group: %v", err)
}
