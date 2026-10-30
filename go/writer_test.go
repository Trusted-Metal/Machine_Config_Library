package machineconfig_test

import (
	"crypto/sha256"
	"encoding/binary"
	"encoding/json"
	"math"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	machineconfig "machine-config-go"
	"machine-config-go/extension"
	"machine-config-go/internal/h5c"
)

// roundtrip writes original to a temp file and reads it back.
func roundtrip(t *testing.T, src string) (*machineconfig.MachineConfig, *machineconfig.MachineConfig) {
	t.Helper()
	original, err := machineconfig.NewReader(src).Parse()
	if err != nil {
		t.Fatal(err)
	}
	tmp := filepath.Join(t.TempDir(), "rt.h5")
	if err := machineconfig.NewWriter().Write(original, tmp); err != nil {
		t.Fatal(err)
	}
	rt, err := machineconfig.NewReader(tmp).Parse()
	if err != nil {
		t.Fatal(err)
	}
	return original, rt
}

func correctionHash(data []float64) string {
	h := sha256.New()
	buf := make([]byte, 8)
	for _, v := range data {
		binary.LittleEndian.PutUint64(buf, math.Float64bits(v))
		h.Write(buf)
	}
	return string(h.Sum(nil))
}

func TestWriterRoundtripMetaFields(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config.h5"))
	if rt.Meta.MachineName != orig.Meta.MachineName {
		t.Errorf("machine_name: got %q want %q", rt.Meta.MachineName, orig.Meta.MachineName)
	}
	if rt.Meta.Manufacturer != orig.Meta.Manufacturer {
		t.Errorf("manufacturer: got %q want %q", rt.Meta.Manufacturer, orig.Meta.Manufacturer)
	}
	// reference_config.h5 is externally-authored, so its own stored hash is
	// expected to be invalid — but once *this* writer produces a file, that
	// file's stored hash must be real (64 hex chars, not a passthrough of
	// the input) and must read back valid.
	if len(rt.Meta.ConfigurationHash) != 64 {
		t.Errorf("configuration_hash len = %d, want 64", len(rt.Meta.ConfigurationHash))
	}
	if rt.Meta.ConfigurationHash == orig.Meta.ConfigurationHash {
		t.Errorf("configuration_hash should differ from the externally-authored original, got same value")
	}
	if rt.Meta.IsValid == nil || !*rt.Meta.IsValid {
		t.Errorf("is_valid = %v, want true", rt.Meta.IsValid)
	}
	if rt.Meta.FileVersion != orig.Meta.FileVersion {
		t.Errorf("file_version: got %q want %q", rt.Meta.FileVersion, orig.Meta.FileVersion)
	}
	if rt.Meta.ExportDate != orig.Meta.ExportDate {
		t.Errorf("export_date: got %q want %q", rt.Meta.ExportDate, orig.Meta.ExportDate)
	}
}

func TestWriterRoundtripMachineFields(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config.h5"))
	m, mo := rt.Machine, orig.Machine
	if (m.BuildPlateX == nil) != (mo.BuildPlateX == nil) || (mo.BuildPlateX != nil && math.Abs(*m.BuildPlateX-*mo.BuildPlateX) > 1e-9) {
		t.Errorf("build_plate_x: got %v want %v", m.BuildPlateX, mo.BuildPlateX)
	}
	if (m.BuildPlateXUnit == nil) != (mo.BuildPlateXUnit == nil) || (mo.BuildPlateXUnit != nil && *m.BuildPlateXUnit != *mo.BuildPlateXUnit) {
		t.Errorf("build_plate_x_unit mismatch")
	}
	if (m.GasFlowDirection == nil) != (mo.GasFlowDirection == nil) || (mo.GasFlowDirection != nil && *m.GasFlowDirection != *mo.GasFlowDirection) {
		t.Errorf("gas_flow_direction: got %v want %v", m.GasFlowDirection, mo.GasFlowDirection)
	}
	if (m.RecoatDirection == nil) != (mo.RecoatDirection == nil) || (mo.RecoatDirection != nil && *m.RecoatDirection != *mo.RecoatDirection) {
		t.Errorf("recoat_direction: got %v want %v", m.RecoatDirection, mo.RecoatDirection)
	}
	if (m.RecoaterBladeType == nil) != (mo.RecoaterBladeType == nil) || (mo.RecoaterBladeType != nil && *m.RecoaterBladeType != *mo.RecoaterBladeType) {
		t.Errorf("recoater_blade_type: got %v want %v", m.RecoaterBladeType, mo.RecoaterBladeType)
	}
}

