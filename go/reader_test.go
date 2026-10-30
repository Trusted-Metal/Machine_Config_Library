package machineconfig_test

import (
	"encoding/json"
	"errors"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"strings"
	"testing"

	machineconfig "machine-config-go"
	"machine-config-go/capabilities"
	"machine-config-go/extension"
	"machine-config-go/internal/h5c"
)

func fixturesDir(t *testing.T) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	// go/reader_test.go → ../fixtures
	return filepath.Clean(filepath.Join(filepath.Dir(file), "..", "fixtures"))
}

func TestReadMetaMachineName(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Meta.MachineName != "TM-LPBF-02: AconityMIDI+_OG" {
		t.Fatalf("machine_name = %q", cfg.Meta.MachineName)
	}
	if cfg.Meta.FileVersion != "1.0" {
		t.Fatalf("file_version = %q", cfg.Meta.FileVersion)
	}
	if len(cfg.Meta.ConfigurationHash) != 64 {
		t.Fatalf("hash len = %d", len(cfg.Meta.ConfigurationHash))
	}
}

func TestReadMachineBuildPlate(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Machine.BuildPlateX == nil || math.Abs(*cfg.Machine.BuildPlateX-250.0) > 1e-9 {
		t.Fatalf("build_plate_x = %v", cfg.Machine.BuildPlateX)
	}
}

func TestOpticalTrainCountAndScanner(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if len(cfg.OpticalTrains) != 2 {
		t.Fatalf("train count = %d", len(cfg.OpticalTrains))
	}
	wd := cfg.OpticalTrains[0].Scanner.WorkingDistance
	if wd == nil || math.Abs(*wd-670.0) > 1e-9 {
		t.Fatalf("working_distance = %v", wd)
	}
	ox := cfg.OpticalTrains[0].Scanner.ScanHeadOffsetX
	if ox == nil || math.Abs(*ox-(-87.5)) > 1e-9 {
		t.Fatalf("scan_head_offset_x = %v", ox)
	}
	if cfg.OpticalTrains[0].Collimator.FocalLength == nil ||
		math.Abs(*cfg.OpticalTrains[0].Collimator.FocalLength-120.0) > 1e-9 {
		t.Fatalf("collimator focal_length = %v", cfg.OpticalTrains[0].Collimator.FocalLength)
	}
	if cfg.OpticalTrains[0].ScannerCard.Model != "SP-ICE-3" {
		t.Fatalf("scanner_card.model = %q", cfg.OpticalTrains[0].ScannerCard.Model)
	}
}

func TestClearboxPresent(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	root := clearboxRoot(t, cfg)
	if len(root.Children) != 2 {
		t.Fatalf("expected 2 per-train children, got %d", len(root.Children))
	}
	cb := clearboxTrain(t, cfg, 0)
	if cbText(cb, "Ip_Address") == "" {
		t.Fatal("clearbox ip empty")
	}
	if _, ok := cbInt(cb, "Data_Port"); !ok {
		t.Fatal("clearbox Data_Port missing or not an integer")
	}
}

func TestClearboxSharedValuesLiveOnTheRootOnce(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	root := clearboxRoot(t, cfg)
	if cbText(root, "Output_Path") == "" {
		t.Fatal("root Output_Path missing")
	}
	if _, ok := cbInt(root, "Software_Trigger_Delay"); !ok {
		t.Fatal("root Software_Trigger_Delay missing")
	}
	train := clearboxTrain(t, cfg, 0)
	if _, ok := train.Attrs["Output_Path"]; ok {
		t.Fatal("Output_Path must not be repeated per train")
	}
	// v1.0-only attributes ride along; the flat Volts_To_Watts fields became Power_Characterization.
	if _, ok := train.Attrs["Show_Console"]; !ok {
		t.Fatal("v1.0-only Show_Console should ride along on each train")
	}
	if _, ok := train.Attrs["Volts_To_Watts_Algorithm"]; ok {
		t.Fatal("Volts_To_Watts_Algorithm should have become Power_Characterization")
	}
}

// ===========================================================================
// Synchronous sensors
// ===========================================================================

