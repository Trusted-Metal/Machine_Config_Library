// Unit tests for internal/models's power_characterization.go — the shared,
// version-agnostic shape-conversion functions Phase 2
// (V1_1_IMPLEMENTATION_PLAN.md) introduced to replace
// MigrateV1ToV1_1/MigrateV1_1ToV1. Pure functions, no HDF5 I/O — integration
// with the real writers is covered separately in v1_1_adapter_test.go.
// Mirrors python/tests/test_power_characterization.py,
// rust/tests/power_characterization_test.rs, and
// cpp/tests/test_power_characterization.cpp.
package machineconfig_test

import (
	"testing"

	machineconfig "machine-config-go"
	"machine-config-go/internal/models"
)

// ---------------------------------------------------------------------------
// ForwardPowerCharacterizationCoefficients (Change 3 / ClearBox shape)
// ---------------------------------------------------------------------------

func TestForwardCoefficientsBothNilReturnsNil(t *testing.T) {
	pc, err := models.ForwardPowerCharacterizationCoefficients(nil, nil)
	if err != nil {
		t.Fatal(err)
	}
	if pc != nil {
		t.Fatalf("expected nil, got %+v", pc)
	}
}

func TestForwardCoefficientsLinear(t *testing.T) {
	pc, err := models.ForwardPowerCharacterizationCoefficients(machineconfig.StrPtr("LINEAR"), machineconfig.StrPtr("50.0,100.0"))
	if err != nil {
		t.Fatal(err)
	}
	if pc.AlgorithmType == nil || *pc.AlgorithmType != "LINEAR" {
		t.Fatalf("algorithm_type = %v", pc.AlgorithmType)
	}
	if pc.AlgorithmEquation == nil || *pc.AlgorithmEquation != "W = a*V + b" {
		t.Fatalf("algorithm_equation = %v", pc.AlgorithmEquation)
	}
	if !strSliceEqual(equationConstantNames(pc.DerivationEquationConstants), []string{"b", "a"}) {
		t.Fatalf("constant names = %v", equationConstantNames(pc.DerivationEquationConstants))
	}
	if pc.DerivationEquationConstants[0].Value != 50.0 || pc.DerivationEquationConstants[1].Value != 100.0 {
		t.Fatalf("constant values = %v", pc.DerivationEquationConstants)
	}
	if len(pc.CharacterizationPoints) != 0 {
		t.Fatalf("characterization_points = %v, want empty", pc.CharacterizationPoints)
	}
}

func TestForwardCoefficientsPolynomial(t *testing.T) {
	pc, err := models.ForwardPowerCharacterizationCoefficients(machineconfig.StrPtr("POLYNOMIAL"), machineconfig.StrPtr("1.0,2.0,3.0"))
	if err != nil {
		t.Fatal(err)
	}
	if pc.AlgorithmType == nil || *pc.AlgorithmType != "POLYNOMIAL" {
		t.Fatalf("algorithm_type = %v", pc.AlgorithmType)
	}
	if pc.AlgorithmEquation == nil || *pc.AlgorithmEquation != "W = c0 + c1*V + c2*V^2" {
		t.Fatalf("algorithm_equation = %v", pc.AlgorithmEquation)
	}
	if !strSliceEqual(equationConstantNames(pc.DerivationEquationConstants), []string{"c0", "c1", "c2"}) {
		t.Fatalf("constant names = %v", equationConstantNames(pc.DerivationEquationConstants))
	}
}

func TestForwardCoefficientsBlankInputTypeAndUnits(t *testing.T) {
	// Confirmed 2026-09-01: blank, not hardcoded — see power_characterization.go's docs.
	pc, err := models.ForwardPowerCharacterizationCoefficients(machineconfig.StrPtr("LINEAR"), machineconfig.StrPtr("50.0,100.0"))
	if err != nil {
		t.Fatal(err)
	}
	if pc.InputType != nil {
		t.Fatalf("input_type = %v, want nil", pc.InputType)
	}
	if pc.UnitsDerivedQuantity != nil {
		t.Fatalf("units_derived_quantity = %v, want nil", pc.UnitsDerivedQuantity)
	}
}

