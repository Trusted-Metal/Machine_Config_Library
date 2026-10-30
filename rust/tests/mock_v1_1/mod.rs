//! Mock v1.1 adapter — test artifact only, exercises the change-category
//! architecture (Addition/Removal/Name/Path/Name+Path), not a planned schema
//! change. See `docs/migrations/mock_v1_0_to_v1_1.md` for the full manifest.
//!
//! Design: fully self-contained. Earlier revisions of this file delegated to
//! the real, public `capabilities::v1_0` adapter for the whole file, then
//! patched the 10 documented differences afterward. That violated the
//! project-wide rule that no version's adapter may import or call into
//! another version's adapter code — even a test-only mock — because it would
//! mean v1.0 could never be changed or removed without checking every mock
//! (and every later real version) that quietly depends on it. This file
//! instead reimplements the equivalent of `Hdf5AdapterV1_0::parse()` /
//! `Hdf5WriterV1_0::write()` from scratch: every subcomponent this mock does
//! *not* change (OPCUA, LightSource, Collimator, ScannerCard,
//! sfcf, all other Scanner/AxisConfig fields) is read and
//! written natively here, not borrowed from `capabilities::v1_0`. The 10
//! documented differences are implemented directly in the native
//! read/write paths rather than as a patch-after-the-fact. Mirrors
//! `nodejs/tests/mockV1_1.ts`'s design intent (self-contained mock), not its
//! literal delegate-then-patch implementation.
//!
//! Lives at `rust/tests/mock_v1_1/mod.rs` (not a flat `tests/mock_v1_1.rs`)
//! so it's an ordinary module `mod`-included by `adapter_migration_test.rs`,
//! not a second, independent integration-test binary that Cargo would try to
//! compile on its own — a flat file directly under `tests/` is always
//! auto-registered as its own test target.
//!
//! Being an integration-test module (a separate crate from `machine_config`
//! itself), this file only has access to the library's `pub` items — it
//! cannot reach `capabilities::v1_0`'s `pub(crate)` helpers
//! (`RawEquationConstant`, `RawCalibrationPoint`, `nested_to_array3`, ...)
//! even if it wanted to. Every such helper below is this file's own copy.
//!
//! There is no dispatch-table injection here (unlike Python's `_ADAPTERS` or
//! Node's `_READERS`/`_WRITERS`): Rust's public dispatcher is a hardcoded
//! `match` in `reader.rs`/`writer.rs`, not a registry, so AV-09–11 test this
//! mock directly rather than through `MachineConfigReader`/`MachineConfigWriter`
//! (see VALIDATION_PLAN.md §9.3 for the reasoning).

use hdf5::types::{TypeDescriptor, VarLenAscii, VarLenUnicode};
use hdf5::{Dataset, File as H5File, Group, Location};
use indexmap::IndexMap;
use machine_config::error::{MachineConfigError, Result};
use machine_config::models::*;
use machine_config::MachineConfig;
use ndarray::Array1;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// MockV1_1Layout — on-disk constants that differ from v1.0
// ---------------------------------------------------------------------------

pub const FILE_VERSION: &str = "1.1-mock";
pub const ATTR_FACILITY_ID: &str = "Facility_ID";
pub const ATTR_CONFIG_AUTHOR: &str = "Config_Author";
pub const ATTR_MACHINE_LABEL: &str = "Machine_Label";
pub const ATTR_FOCAL_DISTANCE: &str = "Focal_Distance";
pub const ATTR_BP_WIDTH: &str = "Width";
pub const ATTR_BP_HEIGHT: &str = "Height";

const SCHEMA_VERSION: &str = "v1";

const KNOWN_ROOT_KEYS: &[&str] = &[
    "machine_name",
    "manufacturer",
    "model",
    "serial_number",
    "File_Version",
    "Export_Date",
    "Configuration_Hash",
    ATTR_FACILITY_ID,
    ATTR_CONFIG_AUTHOR,
];

// ---------------------------------------------------------------------------
// Attribute-reading helpers — local copies of the same-named helpers in
// `capabilities::v1_0::hdf5` (which are private to that module). See that
// file's §3.11 rule-table comments for the semantics each one implements;
// not repeated here to keep this file focused on what differs.
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

