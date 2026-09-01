//! File_Version 1.1 HDF5 reader — on-disk layout, attribute names, casting,
//! and v1.0<->v1.1 migration.
//!
//! Deliberately independent of `capabilities/v1_0/hdf5.rs` — every helper
//! used here (type-conversion, sub-group parsing) is defined fresh in this
//! module, even where the on-disk shape happens to match v1.0 today
//! (Collimator, Scanner_Card, sfcf, OPCUA's inner Client/Pipe/Triggers shape,
//! train-attribute read/write, axis reading minus tuning). v1.0 could change
//! or be removed later without affecting v1.1. See
//! `docs/migrations/v1_0_to_v1_1.md` for the full migration manifest.

use std::path::{Path, PathBuf};

use hdf5::types::{FixedUnicode, TypeDescriptor, VarLenAscii, VarLenUnicode};
use hdf5::{Dataset, File as H5File, Group, Location};
use indexmap::IndexMap;

use super::layout;
use crate::error::{MachineConfigError, Result};
use crate::models::*;
use crate::reader::ReaderAdapter;

const SCHEMA_VERSION: &str = "v1";

/// Same fixed-length compound-dataset convention as v1.0's own
/// `Derivation_Equation_Constants`/`Calibration_Points` — defined fresh here
/// rather than imported from v1_0, per this module's independence
/// requirement (see module docstring).
pub(crate) const EQUATION_CONSTANT_NAME_MAX_BYTES: usize = 64;

/// Shared `Derivation_Equation_Constants` compound-row type — UTF-8,
/// matching v1.0's own established SynchronousSensor convention. Used for
/// both `Power_Characterization`'s and `Synchronous_Sensors`' datasets: both
/// are meant to be the identical on-disk convention (confirmed against
/// `fixtures/reference_config_v1_1.h5` after fixing a fixture-generation bug
/// that had briefly given `Power_Characterization`'s copy an ASCII cset
/// instead of UTF-8 — see `file_testing/build_v1_1_review_file.py`'s git
/// history for that fix).
#[derive(hdf5::H5Type, Clone, Debug)]
#[repr(C)]
pub(crate) struct RawEquationConstant {
    pub(crate) name: FixedUnicode<64>,
    pub(crate) value: f64,
}

#[derive(hdf5::H5Type, Clone, Debug)]
#[repr(C)]
pub(crate) struct RawCalibrationPoint {
    pub(crate) input_value: f64,
    pub(crate) output_value: f64,
}

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
// Attribute-reading helpers — independent copies of the equivalent v1.0
// helpers (private-to-module either way, so this duplication is also
// mechanically required, not just a style choice).
// ---------------------------------------------------------------------------

enum RawValue {
    Str(String),
    Int(i64),
    Float(f64),
}

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

fn read_required_str(loc: &Location, key: &str) -> Result<String> {
    match read_raw(loc, key)? {
        None => Ok(String::new()),
        Some(RawValue::Str(s)) => Ok(s),
        Some(RawValue::Int(i)) => Ok(i.to_string()),
        Some(RawValue::Float(f)) => Ok(f.to_string()),
    }
}

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

