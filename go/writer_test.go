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
	if rt.Meta.ConfigurationHash != orig.Meta.ConfigurationHash {
		t.Errorf("configuration_hash mismatch")
	}
	if len(rt.Meta.ConfigurationHash) != 64 {
		t.Errorf("configuration_hash len = %d", len(rt.Meta.ConfigurationHash))
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

func TestWriterRoundtripClearboxScalars(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config.h5"))
	cbo := orig.OpticalTrains[0].OptionalComponents.Clearbox
	cbr := rt.OpticalTrains[0].OptionalComponents.Clearbox
	if cbo == nil || cbr == nil {
		t.Fatal("expected clearbox on both original and roundtrip")
	}
	if cbr.IPAddress != cbo.IPAddress {
		t.Errorf("ip_address: got %q want %q", cbr.IPAddress, cbo.IPAddress)
	}
	if (cbr.DataPort == nil) != (cbo.DataPort == nil) || (cbo.DataPort != nil && *cbr.DataPort != *cbo.DataPort) {
		t.Errorf("data_port: got %v want %v", cbr.DataPort, cbo.DataPort)
	}
	if (cbr.ShowConsole == nil) != (cbo.ShowConsole == nil) || (cbo.ShowConsole != nil && *cbr.ShowConsole != *cbo.ShowConsole) {
		t.Errorf("show_console: got %v want %v", cbr.ShowConsole, cbo.ShowConsole)
	}
	if (cbr.CorrectionGridDomainShape == nil) != (cbo.CorrectionGridDomainShape == nil) || (cbo.CorrectionGridDomainShape != nil && *cbr.CorrectionGridDomainShape != *cbo.CorrectionGridDomainShape) {
		t.Errorf("correction_grid_domain_shape: got %v want %v", cbr.CorrectionGridDomainShape, cbo.CorrectionGridDomainShape)
	}
	if len(cbr.SynchronousSensors) != 0 {
		t.Errorf("expected no sensors on reference_config.h5 roundtrip, got %v", cbr.SynchronousSensors)
	}
}

// ---------------------------------------------------------------------------
// Roundtrip: SynchronousSensor
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

func TestWriterRoundtripSynchronousSensorCompoundDatasetsExactValuesInOrder(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config_synchronous_sensors.h5"))
	so := orig.OpticalTrains[0].OptionalComponents.Clearbox.SynchronousSensors["Oxygen Sensor"]
	sr := rt.OpticalTrains[0].OptionalComponents.Clearbox.SynchronousSensors["Oxygen Sensor"]
	if !reflect.DeepEqual(sr.DerivationEquationConstants, so.DerivationEquationConstants) {
		t.Errorf("derivation_equation_constants: got %+v want %+v", sr.DerivationEquationConstants, so.DerivationEquationConstants)
	}
	if !reflect.DeepEqual(sr.CalibrationPoints, so.CalibrationPoints) {
		t.Errorf("calibration_points: got %+v want %+v", sr.CalibrationPoints, so.CalibrationPoints)
	}
	if sr.SensorName == nil || *sr.SensorName != "ZR800 Oxygen Analyzer" {
		t.Errorf("sensor_name: got %v", sr.SensorName)
	}
}

// TestWriterRoundtripSynchronousSensorWithZeroRowCompoundDatasets is a
// distinct edge case from the empty-*map* test above: here the sensor
// itself exists (its group is created), but both compound datasets have
// zero rows — proving 0-length compound dataset creation/read genuinely
// works, not assumed.
func TestWriterRoundtripSynchronousSensorWithZeroRowCompoundDatasets(t *testing.T) {
	cfg, err := machineconfig.NewReader(filepath.Join(fixturesDir(t), "reference_config.h5")).Parse()
	if err != nil {
		t.Fatal(err)
	}
	cb := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	cb.SynchronousSensors = map[string]machineconfig.SynchronousSensor{
		"Untested Sensor": {
			Enabled:                     machineconfig.BoolPtr(false),
			SensorName:                  machineconfig.StrPtr("Placeholder"),
			DerivationEquationConstants: []machineconfig.EquationConstant{},
			CalibrationPoints:           []machineconfig.CalibrationPoint{},
		},
	}
	rt := writeAndRead(t, cfg)
	s, ok := rt.OpticalTrains[0].OptionalComponents.Clearbox.SynchronousSensors["Untested Sensor"]
	if !ok {
		t.Fatal(`expected key "Untested Sensor"`)
	}
	if len(s.DerivationEquationConstants) != 0 {
		t.Errorf("derivation_equation_constants: got %+v, want empty", s.DerivationEquationConstants)
	}
	if len(s.CalibrationPoints) != 0 {
		t.Errorf("calibration_points: got %+v, want empty", s.CalibrationPoints)
	}
	if s.SensorName == nil || *s.SensorName != "Placeholder" {
		t.Errorf("sensor_name: got %v", s.SensorName)
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
	cb := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	cb.SynchronousSensors = map[string]machineconfig.SynchronousSensor{
		"HUMIDITY_SENSOR_2": {
			Enabled:                     machineconfig.BoolPtr(true),
			PortID:                      machineconfig.IntPtr(9),
			DerivationEquationConstants: []machineconfig.EquationConstant{},
			CalibrationPoints:           []machineconfig.CalibrationPoint{},
		},
	}
	rt := writeAndRead(t, cfg)
	sensors := rt.OpticalTrains[0].OptionalComponents.Clearbox.SynchronousSensors
	s, ok := sensors["HUMIDITY_SENSOR_2"]
	if !ok {
		t.Fatalf(`expected key "HUMIDITY_SENSOR_2", got keys %v`, sensors)
	}
	if s.PortID == nil || *s.PortID != 9 {
		t.Errorf("port_id: got %v want 9", s.PortID)
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
	if cfg.Opcua == nil {
		t.Fatal("fixture must already have OPCUA before the test adds a sensor")
	}
	cb := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	cb.SynchronousSensors = map[string]machineconfig.SynchronousSensor{
		"Oxygen Sensor": {
			Enabled:             machineconfig.BoolPtr(true),
			SensorName:          machineconfig.StrPtr("ZR800 Oxygen Analyzer"),
			CalibrationVerified: machineconfig.BoolPtr(false),
			SamplePeriod:        machineconfig.Float64Ptr(5.0),
			DerivationEquationConstants: []machineconfig.EquationConstant{
				{Name: "a", Value: 0.4375},
				{Name: "b", Value: -2.75},
			},
			CalibrationPoints: []machineconfig.CalibrationPoint{
				{InputValue: 4.0, OutputValue: -1.0},
				{InputValue: 20.0, OutputValue: 6.0},
			},
		},
	}
	rt := writeAndRead(t, cfg)
	if rt.Opcua == nil {
		t.Fatal("OPCUA must survive alongside the newly-added sensor")
	}
	s, ok := rt.OpticalTrains[0].OptionalComponents.Clearbox.SynchronousSensors["Oxygen Sensor"]
	if !ok {
		t.Fatal(`expected key "Oxygen Sensor"`)
	}
	want := []machineconfig.EquationConstant{{Name: "a", Value: 0.4375}, {Name: "b", Value: -2.75}}
	if !reflect.DeepEqual(s.DerivationEquationConstants, want) {
		t.Errorf("derivation_equation_constants: got %+v want %+v", s.DerivationEquationConstants, want)
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
	cb := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	cb.SynchronousSensors = map[string]machineconfig.SynchronousSensor{
		"Oversized Name Sensor": {
			DerivationEquationConstants: []machineconfig.EquationConstant{
				{Name: strings.Repeat("a", 65), Value: 1.0},
			},
			CalibrationPoints: []machineconfig.CalibrationPoint{},
		},
	}
	tmp := filepath.Join(t.TempDir(), "oversized.h5")
	err = machineconfig.NewWriter().Write(cfg, tmp)
	if err == nil {
		t.Fatal("expected an error for an oversized constant name, got nil")
	}
	if !strings.Contains(err.Error(), "does not fit") {
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
	src := filepath.Join(fixturesDir(t), "reference_config.h5")
	orig, err := machineconfig.NewReader(src).ParseWithOptions(machineconfig.ParseOptions{IncludeBinary: true})
	if err != nil {
		t.Fatal(err)
	}
	tmp := filepath.Join(t.TempDir(), "rt.h5")
	if err := machineconfig.NewWriter().Write(orig, tmp); err != nil {
		t.Fatal(err)
	}

	fwdOrig, err := machineconfig.NewReader(src).GetCorrectionData(0)
	if err != nil {
		t.Fatal(err)
	}
	fwdRT, err := machineconfig.NewReader(tmp).GetCorrectionData(0)
	if err != nil {
		t.Fatal(err)
	}
	if correctionHash(fwdOrig.Data) != correctionHash(fwdRT.Data) {
		t.Error("forward correction data hash changed across roundtrip")
	}

	invOrig, err := machineconfig.NewReader(src).GetInverseCorrectionData(0)
	if err != nil {
		t.Fatal(err)
	}
	invRT, err := machineconfig.NewReader(tmp).GetInverseCorrectionData(0)
	if err != nil {
		t.Fatal(err)
	}
	if correctionHash(invOrig.Data) != correctionHash(invRT.Data) {
		t.Error("inverse correction data hash changed across roundtrip")
	}
}

func TestWriterRoundtripOPCUA(t *testing.T) {
	orig, rt := roundtrip(t, filepath.Join(fixturesDir(t), "reference_config_opcua.h5"))
	oo, or_ := orig.Opcua, rt.Opcua
	if oo == nil || or_ == nil {
		t.Fatal("expected opcua on both original and roundtrip")
	}
	if or_.Client.ServerURL != oo.Client.ServerURL {
		t.Errorf("server_url: got %q want %q", or_.Client.ServerURL, oo.Client.ServerURL)
	}
	if or_.Client.SessionTimeout != oo.Client.SessionTimeout {
		t.Errorf("session_timeout: got %d want %d", or_.Client.SessionTimeout, oo.Client.SessionTimeout)
	}
	if or_.Client.BfsMaxDepth != oo.Client.BfsMaxDepth {
		t.Errorf("bfs_max_depth: got %d want %d", or_.Client.BfsMaxDepth, oo.Client.BfsMaxDepth)
	}
	if or_.Pipe.BufferSize != oo.Pipe.BufferSize {
		t.Errorf("pipe.buffer_size: got %d want %d", or_.Pipe.BufferSize, oo.Pipe.BufferSize)
	}
	if (or_.TriggersEnabled == nil) != (oo.TriggersEnabled == nil) || (oo.TriggersEnabled != nil && *or_.TriggersEnabled != *oo.TriggersEnabled) {
		t.Errorf("triggers_enabled: got %v want %v", or_.TriggersEnabled, oo.TriggersEnabled)
	}
	if len(or_.Triggers) != len(oo.Triggers) {
		t.Errorf("triggers count: got %d want %d", len(or_.Triggers), len(oo.Triggers))
	}
	key := "Chamber Oxygen Level"
	to, tr := oo.Triggers[key], or_.Triggers[key]
	if (tr.Signal == nil) != (to.Signal == nil) || (to.Signal != nil && *tr.Signal != *to.Signal) {
		t.Errorf("%q signal: got %v want %v", key, tr.Signal, to.Signal)
	}
	if (tr.Subsystem == nil) != (to.Subsystem == nil) || (to.Subsystem != nil && *tr.Subsystem != *to.Subsystem) {
		t.Errorf("%q subsystem: got %v want %v", key, tr.Subsystem, to.Subsystem)
	}
	if (tr.RuleEnabled == nil) != (to.RuleEnabled == nil) || (to.RuleEnabled != nil && *tr.RuleEnabled != *to.RuleEnabled) {
		t.Errorf("%q rule_enabled: got %v want %v", key, tr.RuleEnabled, to.RuleEnabled)
	}
	if (tr.StartValue == nil) != (to.StartValue == nil) || (to.StartValue != nil && *tr.StartValue != *to.StartValue) {
		t.Errorf("%q start_value: got %v want %v", key, tr.StartValue, to.StartValue)
	}
	if (tr.StopValue == nil) != (to.StopValue == nil) || (to.StopValue != nil && *tr.StopValue != *to.StopValue) {
		t.Errorf("%q stop_value: got %v want %v", key, tr.StopValue, to.StopValue)
	}

	// Promoted fields (OPCUA_FIELD_PROMOTION_PLAN.md Phase 1) — all 22
	// promoted fields survive a real write→read cycle, not just parsing.
	oc, rc := oo.Client, or_.Client
	if (rc.KeepAliveCount == nil) != (oc.KeepAliveCount == nil) || (oc.KeepAliveCount != nil && *rc.KeepAliveCount != *oc.KeepAliveCount) {
		t.Errorf("client.keep_alive_count: got %v want %v", rc.KeepAliveCount, oc.KeepAliveCount)
	}
	if (rc.LifetimeCount == nil) != (oc.LifetimeCount == nil) || (oc.LifetimeCount != nil && *rc.LifetimeCount != *oc.LifetimeCount) {
		t.Errorf("client.lifetime_count: got %v want %v", rc.LifetimeCount, oc.LifetimeCount)
	}
	if (rc.MachineProfile == nil) != (oc.MachineProfile == nil) || (oc.MachineProfile != nil && *rc.MachineProfile != *oc.MachineProfile) {
		t.Errorf("client.machine_profile: got %v want %v", rc.MachineProfile, oc.MachineProfile)
	}
	if (rc.QueuePolicy == nil) != (oc.QueuePolicy == nil) || (oc.QueuePolicy != nil && *rc.QueuePolicy != *oc.QueuePolicy) {
		t.Errorf("client.queue_policy: got %v want %v", rc.QueuePolicy, oc.QueuePolicy)
	}
	if (rc.QueueSizeDataChange == nil) != (oc.QueueSizeDataChange == nil) || (oc.QueueSizeDataChange != nil && *rc.QueueSizeDataChange != *oc.QueueSizeDataChange) {
		t.Errorf("client.queue_size_data_change: got %v want %v", rc.QueueSizeDataChange, oc.QueueSizeDataChange)
	}
	if (rc.QueueSizeEvents == nil) != (oc.QueueSizeEvents == nil) || (oc.QueueSizeEvents != nil && *rc.QueueSizeEvents != *oc.QueueSizeEvents) {
		t.Errorf("client.queue_size_events: got %v want %v", rc.QueueSizeEvents, oc.QueueSizeEvents)
	}
	if (rc.ReconnectInterval == nil) != (oc.ReconnectInterval == nil) || (oc.ReconnectInterval != nil && *rc.ReconnectInterval != *oc.ReconnectInterval) {
		t.Errorf("client.reconnect_interval: got %v want %v", rc.ReconnectInterval, oc.ReconnectInterval)
	}
	if (rc.RootNode == nil) != (oc.RootNode == nil) || (oc.RootNode != nil && *rc.RootNode != *oc.RootNode) {
		t.Errorf("client.root_node: got %v want %v", rc.RootNode, oc.RootNode)
	}
	if (rc.SyncLoopIntervalInitial == nil) != (oc.SyncLoopIntervalInitial == nil) || (oc.SyncLoopIntervalInitial != nil && *rc.SyncLoopIntervalInitial != *oc.SyncLoopIntervalInitial) {
		t.Errorf("client.sync_loop_interval_initial: got %v want %v", rc.SyncLoopIntervalInitial, oc.SyncLoopIntervalInitial)
	}
	if (rc.SyncLoopIntervalSettled == nil) != (oc.SyncLoopIntervalSettled == nil) || (oc.SyncLoopIntervalSettled != nil && *rc.SyncLoopIntervalSettled != *oc.SyncLoopIntervalSettled) {
		t.Errorf("client.sync_loop_interval_settled: got %v want %v", rc.SyncLoopIntervalSettled, oc.SyncLoopIntervalSettled)
	}

	op, rp := oo.Pipe, or_.Pipe
	if (rp.ConfigureClient == nil) != (op.ConfigureClient == nil) || (op.ConfigureClient != nil && *rp.ConfigureClient != *op.ConfigureClient) {
		t.Errorf("pipe.configure_client: got %v want %v", rp.ConfigureClient, op.ConfigureClient)
	}
	if (rp.InboundRateLimit == nil) != (op.InboundRateLimit == nil) || (op.InboundRateLimit != nil && *rp.InboundRateLimit != *op.InboundRateLimit) {
		t.Errorf("pipe.inbound_rate_limit: got %v want %v", rp.InboundRateLimit, op.InboundRateLimit)
	}
	if (rp.MaxInboundMessageSize == nil) != (op.MaxInboundMessageSize == nil) || (op.MaxInboundMessageSize != nil && *rp.MaxInboundMessageSize != *op.MaxInboundMessageSize) {
		t.Errorf("pipe.max_inbound_message_size: got %v want %v", rp.MaxInboundMessageSize, op.MaxInboundMessageSize)
	}
	if (rp.MinIntegrityLevel == nil) != (op.MinIntegrityLevel == nil) || (op.MinIntegrityLevel != nil && *rp.MinIntegrityLevel != *op.MinIntegrityLevel) {
		t.Errorf("pipe.min_integrity_level: got %v want %v", rp.MinIntegrityLevel, op.MinIntegrityLevel)
	}
	if (rp.PipeName == nil) != (op.PipeName == nil) || (op.PipeName != nil && *rp.PipeName != *op.PipeName) {
		t.Errorf("pipe.pipe_name: got %v want %v", rp.PipeName, op.PipeName)
	}
	if (rp.UserAccessLevel == nil) != (op.UserAccessLevel == nil) || (op.UserAccessLevel != nil && *rp.UserAccessLevel != *op.UserAccessLevel) {
		t.Errorf("pipe.user_access_level: got %v want %v", rp.UserAccessLevel, op.UserAccessLevel)
	}

	if (or_.TriggerStopCeilingLayers == nil) != (oo.TriggerStopCeilingLayers == nil) || (oo.TriggerStopCeilingLayers != nil && *or_.TriggerStopCeilingLayers != *oo.TriggerStopCeilingLayers) {
		t.Errorf("trigger_stop_ceiling_layers: got %v want %v", or_.TriggerStopCeilingLayers, oo.TriggerStopCeilingLayers)
	}
	if oo.TriggerStopCeilingLayers == nil || *oo.TriggerStopCeilingLayers != 3 {
		t.Fatalf("trigger_stop_ceiling_layers expected 3, got %v", oo.TriggerStopCeilingLayers)
	}

	if (tr.CaseSensitivity == nil) != (to.CaseSensitivity == nil) || (to.CaseSensitivity != nil && *tr.CaseSensitivity != *to.CaseSensitivity) {
		t.Errorf("%q case_sensitivity: got %v want %v", key, tr.CaseSensitivity, to.CaseSensitivity)
	}
	if (tr.Component == nil) != (to.Component == nil) || (to.Component != nil && *tr.Component != *to.Component) {
		t.Errorf("%q component: got %v want %v", key, tr.Component, to.Component)
	}
	if (tr.CooldownPeriod == nil) != (to.CooldownPeriod == nil) || (to.CooldownPeriod != nil && *tr.CooldownPeriod != *to.CooldownPeriod) {
		t.Errorf("%q cooldown_period: got %v want %v", key, tr.CooldownPeriod, to.CooldownPeriod)
	}
	if (tr.Event == nil) != (to.Event == nil) || (to.Event != nil && *tr.Event != *to.Event) {
		t.Errorf("%q event: got %v want %v", key, tr.Event, to.Event)
	}
	if (tr.MaxFiresPerJob == nil) != (to.MaxFiresPerJob == nil) || (to.MaxFiresPerJob != nil && *tr.MaxFiresPerJob != *to.MaxFiresPerJob) {
		t.Errorf("%q max_fires_per_job: got %v want %v", key, tr.MaxFiresPerJob, to.MaxFiresPerJob)
	}
	if (tr.TriggerLabel == nil) != (to.TriggerLabel == nil) || (to.TriggerLabel != nil && *tr.TriggerLabel != *to.TriggerLabel) {
		t.Errorf("%q trigger_label: got %v want %v", key, tr.TriggerLabel, to.TriggerLabel)
	}
}

// TestWriterRoundtripTriggerStopCeilingLayersNilAndSome covers the one field
// with no Extra bucket to fall back on if the write/read pairing were
// mismatched — both the real value (3) and the nil case.
func TestWriterRoundtripTriggerStopCeilingLayersNilAndSome(t *testing.T) {
	src := filepath.Join(fixturesDir(t), "reference_config_opcua.h5")

	_, rtSome := roundtrip(t, src)
	if rtSome.Opcua.TriggerStopCeilingLayers == nil || *rtSome.Opcua.TriggerStopCeilingLayers != 3 {
		t.Fatalf("trigger_stop_ceiling_layers expected 3, got %v", rtSome.Opcua.TriggerStopCeilingLayers)
	}

	cfg, err := machineconfig.NewReader(src).Parse()
	if err != nil {
		t.Fatal(err)
	}
	cfg.Opcua.TriggerStopCeilingLayers = nil
	tmp := filepath.Join(t.TempDir(), "ceiling_nil.h5")
	if err := machineconfig.NewWriter().Write(cfg, tmp); err != nil {
		t.Fatal(err)
	}
	rtNil, err := machineconfig.NewReader(tmp).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if rtNil.Opcua.TriggerStopCeilingLayers != nil {
		t.Fatalf("trigger_stop_ceiling_layers expected nil, got %v", *rtNil.Opcua.TriggerStopCeilingLayers)
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