/// Reads `key` as a float from an optional group — `Ok(None)` if the group
/// itself is absent (used for the mock's `Machine/Dimensions` group, which
/// won't exist when parsing a plain v1.0-shaped file by mistake).
fn read_float_from(grp: &Option<Group>, key: &str) -> Result<Option<f64>> {
    match grp {
        Some(g) => read_float(g, key),
        None => Ok(None),
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
// Write helpers — local copies of the same-named helpers in
// `capabilities::v1_0::writer` (private to that module).
// ---------------------------------------------------------------------------

fn ws(grp: &Group, key: &str, val: &str) -> Result<()> {
    let vlu: VarLenUnicode =
        val.parse().map_err(|e| MachineConfigError::Parse(format!("VarLenUnicode: {e:?}")))?;
    grp.new_attr::<VarLenUnicode>().create(key)?.write_scalar(&vlu)?;
    Ok(())
}

fn wf(grp: &Group, key: &str, val: Option<f64>) -> Result<()> {
    match val {
        Some(v) => {
            grp.new_attr::<f64>().create(key)?.write_scalar(&v)?;
        }
        None => ws(grp, key, "")?,
    }
    Ok(())
}

fn wi(grp: &Group, key: &str, val: Option<i64>) -> Result<()> {
    match val {
        Some(v) => {
            grp.new_attr::<i64>().create(key)?.write_scalar(&v)?;
        }
        None => ws(grp, key, "")?,
    }
    Ok(())
}

fn wb(grp: &Group, key: &str, val: Option<bool>) -> Result<()> {
    match val {
        Some(b) => {
            grp.new_attr::<i64>().create(key)?.write_scalar(&(b as i64))?;
        }
        None => ws(grp, key, "")?,
    }
    Ok(())
}

/// Mirrors `Hdf5WriterV1_0`'s `wb_if_true`: writes `1i64` only when `true`;
/// writes nothing at all when `false` (deliberately lossy for the four
/// `invert_*` scanner fields — see that function's doc for the rationale).
fn wb_if_true(grp: &Group, key: &str, val: bool) -> Result<()> {
    if val {
        grp.new_attr::<i64>().create(key)?.write_scalar(&1i64)?;
    }
    Ok(())
}

fn ws_ds(ds: &Dataset, key: &str, val: &str) -> Result<()> {
    let vlu: VarLenUnicode =
        val.parse().map_err(|e| MachineConfigError::Parse(format!("VarLenUnicode: {e:?}")))?;
    ds.new_attr::<VarLenUnicode>().create(key)?.write_scalar(&vlu)?;
    Ok(())
}

fn wi_ds(ds: &Dataset, key: &str, val: i64) -> Result<()> {
    ds.new_attr::<i64>().create(key)?.write_scalar(&val)?;
    Ok(())
}

fn write_extra_value(grp: &Group, key: &str, val: &serde_json::Value) -> Result<()> {
    match val {
        serde_json::Value::String(s) => ws(grp, key, s)?,
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                grp.new_attr::<i64>().create(key)?.write_scalar(&i)?;
            } else if let Some(f) = n.as_f64() {
                grp.new_attr::<f64>().create(key)?.write_scalar(&f)?;
            } else {
                ws(grp, key, &val.to_string())?;
            }
        }
        serde_json::Value::Bool(b) => {
            grp.new_attr::<i64>().create(key)?.write_scalar(&(*b as i64))?;
        }
        other => ws(grp, key, &other.to_string())?,
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// MockV1_1Reader — fully independent mock-v1.1 HDF5 parser
// ---------------------------------------------------------------------------

pub struct MockV1_1Reader {
    path: PathBuf,
}

impl MockV1_1Reader {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self { path: path.as_ref().to_path_buf() }
    }

    pub fn parse(&self) -> Result<MachineConfig> {
        let f = H5File::open(&self.path)?;

        // ADDITION (x2): Facility_ID / Config_Author are typed root attrs
        // here (unlike v1.0, where they don't exist at all).
        let meta = MachineConfigMeta {
            schema_version: SCHEMA_VERSION.to_string(),
            machine_name: read_required_str(&f, "machine_name")?,
            manufacturer: read_required_str(&f, "manufacturer")?,
            model: read_required_str(&f, "model")?,
            serial_number: read_required_str(&f, "serial_number")?,
            file_version: read_required_str(&f, "File_Version")?,
            export_date: read_required_str(&f, "Export_Date")?,
            configuration_hash: read_required_str(&f, "Configuration_Hash")?,
            facility_id: read_str(&f, ATTR_FACILITY_ID)?,
            config_author: read_str(&f, ATTR_CONFIG_AUTHOR)?,
            is_valid: None,
            extra: collect_extra(&f, KNOWN_ROOT_KEYS)?,
        };

        let m = require_group(&f, "Machine")?;
        // PATH / NAME+PATH: build-plate values live in Machine/Dimensions/
        // here, not directly on Machine/ as in v1.0. The group itself is
        // optional in principle (mirrors every other optional sub-group in
        // this reader) even though the mock writer always creates it.
        let dims = m.group("Dimensions").ok();

        let machine = Machine {
            id: read_str(&m, "ID")?,
            // NAME: Machine_Name -> Machine_Label.
            machine_name: read_required_str(&m, ATTR_MACHINE_LABEL)?,
            manufacturer: read_required_str(&m, "Manufacturer")?,
            model: read_required_str(&m, "Model")?,
            serial_number: read_required_str(&m, "Serial_Number")?,
            // NAME+PATH: Build_Plate_X/Y_Dimension -> Dimensions/Width|Height.
            build_plate_x: read_float_from(&dims, ATTR_BP_WIDTH)?,
            build_plate_x_unit: read_unit_locked(&m, "Build_Plate_X_Dimension_unit", "mm")?,
            build_plate_y: read_float_from(&dims, ATTR_BP_HEIGHT)?,
            build_plate_y_unit: read_unit_locked(&m, "Build_Plate_Y_Dimension_unit", "mm")?,
            // PATH: Build_Plate_Z_Dimension / Build_Plate_Corner_Radius move
            // into Dimensions/ under the same key.
            build_plate_z: read_float_from(&dims, "Build_Plate_Z_Dimension")?,
            build_plate_z_unit: read_unit_locked(&m, "Build_Plate_Z_Dimension_unit", "mm")?,
            build_plate_radius: read_float_from(&dims, "Build_Plate_Corner_Radius")?,
            build_plate_radius_unit: read_unit_locked(
                &m,
                "Build_Plate_Corner_Radius_unit",
                "mm",
            )?,
            // REMOVAL (x2): never read in the mock v1.1 layout.
            gas_flow_direction: None,
            recoat_direction: None,
            // Not part of this mock's manifest; always None here, same as any
            // other untouched Machine field.
            recoater_blade_type: None,
        };

        let trains_grp = require_group(&m, "Optical_Trains")?;
        let mut train_ids: Vec<String> = trains_grp
            .member_names()?
            .into_iter()
            .filter(|k| k.starts_with("Optical_Train_"))
            .collect();
        train_ids.sort();
        let optical_trains =
            train_ids.iter().map(|tid| self.parse_train(&f, tid)).collect::<Result<Vec<_>>>()?;


        Ok(MachineConfig { meta, machine, optical_trains, extensions: Default::default() })
    }

    fn parse_train(&self, f: &H5File, train_id: &str) -> Result<OpticalTrain> {
        let base = format!("Machine/Optical_Trains/{train_id}");
        let a = require_group(f, &base)?;

        let scanner = self.parse_scanner(&require_group(&a, "Scanner")?)?;
        let light_source = self.parse_light_source(&require_group(&a, "Light_Source")?)?;
        let collimator = self.parse_collimator(&require_group(&a, "Collimator")?)?;
        let scanner_card = self.parse_scanner_card(&require_group(&a, "Scanner_Card")?)?;

        let sfcf = match a.dataset("scan_field_correction_file") {
            Ok(ds) => Some(self.parse_sfcf(&ds)?),
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
            // NAME: Working_Distance -> Focal_Distance. The unit attr's key
            // is unaffected by the manifest and stays Working_Distance_unit.
            working_distance: read_float(grp, ATTR_FOCAL_DISTANCE)?,
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
            power_characterization: None,
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

    fn parse_sfcf(&self, ds: &Dataset) -> Result<ScanFieldCorrectionFile> {
        Ok(ScanFieldCorrectionFile {
            document_name: read_required_str(ds, "document_name")?,
            document_id: read_required_str(ds, "document_id")?,
            file_size: read_required_int(ds, "file_size", 0)?,
            valid_as_of_date: read_required_str(ds, "valid_as_of_date")?,
            document_created_at: read_str(ds, "document_created_at")?,
            document_type: read_str(ds, "document_type")?,
            original_uri: read_str(ds, "original_uri")?,
            raw_bytes: None,
        })
    }
}

// ---------------------------------------------------------------------------
// MockV1_1Writer — fully independent mock-v1.1 HDF5 writer
// ---------------------------------------------------------------------------

pub struct MockV1_1Writer<'a> {
    config: &'a MachineConfig,
}

impl<'a> MockV1_1Writer<'a> {
    pub fn new(config: &'a MachineConfig) -> Self {
        Self { config }
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let f = H5File::create(path)?;
        let root = f.group("/")?;
        self.write_root_attrs(&root)?;

        let machine_grp = f.create_group("Machine")?;
        self.write_machine_attrs(&machine_grp)?;
        let trains_grp = machine_grp.create_group("Optical_Trains")?;
        for (i, train) in self.config.optical_trains.iter().enumerate() {
            let tid = format!("Optical_Train_{:02}", i + 1);
            let train_grp = trains_grp.create_group(&tid)?;
            self.write_train_attrs(&train_grp, train)?;

            let scanner_grp = train_grp.create_group("Scanner")?;
            self.write_scanner(&scanner_grp, &train.scanner)?;

            let ls_grp = train_grp.create_group("Light_Source")?;
            self.write_light_source(&ls_grp, &train.light_source)?;

            let col_grp = train_grp.create_group("Collimator")?;
            self.write_collimator(&col_grp, &train.collimator)?;

            let sc_grp = train_grp.create_group("Scanner_Card")?;
            self.write_scanner_card(&sc_grp, &train.scanner_card)?;

            if let Some(sfcf) = &train.scan_field_correction_file {
                self.write_sfcf(&train_grp, sfcf)?;
            }
        }

        Ok(())
    }

    fn write_root_attrs(&self, root: &Group) -> Result<()> {
        let m = &self.config.meta;
        ws(root, "machine_name", &m.machine_name)?;
        ws(root, "manufacturer", &m.manufacturer)?;
        ws(root, "model", &m.model)?;
        ws(root, "serial_number", &m.serial_number)?;
        ws(root, "File_Version", &m.file_version)?;
        ws(root, "Export_Date", &m.export_date)?;
        ws(root, "Configuration_Hash", &m.configuration_hash)?;
        // ADDITION (x2): written as empty strings when absent so the file
        // stays schema-complete, matching every other optional string field.
        ws(root, ATTR_FACILITY_ID, m.facility_id.as_deref().unwrap_or(""))?;
        ws(root, ATTR_CONFIG_AUTHOR, m.config_author.as_deref().unwrap_or(""))?;
        for (k, v) in &m.extra {
            write_extra_value(root, k, v)?;
        }
        Ok(())
    }

    fn write_machine_attrs(&self, grp: &Group) -> Result<()> {
        let ma = &self.config.machine;
        ws(grp, "ID", ma.id.as_deref().unwrap_or(""))?;
        // NAME: Machine_Name -> Machine_Label.
        ws(grp, ATTR_MACHINE_LABEL, &ma.machine_name)?;
        ws(grp, "Manufacturer", &ma.manufacturer)?;
        ws(grp, "Model", &ma.model)?;
        ws(grp, "Serial_Number", &ma.serial_number)?;
        // Unit attrs stay on Machine/ unchanged, per the manifest.
        ws(grp, "Build_Plate_X_Dimension_unit", ma.build_plate_x_unit.as_deref().unwrap_or("mm"))?;
        ws(grp, "Build_Plate_Y_Dimension_unit", ma.build_plate_y_unit.as_deref().unwrap_or("mm"))?;
        ws(grp, "Build_Plate_Z_Dimension_unit", ma.build_plate_z_unit.as_deref().unwrap_or("mm"))?;
        ws(
            grp,
            "Build_Plate_Corner_Radius_unit",
            ma.build_plate_radius_unit.as_deref().unwrap_or("mm"),
        )?;
        // REMOVAL (x2): Gas_Flow_Direction / Recoat_Direction never written.

        // PATH / NAME+PATH: build-plate values move into Machine/Dimensions/.
        let dims = grp.create_group("Dimensions")?;
        wf(&dims, ATTR_BP_WIDTH, ma.build_plate_x)?;
        wf(&dims, ATTR_BP_HEIGHT, ma.build_plate_y)?;
        wf(&dims, "Build_Plate_Z_Dimension", ma.build_plate_z)?;
        wf(&dims, "Build_Plate_Corner_Radius", ma.build_plate_radius)?;
        Ok(())
    }

    fn write_train_attrs(&self, grp: &Group, t: &OpticalTrain) -> Result<()> {
        ws(grp, "ID", t.id.as_deref().unwrap_or(""))?;
        ws(grp, "Beam_Profile_Type", t.beam_profile_type.as_deref().unwrap_or(""))?;
        ws(grp, "Beam_Waist_Definition", t.beam_waist_definition.as_deref().unwrap_or(""))?;
        wf(grp, "Beam_Waist_Major", t.beam_waist_major)?;
        ws(grp, "Beam_Waist_Major_unit", t.beam_waist_major_unit.as_deref().unwrap_or("\u{03bc}m"))?;
        wf(grp, "Beam_Waist_Minor", t.beam_waist_minor)?;
        ws(grp, "Beam_Waist_Minor_unit", t.beam_waist_minor_unit.as_deref().unwrap_or("\u{03bc}m"))?;
        wf(grp, "Beam_Waist_Offset_Z", t.beam_waist_offset_z)?;
        ws(grp, "Beam_Waist_Offset_Z_unit", t.beam_waist_offset_z_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Build_Plane_Offset_Major", t.build_plane_offset_major)?;
        ws(grp, "Build_Plane_Offset_Major_unit", t.build_plane_offset_major_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Build_Plane_Offset_Minor", t.build_plane_offset_minor)?;
        ws(grp, "Build_Plane_Offset_Minor_unit", t.build_plane_offset_minor_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Collimator_Focal_Length", t.collimator_focal_length)?;
        ws(grp, "Collimator_Focal_Length_unit", t.collimator_focal_length_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "M2_Major", t.m2_major)?;
        wf(grp, "M2_Minor", t.m2_minor)?;
        wf(grp, "Major_Axis_Angle", t.major_axis_angle)?;
        ws(grp, "Major_Axis_Angle_unit", t.major_axis_angle_unit.as_deref().unwrap_or("degrees"))?;
        wf(grp, "Rayleigh_Length_Major", t.rayleigh_length_major)?;
        ws(grp, "Rayleigh_Length_Major_unit", t.rayleigh_length_major_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Rayleigh_Length_Minor", t.rayleigh_length_minor)?;
        ws(grp, "Rayleigh_Length_Minor_unit", t.rayleigh_length_minor_unit.as_deref().unwrap_or("mm"))?;
        ws(grp, "Scanner_Number", t.scanner_number.as_deref().unwrap_or(""))?;
        wb(grp, "Thermal_Lensing_Test_Passed", t.thermal_lensing_passed)?;
        wf(grp, "Thermal_Lensing_Focal_Plane_Shift", t.thermal_lensing_focal_plane_shift)?;
        ws(grp, "Thermal_Lensing_Focal_Plane_Shift_unit", t.thermal_lensing_focal_plane_shift_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Thermal_Lensing_Threshold", t.thermal_lensing_threshold)?;
        ws(grp, "Thermal_Lensing_Threshold_unit", t.thermal_lensing_threshold_unit.as_deref().unwrap_or("mm"))?;
        Ok(())
    }

    fn write_scanner(&self, grp: &Group, s: &Scanner) -> Result<()> {
        ws(grp, "Manufacturer", &s.manufacturer)?;
        ws(grp, "Model", &s.model)?;
        ws(grp, "Serial_Number", &s.serial_number)?;
        // NAME: Working_Distance -> Focal_Distance. Unit attr key unaffected.
        wf(grp, ATTR_FOCAL_DISTANCE, s.working_distance)?;
        ws(grp, "Working_Distance_unit", s.working_distance_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Scan_Field_Size_X", s.scan_field_x)?;
        ws(grp, "Scan_Field_Size_X_unit", s.scan_field_x_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Scan_Field_Size_Y", s.scan_field_y)?;
        ws(grp, "Scan_Field_Size_Y_unit", s.scan_field_y_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Scan_Field_Size_Z", s.scan_field_z)?;
        ws(grp, "Scan_Field_Size_Z_unit", s.scan_field_z_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Scan_Head_Offset_X", s.scan_head_offset_x)?;
        ws(grp, "Scan_Head_Offset_X_unit", s.scan_head_offset_x_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Scan_Head_Offset_Y", s.scan_head_offset_y)?;
        ws(grp, "Scan_Head_Offset_Y_unit", s.scan_head_offset_y_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Scan_Head_Offset_Z", s.scan_head_offset_z)?;
        ws(grp, "Scan_Head_Offset_Z_unit", s.scan_head_offset_z_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Scan_Head_Rotation", s.scan_head_rotation)?;
        ws(grp, "Scan_Head_Rotation_unit", s.scan_head_rotation_unit.as_deref().unwrap_or("degrees"))?;
        ws(grp, "Axis_Configuration", s.axis_configuration.as_deref().unwrap_or(""))?;
        wb_if_true(grp, "Invert_Actual_X", s.invert_actual_x)?;
        wb_if_true(grp, "Invert_Actual_Y", s.invert_actual_y)?;
        wb_if_true(grp, "Invert_Commanded_X", s.invert_commanded_x)?;
        wb_if_true(grp, "Invert_Commanded_Y", s.invert_commanded_y)?;

        let x_grp = grp.create_group("X_Axis")?;
        self.write_axis(&x_grp, &s.x_axis)?;
        let y_grp = grp.create_group("Y_Axis")?;
        self.write_axis(&y_grp, &s.y_axis)?;
        if let Some(z) = &s.z_axis {
            let z_grp = grp.create_group("Z_Axis")?;
            self.write_axis(&z_grp, z)?;
        }
        if let Some(focus) = &s.focus {
            let f_grp = grp.create_group("Focus")?;
            self.write_axis(&f_grp, focus)?;
        }
        Ok(())
    }

    fn write_axis(&self, grp: &Group, ax: &AxisConfig) -> Result<()> {
        wi(grp, "Actual_Bit_Resolution", ax.actual_bit_resolution)?;
        ws(grp, "Actual_Bit_Resolution_unit", ax.actual_bit_resolution_unit.as_deref().unwrap_or(""))?;
        wi(grp, "Commanded_Bit_Resolution", ax.commanded_bit_resolution)?;
        ws(grp, "Commanded_Bit_Resolution_unit", ax.commanded_bit_resolution_unit.as_deref().unwrap_or(""))?;
        ws(grp, "Control_Type", ax.control_type.as_deref().unwrap_or(""))?;
        wf(grp, "Range_Of_Motion", ax.range_of_motion)?;
        ws(grp, "Range_Of_Motion_unit", ax.range_of_motion_unit.as_deref().unwrap_or(""))?;
        ws(grp, "Smoothing_Kernel", ax.smoothing_kernel.as_deref().unwrap_or(""))?;
        wf(grp, "Smoothing_Parameters", ax.smoothing_parameters)?;
        ws(grp, "Tuning_Parameters", ax.tuning_parameters.as_deref().unwrap_or(""))?;
        ws(grp, "Tuning_Type", ax.tuning_type.as_deref().unwrap_or(""))?;
        Ok(())
    }

    fn write_light_source(&self, grp: &Group, ls: &LightSource) -> Result<()> {
        ws(grp, "Manufacturer", &ls.manufacturer)?;
        ws(grp, "Model", &ls.model)?;
        ws(grp, "Serial_Number", &ls.serial_number)?;
        wf(grp, "Light_Wavelength", ls.wavelength)?;
        ws(grp, "Light_Wavelength_unit", ls.wavelength_unit.as_deref().unwrap_or("nm"))?;
        wf(grp, "Power_Max_Nominal", ls.power_max_nominal)?;
        ws(grp, "Power_Max_Nominal_unit", ls.power_max_nominal_unit.as_deref().unwrap_or("W"))?;
        wf(grp, "Power_Max_Actual", ls.power_max_actual)?;
        ws(grp, "Power_Max_Actual_unit", ls.power_max_actual_unit.as_deref().unwrap_or("W"))?;
        wf(grp, "Power_Min_Actual", ls.power_min_actual)?;
        ws(grp, "Power_Min_Actual_unit", ls.power_min_actual_unit.as_deref().unwrap_or("W"))?;
        wf(grp, "Power_Min_Nominal", ls.power_min_nominal)?;
        ws(grp, "Power_Min_Nominal_unit", ls.power_min_nominal_unit.as_deref().unwrap_or("W"))?;
        // Power_Bit_Resolution is always stored as a string, matching v1.0.
        let pbr_str = ls.power_bit_resolution.map(|v| v.to_string()).unwrap_or_default();
        ws(grp, "Power_Bit_Resolution", &pbr_str)?;
        ws(grp, "Power_Bit_Resolution_unit", ls.power_bit_resolution_unit.as_deref().unwrap_or("bits"))?;
        Ok(())
    }

    fn write_collimator(&self, grp: &Group, c: &Collimator) -> Result<()> {
        ws(grp, "Manufacturer", &c.manufacturer)?;
        ws(grp, "Model", &c.model)?;
        ws(grp, "Serial_Number", &c.serial_number)?;
        wf(grp, "Focal_Length", c.focal_length)?;
        ws(grp, "Focal_Length_unit", c.focal_length_unit.as_deref().unwrap_or("mm"))?;
        Ok(())
    }

    fn write_scanner_card(&self, grp: &Group, sc: &ScannerCard) -> Result<()> {
        ws(grp, "Manufacturer", &sc.manufacturer)?;
        ws(grp, "Model", &sc.model)?;
        ws(grp, "Serial_Number", &sc.serial_number)?;
        ws(grp, "Communication_Protocol", sc.communication_protocol.as_deref().unwrap_or(""))?;
        wf(grp, "Sample_Period", sc.sample_period)?;
        ws(grp, "Sample_Period_unit", sc.sample_period_unit.as_deref().unwrap_or("\u{03bc}s"))?;
        Ok(())
    }

    fn write_sfcf(&self, train_grp: &Group, sfcf: &ScanFieldCorrectionFile) -> Result<()> {
        let data: Vec<u8> = match &sfcf.raw_bytes {
            Some(b) => b.clone(),
            None => vec![0u8; sfcf.file_size.max(1) as usize],
        };
        let arr = Array1::from(data);
        let ds = train_grp.new_dataset_builder().with_data(&arr).create("scan_field_correction_file")?;
        ws_ds(&ds, "document_name", &sfcf.document_name)?;
        ws_ds(&ds, "document_id", &sfcf.document_id)?;
        wi_ds(&ds, "file_size", sfcf.file_size)?;
        ws_ds(&ds, "valid_as_of_date", &sfcf.valid_as_of_date)?;
        ws_ds(&ds, "document_created_at", sfcf.document_created_at.as_deref().unwrap_or(""))?;
        ws_ds(&ds, "document_type", sfcf.document_type.as_deref().unwrap_or(""))?;
        ws_ds(&ds, "original_uri", sfcf.original_uri.as_deref().unwrap_or(""))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Factory helper — mirrors Python's `_make_config()` / Node's `makeMockConfig`
// ---------------------------------------------------------------------------

pub fn make_mock_config(
    machine_name: Option<&str>,
    facility_id: Option<&str>,
    config_author: Option<&str>,
) -> MachineConfig {
    let mut builder = machine_config::MockConfigBuilder::new(1);
    builder.build_plate_x = 250.0;
    builder.build_plate_y = 175.0; // distinct from x so name+path assertions are unambiguous
    builder.machine_name = machine_name.unwrap_or("MigrationTestMachine").to_string();

    let mut cfg = builder.build();
    cfg.meta.file_version = FILE_VERSION.to_string();
    cfg.meta.facility_id = facility_id.map(str::to_string);
    cfg.meta.config_author = config_author.map(str::to_string);
    cfg.machine.gas_flow_direction = None; // absent in v1.1-mock by design
    cfg.machine.recoat_direction = None;
    // MockConfigBuilder never sets this (always None) — give it a real value
    // so the PATH category (Build_Plate_Corner_Radius) has something
    // non-trivial to verify preservation of, mirroring Python's `_make_config`.
    cfg.machine.build_plate_radius = Some(10.0);
    cfg
}
