package machineconfig_test

import (
	"math"
	"path/filepath"
	"runtime"
	"testing"

	machineconfig "machine-config-go"
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
	cb := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	if cb == nil {
		t.Fatal("expected clearbox")
	}
	if cb.IPAddress == "" {
		t.Fatal("clearbox ip empty")
	}
}

func TestOpcuaAbsentAndPresent(t *testing.T) {
	dir := fixturesDir(t)
	cfg, err := machineconfig.NewReader(filepath.Join(dir, "reference_config.h5")).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Opcua != nil {
		t.Fatal("expected no opcua on reference_config")
	}
	cfg2, err := machineconfig.NewReader(filepath.Join(dir, "reference_config_opcua.h5")).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if cfg2.Opcua == nil || cfg2.Opcua.Client.ServerURL == "" {
		t.Fatalf("expected opcua client, got %#v", cfg2.Opcua)
	}
}

func TestGetCorrectionData(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cd, err := machineconfig.NewReader(path).GetCorrectionData(0)
	if err != nil {
		t.Fatal(err)
	}
	if cd.Shape != [3]int{257, 257, 2} {
		t.Fatalf("shape = %v", cd.Shape)
	}
	nan := 0
	for _, v := range cd.Data {
		if math.IsNaN(v) {
			nan++
		}
	}
	if nan == 0 {
		t.Fatal("expected NaNs in correction grid")
	}
}
