package models

// StrPtr returns a pointer to s.
func StrPtr(s string) *string { return &s }

// Float64Ptr returns a pointer to f.
func Float64Ptr(f float64) *float64 { return &f }

// IntPtr returns a pointer to i.
func IntPtr(i int) *int { return &i }

// BoolPtr returns a pointer to b.
func BoolPtr(b bool) *bool { return &b }

// CorrectionData holds a raw flat float64 correction grid and its shape.
// NaN values represent out-of-field cells (identical convention to Python/Rust/Node.js/C++).
// This type is not part of canonical JSON output; grids are read via dedicated reader APIs.
type CorrectionData struct {
	Data  []float64
	Shape [3]int
}

// MachineConfig is the top-level configuration document.
type MachineConfig struct {
	Meta          MachineConfigMeta `json:"meta"`
	Machine       Machine           `json:"machine"`
	OpticalTrains []OpticalTrain    `json:"optical_trains"`
	Opcua         *OpcuaConfig      `json:"opcua,omitempty"`
}

// MachineConfigMeta holds root-level metadata attributes.
type MachineConfigMeta struct {
	SchemaVersion     string `json:"schema_version"`
	MachineName       string `json:"machine_name"`
	Manufacturer      string `json:"manufacturer"`
	Model             string `json:"model"`
	SerialNumber      string `json:"serial_number"`
	FileVersion       string `json:"file_version"`
	ExportDate        string `json:"export_date"`
	ConfigurationHash string `json:"configuration_hash"`
	// TEST FIXTURE for the mock v1.1 adapter (docs/migrations/mock_v1_0_to_v1_1.md).
	// Not a real schema field, never populated for a v1.0 config — only the
	// mock v1.1 reader/writer (test-only, in go/*_test.go) ever sets it.
	// omitempty keeps it out of every real JSON export unless explicitly set,
	// matching Python's/Rust's/Node's behavior for the same fixture fields.
	FacilityID *string `json:"facility_id,omitempty"`
	// TEST FIXTURE for the mock v1.1 adapter — see FacilityID above.
	ConfigAuthor *string        `json:"config_author,omitempty"`
	Extra        map[string]any `json:"extra"`
}

// BuildPlate holds build-plate dimensions from the Machine/Build_Plate/ HDF5 subgroup.
// Canonical JSON flattens these onto Machine as build_plate_x, build_plate_y, etc.
type BuildPlate struct {
	X                *float64 `json:"x,omitempty"`
	XUnit            *string  `json:"x_unit,omitempty"`
	Y                *float64 `json:"y,omitempty"`
	YUnit            *string  `json:"y_unit,omitempty"`
	Z                *float64 `json:"z,omitempty"`
	ZUnit            *string  `json:"z_unit,omitempty"`
	CornerRadius     *float64 `json:"corner_radius,omitempty"`
	CornerRadiusUnit *string  `json:"corner_radius_unit,omitempty"`
}

// Machine holds machine-level attributes. Build-plate dimensions are stored as flat
// keys (build_plate_x, build_plate_y, etc.) in JSON, not as a nested sub-object.
type Machine struct {
	ID                   *string  `json:"id"`
	MachineName          string   `json:"machine_name"`
	Manufacturer         string   `json:"manufacturer"`
	Model                string   `json:"model"`
	SerialNumber         string   `json:"serial_number"`
	BuildPlateX          *float64 `json:"build_plate_x"`
	BuildPlateXUnit      *string  `json:"build_plate_x_unit"`
	BuildPlateY          *float64 `json:"build_plate_y"`
	BuildPlateYUnit      *string  `json:"build_plate_y_unit"`
	BuildPlateZ          *float64 `json:"build_plate_z"`
	BuildPlateZUnit      *string  `json:"build_plate_z_unit"`
	BuildPlateRadius     *float64 `json:"build_plate_radius"`
	BuildPlateRadiusUnit *string  `json:"build_plate_radius_unit"`
	GasFlowDirection     *string  `json:"gas_flow_direction"`
	RecoatDirection      *string  `json:"recoat_direction"`
}

