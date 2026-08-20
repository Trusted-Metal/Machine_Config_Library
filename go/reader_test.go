package machineconfig_test

import (
	"encoding/json"
	"errors"
	"math"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	machineconfig "machine-config-go"
	"machine-config-go/capabilities"
	"machine-config-go/internal/h5c"
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

func writeFutureFileVersion(t *testing.T, path string) {
	t.Helper()
	f, err := h5c.Create(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	root, err := f.Root()
	if err != nil {
		t.Fatal(err)
	}
	defer root.Close()
	if err := root.WriteStringAttr("File_Version", "2.0"); err != nil {
		t.Fatal(err)
	}
}

func TestUnknownFileVersionDoesNotUseV1Layout(t *testing.T) {
	path := filepath.Join(t.TempDir(), "future.h5")
	writeFutureFileVersion(t, path)

	_, err := machineconfig.NewReader(path).Parse()
	var u *machineconfig.UnsupportedFileVersionError
	if !errors.As(err, &u) {
		t.Fatalf("Parse: got %#v, want UnsupportedFileVersionError", err)
	}
	if u.Version != "2.0" {
		t.Fatalf("version = %q", u.Version)
	}

	_, capErr := capabilities.OpenMachineConfig(path)
	if capErr == nil {
		t.Fatal("OpenMachineConfig: expected error")
	}
	if capErr.Code != capabilities.ErrUnsupportedVersion {
		t.Fatalf("code = %q", capErr.Code)
	}
	if !strings.Contains(capErr.Message, "2.0") {
		t.Fatalf("message = %q", capErr.Message)
	}
}

// ---------------------------------------------------------------------------
// meta.extra
// ---------------------------------------------------------------------------

func TestMetaExtra(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Meta.Extra == nil {
		t.Fatal("meta.extra is nil")
	}
	desc, ok := cfg.Meta.Extra["Description"].(string)
	if !ok || desc == "" {
		t.Fatalf("meta.extra.Description = %v", cfg.Meta.Extra["Description"])
	}
	gen, ok := cfg.Meta.Extra["Generator"].(string)
	if !ok || gen == "" {
		t.Fatalf("meta.extra.Generator = %v", cfg.Meta.Extra["Generator"])
	}
}

// ---------------------------------------------------------------------------
// Machine fields
// ---------------------------------------------------------------------------

func TestMachineFields(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	m := cfg.Machine
	if m.BuildPlateY == nil || math.Abs(*m.BuildPlateY-250.0) > 1e-9 {
		t.Fatalf("build_plate_y = %v", m.BuildPlateY)
	}
	if m.BuildPlateZ == nil || math.Abs(*m.BuildPlateZ-20.0) > 1e-9 {
		t.Fatalf("build_plate_z = %v", m.BuildPlateZ)
	}
	if m.SerialNumber != "500300_1" {
		t.Fatalf("machine serial_number = %q", m.SerialNumber)
	}
	if m.GasFlowDirection == nil || *m.GasFlowDirection != "Y+" {
		t.Fatalf("gas_flow_direction = %v", m.GasFlowDirection)
	}
}

// ---------------------------------------------------------------------------
// Thermal lensing — both trains
// ---------------------------------------------------------------------------

func TestThermalLensing(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}

	t0 := cfg.OpticalTrains[0]
	if t0.ThermalLensingPassed == nil || *t0.ThermalLensingPassed != false {
		t.Fatalf("train 0 thermal_lensing_passed = %v", t0.ThermalLensingPassed)
	}
	if t0.ThermalLensingFocalPlaneShift == nil || math.Abs(*t0.ThermalLensingFocalPlaneShift-1.178387083333333) > 1e-9 {
		t.Fatalf("train 0 thermal_lensing_focal_plane_shift = %v", t0.ThermalLensingFocalPlaneShift)
	}
	if t0.ThermalLensingThreshold == nil || math.Abs(*t0.ThermalLensingThreshold-0.7848370833333334) > 1e-9 {
		t.Fatalf("train 0 thermal_lensing_threshold = %v", t0.ThermalLensingThreshold)
	}

	t1 := cfg.OpticalTrains[1]
	if t1.ThermalLensingPassed == nil || *t1.ThermalLensingPassed != true {
		t.Fatalf("train 1 thermal_lensing_passed = %v", t1.ThermalLensingPassed)
	}
	if t1.ThermalLensingFocalPlaneShift == nil || math.Abs(*t1.ThermalLensingFocalPlaneShift-0.1906766666666668) > 1e-9 {
		t.Fatalf("train 1 thermal_lensing_focal_plane_shift = %v", t1.ThermalLensingFocalPlaneShift)
	}
}

// ---------------------------------------------------------------------------
// Scan-field correction file
// ---------------------------------------------------------------------------

func TestScanFieldCorrectionFile(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	sfcf0 := cfg.OpticalTrains[0].ScanFieldCorrectionFile
	if sfcf0 == nil {
		t.Fatal("train 0 scan_field_correction_file is nil")
	}
	if sfcf0.FileSize != 1138799 {
		t.Fatalf("train 0 sfcf file_size = %d", sfcf0.FileSize)
	}
	if sfcf0.DocumentName != "Story6.17.3.1_Laser_1_VMM.fc3" {
		t.Fatalf("train 0 sfcf document_name = %q", sfcf0.DocumentName)
	}
	if sfcf0.DocumentID == "" {
		t.Fatal("train 0 sfcf document_id is empty")
	}

	sfcf1 := cfg.OpticalTrains[1].ScanFieldCorrectionFile
	if sfcf1 == nil {
		t.Fatal("train 1 scan_field_correction_file is nil")
	}
	if sfcf1.FileSize != 1142763 {
		t.Fatalf("train 1 sfcf file_size = %d", sfcf1.FileSize)
	}
}

// ---------------------------------------------------------------------------
// Scanner: scan_head_rotation and scan_head_offset_y
// ---------------------------------------------------------------------------

func TestScannerRotationAndOffsetY(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	rot0 := cfg.OpticalTrains[0].Scanner.ScanHeadRotation
	if rot0 == nil || math.Abs(*rot0-0.0) > 1e-9 {
		t.Fatalf("train 0 scan_head_rotation = %v", rot0)
	}
	rot1 := cfg.OpticalTrains[1].Scanner.ScanHeadRotation
	if rot1 == nil || math.Abs(*rot1-180.0) > 1e-9 {
		t.Fatalf("train 1 scan_head_rotation = %v", rot1)
	}
	oy0 := cfg.OpticalTrains[0].Scanner.ScanHeadOffsetY
	if oy0 == nil || math.Abs(*oy0-23.5) > 1e-9 {
		t.Fatalf("train 0 scan_head_offset_y = %v", oy0)
	}
	oy1 := cfg.OpticalTrains[1].Scanner.ScanHeadOffsetY
	if oy1 == nil || math.Abs(*oy1-(-21.695)) > 1e-4 {
		t.Fatalf("train 1 scan_head_offset_y = %v", oy1)
	}
}

// ---------------------------------------------------------------------------
// Light source and scanner card
// ---------------------------------------------------------------------------

func TestLightSourceAndScannerCard(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	ls := cfg.OpticalTrains[0].LightSource
	if ls.Wavelength == nil || math.Abs(*ls.Wavelength-1070.0) > 1e-9 {
		t.Fatalf("light_source.wavelength = %v", ls.Wavelength)
	}
	if ls.WavelengthUnit == nil || *ls.WavelengthUnit != "nm" {
		t.Fatalf("light_source.wavelength_unit = %v", ls.WavelengthUnit)
	}
	if ls.PowerMaxNominal == nil || math.Abs(*ls.PowerMaxNominal-1000.0) > 1e-9 {
		t.Fatalf("light_source.power_max_nominal = %v", ls.PowerMaxNominal)
	}

	sc := cfg.OpticalTrains[0].ScannerCard
	if sc.SamplePeriod == nil || math.Abs(*sc.SamplePeriod-10.0) > 1e-9 {
		t.Fatalf("scanner_card.sample_period = %v", sc.SamplePeriod)
	}
}

// ---------------------------------------------------------------------------
// Train IDs
// ---------------------------------------------------------------------------

func TestTrainIDs(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.OpticalTrains[0].TrainID != "Optical_Train_01" {
		t.Fatalf("train 0 id = %q", cfg.OpticalTrains[0].TrainID)
	}
	if cfg.OpticalTrains[1].TrainID != "Optical_Train_02" {
		t.Fatalf("train 1 id = %q", cfg.OpticalTrains[1].TrainID)
	}
}

// ---------------------------------------------------------------------------
// Inverse correction data
// ---------------------------------------------------------------------------

func TestGetInverseCorrectionData(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	inv, err := machineconfig.NewReader(path).GetInverseCorrectionData(0)
	if err != nil {
		t.Fatal(err)
	}
	if inv.Shape != [3]int{257, 257, 2} {
		t.Fatalf("inverse shape = %v", inv.Shape)
	}
	nan := 0
	for _, v := range inv.Data {
		if math.IsNaN(v) {
			nan++
		}
	}
	if nan == 0 {
		t.Fatal("expected NaNs in inverse correction grid")
	}
	// forward and inverse grids must differ
	fwd, err := machineconfig.NewReader(path).GetCorrectionData(0)
	if err != nil {
		t.Fatal(err)
	}
	same := true
	for i := range fwd.Data {
		if !math.IsNaN(fwd.Data[i]) && !math.IsNaN(inv.Data[i]) && fwd.Data[i] != inv.Data[i] {
			same = false
			break
		}
	}
	if same {
		t.Fatal("forward and inverse correction grids are identical — expected them to differ")
	}
}

// ---------------------------------------------------------------------------
// OPCUA detailed fields (client, pipe, triggers)
// ---------------------------------------------------------------------------

func TestOpcuaDetailedFields(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config_opcua.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	o := cfg.Opcua
	if o == nil {
		t.Fatal("expected opcua config")
	}

	// client
	if o.Client.BfsMaxDepth != 16 {
		t.Fatalf("client.bfs_max_depth = %d", o.Client.BfsMaxDepth)
	}
	if o.Client.PublishInterval != 250 {
		t.Fatalf("client.publish_interval = %d", o.Client.PublishInterval)
	}
	if o.Client.SamplingInterval != 250 {
		t.Fatalf("client.sampling_interval = %d", o.Client.SamplingInterval)
	}
	if o.Client.SessionTimeout != 60000 {
		t.Fatalf("client.session_timeout = %d", o.Client.SessionTimeout)
	}
	if o.Client.AuthMode != "UsernamePassword" {
		t.Fatalf("client.auth_mode = %q", o.Client.AuthMode)
	}

	// pipe
	if !o.Pipe.PipeEnabled {
		t.Fatal("pipe.pipe_enabled expected true")
	}
	if o.Pipe.BufferSize != 65536 {
		t.Fatalf("pipe.buffer_size = %d", o.Pipe.BufferSize)
	}

	// triggers
	if o.TriggersEnabled == nil || !*o.TriggersEnabled {
		t.Fatalf("triggers_enabled = %v", o.TriggersEnabled)
	}
	lei, ok := o.Triggers["Laser Emission Interlock"]
	if !ok {
		t.Fatal(`triggers missing "Laser Emission Interlock"`)
	}
	if lei.ID == nil || *lei.ID != "trigger_1" {
		t.Fatalf("LEI id = %v", lei.ID)
	}
	if lei.Signal == nil || *lei.Signal != "yellow_light" {
		t.Fatalf("LEI signal = %v", lei.Signal)
	}
	if lei.Subsystem == nil || *lei.Subsystem != "Chamber" {
		t.Fatalf("LEI subsystem = %v", lei.Subsystem)
	}

	coLevel, ok := o.Triggers["Chamber Oxygen Level"]
	if !ok {
		t.Fatal(`triggers missing "Chamber Oxygen Level"`)
	}

	// Promoted fields (OPCUA_FIELD_PROMOTION_PLAN.md Phase 1) — every one of
	// the 22 promoted fields checked against real values (verified via h5py
	// before writing this test), plus TriggerStopCeilingLayers, plus
	// confirmation none of them land in Extra.
	if o.Client.KeepAliveCount == nil || *o.Client.KeepAliveCount != 240 {
		t.Fatalf("client.keep_alive_count = %v", o.Client.KeepAliveCount)
	}
	if o.Client.LifetimeCount == nil || *o.Client.LifetimeCount != 2400 {
		t.Fatalf("client.lifetime_count = %v", o.Client.LifetimeCount)
	}
	if o.Client.MachineProfile == nil || *o.Client.MachineProfile != "Aconity" {
		t.Fatalf("client.machine_profile = %v", o.Client.MachineProfile)
	}
	if o.Client.QueuePolicy == nil || *o.Client.QueuePolicy != "DropOldest" {
		t.Fatalf("client.queue_policy = %v", o.Client.QueuePolicy)
	}
	if o.Client.QueueSizeDataChange == nil || *o.Client.QueueSizeDataChange != 100 {
		t.Fatalf("client.queue_size_data_change = %v", o.Client.QueueSizeDataChange)
	}
	if o.Client.QueueSizeEvents == nil || *o.Client.QueueSizeEvents != 7200 {
		t.Fatalf("client.queue_size_events = %v", o.Client.QueueSizeEvents)
	}
	if o.Client.ReconnectInterval == nil || *o.Client.ReconnectInterval != 10000 {
		t.Fatalf("client.reconnect_interval = %v", o.Client.ReconnectInterval)
	}
	if o.Client.RootNode == nil || *o.Client.RootNode != "MachineFleet" {
		t.Fatalf("client.root_node = %v", o.Client.RootNode)
	}
	if o.Client.SyncLoopIntervalInitial == nil || *o.Client.SyncLoopIntervalInitial != 1000 {
		t.Fatalf("client.sync_loop_interval_initial = %v", o.Client.SyncLoopIntervalInitial)
	}
	if o.Client.SyncLoopIntervalSettled == nil || *o.Client.SyncLoopIntervalSettled != 30000 {
		t.Fatalf("client.sync_loop_interval_settled = %v", o.Client.SyncLoopIntervalSettled)
	}
	if len(o.Client.Extra) != 0 {
		t.Fatalf("client.extra should be empty, got %#v", o.Client.Extra)
	}

	if o.Pipe.ConfigureClient == nil || !*o.Pipe.ConfigureClient {
		t.Fatalf("pipe.configure_client = %v", o.Pipe.ConfigureClient)
	}
	if o.Pipe.InboundRateLimit == nil || *o.Pipe.InboundRateLimit != -1 {
		t.Fatalf("pipe.inbound_rate_limit = %v", o.Pipe.InboundRateLimit)
	}
	if o.Pipe.MaxInboundMessageSize == nil || *o.Pipe.MaxInboundMessageSize != 65536 {
		t.Fatalf("pipe.max_inbound_message_size = %v", o.Pipe.MaxInboundMessageSize)
	}
	if o.Pipe.MinIntegrityLevel == nil || *o.Pipe.MinIntegrityLevel != "0x2000" {
		t.Fatalf("pipe.min_integrity_level = %v", o.Pipe.MinIntegrityLevel)
	}
	if o.Pipe.PipeName == nil || *o.Pipe.PipeName != `\\.\pipe\opc_ua_client_pipe` {
		t.Fatalf("pipe.pipe_name = %v", o.Pipe.PipeName)
	}
	if o.Pipe.UserAccessLevel == nil || *o.Pipe.UserAccessLevel != "AnyLocalUser" {
		t.Fatalf("pipe.user_access_level = %v", o.Pipe.UserAccessLevel)
	}
	if len(o.Pipe.Extra) != 0 {
		t.Fatalf("pipe.extra should be empty, got %#v", o.Pipe.Extra)
	}

	if o.TriggerStopCeilingLayers == nil || *o.TriggerStopCeilingLayers != 3 {
		t.Fatalf("trigger_stop_ceiling_layers = %v", o.TriggerStopCeilingLayers)
	}

	if lei.CaseSensitivity == nil || *lei.CaseSensitivity != "Exact" {
		t.Fatalf("LEI case_sensitivity = %v", lei.CaseSensitivity)
	}
	if lei.Component == nil || *lei.Component != "machine_state_indicator" {
		t.Fatalf("LEI component = %v", lei.Component)
	}
	if lei.CooldownPeriod == nil || *lei.CooldownPeriod != 0 {
		t.Fatalf("LEI cooldown_period = %v", lei.CooldownPeriod)
	}
	if lei.Event == nil || *lei.Event != "SensorEvents" {
		t.Fatalf("LEI event = %v", lei.Event)
	}
	if lei.MaxFiresPerJob == nil || *lei.MaxFiresPerJob != 0 {
		t.Fatalf("LEI max_fires_per_job = %v", lei.MaxFiresPerJob)
	}
	if lei.TriggerLabel == nil || *lei.TriggerLabel != "Laser Emission Interlock" {
		t.Fatalf("LEI trigger_label = %v", lei.TriggerLabel)
	}
	if len(lei.Extra) != 0 {
		t.Fatalf("LEI extra should be empty, got %#v", lei.Extra)
	}

	if coLevel.Component == nil || *coLevel.Component != "process_chamber::gas_management::oxygen_sensor::1" {
		t.Fatalf("Chamber Oxygen Level component = %v", coLevel.Component)
	}
	if coLevel.Event == nil || *coLevel.Event != "SensorEvents" {
		t.Fatalf("Chamber Oxygen Level event = %v", coLevel.Event)
	}
	if coLevel.TriggerLabel == nil || *coLevel.TriggerLabel != "Chamber Oxygen Level" {
		t.Fatalf("Chamber Oxygen Level trigger_label = %v", coLevel.TriggerLabel)
	}
	if len(coLevel.Extra) != 0 {
		t.Fatalf("Chamber Oxygen Level extra should be empty, got %#v", coLevel.Extra)
	}
}

// ---------------------------------------------------------------------------
// opcua_missing_required.h5 (Phase 0) removes all seven Phase-2-required
// attributes. The reader must stay permissive (facade-only enforcement — see
// OPCUA_FIELD_PROMOTION_PLAN.md): parsing succeeds, the removed fields read
// back nil, and the per-trigger Event asymmetry is exactly as the fixture
// intends.
// ---------------------------------------------------------------------------

func validationFixturesDir(t *testing.T) string {
	t.Helper()
	return filepath.Clean(filepath.Join(fixturesDir(t), "..", "docs", "validation", "fixtures"))
}

func TestOpcuaMissingRequiredFixtureParsesGracefully(t *testing.T) {
	path := filepath.Join(validationFixturesDir(t), "opcua_missing_required.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	o := cfg.Opcua
	if o == nil {
		t.Fatal("expected opcua config")
	}

	if o.Client.MachineProfile != nil {
		t.Fatalf("client.machine_profile should be nil, got %v", *o.Client.MachineProfile)
	}
	if o.Client.RootNode != nil {
		t.Fatalf("client.root_node should be nil, got %v", *o.Client.RootNode)
	}
	if o.Pipe.ConfigureClient != nil {
		t.Fatalf("pipe.configure_client should be nil, got %v", *o.Pipe.ConfigureClient)
	}
	if o.Pipe.PipeName != nil {
		t.Fatalf("pipe.pipe_name should be nil, got %v", *o.Pipe.PipeName)
	}
	if o.TriggersEnabled != nil {
		t.Fatalf("triggers_enabled should be nil, got %v", *o.TriggersEnabled)
	}
	if o.TriggerStopCeilingLayers != nil {
		t.Fatalf("trigger_stop_ceiling_layers should be nil, got %v", *o.TriggerStopCeilingLayers)
	}

	lei, ok := o.Triggers["Laser Emission Interlock"]
	if !ok {
		t.Fatal(`triggers missing "Laser Emission Interlock"`)
	}
	if lei.Event != nil {
		t.Fatalf("LEI event should be nil (deliberately removed), got %v", *lei.Event)
	}
	coLevel, ok := o.Triggers["Chamber Oxygen Level"]
	if !ok {
		t.Fatal(`triggers missing "Chamber Oxygen Level"`)
	}
	if coLevel.Event == nil || *coLevel.Event != "SensorEvents" {
		t.Fatalf("Chamber Oxygen Level event should be unaffected, got %v", coLevel.Event)
	}

	// Untouched fields elsewhere confirm the rest of the file is unaffected.
	if o.Client.ServerURL == "" {
		t.Fatal("client.server_url should be non-empty")
	}
	if o.Client.KeepAliveCount == nil || *o.Client.KeepAliveCount != 240 {
		t.Fatalf("client.keep_alive_count = %v", o.Client.KeepAliveCount)
	}
	if o.Pipe.BufferSize != 65536 {
		t.Fatalf("pipe.buffer_size = %d", o.Pipe.BufferSize)
	}
	if lei.TriggerLabel == nil || *lei.TriggerLabel != "Laser Emission Interlock" {
		t.Fatalf("LEI trigger_label = %v", lei.TriggerLabel)
	}
}

// ---------------------------------------------------------------------------
// JSON top-level structure
// ---------------------------------------------------------------------------

func TestJSONTopLevelKeys(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	b, err := json.Marshal(cfg)
	if err != nil {
		t.Fatal(err)
	}
	var v map[string]json.RawMessage
	if err := json.Unmarshal(b, &v); err != nil {
		t.Fatal(err)
	}
	for _, key := range []string{"meta", "machine", "optical_trains"} {
		if _, ok := v[key]; !ok {
			t.Errorf("JSON missing top-level key %q", key)
		}
	}
	if _, ok := v["opcua"]; ok {
		t.Error("reference fixture JSON must not contain opcua key")
	}

	// opcua fixture must include the key
	path2 := filepath.Join(fixturesDir(t), "reference_config_opcua.h5")
	cfg2, err := machineconfig.NewReader(path2).Parse()
	if err != nil {
		t.Fatal(err)
	}
	b2, _ := json.Marshal(cfg2)
	var v2 map[string]json.RawMessage
	json.Unmarshal(b2, &v2) //nolint:errcheck
	if _, ok := v2["opcua"]; !ok {
		t.Error("opcua fixture JSON must contain opcua key")
	}
}

func TestJSONSchemaValidation(t *testing.T) {
	path := filepath.Join(fixturesDir(t), "reference_config.h5")
	cfg, err := machineconfig.NewReader(path).Parse()
	if err != nil {
		t.Fatal(err)
	}
	b, err := json.Marshal(cfg)
	if err != nil {
		t.Fatal(err)
	}
	var v map[string]any
	if err := json.Unmarshal(b, &v); err != nil {
		t.Fatal(err)
	}

	for _, key := range []string{"meta", "machine", "optical_trains"} {
		if _, ok := v[key]; !ok {
			t.Errorf("missing top-level key %q", key)
		}
	}
	meta, _ := v["meta"].(map[string]any)
	if name, _ := meta["machine_name"].(string); name == "" {
		t.Error("meta.machine_name must be non-empty")
	}
	if hash, _ := meta["configuration_hash"].(string); len(hash) != 64 {
		t.Errorf("meta.configuration_hash len = %d, want 64", len(hash))
	}
	trains, _ := v["optical_trains"].([]any)
	if len(trains) < 1 {
		t.Fatalf("optical_trains is empty")
	}
	for i, tr := range trains {
		m, _ := tr.(map[string]any)
		if _, ok := m["train_id"]; !ok {
			t.Errorf("train %d missing train_id", i)
		}
		if _, ok := m["scanner"]; !ok {
			t.Errorf("train %d missing scanner", i)
		}
	}

	// Schema file must exist and be valid JSON.
	_, srcFile, _, _ := runtime.Caller(0)
	schemaPath := filepath.Join(filepath.Dir(srcFile), "..", "schema", "machine_config_v1.schema.json")
	schemaBytes, err := os.ReadFile(schemaPath)
	if err != nil {
		t.Fatalf("read schema: %v", err)
	}
	var schemaAny any
	if err := json.Unmarshal(schemaBytes, &schemaAny); err != nil {
		t.Fatalf("schema is not valid JSON: %v", err)
	}
}
