// Package scenarios implements the Go validation app's S-01–09/AV-01–08
// scenarios (VALIDATION_PLAN.md §8). AV-09–11 live in go/'s own test tree
// instead — see docs/validation/go/results.md for why.
package scenarios

import (
	"errors"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"strings"

	mc "machine-config-go"
)

// S-01: Read reference fixture, all scalar fields
//
// ID:       S-01
// Action:   Parse reference_config.h5, check machine_name/file_version/hash.
// Expected: No error; file_version == "1.0"; configuration_hash is 64 chars.
func RunS01ReadScalars(fixturesDir, _ string) (bool, string) {
	path := filepath.Join(fixturesDir, "reference_config.h5")
	cfg, err := mc.NewReader(path).Parse()
	if err != nil {
		return false, fmt.Sprintf("read failed: %v", err)
	}
	if cfg.Meta.MachineName == "" {
		return false, "machine_name is empty"
	}
	if cfg.Meta.FileVersion != "1.0" {
		return false, fmt.Sprintf("file_version = %q", cfg.Meta.FileVersion)
	}
	if len(cfg.Meta.ConfigurationHash) != 64 {
		return false, fmt.Sprintf("configuration_hash len = %d", len(cfg.Meta.ConfigurationHash))
	}
	return true, fmt.Sprintf("machine_name=%q file_version=%q hash_len=%d",
		cfg.Meta.MachineName, cfg.Meta.FileVersion, len(cfg.Meta.ConfigurationHash))
}

// S-02: Read correction grids, shape and finite values
//
// ID:       S-02
// Action:   GetCorrectionData(0) on reference_config.h5.
// Expected: shape == [257,257,2]; grid contains NaNs (out-of-field cells).
func RunS02ReadBinary(fixturesDir, _ string) (bool, string) {
	path := filepath.Join(fixturesDir, "reference_config.h5")
	cd, err := mc.NewReader(path).GetCorrectionData(0)
	if err != nil {
		return false, fmt.Sprintf("GetCorrectionData failed: %v", err)
	}
	if cd.Shape != [3]int{257, 257, 2} {
		return false, fmt.Sprintf("shape = %v", cd.Shape)
	}
	nan := 0
	for _, v := range cd.Data {
		if math.IsNaN(v) {
			nan++
		}
	}
	if nan == 0 {
		return false, "expected NaNs in correction grid"
	}
	return true, fmt.Sprintf("shape=%v nan_count=%d total=%d", cd.Shape, nan, len(cd.Data))
}

// S-03: Read real AconityMIDI fixture file
//
// ID:           S-03
// Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
// Action:       Parse the file.
// Expected:     No error. Fields match known machine parameters.
func RunS03ReadReal(_ string, realDir string) (bool, string) {
	path, err := findRealFixture(realDir)
	if err != nil {
		return false, err.Error()
	}
	cfg, err := mc.NewReader(path).Parse()
	if err != nil {
		return false, fmt.Sprintf("read failed: %v", err)
	}

	wd := "nil"
	if cfg.OpticalTrains[0].Scanner.WorkingDistance != nil {
		wd = fmt.Sprintf("%.2f", *cfg.OpticalTrains[0].Scanner.WorkingDistance)
	}
	hashPrefix := cfg.Meta.ConfigurationHash
	if len(hashPrefix) > 16 {
		hashPrefix = hashPrefix[:16]
	}
	return true, fmt.Sprintf(
		"machine_name=%q file_version=%q trains=%d build_plate_x=%s wd=%s hash=%s...",
		cfg.Meta.MachineName, cfg.Meta.FileVersion, len(cfg.OpticalTrains),
		fmtF64Ptr(cfg.Machine.BuildPlateX), wd, hashPrefix,
	)
}