// OpticalTrain holds one optical train's configuration.
type OpticalTrain struct {
	TrainID                           string                   `json:"train_id"`
	ID                                *string                  `json:"id"`
	BeamProfileType                   *string                  `json:"beam_profile_type"`
	BeamWaistDefinition               *string                  `json:"beam_waist_definition"`
	BeamWaistMajor                    *float64                 `json:"beam_waist_major"`
	BeamWaistMajorUnit                *string                  `json:"beam_waist_major_unit"`
	BeamWaistMinor                    *float64                 `json:"beam_waist_minor"`
	BeamWaistMinorUnit                *string                  `json:"beam_waist_minor_unit"`
	BeamWaistOffsetZ                  *float64                 `json:"beam_waist_offset_z"`
	BeamWaistOffsetZUnit              *string                  `json:"beam_waist_offset_z_unit"`
	M2Major                           *float64                 `json:"m2_major"`
	M2Minor                           *float64                 `json:"m2_minor"`
	RayleighLengthMajor               *float64                 `json:"rayleigh_length_major"`
	RayleighLengthMajorUnit           *string                  `json:"rayleigh_length_major_unit"`
	RayleighLengthMinor               *float64                 `json:"rayleigh_length_minor"`
	RayleighLengthMinorUnit           *string                  `json:"rayleigh_length_minor_unit"`
	BuildPlaneOffsetMajor             *float64                 `json:"build_plane_offset_major"`
	BuildPlaneOffsetMajorUnit         *string                  `json:"build_plane_offset_major_unit"`
	BuildPlaneOffsetMinor             *float64                 `json:"build_plane_offset_minor"`
	BuildPlaneOffsetMinorUnit         *string                  `json:"build_plane_offset_minor_unit"`
	CollimatorFocalLength             *float64                 `json:"collimator_focal_length"`
	CollimatorFocalLengthUnit         *string                  `json:"collimator_focal_length_unit"`
	MajorAxisAngle                    *float64                 `json:"major_axis_angle"`
	MajorAxisAngleUnit                *string                  `json:"major_axis_angle_unit"`
	ScannerNumber                     *string                  `json:"scanner_number"`
	ThermalLensingPassed              *bool                    `json:"thermal_lensing_passed"`
	ThermalLensingFocalPlaneShift     *float64                 `json:"thermal_lensing_focal_plane_shift"`
	ThermalLensingFocalPlaneShiftUnit *string                  `json:"thermal_lensing_focal_plane_shift_unit"`
	ThermalLensingThreshold           *float64                 `json:"thermal_lensing_threshold"`
	ThermalLensingThresholdUnit       *string                  `json:"thermal_lensing_threshold_unit"`
	Scanner                           Scanner                  `json:"scanner"`
	LightSource                       LightSource              `json:"light_source"`
	Collimator                        Collimator               `json:"collimator"`
	ScannerCard                       ScannerCard              `json:"scanner_card"`
	OptionalComponents                OptionalComponents       `json:"optional_components"`
	ScanFieldCorrectionFile           *ScanFieldCorrectionFile `json:"scan_field_correction_file"`
}

// Scanner holds galvo scanner configuration.
type Scanner struct {
	Manufacturer         string      `json:"manufacturer"`
	Model                string      `json:"model"`
	SerialNumber         string      `json:"serial_number"`
	WorkingDistance      *float64    `json:"working_distance"`
	WorkingDistanceUnit  *string     `json:"working_distance_unit"`
	ScanFieldX           *float64    `json:"scan_field_x"`
	ScanFieldXUnit       *string     `json:"scan_field_x_unit"`
	ScanFieldY           *float64    `json:"scan_field_y"`
	ScanFieldYUnit       *string     `json:"scan_field_y_unit"`
	ScanFieldZ           *float64    `json:"scan_field_z"`
	ScanFieldZUnit       *string     `json:"scan_field_z_unit"`
	ScanHeadOffsetX      *float64    `json:"scan_head_offset_x"`
	ScanHeadOffsetXUnit  *string     `json:"scan_head_offset_x_unit"`
	ScanHeadOffsetY      *float64    `json:"scan_head_offset_y"`
	ScanHeadOffsetYUnit  *string     `json:"scan_head_offset_y_unit"`
	ScanHeadOffsetZ      *float64    `json:"scan_head_offset_z"`
	ScanHeadOffsetZUnit  *string     `json:"scan_head_offset_z_unit"`
	ScanHeadRotation     *float64    `json:"scan_head_rotation"`
	ScanHeadRotationUnit *string     `json:"scan_head_rotation_unit"`
	AxisConfiguration    *string     `json:"axis_configuration"`
	XAxis                AxisConfig  `json:"x_axis"`
	YAxis                AxisConfig  `json:"y_axis"`
	ZAxis                *AxisConfig `json:"z_axis"`
	Focus                *AxisConfig `json:"focus"`
	// Plain bool, not *bool — deliberately different from every other bool
	// field in this schema. Defaults to false whether the on-disk attribute
	// is absent or explicitly 0 (user-confirmed, 2026-08-21). `omitempty` on
	// a plain bool omits the key for the zero value (false), which is
	// exactly the desired rule: only true ever appears in output (MCF or
	// JSON) — a write->read round-trip is deliberately lossy for an
	// explicit false, which becomes indistinguishable from "never set".
	InvertActualX    bool `json:"invert_actual_x,omitempty"`
	InvertActualY    bool `json:"invert_actual_y,omitempty"`
	InvertCommandedX bool `json:"invert_commanded_x,omitempty"`
	InvertCommandedY bool `json:"invert_commanded_y,omitempty"`
}

