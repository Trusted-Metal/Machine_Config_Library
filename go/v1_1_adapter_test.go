// File_Version 1.1 adapter tests — the real v1.1 adapter (not
// internal/mockv1_1). Mirrors python/tests/test_v1_1_adapter.py,
// rust/tests/v1_1_adapter_test.rs, and cpp/tests/test_v1_1_adapter.cpp, made
// concrete for docs/migrations/v1_0_to_v1_1.md's 5 changes, plus the extra
// hard-error tests V1_1_IMPLEMENTATION_PLAN.md's Testing section calls for.
//
// Phase 2 (V1_1_IMPLEMENTATION_PLAN.md, clean-up phase) removed
// MigrateV1ToV1_1/MigrateV1_1ToV1 — upgrade/downgrade is now just
// Parse/Write, using MachineConfigWriter.WriteAs. Every test below that
// used to call a migrate function directly now goes through a real HDF5
// write+read instead — a genuine strengthening, since it now exercises the
// same code path a real caller uses.
package machineconfig_test

import (
	"encoding/json"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	machineconfig "machine-config-go"
	"machine-config-go/capabilities"
	v1_1hdf5 "machine-config-go/capabilities/v1_1/hdf5"
	"machine-config-go/internal/models"
)

func referenceV1_0Fixture(t *testing.T) string {
	t.Helper()
	return filepath.Join(fixturesDir(t), "reference_config_opcua_synchronous_sensors.h5")
}

func referenceV1_1Fixture(t *testing.T) string {
	t.Helper()
	return filepath.Join(fixturesDir(t), "reference_config_v1_1.h5")
}

func mockV1_0Config(t *testing.T, nLasers int) *machineconfig.MachineConfig {
	t.Helper()
	b := machineconfig.NewMockConfigBuilder()
	b.NLasers = nLasers
	return b.Build()
}

// mockV1_1Config builds an in-memory v1.1-shaped MachineConfig for tests
// that need one without touching disk — mirrors exactly what a real v1.1
// file read produces under Phase 2 (PowerCharacterization populated, flat
// fields nil), so tests exercising a writer's fallback behave the same as
// they would against real v1.1-sourced data.
func mockV1_1Config(t *testing.T, nLasers int) *machineconfig.MachineConfig {
	t.Helper()
	cfg := mockV1_0Config(t, nLasers)
	cfg.Meta.FileVersion = "1.1"
	for i := range cfg.OpticalTrains {
		train := &cfg.OpticalTrains[i]
		if cb := train.OptionalComponents.Clearbox; cb != nil {
			pc, err := models.ForwardPowerCharacterizationCoefficients(cb.VoltsToWattsAlgorithm, cb.VoltsToWattsParams)
			if err != nil {
				t.Fatal(err)
			}
			cb.PowerCharacterization = pc
			cb.VoltsToWattsAlgorithm = nil
			cb.VoltsToWattsParams = nil
		}
		lsPC, err := models.ForwardPowerCharacterizationPoints(train.LightSource.WattsToVoltsAlgorithm, train.LightSource.WattsToVoltsParams)
		if err != nil {
			t.Fatal(err)
		}
		train.LightSource.PowerCharacterization = lsPC
		train.LightSource.WattsToVoltsAlgorithm = nil
		train.LightSource.WattsToVoltsParams = nil
	}
	return cfg
}

func equationConstantNames(constants []machineconfig.EquationConstant) []string {
	names := make([]string, len(constants))
	for i, c := range constants {
		names[i] = c.Name
	}
	return names
}