fn read_required_int(loc: &Location, key: &str, default: i64) -> Result<i64> {
    match read_int(loc, key)? {
        Some(v) => Ok(v),
        None => {
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

fn read_required_bool_from_int(loc: &Location, key: &str, default: i64) -> Result<bool> {
    Ok(read_required_int(loc, key, default)? != 0)
}

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

fn require_group(parent: &Group, path: &str) -> Result<Group> {
    parent.group(path).map_err(|_| MachineConfigError::MissingGroup(path.to_string()))
}

// ---------------------------------------------------------------------------
// File_Version 1.1 HDF5 adapter
// ---------------------------------------------------------------------------

/// File_Version 1.1 HDF5 reader. Public [`crate::reader::MachineConfigReader`] dispatches here.
pub struct Hdf5AdapterV1_1 {
    path: PathBuf,
}

impl Hdf5AdapterV1_1 {
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

    pub fn get_correction_data(&self, train_index: usize) -> Result<CorrectionData> {
        let f = H5File::open(&self.path)?;
        let tid = layout::train_id(train_index);
        let path = layout::correction_data_path_by_id(&tid);
        let arr = f.dataset(&path)?.read::<f64, ndarray::Ix3>()?;
        let shape = [arr.shape()[0], arr.shape()[1], arr.shape()[2]];
        Ok(CorrectionData { data: arr.into_raw_vec_and_offset().0, shape })
    }

    pub fn get_inverse_correction_data(&self, train_index: usize) -> Result<CorrectionData> {
        let f = H5File::open(&self.path)?;
        let tid = layout::train_id(train_index);
        let path = layout::inverse_correction_data_path_by_id(&tid);
        let arr = f.dataset(&path)?.read::<f64, ndarray::Ix3>()?;
        let shape = [arr.shape()[0], arr.shape()[1], arr.shape()[2]];
        Ok(CorrectionData { data: arr.into_raw_vec_and_offset().0, shape })
    }

    pub fn get_scan_field_correction_bytes(&self, train_index: usize) -> Result<Vec<u8>> {
        let f = H5File::open(&self.path)?;
        let tid = layout::train_id(train_index);
        let path = layout::scan_field_correction_file_path_by_id(&tid);
        Ok(f.dataset(&path)?.read_raw::<u8>()?)
    }

    pub fn get_raw_group(&self, hdf5_path: &str) -> Result<ExtraAttrs> {
        let f = H5File::open(&self.path)?;
        let grp = match f.group(hdf5_path) {
            Ok(g) => g,
            Err(_) => return Ok(IndexMap::new()),
        };
        collect_extra(&grp, &[])
    }

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
            build_plate_radius_unit: read_unit_locked(&m, "Build_Plate_Corner_Radius_unit", "mm")?,
            gas_flow_direction: read_str(&m, "Gas_Flow_Direction")?,
            recoat_direction: read_str(&m, "Recoat_Direction")?,
        };

        // ---- ClearBox: shared Output_Path/Software_Trigger_Delay (Consolidate, Change 1) ----
        let has_clearbox_root = f.group(layout::GROUP_CLEARBOX).is_ok();
        let mut shared_output_path: Option<String> = None;
        let mut shared_software_trigger_delay: Option<i64> = None;
        if has_clearbox_root {
            let cb_root = f.group(layout::GROUP_CLEARBOX)?;
            shared_output_path = read_str(&cb_root, "Output_Path")?;
            shared_software_trigger_delay = read_int(&cb_root, "Software_Trigger_Delay")?;
        }

        let trains_grp = require_group(&m, "Optical_Trains")?;
        let mut train_ids: Vec<String> = trains_grp
            .member_names()?
            .into_iter()
            .filter(|k| k.starts_with(layout::TRAIN_ID_PREFIX))
            .collect();
        train_ids.sort();
        let optical_trains = train_ids
            .iter()
            .map(|tid| {
                self.parse_train(
                    f,
                    tid,
                    has_clearbox_root,
                    shared_output_path.clone(),
                    shared_software_trigger_delay,
                    include_binary,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let opcua = self.parse_opcua(f)?;

        Ok(MachineConfig { meta, machine, optical_trains, opcua })
    }

    fn parse_train(
        &self,
        f: &H5File,
        train_id: &str,
        has_clearbox_root: bool,
        shared_output_path: Option<String>,
        shared_software_trigger_delay: Option<i64>,
        include_binary: bool,
    ) -> Result<OpticalTrain> {
        let base = layout::train_path_by_id(train_id);
        let a = require_group(f, &base)?;

        let scanner = self.parse_scanner(&require_group(&a, layout::GROUP_SCANNER)?)?;
        let light_source =
            self.parse_light_source(&require_group(&a, layout::GROUP_LIGHT_SOURCE)?)?;
        let collimator = self.parse_collimator(&require_group(&a, layout::GROUP_COLLIMATOR)?)?;
        let scanner_card =
            self.parse_scanner_card(&require_group(&a, layout::GROUP_SCANNER_CARD)?)?;

        let clearbox = if has_clearbox_root {
            let cb_path = layout::clearbox_path_by_id(train_id);
            match f.group(&cb_path) {
                Ok(grp) => Some(self.parse_clearbox(
                    &grp,
                    shared_output_path,
                    shared_software_trigger_delay,
                    include_binary,
                )?),
                Err(_) => None,
            }
        } else {
            None
        };

        let sfcf_path = layout::scan_field_correction_file_path_by_id(train_id);
        let sfcf = match f.dataset(&sfcf_path) {
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
            optional_components: OptionalComponents { clearbox },
            scan_field_correction_file: sfcf,
        })
    }

    /// Change 5: `Tuning_Parameters`/`Tuning_Type` have no on-disk source in
    /// v1.1 — always `None` here, regardless of what's in the group.
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
            tuning_parameters: None,
            tuning_type: None,
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
            invert_actual_x: read_bool_from_int(grp, "Invert_Actual_X")?.unwrap_or(false),
            invert_actual_y: read_bool_from_int(grp, "Invert_Actual_Y")?.unwrap_or(false),
            invert_commanded_x: read_bool_from_int(grp, "Invert_Commanded_X")?.unwrap_or(false),
            invert_commanded_y: read_bool_from_int(grp, "Invert_Commanded_Y")?.unwrap_or(false),
        })
    }

    fn parse_power_characterization(&self, grp: &Group) -> Result<PowerCharacterization> {
        let derivation_equation_constants = match grp.dataset(layout::DS_DERIVATION_EQUATION_CONSTANTS)
        {
            Ok(ds) => ds
                .read_raw::<RawEquationConstant>()?
                .into_iter()
                .map(|r| EquationConstant { name: r.name.as_str().to_string(), value: r.value })
                .collect(),
            Err(_) => Vec::new(),
        };
        let characterization_points = match grp.dataset(layout::DS_CHARACTERIZATION_POINTS) {
            Ok(ds) => ds
                .read_raw::<RawCalibrationPoint>()?
                .into_iter()
                .map(|r| CalibrationPoint { input_value: r.input_value, output_value: r.output_value })
                .collect(),
            Err(_) => Vec::new(),
        };
        Ok(PowerCharacterization {
            algorithm_type: read_str(grp, "Algorithm_Type")?,
            algorithm_equation: read_str(grp, "Algorithm_Equation")?,
            input_type: read_str(grp, "Input_Type")?,
            units_derived_quantity: read_str(grp, "Units_Derived_Quantity")?,
            derivation_equation_constants,
            characterization_points,
        })
    }

    fn parse_light_source(&self, grp: &Group) -> Result<LightSource> {
        let power_characterization = match grp.group(layout::GROUP_POWER_CHARACTERIZATION) {
            Ok(pc_grp) => Some(self.parse_power_characterization(&pc_grp)?),
            Err(_) => None,
        };
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
            power_bit_resolution_unit: read_unit_locked(grp, "Power_Bit_Resolution_unit", "bits")?,
            // Change 4: no on-disk source in v1.1 — superseded by power_characterization.
            watts_to_volts_algorithm: None,
            watts_to_volts_params: None,
            power_characterization,
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

    fn parse_clearbox(
        &self,
        grp: &Group,
        shared_output_path: Option<String>,
        shared_software_trigger_delay: Option<i64>,
        include_binary: bool,
    ) -> Result<ClearBox> {
        let (correction_data, inverse_correction_data) = if include_binary {
            let cd = grp.dataset(layout::DS_CORRECTION_DATA)?.read::<f64, ndarray::Ix3>()?;
            let icd =
                grp.dataset(layout::DS_INVERSE_CORRECTION_DATA)?.read::<f64, ndarray::Ix3>()?;
            (Some(nan_array3_to_nested(&cd)), Some(nan_array3_to_nested(&icd)))
        } else {
            (None, None)
        };
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
        let power_characterization = match grp.group(layout::GROUP_POWER_CHARACTERIZATION) {
            Ok(pc_grp) => Some(self.parse_power_characterization(&pc_grp)?),
            Err(_) => None,
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
            // Change 1 (Consolidate): one shared value for every train that
            // has a ClearBox, read from Extensions/ClearBox/ itself.
            output_path: shared_output_path,
            software_trigger_delay: shared_software_trigger_delay,
            // Change 1 (Removal): no on-disk source in v1.1.
            selected_camera: None,
            custom_video_format: None,
            video_output: None,
            show_console: None,
            // Change 3: no on-disk source in v1.1 — superseded by power_characterization.
            volts_to_watts_algorithm: None,
            volts_to_watts_params: None,
            correction_grid_domain_shape: None,
            inverse_grid_domain_shape: None,
            synchronous_sensors,
            firmware_version: read_str(grp, "Firmware_Version")?,
            power_characterization,
        })
    }

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

    /// Parses the optional `Extensions/TM_OPCUA` group tree (Change 2).
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

        Ok(Some(OpcuaConfig { client, pipe, triggers, triggers_enabled, trigger_stop_ceiling_layers }))
    }
}

