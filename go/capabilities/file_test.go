package capabilities_test

import (
	"math"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	machineconfig "machine-config-go"
	"machine-config-go/capabilities"
)

// correctionDataEqual compares two CorrectionData buffers treating "both NaN"
// as equal — plain == (and reflect.DeepEqual) treat NaN != NaN, which would
// make a straightforward comparison fail even on bit-for-bit identical
// real correction grids (which always contain NaN cells).
func correctionDataEqual(a, b *machineconfig.CorrectionData) bool {
	if a.Shape != b.Shape {
		return false
	}
	if len(a.Data) != len(b.Data) {
		return false
	}
	for i := range a.Data {
		x, y := a.Data[i], b.Data[i]
		if math.IsNaN(x) || math.IsNaN(y) {
			if !(math.IsNaN(x) && math.IsNaN(y)) {
				return false
			}
			continue
		}
		if x != y {
			return false
		}
	}
	return true
}

func fixturesDir(t *testing.T) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	return filepath.Clean(filepath.Join(filepath.Dir(file), "..", "..", "fixtures"))
}

func validationFixturesDir(t *testing.T) string {
	t.Helper()
	return filepath.Clean(filepath.Join(fixturesDir(t), "..", "docs", "validation", "fixtures"))
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

func TestOpcuaRequiredFieldsPresentOnReferenceFixture(t *testing.T) {
	f, err := capabilities.OpenMachineConfig(filepath.Join(fixturesDir(t), "reference_config_opcua.h5"))
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	opc, e := f.GetOpcua()
	if e != nil {
		t.Fatalf("expected Ok, got %#v", e)
	}
	if opc.Client.MachineProfile == nil {
		t.Fatal("client.machine_profile should be populated")
	}
	for name, trigger := range opc.Triggers {
		if trigger.Event == nil {
			t.Fatalf("trigger %q event should be populated", name)
		}
	}
}

func TestOpcuaMissingRequiredFieldsReportsAllSevenAtOnce(t *testing.T) {
	path := filepath.Join(validationFixturesDir(t), "opcua_missing_required.h5")
	f, err := capabilities.OpenMachineConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()

	_, e := f.GetOpcua()
	if e == nil {
		t.Fatal("expected ValidationError, got nil")
	}
	if e.Code != capabilities.ErrValidation {
		t.Fatalf("expected ErrValidation, got %q", e.Code)
	}

	expected := map[string]bool{
		"Machine_Profile":                true,
		"Root_Node":                      true,
		"Configure_Client":               true,
		"Pipe_Name":                      true,
		"Triggers_Enabled":               true,
		"Trigger_Stop_Ceiling_Layers":    true,
		"Laser Emission Interlock.Event": true,
	}
	if len(e.Details) != len(expected) {
		t.Fatalf("details = %v, want exactly %d items", e.Details, len(expected))
	}
	for _, d := range e.Details {
		if !expected[d] {
			t.Errorf("unexpected detail %q", d)
		}
		if strings.HasPrefix(d, "Chamber Oxygen Level") {
			t.Errorf("Chamber Oxygen Level must not be reported, got %q", d)
		}
	}
}

func TestOpcuaOptionalFieldNeverAppearsInMissingDetails(t *testing.T) {
	// opcua_missing_required.h5 only clears the 7 required fields — every
	// optional field is still present there, so absence of an optional field
	// from Details would be trivially true. Also clear an optional field
	// (keep_alive_count) in memory, re-write to a temp file, and confirm
	// Details still names exactly the same 7 items, not 8.
	src := filepath.Join(validationFixturesDir(t), "opcua_missing_required.h5")
	cfg, err := machineconfig.NewReader(src).Parse()
	if err != nil {
		t.Fatal(err)
	}
	cfg.Opcua.Client.KeepAliveCount = nil
	tmp := filepath.Join(t.TempDir(), "missing_required_plus_optional.h5")
	if err := machineconfig.NewWriter().Write(cfg, tmp); err != nil {
		t.Fatal(err)
	}

	f, capErr := capabilities.OpenMachineConfig(tmp)
	if capErr != nil {
		t.Fatal(capErr)
	}
	defer f.Close()

	_, e := f.GetOpcua()
	if e == nil {
		t.Fatal("expected ValidationError, got nil")
	}
	for _, d := range e.Details {
		if strings.Contains(d, "Keep_Alive_Count") {
			t.Fatalf("optional field must never appear in details, got %v", e.Details)
		}
	}
	if len(e.Details) != 7 {
		t.Fatalf("clearing an optional field must not change the missing count: got %v", e.Details)
	}
}

func TestOpenNonExistentFileIsIoError(t *testing.T) {
	_, err := capabilities.OpenMachineConfig("/nonexistent/path/to/file.h5")
	if err == nil || err.Code != capabilities.ErrIo {
		t.Fatalf("expected ErrIo, got %#v", err)
	}
}

func TestSaveRoundTrip(t *testing.T) {
	t.Run("reference roundtrip", func(t *testing.T) {
		path := filepath.Join(fixturesDir(t), "reference_config.h5")
		f, err := capabilities.OpenMachineConfig(path)
		if err != nil {
			t.Fatal(err)
		}
		defer f.Close()

		meta, err := f.GetMeta()
		if err != nil {
			t.Fatal(err)
		}
		origName := meta.MachineName
		origHash := meta.ConfigurationHash
		sc, err := f.GetScanner(0)
		if err != nil {
			t.Fatal(err)
		}
		origWD := sc.WorkingDistance
		origMfr := sc.Manufacturer

		out := filepath.Join(t.TempDir(), "rt.h5")
		if err := f.Save(out); err != nil {
			t.Fatal(err)
		}

		f2, err := capabilities.OpenMachineConfig(out)
		if err != nil {
			t.Fatal(err)
		}
		defer f2.Close()

		meta2, err := f2.GetMeta()
		if err != nil {
			t.Fatal(err)
		}
		if meta2.MachineName != origName {
			t.Errorf("machine_name: got %q want %q", meta2.MachineName, origName)
		}
		if len(meta2.ConfigurationHash) != 64 {
			t.Errorf("configuration_hash len %d", len(meta2.ConfigurationHash))
		}
		if meta2.ConfigurationHash != origHash {
			t.Errorf("configuration_hash mismatch")
		}

		sc2, err := f2.GetScanner(0)
		if err != nil {
			t.Fatal(err)
		}
		if (sc2.WorkingDistance == nil) != (origWD == nil) || (origWD != nil && *sc2.WorkingDistance != *origWD) {
			t.Errorf("working_distance: got %v want %v", sc2.WorkingDistance, origWD)
		}
		if sc2.Manufacturer != origMfr {
			t.Errorf("manufacturer: got %q want %q", sc2.Manufacturer, origMfr)
		}

		count, err := f2.OpticalTrainCount()
		if err != nil {
			t.Fatal(err)
		}
		if count != 2 {
			t.Errorf("optical_train_count: got %d want 2", count)
		}
	})

	t.Run("opcua roundtrip", func(t *testing.T) {
		path := filepath.Join(fixturesDir(t), "reference_config_opcua.h5")
		f, err := capabilities.OpenMachineConfig(path)
		if err != nil {
			t.Fatal(err)
		}
		defer f.Close()

		opcua, err := f.GetOpcua()
		if err != nil {
			t.Fatal(err)
		}
		origURL := opcua.Client.ServerURL

		out := filepath.Join(t.TempDir(), "opcua_rt.h5")
		if err := f.Save(out); err != nil {
			t.Fatal(err)
		}

		f2, err := capabilities.OpenMachineConfig(out)
		if err != nil {
			t.Fatal(err)
		}
		defer f2.Close()

		opcua2, err := f2.GetOpcua()
		if err != nil {
			t.Fatal(err)
		}
		if opcua2.Client.ServerURL != origURL {
			t.Errorf("server_url: got %q want %q", opcua2.Client.ServerURL, origURL)
		}
	})

	t.Run("synthetic roundtrip", func(t *testing.T) {
		path := filepath.Join(fixturesDir(t), "synthetic_2laser.h5")
		f, err := capabilities.OpenMachineConfig(path)
		if err != nil {
			t.Fatal(err)
		}
		defer f.Close()

		count, err := f.OpticalTrainCount()
		if err != nil {
			t.Fatal(err)
		}
		if count != 2 {
			t.Fatalf("expected 2 trains, got %d", count)
		}
		sc0, err := f.GetScanner(0)
		if err != nil {
			t.Fatal(err)
		}
		sc1, err := f.GetScanner(1)
		if err != nil {
			t.Fatal(err)
		}

		out := filepath.Join(t.TempDir(), "synth_rt.h5")
		if err := f.Save(out); err != nil {
			t.Fatal(err)
		}

		f2, err := capabilities.OpenMachineConfig(out)
		if err != nil {
			t.Fatal(err)
		}
		defer f2.Close()

		count2, err := f2.OpticalTrainCount()
		if err != nil {
			t.Fatal(err)
		}
		if count2 != 2 {
			t.Errorf("optical_train_count: got %d want 2", count2)
		}

		rt0, err := f2.GetScanner(0)
		if err != nil {
			t.Fatal(err)
		}
		rt1, err := f2.GetScanner(1)
		if err != nil {
			t.Fatal(err)
		}
		if rt0.SerialNumber == rt1.SerialNumber {
			t.Errorf("train serial numbers should differ; both = %q", rt0.SerialNumber)
		}
		if rt0.SerialNumber != sc0.SerialNumber {
			t.Errorf("train 0 serial: got %q want %q", rt0.SerialNumber, sc0.SerialNumber)
		}
		if rt1.SerialNumber != sc1.SerialNumber {
			t.Errorf("train 1 serial: got %q want %q", rt1.SerialNumber, sc1.SerialNumber)
		}
	})
}

func TestGetCorrectionDataMatchesReader(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	f, err := capabilities.OpenMachineConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()

	expected, rdErr := machineconfig.NewReader(path).GetCorrectionData(0)
	if rdErr != nil {
		t.Fatal(rdErr)
	}
	got, capErr := f.GetCorrectionData(0)
	if capErr != nil {
		t.Fatal(capErr)
	}
	if !correctionDataEqual(got, expected) {
		t.Fatalf("facade GetCorrectionData does not match Reader")
	}

	expectedInv, rdErr := machineconfig.NewReader(path).GetInverseCorrectionData(0)
	if rdErr != nil {
		t.Fatal(rdErr)
	}
	gotInv, capErr := f.GetInverseCorrectionData(0)
	if capErr != nil {
		t.Fatal(capErr)
	}
	if !correctionDataEqual(gotInv, expectedInv) {
		t.Fatalf("facade GetInverseCorrectionData does not match Reader")
	}
}

func TestGetCorrectionDataShapeAndNaNPresentThroughFacade(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	f, err := capabilities.OpenMachineConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()

	cd, capErr := f.GetCorrectionData(0)
	if capErr != nil {
		t.Fatal(capErr)
	}
	if cd.Shape != [3]int{257, 257, 2} {
		t.Fatalf("shape = %v", cd.Shape)
	}
	anyNaN := false
	for _, v := range cd.Data {
		if math.IsNaN(v) {
			anyNaN = true
			break
		}
	}
	if !anyNaN {
		t.Fatal("expected at least one NaN cell in a real correction grid")
	}
}

func TestGetCorrectionDataMissingClearboxIsNotPresent(t *testing.T) {
	// Every stock fixture's trains have a ClearBox, so build one without:
	// take the reference config, strip train 0's ClearBox, re-write.
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	cfg.OpticalTrains[0].OptionalComponents.Clearbox = nil
	tmp := filepath.Join(t.TempDir(), "no_clearbox.h5")
	if err := machineconfig.NewWriter().Write(cfg, tmp); err != nil {
		t.Fatal(err)
	}

	f, capErr := capabilities.OpenMachineConfig(tmp)
	if capErr != nil {
		t.Fatal(capErr)
	}
	defer f.Close()

	if _, e := f.GetCorrectionData(0); e == nil || e.Code != capabilities.ErrNotPresent {
		t.Fatalf("expected NotPresent, got %#v", e)
	}
	if _, e := f.GetInverseCorrectionData(0); e == nil || e.Code != capabilities.ErrNotPresent {
		t.Fatalf("expected NotPresent, got %#v", e)
	}
}

func TestGetCorrectionDataWorksOnCreateBasedInstanceWithoutTouchingDisk(t *testing.T) {
	// The specific case that rules out delegate-to-Reader-by-reopening: a
	// Create()-d facade has no path at all, so this must convert the
	// already-loaded in-memory model, not re-read from anywhere.
	//
	// Unlike the other four languages' mock builders (which default to
	// including a ClearBox), Go's Create() builds a train with no
	// OptionalComponents at all — and Go's facade has no SetClearbox to add
	// one afterward. So the no-disk-I/O behavior this test actually proves
	// is: NotPresent comes back immediately, not a crash or a disk read.
	f, err := capabilities.CreateMachineConfig("1.0")
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()

	if _, capErr := f.GetCorrectionData(0); capErr == nil || capErr.Code != capabilities.ErrNotPresent {
		t.Fatalf("expected NotPresent, got %#v", capErr)
	}
	if _, capErr := f.GetInverseCorrectionData(0); capErr == nil || capErr.Code != capabilities.ErrNotPresent {
		t.Fatalf("expected NotPresent, got %#v", capErr)
	}
}

// DISPATCH_REGISTRY_PLAN.md — proves ResolveOpen/ResolveCreate are genuinely
// registry-driven, not a relocated hardcoded check. fileVersion() returns a
// sentinel string; every other method is a trivial stub, since the tests
// below only ever check identity via that sentinel.
type fakeFile struct{}

func (fakeFile) FileVersion() string { return "9.9-test-sentinel" }
func (fakeFile) OpticalTrainCount() (int, *capabilities.Error) { return 0, nil }
func (fakeFile) GetMeta() (machineconfig.MachineConfigMeta, *capabilities.Error) {
	return machineconfig.MachineConfigMeta{}, nil
}
func (fakeFile) SetMeta(machineconfig.MachineConfigMeta, capabilities.SetMode) *capabilities.Error {
	return nil
}
func (fakeFile) GetScanner(int) (machineconfig.Scanner, *capabilities.Error) {
	return machineconfig.Scanner{}, nil
}
func (fakeFile) SetScanner(int, machineconfig.Scanner, capabilities.SetMode) *capabilities.Error {
	return nil
}
func (fakeFile) GetOpcua() (machineconfig.OpcuaConfig, *capabilities.Error) {
	return machineconfig.OpcuaConfig{}, nil
}
func (fakeFile) GetClearbox(int) (machineconfig.ClearBox, *capabilities.Error) {
	return machineconfig.ClearBox{}, nil
}
func (fakeFile) GetCorrectionData(int) (*machineconfig.CorrectionData, *capabilities.Error) {
	return nil, nil
}
func (fakeFile) GetInverseCorrectionData(int) (*machineconfig.CorrectionData, *capabilities.Error) {
	return nil, nil
}
func (fakeFile) Save(string) *capabilities.Error { return nil }
func (fakeFile) Close()                          {}

func TestOpenRegistryRejectsUnregisteredVersion(t *testing.T) {
	registry := capabilities.OpenRegistry{}
	_, err := capabilities.ResolveOpen("9.9-nope", "unused-path.h5", registry)
	if err == nil || err.Code != capabilities.ErrUnsupportedVersion {
		t.Fatalf("expected UnsupportedVersion, got %#v", err)
	}
}

func TestOpenRegistryDispatchesViaInjectedFile(t *testing.T) {
	registry := capabilities.OpenRegistry{
		"9.9-test": func(path string) (capabilities.File, *capabilities.Error) { return fakeFile{}, nil },
	}
	f, err := capabilities.ResolveOpen("9.9-test", "unused-path.h5", registry)
	if err != nil {
		t.Fatal(err)
	}
	if f.FileVersion() != "9.9-test-sentinel" {
		t.Fatalf("expected sentinel, got %q — dispatch did not route to the fake", f.FileVersion())
	}
}

func TestCreateRegistryRejectsUnregisteredVersion(t *testing.T) {
	registry := capabilities.CreateRegistry{}
	_, err := capabilities.ResolveCreate("9.9-nope", registry)
	if err == nil || err.Code != capabilities.ErrUnsupportedVersion {
		t.Fatalf("expected UnsupportedVersion, got %#v", err)
	}
}

func TestCreateRegistryDispatchesViaInjectedFile(t *testing.T) {
	registry := capabilities.CreateRegistry{
		"9.9-test": func(version string) (capabilities.File, *capabilities.Error) { return fakeFile{}, nil },
	}
	f, err := capabilities.ResolveCreate("9.9-test", registry)
	if err != nil {
		t.Fatal(err)
	}
	if f.FileVersion() != "9.9-test-sentinel" {
		t.Fatalf("expected sentinel, got %q — dispatch did not route to the fake", f.FileVersion())
	}
}

func TestSupportedFileVersionsReflectsRegistry(t *testing.T) {
	versions := capabilities.SupportedFileVersions()
	if len(versions) != 1 || versions[0] != "1.0" {
		t.Fatalf("expected exactly [\"1.0\"], got %v", versions)
	}
}
