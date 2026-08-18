// Phase 3.4 — Data models: MachineConfig and its full field/type mirror of the
// Python reference implementation (python/src/machine_config/models.py).
// See IMPLEMENTATION_PLAN.md §3.4 for design decisions and §3.10 for the
// HDF5 attribute -> field mapping that governs field names/types here.
//
// Struct shape follows the canonical JSON output (fixtures/reference_output.json)
// rather than Python's internal dataclass nesting in the one place they diverge:
// `Machine` inlines the build-plate fields directly (`build_plate_x`, ...)
// instead of nesting a separate `BuildPlate` struct, matching §3.10's table
// and the flat `machine` object in the canonical schema (§0.2).
//
// Validation (e.g. Scanner.axis_configuration's "2D"/"3D"/"3D+Focus"
// constraint, which Python enforces in `__post_init__`) is deliberately not
// implemented here — Rust has no dataclass post-init hook, so it belongs in
// the reader's `validate()` step (Phase 3.5), not in these plain data structs.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Raw correction grid read directly from a ClearBox HDF5 dataset.
///
/// Values are stored in **row-major (C-order)** layout — the same memory order
/// used by HDF5 and NumPy. Given a shape `[d0, d1, d2]`, the element at
/// `(i, j, k)` lives at index `i * d1 * d2 + j * d2 + k`.
///
/// For an AconityMIDI machine the shape is `[257, 257, 2]`: 257 × 257 spatial
/// positions with an X-correction and a Y-correction value at each position.
/// Cells outside the scan-field boundary are encoded as IEEE 754 NaN.
///
/// # Why not `ndarray::Array3<f64>`?
///
/// Returning `Array3` in the public API would expose `ndarray` as an implicit
/// version constraint on every downstream crate. By returning a plain-`std`
/// type instead, this library stays version-agnostic: consumers may use any
/// `ndarray` release they like and reconstruct the array with a single call:
///
/// ```rust
/// # use machine_config::models::CorrectionData;
/// # let cd = CorrectionData { data: vec![0.0_f64; 257 * 257 * 2], shape: [257, 257, 2] };
/// let arr = ndarray::Array3::from_shape_vec(cd.shape, cd.data)
///     .expect("shape is always consistent");
/// assert_eq!(arr.shape(), &[257, 257, 2]);
/// ```
///
/// # Accessing elements without ndarray
///
/// ```rust
/// # use machine_config::models::CorrectionData;
/// # let cd = CorrectionData { data: vec![1.0_f64; 257 * 257 * 2], shape: [257, 257, 2] };
/// # let (i, j, k) = (10_usize, 20_usize, 1_usize);
/// // value at (i, j, k)
/// let [_, d1, d2] = cd.shape;
/// let val = cd.data[i * d1 * d2 + j * d2 + k];
/// # assert_eq!(val, 1.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CorrectionData {
    /// Flat, row-major buffer of `f64` values.
    /// `data.len() == shape[0] * shape[1] * shape[2]` is always true.
    /// NaN values represent out-of-field cells.
    pub data: Vec<f64>,
    /// Array dimensions as `[d0, d1, d2]`, e.g. `[257, 257, 2]`.
    pub shape: [usize; 3],
}

/// Arbitrary, non-schema HDF5 attributes preserved verbatim.
/// `IndexMap` (not `HashMap`) to preserve HDF5 attribute enumeration order,
/// matching Python's `dict` insertion-order semantics for JSON parity.
pub type ExtraAttrs = IndexMap<String, serde_json::Value>;

/// Metadata for the embedded `.fc3` scan-field correction file.
/// HDF5 source: a Dataset (not a Group) at `.../scan_field_correction_file`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanFieldCorrectionFile {
    pub document_name: String,
    pub document_id: String,
    pub file_size: i64,
    pub valid_as_of_date: String,
    pub document_created_at: Option<String>,
    pub document_type: Option<String>,
    pub original_uri: Option<String>,
    /// Raw `.fc3` bytes. Only present when read via `parse_with_binary()` /
    /// `to_json(include_binary = true)` — omitted from JSON entirely otherwise.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub raw_bytes: Option<Vec<u8>>,
}

