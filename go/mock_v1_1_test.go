// AV-09-AV-11: mock v1.1 adapter migration tests.
//
// Mirrors rust/tests/adapter_migration_test.rs / python/tests/test_adapter_migration.py's
// adapter-level tests (v1_1_read, v1_1_roundtrip, v1_to_v1_1, v1_1_to_v1, v1_unaffected)
// and nodejs/tests/adapterMigration.test.ts.
//
// Deliberately missing, by design (see VALIDATION_PLAN.md §9.4 and mockv1_1's package
// doc): Python's/Node's *dispatcher-level* tests, which prove the public
// MachineConfigReader/Writer facade itself routes to the mock via a temporarily-injected
// dispatch-table entry. Go's public dispatcher is a hardcoded switch in reader.go/writer.go,
// not a registry — there's no entry to inject. AV-09's actual rationale ("adding v1.1
// doesn't require modifying the v1.0 adapter") is satisfied here by the mock living in its
// own package (go/internal/mockv1_1) with zero edits to capabilities/v1_0/, plus the full
// pre-existing suite (every other test in this module) staying green after it was added.
package machineconfig_test

import (
	"path/filepath"
	"runtime"
	"testing"

	mc "machine-config-go"
	"machine-config-go/internal/mockv1_1"
)

func referenceFixture(t *testing.T) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	return filepath.Join(filepath.Dir(file), "..", "fixtures", "reference_config.h5")
}

// Existing v1.0 read path is undisturbed — no mock adapter involved. This,
// combined with every other test in this module staying green after the mock
// was added, is AV-09's actual proof: adding v1.1 required zero edits to the
// v1.0 adapter.
func TestV1Unaffected(t *testing.T) {
	cfg, err := mc.NewReader(referenceFixture(t)).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Meta.FileVersion != "1.0" {
		t.Fatalf("file_version = %q", cfg.Meta.FileVersion)
	}
	if len(cfg.OpticalTrains) == 0 {
		t.Fatal("expected optical trains")
	}
	if cfg.Machine.BuildPlateX == nil {
		t.Fatal("expected build_plate_x")
	}
}

// Mock v1.1 file -> StableModel -- all five change categories asserted.
func TestV1_1ReadAllCategories(t *testing.T) {
	cfg := mockv1_1.MakeMockConfig(nil, mc.StrPtr("Lab-001"), mc.StrPtr("TestEngineer"))
	path := filepath.Join(t.TempDir(), "v1_1.h5")
	if err := mockv1_1.NewWriter(cfg).Write(path); err != nil {
		t.Fatal(err)
	}
	result, err := mockv1_1.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}

	// ADDITION (x2)
	if result.Meta.FacilityID == nil || *result.Meta.FacilityID != "Lab-001" {
		t.Fatalf("facility_id = %v", result.Meta.FacilityID)
	}
	if result.Meta.ConfigAuthor == nil || *result.Meta.ConfigAuthor != "TestEngineer" {
		t.Fatalf("config_author = %v", result.Meta.ConfigAuthor)
	}

	// REMOVAL (x2)
	if result.Machine.GasFlowDirection != nil {
		t.Fatalf("gas_flow_direction = %v, want nil", result.Machine.GasFlowDirection)
	}
	if result.Machine.RecoatDirection != nil {
		t.Fatalf("recoat_direction = %v, want nil", result.Machine.RecoatDirection)
	}

	// NAME (x2)
	if result.Machine.MachineName != "MigrationTestMachine" {
		t.Fatalf("machine_name = %q", result.Machine.MachineName)
	}
	if !f64PtrEqual(result.OpticalTrains[0].Scanner.WorkingDistance, cfg.OpticalTrains[0].Scanner.WorkingDistance) {
		t.Fatalf("working_distance = %v, want %v", result.OpticalTrains[0].Scanner.WorkingDistance, cfg.OpticalTrains[0].Scanner.WorkingDistance)
	}

	// PATH (x2)
	if !f64PtrEqual(result.Machine.BuildPlateZ, cfg.Machine.BuildPlateZ) {
		t.Fatalf("build_plate_z = %v, want %v", result.Machine.BuildPlateZ, cfg.Machine.BuildPlateZ)
	}
	if !f64PtrEqual(result.Machine.BuildPlateRadius, cfg.Machine.BuildPlateRadius) {
		t.Fatalf("build_plate_radius = %v, want %v", result.Machine.BuildPlateRadius, cfg.Machine.BuildPlateRadius)
	}

	// NAME+PATH (x2)
	if !f64PtrEqual(result.Machine.BuildPlateX, cfg.Machine.BuildPlateX) {
		t.Fatalf("build_plate_x = %v, want %v", result.Machine.BuildPlateX, cfg.Machine.BuildPlateX)
	}
	if !f64PtrEqual(result.Machine.BuildPlateY, cfg.Machine.BuildPlateY) {
		t.Fatalf("build_plate_y = %v, want %v", result.Machine.BuildPlateY, cfg.Machine.BuildPlateY)
	}
}

