package scenarios

// AV-07: Dispatcher handles empty string File_Version
//
// Either outcome is acceptable per VALIDATION_PLAN.md — this records the
// actual behavior rather than asserting one, mirroring Rust's/Python's/Node's
// AV-07. Verified: Go's PeekFileVersion treats an empty File_Version string
// as "1.0" (no error).

import (
	"fmt"

	mc "machine-config-go"
)

func RunAv07EmptyVersion(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "empty_version.h5")
	cfg, err := mc.NewReader(fixture).Parse()
	if err != nil {
		return true, fmt.Sprintf("empty File_Version raises %v", err)
	}
	return true, fmt.Sprintf("empty File_Version dispatches OK, file_version=%q", cfg.Meta.FileVersion)
}