impl ReaderAdapter for Hdf5AdapterV1_1 {
    fn parse(&self) -> Result<MachineConfig> {
        self.parse()
    }
    fn parse_with_binary(&self) -> Result<MachineConfig> {
        self.parse_with_binary()
    }
    fn get_correction_data(&self, train_index: usize) -> Result<CorrectionData> {
        self.get_correction_data(train_index)
    }
    fn get_inverse_correction_data(&self, train_index: usize) -> Result<CorrectionData> {
        self.get_inverse_correction_data(train_index)
    }
    fn get_scan_field_correction_bytes(&self, train_index: usize) -> Result<Vec<u8>> {
        self.get_scan_field_correction_bytes(train_index)
    }
    fn get_raw_group(&self, hdf5_path: &str) -> Result<ExtraAttrs> {
        self.get_raw_group(hdf5_path)
    }
    fn to_json(&self, pretty: bool, include_binary: bool) -> Result<String> {
        self.to_json(pretty, include_binary)
    }
}

// ---------------------------------------------------------------------------
// Phase 2 (V1_1_IMPLEMENTATION_PLAN.md) removed migrate_v1_to_v1_1/
// migrate_v1_1_to_v1 from here — the coefficients/points shape-conversion
// functions they used now live in crate::power_characterization (imported by
// capabilities/v1_0/writer.rs and this module's own writer.rs), called as a
// write-time fallback, not a standalone migration step. See that module's
// docs for the full design.
// ---------------------------------------------------------------------------
