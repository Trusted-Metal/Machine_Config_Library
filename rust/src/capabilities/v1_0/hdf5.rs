// Phase 3.5 — HDF5 reader: converts .h5 machine-config files to MachineConfig structs.
// Implemented in Phase 3.5; see IMPLEMENTATION_PLAN.md §3.5
// Reference: python/src/machine_config/reader.py
// HDF5 path mapping: see §3.10; type coercion rules: see §3.11
//
// Design note (deliberate divergence from Python): Python's single `parse()`
// always reads the ClearBox correction grids and the raw `.fc3` bytes into the
// model; `include_binary` only controls whether `to_json()` serialises them.
// The Rust reader instead controls this at parse time via two entry points —
// `parse()` (scalars/metadata only) and `parse_with_binary()` (also reads the
// binary payloads) — so callers who only need scalar fields never pay the
// cost of reading the (257, 257, 2) float64 grids or multi-megabyte byte
// arrays. `to_json()` simply picks which of the two to call.

use std::path::{Path, PathBuf};

use hdf5::types::{FixedUnicode, TypeDescriptor, VarLenAscii, VarLenUnicode};
use hdf5::{Dataset, File as H5File, Group, Location};
use indexmap::IndexMap;
use ndarray::Array3;

/// On-disk compound-dataset row for `Derivation_Equation_Constants`. Distinct
/// from the model-facing [`EquationConstant`] (which uses `String`) because
/// `String` doesn't implement `H5Type` — `FixedUnicode<64>` is the
/// HDF5-compound equivalent; converted to/from `String` at the model
/// boundary, the same way every other field already converts between its
/// on-disk and in-memory representation.
///
/// **Fixed-length, not `VarLenUnicode` — deliberate, cross-language decision.**
/// The HDF5 C library cannot convert between fixed-length and variable-length
/// strings when they're compound-type *members* (confirmed at the raw
/// `H5Tinsert`/`H5Dread` level, independent of any single binding), and
/// Node.js's h5wasm cannot write a non-empty VLEN string inside a compound
/// row at all. A 64-byte fixed-length UTF-8 string (NULLPAD-padded on disk)
/// is the one representation every language's HDF5 binding can both read and
/// write here. 64 bytes is generous relative to real usage (`a`/`b`, or
/// `c0`..`cN` for `POLYNOMIAL`) while leaving room for more descriptive
/// names. `FixedUnicode::from_str` (used at the model boundary) rejects
/// names whose UTF-8 encoding exceeds 64 bytes with `StringError`, rather
/// than silently truncating. See `SYNCHRONOUS_SENSOR_PLAN.md`'s "Compound
/// dataset string convention" section for the full cross-language
/// investigation.
#[derive(hdf5::H5Type, Clone, Debug)]
#[repr(C)]
pub(crate) struct RawEquationConstant {
    pub(crate) name: FixedUnicode<64>,
    pub(crate) value: f64,
}

/// On-disk compound-dataset row for `Calibration_Points`. All-`f64`, so there
/// is no string-conversion concern here — kept as its own type anyway, for
/// symmetry with [`RawEquationConstant`] and to keep the model layer
/// (`crate::models`) free of any HDF5-specific derive.
#[derive(hdf5::H5Type, Clone, Debug)]
#[repr(C)]
pub(crate) struct RawCalibrationPoint {
    pub(crate) input_value: f64,
    pub(crate) output_value: f64,
}

use super::layout;
use crate::error::{MachineConfigError, Result};
use crate::models::*;

const SCHEMA_VERSION: &str = "v1";

const KNOWN_ROOT_KEYS: &[&str] = &[
    "machine_name",
    "manufacturer",
    "model",
    "serial_number",
    "File_Version",
    "Export_Date",
    "Configuration_Hash",
];

const KNOWN_CLIENT_KEYS: &[&str] = &[
    "Server_URL",
    "Auth_Mode",
    "Security_Mode",
    "Security_Policy",
    "BFS_Max_Depth",
    "Publish_Interval",
    "Sampling_Interval",
    "Session_Timeout",
    "Keep_Alive_Count",
    "Lifetime_Count",
    "Machine_Profile",
    "Queue_Policy",
    "Queue_Size_Data_Change",
    "Queue_Size_Events",
    "Reconnect_Interval",
    "Root_Node",
    "Sync_Loop_Interval_Initial",
    "Sync_Loop_Interval_Settled",
];

const KNOWN_PIPE_KEYS: &[&str] = &[
    "Pipe_Enabled",
    "Buffer_Size",
    "Configure_Client",
    "Inbound_Rate_Limit",
    "Max_Inbound_Message_Size",
    "Min_Integrity_Level",
    "Pipe_Name",
    "User_Access_Level",
];

const KNOWN_TRIGGER_KEYS: &[&str] = &[
    "ID",
    "Signal",
    "Subsystem",
    "Rule_Enabled",
    "Start_Value",
    "Stop_Value",
    "Case_Sensitivity",
    "Component",
    "Cooldown_Period",
    "Event",
    "Max_Fires_Per_Job",
    "Trigger_Label",
];

// ---------------------------------------------------------------------------
// Attribute-reading helpers — all HDF5 attribute access flows through these.
// See §3.11 for the rule table this implements.
// ---------------------------------------------------------------------------

/// An attribute's value, decoded to the closest-matching Rust primitive.
/// HDF5 attributes in these files may hold a numeric value *or* an empty/
/// non-empty string even for fields the model treats as numeric (Rule 3) —
/// see `Range_Of_Motion`, which is `VarLenUnicode` when unpopulated and
/// `Float(U8)` when populated. Every read helper below dispatches on this
/// enum rather than assuming a field's HDF5 storage type in advance.
enum RawValue {
    Str(String),
    Int(i64),
    Float(f64),
}

/// Reads attribute `key` on `loc`, or `Ok(None)` if it does not exist.
/// Errors only on an HDF5 type this reader does not support (Rule 1: no
/// catch-all — an unsupported type is a loud error, not a silent `None`).
fn read_raw(loc: &Location, key: &str) -> Result<Option<RawValue>> {
    let attr = match loc.attr(key) {
        Ok(attr) => attr,
        Err(_) => return Ok(None),
    };
    let td = attr.dtype()?.to_descriptor()?;
    let value = match td {
        TypeDescriptor::VarLenUnicode => {
            RawValue::Str(attr.read_scalar::<VarLenUnicode>()?.as_str().to_string())
        }
        TypeDescriptor::VarLenAscii => {
            RawValue::Str(attr.read_scalar::<VarLenAscii>()?.as_str().to_string())
        }
        TypeDescriptor::Integer(_) | TypeDescriptor::Unsigned(_) => {
            RawValue::Int(attr.read_scalar::<i64>()?)
        }
        TypeDescriptor::Float(_) => RawValue::Float(attr.read_scalar::<f64>()?),
        other => {
            return Err(MachineConfigError::Parse(format!(
                "attribute '{key}' has unsupported HDF5 type: {other}"
            )))
        }
    };
    Ok(Some(value))
}

