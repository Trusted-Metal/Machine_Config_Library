package models

import "machine-config-go/extension"

// StrPtr returns a pointer to s.
func StrPtr(s string) *string { return &s }

// Float64Ptr returns a pointer to f.
func Float64Ptr(f float64) *float64 { return &f }

// IntPtr returns a pointer to i.
func IntPtr(i int) *int { return &i }

// BoolPtr returns a pointer to b.
func BoolPtr(b bool) *bool { return &b }

// MachineConfig is the top-level configuration document.
type MachineConfig struct {
	Meta          MachineConfigMeta `json:"meta"`
	Machine       Machine           `json:"machine"`
	OpticalTrains []OpticalTrain    `json:"optical_trains"`
	// Extensions holds opaque extension trees keyed by group name under Extensions/. Core reads and
	// writes them verbatim (see docs/EXTENSION_CONTRACT.md). ClearBox is one of them: core carries it
	// as an opaque tree in the File_Version 1.1 layout for both file versions (clearbox_legacy.go).
	Extensions map[string]*extension.Tree `json:"extensions,omitempty"`
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
	ConfigAuthor *string `json:"config_author,omitempty"`
	// IsValid is reader-computed, in-memory only: non-nil once a
	// MachineConfigReader has recomputed the configuration hash and compared
	// it to the on-disk value; nil until then. Never set by a writer or by
	// MockConfigBuilder. `json:"-"` unconditionally excludes it from
	// serialization — never written to HDF5, never part of the interop
	// contract. A mismatch is never an error: the file remains fully
	// readable either way (see hash.go).
	IsValid *bool          `json:"-"`
	Extra   map[string]any `json:"extra"`
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
	RecoaterBladeType    *string  `json:"recoater_blade_type"`
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
	// PowerCharacterization is the only representation of the
	// watts<->volts conversion concept, for files of either version.
	// omitempty for the same byte-identical-output reasoning as
	// ClearBox.SynchronousSensors.
	PowerCharacterization *PowerCharacterization `json:"power_characterization,omitempty"`
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
// unit the owning extension's input type implies; OutputValue is in whatever unit
// the owning extension's output space implies (no per-row unit tag) — see
// SYNCHRONOUS_SENSOR_PLAN.md's "Why compound datasets" for the convention.
type CalibrationPoint struct {
	InputValue  float64 `json:"input_value"`
	OutputValue float64 `json:"output_value"`
}

// PowerCharacterization is a v1.1 addition (Changes 3/4): a structured
// algorithm + equation + constants + characterization points describing a
// power conversion. Shared, identical struct for both
// ClearBox.PowerCharacterization (ClearBox's Volts->Watts fit; migrated data
// has real DerivationEquationConstants but zero CharacterizationPoints) and
// LightSource.PowerCharacterization (Light_Source's Volts->Watts fit;
// migrated data is the inverse — zero DerivationEquationConstants, real
// CharacterizationPoints) — same kind of thing at two different HDF5 paths,
// not the same instance. See docs/migrations/v1_0_to_v1_1.md Changes 3/4 for
// the full derivation rules.
type PowerCharacterization struct {
	AlgorithmType               *string            `json:"algorithm_type"`
	AlgorithmEquation           *string            `json:"algorithm_equation"`
	InputType                   *string            `json:"input_type"`
	UnitsDerivedQuantity        *string            `json:"units_derived_quantity"`
	DerivationEquationConstants []EquationConstant `json:"derivation_equation_constants"`
	CharacterizationPoints      []CalibrationPoint `json:"characterization_points"`
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