// AxisConfig holds one scanner axis tuning subgroup (X_Axis, Y_Axis, Z_Axis, Focus).
type AxisConfig struct {
	ActualBitResolution        *int     `json:"actual_bit_resolution"`
	ActualBitResolutionUnit    *string  `json:"actual_bit_resolution_unit"`
	CommandedBitResolution     *int     `json:"commanded_bit_resolution"`
	CommandedBitResolutionUnit *string  `json:"commanded_bit_resolution_unit"`
	ControlType                *string  `json:"control_type"`
	RangeOfMotion              *float64 `json:"range_of_motion"`
	RangeOfMotionUnit          *string  `json:"range_of_motion_unit"`
	SmoothingKernel            *string  `json:"smoothing_kernel"`
	SmoothingParameters        *float64 `json:"smoothing_parameters"`
	TuningParameters           *string  `json:"tuning_parameters"`
	TuningType                 *string  `json:"tuning_type"`
}

// LightSource holds laser/light-source configuration.
type LightSource struct {
	Manufacturer           string   `json:"manufacturer"`
	Model                  string   `json:"model"`
	SerialNumber           string   `json:"serial_number"`
	Wavelength             *float64 `json:"wavelength"`
	WavelengthUnit         *string  `json:"wavelength_unit"`
	PowerMaxNominal        *float64 `json:"power_max_nominal"`
	PowerMaxNominalUnit    *string  `json:"power_max_nominal_unit"`
	PowerMaxActual         *float64 `json:"power_max_actual"`
	PowerMaxActualUnit     *string  `json:"power_max_actual_unit"`
	PowerMinActual         *float64 `json:"power_min_actual"`
	PowerMinActualUnit     *string  `json:"power_min_actual_unit"`
	PowerMinNominal        *float64 `json:"power_min_nominal"`
	PowerMinNominalUnit    *string  `json:"power_min_nominal_unit"`
	PowerBitResolution     *float64 `json:"power_bit_resolution"`
	PowerBitResolutionUnit *string  `json:"power_bit_resolution_unit"`
	WattsToVoltsAlgorithm  *string  `json:"watts_to_volts_algorithm"`
	WattsToVoltsParams     *string  `json:"watts_to_volts_params"`
}

// Collimator holds collimator hardware metadata.
type Collimator struct {
	Manufacturer    string   `json:"manufacturer"`
	Model           string   `json:"model"`
	SerialNumber    string   `json:"serial_number"`
	FocalLength     *float64 `json:"focal_length"`
	FocalLengthUnit *string  `json:"focal_length_unit"`
}

// ScannerCard holds scanner control card metadata.
type ScannerCard struct {
	Manufacturer          string   `json:"manufacturer"`
	Model                 string   `json:"model"`
	SerialNumber          string   `json:"serial_number"`
	CommunicationProtocol *string  `json:"communication_protocol"`
	SamplePeriod          *float64 `json:"sample_period"`
	SamplePeriodUnit      *string  `json:"sample_period_unit"`
}

// OptionalComponents holds optional add-on hardware on an optical train.
type OptionalComponents struct {
	Clearbox *ClearBox `json:"clearbox"`
}