func sensorsOf(t *testing.T, cfg *machineconfig.MachineConfig) map[string]*extension.Tree {
	t.Helper()
	group := clearboxTrain(t, cfg, 0).Children["Synchronous_Sensors"]
	if group == nil {
		t.Fatal("train 0 has no Synchronous_Sensors")
	}
	return group.Children
}

func TestSynchronousSensorsAbsentWhenFixtureHasNone(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if _, ok := clearboxTrain(t, cfg, 0).Children["Synchronous_Sensors"]; ok {
		t.Fatal("expected no Synchronous_Sensors")
	}
}

// reference_config_synchronous_sensors.h5, not reference_config.h5, which
// deliberately has no sensors (see SYNCHRONOUS_SENSOR_PLAN.md Phase 0).
func TestSynchronousSensorFixtureHasRealValues(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config_synchronous_sensors.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	sensors := sensorsOf(t, cfg)
	if len(sensors) != 1 {
		t.Fatalf("expected exactly 1 sensor, got %d", len(sensors))
	}
	s, ok := sensors["Oxygen Sensor"]
	if !ok {
		t.Fatal(`expected key "Oxygen Sensor"`)
	}

	for field, want := range map[string]string{
		"Sensor_Name":            "ZR800 Oxygen Analyzer",
		"Sensor_Output_Space":    "log10(ppm)",
		"Sensor_Model":           "ZR810",
		"Sensor_Manufacturer":    "Industrial Physics",
		"Sensor_Scope":           "Global",
		"Units_Derived_Quantity": "ppm",
		"Sensor_Type":            "Oxygen Sensor",
		"Input_Type":             "4-20 mA",
		"Algorithm_Type":         "Log-Linear",
		"Algorithm_Equation":     "log(ppm) = a*mA + b",
		"Calibration_Source":     "Datasheet",
	} {
		if got := cbText(s, field); got != want {
			t.Errorf("%s: got %q want %q", field, got, want)
		}
	}
	for field, want := range map[string]float64{
		"Sensor_Output_Range_Low":  -1.0,
		"Sensor_Output_Range_High": 6.0,
		"Sample_Period":            5.0,
	} {
		if got, ok := cbFloat(s, field); !ok || got != want {
			t.Errorf("%s: got %v want %v", field, got, want)
		}
	}
	if got, ok := cbInt(s, "Port_ID"); !ok || got != 5 {
		t.Errorf("Port_ID: got %v want 5", got)
	}
	if !strings.Contains(cbText(s, "Metadata"), "Synchronous Sensor because Clearbox is responsible") {
		t.Errorf("Metadata: got %q", cbText(s, "Metadata"))
	}

	consts := s.Tables["Derivation_Equation_Constants"]
	if consts == nil {
		t.Fatal("missing Derivation_Equation_Constants table")
	}
	if !reflect.DeepEqual(consts.Columns["name"], []string{"a", "b"}) ||
		!reflect.DeepEqual(consts.Columns["value"], []float64{0.4375, -2.75}) {
		t.Errorf("derivation_equation_constants: got %+v", consts.Columns)
	}
	points := s.Tables["Calibration_Points"]
	if points == nil {
		t.Fatal("missing Calibration_Points table")
	}
	if !reflect.DeepEqual(points.Columns["input_value"], []float64{4.0, 20.0}) ||
		!reflect.DeepEqual(points.Columns["output_value"], []float64{-1.0, 6.0}) {
		t.Errorf("calibration_points: got %+v", points.Columns)
	}

	// log(ppm) = a*mA + b, per algorithm_equation — proves the points are
	// recorded in Sensor_Output_Space (log10(ppm)) units, not the linear
	// Units_Derived_Quantity (ppm) units of the same underlying quantity.
	values := consts.Columns["value"].([]float64)
	in := points.Columns["input_value"].([]float64)
	out := points.Columns["output_value"].([]float64)
	for i := range in {
		if got := values[0]*in[i] + values[1]; math.Abs(got-out[i]) > 1e-10 {
			t.Errorf("a*%v+b = %v, want %v", in[i], got, out[i])
		}
	}
}