func TestForwardCoefficientsUnrecognizedAlgorithmNeverErrors(t *testing.T) {
	// Never a hard error — see power_characterization.go's "Never raises" docs.
	pc, err := models.ForwardPowerCharacterizationCoefficients(machineconfig.StrPtr("EXPONENTIAL"), machineconfig.StrPtr("1.5,2.5,3.5"))
	if err != nil {
		t.Fatal(err)
	}
	if pc.AlgorithmType == nil || *pc.AlgorithmType != "EXPONENTIAL" {
		t.Fatalf("algorithm_type = %v", pc.AlgorithmType)
	}
	if pc.AlgorithmEquation != nil {
		t.Fatalf("algorithm_equation = %v, want nil", pc.AlgorithmEquation)
	}
	if !strSliceEqual(equationConstantNames(pc.DerivationEquationConstants), []string{"0", "1", "2"}) {
		t.Fatalf("constant names = %v", equationConstantNames(pc.DerivationEquationConstants))
	}
	gotValues := []float64{
		pc.DerivationEquationConstants[0].Value,
		pc.DerivationEquationConstants[1].Value,
		pc.DerivationEquationConstants[2].Value,
	}
	if !floatSliceEqual(gotValues, []float64{1.5, 2.5, 3.5}) {
		t.Fatalf("constant values = %v", gotValues)
	}
}

// ---------------------------------------------------------------------------
// ForwardPowerCharacterizationPoints (Change 4 / Light_Source shape)
// ---------------------------------------------------------------------------

func TestForwardPointsBothNilReturnsNil(t *testing.T) {
	pc, err := models.ForwardPowerCharacterizationPoints(nil, nil)
	if err != nil {
		t.Fatal(err)
	}
	if pc != nil {
		t.Fatalf("expected nil, got %+v", pc)
	}
}

func TestForwardPointsLinear(t *testing.T) {
	pc, err := models.ForwardPowerCharacterizationPoints(machineconfig.StrPtr("LINEAR"), machineconfig.StrPtr("1,100,10,1000"))
	if err != nil {
		t.Fatal(err)
	}
	if pc.AlgorithmType == nil || *pc.AlgorithmType != "LINEAR" {
		t.Fatalf("algorithm_type = %v", pc.AlgorithmType)
	}
	if pc.AlgorithmEquation == nil || *pc.AlgorithmEquation != "W = a*V + b" {
		t.Fatalf("algorithm_equation = %v", pc.AlgorithmEquation)
	}
	if len(pc.CharacterizationPoints) != 2 {
		t.Fatalf("characterization_points len = %d, want 2", len(pc.CharacterizationPoints))
	}
	if pc.CharacterizationPoints[0].InputValue != 1.0 || pc.CharacterizationPoints[0].OutputValue != 100.0 {
		t.Fatalf("point 0 = %+v", pc.CharacterizationPoints[0])
	}
	if pc.CharacterizationPoints[1].InputValue != 10.0 || pc.CharacterizationPoints[1].OutputValue != 1000.0 {
		t.Fatalf("point 1 = %+v", pc.CharacterizationPoints[1])
	}
	if len(pc.DerivationEquationConstants) != 0 {
		t.Fatalf("derivation_equation_constants = %v, want empty", pc.DerivationEquationConstants)
	}
}

func TestForwardPointsPolynomialLeavesEquationBlank(t *testing.T) {
	// Point count doesn't reliably indicate polynomial degree — never
	// generated for POLYNOMIAL, unlike the coefficients shape.
	pc, err := models.ForwardPowerCharacterizationPoints(machineconfig.StrPtr("POLYNOMIAL"), machineconfig.StrPtr("1,100,10,1000"))
	if err != nil {
		t.Fatal(err)
	}
	if pc.AlgorithmEquation != nil {
		t.Fatalf("algorithm_equation = %v, want nil", pc.AlgorithmEquation)
	}
}

func TestForwardPointsUnrecognizedAlgorithmNeverErrors(t *testing.T) {
	pc, err := models.ForwardPowerCharacterizationPoints(machineconfig.StrPtr("QUADRATIC"), machineconfig.StrPtr("1,100,10,1000"))
	if err != nil {
		t.Fatal(err)
	}
	if pc.AlgorithmType == nil || *pc.AlgorithmType != "QUADRATIC" {
		t.Fatalf("algorithm_type = %v", pc.AlgorithmType)
	}
	if pc.AlgorithmEquation != nil {
		t.Fatalf("algorithm_equation = %v, want nil", pc.AlgorithmEquation)
	}
	if len(pc.CharacterizationPoints) != 2 {
		t.Fatalf("characterization_points len = %d, want 2", len(pc.CharacterizationPoints))
	}
}