func strSliceEqual(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

func floatSliceEqual(a, b []float64) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

func parseCSVToFloats(t *testing.T, s *string) []float64 {
	t.Helper()
	if s == nil {
		t.Fatal("parseCSVToFloats: nil string")
	}
	str := strings.Trim(strings.TrimSpace(*s), "[]")
	str = strings.TrimSpace(str)
	if str == "" {
		return nil
	}
	parts := strings.Split(str, ",")
	out := make([]float64, 0, len(parts))
	for _, p := range parts {
		p = strings.TrimSpace(p)
		if p == "" {
			continue
		}
		v, err := strconv.ParseFloat(p, 64)
		if err != nil {
			t.Fatalf("parseCSVToFloats: %v", err)
		}
		out = append(out, v)
	}
	return out
}

func writeAs(t *testing.T, cfg *machineconfig.MachineConfig, path string, targetVersion string) {
	t.Helper()
	if err := machineconfig.NewWriter().WriteAs(cfg, path, targetVersion); err != nil {
		t.Fatal(err)
	}
}

func parseFile(t *testing.T, path string) *machineconfig.MachineConfig {
	t.Helper()
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	return cfg
}

// ---------------------------------------------------------------------------
// TestV1_1AdapterRead
// ---------------------------------------------------------------------------

func TestV1_1AdapterRead(t *testing.T) {
	cfg, err := v1_1hdf5.Parse(referenceV1_1Fixture(t), true)
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Meta.FileVersion != "1.1" {
		t.Fatalf("file_version = %q", cfg.Meta.FileVersion)
	}

	for _, train := range cfg.OpticalTrains {
		cb := train.OptionalComponents.Clearbox
		if cb == nil {
			t.Fatal("expected clearbox present on every train")
		}
		// Change 1: Consolidate — same shared value on every train.
		if cb.OutputPath == nil || *cb.OutputPath != "/recordings/" {
			t.Fatalf("output_path = %v", cb.OutputPath)
		}
		if cb.SoftwareTriggerDelay == nil || *cb.SoftwareTriggerDelay != 3000 {
			t.Fatalf("software_trigger_delay = %v", cb.SoftwareTriggerDelay)
		}
		// Change 1: Addition.
		if cb.FirmwareVersion == nil || *cb.FirmwareVersion != "2.4.1" {
			t.Fatalf("firmware_version = %v", cb.FirmwareVersion)
		}
		// Change 1: Removal.
		if cb.SelectedCamera != nil || cb.CustomVideoFormat != nil || cb.VideoOutput != nil ||
			cb.ShowConsole != nil || cb.CorrectionGridDomainShape != nil || cb.InverseGridDomainShape != nil {
			t.Fatalf("expected all removed ClearBox fields nil, got %+v", cb)
		}
		// Change 3: superseded — v1.1's reader reads only its own native
		// shape (Phase 2 does not change this); volts_to_watts_* stay nil
		// unless/until this model is written as v1.0.
		if cb.VoltsToWattsAlgorithm != nil || cb.VoltsToWattsParams != nil {
			t.Fatalf("expected volts_to_watts_* nil, got %v / %v", cb.VoltsToWattsAlgorithm, cb.VoltsToWattsParams)
		}

		// Change 5: no on-disk source in v1.1, always nil.
		if train.Scanner.XAxis.TuningParameters != nil || train.Scanner.XAxis.TuningType != nil {
			t.Fatal("expected x_axis tuning_parameters/tuning_type nil")
		}
		if train.Scanner.YAxis.TuningParameters != nil || train.Scanner.YAxis.TuningType != nil {
			t.Fatal("expected y_axis tuning_parameters/tuning_type nil")
		}

		// Change 4: superseded, same reasoning as Change 3.
		if train.LightSource.WattsToVoltsAlgorithm != nil || train.LightSource.WattsToVoltsParams != nil {
			t.Fatal("expected light_source watts_to_volts_* nil")
		}
	}

	// Change 2: OPCUA relocated, contents unaffected.
	if cfg.Opcua == nil || cfg.Opcua.Client.MachineProfile == nil {
		t.Fatal("expected opcua present with machine_profile populated")
	}

	// Change 3: ClearBox Power_Characterization — train 1 LINEAR, train 2 POLYNOMIAL.
	cb0 := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	pc0 := cb0.PowerCharacterization
	if pc0 == nil {
		t.Fatal("expected pc0 present")
	}
	if pc0.AlgorithmType == nil || *pc0.AlgorithmType != "LINEAR" {
		t.Fatalf("pc0.algorithm_type = %v", pc0.AlgorithmType)
	}
	if pc0.AlgorithmEquation == nil || *pc0.AlgorithmEquation != "W = a*V + b" {
		t.Fatalf("pc0.algorithm_equation = %v", pc0.AlgorithmEquation)
	}
	if pc0.InputType == nil || *pc0.InputType != "0-10 V" {
		t.Fatalf("pc0.input_type = %v", pc0.InputType)
	}
	if pc0.UnitsDerivedQuantity == nil || *pc0.UnitsDerivedQuantity != "Watts" {
		t.Fatalf("pc0.units_derived_quantity = %v", pc0.UnitsDerivedQuantity)
	}
	if !strSliceEqual(equationConstantNames(pc0.DerivationEquationConstants), []string{"b", "a"}) {
		t.Fatalf("pc0 constant names = %v", equationConstantNames(pc0.DerivationEquationConstants))
	}
	if len(pc0.CharacterizationPoints) != 3 {
		t.Fatalf("pc0 characterization_points len = %d, want 3", len(pc0.CharacterizationPoints))
	}

	cb1 := cfg.OpticalTrains[1].OptionalComponents.Clearbox
	pc1 := cb1.PowerCharacterization
	if pc1 == nil {
		t.Fatal("expected pc1 present")
	}
	if pc1.AlgorithmType == nil || *pc1.AlgorithmType != "POLYNOMIAL" {
		t.Fatalf("pc1.algorithm_type = %v", pc1.AlgorithmType)
	}
	if pc1.AlgorithmEquation == nil || *pc1.AlgorithmEquation != "W = c0 + c1*V + c2*V^2" {
		t.Fatalf("pc1.algorithm_equation = %v", pc1.AlgorithmEquation)
	}
	if !strSliceEqual(equationConstantNames(pc1.DerivationEquationConstants), []string{"c0", "c1", "c2"}) {
		t.Fatalf("pc1 constant names = %v", equationConstantNames(pc1.DerivationEquationConstants))
	}

	// Change 4: LightSource Power_Characterization — inverse data availability.
	lspc0 := cfg.OpticalTrains[0].LightSource.PowerCharacterization
	if lspc0 == nil {
		t.Fatal("expected lspc0 present")
	}
	if lspc0.AlgorithmType == nil || *lspc0.AlgorithmType != "LINEAR" {
		t.Fatalf("lspc0.algorithm_type = %v", lspc0.AlgorithmType)
	}
	if lspc0.InputType == nil || *lspc0.InputType != "Volts" {
		t.Fatalf("lspc0.input_type = %v", lspc0.InputType)
	}
	if len(lspc0.CharacterizationPoints) != 5 {
		t.Fatalf("lspc0 characterization_points len = %d, want 5", len(lspc0.CharacterizationPoints))
	}
	if len(lspc0.DerivationEquationConstants) == 0 {
		t.Fatal("expected lspc0 derivation_equation_constants non-empty (natively-authored fixture)")
	}

	// JSON-serialization verification (Go's encoding/json is reflection-
	// driven via struct tags, not hand-written per-field like Python's/
	// C++'s facades — but verify directly rather than assuming the tags
	// are enough): firmware_version and power_characterization must
	// actually appear in the marshaled output.
	b, err := json.Marshal(cfg)
	if err != nil {
		t.Fatal(err)
	}
	var top map[string]json.RawMessage
	if err := json.Unmarshal(b, &top); err != nil {
		t.Fatal(err)
	}
	var trains []json.RawMessage
	if err := json.Unmarshal(top["optical_trains"], &trains); err != nil {
		t.Fatal(err)
	}
	var train0 map[string]json.RawMessage
	if err := json.Unmarshal(trains[0], &train0); err != nil {
		t.Fatal(err)
	}
	var oc map[string]json.RawMessage
	if err := json.Unmarshal(train0["optional_components"], &oc); err != nil {
		t.Fatal(err)
	}
	var cbJSON map[string]json.RawMessage
	if err := json.Unmarshal(oc["clearbox"], &cbJSON); err != nil {
		t.Fatal(err)
	}
	if _, ok := cbJSON["firmware_version"]; !ok {
		t.Error("expected clearbox JSON to contain \"firmware_version\"")
	}
	if _, ok := cbJSON["power_characterization"]; !ok {
		t.Error("expected clearbox JSON to contain \"power_characterization\"")
	}
	var lsJSON map[string]json.RawMessage
	if err := json.Unmarshal(train0["light_source"], &lsJSON); err != nil {
		t.Fatal(err)
	}
	if _, ok := lsJSON["power_characterization"]; !ok {
		t.Error("expected light_source JSON to contain \"power_characterization\"")
	}
}

// ---------------------------------------------------------------------------
// TestV1_1AdapterRoundtrip
// ---------------------------------------------------------------------------

func TestV1_1AdapterRoundtrip(t *testing.T) {
	// write -> read -> write -> read, no drift. ClearBox's and Light_Source's
	// power_characterization are deliberately given different values in the
	// same object — the direct test for the cross-wiring risk (same struct,
	// two HDF5 paths per train; a swapped read/write assignment would only
	// be caught if the two instances are actually distinguishable).
	cfg := mockV1_1Config(t, 1)
	cb := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	pc := *cb.PowerCharacterization
	pc.AlgorithmType = machineconfig.StrPtr("LINEAR")
	pc.DerivationEquationConstants = []machineconfig.EquationConstant{
		{Name: "b", Value: 1.0},
		{Name: "a", Value: 2.0},
	}
	pc.CharacterizationPoints = []machineconfig.CalibrationPoint{}
	cb.PowerCharacterization = &pc

	lsPC := *cfg.OpticalTrains[0].LightSource.PowerCharacterization
	lsPC.AlgorithmType = machineconfig.StrPtr("POLYNOMIAL")
	lsPC.DerivationEquationConstants = []machineconfig.EquationConstant{}
	lsPC.CharacterizationPoints = []machineconfig.CalibrationPoint{{InputValue: 9.0, OutputValue: 99.0}}
	cfg.OpticalTrains[0].LightSource.PowerCharacterization = &lsPC

	p1 := filepath.Join(t.TempDir(), "v1_1_pass1.h5")
	if err := v1_1hdf5.Write(cfg, p1); err != nil {
		t.Fatal(err)
	}
	mid, err := v1_1hdf5.Parse(p1, true)
	if err != nil {
		t.Fatal(err)
	}
	p2 := filepath.Join(t.TempDir(), "v1_1_pass2.h5")
	if err := v1_1hdf5.Write(mid, p2); err != nil {
		t.Fatal(err)
	}
	result, err := v1_1hdf5.Parse(p2, true)
	if err != nil {
		t.Fatal(err)
	}

	cbR := result.OpticalTrains[0].OptionalComponents.Clearbox
	lsR := result.OpticalTrains[0].LightSource
	cbPC := cbR.PowerCharacterization
	lsPCR := lsR.PowerCharacterization
	if cbPC == nil || lsPCR == nil {
		t.Fatal("expected both power_characterization instances present")
	}
	if cbPC.AlgorithmType == nil || *cbPC.AlgorithmType != "LINEAR" {
		t.Fatalf("cb power_characterization.algorithm_type = %v", cbPC.AlgorithmType)
	}
	if !strSliceEqual(equationConstantNames(cbPC.DerivationEquationConstants), []string{"b", "a"}) {
		t.Fatalf("cb constant names = %v", equationConstantNames(cbPC.DerivationEquationConstants))
	}
	if len(cbPC.CharacterizationPoints) != 0 {
		t.Fatalf("cb characterization_points should be empty, got %v", cbPC.CharacterizationPoints)
	}
	if lsPCR.AlgorithmType == nil || *lsPCR.AlgorithmType != "POLYNOMIAL" {
		t.Fatalf("ls power_characterization.algorithm_type = %v", lsPCR.AlgorithmType)
	}
	if len(lsPCR.DerivationEquationConstants) != 0 {
		t.Fatalf("ls derivation_equation_constants should be empty, got %v", lsPCR.DerivationEquationConstants)
	}
	if len(lsPCR.CharacterizationPoints) != 1 || lsPCR.CharacterizationPoints[0].InputValue != 9.0 {
		t.Fatalf("ls characterization_points = %v", lsPCR.CharacterizationPoints)
	}
	// Cross-wiring guard: the two instances must not have swapped.
	if *cbPC.AlgorithmType == *lsPCR.AlgorithmType {
		t.Fatal("cross-wiring guard failed: clearbox and light_source power_characterization have the same algorithm_type")
	}
}

// ---------------------------------------------------------------------------
// v1.0 fixture written as v1.1
// ---------------------------------------------------------------------------

func TestV1_1AdapterForwardMigration(t *testing.T) {
	source := parseFile(t, referenceV1_0Fixture(t))
	tmp := filepath.Join(t.TempDir(), "v1_1_from_v1_0.h5")
	writeAs(t, source, tmp, "1.1")
	migrated := parseFile(t, tmp)
	if migrated.Meta.FileVersion != "1.1" {
		t.Fatalf("file_version = %q", migrated.Meta.FileVersion)
	}

	for i, train := range migrated.OpticalTrains {
		srcCB := source.OpticalTrains[i].OptionalComponents.Clearbox
		cb := train.OptionalComponents.Clearbox
		if cb == nil || srcCB == nil {
			t.Fatal("expected clearbox on both source and migrated")
		}
		// Consolidate: same per-train value carried straight through (real
		// fixture already agrees).
		if (cb.OutputPath == nil) != (srcCB.OutputPath == nil) || (srcCB.OutputPath != nil && *cb.OutputPath != *srcCB.OutputPath) {
			t.Fatalf("output_path: got %v want %v", cb.OutputPath, srcCB.OutputPath)
		}
		if (cb.SoftwareTriggerDelay == nil) != (srcCB.SoftwareTriggerDelay == nil) ||
			(srcCB.SoftwareTriggerDelay != nil && *cb.SoftwareTriggerDelay != *srcCB.SoftwareTriggerDelay) {
			t.Fatalf("software_trigger_delay: got %v want %v", cb.SoftwareTriggerDelay, srcCB.SoftwareTriggerDelay)
		}
		// Addition: no v1.0 source.
		if cb.FirmwareVersion != nil {
			t.Fatalf("firmware_version = %v, want nil", cb.FirmwareVersion)
		}
		// Removal.
		if cb.SelectedCamera != nil || cb.CustomVideoFormat != nil {
			t.Fatal("expected removed fields nil")
		}
		// Change 3: derived ClearBox data has real constants, zero points.
		pc := cb.PowerCharacterization
		if pc == nil {
			t.Fatal("expected clearbox power_characterization present")
		}
		if len(pc.CharacterizationPoints) != 0 {
			t.Fatalf("expected zero characterization_points, got %d", len(pc.CharacterizationPoints))
		}
		if len(pc.DerivationEquationConstants) == 0 {
			t.Fatal("expected non-empty derivation_equation_constants")
		}
		if (pc.AlgorithmType == nil) != (srcCB.VoltsToWattsAlgorithm == nil) ||
			(srcCB.VoltsToWattsAlgorithm != nil && *pc.AlgorithmType != *srcCB.VoltsToWattsAlgorithm) {
			t.Fatalf("algorithm_type: got %v want %v", pc.AlgorithmType, srcCB.VoltsToWattsAlgorithm)
		}

		// Change 4: derived Light_Source data is the inverse — zero
		// constants, real points.
		lsPC := train.LightSource.PowerCharacterization
		if lsPC == nil {
			t.Fatal("expected light_source power_characterization present")
		}
		if len(lsPC.DerivationEquationConstants) != 0 {
			t.Fatalf("expected zero derivation_equation_constants, got %d", len(lsPC.DerivationEquationConstants))
		}
		if len(lsPC.CharacterizationPoints) == 0 {
			t.Fatal("expected non-empty characterization_points")
		}
	}
}

// ---------------------------------------------------------------------------
// Change 1 Consolidate hard-error tests
// ---------------------------------------------------------------------------

func TestV1_1AdapterOutputPathDisagreementRaises(t *testing.T) {
	cfg := mockV1_0Config(t, 2)
	cfg.OpticalTrains[1].OptionalComponents.Clearbox.OutputPath = machineconfig.StrPtr("/other/")
	tmp := filepath.Join(t.TempDir(), "conflict.h5")
	werr := machineconfig.NewWriter().WriteAs(cfg, tmp, "1.1")
	if werr == nil {
		t.Fatal("expected an error, got nil")
	}
	if !strings.Contains(werr.Error(), `consolidate conflict on "Output_Path"`) {
		t.Fatalf("unexpected message: %v", werr)
	}
}

func TestV1_1AdapterSoftwareTriggerDelayDisagreementRaises(t *testing.T) {
	cfg := mockV1_0Config(t, 2)
	cfg.OpticalTrains[1].OptionalComponents.Clearbox.SoftwareTriggerDelay = machineconfig.IntPtr(9999)
	tmp := filepath.Join(t.TempDir(), "conflict2.h5")
	werr := machineconfig.NewWriter().WriteAs(cfg, tmp, "1.1")
	if werr == nil {
		t.Fatal("expected an error, got nil")
	}
	if !strings.Contains(werr.Error(), `consolidate conflict on "Software_Trigger_Delay"`) {
		t.Fatalf("unexpected message: %v", werr)
	}
}

// ---------------------------------------------------------------------------
// Change 3/4 unrecognized-Algorithm_Type: never raises, best-effort (Phase 2)
// ---------------------------------------------------------------------------

func TestV1_1AdapterUnrecognizedAlgorithmTypeBestEffortClearbox(t *testing.T) {
	cfg := mockV1_0Config(t, 1)
	cfg.OpticalTrains[0].OptionalComponents.Clearbox.VoltsToWattsAlgorithm = machineconfig.StrPtr("EXPONENTIAL")
	cfg.OpticalTrains[0].OptionalComponents.Clearbox.VoltsToWattsParams = machineconfig.StrPtr("1.5,2.5,3.5")
	tmp := filepath.Join(t.TempDir(), "unrecognized_clearbox.h5")
	writeAs(t, cfg, tmp, "1.1")
	result := parseFile(t, tmp)
	pc := result.OpticalTrains[0].OptionalComponents.Clearbox.PowerCharacterization
	if pc == nil {
		t.Fatal("expected power_characterization present")
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

	// Round trip: writing back to v1.0 reproduces the original CSV exactly.
	v1_0Out := filepath.Join(t.TempDir(), "unrecognized_clearbox_roundtrip.h5")
	writeAs(t, result, v1_0Out, "1.0")
	back := parseFile(t, v1_0Out)
	backCB := back.OpticalTrains[0].OptionalComponents.Clearbox
	if backCB.VoltsToWattsParams == nil || *backCB.VoltsToWattsParams != "1.5,2.5,3.5" {
		t.Fatalf("volts_to_watts_params = %v, want \"1.5,2.5,3.5\"", backCB.VoltsToWattsParams)
	}
}

func TestV1_1AdapterUnrecognizedAlgorithmTypeBestEffortLightSource(t *testing.T) {
	cfg := mockV1_0Config(t, 1)
	cfg.OpticalTrains[0].LightSource.WattsToVoltsAlgorithm = machineconfig.StrPtr("QUADRATIC")
	cfg.OpticalTrains[0].LightSource.WattsToVoltsParams = machineconfig.StrPtr("1,2,3,4")
	tmp := filepath.Join(t.TempDir(), "unrecognized_lightsource.h5")
	writeAs(t, cfg, tmp, "1.1")
	result := parseFile(t, tmp)
	pc := result.OpticalTrains[0].LightSource.PowerCharacterization
	if pc == nil {
		t.Fatal("expected power_characterization present")
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

// ---------------------------------------------------------------------------
// v1.1 fixture written as v1.0
// ---------------------------------------------------------------------------

func TestV1_1AdapterBackwardMigration(t *testing.T) {
	source := parseFile(t, referenceV1_1Fixture(t))
	tmp := filepath.Join(t.TempDir(), "v1_0_from_v1_1.h5")
	writeAs(t, source, tmp, "1.0")
	back := parseFile(t, tmp)
	if back.Meta.FileVersion != "1.0" {
		t.Fatalf("file_version = %q", back.Meta.FileVersion)
	}

	srcCB0 := source.OpticalTrains[0].OptionalComponents.Clearbox
	cb0 := back.OpticalTrains[0].OptionalComponents.Clearbox
	srcPC0 := srcCB0.PowerCharacterization
	if cb0.VoltsToWattsAlgorithm == nil || srcPC0.AlgorithmType == nil || *cb0.VoltsToWattsAlgorithm != *srcPC0.AlgorithmType {
		t.Fatalf("volts_to_watts_algorithm: got %v want %v", cb0.VoltsToWattsAlgorithm, srcPC0.AlgorithmType)
	}
	expected := make([]float64, len(srcPC0.DerivationEquationConstants))
	for i, c := range srcPC0.DerivationEquationConstants {
		expected[i] = c.Value
	}
	actual := parseCSVToFloats(t, cb0.VoltsToWattsParams)
	if len(actual) != len(expected) {
		t.Fatalf("volts_to_watts_params count: got %v want %v", actual, expected)
	}

	if (cb0.OutputPath == nil) != (srcCB0.OutputPath == nil) || (srcCB0.OutputPath != nil && *cb0.OutputPath != *srcCB0.OutputPath) {
		t.Fatalf("output_path: got %v want %v", cb0.OutputPath, srcCB0.OutputPath)
	}
	if (cb0.SoftwareTriggerDelay == nil) != (srcCB0.SoftwareTriggerDelay == nil) ||
		(srcCB0.SoftwareTriggerDelay != nil && *cb0.SoftwareTriggerDelay != *srcCB0.SoftwareTriggerDelay) {
		t.Fatalf("software_trigger_delay: got %v want %v", cb0.SoftwareTriggerDelay, srcCB0.SoftwareTriggerDelay)
	}

	srcLS0 := source.OpticalTrains[0].LightSource
	ls0 := back.OpticalTrains[0].LightSource
	srcLsPC0 := srcLS0.PowerCharacterization
	if ls0.WattsToVoltsAlgorithm == nil || srcLsPC0.AlgorithmType == nil || *ls0.WattsToVoltsAlgorithm != *srcLsPC0.AlgorithmType {
		t.Fatalf("watts_to_volts_algorithm: got %v want %v", ls0.WattsToVoltsAlgorithm, srcLsPC0.AlgorithmType)
	}
	expectedPoints := make([]float64, 0, len(srcLsPC0.CharacterizationPoints)*2)
	for _, p := range srcLsPC0.CharacterizationPoints {
		expectedPoints = append(expectedPoints, p.InputValue, p.OutputValue)
	}
	actualPoints := parseCSVToFloats(t, ls0.WattsToVoltsParams)
	if !floatSliceEqual(actualPoints, expectedPoints) {
		t.Fatalf("watts_to_volts_params: got %v want %v", actualPoints, expectedPoints)
	}

	// Change 1 Removal fields: lost forever, not restored.
	if cb0.SelectedCamera != nil {
		t.Fatalf("selected_camera = %v, want nil", cb0.SelectedCamera)
	}
}

func TestV1_1AdapterBackwardMigrationReordersConstantsByName(t *testing.T) {
	// Change 3 backward: Derivation_Equation_Constants rows aren't
	// positionally guaranteed — must re-sort by name before joining as CSV.
	cfg := mockV1_1Config(t, 1)
	cb0 := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	pc := *cb0.PowerCharacterization
	pc.AlgorithmType = machineconfig.StrPtr("LINEAR")
	pc.DerivationEquationConstants = []machineconfig.EquationConstant{
		{Name: "a", Value: 2.0},
		{Name: "b", Value: 1.0},
	}
	cb0.PowerCharacterization = &pc

	tmp := filepath.Join(t.TempDir(), "backward_reorders.h5")
	writeAs(t, cfg, tmp, "1.0")
	back := parseFile(t, tmp)
	backCB0 := back.OpticalTrains[0].OptionalComponents.Clearbox
	if backCB0.VoltsToWattsParams == nil || *backCB0.VoltsToWattsParams != "1.0,2.0" {
		t.Fatalf("volts_to_watts_params = %v, want \"1.0,2.0\"", backCB0.VoltsToWattsParams)
	}
}

func TestV1_1AdapterBackwardMigrationMissingDerivationConstantsWritesBlank(t *testing.T) {
	cfg := mockV1_1Config(t, 1)
	cb0 := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	pc := *cb0.PowerCharacterization
	pc.DerivationEquationConstants = []machineconfig.EquationConstant{}
	cb0.PowerCharacterization = &pc

	tmp := filepath.Join(t.TempDir(), "backward_blank_constants.h5")
	writeAs(t, cfg, tmp, "1.0")
	back := parseFile(t, tmp)
	backCB0 := back.OpticalTrains[0].OptionalComponents.Clearbox
	// The writer's fallback produces "" (blank, not an error) — but a real
	// disk round-trip normalizes an empty string attribute back to nil on
	// read, this codebase's standard convention for absent optional
	// strings (confirmed against Python's/Rust's/C++'s identical finding).
	if backCB0.VoltsToWattsParams != nil {
		t.Fatalf("volts_to_watts_params = %v, want nil", *backCB0.VoltsToWattsParams)
	}
}

func TestV1_1AdapterBackwardMigrationMissingCharacterizationPointsWritesBlank(t *testing.T) {
	cfg := mockV1_1Config(t, 1)
	ls0 := &cfg.OpticalTrains[0].LightSource
	pc := *ls0.PowerCharacterization
	pc.CharacterizationPoints = []machineconfig.CalibrationPoint{}
	ls0.PowerCharacterization = &pc

	tmp := filepath.Join(t.TempDir(), "backward_blank_points.h5")
	writeAs(t, cfg, tmp, "1.0")
	back := parseFile(t, tmp)
	backLS0 := back.OpticalTrains[0].LightSource
	if backLS0.WattsToVoltsParams != nil {
		t.Fatalf("watts_to_volts_params = %v, want nil", *backLS0.WattsToVoltsParams)
	}
}

// ---------------------------------------------------------------------------
// TestV1_1AdapterV1Unaffected
// ---------------------------------------------------------------------------

func TestV1_1AdapterV1Unaffected(t *testing.T) {
	// Existing v1.0 read path is undisturbed by v1.1's existence — Phase 2
	// does not touch either reader at all (Option C: derivation lives only
	// in the writers' fallbacks).
	cfg := parseFile(t, referenceV1_0Fixture(t))
	if cfg.Meta.FileVersion != "1.0" {
		t.Fatalf("file_version = %q", cfg.Meta.FileVersion)
	}
	if len(cfg.OpticalTrains) == 0 {
		t.Fatal("expected optical trains")
	}
	cb0 := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	if cb0 == nil || cb0.VoltsToWattsAlgorithm == nil || *cb0.VoltsToWattsAlgorithm != "LINEAR" {
		t.Fatalf("volts_to_watts_algorithm = %v", cb0)
	}
	if cb0.PowerCharacterization != nil {
		t.Fatalf("power_characterization = %v, want nil", cb0.PowerCharacterization)
	}
}

func TestV1_1AdapterV1_0WriteNeverInvokesFallbackWhenNativePresent(t *testing.T) {
	// The writer's fallback must never fire when the native field is
	// already present — a deliberately odd-but-valid string must pass
	// through byte-for-byte, proving v1_0/hdf5.Write never reformats via
	// PowerCharacterization when there's nothing for the fallback to do.
	cfg := mockV1_0Config(t, 1)
	cfg.OpticalTrains[0].OptionalComponents.Clearbox.VoltsToWattsParams = machineconfig.StrPtr("50.50,107.500")
	tmp := filepath.Join(t.TempDir(), "v1_0_verbatim.h5")
	if err := machineconfig.NewWriter().Write(cfg, tmp); err != nil {
		t.Fatal(err)
	}
	result := parseFile(t, tmp)
	cb := result.OpticalTrains[0].OptionalComponents.Clearbox
	if cb.VoltsToWattsParams == nil || *cb.VoltsToWattsParams != "50.50,107.500" {
		t.Fatalf("volts_to_watts_params = %v, want \"50.50,107.500\"", cb.VoltsToWattsParams)
	}
}

// ---------------------------------------------------------------------------
// Round-trip tests — the acceptance criterion stated 2026-09-01, made concrete
// ---------------------------------------------------------------------------

func TestV1_1AdapterRoundtripV1_0ToV1_1ToV1_0(t *testing.T) {
	source := parseFile(t, referenceV1_0Fixture(t))
	v1_1Path := filepath.Join(t.TempDir(), "up.h5")
	writeAs(t, source, v1_1Path, "1.1")
	mid := parseFile(t, v1_1Path)
	v1_0Path := filepath.Join(t.TempDir(), "down.h5")
	writeAs(t, mid, v1_0Path, "1.0")
	back := parseFile(t, v1_0Path)

	if back.Meta.FileVersion != "1.0" {
		t.Fatalf("file_version = %q", back.Meta.FileVersion)
	}
	for i, train := range back.OpticalTrains {
		srcCB := source.OpticalTrains[i].OptionalComponents.Clearbox
		cb := train.OptionalComponents.Clearbox
		if (cb.OutputPath == nil) != (srcCB.OutputPath == nil) || (srcCB.OutputPath != nil && *cb.OutputPath != *srcCB.OutputPath) {
			t.Fatalf("output_path: got %v want %v", cb.OutputPath, srcCB.OutputPath)
		}
		if (cb.SoftwareTriggerDelay == nil) != (srcCB.SoftwareTriggerDelay == nil) ||
			(srcCB.SoftwareTriggerDelay != nil && *cb.SoftwareTriggerDelay != *srcCB.SoftwareTriggerDelay) {
			t.Fatalf("software_trigger_delay: got %v want %v", cb.SoftwareTriggerDelay, srcCB.SoftwareTriggerDelay)
		}
		if (cb.VoltsToWattsAlgorithm == nil) != (srcCB.VoltsToWattsAlgorithm == nil) ||
			(srcCB.VoltsToWattsAlgorithm != nil && *cb.VoltsToWattsAlgorithm != *srcCB.VoltsToWattsAlgorithm) {
			t.Fatalf("volts_to_watts_algorithm: got %v want %v", cb.VoltsToWattsAlgorithm, srcCB.VoltsToWattsAlgorithm)
		}
		if !floatSliceEqual(parseCSVToFloats(t, cb.VoltsToWattsParams), parseCSVToFloats(t, srcCB.VoltsToWattsParams)) {
			t.Fatalf("volts_to_watts_params: got %v want %v", cb.VoltsToWattsParams, srcCB.VoltsToWattsParams)
		}

		srcLS := source.OpticalTrains[i].LightSource
		ls := train.LightSource
		if (ls.WattsToVoltsAlgorithm == nil) != (srcLS.WattsToVoltsAlgorithm == nil) ||
			(srcLS.WattsToVoltsAlgorithm != nil && *ls.WattsToVoltsAlgorithm != *srcLS.WattsToVoltsAlgorithm) {
			t.Fatalf("watts_to_volts_algorithm: got %v want %v", ls.WattsToVoltsAlgorithm, srcLS.WattsToVoltsAlgorithm)
		}
		if !floatSliceEqual(parseCSVToFloats(t, ls.WattsToVoltsParams), parseCSVToFloats(t, srcLS.WattsToVoltsParams)) {
			t.Fatalf("watts_to_volts_params: got %v want %v", ls.WattsToVoltsParams, srcLS.WattsToVoltsParams)
		}
	}
}

func TestV1_1AdapterRoundtripV1_1ToV1_0ToV1_1(t *testing.T) {
	// Mirror direction: starting from a natively-authored v1.1 file,
	// downgrade then upgrade again. Fields v1.0 can represent survive;
	// fields only v1.1 can hold come back blank — expected, documented
	// richer-to-simpler loss, not a bug.
	source := parseFile(t, referenceV1_1Fixture(t))
	v1_0Path := filepath.Join(t.TempDir(), "mirror_down.h5")
	writeAs(t, source, v1_0Path, "1.0")
	mid := parseFile(t, v1_0Path)
	v1_1Path := filepath.Join(t.TempDir(), "mirror_up.h5")
	writeAs(t, mid, v1_1Path, "1.1")
	back := parseFile(t, v1_1Path)

	if back.Meta.FileVersion != "1.1" {
		t.Fatalf("file_version = %q", back.Meta.FileVersion)
	}
	for i, train := range back.OpticalTrains {
		srcCB := source.OpticalTrains[i].OptionalComponents.Clearbox
		cb := train.OptionalComponents.Clearbox
		srcPC := srcCB.PowerCharacterization
		pc := cb.PowerCharacterization
		if (pc.AlgorithmType == nil) != (srcPC.AlgorithmType == nil) || (srcPC.AlgorithmType != nil && *pc.AlgorithmType != *srcPC.AlgorithmType) {
			t.Fatalf("algorithm_type: got %v want %v", pc.AlgorithmType, srcPC.AlgorithmType)
		}
		if len(pc.DerivationEquationConstants) != len(srcPC.DerivationEquationConstants) {
			t.Fatalf("derivation_equation_constants count: got %d want %d",
				len(pc.DerivationEquationConstants), len(srcPC.DerivationEquationConstants))
		}
		// Expected loss: v1.1-only fields have no v1.0 round-trip path.
		if cb.FirmwareVersion != nil {
			t.Fatalf("firmware_version = %v, want nil", cb.FirmwareVersion)
		}
		if pc.InputType != nil {
			t.Fatalf("input_type = %v, want nil", pc.InputType)
		}
		if pc.UnitsDerivedQuantity != nil {
			t.Fatalf("units_derived_quantity = %v, want nil", pc.UnitsDerivedQuantity)
		}
		if len(pc.CharacterizationPoints) != 0 {
			t.Fatalf("characterization_points = %v, want empty", pc.CharacterizationPoints)
		}
	}
}

// ---------------------------------------------------------------------------
// Writer WriteAs (targetVersion)
// ---------------------------------------------------------------------------

func TestV1_1AdapterWriterTargetVersionOverridesMeta(t *testing.T) {
	source := parseFile(t, referenceV1_0Fixture(t))
	if source.Meta.FileVersion != "1.0" {
		t.Fatalf("file_version = %q", source.Meta.FileVersion)
	}
	out := filepath.Join(t.TempDir(), "overridden_up.h5")
	writeAs(t, source, out, "1.1")
	result := parseFile(t, out)
	if result.Meta.FileVersion != "1.1" {
		t.Fatalf("file_version = %q", result.Meta.FileVersion)
	}
	// Writer must not mutate its input.
	if source.Meta.FileVersion != "1.0" {
		t.Fatalf("source.Meta.FileVersion mutated to %q", source.Meta.FileVersion)
	}

	v1_1Source := parseFile(t, referenceV1_1Fixture(t))
	if v1_1Source.Meta.FileVersion != "1.1" {
		t.Fatalf("file_version = %q", v1_1Source.Meta.FileVersion)
	}
	out2 := filepath.Join(t.TempDir(), "overridden_down.h5")
	writeAs(t, v1_1Source, out2, "1.0")
	result2 := parseFile(t, out2)
	if result2.Meta.FileVersion != "1.0" {
		t.Fatalf("file_version = %q", result2.Meta.FileVersion)
	}
	if v1_1Source.Meta.FileVersion != "1.1" {
		t.Fatalf("v1_1Source.Meta.FileVersion mutated to %q", v1_1Source.Meta.FileVersion)
	}
}

func TestV1_1AdapterWriterDefaultsToMetaFileVersion(t *testing.T) {
	source := parseFile(t, referenceV1_0Fixture(t))
	out := filepath.Join(t.TempDir(), "default.h5")
	if err := machineconfig.NewWriter().Write(source, out); err != nil { // no targetVersion override
		t.Fatal(err)
	}
	result := parseFile(t, out)
	if result.Meta.FileVersion != "1.0" {
		t.Fatalf("file_version = %q", result.Meta.FileVersion)
	}
}

// ---------------------------------------------------------------------------
// Dispatcher-level tests — full public API via the real "1.1" registry entry.
// ---------------------------------------------------------------------------

func TestV1_1AdapterDispatcherForward(t *testing.T) {
	source := parseFile(t, referenceV1_0Fixture(t))
	tmp := filepath.Join(t.TempDir(), "dispatcher_fwd.h5")
	writeAs(t, source, tmp, "1.1")
	result := parseFile(t, tmp)
	if result.Meta.FileVersion != "1.1" {
		t.Fatalf("file_version = %q", result.Meta.FileVersion)
	}
	cb := result.OpticalTrains[0].OptionalComponents.Clearbox
	if cb == nil || cb.OutputPath == nil || *cb.OutputPath != "/recordings/" {
		t.Fatalf("output_path = %v", cb)
	}
	if cb.PowerCharacterization == nil {
		t.Fatal("expected power_characterization present")
	}

	// Facade dispatcher too.
	f, capErr := capabilities.OpenMachineConfig(tmp)
	if capErr != nil {
		t.Fatal(capErr)
	}
	defer f.Close()
	if f.FileVersion() != "1.1" {
		t.Fatalf("facade file_version = %q", f.FileVersion())
	}
}

func TestV1_1AdapterDispatcherBackward(t *testing.T) {
	v1_1Cfg := parseFile(t, referenceV1_1Fixture(t))
	tmp := filepath.Join(t.TempDir(), "dispatcher_back.h5")
	writeAs(t, v1_1Cfg, tmp, "1.0")
	result := parseFile(t, tmp)
	if result.Meta.FileVersion != "1.0" {
		t.Fatalf("file_version = %q", result.Meta.FileVersion)
	}
	cb := result.OpticalTrains[0].OptionalComponents.Clearbox
	if cb == nil || cb.VoltsToWattsAlgorithm == nil || *cb.VoltsToWattsAlgorithm != "LINEAR" {
		t.Fatalf("volts_to_watts_algorithm = %v", cb)
	}
}

// ---------------------------------------------------------------------------
// TestV1_1AdapterSatisfiesRegistryProtocol
// ---------------------------------------------------------------------------

// In Go, ReaderRegistry/WriterRegistry are typed as
// map[string]func(path string) ReaderAdapter / map[string]WriterAdapter —
// so a v1.1 entry satisfying ReaderAdapter/WriterAdapter is primarily a
// compile-time fact enforced by reader.go's/writer.go's own type
// signatures (the package would not compile otherwise), the same framing
// Rust's equivalent test uses. This test makes it a concrete,
// runtime-checked fact too: the production registries (exercised via the
// public API, since the registries themselves are unexported) actually
// dispatch "1.1" to a working reader/writer pair, and SupportedFileVersions
// reports it.
func TestV1_1AdapterSatisfiesRegistryProtocol(t *testing.T) {
	versions := capabilities.SupportedFileVersions()
	found := false
	for _, v := range versions {
		if v == "1.1" {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected \"1.1\" in SupportedFileVersions(), got %v", versions)
	}

	cfg := mockV1_1Config(t, 1)
	tmp := filepath.Join(t.TempDir(), "protocol.h5")
	if err := machineconfig.NewWriter().Write(cfg, tmp); err != nil {
		t.Fatal(err)
	}
	result, err := machineconfig.NewReader(tmp).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if result.Meta.FileVersion != "1.1" {
		t.Fatalf("file_version = %q", result.Meta.FileVersion)
	}
	if _, err := machineconfig.NewReader(tmp).GetCorrectionData(0); err != nil {
		t.Fatal(err)
	}
	if _, err := machineconfig.NewReader(tmp).GetInverseCorrectionData(0); err != nil {
		t.Fatal(err)
	}
}