func TestCombinedOpcuaAndSynchronousSensorsFixtureHasBoth(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config_opcua_synchronous_sensors.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Extensions["TM_OPCUA"] == nil {
		t.Fatal("expected the TM_OPCUA extension")
	}
	s, ok := sensorsOf(t, cfg)["Oxygen Sensor"]
	if !ok {
		t.Fatal(`expected key "Oxygen Sensor"`)
	}
	if !reflect.DeepEqual(s.Tables["Derivation_Equation_Constants"].Columns["name"], []string{"a", "b"}) {
		t.Errorf("derivation_equation_constants: got %+v", s.Tables["Derivation_Equation_Constants"].Columns)
	}
}

func TestClearboxCorrectionGrid(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	data, shape := cbGrid(t, cfg, 0, "Correction_Data")
	if !reflect.DeepEqual(shape, []uint64{257, 257, 2}) {
		t.Fatalf("shape = %v", shape)
	}
	nan := 0
	for _, v := range data {
		if math.IsNaN(v) {
			nan++
		}
	}
	if nan == 0 {
		t.Fatal("expected NaNs in correction grid")
	}
}

func writeFutureFileVersion(t *testing.T, path string) {
	t.Helper()
	f, err := h5c.Create(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	root, err := f.Root()
	if err != nil {
		t.Fatal(err)
	}
	defer root.Close()
	if err := root.WriteStringAttr("File_Version", "2.0"); err != nil {
		t.Fatal(err)
	}
}

func TestUnknownFileVersionDoesNotUseV1Layout(t *testing.T) {
	path := filepath.Join(t.TempDir(), "future.h5")
	writeFutureFileVersion(t, path)

	_, err := machineconfig.NewReader(path).Parse()
	var u *machineconfig.UnsupportedFileVersionError
	if !errors.As(err, &u) {
		t.Fatalf("Parse: got %#v, want UnsupportedFileVersionError", err)
	}
	if u.Version != "2.0" {
		t.Fatalf("version = %q", u.Version)
	}

	_, capErr := capabilities.OpenMachineConfig(path)
	if capErr == nil {
		t.Fatal("OpenMachineConfig: expected error")
	}
	if capErr.Code != capabilities.ErrUnsupportedVersion {
		t.Fatalf("code = %q", capErr.Code)
	}
	if !strings.Contains(capErr.Message, "2.0") {
		t.Fatalf("message = %q", capErr.Message)
	}
}

// DISPATCH_REGISTRY_PLAN.md — proves ResolveReader is genuinely
// registry-driven, not a relocated hardcoded check.
type fakeReaderAdapter struct{}

func (fakeReaderAdapter) Parse(opts machineconfig.ParseOptions) (*machineconfig.MachineConfig, error) {
	return &machineconfig.MachineConfig{
		Meta: machineconfig.MachineConfigMeta{MachineName: "fake-sentinel"},
	}, nil
}

func TestReaderRegistryRejectsUnregisteredVersion(t *testing.T) {
	registry := machineconfig.ReaderRegistry{}
	_, err := machineconfig.ResolveReader("9.9-nope", "unused-path.h5", registry)
	var u *machineconfig.UnsupportedFileVersionError
	if !errors.As(err, &u) {
		t.Fatalf("expected UnsupportedFileVersionError, got %#v", err)
	}
	if u.Version != "9.9-nope" {
		t.Fatalf("version = %q", u.Version)
	}
}

func TestReaderRegistryDispatchesViaInjectedAdapter(t *testing.T) {
	registry := machineconfig.ReaderRegistry{
		"9.9-test": func(path string) machineconfig.ReaderAdapter { return fakeReaderAdapter{} },
	}
	adapter, err := machineconfig.ResolveReader("9.9-test", "unused-path.h5", registry)
	if err != nil {
		t.Fatal(err)
	}
	cfg, err := adapter.Parse(machineconfig.ParseOptions{})
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Meta.MachineName != "fake-sentinel" {
		t.Fatalf("expected fake sentinel, got %q — dispatch did not route to the fake", cfg.Meta.MachineName)
	}
}

// ---------------------------------------------------------------------------
// meta.extra
// ---------------------------------------------------------------------------

func TestMetaExtra(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Meta.Extra == nil {
		t.Fatal("meta.extra is nil")
	}
	desc, ok := cfg.Meta.Extra["Description"].(string)
	if !ok || desc == "" {
		t.Fatalf("meta.extra.Description = %v", cfg.Meta.Extra["Description"])
	}
	gen, ok := cfg.Meta.Extra["Generator"].(string)
	if !ok || gen == "" {
		t.Fatalf("meta.extra.Generator = %v", cfg.Meta.Extra["Generator"])
	}
}

// ---------------------------------------------------------------------------
// Machine fields
// ---------------------------------------------------------------------------

func TestMachineFields(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	m := cfg.Machine
	if m.BuildPlateY == nil || math.Abs(*m.BuildPlateY-250.0) > 1e-9 {
		t.Fatalf("build_plate_y = %v", m.BuildPlateY)
	}
	if m.BuildPlateZ == nil || math.Abs(*m.BuildPlateZ-20.0) > 1e-9 {
		t.Fatalf("build_plate_z = %v", m.BuildPlateZ)
	}
	if m.SerialNumber != "500300_1" {
		t.Fatalf("machine serial_number = %q", m.SerialNumber)
	}
	if m.GasFlowDirection == nil || *m.GasFlowDirection != "Y+" {
		t.Fatalf("gas_flow_direction = %v", m.GasFlowDirection)
	}
}

// ---------------------------------------------------------------------------
// Thermal lensing — both trains
// ---------------------------------------------------------------------------

func TestThermalLensing(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}

	t0 := cfg.OpticalTrains[0]
	if t0.ThermalLensingPassed == nil || *t0.ThermalLensingPassed != false {
		t.Fatalf("train 0 thermal_lensing_passed = %v", t0.ThermalLensingPassed)
	}
	if t0.ThermalLensingFocalPlaneShift == nil || math.Abs(*t0.ThermalLensingFocalPlaneShift-1.178387083333333) > 1e-9 {
		t.Fatalf("train 0 thermal_lensing_focal_plane_shift = %v", t0.ThermalLensingFocalPlaneShift)
	}
	if t0.ThermalLensingThreshold == nil || math.Abs(*t0.ThermalLensingThreshold-0.7848370833333334) > 1e-9 {
		t.Fatalf("train 0 thermal_lensing_threshold = %v", t0.ThermalLensingThreshold)
	}

	t1 := cfg.OpticalTrains[1]
	if t1.ThermalLensingPassed == nil || *t1.ThermalLensingPassed != true {
		t.Fatalf("train 1 thermal_lensing_passed = %v", t1.ThermalLensingPassed)
	}
	if t1.ThermalLensingFocalPlaneShift == nil || math.Abs(*t1.ThermalLensingFocalPlaneShift-0.1906766666666668) > 1e-9 {
		t.Fatalf("train 1 thermal_lensing_focal_plane_shift = %v", t1.ThermalLensingFocalPlaneShift)
	}
}