func TestForwardPointsStripsBrackets(t *testing.T) {
	// Real Watts_To_Volts_Params fixtures wrap the CSV in brackets, unlike
	// Volts_To_Watts_Params's plain form.
	pc, err := models.ForwardPowerCharacterizationPoints(machineconfig.StrPtr("LINEAR"), machineconfig.StrPtr("[1,100,10,1000]"))
	if err != nil {
		t.Fatal(err)
	}
	if len(pc.CharacterizationPoints) != 2 {
		t.Fatalf("characterization_points len = %d, want 2", len(pc.CharacterizationPoints))
	}
	if pc.CharacterizationPoints[0].InputValue != 1.0 || pc.CharacterizationPoints[1].OutputValue != 1000.0 {
		t.Fatalf("characterization_points = %v", pc.CharacterizationPoints)
	}
}

func TestForwardPointsBlankInputTypeAndUnits(t *testing.T) {
	pc, err := models.ForwardPowerCharacterizationPoints(machineconfig.StrPtr("LINEAR"), machineconfig.StrPtr("1,100"))
	if err != nil {
		t.Fatal(err)
	}
	if pc.InputType != nil {
		t.Fatalf("input_type = %v, want nil", pc.InputType)
	}
	if pc.UnitsDerivedQuantity != nil {
		t.Fatalf("units_derived_quantity = %v, want nil", pc.UnitsDerivedQuantity)
	}
}

// ---------------------------------------------------------------------------
// BackwardFlatFieldsCoefficients (Change 3 backward)
// ---------------------------------------------------------------------------

func TestBackwardCoefficientsNilPCReturnsNilNil(t *testing.T) {
	algorithm, params := models.BackwardFlatFieldsCoefficients(nil)
	if algorithm != nil || params != nil {
		t.Fatalf("got (%v, %v), want (nil, nil)", algorithm, params)
	}
}

func TestBackwardCoefficientsEmptyConstantsWritesBlankNotError(t *testing.T) {
	pc := &machineconfig.PowerCharacterization{
		AlgorithmType:     machineconfig.StrPtr("LINEAR"),
		AlgorithmEquation: machineconfig.StrPtr("W = a*V + b"),
	}
	algorithm, params := models.BackwardFlatFieldsCoefficients(pc)
	if algorithm == nil || *algorithm != "LINEAR" {
		t.Fatalf("algorithm = %v", algorithm)
	}
	if params == nil || *params != "" {
		t.Fatalf("params = %v, want \"\"", params)
	}
}

func TestBackwardCoefficientsReordersByName(t *testing.T) {
	// Rows aren't positionally guaranteed on disk — must re-sort before
	// joining as CSV: b before a for LINEAR.
	pc := &machineconfig.PowerCharacterization{
		AlgorithmType: machineconfig.StrPtr("LINEAR"),
		DerivationEquationConstants: []machineconfig.EquationConstant{
			{Name: "a", Value: 2.0},
			{Name: "b", Value: 1.0},
		},
	}
	algorithm, params := models.BackwardFlatFieldsCoefficients(pc)
	if algorithm == nil || *algorithm != "LINEAR" {
		t.Fatalf("algorithm = %v", algorithm)
	}
	if params == nil || *params != "1.0,2.0" {
		t.Fatalf("params = %v, want \"1.0,2.0\"", params)
	}
}

func TestBackwardCoefficientsReordersPolynomialByNumericSuffix(t *testing.T) {
	pc := &machineconfig.PowerCharacterization{
		AlgorithmType: machineconfig.StrPtr("POLYNOMIAL"),
		DerivationEquationConstants: []machineconfig.EquationConstant{
			{Name: "c2", Value: 3.0},
			{Name: "c0", Value: 1.0},
			{Name: "c1", Value: 2.0},
		},
	}
	_, params := models.BackwardFlatFieldsCoefficients(pc)
	if params == nil || *params != "1.0,2.0,3.0" {
		t.Fatalf("params = %v, want \"1.0,2.0,3.0\"", params)
	}
}