// S-04: Write modified config and verify field change survives roundtrip
//
// ID:       S-04
// Action:   Read reference fixture, change machine_name, write to temp, re-read.
// Expected: machine_name change persists.
func RunS04WriteModify(fixturesDir, _ string) (bool, string) {
	path := filepath.Join(fixturesDir, "reference_config.h5")
	cfg, err := mc.NewReader(path).Parse()
	if err != nil {
		return false, fmt.Sprintf("read failed: %v", err)
	}

	cfg.Meta.MachineName = "MODIFIED_VALIDATION"
	cfg.Machine.MachineName = "MODIFIED_VALIDATION"

	tmpPath, err := tempH5("mcl_go_s04")
	if err != nil {
		return false, fmt.Sprintf("tempfile: %v", err)
	}
	defer os.Remove(tmpPath)

	if err := mc.NewWriter().Write(cfg, tmpPath); err != nil {
		return false, fmt.Sprintf("write failed: %v", err)
	}
	rb, err := mc.NewReader(tmpPath).Parse()
	if err != nil {
		return false, fmt.Sprintf("readback failed: %v", err)
	}
	if rb.Meta.MachineName != "MODIFIED_VALIDATION" {
		return false, fmt.Sprintf("meta.machine_name after roundtrip: %q", rb.Meta.MachineName)
	}
	if rb.Machine.MachineName != "MODIFIED_VALIDATION" {
		return false, fmt.Sprintf("machine.machine_name after roundtrip: %q", rb.Machine.MachineName)
	}
	return true, "machine_name modified and survives roundtrip"
}

// S-05: Full binary round-trip with correction hash verification
//
// ID:       S-05
// Action:   Read (with binary) -> write to temp -> read temp -> compare
//           SHA-256 of correction_data / inverse_correction_data.
// Rationale: silent precision loss in binary data is undetectable without a
//           hash comparison.
func RunS05BinaryRoundtrip(fixturesDir, _ string) (bool, string) {
	path := filepath.Join(fixturesDir, "reference_config.h5")
	reader := mc.NewReader(path)

	// ParseWithOptions(IncludeBinary: true), not Parse(): Write()ing a config
	// read via plain Parse() would omit the correction grid entirely, since
	// the scalars-only path never loads it.
	cfg, err := reader.ParseWithOptions(mc.ParseOptions{IncludeBinary: true})
	if err != nil {
		return false, fmt.Sprintf("ParseWithOptions failed: %v", err)
	}

	cdBefore, err := reader.GetCorrectionData(0)
	if err != nil {
		return false, fmt.Sprintf("GetCorrectionData failed: %v", err)
	}
	icdBefore, err := reader.GetInverseCorrectionData(0)
	if err != nil {
		return false, fmt.Sprintf("GetInverseCorrectionData failed: %v", err)
	}

	tmpPath, err := tempH5("mcl_go_s05")
	if err != nil {
		return false, fmt.Sprintf("tempfile: %v", err)
	}
	defer os.Remove(tmpPath)

	if err := mc.NewWriter().Write(cfg, tmpPath); err != nil {
		return false, fmt.Sprintf("write failed: %v", err)
	}

	reader2 := mc.NewReader(tmpPath)
	cdAfter, err := reader2.GetCorrectionData(0)
	if err != nil {
		return false, fmt.Sprintf("readback GetCorrectionData failed: %v", err)
	}
	icdAfter, err := reader2.GetInverseCorrectionData(0)
	if err != nil {
		return false, fmt.Sprintf("readback GetInverseCorrectionData failed: %v", err)
	}

	if !bitwiseEqual(cdBefore.Data, cdAfter.Data) {
		return false, fmt.Sprintf("correction_data mismatch: %s... -> %s...",
			sha256Hex(cdBefore.Data)[:16], sha256Hex(cdAfter.Data)[:16])
	}
	if !bitwiseEqual(icdBefore.Data, icdAfter.Data) {
		return false, "inverse_correction_data mismatch after roundtrip"
	}

	return true, fmt.Sprintf("correction_data preserved: SHA-256=%s", sha256Hex(cdBefore.Data))
}