// ---------------------------------------------------------------------------
// Scan-field correction file
// ---------------------------------------------------------------------------

func TestScanFieldCorrectionFile(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	sfcf0 := cfg.OpticalTrains[0].ScanFieldCorrectionFile
	if sfcf0 == nil {
		t.Fatal("train 0 scan_field_correction_file is nil")
	}
	if sfcf0.FileSize != 1138799 {
		t.Fatalf("train 0 sfcf file_size = %d", sfcf0.FileSize)
	}
	if sfcf0.DocumentName != "Story6.17.3.1_Laser_1_VMM.fc3" {
		t.Fatalf("train 0 sfcf document_name = %q", sfcf0.DocumentName)
	}
	if sfcf0.DocumentID == "" {
		t.Fatal("train 0 sfcf document_id is empty")
	}

	sfcf1 := cfg.OpticalTrains[1].ScanFieldCorrectionFile
	if sfcf1 == nil {
		t.Fatal("train 1 scan_field_correction_file is nil")
	}
	if sfcf1.FileSize != 1142763 {
		t.Fatalf("train 1 sfcf file_size = %d", sfcf1.FileSize)
	}
}

// ---------------------------------------------------------------------------
// Scanner: scan_head_rotation and scan_head_offset_y
// ---------------------------------------------------------------------------