func TestBackwardCoefficientsReordersPositionalFallbackNumerically(t *testing.T) {
	// Unrecognized-algorithm-type fallback names ('0','1',...) must sort
	// numerically, not lexically (else '10' would sort before '2').
	pc := &machineconfig.PowerCharacterization{
		AlgorithmType: machineconfig.StrPtr("EXPONENTIAL"),
		DerivationEquationConstants: []machineconfig.EquationConstant{
			{Name: "1", Value: 2.5},
			{Name: "0", Value: 1.5},
			{Name: "2", Value: 3.5},
		},
	}
	algorithm, params := models.BackwardFlatFieldsCoefficients(pc)
	if algorithm == nil || *algorithm != "EXPONENTIAL" {
		t.Fatalf("algorithm = %v", algorithm)
	}
	if params == nil || *params != "1.5,2.5,3.5" {
		t.Fatalf("params = %v, want \"1.5,2.5,3.5\"", params)
	}
}

func TestForwardThenBackwardCoefficientsRoundTripsUnrecognizedType(t *testing.T) {
	originalAlgorithm, originalParams := "EXPONENTIAL", "1.5,2.5,3.5"
	pc, err := models.ForwardPowerCharacterizationCoefficients(&originalAlgorithm, &originalParams)
	if err != nil {
		t.Fatal(err)
	}
	algorithm, params := models.BackwardFlatFieldsCoefficients(pc)
	if algorithm == nil || *algorithm != originalAlgorithm {
		t.Fatalf("algorithm = %v, want %q", algorithm, originalAlgorithm)
	}
	if params == nil || *params != originalParams {
		t.Fatalf("params = %v, want %q", params, originalParams)
	}
}

// ---------------------------------------------------------------------------
// BackwardFlatFieldsPoints (Change 4 backward)
// ---------------------------------------------------------------------------

func TestBackwardPointsNilPCReturnsNilNil(t *testing.T) {
	algorithm, params := models.BackwardFlatFieldsPoints(nil)
	if algorithm != nil || params != nil {
		t.Fatalf("got (%v, %v), want (nil, nil)", algorithm, params)
	}
}

func TestBackwardPointsEmptyWritesBlankNotError(t *testing.T) {
	pc := &machineconfig.PowerCharacterization{
		AlgorithmType:     machineconfig.StrPtr("LINEAR"),
		AlgorithmEquation: machineconfig.StrPtr("W = a*V + b"),
	}
	algorithm, params := models.BackwardFlatFieldsPoints(pc)
	if algorithm == nil || *algorithm != "LINEAR" {
		t.Fatalf("algorithm = %v", algorithm)
	}
	if params == nil || *params != "" {
		t.Fatalf("params = %v, want \"\"", params)
	}
}

func TestBackwardPointsNoResortNeeded(t *testing.T) {
	// Characterization_Points rows aren't named — on-disk order is already
	// correct, unlike the coefficients shape.
	pc := &machineconfig.PowerCharacterization{
		AlgorithmType: machineconfig.StrPtr("LINEAR"),
		CharacterizationPoints: []machineconfig.CalibrationPoint{
			{InputValue: 1.0, OutputValue: 100.0},
			{InputValue: 10.0, OutputValue: 1000.0},
		},
	}
	algorithm, params := models.BackwardFlatFieldsPoints(pc)
	if algorithm == nil || *algorithm != "LINEAR" {
		t.Fatalf("algorithm = %v", algorithm)
	}
	if params == nil || *params != "1.0,100.0,10.0,1000.0" {
		t.Fatalf("params = %v, want \"1.0,100.0,10.0,1000.0\"", params)
	}
}

func TestForwardThenBackwardPointsRoundTrips(t *testing.T) {
	originalAlgorithm, originalParams := "LINEAR", "1.0,100.0,10.0,1000.0"
	pc, err := models.ForwardPowerCharacterizationPoints(&originalAlgorithm, &originalParams)
	if err != nil {
		t.Fatal(err)
	}
	algorithm, params := models.BackwardFlatFieldsPoints(pc)
	if algorithm == nil || *algorithm != originalAlgorithm {
		t.Fatalf("algorithm = %v, want %q", algorithm, originalAlgorithm)
	}
	if params == nil || *params != originalParams {
		t.Fatalf("params = %v, want %q", params, originalParams)
	}
}
