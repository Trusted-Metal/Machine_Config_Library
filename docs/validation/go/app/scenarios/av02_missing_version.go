package scenarios

// AV-02: Reader handles absent File_Version attribute predictably
//
// Either outcome (defaults to v1.0 dispatch, or a typed error) is acceptable
// per VALIDATION_PLAN.md — behavior must simply be documented, and must match
// across all five languages. This scenario records what actually happens
// rather than asserting one outcome, mirroring Rust's/Python's/Node's AV-02.
// Verified: Go's PeekFileVersion (go/file_version.go) treats a missing
// File_Version attribute as "1.0" (no error).

import (
	"fmt"

	mc "machine-config-go"
)

func RunAv02MissingVersion(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "missing_version.h5")
	cfg, err := mc.NewReader(fixture).Parse()
	if err != nil {
		return true, fmt.Sprintf("missing File_Version raises %v", err)
	}
	return true, fmt.Sprintf("missing File_Version dispatches OK, file_version=%q", cfg.Meta.FileVersion)
}