func TestScannerRotationAndOffsetY(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	rot0 := cfg.OpticalTrains[0].Scanner.ScanHeadRotation
	if rot0 == nil || math.Abs(*rot0-0.0) > 1e-9 {
		t.Fatalf("train 0 scan_head_rotation = %v", rot0)
	}
	rot1 := cfg.OpticalTrains[1].Scanner.ScanHeadRotation
	if rot1 == nil || math.Abs(*rot1-180.0) > 1e-9 {
		t.Fatalf("train 1 scan_head_rotation = %v", rot1)
	}
	oy0 := cfg.OpticalTrains[0].Scanner.ScanHeadOffsetY
	if oy0 == nil || math.Abs(*oy0-23.5) > 1e-9 {
		t.Fatalf("train 0 scan_head_offset_y = %v", oy0)
	}
	oy1 := cfg.OpticalTrains[1].Scanner.ScanHeadOffsetY
	if oy1 == nil || math.Abs(*oy1-(-21.695)) > 1e-4 {
		t.Fatalf("train 1 scan_head_offset_y = %v", oy1)
	}
}

func TestScannerInvertFlagsAbsentFromFixtureReadAsFalse(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	s := cfg.OpticalTrains[0].Scanner
	if s.InvertActualX || s.InvertActualY || s.InvertCommandedX || s.InvertCommandedY {
		t.Fatalf("expected all invert flags false, got %+v", s)
	}
}

func TestScannerInvertFlagsOmittedFromJSONWhenFalse(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	b, err := json.Marshal(cfg.OpticalTrains[0].Scanner)
	if err != nil {
		t.Fatal(err)
	}
	var m map[string]any
	if err := json.Unmarshal(b, &m); err != nil {
		t.Fatal(err)
	}
	for _, key := range []string{"invert_actual_x", "invert_actual_y", "invert_commanded_x", "invert_commanded_y"} {
		if _, ok := m[key]; ok {
			t.Errorf("expected %q to be omitted from JSON, got %v", key, m[key])
		}
	}
}

// ---------------------------------------------------------------------------
// Light source and scanner card
// ---------------------------------------------------------------------------

func TestLightSourceAndScannerCard(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	ls := cfg.OpticalTrains[0].LightSource
	if ls.Wavelength == nil || math.Abs(*ls.Wavelength-1070.0) > 1e-9 {
		t.Fatalf("light_source.wavelength = %v", ls.Wavelength)
	}
	if ls.WavelengthUnit == nil || *ls.WavelengthUnit != "nm" {
		t.Fatalf("light_source.wavelength_unit = %v", ls.WavelengthUnit)
	}
	if ls.PowerMaxNominal == nil || math.Abs(*ls.PowerMaxNominal-1000.0) > 1e-9 {
		t.Fatalf("light_source.power_max_nominal = %v", ls.PowerMaxNominal)
	}

	sc := cfg.OpticalTrains[0].ScannerCard
	if sc.SamplePeriod == nil || math.Abs(*sc.SamplePeriod-10.0) > 1e-9 {
		t.Fatalf("scanner_card.sample_period = %v", sc.SamplePeriod)
	}
}

// ---------------------------------------------------------------------------
// Train IDs
// ---------------------------------------------------------------------------

func TestTrainIDs(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.OpticalTrains[0].TrainID != "Optical_Train_01" {
		t.Fatalf("train 0 id = %q", cfg.OpticalTrains[0].TrainID)
	}
	if cfg.OpticalTrains[1].TrainID != "Optical_Train_02" {
		t.Fatalf("train 1 id = %q", cfg.OpticalTrains[1].TrainID)
	}
}

// ---------------------------------------------------------------------------
// Inverse correction data
// ---------------------------------------------------------------------------