// ClearBox holds ClearBox add-on component configuration.
type ClearBox struct {
	IPAddress                 string          `json:"ip_address"`
	SerialNumber              *string         `json:"serial_number"`
	DataPort                  *int            `json:"data_port"`
	ServerPort                *int            `json:"server_port"`
	ActualTimingOffset        *int            `json:"actual_timing_offset"`
	CommandedTimingOffset     *int            `json:"commanded_timing_offset"`
	CorrectionData            *[][][]*float64 `json:"correction_data,omitempty"`
	InverseCorrectionData     *[][][]*float64 `json:"inverse_correction_data,omitempty"`
	Manufacturer              *string         `json:"manufacturer"`
	Model                     *string         `json:"model"`
	OutputPath                *string         `json:"output_path"`
	SelectedCamera            *string         `json:"selected_camera"`
	CustomVideoFormat         *string         `json:"custom_video_format"`
	VideoOutput               *string         `json:"video_output"`
	ShowConsole               *bool           `json:"show_console"`
	SoftwareTriggerDelay      *int            `json:"software_trigger_delay"`
	VoltsToWattsAlgorithm     *string         `json:"volts_to_watts_algorithm"`
	VoltsToWattsParams        *string         `json:"volts_to_watts_params"`
	CorrectionGridDomainShape *string         `json:"correction_grid_domain_shape"`
	InverseGridDomainShape    *string         `json:"inverse_grid_domain_shape"`
	// omitempty: omitted entirely (not "{}") when there are no sensors —
	// matches Rust's skip_serializing_if and Python's _clearbox_to_dict
	// choice to omit the key when empty, so every fixture that doesn't use
	// this feature stays byte-for-byte identical in JSON shape to before it
	// existed. Same optional-whole-feature shape as MachineConfig.Opcua,
	// not OpcuaConfig.Triggers (which is always present, even as {}) —
	// deliberately different from that precedent.
	SynchronousSensors map[string]SynchronousSensor `json:"synchronous_sensors,omitempty"`
}

// EquationConstant is one named constant used to derive an equation (e.g.
// `a`/`b` for a Log-Linear fit, `c0`..`cN` for a polynomial fit). Stored
// on disk as a 64-byte fixed-length UTF-8 string (see
// SYNCHRONOUS_SENSOR_PLAN.md's "Compound dataset string convention") —
// Name here is a plain Go string; the fixed-width conversion happens only
// at the h5c layer (h5c.EquationConstantRow).
type EquationConstant struct {
	Name  string  `json:"name"`
	Value float64 `json:"value"`
}

// CalibrationPoint is one raw calibration pair. InputValue is in whatever
// unit SynchronousSensor.InputType implies; OutputValue is in whatever unit
// SynchronousSensor.SensorOutputSpace implies (no per-row unit tag) — see
// SYNCHRONOUS_SENSOR_PLAN.md's "Why compound datasets" for the convention.
type CalibrationPoint struct {
	InputValue  float64 `json:"input_value"`
	OutputValue float64 `json:"output_value"`
}

// SynchronousSensor holds one Synchronous Sensor record. The map key (on
// ClearBox.SynchronousSensors) is a free-form label chosen by the file's
// author — not required to equal any attribute value inside the sensor's
// own group (same convention as OpcuaConfig.Triggers's keys).
type SynchronousSensor struct {
	Enabled                     *bool              `json:"enabled"`
	SensorName                  *string            `json:"sensor_name"`
	SensorOutputRangeLow        *float64           `json:"sensor_output_range_low"`
	SensorOutputRangeHigh       *float64           `json:"sensor_output_range_high"`
	SensorOutputSpace           *string            `json:"sensor_output_space"`
	SensorModel                 *string            `json:"sensor_model"`
	SensorManufacturer          *string            `json:"sensor_manufacturer"`
	SensorScope                 *string            `json:"sensor_scope"`
	UnitsDerivedQuantity        *string            `json:"units_derived_quantity"`
	PortID                      *int               `json:"port_id"`
	SensorType                  *string            `json:"sensor_type"`
	InputType                   *string            `json:"input_type"`
	AlgorithmType               *string            `json:"algorithm_type"`
	AlgorithmEquation           *string            `json:"algorithm_equation"`
	CalibrationSource           *string            `json:"calibration_source"`
	CalibrationVerified         *bool              `json:"calibration_verified"`
	SamplePeriod                *float64           `json:"sample_period"`
	Metadata                    *string            `json:"metadata"`
	DerivationEquationConstants []EquationConstant `json:"derivation_equation_constants"`
	CalibrationPoints           []CalibrationPoint `json:"calibration_points"`
}

