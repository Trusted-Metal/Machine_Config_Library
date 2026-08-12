package capabilities_test

import (
	"path/filepath"
	"runtime"
	"testing"

	"machine-config-go/capabilities"
	machineconfig "machine-config-go"
)

func fixturesDir(t *testing.T) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	return filepath.Clean(filepath.Join(filepath.Dir(file), "..", "..", "fixtures"))
}

func TestOpenGetScanner(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	f, err := capabilities.OpenMachineConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	if f.FileVersion() != "1.0" {
		t.Fatalf("version %q", f.FileVersion())
	}
	sc, err := f.GetScanner(0)
	if err != nil {
		t.Fatal(err)
	}
	if sc.WorkingDistance == nil || *sc.WorkingDistance != 670 {
		t.Fatalf("working_distance = %v", sc.WorkingDistance)
	}
	cfg, _ := machineconfig.NewReader(path).Parse()
	if sc.Manufacturer != cfg.OpticalTrains[0].Scanner.Manufacturer {
		t.Fatalf("manufacturer mismatch")
	}
}

func TestMergeSetScannerInMemory(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	f, err := capabilities.OpenMachineConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	sc, _ := f.GetScanner(0)
	mfr := sc.Manufacturer
	sc.WorkingDistance = machineconfig.Float64Ptr(123.5)
	if err := f.SetScanner(0, sc, capabilities.Merge); err != nil {
		t.Fatal(err)
	}
	after, _ := f.GetScanner(0)
	if after.WorkingDistance == nil || *after.WorkingDistance != 123.5 {
		t.Fatalf("got %v", after.WorkingDistance)
	}
	if after.Manufacturer != mfr {
		t.Fatalf("manufacturer changed under Merge")
	}
}

func TestInvalidIndexAndOpcua(t *testing.T) {
	dir := fixturesDir(t)
	f, err := capabilities.OpenMachineConfig(filepath.Join(dir, "reference_config.h5"))
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	if _, e := f.GetScanner(999); e == nil || e.Code != capabilities.ErrInvalidIndex {
		t.Fatalf("expected InvalidIndex, got %#v", e)
	}
	if _, e := f.GetOpcua(); e == nil || e.Code != capabilities.ErrNotPresent {
		t.Fatalf("expected NotPresent, got %#v", e)
	}
	f2, err := capabilities.OpenMachineConfig(filepath.Join(dir, "reference_config_opcua.h5"))
	if err != nil {
		t.Fatal(err)
	}
	defer f2.Close()
	if _, e := f2.GetOpcua(); e != nil {
		t.Fatal(e)
	}
}

func TestCreate(t *testing.T) {
	f, err := capabilities.CreateMachineConfig("1.0")
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	if f.FileVersion() != "1.0" {
		t.Fatal(f.FileVersion())
	}
	meta, _ := f.GetMeta()
	meta.MachineName = "CreatedMachine"
	if e := f.SetMeta(meta, capabilities.Merge); e != nil {
		t.Fatal(e)
	}
	again, _ := f.GetMeta()
	if again.MachineName != "CreatedMachine" {
		t.Fatal(again.MachineName)
	}
}
