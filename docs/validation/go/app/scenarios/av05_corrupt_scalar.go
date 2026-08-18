package scenarios

// AV-05: Reader handles corrupt required attribute (Build_Plate_X_Dimension) gracefully

import (
	"fmt"

	mc "machine-config-go"
)

func RunAv05CorruptScalar(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "corrupt_scalar.h5")
	_, err := mc.NewReader(fixture).Parse()
	if err == nil {
		return false, "no error raised for corrupt Build_Plate_X_Dimension"
	}
	return true, fmt.Sprintf("error raised for corrupt scalar: %v", err)
}