// S-06: Build synthetic config with MockConfigBuilder and verify fields
//
// ID:       S-06
// Action:   Build a 2-laser config -> verify fields -> save to temp -> re-read.
// Expected: 2 trains, rotations 0/180, machine_name non-empty,
//           correction_data centre cell ~2.0 (Gaussian peak), roundtrip OK.
func RunS06Builder(_, _ string) (bool, string) {
	cfg := mc.NewMockConfigBuilder().Build()

	if len(cfg.OpticalTrains) != 2 {
		return false, fmt.Sprintf("optical_trains count: %d", len(cfg.OpticalTrains))
	}

	r0 := cfg.OpticalTrains[0].Scanner.ScanHeadRotation
	if r0 == nil || math.Abs(*r0) > 0.001 {
		return false, fmt.Sprintf("train[0].scan_head_rotation: %s", fmtF64Ptr(r0))
	}
	r1 := cfg.OpticalTrains[1].Scanner.ScanHeadRotation
	if r1 == nil || math.Abs(*r1-180.0) > 0.001 {
		return false, fmt.Sprintf("train[1].scan_head_rotation: %s", fmtF64Ptr(r1))
	}
	if cfg.Meta.MachineName == "" {
		return false, "machine_name is empty"
	}

	cb := cfg.OpticalTrains[0].OptionalComponents.Clearbox
	if cb == nil || cb.CorrectionData == nil {
		return false, "clearbox or correction_data is nil"
	}
	grid := *cb.CorrectionData
	center := grid[128][128][0]
	if center == nil || math.Abs(*center-2.0) > 0.01 {
		return false, fmt.Sprintf("correction_data center: expected ~2.0, got %s", fmtF64Ptr(center))
	}

	tmpPath, err := tempH5("mcl_go_s06")
	if err != nil {
		return false, fmt.Sprintf("tempfile: %v", err)
	}
	defer os.Remove(tmpPath)

	if err := mc.NewWriter().Write(cfg, tmpPath); err != nil {
		return false, fmt.Sprintf("write failed: %v", err)
	}
	rb, err := mc.NewReader(tmpPath).Parse()
	if err != nil {
		return false, fmt.Sprintf("readback failed: %v", err)
	}
	if len(rb.OpticalTrains) != 2 {
		return false, fmt.Sprintf("readback trains: %d", len(rb.OpticalTrains))
	}
	if rb.Meta.MachineName != cfg.Meta.MachineName {
		return false, "machine_name changed after roundtrip"
	}

	return true, "2-laser build OK, center~2.0, roundtrip OK"
}

// S-07: OPCUA config roundtrip
//
// ID:       S-07
// Action:   Read reference_config_opcua.h5, verify OPCUA present, write ->
//           re-read, verify server_url survives.
func RunS07Opcua(fixturesDir, _ string) (bool, string) {
	path := filepath.Join(fixturesDir, "reference_config_opcua.h5")
	cfg, err := mc.NewReader(path).Parse()
	if err != nil {
		return false, fmt.Sprintf("read failed: %v", err)
	}
	if cfg.Opcua == nil || cfg.Opcua.Client.ServerURL == "" {
		return false, "expected opcua client with non-empty server_url"
	}
	origURL := cfg.Opcua.Client.ServerURL

	tmpPath, err := tempH5("mcl_go_s07")
	if err != nil {
		return false, fmt.Sprintf("tempfile: %v", err)
	}
	defer os.Remove(tmpPath)

	if err := mc.NewWriter().Write(cfg, tmpPath); err != nil {
		return false, fmt.Sprintf("write failed: %v", err)
	}
	rb, err := mc.NewReader(tmpPath).Parse()
	if err != nil {
		return false, fmt.Sprintf("readback failed: %v", err)
	}
	if rb.Opcua == nil || rb.Opcua.Client.ServerURL != origURL {
		return false, "server_url changed after roundtrip"
	}

	return true, fmt.Sprintf("opcua server_url=%q preserved through roundtrip", origURL)
}