fn raw_to_json(value: RawValue) -> serde_json::Value {
    match value {
        RawValue::Str(s) => serde_json::Value::String(s),
        RawValue::Int(i) => serde_json::Value::from(i),
        RawValue::Float(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
    }
}

/// `None` if absent or empty after trimming; never raises (Rule 3).
fn read_str(loc: &Location, key: &str) -> Result<Option<String>> {
    match read_raw(loc, key)? {
        None => Ok(None),
        Some(RawValue::Str(s)) => {
            let trimmed = s.trim();
            Ok(if trimmed.is_empty() { None } else { Some(trimmed.to_string()) })
        }
        Some(RawValue::Int(i)) => Ok(Some(i.to_string())),
        Some(RawValue::Float(f)) => Ok(Some(f.to_string())),
    }
}

/// For fields Python reads as `str(attrs.get(key, ""))` — no trimming, `""`
/// default when absent, never raises. Distinct from `read_str`: a value of
/// `"  "` stays `"  "` here but becomes `None` via `read_str`.
fn read_required_str(loc: &Location, key: &str) -> Result<String> {
    match read_raw(loc, key)? {
        None => Ok(String::new()),
        Some(RawValue::Str(s)) => Ok(s),
        Some(RawValue::Int(i)) => Ok(i.to_string()),
        Some(RawValue::Float(f)) => Ok(f.to_string()),
    }
}

/// `None` if absent or empty string; numeric value otherwise. Raises on a
/// non-empty, non-numeric string (Rule 3). Handles fields that are
/// float-typed when populated but string-typed (`""`) when not, and fields
/// like `Power_Bit_Resolution` that are always string-typed.
fn read_float(loc: &Location, key: &str) -> Result<Option<f64>> {
    match read_raw(loc, key)? {
        None => Ok(None),
        Some(RawValue::Float(f)) => Ok(Some(f)),
        Some(RawValue::Int(i)) => Ok(Some(i as f64)),
        Some(RawValue::Str(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            trimmed.parse::<f64>().map(Some).map_err(|_| {
                MachineConfigError::Parse(format!(
                    "attribute '{key}' has non-numeric string value {trimmed:?}"
                ))
            })
        }
    }
}

/// `None` if absent or empty string; integer value otherwise (Rule 3).
fn read_int(loc: &Location, key: &str) -> Result<Option<i64>> {
    match read_raw(loc, key)? {
        None => Ok(None),
        Some(RawValue::Int(i)) => Ok(Some(i)),
        Some(RawValue::Float(f)) => Ok(Some(f as i64)),
        Some(RawValue::Str(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            trimmed.parse::<i64>().map(Some).map_err(|_| {
                MachineConfigError::Parse(format!(
                    "attribute '{key}' has non-integer string value {trimmed:?}"
                ))
            })
        }
    }
}

/// For fields Python reads as `int(attrs.get(key, default))` — default value
/// when absent, raises on anything non-numeric (including `""`) when present.
fn read_required_int(loc: &Location, key: &str, default: i64) -> Result<i64> {
    match read_int(loc, key)? {
        Some(v) => Ok(v),
        None => {
            // read_int already collapsed empty-string to None; distinguish
            // "truly absent" (use default) from "present but blank" (Python's
            // int("") would raise) by checking existence directly.
            if loc.attr(key).is_ok() {
                Err(MachineConfigError::Parse(format!(
                    "attribute '{key}' is blank; a required integer value is expected"
                )))
            } else {
                Ok(default)
            }
        }
    }
}

/// Converts HDF5 integer `0`/`1` to `bool`. `None` if absent/blank.
/// Any other integer is a hard error (Rule 4).
fn read_bool_from_int(loc: &Location, key: &str) -> Result<Option<bool>> {
    match read_int(loc, key)? {
        None => Ok(None),
        Some(0) => Ok(Some(false)),
        Some(1) => Ok(Some(true)),
        Some(other) => Err(MachineConfigError::Parse(format!(
            "attribute '{key}' has value {other}; expected 0 or 1 (Rule 8)"
        ))),
    }
}

/// For `pipe_enabled`, which Python reads as `bool(int(attrs.get(key, default)))`
/// — any non-zero value is `true`, not just `1` (unlike `read_bool_from_int`).
fn read_required_bool_from_int(loc: &Location, key: &str, default: i64) -> Result<bool> {
    Ok(read_required_int(loc, key, default)? != 0)
}

/// Converts HDF5 float `0.0`/`1.0` to `bool` (used only for `Triggers_Enabled`,
/// which is stored as float64, not int — see §3.10).
fn read_bool_from_float(loc: &Location, key: &str) -> Result<Option<bool>> {
    match read_float(loc, key)? {
        None => Ok(None),
        Some(v) if v == 0.0 => Ok(Some(false)),
        Some(v) if v == 1.0 => Ok(Some(true)),
        Some(other) => Err(MachineConfigError::Parse(format!(
            "attribute '{key}' has value {other}; expected 0.0 or 1.0"
        ))),
    }
}

/// Rule 8: reads a `_unit` attribute and asserts it matches the locked value
/// for the current schema version. `None` if absent/blank; error if present
/// but different from `expected`.
fn read_unit_locked(loc: &Location, key: &str, expected: &str) -> Result<Option<String>> {
    match read_str(loc, key)? {
        None => Ok(None),
        Some(actual) if actual == expected => Ok(Some(actual)),
        Some(actual) => Err(MachineConfigError::UnitMismatch {
            attr: key.to_string(),
            expected: expected.to_string(),
            actual,
        }),
    }
}

/// Collects every attribute on `loc` not in `known` into an order-preserving
/// map (Rule 5). Numpy/HDF5 scalar coercion happens the same way as ordinary
/// attribute reads (`read_raw`), so `extra` values keep their natural type.
fn collect_extra(loc: &Location, known: &[&str]) -> Result<ExtraAttrs> {
    let mut out = IndexMap::new();
    for name in loc.attr_names()? {
        if known.contains(&name.as_str()) {
            continue;
        }
        if let Some(raw) = read_raw(loc, &name)? {
            out.insert(name, raw_to_json(raw));
        }
    }
    Ok(out)
}

/// Opens a child group, converting a missing-group error into the dedicated
/// `MissingGroup` variant (Rule 1: required groups raise a clear error).
fn require_group(parent: &Group, path: &str) -> Result<Group> {
    parent
        .group(path)
        .map_err(|_| MachineConfigError::MissingGroup(path.to_string()))
}

/// Converts a `(257, 257, 2)` float64 grid to nested lists, mapping IEEE 754
/// NaN cells to `None` (Rule: NaN in float64 dataset → JSON `null`).
fn nan_array3_to_nested(arr: &Array3<f64>) -> Vec<Vec<Vec<Option<f64>>>> {
    let shape = arr.shape();
    let (d0, d1, d2) = (shape[0], shape[1], shape[2]);
    (0..d0)
        .map(|i| {
            (0..d1)
                .map(|j| {
                    (0..d2)
                        .map(|k| {
                            let v = arr[[i, j, k]];
                            if v.is_nan() {
                                None
                            } else {
                                Some(v)
                            }
                        })
                        .collect()
                })
                .collect()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// File_Version 1.0 HDF5 adapter
// ---------------------------------------------------------------------------

/// File_Version 1.0 HDF5 reader. Public [`crate::reader::MachineConfigReader`] dispatches here.
pub struct Hdf5AdapterV1_0 {
    path: PathBuf,
}

/// Read only the root `File_Version` attribute. Does not walk groups.
pub fn peek_file_version<P: AsRef<Path>>(path: P) -> Result<String> {
    let f = H5File::open(path)?;
    let version = read_required_str(&f, "File_Version").unwrap_or_default();
    let v = version.trim();
    if v.is_empty() {
        Ok("1.0".into())
    } else {
        Ok(v.to_string())
    }
}

impl Hdf5AdapterV1_0 {
    /// Opens the file to validate it exists and is a readable HDF5 file.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        H5File::open(&path)?;
        Ok(Self { path })
    }

    pub fn parse(&self) -> Result<MachineConfig> {
        let f = H5File::open(&self.path)?;
        self.parse_inner(&f, false)
    }

    pub fn parse_with_binary(&self) -> Result<MachineConfig> {
        let f = H5File::open(&self.path)?;
        self.parse_inner(&f, true)
    }

    /// Returns the `(257, 257, 2)` float64 correction grid for optical train
    /// `train_index` (0-indexed) as a version-agnostic [`CorrectionData`].
    ///
    /// Data is row-major; NaN cells (outside the scan-field boundary) are
    /// preserved as IEEE 754 NaN. See [`CorrectionData`] for indexing details
    /// and instructions on reconstructing an `ndarray::Array3` if needed.
    pub fn get_correction_data(&self, train_index: usize) -> Result<CorrectionData> {
        let f = H5File::open(&self.path)?;
        let path = layout::correction_data_path(train_index);
        let arr = f.dataset(&path)?.read::<f64, ndarray::Ix3>()?;
        let shape = [arr.shape()[0], arr.shape()[1], arr.shape()[2]];
        Ok(CorrectionData { data: arr.into_raw_vec_and_offset().0, shape })
    }

    /// Returns the `(257, 257, 2)` float64 *inverse* correction grid for
    /// optical train `train_index` (0-indexed) as a version-agnostic
    /// [`CorrectionData`].
    ///
    /// Data is row-major; NaN cells are preserved as IEEE 754 NaN.
    /// See [`CorrectionData`] for indexing details.
    pub fn get_inverse_correction_data(&self, train_index: usize) -> Result<CorrectionData> {
        let f = H5File::open(&self.path)?;
        let path = layout::inverse_correction_data_path(train_index);
        let arr = f.dataset(&path)?.read::<f64, ndarray::Ix3>()?;
        let shape = [arr.shape()[0], arr.shape()[1], arr.shape()[2]];
        Ok(CorrectionData { data: arr.into_raw_vec_and_offset().0, shape })
    }

    /// Returns the raw `.fc3` bytes embedded as a `uint8` dataset for optical
    /// train `train_index` (0-indexed).
    pub fn get_scan_field_correction_bytes(&self, train_index: usize) -> Result<Vec<u8>> {
        let f = H5File::open(&self.path)?;
        let path = layout::scan_field_correction_file_path(train_index);
        Ok(f.dataset(&path)?.read_raw::<u8>()?)
    }

    /// Returns all attributes of an arbitrary HDF5 group as a plain map.
    /// Returns an empty map (not an error) if the path does not exist.
    /// Use for non-schema groups: `OPCUA`, `Scanner/X_Axis`, etc. (Rule 5).
    pub fn get_raw_group(&self, hdf5_path: &str) -> Result<ExtraAttrs> {
        let f = H5File::open(&self.path)?;
        let grp = match f.group(hdf5_path) {
            Ok(g) => g,
            Err(_) => return Ok(IndexMap::new()),
        };
        collect_extra(&grp, &[])
    }

    /// Parses and returns the canonical JSON representation.
    ///
    /// `include_binary` controls whether `correction_data`,
    /// `inverse_correction_data`, and `raw_bytes` are included — the HDF5
    /// file remains the source of truth for those; use
    /// [`Self::get_correction_data`] and [`Self::get_scan_field_correction_bytes`]
    /// for direct numeric access instead.
    pub fn to_json(&self, pretty: bool, include_binary: bool) -> Result<String> {
        let config = if include_binary { self.parse_with_binary()? } else { self.parse()? };
        if pretty {
            Ok(serde_json::to_string_pretty(&config)?)
        } else {
            Ok(serde_json::to_string(&config)?)
        }
    }

    // ------------------------------------------------------------------
    // Private parse helpers
    // ------------------------------------------------------------------

    fn parse_inner(&self, f: &H5File, include_binary: bool) -> Result<MachineConfig> {
        let meta = MachineConfigMeta {
            schema_version: SCHEMA_VERSION.to_string(),
            machine_name: read_required_str(f, "machine_name")?,
            manufacturer: read_required_str(f, "manufacturer")?,
            model: read_required_str(f, "model")?,
            serial_number: read_required_str(f, "serial_number")?,
            file_version: read_required_str(f, "File_Version")?,
            export_date: read_required_str(f, "Export_Date")?,
            configuration_hash: read_required_str(f, "Configuration_Hash")?,
            facility_id: None,
            config_author: None,
            extra: collect_extra(f, KNOWN_ROOT_KEYS)?,
        };

        let m = require_group(f, layout::ROOT_MACHINE)?;
        let machine = Machine {
            id: read_str(&m, "ID")?,
            machine_name: read_required_str(&m, "Machine_Name")?,
            manufacturer: read_required_str(&m, "Manufacturer")?,
            model: read_required_str(&m, "Model")?,
            serial_number: read_required_str(&m, "Serial_Number")?,
            build_plate_x: read_float(&m, "Build_Plate_X_Dimension")?,
            build_plate_x_unit: read_unit_locked(&m, "Build_Plate_X_Dimension_unit", "mm")?,
            build_plate_y: read_float(&m, "Build_Plate_Y_Dimension")?,
            build_plate_y_unit: read_unit_locked(&m, "Build_Plate_Y_Dimension_unit", "mm")?,
            build_plate_z: read_float(&m, "Build_Plate_Z_Dimension")?,
            build_plate_z_unit: read_unit_locked(&m, "Build_Plate_Z_Dimension_unit", "mm")?,
            build_plate_radius: read_float(&m, "Build_Plate_Corner_Radius")?,
            build_plate_radius_unit: read_unit_locked(
                &m,
                "Build_Plate_Corner_Radius_unit",
                "mm",
            )?,
            gas_flow_direction: read_str(&m, "Gas_Flow_Direction")?,
            recoat_direction: read_str(&m, "Recoat_Direction")?,
        };

        let trains_grp = require_group(&m, "Optical_Trains")?;
        let mut train_ids: Vec<String> = trains_grp
            .member_names()?
            .into_iter()
            .filter(|k| k.starts_with(layout::TRAIN_ID_PREFIX))
            .collect();
        train_ids.sort();
        let optical_trains = train_ids
            .iter()
            .map(|tid| self.parse_train(f, tid, include_binary))
            .collect::<Result<Vec<_>>>()?;

        let opcua = self.parse_opcua(f)?;

        Ok(MachineConfig { meta, machine, optical_trains, opcua })
    }

    fn parse_train(&self, f: &H5File, train_id: &str, include_binary: bool) -> Result<OpticalTrain> {
        let base = layout::train_path_by_id(train_id);
        let a = require_group(f, &base)?;

        let scanner = self.parse_scanner(&require_group(&a, "Scanner")?)?;
        let light_source = self.parse_light_source(&require_group(&a, "Light_Source")?)?;
        let collimator = self.parse_collimator(&require_group(&a, "Collimator")?)?;
        let scanner_card = self.parse_scanner_card(&require_group(&a, "Scanner_Card")?)?;

        let clearbox = match a.group("Optional_Components/ClearBox") {
            Ok(grp) => Some(self.parse_clearbox(&grp, include_binary)?),
            Err(_) => None,
        };

        let sfcf = match a.dataset("scan_field_correction_file") {
            Ok(ds) => Some(self.parse_sfcf(&ds, include_binary)?),
            Err(_) => None,
        };

        Ok(OpticalTrain {
            train_id: train_id.to_string(),
            id: read_str(&a, "ID")?,
            beam_profile_type: read_str(&a, "Beam_Profile_Type")?,
            beam_waist_definition: read_str(&a, "Beam_Waist_Definition")?,
            beam_waist_major: read_float(&a, "Beam_Waist_Major")?,
            beam_waist_major_unit: read_unit_locked(&a, "Beam_Waist_Major_unit", "\u{03bc}m")?,
            beam_waist_minor: read_float(&a, "Beam_Waist_Minor")?,
            beam_waist_minor_unit: read_unit_locked(&a, "Beam_Waist_Minor_unit", "\u{03bc}m")?,
            beam_waist_offset_z: read_float(&a, "Beam_Waist_Offset_Z")?,
            beam_waist_offset_z_unit: read_unit_locked(&a, "Beam_Waist_Offset_Z_unit", "mm")?,
            build_plane_offset_major: read_float(&a, "Build_Plane_Offset_Major")?,
            build_plane_offset_major_unit: read_unit_locked(
                &a,
                "Build_Plane_Offset_Major_unit",
                "mm",
            )?,
            build_plane_offset_minor: read_float(&a, "Build_Plane_Offset_Minor")?,
            build_plane_offset_minor_unit: read_unit_locked(
                &a,
                "Build_Plane_Offset_Minor_unit",
                "mm",
            )?,
            collimator_focal_length: read_float(&a, "Collimator_Focal_Length")?,
            collimator_focal_length_unit: read_unit_locked(
                &a,
                "Collimator_Focal_Length_unit",
                "mm",
            )?,
            m2_major: read_float(&a, "M2_Major")?,
            m2_minor: read_float(&a, "M2_Minor")?,
            major_axis_angle: read_float(&a, "Major_Axis_Angle")?,
            major_axis_angle_unit: read_unit_locked(&a, "Major_Axis_Angle_unit", "degrees")?,
            rayleigh_length_major: read_float(&a, "Rayleigh_Length_Major")?,
            rayleigh_length_major_unit: read_unit_locked(&a, "Rayleigh_Length_Major_unit", "mm")?,
            rayleigh_length_minor: read_float(&a, "Rayleigh_Length_Minor")?,
            rayleigh_length_minor_unit: read_unit_locked(&a, "Rayleigh_Length_Minor_unit", "mm")?,
            scanner_number: read_str(&a, "Scanner_Number")?,
            thermal_lensing_passed: read_bool_from_int(&a, "Thermal_Lensing_Test_Passed")?,
            thermal_lensing_focal_plane_shift: read_float(&a, "Thermal_Lensing_Focal_Plane_Shift")?,
            thermal_lensing_focal_plane_shift_unit: read_unit_locked(
                &a,
                "Thermal_Lensing_Focal_Plane_Shift_unit",
                "mm",
            )?,
            thermal_lensing_threshold: read_float(&a, "Thermal_Lensing_Threshold")?,
            thermal_lensing_threshold_unit: read_unit_locked(
                &a,
                "Thermal_Lensing_Threshold_unit",
                "mm",
            )?,
            scanner,
            light_source,
            collimator,
            scanner_card,
            optional_components: crate::models::OptionalComponents { clearbox },
            scan_field_correction_file: sfcf,
        })
    }

    fn parse_axis(&self, grp: &Group) -> Result<AxisConfig> {
        Ok(AxisConfig {
            actual_bit_resolution: read_int(grp, "Actual_Bit_Resolution")?,
            actual_bit_resolution_unit: read_str(grp, "Actual_Bit_Resolution_unit")?,
            commanded_bit_resolution: read_int(grp, "Commanded_Bit_Resolution")?,
            commanded_bit_resolution_unit: read_str(grp, "Commanded_Bit_Resolution_unit")?,
            control_type: read_str(grp, "Control_Type")?,
            range_of_motion: read_float(grp, "Range_Of_Motion")?,
            range_of_motion_unit: read_str(grp, "Range_Of_Motion_unit")?,
            smoothing_kernel: read_str(grp, "Smoothing_Kernel")?,
            smoothing_parameters: read_float(grp, "Smoothing_Parameters")?,
            tuning_parameters: read_str(grp, "Tuning_Parameters")?,
            tuning_type: read_str(grp, "Tuning_Type")?,
        })
    }

    fn parse_scanner(&self, grp: &Group) -> Result<Scanner> {
        let axis_configuration = read_str(grp, "Axis_Configuration")?;
        let x_axis = self.parse_axis(&require_group(grp, "X_Axis")?)?;
        let y_axis = self.parse_axis(&require_group(grp, "Y_Axis")?)?;
        let z_axis = match grp.group("Z_Axis") {
            Ok(g) => Some(self.parse_axis(&g)?),
            Err(_) => None,
        };
        let focus = match grp.group("Focus") {
            Ok(g) => Some(self.parse_axis(&g)?),
            Err(_) => None,
        };
        Ok(Scanner {
            manufacturer: read_required_str(grp, "Manufacturer")?,
            model: read_required_str(grp, "Model")?,
            serial_number: read_str(grp, "Serial_Number")?.unwrap_or_default(),
            working_distance: read_float(grp, "Working_Distance")?,
            working_distance_unit: read_unit_locked(grp, "Working_Distance_unit", "mm")?,
            scan_field_x: read_float(grp, "Scan_Field_Size_X")?,
            scan_field_x_unit: read_unit_locked(grp, "Scan_Field_Size_X_unit", "mm")?,
            scan_field_y: read_float(grp, "Scan_Field_Size_Y")?,
            scan_field_y_unit: read_unit_locked(grp, "Scan_Field_Size_Y_unit", "mm")?,
            scan_field_z: read_float(grp, "Scan_Field_Size_Z")?,
            scan_field_z_unit: read_unit_locked(grp, "Scan_Field_Size_Z_unit", "mm")?,
            scan_head_offset_x: read_float(grp, "Scan_Head_Offset_X")?,
            scan_head_offset_x_unit: read_unit_locked(grp, "Scan_Head_Offset_X_unit", "mm")?,
            scan_head_offset_y: read_float(grp, "Scan_Head_Offset_Y")?,
            scan_head_offset_y_unit: read_unit_locked(grp, "Scan_Head_Offset_Y_unit", "mm")?,
            scan_head_offset_z: read_float(grp, "Scan_Head_Offset_Z")?,
            scan_head_offset_z_unit: read_unit_locked(grp, "Scan_Head_Offset_Z_unit", "mm")?,
            scan_head_rotation: read_float(grp, "Scan_Head_Rotation")?,
            scan_head_rotation_unit: read_unit_locked(grp, "Scan_Head_Rotation_unit", "degrees")?,
            axis_configuration,
            x_axis,
            y_axis,
            z_axis,
            focus,
        })
    }

    fn parse_light_source(&self, grp: &Group) -> Result<LightSource> {
        Ok(LightSource {
            manufacturer: read_required_str(grp, "Manufacturer")?,
            model: read_required_str(grp, "Model")?,
            serial_number: read_required_str(grp, "Serial_Number")?,
            wavelength: read_float(grp, "Light_Wavelength")?,
            wavelength_unit: read_unit_locked(grp, "Light_Wavelength_unit", "nm")?,
            power_max_nominal: read_float(grp, "Power_Max_Nominal")?,
            power_max_nominal_unit: read_unit_locked(grp, "Power_Max_Nominal_unit", "W")?,
            power_max_actual: read_float(grp, "Power_Max_Actual")?,
            power_max_actual_unit: read_unit_locked(grp, "Power_Max_Actual_unit", "W")?,
            power_min_actual: read_float(grp, "Power_Min_Actual")?,
            power_min_actual_unit: read_unit_locked(grp, "Power_Min_Actual_unit", "W")?,
            power_min_nominal: read_float(grp, "Power_Min_Nominal")?,
            power_min_nominal_unit: read_unit_locked(grp, "Power_Min_Nominal_unit", "W")?,
            power_bit_resolution: read_float(grp, "Power_Bit_Resolution")?,
            power_bit_resolution_unit: read_unit_locked(
                grp,
                "Power_Bit_Resolution_unit",
                "bits",
            )?,
            watts_to_volts_algorithm: read_str(grp, "Watts_To_Volts_Algorithm")?,
            watts_to_volts_params: read_str(grp, "Watts_To_Volts_Params")?,
        })
    }

    fn parse_collimator(&self, grp: &Group) -> Result<Collimator> {
        Ok(Collimator {
            manufacturer: read_required_str(grp, "Manufacturer")?,
            model: read_required_str(grp, "Model")?,
            serial_number: read_required_str(grp, "Serial_Number")?,
            focal_length: read_float(grp, "Focal_Length")?,
            focal_length_unit: read_unit_locked(grp, "Focal_Length_unit", "mm")?,
        })
    }

    fn parse_scanner_card(&self, grp: &Group) -> Result<ScannerCard> {
        Ok(ScannerCard {
            manufacturer: read_required_str(grp, "Manufacturer")?,
            model: read_required_str(grp, "Model")?,
            serial_number: read_required_str(grp, "Serial_Number")?,
            communication_protocol: read_str(grp, "Communication_Protocol")?,
            sample_period: read_float(grp, "Sample_Period")?,
            sample_period_unit: read_unit_locked(grp, "Sample_Period_unit", "\u{03bc}s")?,
        })
    }

    fn parse_clearbox(&self, grp: &Group, include_binary: bool) -> Result<ClearBox> {
        let (correction_data, inverse_correction_data) = if include_binary {
            let cd = grp.dataset("Correction_Data")?.read::<f64, ndarray::Ix3>()?;
            let icd = grp.dataset("Inverse_Correction_Data")?.read::<f64, ndarray::Ix3>()?;
            (Some(nan_array3_to_nested(&cd)), Some(nan_array3_to_nested(&icd)))
        } else {
            (None, None)
        };
        // Synchronous_Sensors: absent entirely (e.g. today's plain
        // reference_config.h5) and present-but-empty are the same state —
        // an empty IndexMap, not a separate "absent" marker. member_names()
        // only ever enumerates sensor sub-groups here since nothing else is
        // ever placed directly under Synchronous_Sensors itself.
        let synchronous_sensors = match grp.group("Synchronous_Sensors") {
            Ok(sensors_grp) => {
                let mut map = IndexMap::new();
                for name in sensors_grp.member_names()? {
                    let sg = sensors_grp.group(&name)?;
                    map.insert(name, self.parse_synchronous_sensor(&sg)?);
                }
                map
            }
            Err(_) => IndexMap::new(),
        };
        Ok(ClearBox {
            ip_address: read_required_str(grp, "Ip_Address")?,
            serial_number: read_str(grp, "Serial_Number")?,
            data_port: read_int(grp, "Data_Port")?,
            server_port: read_int(grp, "Server_Port")?,
            actual_timing_offset: read_int(grp, "Actual_Timing_Offset")?,
            commanded_timing_offset: read_int(grp, "Commanded_Timing_Offset")?,
            correction_data,
            inverse_correction_data,
            manufacturer: read_str(grp, "Manufacturer")?,
            model: read_str(grp, "Model")?,
            output_path: read_str(grp, "Output_Path")?,
            selected_camera: read_str(grp, "Selected_Camera")?,
            custom_video_format: read_str(grp, "Custom_Video_Format")?,
            video_output: read_str(grp, "Video_Output")?,
            show_console: read_bool_from_int(grp, "Show_Console")?,
            software_trigger_delay: read_int(grp, "Software_Trigger_Delay")?,
            volts_to_watts_algorithm: read_str(grp, "Volts_To_Watts_Algorithm")?,
            volts_to_watts_params: read_str(grp, "Volts_To_Watts_Params")?,
            correction_grid_domain_shape: read_str(grp, "Correction_Grid_Domain_Shape")?,
            inverse_grid_domain_shape: read_str(grp, "Inverse_Grid_Domain_Shape")?,
            synchronous_sensors,
        })
    }

    /// Parses one `ClearBox/Synchronous_Sensors/<key>/` sub-group. Both
    /// compound datasets default to an empty `Vec` if the dataset itself is
    /// absent — the same "never panic on missing optional data" discipline
    /// used everywhere else in this reader, extended to datasets, not just
    /// attributes.
    fn parse_synchronous_sensor(&self, grp: &Group) -> Result<SynchronousSensor> {
        let derivation_equation_constants = match grp.dataset("Derivation_Equation_Constants") {
            Ok(ds) => ds
                .read_raw::<RawEquationConstant>()?
                .into_iter()
                .map(|r| EquationConstant { name: r.name.as_str().to_string(), value: r.value })
                .collect(),
            Err(_) => Vec::new(),
        };
        let calibration_points = match grp.dataset("Calibration_Points") {
            Ok(ds) => ds
                .read_raw::<RawCalibrationPoint>()?
                .into_iter()
                .map(|r| CalibrationPoint { input_value: r.input_value, output_value: r.output_value })
                .collect(),
            Err(_) => Vec::new(),
        };
        Ok(SynchronousSensor {
            enabled: read_bool_from_int(grp, "Enabled")?,
            sensor_name: read_str(grp, "Sensor_Name")?,
            sensor_output_range_low: read_float(grp, "Sensor_Output_Range_Low")?,
            sensor_output_range_high: read_float(grp, "Sensor_Output_Range_High")?,
            sensor_output_space: read_str(grp, "Sensor_Output_Space")?,
            sensor_model: read_str(grp, "Sensor_Model")?,
            sensor_manufacturer: read_str(grp, "Sensor_Manufacturer")?,
            sensor_scope: read_str(grp, "Sensor_Scope")?,
            units_derived_quantity: read_str(grp, "Units_Derived_Quantity")?,
            port_id: read_int(grp, "Port_ID")?,
            sensor_type: read_str(grp, "Sensor_Type")?,
            input_type: read_str(grp, "Input_Type")?,
            algorithm_type: read_str(grp, "Algorithm_Type")?,
            algorithm_equation: read_str(grp, "Algorithm_Equation")?,
            calibration_source: read_str(grp, "Calibration_Source")?,
            calibration_verified: read_bool_from_int(grp, "Calibration_Verified")?,
            sample_period: read_float(grp, "Sample_Period")?,
            metadata: read_str(grp, "Metadata")?,
            derivation_equation_constants,
            calibration_points,
        })
    }

    fn parse_sfcf(&self, ds: &Dataset, include_binary: bool) -> Result<ScanFieldCorrectionFile> {
        let raw_bytes = if include_binary { Some(ds.read_raw::<u8>()?) } else { None };
        Ok(ScanFieldCorrectionFile {
            document_name: read_required_str(ds, "document_name")?,
            document_id: read_required_str(ds, "document_id")?,
            file_size: read_required_int(ds, "file_size", 0)?,
            valid_as_of_date: read_required_str(ds, "valid_as_of_date")?,
            document_created_at: read_str(ds, "document_created_at")?,
            document_type: read_str(ds, "document_type")?,
            original_uri: read_str(ds, "original_uri")?,
            raw_bytes,
        })
    }

    /// Parses the optional `OPCUA` group tree. Returns `Ok(None)` if the file
    /// has no `OPCUA` group (most machine configs).
    fn parse_opcua(&self, f: &H5File) -> Result<Option<OpcuaConfig>> {
        let opcua_grp = match f.group(layout::ROOT_OPCUA) {
            Ok(g) => g,
            Err(_) => return Ok(None),
        };

        let client_grp = opcua_grp.group("Client")?;
        let client = OpcuaClientConfig {
            server_url: read_required_str(&client_grp, "Server_URL")?,
            auth_mode: read_required_str(&client_grp, "Auth_Mode")?,
            security_mode: read_required_str(&client_grp, "Security_Mode")?,
            security_policy: read_required_str(&client_grp, "Security_Policy")?,
            bfs_max_depth: read_required_int(&client_grp, "BFS_Max_Depth", 0)?,
            publish_interval: read_required_int(&client_grp, "Publish_Interval", 0)?,
            sampling_interval: read_required_int(&client_grp, "Sampling_Interval", 0)?,
            session_timeout: read_required_int(&client_grp, "Session_Timeout", 0)?,
            keep_alive_count: read_int(&client_grp, "Keep_Alive_Count")?,
            lifetime_count: read_int(&client_grp, "Lifetime_Count")?,
            machine_profile: read_str(&client_grp, "Machine_Profile")?,
            queue_policy: read_str(&client_grp, "Queue_Policy")?,
            queue_size_data_change: read_int(&client_grp, "Queue_Size_Data_Change")?,
            queue_size_events: read_int(&client_grp, "Queue_Size_Events")?,
            reconnect_interval: read_int(&client_grp, "Reconnect_Interval")?,
            root_node: read_str(&client_grp, "Root_Node")?,
            sync_loop_interval_initial: read_int(&client_grp, "Sync_Loop_Interval_Initial")?,
            sync_loop_interval_settled: read_int(&client_grp, "Sync_Loop_Interval_Settled")?,
            extra: collect_extra(&client_grp, KNOWN_CLIENT_KEYS)?,
        };

        let pipe_grp = opcua_grp.group("Pipe")?;
        let pipe = OpcuaPipeConfig {
            pipe_enabled: read_required_bool_from_int(&pipe_grp, "Pipe_Enabled", 0)?,
            buffer_size: read_required_int(&pipe_grp, "Buffer_Size", 0)?,
            configure_client: read_bool_from_int(&pipe_grp, "Configure_Client")?,
            inbound_rate_limit: read_int(&pipe_grp, "Inbound_Rate_Limit")?,
            max_inbound_message_size: read_int(&pipe_grp, "Max_Inbound_Message_Size")?,
            min_integrity_level: read_str(&pipe_grp, "Min_Integrity_Level")?,
            pipe_name: read_str(&pipe_grp, "Pipe_Name")?,
            user_access_level: read_str(&pipe_grp, "User_Access_Level")?,
            extra: collect_extra(&pipe_grp, KNOWN_PIPE_KEYS)?,
        };

        let triggers_grp = opcua_grp.group("Triggers")?;
        let triggers_enabled = read_bool_from_float(&triggers_grp, "Triggers_Enabled")?;
        let trigger_stop_ceiling_layers = read_int(&triggers_grp, "Trigger_Stop_Ceiling_Layers")?;

        let mut triggers = IndexMap::new();
        for name in triggers_grp.member_names()? {
            let tg = triggers_grp.group(&name)?;
            let extra = collect_extra(&tg, KNOWN_TRIGGER_KEYS)?;
            triggers.insert(
                name,
                OpcuaTrigger {
                    id: read_str(&tg, "ID")?,
                    signal: read_str(&tg, "Signal")?,
                    subsystem: read_str(&tg, "Subsystem")?,
                    rule_enabled: read_bool_from_int(&tg, "Rule_Enabled")?,
                    start_value: read_str(&tg, "Start_Value")?,
                    stop_value: read_str(&tg, "Stop_Value")?,
                    case_sensitivity: read_str(&tg, "Case_Sensitivity")?,
                    component: read_str(&tg, "Component")?,
                    cooldown_period: read_int(&tg, "Cooldown_Period")?,
                    event: read_str(&tg, "Event")?,
                    max_fires_per_job: read_int(&tg, "Max_Fires_Per_Job")?,
                    trigger_label: read_str(&tg, "Trigger_Label")?,
                    extra,
                },
            );
        }

        Ok(Some(OpcuaConfig {
            client,
            pipe,
            triggers,
            triggers_enabled,
            trigger_stop_ceiling_layers,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REFERENCE: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config.h5");
    const REFERENCE_OPCUA: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config_opcua.h5");
    const OPCUA_MISSING_REQUIRED: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../docs/validation/fixtures/opcua_missing_required.h5"
    );
    const SYNTHETIC: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/synthetic_2laser.h5");
    const REFERENCE_SENSORS: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/reference_config_synchronous_sensors.h5"
    );
    const REFERENCE_OPCUA_SENSORS: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/reference_config_opcua_synchronous_sensors.h5"
    );

    #[test]
    fn open_rejects_nonexistent_path() {
        assert!(Hdf5AdapterV1_0::open("does/not/exist.h5").is_err());
    }

    #[test]
    fn parse_reference_meta_and_machine() {
        let config = Hdf5AdapterV1_0::open(REFERENCE).unwrap().parse().unwrap();
        assert_eq!(config.meta.machine_name, "TM-LPBF-02: AconityMIDI+_OG");
        assert_eq!(config.meta.manufacturer, "Aconity3D");
        assert_eq!(config.meta.file_version, "1.0");
        assert_eq!(config.meta.configuration_hash.len(), 64);
        assert_eq!(config.meta.schema_version, "v1");
        assert_eq!(
            config.meta.extra.get("Description").and_then(|v| v.as_str()),
            Some("Machine Configuration Export from Service Observations")
        );

        assert_eq!(config.machine.build_plate_x, Some(250.0));
        assert_eq!(config.machine.build_plate_x_unit, Some("mm".to_string()));
        assert_eq!(config.machine.build_plate_z, Some(20.0));
        assert_eq!(config.machine.gas_flow_direction, Some("Y+".to_string()));
    }

    #[test]
    fn parse_reference_optical_trains() {
        let config = Hdf5AdapterV1_0::open(REFERENCE).unwrap().parse().unwrap();
        assert_eq!(config.optical_trains.len(), 2);

        let t1 = &config.optical_trains[0];
        assert_eq!(t1.train_id, "Optical_Train_01");
        assert_eq!(t1.scanner.working_distance, Some(670.0));
        assert_eq!(t1.scanner.working_distance_unit, Some("mm".to_string()));
        assert_eq!(t1.scanner.scan_head_offset_x, Some(-87.5));
        assert_eq!(t1.scanner.axis_configuration, Some("3D".to_string()));
        assert_eq!(t1.thermal_lensing_passed, Some(false));
        // X_Axis is always present; Focus is absent for a "3D" (not "3D+Focus") config.
        assert_eq!(t1.scanner.x_axis.smoothing_kernel, Some("GAUSSIAN".to_string()));
        assert!(t1.scanner.z_axis.is_some());
        assert!(t1.scanner.focus.is_none());
        // Range_Of_Motion is unpopulated in the fixture and stored as an empty
        // string rather than a float — read_float must still yield None, not error.
        assert_eq!(t1.scanner.x_axis.range_of_motion, None);

        let t2 = &config.optical_trains[1];
        assert_eq!(t2.train_id, "Optical_Train_02");
        assert_eq!(t2.thermal_lensing_passed, Some(true));
        assert_eq!(t2.scanner.scan_head_rotation, Some(180.0));
    }

    #[test]
    fn parse_reference_clearbox_and_sfcf_metadata_without_binary() {
        let config = Hdf5AdapterV1_0::open(REFERENCE).unwrap().parse().unwrap();
        let t1 = &config.optical_trains[0];

        let cb = t1.optional_components.clearbox.as_ref().expect("clearbox present on reference fixture");
        assert!(!cb.ip_address.is_empty());
        assert!(cb.correction_data.is_none(), "parse() must not read binary correction grids");
        assert!(cb.inverse_correction_data.is_none());

        let sfcf = t1
            .scan_field_correction_file
            .as_ref()
            .expect("scan field correction file present");
        assert_eq!(sfcf.file_size, 1138799);
        assert!(sfcf.raw_bytes.is_none(), "parse() must not read raw .fc3 bytes");
    }

    #[test]
    fn reference_fixture_has_no_synchronous_sensors() {
        // reference_config.h5 deliberately has no Synchronous_Sensors group
        // (SYNCHRONOUS_SENSOR_PLAN.md Phase 0) — must read back as an empty
        // map, not an error, not a panic.
        let config = Hdf5AdapterV1_0::open(REFERENCE).unwrap().parse().unwrap();
        let cb = config.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
        assert!(cb.synchronous_sensors.is_empty());
    }

    /// Every one of the 18 scalar fields plus both compound datasets, checked
    /// against the real ZR800 example values in
    /// `reference_config_synchronous_sensors.h5` — verified directly via
    /// h5py before writing this test (SYNCHRONOUS_SENSOR_PLAN.md Phase 0).
    #[test]
    fn synchronous_sensor_fixture_has_real_values() {
        let config = Hdf5AdapterV1_0::open(REFERENCE_SENSORS).unwrap().parse().unwrap();
        let cb = config.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
        assert_eq!(cb.synchronous_sensors.len(), 1);
        let sensor = &cb.synchronous_sensors["Oxygen Sensor"];

        assert_eq!(sensor.enabled, Some(true));
        assert_eq!(sensor.sensor_name, Some("ZR800 Oxygen Analyzer".to_string()));
        assert_eq!(sensor.sensor_output_range_low, Some(-1.0));
        assert_eq!(sensor.sensor_output_range_high, Some(6.0));
        assert_eq!(sensor.sensor_output_space, Some("log10(ppm)".to_string()));
        assert_eq!(sensor.sensor_model, Some("ZR810".to_string()));
        assert_eq!(sensor.sensor_manufacturer, Some("Industrial Physics".to_string()));
        assert_eq!(sensor.sensor_scope, Some("Global".to_string()));
        assert_eq!(sensor.units_derived_quantity, Some("ppm".to_string()));
        assert_eq!(sensor.port_id, Some(5));
        assert_eq!(sensor.sensor_type, Some("Oxygen Sensor".to_string()));
        assert_eq!(sensor.input_type, Some("4-20 mA".to_string()));
        assert_eq!(sensor.algorithm_type, Some("Log-Linear".to_string()));
        assert_eq!(sensor.algorithm_equation, Some("log(ppm) = a*mA + b".to_string()));
        assert_eq!(sensor.calibration_source, Some("Datasheet".to_string()));
        assert_eq!(sensor.calibration_verified, Some(false));
        assert_eq!(sensor.sample_period, Some(5.0));
        assert!(sensor.metadata.as_deref().unwrap_or("").contains("100KHz"));

        // Compound datasets — exact values, in on-disk row order.
        assert_eq!(sensor.derivation_equation_constants.len(), 2);
        assert_eq!(sensor.derivation_equation_constants[0].name, "a");
        assert_eq!(sensor.derivation_equation_constants[0].value, 0.4375);
        assert_eq!(sensor.derivation_equation_constants[1].name, "b");
        assert_eq!(sensor.derivation_equation_constants[1].value, -2.75);

        assert_eq!(sensor.calibration_points.len(), 2);
        assert_eq!(sensor.calibration_points[0].input_value, 4.0);
        assert_eq!(sensor.calibration_points[0].output_value, -1.0);
        assert_eq!(sensor.calibration_points[1].input_value, 20.0);
        assert_eq!(sensor.calibration_points[1].output_value, 6.0);

        // Confirms the calibration points are in Sensor_Output_Space
        // (log-space), not Units_Derived_Quantity (linear ppm) — the exact
        // proof from SYNCHRONOUS_SENSOR_PLAN.md's unit-convention discussion.
        let a = sensor.derivation_equation_constants[0].value;
        let b = sensor.derivation_equation_constants[1].value;
        for point in &sensor.calibration_points {
            assert!((a * point.input_value + b - point.output_value).abs() < 1e-9);
        }
    }

    #[test]
    fn combined_opcua_and_synchronous_sensors_fixture_has_both() {
        let config = Hdf5AdapterV1_0::open(REFERENCE_OPCUA_SENSORS).unwrap().parse().unwrap();

        let opcua = config.opcua.as_ref().expect("OPCUA present on the combined fixture");
        assert!(!opcua.client.server_url.is_empty());

        let cb = config.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
        assert_eq!(cb.synchronous_sensors.len(), 1);
        assert_eq!(
            cb.synchronous_sensors["Oxygen Sensor"].sensor_name,
            Some("ZR800 Oxygen Analyzer".to_string())
        );
    }

    #[test]
    fn parse_with_binary_populates_correction_data_and_raw_bytes() {
        let config = Hdf5AdapterV1_0::open(REFERENCE).unwrap().parse_with_binary().unwrap();
        let t1 = &config.optical_trains[0];

        let cb = t1.optional_components.clearbox.as_ref().unwrap();
        let data = cb.correction_data.as_ref().expect("correction_data populated");
        assert_eq!(data.len(), 257);
        assert_eq!(data[0].len(), 257);
        assert_eq!(data[0][0].len(), 2);

        let sfcf = t1.scan_field_correction_file.as_ref().unwrap();
        let raw = sfcf.raw_bytes.as_ref().expect("raw_bytes populated");
        assert_eq!(raw.len(), sfcf.file_size as usize);
    }

    #[test]
    fn get_correction_data_shape_and_nan_present() {
        let reader = Hdf5AdapterV1_0::open(REFERENCE).unwrap();
        let cd = reader.get_correction_data(0).unwrap();
        assert_eq!(cd.shape, [257, 257, 2]);
        // Out-of-field corners of a real correction grid are NaN.
        assert!(cd.data.iter().any(|v| v.is_nan()));
    }

    #[test]
    fn get_scan_field_correction_bytes_matches_file_size() {
        let reader = Hdf5AdapterV1_0::open(REFERENCE).unwrap();
        let bytes = reader.get_scan_field_correction_bytes(0).unwrap();
        assert_eq!(bytes.len(), 1_138_799);
    }

    #[test]
    fn get_raw_group_returns_empty_map_for_missing_path() {
        let reader = Hdf5AdapterV1_0::open(REFERENCE).unwrap();
        let result = reader.get_raw_group("OPCUA").unwrap();
        assert!(result.is_empty());
        let result = reader.get_raw_group("does/not/exist").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn reference_fixture_has_no_opcua() {
        let config = Hdf5AdapterV1_0::open(REFERENCE).unwrap().parse().unwrap();
        assert!(config.opcua.is_none());
    }

    #[test]
    fn opcua_fixture_parses_client_pipe_and_triggers() {
        let config = Hdf5AdapterV1_0::open(REFERENCE_OPCUA).unwrap().parse().unwrap();
        let opcua = config.opcua.expect("OPCUA group present on this fixture");

        assert!(!opcua.client.server_url.is_empty());
        assert!(opcua.client.bfs_max_depth > 0);
        assert_eq!(opcua.triggers_enabled, Some(true));
        assert!(opcua.triggers.contains_key("Laser Emission Interlock"));

        let trigger = &opcua.triggers["Laser Emission Interlock"];
        assert!(trigger.signal.is_some());
        // Trigger_Label is now a typed field, not swept into `extra`.
        assert_eq!(trigger.trigger_label, Some("Laser Emission Interlock".into()));
        assert!(!trigger.extra.contains_key("Trigger_Label"));
    }

    /// Every one of the 22 newly-promoted fields (plus the new
    /// `trigger_stop_ceiling_layers`), checked against the real values in
    /// `reference_config_opcua.h5` — verified directly via h5py before writing
    /// this test. Also confirms none of them still land in `extra`.
    #[test]
    fn opcua_fixture_promoted_fields_have_real_values() {
        let config = Hdf5AdapterV1_0::open(REFERENCE_OPCUA).unwrap().parse().unwrap();
        let opcua = config.opcua.expect("OPCUA group present on this fixture");

        let c = &opcua.client;
        assert_eq!(c.keep_alive_count, Some(240));
        assert_eq!(c.lifetime_count, Some(2400));
        assert_eq!(c.machine_profile, Some("Aconity".into()));
        assert_eq!(c.queue_policy, Some("DropOldest".into()));
        assert_eq!(c.queue_size_data_change, Some(100));
        assert_eq!(c.queue_size_events, Some(7200));
        assert_eq!(c.reconnect_interval, Some(10000));
        assert_eq!(c.root_node, Some("MachineFleet".into()));
        assert_eq!(c.sync_loop_interval_initial, Some(1000));
        assert_eq!(c.sync_loop_interval_settled, Some(30000));
        for key in KNOWN_CLIENT_KEYS {
            assert!(!c.extra.contains_key(*key), "{key} should be typed, not in extra");
        }

        let p = &opcua.pipe;
        assert_eq!(p.configure_client, Some(true));
        assert_eq!(p.inbound_rate_limit, Some(-1));
        assert_eq!(p.max_inbound_message_size, Some(65536));
        assert_eq!(p.min_integrity_level, Some("0x2000".into()));
        assert_eq!(p.pipe_name, Some("\\\\.\\pipe\\opc_ua_client_pipe".into()));
        assert_eq!(p.user_access_level, Some("AnyLocalUser".into()));
        for key in KNOWN_PIPE_KEYS {
            assert!(!p.extra.contains_key(*key), "{key} should be typed, not in extra");
        }

        assert_eq!(opcua.trigger_stop_ceiling_layers, Some(3));

        let laser = &opcua.triggers["Laser Emission Interlock"];
        assert_eq!(laser.case_sensitivity, Some("Exact".into()));
        assert_eq!(laser.component, Some("machine_state_indicator".into()));
        assert_eq!(laser.cooldown_period, Some(0));
        assert_eq!(laser.event, Some("SensorEvents".into()));
        assert_eq!(laser.max_fires_per_job, Some(0));
        assert_eq!(laser.trigger_label, Some("Laser Emission Interlock".into()));

        let oxygen = &opcua.triggers["Chamber Oxygen Level"];
        assert_eq!(oxygen.component, Some("process_chamber::gas_management::oxygen_sensor::1".into()));
        assert_eq!(oxygen.event, Some("SensorEvents".into()));
        assert_eq!(oxygen.trigger_label, Some("Chamber Oxygen Level".into()));
        for key in KNOWN_TRIGGER_KEYS {
            assert!(!laser.extra.contains_key(*key), "{key} should be typed, not in extra");
            assert!(!oxygen.extra.contains_key(*key), "{key} should be typed, not in extra");
        }
    }

    /// `opcua_missing_required.h5` (Phase 0) deletes all seven Phase-2-required
    /// attributes. The reader stays permissive (facade-only enforcement — see
    /// OPCUA_FIELD_PROMOTION_PLAN.md): parsing must still succeed, the removed
    /// fields read back `None`, and everything else is unaffected — including
    /// the deliberate asymmetry that only one trigger lost `Event`.
    #[test]
    fn opcua_missing_required_fixture_parses_gracefully() {
        let config = Hdf5AdapterV1_0::open(OPCUA_MISSING_REQUIRED).unwrap().parse().unwrap();
        let opcua = config.opcua.expect("OPCUA group present on this fixture");

        assert_eq!(opcua.client.machine_profile, None);
        assert_eq!(opcua.client.root_node, None);
        assert_eq!(opcua.pipe.configure_client, None);
        assert_eq!(opcua.pipe.pipe_name, None);
        assert_eq!(opcua.triggers_enabled, None);
        assert_eq!(opcua.trigger_stop_ceiling_layers, None);

        let laser = &opcua.triggers["Laser Emission Interlock"];
        assert_eq!(laser.event, None, "Event was deliberately removed from this trigger only");
        let oxygen = &opcua.triggers["Chamber Oxygen Level"];
        assert_eq!(oxygen.event, Some("SensorEvents".into()), "the other trigger must be unaffected");

        // Untouched fields on both the client and the still-present trigger.
        assert!(!opcua.client.server_url.is_empty());
        assert_eq!(opcua.client.keep_alive_count, Some(240));
        assert_eq!(opcua.pipe.buffer_size, 65536);
        assert_eq!(laser.trigger_label, Some("Laser Emission Interlock".into()));
    }

    #[test]
    fn get_raw_group_on_opcua_client() {
        let reader = Hdf5AdapterV1_0::open(REFERENCE_OPCUA).unwrap();
        let attrs = reader.get_raw_group("OPCUA/Client").unwrap();
        assert!(attrs.contains_key("Server_URL"));
    }

    #[test]
    fn synthetic_fixture_parses() {
        let config = Hdf5AdapterV1_0::open(SYNTHETIC).unwrap().parse().unwrap();
        assert!(!config.meta.machine_name.is_empty());
        assert_eq!(config.optical_trains.len(), 2);
    }

    #[test]
    fn to_json_default_excludes_binary_fields() {
        let reader = Hdf5AdapterV1_0::open(REFERENCE).unwrap();
        let json = reader.to_json(true, false).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert!(parsed.get("meta").is_some());
        assert!(!json.contains("\"correction_data\""));
        assert!(!json.contains("\"raw_bytes\""));
        assert!(!json.contains("\"opcua\""));
    }

    #[test]
    fn to_json_include_binary_adds_correction_and_raw_bytes() {
        let reader = Hdf5AdapterV1_0::open(REFERENCE).unwrap();
        let json = reader.to_json(false, true).unwrap();
        assert!(json.contains("\"correction_data\""));
        assert!(json.contains("\"raw_bytes\""));
    }

    /// Cross-language drift check: the golden file is exactly what Python's
    /// `to_json(indent=2)` (default `include_binary=False`) produces for
    /// `reference_config.h5` (see IMPLEMENTATION_PLAN.md Phase 1.7). Deep
    /// (not string) equality against the Rust reader's output at the same
    /// settings is the same guarantee `tools/cross_check.py` will enforce
    /// once Phase 5 wires this reader into that pipeline.
    #[test]
    fn matches_python_golden_file() {
        let golden_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_output.json");
        let golden: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(golden_path).unwrap()).unwrap();

        let reader = Hdf5AdapterV1_0::open(REFERENCE).unwrap();
        let ours: serde_json::Value =
            serde_json::from_str(&reader.to_json(true, false).unwrap()).unwrap();

        assert_eq!(ours, golden, "Rust reader output diverges from the Python golden file");
    }
}