/// Optional ClearBox add-on component. HDF5 source: `.../Optional_Components/ClearBox/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClearBox {
    pub ip_address: String,
    pub serial_number: Option<String>,
    pub data_port: Option<i64>,
    pub server_port: Option<i64>,
    pub actual_timing_offset: Option<i64>,
    pub commanded_timing_offset: Option<i64>,
    /// `(257, 257, 2)` float64 grid; out-of-field NaN cells become `None`.
    /// Only present when read via `parse_with_binary()` — omitted from JSON otherwise.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub correction_data: Option<Vec<Vec<Vec<Option<f64>>>>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub inverse_correction_data: Option<Vec<Vec<Vec<Option<f64>>>>>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub output_path: Option<String>,
    pub selected_camera: Option<String>,
    pub custom_video_format: Option<String>,
    pub video_output: Option<String>,
    /// HDF5 int 0/1.
    pub show_console: Option<bool>,
    pub software_trigger_delay: Option<i64>,
    pub volts_to_watts_algorithm: Option<String>,
    pub volts_to_watts_params: Option<String>,
    pub correction_grid_domain_shape: Option<String>,
    pub inverse_grid_domain_shape: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collimator {
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub focal_length: Option<f64>,
    pub focal_length_unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScannerCard {
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub communication_protocol: Option<String>,
    pub sample_period: Option<f64>,
    pub sample_period_unit: Option<String>,
}

/// One scanner axis tuning sub-group (`X_Axis`, `Y_Axis`, `Z_Axis`, `Focus`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxisConfig {
    pub actual_bit_resolution: Option<i64>,
    pub actual_bit_resolution_unit: Option<String>,
    pub commanded_bit_resolution: Option<i64>,
    pub commanded_bit_resolution_unit: Option<String>,
    pub control_type: Option<String>,
    pub range_of_motion: Option<f64>,
    pub range_of_motion_unit: Option<String>,
    pub smoothing_kernel: Option<String>,
    pub smoothing_parameters: Option<f64>,
    pub tuning_parameters: Option<String>,
    pub tuning_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scanner {
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub working_distance: Option<f64>,
    pub working_distance_unit: Option<String>,
    pub scan_field_x: Option<f64>,
    pub scan_field_x_unit: Option<String>,
    pub scan_field_y: Option<f64>,
    pub scan_field_y_unit: Option<String>,
    pub scan_field_z: Option<f64>,
    pub scan_field_z_unit: Option<String>,
    pub scan_head_offset_x: Option<f64>,
    pub scan_head_offset_x_unit: Option<String>,
    pub scan_head_offset_y: Option<f64>,
    pub scan_head_offset_y_unit: Option<String>,
    pub scan_head_offset_z: Option<f64>,
    pub scan_head_offset_z_unit: Option<String>,
    pub scan_head_rotation: Option<f64>,
    pub scan_head_rotation_unit: Option<String>,
    /// `"2D"` / `"3D"` / `"3D+Focus"`; constraint enforced by the reader
    /// (`validate()`), not here — Rust has no dataclass `__post_init__`.
    pub axis_configuration: Option<String>,
    pub x_axis: AxisConfig,
    pub y_axis: AxisConfig,
    pub z_axis: Option<AxisConfig>,
    pub focus: Option<AxisConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightSource {
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub wavelength: Option<f64>,
    pub wavelength_unit: Option<String>,
    pub power_max_nominal: Option<f64>,
    pub power_max_nominal_unit: Option<String>,
    pub power_max_actual: Option<f64>,
    pub power_max_actual_unit: Option<String>,
    pub power_min_actual: Option<f64>,
    pub power_min_actual_unit: Option<String>,
    pub power_min_nominal: Option<f64>,
    pub power_min_nominal_unit: Option<String>,
    /// HDF5 stores this attribute as a string; the reader parses it to `f64`.
    pub power_bit_resolution: Option<f64>,
    pub power_bit_resolution_unit: Option<String>,
    pub watts_to_volts_algorithm: Option<String>,
    pub watts_to_volts_params: Option<String>,
}

/// Optional add-on hardware that may or may not be installed on an optical train.
/// Maps to the `Optional_Components` HDF5 group under each `Optical_Train_NN`.
/// Always present on `OpticalTrain` (the group always exists); contents vary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionalComponents {
    pub clearbox: Option<ClearBox>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpticalTrain {
    pub train_id: String,
    /// UUID attribute on the train group (often empty in real files).
    pub id: Option<String>,
    pub beam_profile_type: Option<String>,
    pub beam_waist_definition: Option<String>,
    pub beam_waist_major: Option<f64>,
    pub beam_waist_major_unit: Option<String>,
    pub beam_waist_minor: Option<f64>,
    pub beam_waist_minor_unit: Option<String>,
    pub beam_waist_offset_z: Option<f64>,
    pub beam_waist_offset_z_unit: Option<String>,
    pub m2_major: Option<f64>,
    pub m2_minor: Option<f64>,
    pub rayleigh_length_major: Option<f64>,
    pub rayleigh_length_major_unit: Option<String>,
    pub rayleigh_length_minor: Option<f64>,
    pub rayleigh_length_minor_unit: Option<String>,
    pub build_plane_offset_major: Option<f64>,
    pub build_plane_offset_major_unit: Option<String>,
    pub build_plane_offset_minor: Option<f64>,
    pub build_plane_offset_minor_unit: Option<String>,
    /// Train-level duplicate of `collimator.focal_length`.
    pub collimator_focal_length: Option<f64>,
    pub collimator_focal_length_unit: Option<String>,
    pub major_axis_angle: Option<f64>,
    pub major_axis_angle_unit: Option<String>,
    pub scanner_number: Option<String>,
    /// HDF5 int 0/1; `None` if absent.
    pub thermal_lensing_passed: Option<bool>,
    pub thermal_lensing_focal_plane_shift: Option<f64>,
    pub thermal_lensing_focal_plane_shift_unit: Option<String>,
    pub thermal_lensing_threshold: Option<f64>,
    pub thermal_lensing_threshold_unit: Option<String>,
    pub scanner: Scanner,
    pub light_source: LightSource,
    /// Required; see Rule 7 (§0.5) — a train missing a collimator is invalid.
    pub collimator: Collimator,
    /// Required; see Rule 7 (§0.5) — a train missing a scanner card is invalid.
    pub scanner_card: ScannerCard,
    pub optional_components: OptionalComponents,
    pub scan_field_correction_file: Option<ScanFieldCorrectionFile>,
}

/// Machine-level attributes. Build-plate fields are inlined here (rather than
/// nested under a separate `BuildPlate` struct as in the Python model) to
/// match the flat `machine` object in the canonical JSON schema (§0.2, §3.10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    pub id: Option<String>,
    pub machine_name: String,
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub build_plate_x: Option<f64>,
    pub build_plate_x_unit: Option<String>,
    pub build_plate_y: Option<f64>,
    pub build_plate_y_unit: Option<String>,
    pub build_plate_z: Option<f64>,
    pub build_plate_z_unit: Option<String>,
    pub build_plate_radius: Option<f64>,
    pub build_plate_radius_unit: Option<String>,
    pub gas_flow_direction: Option<String>,
    pub recoat_direction: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineConfigMeta {
    pub schema_version: String,
    pub machine_name: String,
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub file_version: String,
    pub export_date: String,
    pub configuration_hash: String,
    /// TEST FIXTURE for the mock v1.1 adapter (docs/migrations/mock_v1_0_to_v1_1.md).
    /// Not a real schema field, never serialized for a v1.0 config — only the
    /// mock v1.1 reader/writer (test-only, not part of this crate) ever
    /// populates it. `skip_serializing_if` keeps it out of every real JSON
    /// export unless explicitly set, matching Python's/Node's behavior for
    /// the same fixture fields.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub facility_id: Option<String>,
    /// TEST FIXTURE for the mock v1.1 adapter — see `facility_id` above.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub config_author: Option<String>,
    /// Preserves any non-typed root HDF5 attrs.
    #[serde(default)]
    pub extra: ExtraAttrs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpcuaClientConfig {
    pub server_url: String,
    pub auth_mode: String,
    pub security_mode: String,
    pub security_policy: String,
    pub bfs_max_depth: i64,
    pub publish_interval: i64,
    pub sampling_interval: i64,
    pub session_timeout: i64,
    /// Preserves any non-typed HDF5 attrs.
    #[serde(default)]
    pub extra: ExtraAttrs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpcuaPipeConfig {
    /// HDF5 int 0/1.
    pub pipe_enabled: bool,
    pub buffer_size: i64,
    /// Preserves any non-typed HDF5 attrs.
    #[serde(default)]
    pub extra: ExtraAttrs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpcuaTrigger {
    pub id: Option<String>,
    pub signal: Option<String>,
    pub subsystem: Option<String>,
    /// HDF5 int 0/1.
    pub rule_enabled: Option<bool>,
    pub start_value: Option<String>,
    pub stop_value: Option<String>,
    #[serde(default)]
    pub extra: ExtraAttrs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpcuaConfig {
    pub client: OpcuaClientConfig,
    pub pipe: OpcuaPipeConfig,
    /// Key = trigger label (HDF5 sub-group name under `OPCUA/Triggers/`).
    /// `IndexMap` preserves HDF5 group enumeration order.
    pub triggers: IndexMap<String, OpcuaTrigger>,
    /// HDF5 float64 `0.0`/`1.0` on the `OPCUA/Triggers` group attrs — not an int.
    pub triggers_enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineConfig {
    pub meta: MachineConfigMeta,
    pub machine: Machine,
    pub optical_trains: Vec<OpticalTrain>,
    /// Machine connectivity configuration, outside the canonical schema (§0.2).
    /// Omitted from JSON entirely when absent (not serialised as `null`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub opcua: Option<OpcuaConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_axis() -> AxisConfig {
        AxisConfig {
            actual_bit_resolution: Some(20),
            actual_bit_resolution_unit: Some("bits".into()),
            commanded_bit_resolution: Some(20),
            commanded_bit_resolution_unit: Some("bits".into()),
            control_type: None,
            range_of_motion: None,
            range_of_motion_unit: Some("mm".into()),
            smoothing_kernel: Some("GAUSSIAN".into()),
            smoothing_parameters: Some(60.0),
            tuning_parameters: None,
            tuning_type: None,
        }
    }

    fn sample_train() -> OpticalTrain {
        OpticalTrain {
            train_id: "Optical_Train_01".into(),
            id: None,
            beam_profile_type: None,
            beam_waist_definition: Some("knife-edge".into()),
            beam_waist_major: Some(66.72965),
            beam_waist_major_unit: Some("\u{03bc}m".into()),
            beam_waist_minor: None,
            beam_waist_minor_unit: None,
            beam_waist_offset_z: None,
            beam_waist_offset_z_unit: None,
            m2_major: None,
            m2_minor: None,
            rayleigh_length_major: None,
            rayleigh_length_major_unit: None,
            rayleigh_length_minor: None,
            rayleigh_length_minor_unit: None,
            build_plane_offset_major: None,
            build_plane_offset_major_unit: None,
            build_plane_offset_minor: None,
            build_plane_offset_minor_unit: None,
            collimator_focal_length: None,
            collimator_focal_length_unit: None,
            major_axis_angle: None,
            major_axis_angle_unit: None,
            scanner_number: None,
            thermal_lensing_passed: Some(false),
            thermal_lensing_focal_plane_shift: None,
            thermal_lensing_focal_plane_shift_unit: None,
            thermal_lensing_threshold: None,
            thermal_lensing_threshold_unit: None,
            scanner: Scanner {
                manufacturer: "Aconity3D".into(),
                model: "AconityScan".into(),
                serial_number: "01522024181".into(),
                working_distance: Some(670.0),
                working_distance_unit: Some("mm".into()),
                scan_field_x: None,
                scan_field_x_unit: None,
                scan_field_y: None,
                scan_field_y_unit: None,
                scan_field_z: None,
                scan_field_z_unit: None,
                scan_head_offset_x: None,
                scan_head_offset_x_unit: None,
                scan_head_offset_y: None,
                scan_head_offset_y_unit: None,
                scan_head_offset_z: None,
                scan_head_offset_z_unit: None,
                scan_head_rotation: None,
                scan_head_rotation_unit: None,
                axis_configuration: Some("3D".into()),
                x_axis: sample_axis(),
                y_axis: sample_axis(),
                z_axis: Some(sample_axis()),
                focus: None,
            },
            light_source: LightSource {
                manufacturer: "IPG".into(),
                model: "YLR-500".into(),
                serial_number: "L-001".into(),
                wavelength: Some(1070.0),
                wavelength_unit: Some("nm".into()),
                power_max_nominal: None,
                power_max_nominal_unit: None,
                power_max_actual: None,
                power_max_actual_unit: None,
                power_min_actual: None,
                power_min_actual_unit: None,
                power_min_nominal: None,
                power_min_nominal_unit: None,
                power_bit_resolution: None,
                power_bit_resolution_unit: None,
                watts_to_volts_algorithm: None,
                watts_to_volts_params: None,
            },
            collimator: Collimator {
                manufacturer: "Aconity3D".into(),
                model: "COL-1".into(),
                serial_number: "C-001".into(),
                focal_length: Some(120.0),
                focal_length_unit: Some("mm".into()),
            },
            scanner_card: ScannerCard {
                manufacturer: "Aconity3D".into(),
                model: "SC-1".into(),
                serial_number: "SC-001".into(),
                communication_protocol: None,
                sample_period: None,
                sample_period_unit: None,
            },
            optional_components: OptionalComponents { clearbox: None },
            scan_field_correction_file: None,
        }
    }

    fn sample_config() -> MachineConfig {
        MachineConfig {
            meta: MachineConfigMeta {
                schema_version: "v1".into(),
                machine_name: "TM-LPBF-02: AconityMIDI+_OG".into(),
                manufacturer: "Aconity3D".into(),
                model: "AconityMIDI+".into(),
                serial_number: "500300_1".into(),
                file_version: "1.0".into(),
                export_date: "2026-06-09T19:01:02.123Z".into(),
                configuration_hash: "9".repeat(64),
                facility_id: None,
                config_author: None,
                extra: IndexMap::new(),
            },
            machine: Machine {
                id: Some("fefc5037-97ab-401b-bd82-8499962d82c7".into()),
                machine_name: "TM-LPBF-02: AconityMIDI+_OG".into(),
                manufacturer: "Aconity3D".into(),
                model: "AconityMIDI+".into(),
                serial_number: "500300_1".into(),
                build_plate_x: Some(250.0),
                build_plate_x_unit: Some("mm".into()),
                build_plate_y: Some(250.0),
                build_plate_y_unit: Some("mm".into()),
                build_plate_z: Some(20.0),
                build_plate_z_unit: Some("mm".into()),
                build_plate_radius: Some(125.0),
                build_plate_radius_unit: Some("mm".into()),
                gas_flow_direction: Some("Y+".into()),
                recoat_direction: Some("X+".into()),
            },
            optical_trains: vec![sample_train()],
            opcua: None,
        }
    }

    #[test]
    fn configuration_hash_is_64_chars() {
        let config = sample_config();
        assert_eq!(config.meta.configuration_hash.len(), 64);
    }

    #[test]
    fn serde_roundtrip_preserves_all_fields() {
        let config = sample_config();
        let json = serde_json::to_string(&config).unwrap();
        let roundtripped: MachineConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, roundtripped);
    }

    #[test]
    fn absent_opcua_is_omitted_not_null() {
        let config = sample_config();
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("\"opcua\""), "opcua key must be omitted when None");
    }

    #[test]
    fn present_opcua_serialises_as_object() {
        let mut config = sample_config();
        let mut triggers = IndexMap::new();
        triggers.insert(
            "Laser Emission Interlock".to_string(),
            OpcuaTrigger {
                id: None,
                signal: Some("E-Stop".into()),
                subsystem: None,
                rule_enabled: Some(true),
                start_value: None,
                stop_value: None,
                extra: IndexMap::new(),
            },
        );
        config.opcua = Some(OpcuaConfig {
            client: OpcuaClientConfig {
                server_url: "opc.tcp://localhost:4840".into(),
                auth_mode: "Anonymous".into(),
                security_mode: "None".into(),
                security_policy: "None".into(),
                bfs_max_depth: 16,
                publish_interval: 100,
                sampling_interval: 100,
                session_timeout: 60000,
                extra: IndexMap::new(),
            },
            pipe: OpcuaPipeConfig {
                pipe_enabled: true,
                buffer_size: 4096,
                extra: IndexMap::new(),
            },
            triggers,
            triggers_enabled: Some(true),
        });

        let json = serde_json::to_string(&config).unwrap();
        let roundtripped: MachineConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, roundtripped);
        let opcua = roundtripped.opcua.unwrap();
        assert_eq!(opcua.triggers_enabled, Some(true));
        assert!(opcua.triggers.contains_key("Laser Emission Interlock"));
    }

    #[test]
    fn binary_fields_omitted_by_default_and_present_when_set() {
        let mut train = sample_train();
        train.optional_components.clearbox = Some(ClearBox {
            ip_address: "192.168.1.50".into(),
            serial_number: None,
            data_port: None,
            server_port: None,
            actual_timing_offset: None,
            commanded_timing_offset: None,
            correction_data: None,
            inverse_correction_data: None,
            manufacturer: None,
            model: None,
            output_path: None,
            selected_camera: None,
            custom_video_format: None,
            video_output: None,
            show_console: None,
            software_trigger_delay: None,
            volts_to_watts_algorithm: None,
            volts_to_watts_params: None,
            correction_grid_domain_shape: None,
            inverse_grid_domain_shape: None,
        });
        let mut config = sample_config();
        config.optical_trains = vec![train];

        let json_without_binary = serde_json::to_string(&config).unwrap();
        assert!(!json_without_binary.contains("correction_data"));

        // clearbox itself is always present (possibly null) — only the
        // correction grids are conditionally omitted.
        assert!(json_without_binary.contains("\"clearbox\""));

        config.optical_trains[0].optional_components.clearbox.as_mut().unwrap().correction_data =
            Some(vec![vec![vec![Some(1.0), None]]]);
        let json_with_binary = serde_json::to_string(&config).unwrap();
        assert!(json_with_binary.contains("\"correction_data\""));

        let roundtripped: MachineConfig = serde_json::from_str(&json_with_binary).unwrap();
        assert_eq!(config, roundtripped);
    }

    #[test]
    fn extra_map_preserves_insertion_order_through_json() {
        let mut extra = IndexMap::new();
        extra.insert(
            "Description".to_string(),
            serde_json::Value::String("Machine Configuration Export".into()),
        );
        extra.insert(
            "Generator".to_string(),
            serde_json::Value::String("RDT-Core".into()),
        );
        let mut config = sample_config();
        config.meta.extra = extra;

        let json = serde_json::to_string(&config).unwrap();
        let desc_pos = json.find("Description").unwrap();
        let gen_pos = json.find("Generator").unwrap();
        assert!(desc_pos < gen_pos, "extra map must preserve insertion order");

        let roundtripped: MachineConfig = serde_json::from_str(&json).unwrap();
        let keys: Vec<&String> = roundtripped.meta.extra.keys().collect();
        assert_eq!(keys, vec!["Description", "Generator"]);
    }

    #[test]
    fn nan_free_option_none_serialises_as_null() {
        let config = sample_config();
        let json = serde_json::to_string(&config).unwrap();
        // scan_field_x is None on the sample train's scanner.
        assert!(json.contains("\"scan_field_x\":null"));
    }
}
