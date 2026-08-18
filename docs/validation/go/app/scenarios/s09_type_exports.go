package scenarios

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

import (
	mc "machine-config-go"
)

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
