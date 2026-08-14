package machineconfig_test

import (
	"crypto/sha256"
	"encoding/binary"
	"math"
	"path/filepath"
	"testing"

	machineconfig "machine-config-go"
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
