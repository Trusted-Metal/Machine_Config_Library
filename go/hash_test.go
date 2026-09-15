package machineconfig_test

import (
	"path/filepath"
	"regexp"
	"testing"

	machineconfig "machine-config-go"
)

var hexDigest64 = regexp.MustCompile(`^[0-9a-f]{64}$`)

func referenceConfig(t *testing.T) *machineconfig.MachineConfig {
	t.Helper()
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	return cfg
}

func TestComputeConfigurationHashStableAndWellFormed(t *testing.T) {
	cfg := referenceConfig(t)
	a, err := machineconfig.ComputeConfigurationHash(cfg)
	if err != nil {
		t.Fatal(err)
	}
	b, err := machineconfig.ComputeConfigurationHash(cfg)
	if err != nil {
		t.Fatal(err)
	}
	if a != b {
		t.Errorf("hash not stable: %q != %q", a, b)
	}
	if !hexDigest64.MatchString(a) {
		t.Errorf("hash %q is not 64 lowercase hex chars", a)
	}
}

func TestComputeConfigurationHashIgnoresItsOwnStoredValue(t *testing.T) {
	cfg := referenceConfig(t)
	a, err := machineconfig.ComputeConfigurationHash(cfg)
	if err != nil {
		t.Fatal(err)
	}
	tampered := *cfg
	tampered.Meta.ConfigurationHash = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
	b, err := machineconfig.ComputeConfigurationHash(&tampered)
	if err != nil {
		t.Fatal(err)
	}
	if a != b {
		t.Errorf("hash should ignore its own stored value: %q != %q", a, b)
	}
}

func TestComputeConfigurationHashIgnoresExportDate(t *testing.T) {
	cfg := referenceConfig(t)
	a, err := machineconfig.ComputeConfigurationHash(cfg)
	if err != nil {
		t.Fatal(err)
	}
	changed := *cfg
	changed.Meta.ExportDate = "2099-01-01T00:00:00.000Z"
	b, err := machineconfig.ComputeConfigurationHash(&changed)
	if err != nil {
		t.Fatal(err)
	}
	if a != b {
		t.Errorf("hash should ignore export_date: %q != %q", a, b)
	}
}

func TestComputeConfigurationHashIgnoresIsValid(t *testing.T) {
	cfg := referenceConfig(t)
	a, err := machineconfig.ComputeConfigurationHash(cfg)
	if err != nil {
		t.Fatal(err)
	}
	valid := true
	changed := *cfg
	changed.Meta.IsValid = &valid
	b, err := machineconfig.ComputeConfigurationHash(&changed)
	if err != nil {
		t.Fatal(err)
	}
	if a != b {
		t.Errorf("hash should ignore is_valid: %q != %q", a, b)
	}
}

func TestComputeConfigurationHashChangesWhenAScalarFieldChanges(t *testing.T) {
	cfg := referenceConfig(t)
	a, err := machineconfig.ComputeConfigurationHash(cfg)
	if err != nil {
		t.Fatal(err)
	}
	changed := *cfg
	newX := 0.0
	if cfg.Machine.BuildPlateX != nil {
		newX = *cfg.Machine.BuildPlateX + 1
	}
	changed.Machine.BuildPlateX = &newX
	b, err := machineconfig.ComputeConfigurationHash(&changed)
	if err != nil {
		t.Fatal(err)
	}
	if a == b {
		t.Error("hash should change when a scalar field changes")
	}
}

func TestComputeConfigurationHashIgnoresBinaryGridData(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	withBinary, err := machineconfig.NewReader(path).ParseWithOptions(machineconfig.ParseOptions{IncludeBinary: true})
	if err != nil {
		t.Fatal(err)
	}
	withoutBinary, err := machineconfig.NewReader(path).ParseWithOptions(machineconfig.ParseOptions{IncludeBinary: false})
	if err != nil {
		t.Fatal(err)
	}
	a, err := machineconfig.ComputeConfigurationHash(withBinary)
	if err != nil {
		t.Fatal(err)
	}
	b, err := machineconfig.ComputeConfigurationHash(withoutBinary)
	if err != nil {
		t.Fatal(err)
	}
	if a != b {
		t.Errorf("hash should ignore binary grid data: %q != %q", a, b)
	}
}

// IntegerFieldNames is derived via reflection over the model itself (see
// integer_field_names.go), not hand-maintained — this is a light smoke test
// that the derivation actually ran and produced sensible membership, not a
// cross-check against a second source of truth (there isn't one to drift
// against, which is the whole point of deriving it this way).
func TestIntegerFieldNamesSmokeTest(t *testing.T) {
	names := machineconfig.IntegerFieldNames()

	for _, want := range []string{
		"bfs_max_depth", "data_port", "file_size", "port_id",
		"actual_bit_resolution", "trigger_stop_ceiling_layers",
	} {
		if _, ok := names[want]; !ok {
			t.Errorf("expected %q to be classified as integer-typed", want)
		}
	}
	for _, notWant := range []string{
		"build_plate_x", "working_distance", "wavelength", "value",
		"raw_bytes", "configuration_hash",
	} {
		if _, ok := names[notWant]; ok {
			t.Errorf("did not expect %q to be classified as integer-typed", notWant)
		}
	}
}
