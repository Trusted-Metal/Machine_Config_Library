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

func TestReplaceSetScanner(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	f, err := capabilities.OpenMachineConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	before, _ := f.GetScanner(0)
	if before.WorkingDistance == nil {
		t.Skip("reference fixture has no working_distance — cannot verify Replace zeroes it")
	}
	replacement := machineconfig.Scanner{
		Manufacturer: "ReplaceCo",
		Model:        "ReplaceModel",
		SerialNumber: "R-1",
	}
	if e := f.SetScanner(0, replacement, capabilities.Replace); e != nil {
		t.Fatal(e)
	}
	after, _ := f.GetScanner(0)
	if after.Manufacturer != "ReplaceCo" {
		t.Fatalf("manufacturer = %q", after.Manufacturer)
	}
	if after.Model == before.Model {
		t.Fatal("model unchanged after Replace")
	}
	if after.WorkingDistance != nil {
		t.Fatalf("expected nil WorkingDistance after Replace, got %v", after.WorkingDistance)
	}
}

func TestSyntheticTwoTrains(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "synthetic_2laser.h5")
	f, err := capabilities.OpenMachineConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	n, e := f.OpticalTrainCount()
	if e != nil {
		t.Fatal(e)
	}
	if n != 2 {
		t.Fatalf("expected 2 trains, got %d", n)
	}
	sc0, e0 := f.GetScanner(0)
	sc1, e1 := f.GetScanner(1)
	if e0 != nil || e1 != nil {
		t.Fatalf("GetScanner errors: %v, %v", e0, e1)
	}
	if sc0.SerialNumber == sc1.SerialNumber {
		t.Fatal("trains share the same scanner serial number")
	}
	if _, e := f.GetScanner(2); e == nil || e.Code != capabilities.ErrInvalidIndex {
		t.Fatalf("expected ErrInvalidIndex for train 2, got %#v", e)
	}
}

func TestOpcuaStructuredFields(t *testing.T) {
	dir := fixturesDir(t)
	f, err := capabilities.OpenMachineConfig(filepath.Join(dir, "reference_config_opcua.h5"))
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	opc, e := f.GetOpcua()
	if e != nil {
		t.Fatal(e)
	}
	if opc.Client.ServerURL == "" {
		t.Fatal("opcua client server_url is empty")
	}
}

func TestOpenNonExistentFileIsIoError(t *testing.T) {
	_, err := capabilities.OpenMachineConfig("/nonexistent/path/to/file.h5")
	if err == nil || err.Code != capabilities.ErrIo {
		t.Fatalf("expected ErrIo, got %#v", err)
	}
}

func TestSaveRoundTrip(t *testing.T) {
	t.Skip("Go writer §5.7 not implemented — re-enable when Save() is complete")
}