// Mock v1.1 -> StableModel -> mock v1.1 -> StableModel -- all categories
// survive both passes.
func TestV1_1Roundtrip(t *testing.T) {
	cfg := mockv1_1.MakeMockConfig(nil, mc.StrPtr("RoundtripLab"), mc.StrPtr("RoundtripEngineer"))

	p1 := filepath.Join(t.TempDir(), "v1_1_pass1.h5")
	if err := mockv1_1.NewWriter(cfg).Write(p1); err != nil {
		t.Fatal(err)
	}
	mid, err := mockv1_1.NewReader(p1).Parse()
	if err != nil {
		t.Fatal(err)
	}

	p2 := filepath.Join(t.TempDir(), "v1_1_pass2.h5")
	if err := mockv1_1.NewWriter(mid).Write(p2); err != nil {
		t.Fatal(err)
	}
	result, err := mockv1_1.NewReader(p2).Parse()
	if err != nil {
		t.Fatal(err)
	}

	if result.Meta.FacilityID == nil || *result.Meta.FacilityID != "RoundtripLab" {
		t.Fatalf("facility_id = %v", result.Meta.FacilityID)
	}
	if result.Meta.ConfigAuthor == nil || *result.Meta.ConfigAuthor != "RoundtripEngineer" {
		t.Fatalf("config_author = %v", result.Meta.ConfigAuthor)
	}
	if result.Machine.GasFlowDirection != nil {
		t.Fatalf("gas_flow_direction = %v, want nil", result.Machine.GasFlowDirection)
	}
	if result.Machine.RecoatDirection != nil {
		t.Fatalf("recoat_direction = %v, want nil", result.Machine.RecoatDirection)
	}
	if result.Machine.MachineName != cfg.Machine.MachineName {
		t.Fatalf("machine_name = %q, want %q", result.Machine.MachineName, cfg.Machine.MachineName)
	}
	if !f64PtrEqual(result.OpticalTrains[0].Scanner.WorkingDistance, cfg.OpticalTrains[0].Scanner.WorkingDistance) {
		t.Fatalf("working_distance mismatch")
	}
	if !f64PtrEqual(result.Machine.BuildPlateZ, cfg.Machine.BuildPlateZ) ||
		!f64PtrEqual(result.Machine.BuildPlateRadius, cfg.Machine.BuildPlateRadius) ||
		!f64PtrEqual(result.Machine.BuildPlateX, cfg.Machine.BuildPlateX) ||
		!f64PtrEqual(result.Machine.BuildPlateY, cfg.Machine.BuildPlateY) {
		t.Fatal("build-plate fields did not survive roundtrip")
	}
}

