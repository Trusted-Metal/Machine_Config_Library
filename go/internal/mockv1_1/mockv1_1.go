// Package mockv1_1 implements a mock v1.1 adapter — test artifact only, used
// to exercise the change-category architecture (Addition/Removal/Name/Path/
// Name+Path), not a planned schema change. See
// docs/migrations/mock_v1_0_to_v1_1.md for the full manifest.
//
// Design: fully self-contained. The reader (reader.go) and writer (writer.go)
// reimplement v1.0's parse/write logic from scratch — including every
// "unchanged" subcomponent (light_source, collimator, scanner_card,
// clearbox, sfcf, opcua, and every Scanner/AxisConfig field besides the one
// renamed one) — rather than delegating to the real, public v1.0 adapter
// (machine-config-go/capabilities/v1_0/hdf5) and patching the 10 documented
// differences afterward. This package therefore imports nothing under
// machine-config-go/capabilities/v1_0/, per the architectural rule that no
// version's adapter — real or mock — may import or call into another
// version's adapter code: if v1.1 depended on v1.0's code, v1.0 could never
// be changed or removed later without checking v1.1, and every later
// version would compound the problem. See
// go/version_adapter_isolation_test.go for the automated guard.
//
// Layout constants and path helpers live in layout.go (duplicated from
// v1.0's layout package, not imported). Attribute-reading helpers live in
// helpers.go (duplicated from v1.0's hdf5 package). Mirrors
// rust/tests/mock_v1_1/mod.rs and nodejs/tests/mockV1_1.ts's design exactly.
//
// Lives under go/internal/ (not go/*_test.go directly) so it can be a normal
// importable package with non-test helper functions, while staying entirely
// invisible to real consumers of "machine-config-go" — the root package
// never imports it, mirroring the isolation Rust gets from tests/ being a
// separate integration-test crate.
//
// There is no dispatch-table injection here (unlike Python's _ADAPTERS or
// Node's _READERS/_WRITERS): Go's public dispatcher is a hardcoded switch in
// reader.go/writer.go, not a registry, so the AV-09-11 tests call
// MockV1_1Reader/MockV1_1Writer directly rather than through
// MachineConfigReader/MachineConfigWriter (see VALIDATION_PLAN.md §9.4).
package mockv1_1

import (
	mc "machine-config-go"
)

// On-disk constants that differ from v1.0.
const (
	FileVersion       = "1.1-mock"
	AttrFacilityID    = "Facility_ID"
	AttrConfigAuthor  = "Config_Author"
	AttrMachineLabel  = "Machine_Label"
	AttrFocalDistance = "Focal_Distance"
	AttrBPWidth       = "Width"
	AttrBPHeight      = "Height"
)

// MakeMockConfig mirrors Python's _make_config() / Node's makeMockConfig /
// Rust's make_mock_config.
func MakeMockConfig(machineName, facilityID, configAuthor *string) *mc.MachineConfig {
	builder := mc.NewMockConfigBuilder()
	builder.NLasers = 1
	builder.BuildPlateX = 250.0
	builder.BuildPlateY = 175.0 // distinct from X so name+path assertions are unambiguous
	if machineName != nil {
		builder.MachineName = *machineName
	} else {
		builder.MachineName = "MigrationTestMachine"
	}

	cfg := builder.Build()
	cfg.Meta.FileVersion = FileVersion
	cfg.Meta.FacilityID = facilityID
	cfg.Meta.ConfigAuthor = configAuthor
	cfg.Machine.GasFlowDirection = nil // absent in v1.1-mock by design
	cfg.Machine.RecoatDirection = nil
	// MockConfigBuilder never sets this (always nil) — give it a real value
	// so the PATH category (Build_Plate_Corner_Radius) has something
	// non-trivial to verify preservation of, mirroring Rust's make_mock_config.
	cfg.Machine.BuildPlateRadius = mc.Float64Ptr(10.0)

	return cfg
}
