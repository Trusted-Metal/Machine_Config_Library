package h5c

import (
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// repoFixture resolves a shared fixture path relative to this test file, the
// same runtime.Caller(0) technique go/reader_test.go's fixturesDir uses.
func repoFixture(t *testing.T, name string) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	// go/internal/h5c/compound_test.go → ../../../fixtures
	dir := filepath.Clean(filepath.Join(filepath.Dir(file), "..", "..", "..", "fixtures"))
	return filepath.Join(dir, name)
}

// Low-level tests for the H5T_COMPOUND primitives themselves — independent
// of and prior to the model-level (SynchronousSensor) tests, which build on
// top of these. See SYNCHRONOUS_SENSOR_PLAN.md's "Compound dataset string
// convention" for why EquationConstantRow.Name is fixed-length.

func TestEquationConstantsRoundtripExactValuesInOrder(t *testing.T) {
	path := filepath.Join(t.TempDir(), "eq.h5")
	f, err := Create(path)
	if err != nil {
		t.Fatal(err)
	}
	root, err := f.Root()
	if err != nil {
		t.Fatal(err)
	}
	rows := []EquationConstantRow{
		{Name: "a", Value: 0.4375},
		{Name: "b", Value: -2.75},
	}
	if err := root.CreateEquationConstantsDataset("Derivation_Equation_Constants", rows); err != nil {
		t.Fatal(err)
	}
	root.Close()
	f.Close()

	f2, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f2.Close()
	root2, err := f2.Root()
	if err != nil {
		t.Fatal(err)
	}
	defer root2.Close()
	got, err := root2.ReadEquationConstantsDataset("Derivation_Equation_Constants")
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 2 || got[0] != rows[0] || got[1] != rows[1] {
		t.Fatalf("got %+v, want %+v", got, rows)
	}
}

func TestEquationConstantsZeroRowRoundtrip(t *testing.T) {
	path := filepath.Join(t.TempDir(), "eq_empty.h5")
	f, err := Create(path)
	if err != nil {
		t.Fatal(err)
	}
	root, err := f.Root()
	if err != nil {
		t.Fatal(err)
	}
	if err := root.CreateEquationConstantsDataset("Derivation_Equation_Constants", []EquationConstantRow{}); err != nil {
		t.Fatal(err)
	}
	root.Close()
	f.Close()

	f2, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f2.Close()
	root2, err := f2.Root()
	if err != nil {
		t.Fatal(err)
	}
	defer root2.Close()
	got, err := root2.ReadEquationConstantsDataset("Derivation_Equation_Constants")
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 0 {
		t.Fatalf("got %+v, want empty", got)
	}
}