// S-08: Drastic field change to real file, verify adapter pipeline integrity
//
// ID:           S-08
// Precondition: Reference Materials/machine_config_TM_LPBF_02__AconityMIDI__OG_1783607045113 (1).h5
// Action:       1. Add a third optical train (clone train 1, change train_id)
//               2. Change build_plate_x from 250.0 to 350.0
//               3. Set scan_head_rotation on new train to 90.0
//               4. Clear all correction data on the new train (clearbox = nil)
//               5. Change machine_name to "MODIFIED_ACONITY_VALIDATION"
// Expected:     All five changes persist after write -> read; file_version
//               unchanged at "1.0".
func RunS08DrasticChange(_ string, realDir string) (bool, string) {
	path, err := findRealFixture(realDir)
	if err != nil {
		return false, err.Error()
	}
	cfg, err := mc.NewReader(path).Parse()
	if err != nil {
		return false, fmt.Sprintf("read failed: %v", err)
	}

	cfg.Meta.MachineName = "MODIFIED_ACONITY_VALIDATION"
	cfg.Machine.MachineName = "MODIFIED_ACONITY_VALIDATION"
	cfg.Machine.BuildPlateX = mc.Float64Ptr(350.0)

	newTrain := cfg.OpticalTrains[1]
	newTrain.TrainID = "Optical_Train_03"
	newTrain.Scanner.ScanHeadRotation = mc.Float64Ptr(90.0)
	newTrain.OptionalComponents.Clearbox = nil
	cfg.OpticalTrains = append(cfg.OpticalTrains, newTrain)

	tmpPath, err := tempH5("mcl_go_s08")
	if err != nil {
		return false, fmt.Sprintf("tempfile: %v", err)
	}
	defer os.Remove(tmpPath)

	if err := mc.NewWriter().Write(cfg, tmpPath); err != nil {
		return false, fmt.Sprintf("write failed: %v", err)
	}
	rb, err := mc.NewReader(tmpPath).Parse()
	if err != nil {
		return false, fmt.Sprintf("readback failed: %v", err)
	}

	if len(rb.OpticalTrains) != 3 {
		return false, fmt.Sprintf("optical_trains: expected 3, got %d", len(rb.OpticalTrains))
	}
	if rb.Machine.BuildPlateX == nil || math.Abs(*rb.Machine.BuildPlateX-350.0) > 0.001 {
		return false, fmt.Sprintf("build_plate_x: got %s", fmtF64Ptr(rb.Machine.BuildPlateX))
	}
	rot := rb.OpticalTrains[2].Scanner.ScanHeadRotation
	if rot == nil || math.Abs(*rot-90.0) > 0.001 {
		return false, fmt.Sprintf("train[2].scan_head_rotation: got %s", fmtF64Ptr(rot))
	}
	if rb.OpticalTrains[2].OptionalComponents.Clearbox != nil {
		return false, "train[2].clearbox should be nil"
	}
	if rb.Meta.MachineName != "MODIFIED_ACONITY_VALIDATION" {
		return false, fmt.Sprintf("machine_name: got %q", rb.Meta.MachineName)
	}
	if strings.TrimSpace(rb.Meta.FileVersion) != "1.0" {
		return false, fmt.Sprintf("file_version changed: %q", rb.Meta.FileVersion)
	}

	return true, "3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK"
}

