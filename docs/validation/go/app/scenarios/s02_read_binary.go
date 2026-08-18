package scenarios

// S-02: Read correction grids, shape and finite values
//
// ID:       S-02
// Action:   GetCorrectionData(0) on reference_config.h5.
// Expected: shape == [257,257,2]; grid contains NaNs (out-of-field cells).

import (
	"fmt"
	"math"
	"path/filepath"

	mc "machine-config-go"
)

func RunS02ReadBinary(fixturesDir, _ string) (bool, string) {
	path := filepath.Join(fixturesDir, "reference_config.h5")
	cd, err := mc.NewReader(path).GetCorrectionData(0)
	if err != nil {
		return false, fmt.Sprintf("GetCorrectionData failed: %v", err)
	}
	if cd.Shape != [3]int{257, 257, 2} {
		return false, fmt.Sprintf("shape = %v", cd.Shape)
	}
	nan := 0
	for _, v := range cd.Data {
		if math.IsNaN(v) {
			nan++
		}
	}
	if nan == 0 {
		return false, "expected NaNs in correction grid"
	}
	return true, fmt.Sprintf("shape=%v nan_count=%d total=%d", cd.Shape, nan, len(cd.Data))
}
