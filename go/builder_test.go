package machineconfig

import (
	"math"
	"path/filepath"
	"testing"
)

func TestMockBuilderDefaultsToTwoLasers(t *testing.T) {
	cfg := NewMockConfigBuilder().Build()
	if len(cfg.OpticalTrains) != 2 {
		t.Fatalf("expected 2 optical trains, got %d", len(cfg.OpticalTrains))
	}
}

func TestMockBuilderNLasersOverride(t *testing.T) {
	b := NewMockConfigBuilder()
	b.NLasers = 3
	cfg := b.Build()
	if len(cfg.OpticalTrains) != 3 {
		t.Fatalf("expected 3 optical trains, got %d", len(cfg.OpticalTrains))
	}
}

func TestMockBuilderMachineName(t *testing.T) {
	cfg := NewMockConfigBuilder().Build()
	if cfg.Meta.MachineName != "MockMachine" {
		t.Fatalf("meta.MachineName = %q, want MockMachine", cfg.Meta.MachineName)
	}
}

func TestMockBuilderBuildPlateDimensions(t *testing.T) {
	cfg := NewMockConfigBuilder().Build()
	if cfg.Machine.BuildPlateX == nil || *cfg.Machine.BuildPlateX != 250.0 {
		t.Fatalf("build_plate_x = %v, want 250.0", cfg.Machine.BuildPlateX)
	}
	if cfg.Machine.BuildPlateY == nil || *cfg.Machine.BuildPlateY != 250.0 {
		t.Fatalf("build_plate_y = %v, want 250.0", cfg.Machine.BuildPlateY)
	}
}

func TestMockBuilderConfigurationHashLength(t *testing.T) {
	cfg := NewMockConfigBuilder().Build()
	if len(cfg.Meta.ConfigurationHash) != 64 {
		t.Fatalf("configuration_hash len = %d, want 64", len(cfg.Meta.ConfigurationHash))
	}
}

func TestMockBuilderCorrectionGridShape(t *testing.T) {
	cfg := NewMockConfigBuilder().Build()
	cb := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	if cb == nil {
		t.Fatal("expected clearbox, got nil")
	}
	if cb.CorrectionData == nil {
		t.Fatal("correction_data is nil")
	}
	cd := *cb.CorrectionData
	if len(cd) != 257 {
		t.Fatalf("correction_data dim0 = %d, want 257", len(cd))
	}
	if len(cd[0]) != 257 {
		t.Fatalf("correction_data dim1 = %d, want 257", len(cd[0]))
	}
	if len(cd[0][0]) != 2 {
		t.Fatalf("correction_data dim2 = %d, want 2", len(cd[0][0]))
	}
}

func TestMockBuilderCorrectionGridPeak(t *testing.T) {
	cfg := NewMockConfigBuilder().Build()
	cb := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	if cb == nil || cb.CorrectionData == nil {
		t.Fatal("no clearbox or correction data")
	}
	cd := *cb.CorrectionData
	cell := cd[128][128][0]
	if cell == nil {
		t.Fatal("centre cell is nil")
	}
	if math.Abs(*cell-2.0) > 0.01 {
		t.Fatalf("centre cell = %v, want ~2.0", *cell)
	}
}

func TestMockBuilderInverseGridRatio(t *testing.T) {
	cfg := NewMockConfigBuilder().Build()
	cb := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	if cb == nil || cb.CorrectionData == nil || cb.InverseCorrectionData == nil {
		t.Fatal("missing clearbox or grid data")
	}
	fwd := (*cb.CorrectionData)[128][128][0]
	inv := (*cb.InverseCorrectionData)[128][128][0]
	if fwd == nil || inv == nil {
		t.Fatal("centre cell is nil")
	}
	ratio := *inv / *fwd
	if math.Abs(ratio-0.9) > 1e-9 {
		t.Fatalf("inverse/forward ratio = %v, want 0.9", ratio)
	}
}

func TestMockBuilderNoClearboxPath(t *testing.T) {
	b := NewMockConfigBuilder()
	b.IncludeClearbox = false
	cfg := b.Build()
	train := cfg.OpticalTrains[0]
	if train.OptionalComponents.Clearbox != nil {
		t.Fatal("expected no clearbox, got one")
	}
	if train.ScanFieldCorrectionFile != nil {
		t.Fatal("expected no SFCF, got one")
	}
}

func TestMockBuilderSaveRoundtrip(t *testing.T) {
	path := filepath.Join(t.TempDir(), "mock.h5")
	if err := NewMockConfigBuilder().Save(path); err != nil {
		t.Fatalf("Save: %v", err)
	}
	cfg, err := NewReader(path).Parse()
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
	cfg := NewMockConfigBuilder().Build()

	rot0 := cfg.OpticalTrains[0].Scanner.ScanHeadRotation
	if rot0 == nil || *rot0 != 0.0 {
		t.Fatalf("train[0] scan_head_rotation = %v, want 0.0", rot0)
	}

	rot1 := cfg.OpticalTrains[1].Scanner.ScanHeadRotation
	if rot1 == nil || *rot1 != 180.0 {
		t.Fatalf("train[1] scan_head_rotation = %v, want 180.0", rot1)
	}
}