// ScanFieldCorrectionFile holds metadata for the embedded .fc3 scan-field correction file.
type ScanFieldCorrectionFile struct {
	DocumentName      string  `json:"document_name"`
	DocumentID        string  `json:"document_id"`
	FileSize          int     `json:"file_size"`
	ValidAsOfDate     string  `json:"valid_as_of_date"`
	DocumentCreatedAt *string `json:"document_created_at"`
	DocumentType      *string `json:"document_type"`
	OriginalURI       *string `json:"original_uri"`
	RawBytes          []byte  `json:"raw_bytes,omitempty"`
}

// OpcuaConfig holds OPC-UA connectivity configuration.
type OpcuaConfig struct {
	Client                   OpcuaClientConfig       `json:"client"`
	Pipe                     OpcuaPipeConfig         `json:"pipe"`
	TriggersEnabled          *bool                   `json:"triggers_enabled"`
	TriggerStopCeilingLayers *int                    `json:"trigger_stop_ceiling_layers"`
	Triggers                 map[string]OpcuaTrigger `json:"triggers"`
}

// OpcuaClientConfig holds OPC-UA client connection settings.
type OpcuaClientConfig struct {
	ServerURL               string         `json:"server_url"`
	AuthMode                string         `json:"auth_mode"`
	SecurityMode            string         `json:"security_mode"`
	SecurityPolicy          string         `json:"security_policy"`
	BfsMaxDepth             int            `json:"bfs_max_depth"`
	PublishInterval         int            `json:"publish_interval"`
	SamplingInterval        int            `json:"sampling_interval"`
	SessionTimeout          int            `json:"session_timeout"`
	KeepAliveCount          *int           `json:"keep_alive_count"`
	LifetimeCount           *int           `json:"lifetime_count"`
	MachineProfile          *string        `json:"machine_profile"`
	QueuePolicy             *string        `json:"queue_policy"`
	QueueSizeDataChange     *int           `json:"queue_size_data_change"`
	QueueSizeEvents         *int           `json:"queue_size_events"`
	ReconnectInterval       *int           `json:"reconnect_interval"`
	RootNode                *string        `json:"root_node"`
	SyncLoopIntervalInitial *int           `json:"sync_loop_interval_initial"`
	SyncLoopIntervalSettled *int           `json:"sync_loop_interval_settled"`
	Extra                   map[string]any `json:"extra"`
}

// OpcuaPipeConfig holds OPC-UA pipe settings.
type OpcuaPipeConfig struct {
	PipeEnabled           bool           `json:"pipe_enabled"`
	BufferSize            int            `json:"buffer_size"`
	ConfigureClient       *bool          `json:"configure_client"`
	InboundRateLimit      *int           `json:"inbound_rate_limit"`
	MaxInboundMessageSize *int           `json:"max_inbound_message_size"`
	MinIntegrityLevel     *string        `json:"min_integrity_level"`
	PipeName              *string        `json:"pipe_name"`
	UserAccessLevel       *string        `json:"user_access_level"`
	Extra                 map[string]any `json:"extra"`
}

// OpcuaTrigger holds one OPC-UA trigger rule.
type OpcuaTrigger struct {
	ID              *string        `json:"id"`
	Signal          *string        `json:"signal"`
	Subsystem       *string        `json:"subsystem"`
	RuleEnabled     *bool          `json:"rule_enabled"`
	StartValue      *string        `json:"start_value"`
	StopValue       *string        `json:"stop_value"`
	CaseSensitivity *string        `json:"case_sensitivity"`
	Component       *string        `json:"component"`
	CooldownPeriod  *int           `json:"cooldown_period"`
	Event           *string        `json:"event"`
	MaxFiresPerJob  *int           `json:"max_fires_per_job"`
	TriggerLabel    *string        `json:"trigger_label"`
	Extra           map[string]any `json:"extra"`
}
