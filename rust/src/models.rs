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
use mcf_extension_contract::ExtensionTree;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

/// One named equation constant for a power characterization's equation
/// (e.g. `a`/`b` for a Log-Linear fit: `log(ppm) = a*mA + b`). HDF5 source:
/// one row of the `Derivation_Equation_Constants` compound dataset. A named
/// row rather than a positional array so a reader never has to infer which
/// index means what from `algorithm_type` alone, and so adding a constant is
/// an additive row rather than an ordering hazard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquationConstant {
    pub name: String,
    pub value: f64,
}

/// One calibration sample pair. HDF5 source: one row of a `Calibration_Points` or
/// `Characterization_Points` compound dataset.
///
/// `input_value` is in the sensor's `input_type` unit; `output_value` is in
/// the sensor's `sensor_output_space` unit (**not** `units_derived_quantity`)
/// — every row of a given sensor's calibration curve is in the same units by
/// construction. Confirmed against the ZR800 reference example: both points
/// satisfy `algorithm_equation` exactly in log-space
/// (`0.4375 * 4 - 2.75 == -1.0`, `0.4375 * 20 - 2.75 == 6.0`), not linear ppm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationPoint {
    pub input_value: f64,
    pub output_value: f64,
}

/// v1.1 addition (Changes 3/4): a structured algorithm + equation + constants
/// + characterization points describing a power conversion. Used by
/// `LightSource.power_characterization` (Light_Source's Watts→Volts fit). ClearBox's own
/// Volts→Watts fit has the same shape but lives in the opaque `ClearBox` extension tree. See
/// `docs/migrations/v1_0_to_v1_1.md` Changes 3/4 for the full derivation rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerCharacterization {
    pub algorithm_type: Option<String>,
    pub algorithm_equation: Option<String>,
    pub input_type: Option<String>,
    pub units_derived_quantity: Option<String>,
    pub derivation_equation_constants: Vec<EquationConstant>,
    pub characterization_points: Vec<CalibrationPoint>,
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
    /// Plain `bool`, not `Option<bool>` — deliberately different from every
    /// other bool field in this schema. Defaults to `false` whether the
    /// on-disk attribute is absent or explicitly `0`; the API never
    /// distinguishes those two cases (user-confirmed, 2026-08-21). Read
    /// faithfully (0/1/absent all map correctly on read), but serialised
    /// AND written back to HDF5 only when `true` — `false` never appears in
    /// any output, MCF or JSON, so a write→read round-trip is lossy for an
    /// explicit `false` specifically (it becomes indistinguishable from
    /// "never set"), by design.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub invert_actual_x: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub invert_actual_y: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub invert_commanded_x: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub invert_commanded_y: bool,
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
    /// The only representation of this concept, for files of either version.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub power_characterization: Option<PowerCharacterization>,
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
    pub recoater_blade_type: Option<String>,
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
    /// Reader-computed, in-memory only: `Some(true)`/`Some(false)` once a
    /// `MachineConfigReader` has recomputed the configuration hash and
    /// compared it to the on-disk value; `None` until then. Never set by a
    /// writer or by `MockConfigBuilder`. Unconditionally excluded from
    /// serialization (`skip`, not `skip_serializing_if` — this field, unlike
    /// `facility_id`/`config_author`, is genuinely populated on every real
    /// read, so a conditional skip would stop excluding it) — never written
    /// to HDF5, never part of the interop contract. A mismatch is never an
    /// error: the file remains fully readable either way (see `hash.rs`).
    #[serde(skip, default)]
    pub is_valid: Option<bool>,
    /// Preserves any non-typed root HDF5 attrs.
    #[serde(default)]
    pub extra: ExtraAttrs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineConfig {
    pub meta: MachineConfigMeta,
    pub machine: Machine,
    pub optical_trains: Vec<OpticalTrain>,
    /// Opaque extension trees keyed by their group name under `Extensions/`. Read and written
    /// verbatim by core, so extensions core has no model for (including private ones) survive a
    /// read-modify-write; only the matching extension library interprets them. ClearBox is one of
    /// these (`extensions["ClearBox"]`), in its File_Version 1.1 layout for either file version.
    /// Omitted from JSON when empty.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub extensions: BTreeMap<String, ExtensionTree>,
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
                invert_actual_x: false,
                invert_actual_y: false,
                invert_commanded_x: false,
                invert_commanded_y: false,
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
                power_characterization: None,
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
                is_valid: None,
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
                recoater_blade_type: Some("Standard".into()),
            },
            optical_trains: vec![sample_train()],
            extensions: Default::default(),
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
    fn extensions_are_omitted_when_empty_and_roundtrip_through_json() {
        let mut config = sample_config();
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("\"extensions\""), "extensions key must be omitted when empty");

        let mut tree = ExtensionTree::default();
        tree.attrs.insert("Flag".into(), mcf_extension_contract::Attr::scalar(1u32));
        config.extensions.insert("Private_Ext".into(), tree);
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"extensions\""));
        let roundtripped: MachineConfig = serde_json::from_str(&json).unwrap();
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