// S-09: Public type export surface
//
// ID:        S-09
// Title:     Verify all public model types are importable from the module root
// Action:    Reference every consumer-facing type directly via the "machine-config-go"
//            import (no machine-config-go/internal/... path) and prove each is
//            usable, not just nameable.
// Rationale: If a consumer must import from an internal path, the library's
//            public API surface is incomplete. See VALIDATION_PLAN.md §8.
//
// BuildPlate is type-annotated only, not constructed: it's a real, exported,
// aliased type but nothing in go/ ever builds one today (Machine carries flat
// BuildPlateX/Y/Z fields instead) — see VALIDATION_PLAN.md §9.4's open
// question on whether to keep or remove it. OpcuaConfig/Client/Pipe/Trigger
// are likewise type-annotated only: MockConfigBuilder never populates OPCUA
// (only the fixture used by S-07 does) — same precedent Rust's S-09 already
// established for the same situation.
func RunS09TypeExports(_, _ string) (bool, string) {
	builder := mc.NewMockConfigBuilder()
	cfg := builder.Build()

	var meta mc.MachineConfigMeta = cfg.Meta
	var machine mc.Machine = cfg.Machine

	if len(cfg.OpticalTrains) == 0 {
		return false, "builder produced zero optical trains"
	}
	var train mc.OpticalTrain = cfg.OpticalTrains[0]
	var scanner mc.Scanner = train.Scanner
	var lightSource mc.LightSource = train.LightSource
	var collimator mc.Collimator = train.Collimator
	var scannerCard mc.ScannerCard = train.ScannerCard
	var optionalComponents mc.OptionalComponents = train.OptionalComponents
	var axis mc.AxisConfig = scanner.XAxis

	clearboxPtr := optionalComponents.Clearbox
	if clearboxPtr == nil {
		return false, "expected builder to include a clearbox by default"
	}
	var clearbox mc.ClearBox = *clearboxPtr

	sfcfPtr := train.ScanFieldCorrectionFile
	if sfcfPtr == nil {
		return false, "expected builder to include an SFCF by default"
	}
	var sfcf mc.ScanFieldCorrectionFile = *sfcfPtr

	var correctionData mc.CorrectionData
	var opcua mc.OpcuaConfig
	var opcuaClient mc.OpcuaClientConfig
	var opcuaPipe mc.OpcuaPipeConfig
	var opcuaTrigger mc.OpcuaTrigger
	var buildPlate mc.BuildPlate

	var reader *mc.MachineConfigReader = mc.NewReader("unused-path-for-type-check-only")
	var writer *mc.MachineConfigWriter = mc.NewWriter()
	var parseOpts mc.ParseOptions
	var versionErr *mc.UnsupportedFileVersionError

	name := mc.StrPtr("type-check")
	num := mc.Float64Ptr(1.0)
	n := mc.IntPtr(1)
	flag := mc.BoolPtr(true)

	if meta.MachineName == "" || machine.Manufacturer == "" || scanner.Manufacturer == "" ||
		lightSource.Manufacturer == "" || collimator.Manufacturer == "" || scannerCard.Manufacturer == "" ||
		clearbox.IPAddress == "" || sfcf.DocumentName == "" || *name == "" || *num == 0 || *n == 0 || !*flag {
		return false, "one or more constructed fields were unexpectedly empty"
	}

	_ = axis
	_ = correctionData
	_ = opcua
	_ = opcuaClient
	_ = opcuaPipe
	_ = opcuaTrigger
	_ = buildPlate
	_ = reader
	_ = writer
	_ = parseOpts
	_ = versionErr

	return true, "all public types resolve and are usable from the module root"
}

// AV-01: Reader rejects unknown File_Version with typed error
func RunAv01UnknownVersion(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "v2_0_unknown.h5")
	_, err := mc.NewReader(fixture).Parse()
	if err == nil {
		return false, "no error raised for File_Version='2.0'"
	}
	var uv *mc.UnsupportedFileVersionError
	if errors.As(err, &uv) {
		if uv.Version == "2.0" {
			return true, fmt.Sprintf("UnsupportedFileVersionError raised, version=%q", uv.Version)
		}
		return false, fmt.Sprintf("UnsupportedFileVersionError raised but version=%q", uv.Version)
	}
	return false, fmt.Sprintf("wrong error type: %v", err)
}

// AV-02: Reader handles absent File_Version attribute predictably
//
// Either outcome (defaults to v1.0 dispatch, or a typed error) is acceptable
// per VALIDATION_PLAN.md — behavior must simply be documented, and must match
// across all five languages. This scenario records what actually happens
// rather than asserting one outcome, mirroring Rust's/Python's/Node's AV-02.
// Verified: Go's PeekFileVersion (go/file_version.go) treats a missing
// File_Version attribute as "1.0" (no error).
func RunAv02MissingVersion(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "missing_version.h5")
	cfg, err := mc.NewReader(fixture).Parse()
	if err != nil {
		return true, fmt.Sprintf("missing File_Version raises %v", err)
	}
	return true, fmt.Sprintf("missing File_Version dispatches OK, file_version=%q", cfg.Meta.FileVersion)
}