func TestClearboxInverseCorrectionGrid(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	inv, shape := cbGrid(t, cfg, 0, "Inverse_Correction_Data")
	if !reflect.DeepEqual(shape, []uint64{257, 257, 2}) {
		t.Fatalf("inverse shape = %v", shape)
	}
	nan := 0
	for _, v := range inv {
		if math.IsNaN(v) {
			nan++
		}
	}
	if nan == 0 {
		t.Fatal("expected NaNs in inverse correction grid")
	}
	// forward and inverse grids must differ
	fwd, _ := cbGrid(t, cfg, 0, "Correction_Data")
	same := true
	for i := range fwd {
		if !math.IsNaN(fwd[i]) && !math.IsNaN(inv[i]) && fwd[i] != inv[i] {
			same = false
			break
		}
	}
	if same {
		t.Fatal("forward and inverse correction grids are identical — expected them to differ")
	}
}

// ---------------------------------------------------------------------------
// JSON top-level structure
// ---------------------------------------------------------------------------

func TestJSONTopLevelKeys(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	b, err := json.Marshal(cfg)
	if err != nil {
		t.Fatal(err)
	}
	var v map[string]json.RawMessage
	if err := json.Unmarshal(b, &v); err != nil {
		t.Fatal(err)
	}
	for _, key := range []string{"meta", "machine", "optical_trains"} {
		if _, ok := v[key]; !ok {
			t.Errorf("JSON missing top-level key %q", key)
		}
	}
	if _, ok := v["extensions"]; !ok {
		t.Error("reference fixture JSON must contain the ClearBox extension")
	}

	// the opcua fixture carries its OPCUA group as an opaque extension
	path2 := filepath.Join(fixturesDir(t), "reference_config_opcua.h5")
	cfg2, err := machineconfig.NewReader(path2).Parse()
	if err != nil {
		t.Fatal(err)
	}
	b2, _ := json.Marshal(cfg2)
	var v2 map[string]json.RawMessage
	json.Unmarshal(b2, &v2) //nolint:errcheck
	if _, ok := v2["extensions"]; !ok {
		t.Error("opcua fixture JSON must contain an extensions key")
	}
	if _, ok := v2["opcua"]; ok {
		t.Error("typed opcua key must no longer be emitted")
	}
}

func TestJSONSchemaValidation(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	b, err := json.Marshal(cfg)
	if err != nil {
		t.Fatal(err)
	}
	var v map[string]any
	if err := json.Unmarshal(b, &v); err != nil {
		t.Fatal(err)
	}

	for _, key := range []string{"meta", "machine", "optical_trains"} {
		if _, ok := v[key]; !ok {
			t.Errorf("missing top-level key %q", key)
		}
	}
	meta, _ := v["meta"].(map[string]any)
	if name, _ := meta["machine_name"].(string); name == "" {
		t.Error("meta.machine_name must be non-empty")
	}
	if hash, _ := meta["configuration_hash"].(string); len(hash) != 64 {
		t.Errorf("meta.configuration_hash len = %d, want 64", len(hash))
	}
	trains, _ := v["optical_trains"].([]any)
	if len(trains) < 1 {
		t.Fatalf("optical_trains is empty")
	}
	for i, tr := range trains {
		m, _ := tr.(map[string]any)
		if _, ok := m["train_id"]; !ok {
			t.Errorf("train %d missing train_id", i)
		}
		if _, ok := m["scanner"]; !ok {
			t.Errorf("train %d missing scanner", i)
		}
	}

	// Schema file must exist and be valid JSON.
	_, srcFile, _, _ := runtime.Caller(0)
	schemaPath := filepath.Join(filepath.Dir(srcFile), "..", "schema", "machine_config_v1.schema.json")
	schemaBytes, err := os.ReadFile(schemaPath)
	if err != nil {
		t.Fatalf("read schema: %v", err)
	}
	var schemaAny any
	if err := json.Unmarshal(schemaBytes, &schemaAny); err != nil {
		t.Fatalf("schema is not valid JSON: %v", err)
	}
}
