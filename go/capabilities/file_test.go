package capabilities_test

import (
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"testing"

	machineconfig "machine-config-go"
	"machine-config-go/capabilities"
	"machine-config-go/extension"
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

func TestInvalidIndex(t *testing.T) {
	dir := fixturesDir(t)
	f, err := capabilities.OpenMachineConfig(filepath.Join(dir, "reference_config.h5"))
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	if _, e := f.GetScanner(999); e == nil || e.Code != capabilities.ErrInvalidIndex {
		t.Fatalf("expected InvalidIndex, got %#v", e)
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
		// reference_config.h5 is externally-authored, so its own stored hash
		// is expected to be invalid -- but once *this* Save() produces a
		// file, that file's stored hash must be real (64 hex chars, not a
		// passthrough of the input) and must read back valid.
		if len(meta2.ConfigurationHash) != 64 {
			t.Errorf("configuration_hash len %d, want 64", len(meta2.ConfigurationHash))
		}
		if meta2.ConfigurationHash == origHash {
			t.Errorf("configuration_hash should differ from the externally-authored original, got same value")
		}
		if !f2.IsValid() {
			t.Errorf("IsValid() = false, want true for a file this Save() just wrote")
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

	t.Run("extension roundtrip", func(t *testing.T) {
		path := filepath.Join(fixturesDir(t), "reference_config_opcua.h5")
		before, err := machineconfig.NewReader(path).Parse()
		if err != nil {
			t.Fatal(err)
		}
		want, err := extension.Digest(before.Extensions["TM_OPCUA"])
		if err != nil {
			t.Fatal(err)
		}

		f, ferr := capabilities.OpenMachineConfig(path)
		if ferr != nil {
			t.Fatal(ferr)
		}
		defer f.Close()
		sc, _ := f.GetScanner(0)
		sc.WorkingDistance = machineconfig.Float64Ptr(77.5)
		if err := f.SetScanner(0, sc, capabilities.Merge); err != nil {
			t.Fatal(err)
		}

		out := filepath.Join(t.TempDir(), "ext_rt.h5")
		if err := f.Save(out); err != nil {
			t.Fatal(err)
		}
		after, err2 := machineconfig.NewReader(out).Parse()
		if err2 != nil {
			t.Fatal(err2)
		}
		got, err2 := extension.Digest(after.Extensions["TM_OPCUA"])
		if err2 != nil {
			t.Fatal(err2)
		}
		if got != want {
			t.Errorf("extension digest changed across an open/edit/save session: %s != %s", got, want)
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

func TestClearBoxExtensionSurvivesASessionOpenAndSave(t *testing.T) {
	for _, name := range []string{"reference_config.h5", "reference_config_v1_1.h5"} {
		t.Run(name, func(t *testing.T) {
			path := filepath.Join(fixturesDir(t), name)
			before, err := machineconfig.NewReader(path).Parse()
			if err != nil {
				t.Fatal(err)
			}
			want, err := extension.Digest(before.Extensions["ClearBox"])
			if err != nil {
				t.Fatal(err)
			}

			f, ferr := capabilities.OpenMachineConfig(path)
			if ferr != nil {
				t.Fatal(ferr)
			}
			defer f.Close()
			sc, _ := f.GetScanner(0)
			sc.WorkingDistance = machineconfig.Float64Ptr(77.5)
			if err := f.SetScanner(0, sc, capabilities.Merge); err != nil {
				t.Fatal(err)
			}
			out := filepath.Join(t.TempDir(), "clearbox_rt.h5")
			if err := f.Save(out); err != nil {
				t.Fatal(err)
			}
			after, err2 := machineconfig.NewReader(out).Parse()
			if err2 != nil {
				t.Fatal(err2)
			}
			got, err2 := extension.Digest(after.Extensions["ClearBox"])
			if err2 != nil {
				t.Fatal(err2)
			}
			if got != want {
				t.Errorf("ClearBox digest changed across an open/edit/save session: %s != %s", got, want)
			}
		})
	}
}

func TestCreateBasedInstanceHasNoClearBox(t *testing.T) {
	// Go's Create() builds a bare train with no ClearBox; saving it must not invent one.
	f, err := capabilities.CreateMachineConfig("1.0")
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	out := filepath.Join(t.TempDir(), "created_no_clearbox.h5")
	if err := f.Save(out); err != nil {
		t.Fatal(err)
	}
	cfg, rerr := machineconfig.NewReader(out).Parse()
	if rerr != nil {
		t.Fatal(rerr)
	}
	if _, ok := cfg.Extensions["ClearBox"]; ok {
		t.Fatal("unexpected ClearBox extension on a Create()-based file")
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
func (fakeFile) IsValid() bool                   { return false }
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
	sort.Strings(versions)
	want := []string{"1.0", "1.1"}
	if !reflect.DeepEqual(versions, want) {
		t.Fatalf("expected %v, got %v", want, versions)
	}
}

// ---------------------------------------------------------------------------
// IsValid() -- CAPABILITIES_HASH_INTEGRATION_PLAN.md
// ---------------------------------------------------------------------------

func TestIsValidFalseForExternallyAuthoredReferenceFixture(t *testing.T) {
	// Permanent regression guard, mirroring CONFIGURATION_HASH_PLAN.md's own
	// equivalent at the MachineConfigReader layer: reference_config.h5 was
	// authored externally, so MCF's recomputed hash will never match its
	// stored value. Expected and correct, not a defect.
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	f, err := capabilities.OpenMachineConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	if f.IsValid() {
		t.Error("IsValid() = true, want false for an externally-authored fixture")
	}
}

func TestIsValidTrueForSelfWrittenFile(t *testing.T) {
	f, err := capabilities.CreateMachineConfig("1.0")
	if err != nil {
		t.Fatal(err)
	}
	out := filepath.Join(t.TempDir(), "created.h5")
	if err := f.Save(out); err != nil {
		t.Fatal(err)
	}
	if !f.IsValid() {
		t.Error("IsValid() = false, want true immediately after Save()")
	}
	f.Close()

	reopened, err := capabilities.OpenMachineConfig(out)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.Close()
	if !reopened.IsValid() {
		t.Error("IsValid() = false, want true for a freshly reopened self-written file")
	}
}

func TestIsValidFlipsLiveAcrossAnUnsavedEditThenASave(t *testing.T) {
	// The test this whole live-method design exists to satisfy: a cached,
	// computed-once-at-Open() value could never distinguish these states.
	f, err := capabilities.CreateMachineConfig("1.0")
	if err != nil {
		t.Fatal(err)
	}
	out := filepath.Join(t.TempDir(), "created.h5")
	if err := f.Save(out); err != nil {
		t.Fatal(err)
	}
	if !f.IsValid() {
		t.Fatal("freshly saved content must be valid")
	}

	meta, err := f.GetMeta()
	if err != nil {
		t.Fatal(err)
	}
	storedBefore := meta.ConfigurationHash

	sc, err := f.GetScanner(0)
	if err != nil {
		t.Fatal(err)
	}
	sc.WorkingDistance = machineconfig.Float64Ptr(999.5)
	if err := f.SetScanner(0, sc, capabilities.Merge); err != nil {
		t.Fatal(err)
	}

	// Live-ness: an unsaved edit must be reflected immediately, with no
	// Save()/reopen in between. This is expected, not a corruption signal
	// (see CAPABILITIES_HASH_INTEGRATION_PLAN.md's documented caveat).
	if f.IsValid() {
		t.Error("IsValid() = true, want false for an unsaved edit, live, without saving")
	}

	// The save->live round trip: IsValid() must flip back to true from the
	// *same session*, without reopening.
	if err := f.Save(out); err != nil {
		t.Fatal(err)
	}
	if !f.IsValid() {
		t.Error("Save() must update session state so IsValid() is true without reopening")
	}
	meta2, err := f.GetMeta()
	if err != nil {
		t.Fatal(err)
	}
	storedAfter := meta2.ConfigurationHash
	if storedAfter == storedBefore {
		t.Error("the edit must have produced a different hash")
	}
	f.Close()

	// And a completely fresh session on the same path agrees.
	reopened, err := capabilities.OpenMachineConfig(out)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.Close()
	if !reopened.IsValid() {
		t.Error("IsValid() = false, want true for a freshly reopened session")
	}
	reopenedMeta, err := reopened.GetMeta()
	if err != nil {
		t.Fatal(err)
	}
	if reopenedMeta.ConfigurationHash != storedAfter {
		t.Errorf("configuration_hash: got %q want %q", reopenedMeta.ConfigurationHash, storedAfter)
	}
}

func TestV1_1SaveComputesRealHashAndUpdatesSession(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config_v1_1.h5")
	f, err := capabilities.OpenMachineConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	if f.FileVersion() != "1.1" {
		t.Fatalf("expected file_version 1.1, got %q", f.FileVersion())
	}
	meta, err := f.GetMeta()
	if err != nil {
		t.Fatal(err)
	}
	originalStored := meta.ConfigurationHash

	sc, err := f.GetScanner(0)
	if err != nil {
		t.Fatal(err)
	}
	sc.WorkingDistance = machineconfig.Float64Ptr(555.0)
	if err := f.SetScanner(0, sc, capabilities.Merge); err != nil {
		t.Fatal(err)
	}
	if f.IsValid() {
		t.Error("IsValid() = true, want false for an unsaved edit, live")
	}

	out := filepath.Join(t.TempDir(), "v1_1_roundtrip.h5")
	if err := f.Save(out); err != nil {
		t.Fatal(err)
	}

	// Live, no reopen:
	if !f.IsValid() {
		t.Error("IsValid() = false, want true immediately after Save()")
	}
	meta2, err := f.GetMeta()
	if err != nil {
		t.Fatal(err)
	}
	savedHash := meta2.ConfigurationHash
	if len(savedHash) != 64 {
		t.Errorf("configuration_hash len %d, want 64", len(savedHash))
	}
	if savedHash == originalStored {
		t.Error("configuration_hash should differ from the original")
	}
	f.Close()

	// Fresh session on the written file agrees:
	reopened, err := capabilities.OpenMachineConfig(out)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.Close()
	if reopened.FileVersion() != "1.1" {
		t.Fatalf("expected file_version 1.1, got %q", reopened.FileVersion())
	}
	if !reopened.IsValid() {
		t.Error("IsValid() = false, want true for a freshly reopened session")
	}
	reopenedMeta, err := reopened.GetMeta()
	if err != nil {
		t.Fatal(err)
	}
	if reopenedMeta.ConfigurationHash != savedHash {
		t.Errorf("configuration_hash: got %q want %q", reopenedMeta.ConfigurationHash, savedHash)
	}
}