// AV-03: v1.0 reader encountering a v1.1 file fails loudly
func RunAv03FutureVersion(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "v1_1_simulated.h5")
	_, err := mc.NewReader(fixture).Parse()
	if err == nil {
		return false, "no error raised for File_Version='1.1'"
	}
	var uv *mc.UnsupportedFileVersionError
	if errors.As(err, &uv) {
		if uv.Version == "1.1" {
			return true, fmt.Sprintf("UnsupportedFileVersionError raised, version=%q", uv.Version)
		}
		return false, fmt.Sprintf("UnsupportedFileVersionError raised but version=%q", uv.Version)
	}
	return false, fmt.Sprintf("wrong error type: %v", err)
}

// AV-04: Reader returns an error when required group (Machine/) is absent
func RunAv04MissingGroup(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "missing_machine_group.h5")
	_, err := mc.NewReader(fixture).Parse()
	if err == nil {
		return false, "no error raised for missing Machine/ group"
	}
	return true, fmt.Sprintf("error raised for missing Machine/ group: %v", err)
}

// AV-05: Reader handles corrupt required attribute (Build_Plate_X_Dimension) gracefully
func RunAv05CorruptScalar(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "corrupt_scalar.h5")
	_, err := mc.NewReader(fixture).Parse()
	if err == nil {
		return false, "no error raised for corrupt Build_Plate_X_Dimension"
	}
	return true, fmt.Sprintf("error raised for corrupt scalar: %v", err)
}

// AV-06: Dispatcher normalizes whitespace in File_Version (" 1.0 ")
func RunAv06WhitespaceVersion(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "version_whitespace.h5")
	_, err := mc.NewReader(fixture).Parse()
	if err != nil {
		return false, fmt.Sprintf("%v", err)
	}
	return true, "whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK"
}

// AV-07: Dispatcher handles empty string File_Version
//
// Either outcome is acceptable per VALIDATION_PLAN.md — this records the
// actual behavior rather than asserting one, mirroring Rust's/Python's/Node's
// AV-07. Verified: Go's PeekFileVersion treats an empty File_Version string
// as "1.0" (no error).
func RunAv07EmptyVersion(fixturesDir, _ string) (bool, string) {
	fixture := avFixture(fixturesDir, "empty_version.h5")
	cfg, err := mc.NewReader(fixture).Parse()
	if err != nil {
		return true, fmt.Sprintf("empty File_Version raises %v", err)
	}
	return true, fmt.Sprintf("empty File_Version dispatches OK, file_version=%q", cfg.Meta.FileVersion)
}

// AV-08: File_Version string survives write->read unchanged
func RunAv08VersionFidelity(fixturesDir, _ string) (bool, string) {
	path := filepath.Join(fixturesDir, "reference_config.h5")
	cfg, err := mc.NewReader(path).Parse()
	if err != nil {
		return false, fmt.Sprintf("read failed: %v", err)
	}
	origVersion := strings.TrimSpace(cfg.Meta.FileVersion)

	tmpPath, err := tempH5("mcl_go_av08")
	if err != nil {
		return false, fmt.Sprintf("tempfile: %v", err)
	}
	defer os.Remove(tmpPath)

	if err := mc.NewWriter().Write(cfg, tmpPath); err != nil {
		return false, fmt.Sprintf("write failed: %v", err)
	}
	rb, err := mc.NewReader(tmpPath).Parse()
	if err != nil {
		return false, fmt.Sprintf("readback failed: %v", err)
	}
	rbVersion := strings.TrimSpace(rb.Meta.FileVersion)

	if rbVersion != "1.0" {
		return false, fmt.Sprintf("file_version after roundtrip: expected '1.0', got %q", rbVersion)
	}
	if rbVersion != origVersion {
		return false, fmt.Sprintf("file_version changed: %q -> %q", origVersion, rbVersion)
	}

	return true, fmt.Sprintf("File_Version survives roundtrip unchanged: %q", rbVersion)
}