func TestWriterRoundtripTrainCount(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config.h5"))
	if len(rt.OpticalTrains) != len(orig.OpticalTrains) {
		t.Errorf("optical_trains count: got %d want %d", len(rt.OpticalTrains), len(orig.OpticalTrains))
	}
}

func TestWriterRoundtripScannerOffsets(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config.h5"))
	so, sr := orig.OpticalTrains[0].Scanner, rt.OpticalTrains[0].Scanner
	if (sr.ScanHeadOffsetX == nil) != (so.ScanHeadOffsetX == nil) || (so.ScanHeadOffsetX != nil && math.Abs(*sr.ScanHeadOffsetX-*so.ScanHeadOffsetX) > 1e-9) {
		t.Errorf("scan_head_offset_x: got %v want %v", sr.ScanHeadOffsetX, so.ScanHeadOffsetX)
	}
	if (sr.WorkingDistance == nil) != (so.WorkingDistance == nil) || (so.WorkingDistance != nil && math.Abs(*sr.WorkingDistance-*so.WorkingDistance) > 1e-9) {
		t.Errorf("working_distance: got %v want %v", sr.WorkingDistance, so.WorkingDistance)
	}
}

func TestWriterRoundtripInvertFlagsDefaultFalseAndOmittedFromJSON(t *testing.T) {
	_, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config.h5"))
	s := rt.OpticalTrains[0].Scanner
	if s.InvertActualX || s.InvertActualY || s.InvertCommandedX || s.InvertCommandedY {
		t.Fatalf("expected all invert flags false, got %+v", s)
	}
	b, err := json.Marshal(s)
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

func TestWriterOnlyTrueInvertFlagsSurviveAsRealHDF5Attributes(t *testing.T) {
	cfg, err := machineconfig.NewReader(filepath.Join(fixturesDir(t), "reference_config.h5")).Parse()
	if err != nil {
		t.Fatal(err)
	}
	cfg.OpticalTrains[0].Scanner.InvertActualX = true
	cfg.OpticalTrains[0].Scanner.InvertActualY = false
	cfg.OpticalTrains[0].Scanner.InvertCommandedX = true
	cfg.OpticalTrains[0].Scanner.InvertCommandedY = false

	out := filepath.Join(t.TempDir(), "invert.h5")
	if err := machineconfig.NewWriter().Write(cfg, out); err != nil {
		t.Fatal(err)
	}

	// Raw HDF5 inspection: only the two true-valued attributes exist at all.
	f, err := h5c.Open(out)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	scannerGrp, err := f.Group("Machine/Optical_Trains/Optical_Train_01/Scanner")
	if err != nil {
		t.Fatal(err)
	}
	defer scannerGrp.Close()
	if !scannerGrp.HasAttr("Invert_Actual_X") {
		t.Error("expected Invert_Actual_X attribute to exist")
	}
	if !scannerGrp.HasAttr("Invert_Commanded_X") {
		t.Error("expected Invert_Commanded_X attribute to exist")
	}
	if scannerGrp.HasAttr("Invert_Actual_Y") {
		t.Error("expected Invert_Actual_Y attribute to be absent")
	}
	if scannerGrp.HasAttr("Invert_Commanded_Y") {
		t.Error("expected Invert_Commanded_Y attribute to be absent")
	}

	// Read path still returns the correct value either way.
	rt, err := machineconfig.NewReader(out).Parse()
	if err != nil {
		t.Fatal(err)
	}
	s := rt.OpticalTrains[0].Scanner
	if !s.InvertActualX || s.InvertActualY || !s.InvertCommandedX || s.InvertCommandedY {
		t.Fatalf("got %+v", s)
	}
}

func TestWriterRoundtripAxisConfiguration(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config.h5"))
	so, sr := orig.OpticalTrains[0].Scanner, rt.OpticalTrains[0].Scanner
	if (sr.AxisConfiguration == nil) != (so.AxisConfiguration == nil) || (so.AxisConfiguration != nil && *sr.AxisConfiguration != *so.AxisConfiguration) {
		t.Errorf("axis_configuration: got %v want %v", sr.AxisConfiguration, so.AxisConfiguration)
	}
	if sr.ZAxis == nil {
		t.Error("z_axis: got nil, want non-nil (reference fixture is 3D)")
	}
	if sr.Focus != nil {
		t.Error("focus: got non-nil, want nil (reference fixture is 3D, no Focus)")
	}
	if (sr.XAxis.SmoothingKernel == nil) != (so.XAxis.SmoothingKernel == nil) || (so.XAxis.SmoothingKernel != nil && *sr.XAxis.SmoothingKernel != *so.XAxis.SmoothingKernel) {
		t.Errorf("x_axis.smoothing_kernel: got %v want %v", sr.XAxis.SmoothingKernel, so.XAxis.SmoothingKernel)
	}
}

func TestWriterRoundtripThermalLensing(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config.h5"))
	for i := range orig.OpticalTrains {
		ot, otr := orig.OpticalTrains[i], rt.OpticalTrains[i]
		if (otr.ThermalLensingPassed == nil) != (ot.ThermalLensingPassed == nil) || (ot.ThermalLensingPassed != nil && *otr.ThermalLensingPassed != *ot.ThermalLensingPassed) {
			t.Errorf("train %d thermal_lensing_passed: got %v want %v", i, otr.ThermalLensingPassed, ot.ThermalLensingPassed)
		}
	}
}

func TestWriterRoundtripClearboxExtension(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config.h5"))
	// Bit-exact: attributes with their stored dtypes, both correction grids, Power_Characterization.
	want, err := extension.Digest(orig.Extensions["ClearBox"])
	if err != nil {
		t.Fatal(err)
	}
	got, err := extension.Digest(rt.Extensions["ClearBox"])
	if err != nil {
		t.Fatal(err)
	}
	if got != want {
		t.Errorf("ClearBox digest changed across a write/read round trip: %s != %s", got, want)
	}
	cbo, cbr := clearboxTrain(t, orig, 0), clearboxTrain(t, rt, 0)
	if cbText(cbr, "Ip_Address") != cbText(cbo, "Ip_Address") {
		t.Errorf("ip_address: got %q want %q", cbText(cbr, "Ip_Address"), cbText(cbo, "Ip_Address"))
	}
	if _, ok := cbr.Children["Synchronous_Sensors"]; ok {
		t.Error("expected no sensors on reference_config.h5 roundtrip")
	}
}

// ---------------------------------------------------------------------------
// Roundtrip: synchronous sensors
// ---------------------------------------------------------------------------

// writeAndRead writes cfg to a temp file and reads it back.
func writeAndRead(t *testing.T, cfg *machineconfig.MachineConfig) *machineconfig.MachineConfig {
	t.Helper()
	tmp := filepath.Join(t.TempDir(), "rt.h5")
	if err := machineconfig.NewWriter().Write(cfg, tmp); err != nil {
		t.Fatal(err)
	}
	rt, err := machineconfig.NewReader(tmp).Parse()
	if err != nil {
		t.Fatal(err)
	}
	return rt
}

func cbSetSensor(t *testing.T, cfg *machineconfig.MachineConfig, label string, sensor *extension.Tree) {
	t.Helper()
	train := clearboxTrain(t, cfg, 0)
	sensors := extension.NewTree()
	sensors.Children[label] = sensor
	train.Children["Synchronous_Sensors"] = sensors
}

func cbSensorTree(constants []cbConstant) *extension.Tree {
	names := make([]string, len(constants))
	values := make([]float64, len(constants))
	for i, c := range constants {
		names[i], values[i] = c.Name, c.Value
	}
	tree := extension.NewTree()
	tree.Tables["Derivation_Equation_Constants"] = &extension.Table{
		Fields: []extension.TableField{
			{Name: "name", Dtype: extension.Utf8, Size: 64},
			{Name: "value", Dtype: extension.F64},
		},
		Rows:    uint64(len(constants)),
		Columns: map[string]any{"name": names, "value": values},
		Attrs:   map[string]extension.Attr{},
	}
	return tree
}

func TestWriterRoundtripSynchronousSensorTablesExactValuesInOrder(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config_synchronous_sensors.h5"))
	so := sensorsOf(t, orig)["Oxygen Sensor"]
	sr := sensorsOf(t, rt)["Oxygen Sensor"]
	for _, name := range []string{"Derivation_Equation_Constants", "Calibration_Points"} {
		if !reflect.DeepEqual(sr.Tables[name].Columns, so.Tables[name].Columns) {
			t.Errorf("%s: got %+v want %+v", name, sr.Tables[name].Columns, so.Tables[name].Columns)
		}
	}
	if got := cbText(sr, "Sensor_Name"); got != "ZR800 Oxygen Analyzer" {
		t.Errorf("sensor_name: got %q", got)
	}
}

// The sensor itself exists (its group is created), but its compound
// dataset has zero rows — proving 0-length compound dataset
// creation/read genuinely works, not assumed.
func TestWriterRoundtripSynchronousSensorWithZeroRowTable(t *testing.T) {
	cfg, err := machineconfig.NewReader(filepath.Join(fixturesDir(t), "reference_config.h5")).Parse()
	if err != nil {
		t.Fatal(err)
	}
	sensor := cbSensorTree(nil)
	sensor.Attrs["Sensor_Name"] = extension.Attr{Dtype: extension.Utf8, Value: "Placeholder"}
	cbSetSensor(t, cfg, "Untested Sensor", sensor)
	rt := writeAndRead(t, cfg)
	s, ok := sensorsOf(t, rt)["Untested Sensor"]
	if !ok {
		t.Fatal(`expected key "Untested Sensor"`)
	}
	if rows := s.Tables["Derivation_Equation_Constants"].Rows; rows != 0 {
		t.Errorf("derivation_equation_constants rows: got %d, want 0", rows)
	}
	if got := cbText(s, "Sensor_Name"); got != "Placeholder" {
		t.Errorf("sensor_name: got %q", got)
	}
}

// The map key is a free-form label with no schema meaning — proves a key
// unlike the fixture's own "Oxygen Sensor" (different style: underscore-
// joined, all-caps) survives a write->read cycle verbatim.
func TestWriterRoundtripSynchronousSensorArbitraryDifferentlyStyledKey(t *testing.T) {
	cfg, err := machineconfig.NewReader(filepath.Join(fixturesDir(t), "reference_config.h5")).Parse()
	if err != nil {
		t.Fatal(err)
	}
	sensor := extension.NewTree()
	sensor.Attrs["Port_ID"] = extension.Attr{Dtype: extension.I32, Value: int64(9)}
	cbSetSensor(t, cfg, "HUMIDITY_SENSOR_2", sensor)
	rt := writeAndRead(t, cfg)
	sensors := sensorsOf(t, rt)
	s, ok := sensors["HUMIDITY_SENSOR_2"]
	if !ok {
		t.Fatalf(`expected key "HUMIDITY_SENSOR_2", got keys %v`, sensors)
	}
	if got, ok := cbInt(s, "Port_ID"); !ok || got != 9 {
		t.Errorf("port_id: got %v want 9", got)
	}
}

// Proves the Writer side of the cross-feature guarantee: parses a real
// OPCUA-only fixture, adds a sensor purely in memory, writes, and confirms
// both survive re-reading (mirrors Rust's/Python's/Node's equivalent tests).
func TestWriterOpcuaAndSynchronousSensorCoexist(t *testing.T) {
	cfg, err := machineconfig.NewReader(filepath.Join(fixturesDir(t), "reference_config_opcua.h5")).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Extensions["TM_OPCUA"] == nil {
		t.Fatal("fixture must already have OPCUA before the test adds a sensor")
	}
	cbSetSensor(t, cfg, "Oxygen Sensor", cbSensorTree([]cbConstant{{"a", 0.4375}, {"b", -2.75}}))
	rt := writeAndRead(t, cfg)
	if rt.Extensions["TM_OPCUA"] == nil {
		t.Fatal("OPCUA must survive alongside the newly-added sensor")
	}
	s, ok := sensorsOf(t, rt)["Oxygen Sensor"]
	if !ok {
		t.Fatal(`expected key "Oxygen Sensor"`)
	}
	if !reflect.DeepEqual(s.Tables["Derivation_Equation_Constants"].Columns["name"], []string{"a", "b"}) {
		t.Errorf("derivation_equation_constants: got %+v", s.Tables["Derivation_Equation_Constants"].Columns)
	}
}

// Derivation_Equation_Constants.name is a 64-byte fixed-length field (see
// SYNCHRONOUS_SENSOR_PLAN.md's "Compound dataset string convention"). A name
// whose UTF-8 encoding exceeds 64 bytes must be rejected with a clear error
// at write time, not silently truncated.
func TestWriterRejectsEquationConstantNameOver64Bytes(t *testing.T) {
	cfg, err := machineconfig.NewReader(filepath.Join(fixturesDir(t), "reference_config.h5")).Parse()
	if err != nil {
		t.Fatal(err)
	}
	cbSetSensor(t, cfg, "Oversized Name Sensor", cbSensorTree([]cbConstant{{strings.Repeat("a", 65), 1.0}}))
	tmp := filepath.Join(t.TempDir(), "oversized.h5")
	err = machineconfig.NewWriter().Write(cfg, tmp)
	if err == nil {
		t.Fatal("expected an error for an oversized constant name, got nil")
	}
	if !strings.Contains(err.Error(), "fit") {
		t.Fatalf("expected a 'does not fit' error, got: %v", err)
	}
}

func TestWriterRoundtripSFCFMetadata(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config.h5"))
	sfcfo := orig.OpticalTrains[0].ScanFieldCorrectionFile
	sfcfr := rt.OpticalTrains[0].ScanFieldCorrectionFile
	if sfcfo == nil || sfcfr == nil {
		t.Fatal("expected sfcf on both original and roundtrip")
	}
	if sfcfr.DocumentName != sfcfo.DocumentName {
		t.Errorf("document_name: got %q want %q", sfcfr.DocumentName, sfcfo.DocumentName)
	}
	if sfcfr.FileSize != sfcfo.FileSize {
		t.Errorf("file_size: got %d want %d", sfcfr.FileSize, sfcfo.FileSize)
	}
	if sfcfr.DocumentID != sfcfo.DocumentID {
		t.Errorf("document_id: got %q want %q", sfcfr.DocumentID, sfcfo.DocumentID)
	}
}

func TestWriterRoundtripCorrectionDataHash(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config.h5"))
	for _, name := range []string{"Correction_Data", "Inverse_Correction_Data"} {
		a, _ := cbGrid(t, orig, 0, name)
		b, _ := cbGrid(t, rt, 0, name)
		if correctionHash(a) != correctionHash(b) {
			t.Errorf("%s hash changed across roundtrip", name)
		}
	}
}

func TestWriterRejectsUnknownFileVersion(t *testing.T) {
	src := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(src).Parse()
	if err != nil {
		t.Fatal(err)
	}
	cfg.Meta.FileVersion = "2.0"
	werr := machineconfig.NewWriter().Write(cfg, filepath.Join(t.TempDir(), "bad.h5"))
	if werr == nil {
		t.Fatal("expected error for unknown file version, got nil")
	}
	var uvErr *machineconfig.UnsupportedFileVersionError
	if !isUnsupportedVersionError(werr, &uvErr) {
		t.Fatalf("expected *UnsupportedFileVersionError, got %T: %v", werr, werr)
	}
	if uvErr.Version != "2.0" {
		t.Errorf("Version = %q, want %q", uvErr.Version, "2.0")
	}
}

func isUnsupportedVersionError(err error, target **machineconfig.UnsupportedFileVersionError) bool {
	if e, ok := err.(*machineconfig.UnsupportedFileVersionError); ok {
		*target = e
		return true
	}
	return false
}

// DISPATCH_REGISTRY_PLAN.md — proves ResolveWriter is genuinely
// registry-driven, not a relocated hardcoded check.
type fakeWriterAdapter struct{ called *bool }

func (f fakeWriterAdapter) Write(cfg *machineconfig.MachineConfig, path string) error {
	*f.called = true
	return nil
}

func TestWriterRegistryRejectsUnregisteredVersion(t *testing.T) {
	registry := machineconfig.WriterRegistry{}
	_, err := machineconfig.ResolveWriter("9.9-nope", registry)
	var u *machineconfig.UnsupportedFileVersionError
	if !isUnsupportedVersionError(err, &u) {
		t.Fatalf("expected *UnsupportedFileVersionError, got %T: %v", err, err)
	}
	if u.Version != "9.9-nope" {
		t.Errorf("Version = %q, want %q", u.Version, "9.9-nope")
	}
}

func TestWriterRegistryDispatchesViaInjectedAdapter(t *testing.T) {
	called := false
	registry := machineconfig.WriterRegistry{
		"9.9-test": fakeWriterAdapter{called: &called},
	}
	adapter, err := machineconfig.ResolveWriter("9.9-test", registry)
	if err != nil {
		t.Fatal(err)
	}
	if err := adapter.Write(&machineconfig.MachineConfig{}, "unused.h5"); err != nil {
		t.Fatal(err)
	}
	if !called {
		t.Fatal("expected the fake adapter's Write to be called — dispatch did not route to the fake")
	}
}

// Prerequisite fix (CONFIGURATION_HASH_PLAN.md): writers must no longer
// materialize a hardcoded default (e.g. "mm") for an unset *_unit field —
// that default was never real data, just the reader's own Rule-8 locked
// constant written early for no benefit, and it broke pre-write/post-write
// hash agreement. One representative field per affected struct, mirroring
// Python's/Rust's/Node.js's equivalent test's shape exactly.
func TestUnsetUnitFieldsRoundTripAsNilNotADefault(t *testing.T) {
	cfg := machineconfig.NewMockConfigBuilder()
	cfg.NLasers = 1
	built := cfg.Build()

	built.Machine.BuildPlateRadiusUnit = nil
	for i := range built.OpticalTrains {
		built.OpticalTrains[i].MajorAxisAngleUnit = nil
		built.OpticalTrains[i].Scanner.WorkingDistanceUnit = nil
		built.OpticalTrains[i].LightSource.WavelengthUnit = nil
		built.OpticalTrains[i].Collimator.FocalLengthUnit = nil
		built.OpticalTrains[i].ScannerCard.SamplePeriodUnit = nil
	}

	tmp := filepath.Join(t.TempDir(), "unset_units.h5")
	if err := machineconfig.NewWriter().Write(built, tmp); err != nil {
		t.Fatal(err)
	}
	rt, err := machineconfig.NewReader(tmp).Parse()
	if err != nil {
		t.Fatal(err)
	}

	if rt.Machine.BuildPlateRadiusUnit != nil {
		t.Errorf("Machine.BuildPlateRadiusUnit = %q, want nil", *rt.Machine.BuildPlateRadiusUnit)
	}
	if rt.OpticalTrains[0].MajorAxisAngleUnit != nil {
		t.Errorf("OpticalTrains[0].MajorAxisAngleUnit = %q, want nil", *rt.OpticalTrains[0].MajorAxisAngleUnit)
	}
	if rt.OpticalTrains[0].Scanner.WorkingDistanceUnit != nil {
		t.Errorf("Scanner.WorkingDistanceUnit = %q, want nil", *rt.OpticalTrains[0].Scanner.WorkingDistanceUnit)
	}
	if rt.OpticalTrains[0].LightSource.WavelengthUnit != nil {
		t.Errorf("LightSource.WavelengthUnit = %q, want nil", *rt.OpticalTrains[0].LightSource.WavelengthUnit)
	}
	if rt.OpticalTrains[0].Collimator.FocalLengthUnit != nil {
		t.Errorf("Collimator.FocalLengthUnit = %q, want nil", *rt.OpticalTrains[0].Collimator.FocalLengthUnit)
	}
	if rt.OpticalTrains[0].ScannerCard.SamplePeriodUnit != nil {
		t.Errorf("ScannerCard.SamplePeriodUnit = %q, want nil", *rt.OpticalTrains[0].ScannerCard.SamplePeriodUnit)
	}
}