func TestEquationConstantNameOver64BytesRejectedNotTruncated(t *testing.T) {
	path := filepath.Join(t.TempDir(), "eq_overflow.h5")
	f, err := Create(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	root, err := f.Root()
	if err != nil {
		t.Fatal(err)
	}
	defer root.Close()

	longName := strings.Repeat("x", 65)
	err = root.CreateEquationConstantsDataset("Derivation_Equation_Constants", []EquationConstantRow{
		{Name: longName, Value: 1.0},
	})
	if err == nil {
		t.Fatal("expected an error for a 65-byte name, got nil")
	}
	if !strings.Contains(err.Error(), "does not fit") {
		t.Fatalf("expected a 'does not fit' error, got: %v", err)
	}
}

func TestEquationConstantNameExactly64BytesSucceeds(t *testing.T) {
	path := filepath.Join(t.TempDir(), "eq_exact64.h5")
	f, err := Create(path)
	if err != nil {
		t.Fatal(err)
	}
	root, err := f.Root()
	if err != nil {
		t.Fatal(err)
	}
	exactName := strings.Repeat("y", 64)
	if err := root.CreateEquationConstantsDataset("Derivation_Equation_Constants", []EquationConstantRow{
		{Name: exactName, Value: 2.0},
	}); err != nil {
		t.Fatal(err)
	}
	root.Close()
	f.Close()

	f2, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f2.Close()
	root2, err := f2.Root()
	if err != nil {
		t.Fatal(err)
	}
	defer root2.Close()
	got, err := root2.ReadEquationConstantsDataset("Derivation_Equation_Constants")
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 1 || got[0].Name != exactName || got[0].Value != 2.0 {
		t.Fatalf("got %+v", got)
	}
}

func TestCalibrationPointsRoundtripExactValuesInOrder(t *testing.T) {
	path := filepath.Join(t.TempDir(), "cal.h5")
	f, err := Create(path)
	if err != nil {
		t.Fatal(err)
	}
	root, err := f.Root()
	if err != nil {
		t.Fatal(err)
	}
	rows := []CalibrationPointRow{
		{InputValue: 4.0, OutputValue: -1.0},
		{InputValue: 20.0, OutputValue: 6.0},
	}
	if err := root.CreateCalibrationPointsDataset("Calibration_Points", rows); err != nil {
		t.Fatal(err)
	}
	root.Close()
	f.Close()

	f2, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f2.Close()
	root2, err := f2.Root()
	if err != nil {
		t.Fatal(err)
	}
	defer root2.Close()
	got, err := root2.ReadCalibrationPointsDataset("Calibration_Points")
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 2 || got[0] != rows[0] || got[1] != rows[1] {
		t.Fatalf("got %+v, want %+v", got, rows)
	}
}

func TestCalibrationPointsZeroRowRoundtrip(t *testing.T) {
	path := filepath.Join(t.TempDir(), "cal_empty.h5")
	f, err := Create(path)
	if err != nil {
		t.Fatal(err)
	}
	root, err := f.Root()
	if err != nil {
		t.Fatal(err)
	}
	if err := root.CreateCalibrationPointsDataset("Calibration_Points", []CalibrationPointRow{}); err != nil {
		t.Fatal(err)
	}
	root.Close()
	f.Close()

	f2, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f2.Close()
	root2, err := f2.Root()
	if err != nil {
		t.Fatal(err)
	}
	defer root2.Close()
	got, err := root2.ReadCalibrationPointsDataset("Calibration_Points")
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 0 {
		t.Fatalf("got %+v, want empty", got)
	}
}

// TestReadFixedLengthEquationConstantsWrittenByAnotherLanguage confirms real
// cross-language interop, not just self-consistency: reads the committed
// fixture's Derivation_Equation_Constants dataset — written by Python and
// patched in place to the fixed-64-byte convention (see
// SYNCHRONOUS_SENSOR_PLAN.md) — and confirms Go reads back the exact same
// values every other language's tests assert.
func TestReadFixedLengthEquationConstantsWrittenByAnotherLanguage(t *testing.T) {
	path := repoFixture(t, "reference_config_synchronous_sensors.h5")
	f, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	grp, err := f.Group("Machine/Optical_Trains/Optical_Train_01/Optional_Components/ClearBox/Synchronous_Sensors/Oxygen Sensor")
	if err != nil {
		t.Fatal(err)
	}
	defer grp.Close()

	got, err := grp.ReadEquationConstantsDataset("Derivation_Equation_Constants")
	if err != nil {
		t.Fatal(err)
	}
	want := []EquationConstantRow{
		{Name: "a", Value: 0.4375},
		{Name: "b", Value: -2.75},
	}
	if len(got) != 2 || got[0] != want[0] || got[1] != want[1] {
		t.Fatalf("got %+v, want %+v", got, want)
	}

	points, err := grp.ReadCalibrationPointsDataset("Calibration_Points")
	if err != nil {
		t.Fatal(err)
	}
	wantPoints := []CalibrationPointRow{
		{InputValue: 4.0, OutputValue: -1.0},
		{InputValue: 20.0, OutputValue: 6.0},
	}
	if len(points) != 2 || points[0] != wantPoints[0] || points[1] != wantPoints[1] {
		t.Fatalf("got %+v, want %+v", points, wantPoints)
	}
}
