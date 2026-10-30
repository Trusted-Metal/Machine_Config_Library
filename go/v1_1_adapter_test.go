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
	"strings"
	"testing"

	machineconfig "machine-config-go"
	"machine-config-go/capabilities"
	v1_1hdf5 "machine-config-go/capabilities/v1_1/hdf5"
	"machine-config-go/extension"
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
// that need one without touching disk — MockConfigBuilder already builds
// PowerCharacterization natively, so this is just the v1.0 mock with
// FileVersion overridden.
func mockV1_1Config(t *testing.T, nLasers int) *machineconfig.MachineConfig {
	t.Helper()
	cfg := mockV1_0Config(t, nLasers)
	cfg.Meta.FileVersion = "1.1"
	return cfg
}

// constantValues/pointValues flatten a PowerCharacterization's derived data
// to plain float slices, in on-disk order — for comparing derived data
// without caring about its name/label.
func constantValues(pc *machineconfig.PowerCharacterization) []float64 {
	out := make([]float64, len(pc.DerivationEquationConstants))
	for i, c := range pc.DerivationEquationConstants {
		out[i] = c.Value
	}
	return out
}

func pointValues(pc *machineconfig.PowerCharacterization) []float64 {
	out := make([]float64, 0, len(pc.CharacterizationPoints)*2)
	for _, p := range pc.CharacterizationPoints {
		out = append(out, p.InputValue, p.OutputValue)
	}
	return out
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

func cbNames(pc *extension.Tree) []string {
	var out []string
	for _, c := range cbConstants(pc) {
		out = append(out, c.Name)
	}
	return out
}

func cbValues(pc *extension.Tree) []float64 {
	var out []float64
	for _, c := range cbConstants(pc) {
		out = append(out, c.Value)
	}
	return out
}

func TestV1_1AdapterRead(t *testing.T) {
	cfg, err := v1_1hdf5.Parse(referenceV1_1Fixture(t), true)
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Meta.FileVersion != "1.1" {
		t.Fatalf("file_version = %q", cfg.Meta.FileVersion)
	}

	// Change 1: Consolidate — one shared value, on the extension root.
	root := clearboxRoot(t, cfg)
	if got := cbText(root, "Output_Path"); got != "/recordings/" {
		t.Fatalf("output_path = %q", got)
	}
	if got, ok := cbInt(root, "Software_Trigger_Delay"); !ok || got != 3000 {
		t.Fatalf("software_trigger_delay = %v", got)
	}

	for i, train := range cfg.OpticalTrains {
		cb := clearboxTrain(t, cfg, i)
		// Change 1: Addition.
		if got := cbText(cb, "Firmware_Version"); got != "2.4.1" {
			t.Fatalf("firmware_version = %q", got)
		}
		// Change 1: Removal.
		for _, removed := range []string{"Selected_Camera", "Custom_Video_Format", "Video_Output", "Show_Console", "Output_Path", "Software_Trigger_Delay"} {
			if _, ok := cb.Attrs[removed]; ok {
				t.Fatalf("expected %s absent from a v1.1 ClearBox train", removed)
			}
		}
		// Change 5: no on-disk source in v1.1, always nil.
		if train.Scanner.XAxis.TuningParameters != nil || train.Scanner.XAxis.TuningType != nil {
			t.Fatal("expected x_axis tuning_parameters/tuning_type nil")
		}
		if train.Scanner.YAxis.TuningParameters != nil || train.Scanner.YAxis.TuningType != nil {
			t.Fatal("expected y_axis tuning_parameters/tuning_type nil")
		}
	}

	// Change 2: the legacy OPCUA group is carried as the opaque "TM_OPCUA" extension, contents unaffected.
	opcua := cfg.Extensions["TM_OPCUA"]
	if opcua == nil || opcua.Children["Client"] == nil {
		t.Fatal("expected the TM_OPCUA extension with a Client group")
	}
	if _, ok := opcua.Children["Client"].Attrs["Machine_Profile"]; !ok {
		t.Fatal("expected Client/Machine_Profile to survive the migration")
	}

	// Change 3: ClearBox Power_Characterization — train 1 LINEAR, train 2 POLYNOMIAL.
	pc0 := cbPowerCharacterization(t, cfg, 0)
	for field, want := range map[string]string{
		"Algorithm_Type":         "LINEAR",
		"Algorithm_Equation":     "W = a*V + b",
		"Input_Type":             "0-10 V",
		"Units_Derived_Quantity": "Watts",
	} {
		if got := cbText(pc0, field); got != want {
			t.Fatalf("pc0.%s = %q, want %q", field, got, want)
		}
	}
	if !strSliceEqual(cbNames(pc0), []string{"b", "a"}) {
		t.Fatalf("pc0 constant names = %v", cbNames(pc0))
	}
	if cbPointCount(pc0) != 3 {
		t.Fatalf("pc0 characterization_points = %d, want 3", cbPointCount(pc0))
	}

	pc1 := cbPowerCharacterization(t, cfg, 1)
	if got := cbText(pc1, "Algorithm_Type"); got != "POLYNOMIAL" {
		t.Fatalf("pc1.algorithm_type = %q", got)
	}
	if got := cbText(pc1, "Algorithm_Equation"); got != "W = c0 + c1*V + c2*V^2" {
		t.Fatalf("pc1.algorithm_equation = %q", got)
	}
	if !strSliceEqual(cbNames(pc1), []string{"c0", "c1", "c2"}) {
		t.Fatalf("pc1 constant names = %v", cbNames(pc1))
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

	// JSON-serialization verification: the ClearBox extension tree and the light source's
	// power_characterization must actually appear in the marshaled output.
	b, err := json.Marshal(cfg)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(b), `"Firmware_Version"`) {
		t.Error("expected JSON to contain the ClearBox \"Firmware_Version\" attribute")
	}
	if !strings.Contains(string(b), `"power_characterization"`) {
		t.Error("expected JSON to contain the light_source \"power_characterization\"")
	}
}

// ---------------------------------------------------------------------------
// TestV1_1AdapterRoundtrip
// ---------------------------------------------------------------------------

func TestV1_1AdapterRoundtrip(t *testing.T) {
	// write -> read -> write -> read, no drift. ClearBox's and Light_Source's
	// power_characterization are deliberately given different values in the
	// same object — the direct test for the cross-wiring risk.
	cfg := mockV1_1Config(t, 1)
	clearboxTrain(t, cfg, 0).Children["Power_Characterization"] = cbMakePowerCharacterization(
		"LINEAR", []cbConstant{{"b", 1.0}, {"a", 2.0}})

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

	cbPC := cbPowerCharacterization(t, result, 0)
	lsPCR := result.OpticalTrains[0].LightSource.PowerCharacterization
	if lsPCR == nil {
		t.Fatal("expected light source power_characterization present")
	}
	if got := cbText(cbPC, "Algorithm_Type"); got != "LINEAR" {
		t.Fatalf("cb power_characterization.algorithm_type = %q", got)
	}
	if !strSliceEqual(cbNames(cbPC), []string{"b", "a"}) {
		t.Fatalf("cb constant names = %v", cbNames(cbPC))
	}
	if cbPointCount(cbPC) != 0 {
		t.Fatalf("cb characterization_points should be empty, got %d", cbPointCount(cbPC))
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
	if cbText(cbPC, "Algorithm_Type") == *lsPCR.AlgorithmType {
		t.Fatal("cross-wiring guard failed: clearbox and light_source power_characterization have the same algorithm_type")
	}
	// The whole ClearBox tree, grids included, is bit-stable across the double round trip.
	want, _ := extension.Digest(cfg.Extensions["ClearBox"])
	got, _ := extension.Digest(result.Extensions["ClearBox"])
	if got != want {
		t.Fatalf("ClearBox digest drifted across a v1.1 round trip: %s != %s", got, want)
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

	// The shimmed tree is already in the v1.1 layout, so it survives unchanged.
	want, _ := extension.Digest(source.Extensions["ClearBox"])
	got, _ := extension.Digest(migrated.Extensions["ClearBox"])
	if got != want {
		t.Fatalf("ClearBox digest changed across the v1.0 -> v1.1 write: %s != %s", got, want)
	}
	root := clearboxRoot(t, migrated)
	if _, ok := root.Attrs["Output_Path"]; !ok {
		t.Fatal("expected shared Output_Path on the extension root")
	}
	if _, ok := root.Attrs["Software_Trigger_Delay"]; !ok {
		t.Fatal("expected shared Software_Trigger_Delay on the extension root")
	}

	for i, train := range migrated.OpticalTrains {
		cb := clearboxTrain(t, migrated, i)
		// Addition: no v1.0 source.
		if _, ok := cb.Attrs["Firmware_Version"]; ok {
			t.Fatal("firmware_version must be absent")
		}
		// v1.0-only attributes ride along as extra per-train attributes.
		if _, ok := cb.Attrs["Selected_Camera"]; !ok {
			t.Fatal("expected the v1.0-only Selected_Camera to ride along")
		}
		// Change 3: derived ClearBox data has real constants, zero points.
		pc := cbPowerCharacterization(t, migrated, i)
		if cbPointCount(pc) != 0 {
			t.Fatalf("expected zero characterization_points, got %d", cbPointCount(pc))
		}
		if len(cbConstants(pc)) == 0 {
			t.Fatal("expected non-empty derivation_equation_constants")
		}
		if cbText(pc, "Algorithm_Type") != cbText(cbPowerCharacterization(t, source, i), "Algorithm_Type") {
			t.Fatal("algorithm_type changed across the forward migration")
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

func TestV1_0FirstTrainWinsForSharedValues(t *testing.T) {
	// File_Version 1.0 stores Output_Path/Software_Trigger_Delay per train; the extension tree
	// holds one value each, taken from the first train.
	cfg := mockV1_0Config(t, 2)
	second := clearboxTrain(t, cfg, 1)
	second.Attrs["Output_Path"] = extension.Attr{Dtype: extension.Utf8, Value: "/other/"}
	second.Attrs["Software_Trigger_Delay"] = extension.Attr{Dtype: extension.I64, Value: int64(9999)}

	tmp := filepath.Join(t.TempDir(), "first_train_wins.h5")
	writeAs(t, cfg, tmp, "1.0")
	back := parseFile(t, tmp)
	root := clearboxRoot(t, back)
	if got := cbText(root, "Output_Path"); got != "/recordings/" {
		t.Fatalf("Output_Path = %q, want the first train's value", got)
	}
	if got, ok := cbInt(root, "Software_Trigger_Delay"); !ok || got != 3000 {
		t.Fatalf("Software_Trigger_Delay = %v, want the first train's 3000", got)
	}
	if _, ok := clearboxTrain(t, back, 1).Attrs["Output_Path"]; ok {
		t.Fatal("Output_Path must not be repeated per train after the shim")
	}
}

// ---------------------------------------------------------------------------
// Change 3/4 unrecognized-Algorithm_Type: never raises, best-effort (Phase 2)
// ---------------------------------------------------------------------------

func TestV1_1AdapterUnrecognizedAlgorithmTypeBestEffortClearbox(t *testing.T) {
	cfg := mockV1_1Config(t, 1)
	clearboxTrain(t, cfg, 0).Children["Power_Characterization"] = cbMakePowerCharacterization(
		"EXPONENTIAL", []cbConstant{{"0", 1.5}, {"1", 2.5}, {"2", 3.5}})

	v1_0Out := filepath.Join(t.TempDir(), "unrecognized_clearbox_v1_0.h5")
	writeAs(t, cfg, v1_0Out, "1.0")
	v1_0 := parseFile(t, v1_0Out)
	pc := cbPowerCharacterization(t, v1_0, 0)
	if got := cbText(pc, "Algorithm_Type"); got != "EXPONENTIAL" {
		t.Fatalf("algorithm_type = %q", got)
	}
	if got := cbText(pc, "Algorithm_Equation"); got != "" {
		t.Fatalf("algorithm_equation = %q, want blank", got)
	}
	if !strSliceEqual(cbNames(pc), []string{"0", "1", "2"}) {
		t.Fatalf("constant names = %v", cbNames(pc))
	}
	if !floatSliceEqual(cbValues(pc), []float64{1.5, 2.5, 3.5}) {
		t.Fatalf("constant values = %v", cbValues(pc))
	}

	v1_1Out := filepath.Join(t.TempDir(), "unrecognized_clearbox_v1_1.h5")
	writeAs(t, v1_0, v1_1Out, "1.1")
	back := parseFile(t, v1_1Out)
	backPC := cbPowerCharacterization(t, back, 0)
	if got := cbText(backPC, "Algorithm_Type"); got != "EXPONENTIAL" {
		t.Fatalf("algorithm_type = %q, want \"EXPONENTIAL\"", got)
	}
	if !floatSliceEqual(cbValues(backPC), []float64{1.5, 2.5, 3.5}) {
		t.Fatalf("constant values = %v, want [1.5 2.5 3.5]", cbValues(backPC))
	}
}

func TestV1_1AdapterUnrecognizedAlgorithmTypeBestEffortLightSource(t *testing.T) {
	cfg := mockV1_0Config(t, 1)
	inputPC, err := models.ForwardPowerCharacterizationPoints(machineconfig.StrPtr("QUADRATIC"), machineconfig.StrPtr("1,2,3,4"))
	if err != nil {
		t.Fatal(err)
	}
	cfg.OpticalTrains[0].LightSource.PowerCharacterization = inputPC
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

func TestClearboxTrainWithNoMatchingOpticalTrainIsRejectedForV1_0(t *testing.T) {
	cfg := mockV1_1Config(t, 1)
	cfg.Extensions["ClearBox"].Children["Optical_Train_07"] = models.CloneTree(clearboxTrain(t, cfg, 0))
	werr := machineconfig.NewWriter().WriteAs(cfg, filepath.Join(t.TempDir(), "no_matching_train.h5"), "1.0")
	if werr == nil || !strings.Contains(werr.Error(), "Optical_Train_07") {
		t.Fatalf("expected an error naming Optical_Train_07, got %v", werr)
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

	srcPC0 := cbPowerCharacterization(t, source, 0)
	pc0 := cbPowerCharacterization(t, back, 0)
	if cbText(pc0, "Algorithm_Type") != cbText(srcPC0, "Algorithm_Type") {
		t.Fatalf("algorithm_type: got %q want %q", cbText(pc0, "Algorithm_Type"), cbText(srcPC0, "Algorithm_Type"))
	}
	if len(cbValues(pc0)) != len(cbValues(srcPC0)) {
		t.Fatalf("derivation_equation_constants: got %v want %v", cbValues(pc0), cbValues(srcPC0))
	}

	srcRoot, root := clearboxRoot(t, source), clearboxRoot(t, back)
	if cbText(root, "Output_Path") != cbText(srcRoot, "Output_Path") {
		t.Fatalf("output_path: got %q want %q", cbText(root, "Output_Path"), cbText(srcRoot, "Output_Path"))
	}
	g1, _ := cbInt(root, "Software_Trigger_Delay")
	g2, _ := cbInt(srcRoot, "Software_Trigger_Delay")
	if g1 != g2 {
		t.Fatalf("software_trigger_delay: got %v want %v", g1, g2)
	}

	srcLsPC0 := source.OpticalTrains[0].LightSource.PowerCharacterization
	lsPC0 := back.OpticalTrains[0].LightSource.PowerCharacterization
	if lsPC0.AlgorithmType == nil || srcLsPC0.AlgorithmType == nil || *lsPC0.AlgorithmType != *srcLsPC0.AlgorithmType {
		t.Fatalf("algorithm_type: got %v want %v", lsPC0.AlgorithmType, srcLsPC0.AlgorithmType)
	}
	if !floatSliceEqual(pointValues(lsPC0), pointValues(srcLsPC0)) {
		t.Fatalf("characterization_points: got %v want %v", pointValues(lsPC0), pointValues(srcLsPC0))
	}

	// v1.1-only fields: lost on the way down, not restored.
	cb0 := clearboxTrain(t, back, 0)
	if _, ok := cb0.Attrs["Firmware_Version"]; ok {
		t.Fatal("Firmware_Version must not exist in a v1.0 file")
	}
	if _, ok := cb0.Attrs["Selected_Camera"]; ok {
		t.Fatal("Selected_Camera was never in the v1.1 source and must not appear")
	}
}

func TestV1_1AdapterBackwardMigrationReordersConstantsByName(t *testing.T) {
	// Change 3 backward: Derivation_Equation_Constants rows aren't
	// positionally guaranteed — must re-sort by name before joining as CSV.
	cfg := mockV1_1Config(t, 1)
	clearboxTrain(t, cfg, 0).Children["Power_Characterization"] = cbMakePowerCharacterization(
		"LINEAR", []cbConstant{{"a", 2.0}, {"b", 1.0}})

	tmp := filepath.Join(t.TempDir(), "backward_reorders.h5")
	writeAs(t, cfg, tmp, "1.0")
	back := parseFile(t, tmp)
	// Written CSV is "1.0,2.0" (b before a); the v1.0 reader forward-derives
	// that back into named constants positionally (b, then a) for LINEAR.
	got := cbConstants(cbPowerCharacterization(t, back, 0))
	if len(got) != 2 || got[0] != (cbConstant{"b", 1.0}) || got[1] != (cbConstant{"a", 2.0}) {
		t.Fatalf("derivation_equation_constants = %+v, want [{b 1} {a 2}]", got)
	}
}

func TestV1_1AdapterBackwardMigrationMissingDerivationConstantsWritesBlank(t *testing.T) {
	cfg := mockV1_1Config(t, 1)
	clearboxTrain(t, cfg, 0).Children["Power_Characterization"] = cbMakePowerCharacterization("LINEAR", nil)

	tmp := filepath.Join(t.TempDir(), "backward_blank_constants.h5")
	writeAs(t, cfg, tmp, "1.0")
	back := parseFile(t, tmp)
	// Backward derivation produces "" for Volts_To_Watts_Params (blank, not an error); forward
	// re-derivation on read then produces zero constants.
	if got := cbConstants(cbPowerCharacterization(t, back, 0)); len(got) != 0 {
		t.Fatalf("derivation_equation_constants = %v, want empty", got)
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
	backPoints := back.OpticalTrains[0].LightSource.PowerCharacterization.CharacterizationPoints
	if len(backPoints) != 0 {
		t.Fatalf("characterization_points = %v, want empty", backPoints)
	}
}

// ---------------------------------------------------------------------------
// TestV1_1AdapterV1Unaffected
// ---------------------------------------------------------------------------

func TestV1_1AdapterV1Unaffected(t *testing.T) {
	// v1.0 read path surfaces the flat Volts_To_Watts fields as Power_Characterization.
	cfg := parseFile(t, referenceV1_0Fixture(t))
	if cfg.Meta.FileVersion != "1.0" {
		t.Fatalf("file_version = %q", cfg.Meta.FileVersion)
	}
	if len(cfg.OpticalTrains) == 0 {
		t.Fatal("expected optical trains")
	}
	if got := cbText(cbPowerCharacterization(t, cfg, 0), "Algorithm_Type"); got != "LINEAR" {
		t.Fatalf("algorithm_type = %q", got)
	}
	if _, ok := clearboxTrain(t, cfg, 0).Attrs["Volts_To_Watts_Algorithm"]; ok {
		t.Fatal("the flat Volts_To_Watts_Algorithm must not survive the shim")
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
	want, _ := extension.Digest(source.Extensions["ClearBox"])
	got, _ := extension.Digest(back.Extensions["ClearBox"])
	if got != want {
		t.Fatalf("ClearBox digest changed across v1.0 -> v1.1 -> v1.0: %s != %s", got, want)
	}
	for i, train := range back.OpticalTrains {
		srcLsPC := source.OpticalTrains[i].LightSource.PowerCharacterization
		lsPC := train.LightSource.PowerCharacterization
		if (lsPC.AlgorithmType == nil) != (srcLsPC.AlgorithmType == nil) ||
			(srcLsPC.AlgorithmType != nil && *lsPC.AlgorithmType != *srcLsPC.AlgorithmType) {
			t.Fatalf("algorithm_type: got %v want %v", lsPC.AlgorithmType, srcLsPC.AlgorithmType)
		}
		if !floatSliceEqual(pointValues(lsPC), pointValues(srcLsPC)) {
			t.Fatalf("characterization_points: got %v want %v", pointValues(lsPC), pointValues(srcLsPC))
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
	for i := range back.OpticalTrains {
		srcPC := cbPowerCharacterization(t, source, i)
		pc := cbPowerCharacterization(t, back, i)
		if cbText(pc, "Algorithm_Type") != cbText(srcPC, "Algorithm_Type") {
			t.Fatalf("algorithm_type: got %q want %q", cbText(pc, "Algorithm_Type"), cbText(srcPC, "Algorithm_Type"))
		}
		if len(cbConstants(pc)) != len(cbConstants(srcPC)) {
			t.Fatalf("derivation_equation_constants count: got %d want %d", len(cbConstants(pc)), len(cbConstants(srcPC)))
		}
		// Expected loss: v1.1-only fields have no v1.0 round-trip path.
		if _, ok := clearboxTrain(t, back, i).Attrs["Firmware_Version"]; ok {
			t.Fatal("firmware_version must not survive a v1.0 round trip")
		}
		if cbText(pc, "Input_Type") != "" || cbText(pc, "Units_Derived_Quantity") != "" {
			t.Fatalf("input_type/units_derived_quantity should be blank, got %q/%q",
				cbText(pc, "Input_Type"), cbText(pc, "Units_Derived_Quantity"))
		}
		if cbPointCount(pc) != 0 {
			t.Fatalf("characterization_points = %d, want 0", cbPointCount(pc))
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
	if got := cbText(clearboxRoot(t, result), "Output_Path"); got != "/recordings/" {
		t.Fatalf("output_path = %q", got)
	}
	if cbPowerCharacterization(t, result, 0) == nil {
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
	if got := cbText(cbPowerCharacterization(t, result, 0), "Algorithm_Type"); got != "LINEAR" {
		t.Fatalf("power_characterization.algorithm_type = %q", got)
	}
}

// ---------------------------------------------------------------------------
// TestV1_1AdapterSatisfiesRegistryProtocol
// ---------------------------------------------------------------------------

// In Go, ReaderRegistry/WriterRegistry are typed as
// map[string]func(path string) ReaderAdapter / map[string]WriterAdapter —
// so a v1.1 entry satisfying ReaderAdapter/WriterAdapter is primarily a
// compile-time fact enforced by reader.go's/writer.go's own type
// signatures. This test makes it a concrete, runtime-checked fact too: the
// production registries (exercised via the public API) actually dispatch
// "1.1" to a working reader/writer pair, and SupportedFileVersions reports it.
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
}
