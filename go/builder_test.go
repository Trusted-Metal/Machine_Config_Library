package machineconfig_test

import (
	"math"
	"path/filepath"
	"testing"

	machineconfig "machine-config-go"
)

func TestMockBuilderDefaultsToTwoLasers(t *testing.T) {
	cfg := machineconfig.NewMockConfigBuilder().Build()
	if len(cfg.OpticalTrains) != 2 {
		t.Fatalf("expected 2 optical trains, got %d", len(cfg.OpticalTrains))
	}
}

func TestMockBuilderNLasersOverride(t *testing.T) {
	b := machineconfig.NewMockConfigBuilder()
	b.NLasers = 3
	cfg := b.Build()
	if len(cfg.OpticalTrains) != 3 {
		t.Fatalf("expected 3 optical trains, got %d", len(cfg.OpticalTrains))
	}
}

func TestMockBuilderMachineName(t *testing.T) {
	cfg := machineconfig.NewMockConfigBuilder().Build()
	if cfg.Meta.MachineName != "MockMachine" {
		t.Fatalf("meta.MachineName = %q, want MockMachine", cfg.Meta.MachineName)
	}
}

func TestMockBuilderBuildPlateDimensions(t *testing.T) {
	cfg := machineconfig.NewMockConfigBuilder().Build()
	if cfg.Machine.BuildPlateX == nil || *cfg.Machine.BuildPlateX != 250.0 {
		t.Fatalf("build_plate_x = %v, want 250.0", cfg.Machine.BuildPlateX)
	}
	if cfg.Machine.BuildPlateY == nil || *cfg.Machine.BuildPlateY != 250.0 {
		t.Fatalf("build_plate_y = %v, want 250.0", cfg.Machine.BuildPlateY)
	}
}

func TestMockBuilderConfigurationHashLength(t *testing.T) {
	cfg := machineconfig.NewMockConfigBuilder().Build()
	if len(cfg.Meta.ConfigurationHash) != 64 {
		t.Fatalf("configuration_hash len = %d, want 64", len(cfg.Meta.ConfigurationHash))
	}
}

func TestMockBuilderCorrectionGridShape(t *testing.T) {
	cfg := machineconfig.NewMockConfigBuilder().Build()
	data, shape := cbGrid(t, cfg, 0, "Correction_Data")
	if len(shape) != 3 || shape[0] != 257 || shape[1] != 257 || shape[2] != 2 {
		t.Fatalf("correction_data shape = %v, want [257 257 2]", shape)
	}
	if len(data) != 257*257*2 {
		t.Fatalf("correction_data len = %d", len(data))
	}
}

func TestMockBuilderCorrectionGridPeak(t *testing.T) {
	cfg := machineconfig.NewMockConfigBuilder().Build()
	data, _ := cbGrid(t, cfg, 0, "Correction_Data")
	cell := data[(128*257+128)*2]
	if math.Abs(cell-2.0) > 0.01 {
		t.Fatalf("centre cell = %v, want ~2.0", cell)
	}
}

func TestMockBuilderInverseGridRatio(t *testing.T) {
	cfg := machineconfig.NewMockConfigBuilder().Build()
	fwdData, _ := cbGrid(t, cfg, 0, "Correction_Data")
	invData, _ := cbGrid(t, cfg, 0, "Inverse_Correction_Data")
	i := (128*257 + 128) * 2
	ratio := invData[i] / fwdData[i]
	if math.Abs(ratio-0.9) > 1e-9 {
		t.Fatalf("inverse/forward ratio = %v, want 0.9", ratio)
	}
}

func TestMockBuilderNoClearboxPath(t *testing.T) {
	b := machineconfig.NewMockConfigBuilder()
	b.IncludeClearbox = false
	cfg := b.Build()
	if _, ok := cfg.Extensions["ClearBox"]; ok {
		t.Fatal("expected no ClearBox extension, got one")
	}
	if cfg.OpticalTrains[0].ScanFieldCorrectionFile != nil {
		t.Fatal("expected no SFCF, got one")
	}
}

func TestMockBuilderSaveRoundtrip(t *testing.T) {
	path := filepath.Join(t.TempDir(), "mock.h5")
	if err := machineconfig.NewMockConfigBuilder().Save(path); err != nil {
		t.Fatalf("Save: %v", err)
	}
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatalf("Parse: %v", err)
	}
	if len(cfg.OpticalTrains) != 2 {
		t.Fatalf("expected 2 trains after roundtrip, got %d", len(cfg.OpticalTrains))
	}
	if cfg.Meta.MachineName == "" {
		t.Fatal("meta.machine_name is empty after roundtrip")
	}
	if cfg.OpticalTrains[0].Scanner.WorkingDistance == nil {
		t.Fatal("train[0].scanner.working_distance is nil after roundtrip")
	}
}

func TestMockBuilderScanHeadRotation(t *testing.T) {
	cfg := machineconfig.NewMockConfigBuilder().Build()

	rot0 := cfg.OpticalTrains[0].Scanner.ScanHeadRotation
	if rot0 == nil || *rot0 != 0.0 {
		t.Fatalf("train[0] scan_head_rotation = %v, want 0.0", rot0)
	}

	rot1 := cfg.OpticalTrains[1].Scanner.ScanHeadRotation
	if rot1 == nil || *rot1 != 180.0 {
		t.Fatalf("train[1] scan_head_rotation = %v, want 180.0", rot1)
	}
}