// Real v1.0 fixture -> StableModel -> mock v1.1 layout -- surviving fields
// preserved; ADDITION fields nil (no v1.0 source).
func TestV1ToV1_1ForwardMigration(t *testing.T) {
	source, err := mc.NewReader(referenceFixture(t)).Parse()
	if err != nil {
		t.Fatal(err)
	}
	migratedInput := *source
	migratedInput.Meta.FileVersion = mockv1_1.FileVersion

	out := filepath.Join(t.TempDir(), "v1_to_v1_1.h5")
	if err := mockv1_1.NewWriter(&migratedInput).Write(out); err != nil {
		t.Fatal(err)
	}
	result, err := mockv1_1.NewReader(out).Parse()
	if err != nil {
		t.Fatal(err)
	}

	// NAME
	if result.Machine.MachineName != source.Machine.MachineName {
		t.Fatalf("machine_name = %q, want %q", result.Machine.MachineName, source.Machine.MachineName)
	}
	// NAME+PATH
	if !f64PtrEqual(result.Machine.BuildPlateX, source.Machine.BuildPlateX) {
		t.Fatal("build_plate_x mismatch")
	}
	if !f64PtrEqual(result.Machine.BuildPlateY, source.Machine.BuildPlateY) {
		t.Fatal("build_plate_y mismatch")
	}
	// PATH
	if !f64PtrEqual(result.Machine.BuildPlateZ, source.Machine.BuildPlateZ) {
		t.Fatal("build_plate_z mismatch")
	}
	if !f64PtrEqual(result.Machine.BuildPlateRadius, source.Machine.BuildPlateRadius) {
		t.Fatal("build_plate_radius mismatch")
	}
	// NAME (scanner)
	if !f64PtrEqual(result.OpticalTrains[0].Scanner.WorkingDistance, source.OpticalTrains[0].Scanner.WorkingDistance) {
		t.Fatal("working_distance mismatch")
	}
	// REMOVAL: always nil regardless of what the v1.0 source contained
	if result.Machine.GasFlowDirection != nil {
		t.Fatalf("gas_flow_direction = %v, want nil", result.Machine.GasFlowDirection)
	}
	if result.Machine.RecoatDirection != nil {
		t.Fatalf("recoat_direction = %v, want nil", result.Machine.RecoatDirection)
	}
	// ADDITION: no v1.0 source -> typed fields nil after forward migration
	if result.Meta.FacilityID != nil {
		t.Fatalf("facility_id = %v, want nil", result.Meta.FacilityID)
	}
	if result.Meta.ConfigAuthor != nil {
		t.Fatalf("config_author = %v, want nil", result.Meta.ConfigAuthor)
	}
}

// Mock v1.1 file -> StableModel -> v1.0 layout -- surviving fields preserved;
// ADDITION fields lost (v1.0 writer doesn't write them).
func TestV1_1ToV1BackwardMigration(t *testing.T) {
	cfg := mockv1_1.MakeMockConfig(nil, mc.StrPtr("Lab-V11"), mc.StrPtr("MigrationBot"))

	v11Path := filepath.Join(t.TempDir(), "v1_1.h5")
	if err := mockv1_1.NewWriter(cfg).Write(v11Path); err != nil {
		t.Fatal(err)
	}
	v11Config, err := mockv1_1.NewReader(v11Path).Parse()
	if err != nil {
		t.Fatal(err)
	}

	downgradeInput := *v11Config
	downgradeInput.Meta.FileVersion = "1.0"

	v1Path := filepath.Join(t.TempDir(), "v1.h5")
	if err := mc.NewWriter().Write(&downgradeInput, v1Path); err != nil {
		t.Fatal(err)
	}
	result, err := mc.NewReader(v1Path).Parse()
	if err != nil {
		t.Fatal(err)
	}

	if result.Machine.MachineName != cfg.Machine.MachineName {
		t.Fatalf("machine_name = %q, want %q", result.Machine.MachineName, cfg.Machine.MachineName)
	}
	if !f64PtrEqual(result.Machine.BuildPlateX, cfg.Machine.BuildPlateX) ||
		!f64PtrEqual(result.Machine.BuildPlateY, cfg.Machine.BuildPlateY) ||
		!f64PtrEqual(result.Machine.BuildPlateZ, cfg.Machine.BuildPlateZ) ||
		!f64PtrEqual(result.Machine.BuildPlateRadius, cfg.Machine.BuildPlateRadius) {
		t.Fatal("build-plate fields did not survive backward migration")
	}
	if !f64PtrEqual(result.OpticalTrains[0].Scanner.WorkingDistance, cfg.OpticalTrains[0].Scanner.WorkingDistance) {
		t.Fatal("working_distance mismatch")
	}

	// REMOVAL: absent in v1.1 -> remain nil after roundtrip through v1.0
	if result.Machine.GasFlowDirection != nil {
		t.Fatalf("gas_flow_direction = %v, want nil", result.Machine.GasFlowDirection)
	}
	if result.Machine.RecoatDirection != nil {
		t.Fatalf("recoat_direction = %v, want nil", result.Machine.RecoatDirection)
	}

	// ADDITION: typed v1.1 fields are lost during backward migration (the
	// real v1.0 writer never writes Facility_ID/Config_Author).
	if result.Meta.FacilityID != nil {
		t.Fatalf("facility_id = %v, want nil", result.Meta.FacilityID)
	}
	if result.Meta.ConfigAuthor != nil {
		t.Fatalf("config_author = %v, want nil", result.Meta.ConfigAuthor)
	}
}

func f64PtrEqual(a, b *float64) bool {
	if a == nil || b == nil {
		return a == b
	}
	return *a == *b
}
