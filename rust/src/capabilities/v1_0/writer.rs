// Phase 3.6 — HDF5 writer: serialises MachineConfig back to a machine-config .h5 file.
// Exact inverse of reader.rs. See §3.10 (HDF5 path reference) and §3.12 (writer rules).
// Python reference: python/src/machine_config/writer.py

use std::path::Path;

use hdf5::types::VarLenUnicode;
use hdf5::{Dataset, File as H5File, Group};
use ndarray::Array1;

use super::hdf5::{RawCalibrationPoint, RawEquationConstant};
use super::layout;
use crate::error::Result;
use crate::models::*;

// ---------------------------------------------------------------------------
// Write helpers — exact inverses of the reader helpers (§3.11)
//
// All attribute helpers come in two flavours:
//   `ws` / `wf` / `wi` / `wb`   — operate on a `Group` (or root group)
//   `ws_ds` / `wi_ds`            — operate on a `Dataset` (for SFCF attrs)
//
// `File` exposes the same attribute API as `Group` in hdf5-metno.
// Root-level attributes are written by opening the root group with
// `f.group("/")` so all helpers uniformly accept `&Group`.
// ---------------------------------------------------------------------------

/// Writes `val` as a variable-length Unicode string attribute.
/// This matches what Python h5py writes for all string attrs, including `""`
/// for absent optional fields.
fn ws(grp: &Group, key: &str, val: &str) -> Result<()> {
    let vlu: VarLenUnicode = val.parse()
        .map_err(|e| crate::error::MachineConfigError::Parse(format!("VarLenUnicode: {e:?}")))?;
    grp.new_attr::<VarLenUnicode>().create(key)?.write_scalar(&vlu)?;
    Ok(())
}

/// Writes `Some(v)` as a float64 attribute, or `""` (VarLenUnicode) when `None`.
/// Mirrors Python `_f()`: `None` → `""`, numeric → `np.float64`.
fn wf(grp: &Group, key: &str, val: Option<f64>) -> Result<()> {
    match val {
        Some(v) => { grp.new_attr::<f64>().create(key)?.write_scalar(&v)?; }
        None => ws(grp, key, "")?,
    }
    Ok(())
}

/// Writes `Some(v)` as an i64 attribute, or `""` when `None`.
fn wi(grp: &Group, key: &str, val: Option<i64>) -> Result<()> {
    match val {
        Some(v) => { grp.new_attr::<i64>().create(key)?.write_scalar(&v)?; }
        None => ws(grp, key, "")?,
    }
    Ok(())
}

/// Writes `Some(true)` → `1i64`, `Some(false)` → `0i64`, `None` → `""`.
fn wb(grp: &Group, key: &str, val: Option<bool>) -> Result<()> {
    match val {
        Some(b) => { grp.new_attr::<i64>().create(key)?.write_scalar(&(b as i64))?; }
        None => ws(grp, key, "")?,
    }
    Ok(())
}

/// Writes an i64 attribute (`1`) only when `val` is `true`; writes nothing
/// at all when `false` — unlike `wb`, there is no "absent" placeholder
/// written for the `false` case. Used for Scanner's four `invert_*` fields,
/// which never appear in any output unless `true` (user-confirmed,
/// 2026-08-21): a write→read round-trip is deliberately lossy for an
/// explicit `false`, which becomes indistinguishable from "never set".
fn wb_if_true(grp: &Group, key: &str, val: bool) -> Result<()> {
    if val {
        grp.new_attr::<i64>().create(key)?.write_scalar(&1i64)?;
    }
    Ok(())
}

/// Writes a string attribute on a `Dataset` (used for SFCF attrs).
fn ws_ds(ds: &Dataset, key: &str, val: &str) -> Result<()> {
    let vlu: VarLenUnicode = val.parse()
        .map_err(|e| crate::error::MachineConfigError::Parse(format!("VarLenUnicode: {e:?}")))?;
    ds.new_attr::<VarLenUnicode>().create(key)?.write_scalar(&vlu)?;
    Ok(())
}

/// Writes an i64 attribute on a `Dataset`.
fn wi_ds(ds: &Dataset, key: &str, val: i64) -> Result<()> {
    ds.new_attr::<i64>().create(key)?.write_scalar(&val)?;
    Ok(())
}

/// Converts `Option<Vec<Vec<Vec<Option<f64>>>>>` back to an `Array3<f64>`.
/// `None` list cells become NaN; a `None` outer value produces a zero-filled
/// `(257, 257, 2)` array — required when the model was parsed without binary
/// data (see §3.12 writer rules).
fn nested_to_array3(
    data: &Option<Vec<Vec<Vec<Option<f64>>>>>,
) -> ndarray::Array3<f64> {
    const SHAPE: (usize, usize, usize) = (257, 257, 2);
    let zero = || ndarray::Array3::<f64>::zeros(SHAPE);
    let outer = match data {
        None => return zero(),
        Some(v) if v.is_empty() => return zero(),
        Some(v) => v,
    };
    let d0 = outer.len();
    let d1 = outer[0].len();
    let d2 = if d1 > 0 { outer[0][0].len() } else { 0 };
    if d0 == 0 || d1 == 0 || d2 == 0 {
        return zero();
    }
    let mut arr = ndarray::Array3::<f64>::from_elem((d0, d1, d2), f64::NAN);
    for (i, row) in outer.iter().enumerate() {
        for (j, col) in row.iter().enumerate() {
            for (k, &v) in col.iter().enumerate() {
                arr[[i, j, k]] = v.unwrap_or(f64::NAN);
            }
        }
    }
    arr
}

/// Writes a `serde_json::Value` from an `.extra` map back as an HDF5 attribute,
/// preserving the natural type (string → VarLenUnicode, integer → i64, float → f64).
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
// Public writer
// ---------------------------------------------------------------------------

/// File_Version 1.0 HDF5 writer. Public [`crate::writer::MachineConfigWriter`] dispatches here.
pub struct Hdf5WriterV1_0<'a> {
    config: &'a MachineConfig,
}

impl<'a> Hdf5WriterV1_0<'a> {
    pub fn new(config: &'a MachineConfig) -> Self {
        Self { config }
    }

    /// Writes the config to `path`, creating or overwriting the file.
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let f = H5File::create(path)?;
        // Root group opened at "/" so ws/wf/wi/wb helpers work uniformly.
        let root = f.group("/")?;
        self.write_root_attrs(&root)?;

        let machine_grp = f.create_group("Machine")?;
        self.write_machine_attrs(&machine_grp)?;
        let trains_grp = machine_grp.create_group("Optical_Trains")?;
        for (i, train) in self.config.optical_trains.iter().enumerate() {
            let tid = layout::train_id(i);
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

            if let Some(cb) = &train.optional_components.clearbox {
                let opt_grp = train_grp.create_group("Optional_Components")?;
                let cb_grp = opt_grp.create_group("ClearBox")?;
                self.write_clearbox(&cb_grp, cb)?;
            }

            if let Some(sfcf) = &train.scan_field_correction_file {
                self.write_sfcf(&train_grp, sfcf)?;
            }
        }

        if let Some(opcua) = &self.config.opcua {
            self.write_opcua(&f, opcua)?;
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Root attributes → MachineConfigMeta
    // ------------------------------------------------------------------

    fn write_root_attrs(&self, root: &Group) -> Result<()> {
        let m = &self.config.meta;
        ws(root, "machine_name", &m.machine_name)?;
        ws(root, "manufacturer", &m.manufacturer)?;
        ws(root, "model", &m.model)?;
        ws(root, "serial_number", &m.serial_number)?;
        ws(root, "File_Version", &m.file_version)?;
        ws(root, "Export_Date", &m.export_date)?;
        ws(root, "Configuration_Hash", &m.configuration_hash)?;
        for (k, v) in &m.extra {
            write_extra_value(root, k, v)?;
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Machine group → Machine
    // ------------------------------------------------------------------

    fn write_machine_attrs(&self, grp: &Group) -> Result<()> {
        let ma = &self.config.machine;
        ws(grp, "ID", ma.id.as_deref().unwrap_or(""))?;
        ws(grp, "Machine_Name", &ma.machine_name)?;
        ws(grp, "Manufacturer", &ma.manufacturer)?;
        ws(grp, "Model", &ma.model)?;
        ws(grp, "Serial_Number", &ma.serial_number)?;
        wf(grp, "Build_Plate_X_Dimension", ma.build_plate_x)?;
        ws(grp, "Build_Plate_X_Dimension_unit", ma.build_plate_x_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Build_Plate_Y_Dimension", ma.build_plate_y)?;
        ws(grp, "Build_Plate_Y_Dimension_unit", ma.build_plate_y_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Build_Plate_Z_Dimension", ma.build_plate_z)?;
        ws(grp, "Build_Plate_Z_Dimension_unit", ma.build_plate_z_unit.as_deref().unwrap_or("mm"))?;
        wf(grp, "Build_Plate_Corner_Radius", ma.build_plate_radius)?;
        ws(grp, "Build_Plate_Corner_Radius_unit", ma.build_plate_radius_unit.as_deref().unwrap_or("mm"))?;
        ws(grp, "Gas_Flow_Direction", ma.gas_flow_direction.as_deref().unwrap_or(""))?;
        ws(grp, "Recoat_Direction", ma.recoat_direction.as_deref().unwrap_or(""))?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Optical trains
    // ------------------------------------------------------------------

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
        wf(grp, "Working_Distance", s.working_distance)?;
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
        // Power_Bit_Resolution is always stored as a string in real HDF5 files — see §3.11.
        let pbr_str = ls.power_bit_resolution.map(|v| v.to_string()).unwrap_or_default();
        ws(grp, "Power_Bit_Resolution", &pbr_str)?;
        ws(grp, "Power_Bit_Resolution_unit", ls.power_bit_resolution_unit.as_deref().unwrap_or("bits"))?;
        ws(grp, "Watts_To_Volts_Algorithm", ls.watts_to_volts_algorithm.as_deref().unwrap_or(""))?;
        ws(grp, "Watts_To_Volts_Params", ls.watts_to_volts_params.as_deref().unwrap_or(""))?;
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

    fn write_clearbox(&self, grp: &Group, cb: &ClearBox) -> Result<()> {
        ws(grp, "Ip_Address", &cb.ip_address)?;
        ws(grp, "Serial_Number", cb.serial_number.as_deref().unwrap_or(""))?;
        wi(grp, "Data_Port", cb.data_port)?;
        wi(grp, "Server_Port", cb.server_port)?;
        wi(grp, "Actual_Timing_Offset", cb.actual_timing_offset)?;
        wi(grp, "Commanded_Timing_Offset", cb.commanded_timing_offset)?;
        ws(grp, "Manufacturer", cb.manufacturer.as_deref().unwrap_or(""))?;
        ws(grp, "Model", cb.model.as_deref().unwrap_or(""))?;
        ws(grp, "Output_Path", cb.output_path.as_deref().unwrap_or(""))?;
        ws(grp, "Selected_Camera", cb.selected_camera.as_deref().unwrap_or(""))?;
        ws(grp, "Custom_Video_Format", cb.custom_video_format.as_deref().unwrap_or(""))?;
        ws(grp, "Video_Output", cb.video_output.as_deref().unwrap_or(""))?;
        wb(grp, "Show_Console", cb.show_console)?;
        wi(grp, "Software_Trigger_Delay", cb.software_trigger_delay)?;
        ws(grp, "Volts_To_Watts_Algorithm", cb.volts_to_watts_algorithm.as_deref().unwrap_or(""))?;
        ws(grp, "Volts_To_Watts_Params", cb.volts_to_watts_params.as_deref().unwrap_or(""))?;
        ws(grp, "Correction_Grid_Domain_Shape", cb.correction_grid_domain_shape.as_deref().unwrap_or(""))?;
        ws(grp, "Inverse_Grid_Domain_Shape", cb.inverse_grid_domain_shape.as_deref().unwrap_or(""))?;

        // Correction dataset — zero-filled when binary data was not loaded.
        let cd_arr = nested_to_array3(&cb.correction_data);
        let shape = cd_arr.shape().to_owned();
        let cd_ds = grp
            .new_dataset_builder()
            .with_data(&cd_arr)
            .create("Correction_Data")?;
        ws_ds(&cd_ds, "dimensions", "H,W,D")?;
        ws_ds(&cd_ds, "dtype", "float64")?;
        ws_ds(&cd_ds, "shape", &format!("{}x{}x{}", shape[0], shape[1], shape[2]))?;

        let icd_arr = nested_to_array3(&cb.inverse_correction_data);
        let icd_shape = icd_arr.shape().to_owned();
        let icd_ds = grp
            .new_dataset_builder()
            .with_data(&icd_arr)
            .create("Inverse_Correction_Data")?;
        ws_ds(&icd_ds, "dimensions", "H,W,D")?;
        ws_ds(&icd_ds, "dtype", "float64")?;
        ws_ds(&icd_ds, "shape", &format!("{}x{}x{}", icd_shape[0], icd_shape[1], icd_shape[2]))?;

        // Synchronous_Sensors: only created when non-empty, so a ClearBox
        // with no sensors looks identical on disk to before this field
        // existed — no empty placeholder group. Mirrors ClearBox itself
        // being entirely absent rather than an empty shell.
        if !cb.synchronous_sensors.is_empty() {
            let sensors_grp = grp.create_group("Synchronous_Sensors")?;
            for (name, sensor) in &cb.synchronous_sensors {
                let sg = sensors_grp.create_group(name)?;
                self.write_synchronous_sensor(&sg, sensor)?;
            }
        }

        Ok(())
    }

    fn write_synchronous_sensor(&self, grp: &Group, sensor: &SynchronousSensor) -> Result<()> {
        wb(grp, "Enabled", sensor.enabled)?;
        ws(grp, "Sensor_Name", sensor.sensor_name.as_deref().unwrap_or(""))?;
        wf(grp, "Sensor_Output_Range_Low", sensor.sensor_output_range_low)?;
        wf(grp, "Sensor_Output_Range_High", sensor.sensor_output_range_high)?;
        ws(grp, "Sensor_Output_Space", sensor.sensor_output_space.as_deref().unwrap_or(""))?;
        ws(grp, "Sensor_Model", sensor.sensor_model.as_deref().unwrap_or(""))?;
        ws(grp, "Sensor_Manufacturer", sensor.sensor_manufacturer.as_deref().unwrap_or(""))?;
        ws(grp, "Sensor_Scope", sensor.sensor_scope.as_deref().unwrap_or(""))?;
        ws(grp, "Units_Derived_Quantity", sensor.units_derived_quantity.as_deref().unwrap_or(""))?;
        wi(grp, "Port_ID", sensor.port_id)?;
        ws(grp, "Sensor_Type", sensor.sensor_type.as_deref().unwrap_or(""))?;
        ws(grp, "Input_Type", sensor.input_type.as_deref().unwrap_or(""))?;
        ws(grp, "Algorithm_Type", sensor.algorithm_type.as_deref().unwrap_or(""))?;
        ws(grp, "Algorithm_Equation", sensor.algorithm_equation.as_deref().unwrap_or(""))?;
        ws(grp, "Calibration_Source", sensor.calibration_source.as_deref().unwrap_or(""))?;
        wb(grp, "Calibration_Verified", sensor.calibration_verified)?;
        wf(grp, "Sample_Period", sensor.sample_period)?;
        ws(grp, "Metadata", sensor.metadata.as_deref().unwrap_or(""))?;

        let const_rows: Vec<RawEquationConstant> = sensor
            .derivation_equation_constants
            .iter()
            .map(|c| -> Result<RawEquationConstant> {
                Ok(RawEquationConstant {
                    name: c.name.parse().map_err(|e| {
                        crate::error::MachineConfigError::Parse(format!(
                            "Derivation_Equation_Constants name {:?} does not fit in a \
                             64-byte FixedUnicode field: {e:?}",
                            c.name
                        ))
                    })?,
                    value: c.value,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let const_arr = Array1::from(const_rows);
        grp.new_dataset_builder()
            .with_data(&const_arr)
            .create("Derivation_Equation_Constants")?;

        let point_rows: Vec<RawCalibrationPoint> = sensor
            .calibration_points
            .iter()
            .map(|p| RawCalibrationPoint { input_value: p.input_value, output_value: p.output_value })
            .collect();
        let point_arr = Array1::from(point_rows);
        grp.new_dataset_builder()
            .with_data(&point_arr)
            .create("Calibration_Points")?;

        Ok(())
    }

    fn write_sfcf(&self, train_grp: &Group, sfcf: &ScanFieldCorrectionFile) -> Result<()> {
        let data: Vec<u8> = match &sfcf.raw_bytes {
            Some(b) => b.clone(),
            None => vec![0u8; sfcf.file_size.max(1) as usize],
        };
        let arr = Array1::from(data);
        let ds = train_grp
            .new_dataset_builder()
            .with_data(&arr)
            .create("scan_field_correction_file")?;
        ws_ds(&ds, "document_name", &sfcf.document_name)?;
        ws_ds(&ds, "document_id", &sfcf.document_id)?;
        wi_ds(&ds, "file_size", sfcf.file_size)?;
        ws_ds(&ds, "valid_as_of_date", &sfcf.valid_as_of_date)?;
        ws_ds(&ds, "document_created_at", sfcf.document_created_at.as_deref().unwrap_or(""))?;
        ws_ds(&ds, "document_type", sfcf.document_type.as_deref().unwrap_or(""))?;
        ws_ds(&ds, "original_uri", sfcf.original_uri.as_deref().unwrap_or(""))?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // OPCUA
    // ------------------------------------------------------------------

    fn write_opcua(&self, f: &H5File, opcua: &OpcuaConfig) -> Result<()> {
        // Client
        let opcua_grp = f.create_group("OPCUA")?;
        let client_grp = opcua_grp.create_group("Client")?;
        let c = &opcua.client;
        ws(&client_grp, "Server_URL", &c.server_url)?;
        ws(&client_grp, "Auth_Mode", &c.auth_mode)?;
        ws(&client_grp, "Security_Mode", &c.security_mode)?;
        ws(&client_grp, "Security_Policy", &c.security_policy)?;
        client_grp.new_attr::<i64>().create("BFS_Max_Depth")?.write_scalar(&c.bfs_max_depth)?;
        client_grp.new_attr::<i64>().create("Publish_Interval")?.write_scalar(&c.publish_interval)?;
        client_grp.new_attr::<i64>().create("Sampling_Interval")?.write_scalar(&c.sampling_interval)?;
        client_grp.new_attr::<i64>().create("Session_Timeout")?.write_scalar(&c.session_timeout)?;
        wi(&client_grp, "Keep_Alive_Count", c.keep_alive_count)?;
        wi(&client_grp, "Lifetime_Count", c.lifetime_count)?;
        ws(&client_grp, "Machine_Profile", c.machine_profile.as_deref().unwrap_or(""))?;
        ws(&client_grp, "Queue_Policy", c.queue_policy.as_deref().unwrap_or(""))?;
        wi(&client_grp, "Queue_Size_Data_Change", c.queue_size_data_change)?;
        wi(&client_grp, "Queue_Size_Events", c.queue_size_events)?;
        wi(&client_grp, "Reconnect_Interval", c.reconnect_interval)?;
        ws(&client_grp, "Root_Node", c.root_node.as_deref().unwrap_or(""))?;
        wi(&client_grp, "Sync_Loop_Interval_Initial", c.sync_loop_interval_initial)?;
        wi(&client_grp, "Sync_Loop_Interval_Settled", c.sync_loop_interval_settled)?;
        for (k, v) in &c.extra {
            write_extra_value(&client_grp, k, v)?;
        }

        // Pipe
        let pipe_grp = opcua_grp.create_group("Pipe")?;
        let p = &opcua.pipe;
        pipe_grp.new_attr::<i64>().create("Pipe_Enabled")?.write_scalar(&(p.pipe_enabled as i64))?;
        pipe_grp.new_attr::<i64>().create("Buffer_Size")?.write_scalar(&p.buffer_size)?;
        wb(&pipe_grp, "Configure_Client", p.configure_client)?;
        wi(&pipe_grp, "Inbound_Rate_Limit", p.inbound_rate_limit)?;
        wi(&pipe_grp, "Max_Inbound_Message_Size", p.max_inbound_message_size)?;
        ws(&pipe_grp, "Min_Integrity_Level", p.min_integrity_level.as_deref().unwrap_or(""))?;
        ws(&pipe_grp, "Pipe_Name", p.pipe_name.as_deref().unwrap_or(""))?;
        ws(&pipe_grp, "User_Access_Level", p.user_access_level.as_deref().unwrap_or(""))?;
        for (k, v) in &p.extra {
            write_extra_value(&pipe_grp, k, v)?;
        }

        // Triggers
        let triggers_grp = opcua_grp.create_group("Triggers")?;
        // `Triggers_Enabled` is stored as float64, not int — see §3.10.
        if let Some(te) = opcua.triggers_enabled {
            triggers_grp
                .new_attr::<f64>()
                .create("Triggers_Enabled")?
                .write_scalar(&(if te { 1.0_f64 } else { 0.0_f64 }))?;
        }
        wi(&triggers_grp, "Trigger_Stop_Ceiling_Layers", opcua.trigger_stop_ceiling_layers)?;
        for (name, trigger) in &opcua.triggers {
            let tg = triggers_grp.create_group(name)?;
            ws(&tg, "ID", trigger.id.as_deref().unwrap_or(""))?;
            ws(&tg, "Signal", trigger.signal.as_deref().unwrap_or(""))?;
            ws(&tg, "Subsystem", trigger.subsystem.as_deref().unwrap_or(""))?;
            wb(&tg, "Rule_Enabled", trigger.rule_enabled)?;
            ws(&tg, "Start_Value", trigger.start_value.as_deref().unwrap_or(""))?;
            ws(&tg, "Stop_Value", trigger.stop_value.as_deref().unwrap_or(""))?;
            ws(&tg, "Case_Sensitivity", trigger.case_sensitivity.as_deref().unwrap_or(""))?;
            ws(&tg, "Component", trigger.component.as_deref().unwrap_or(""))?;
            wi(&tg, "Cooldown_Period", trigger.cooldown_period)?;
            ws(&tg, "Event", trigger.event.as_deref().unwrap_or(""))?;
            wi(&tg, "Max_Fires_Per_Job", trigger.max_fires_per_job)?;
            ws(&tg, "Trigger_Label", trigger.trigger_label.as_deref().unwrap_or(""))?;
            for (k, v) in &trigger.extra {
                write_extra_value(&tg, k, v)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::MachineConfigReader;
    use tempfile::NamedTempFile;

    const REFERENCE: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config.h5");
    const REFERENCE_OPCUA: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/reference_config_opcua.h5");
    const SYNTHETIC: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/synthetic_2laser.h5");
    const REFERENCE_SENSORS: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/reference_config_synchronous_sensors.h5"
    );

    fn roundtrip(src: &str) -> MachineConfig {
        let original = MachineConfigReader::open(src).unwrap().parse().unwrap();
        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&original).write(tmp.path()).unwrap();
        MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap()
    }

    #[test]
    fn roundtrip_meta_fields() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        assert_eq!(orig.meta.machine_name, rt.meta.machine_name);
        assert_eq!(orig.meta.manufacturer, rt.meta.manufacturer);
        assert_eq!(orig.meta.configuration_hash, rt.meta.configuration_hash);
        assert_eq!(orig.meta.file_version, rt.meta.file_version);
        assert_eq!(orig.meta.export_date, rt.meta.export_date);
    }

    #[test]
    fn roundtrip_meta_extra_preserved() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        assert_eq!(orig.meta.extra, rt.meta.extra);
    }

    #[test]
    fn roundtrip_machine_fields() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        assert_eq!(orig.machine.build_plate_x, rt.machine.build_plate_x);
        assert_eq!(orig.machine.build_plate_x_unit, rt.machine.build_plate_x_unit);
        assert_eq!(orig.machine.gas_flow_direction, rt.machine.gas_flow_direction);
        assert_eq!(orig.machine.recoat_direction, rt.machine.recoat_direction);
    }

    #[test]
    fn roundtrip_train_count() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        assert_eq!(orig.optical_trains.len(), rt.optical_trains.len());
    }

    #[test]
    fn roundtrip_scanner_offsets() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        assert_eq!(
            orig.optical_trains[0].scanner.scan_head_offset_x,
            rt.optical_trains[0].scanner.scan_head_offset_x,
        );
        assert_eq!(
            orig.optical_trains[0].scanner.working_distance,
            rt.optical_trains[0].scanner.working_distance,
        );
    }

    #[test]
    fn invert_flags_absent_from_fixture_read_as_false_and_omitted_from_json() {
        // reference_config.h5 has none of the four Invert_* attributes at all.
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let s = &orig.optical_trains[0].scanner;
        assert!(!s.invert_actual_x);
        assert!(!s.invert_actual_y);
        assert!(!s.invert_commanded_x);
        assert!(!s.invert_commanded_y);

        let json = serde_json::to_value(s).unwrap();
        let obj = json.as_object().unwrap();
        assert!(!obj.contains_key("invert_actual_x"));
        assert!(!obj.contains_key("invert_actual_y"));
        assert!(!obj.contains_key("invert_commanded_x"));
        assert!(!obj.contains_key("invert_commanded_y"));
    }

    #[test]
    fn invert_flags_write_only_true_values_survive_as_real_hdf5_attributes() {
        // Not just "reads back as false" — confirms the false-valued
        // attributes are genuinely absent from the written file, not merely
        // present-and-zero (user-confirmed convention, 2026-08-21).
        let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        {
            let s = &mut config.optical_trains[0].scanner;
            s.invert_actual_x = true;
            s.invert_actual_y = false;
            s.invert_commanded_x = true;
            s.invert_commanded_y = false;
        }

        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&config).write(tmp.path()).unwrap();

        // Raw HDF5 inspection: only the two true-valued attributes exist at all.
        let file = hdf5::File::open(tmp.path()).unwrap();
        let scanner_grp = file
            .group("Machine/Optical_Trains/Optical_Train_01/Scanner")
            .unwrap();
        let names = scanner_grp.attr_names().unwrap();
        assert!(names.iter().any(|n| n == "Invert_Actual_X"));
        assert!(names.iter().any(|n| n == "Invert_Commanded_X"));
        assert!(!names.iter().any(|n| n == "Invert_Actual_Y"));
        assert!(!names.iter().any(|n| n == "Invert_Commanded_Y"));

        // Read path still returns the correct value either way.
        let rt = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();
        let s = &rt.optical_trains[0].scanner;
        assert!(s.invert_actual_x);
        assert!(!s.invert_actual_y);
        assert!(s.invert_commanded_x);
        assert!(!s.invert_commanded_y);

        // JSON output only ever shows the true ones.
        let json = serde_json::to_value(s).unwrap();
        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("invert_actual_x").unwrap(), true);
        assert_eq!(obj.get("invert_commanded_x").unwrap(), true);
        assert!(!obj.contains_key("invert_actual_y"));
        assert!(!obj.contains_key("invert_commanded_y"));
    }

    #[test]
    fn roundtrip_axis_configuration_3d() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        let t1_orig = &orig.optical_trains[0];
        let t1_rt = &rt.optical_trains[0];
        assert_eq!(t1_orig.scanner.axis_configuration, t1_rt.scanner.axis_configuration);
        assert!(t1_rt.scanner.z_axis.is_some());
        assert!(t1_rt.scanner.focus.is_none());
    }

    #[test]
    fn roundtrip_thermal_lensing() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        assert_eq!(
            orig.optical_trains[0].thermal_lensing_passed,
            rt.optical_trains[0].thermal_lensing_passed,
        );
        assert_eq!(
            orig.optical_trains[1].thermal_lensing_passed,
            rt.optical_trains[1].thermal_lensing_passed,
        );
    }

    #[test]
    fn roundtrip_clearbox_scalar_fields() {
        // reference_config_synchronous_sensors.h5, not reference_config.h5,
        // which deliberately has no sensors (SYNCHRONOUS_SENSOR_PLAN.md
        // Phase 0) — every pre-existing scalar assertion below still holds,
        // since the two fixtures are byte-identical outside the new group.
        let orig = MachineConfigReader::open(REFERENCE_SENSORS).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE_SENSORS);
        let cb_orig = orig.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
        let cb_rt = rt.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
        assert_eq!(cb_orig.ip_address, cb_rt.ip_address);
        assert_eq!(cb_orig.data_port, cb_rt.data_port);
        assert_eq!(cb_orig.show_console, cb_rt.show_console);
        assert_eq!(cb_orig.correction_grid_domain_shape, cb_rt.correction_grid_domain_shape);

        assert_eq!(cb_orig.synchronous_sensors.len(), cb_rt.synchronous_sensors.len());
        let sensor_orig = &cb_orig.synchronous_sensors["Oxygen Sensor"];
        let sensor_rt = &cb_rt.synchronous_sensors["Oxygen Sensor"];
        assert_eq!(sensor_orig.sensor_name, sensor_rt.sensor_name);
        assert_eq!(sensor_orig.port_id, sensor_rt.port_id);
        assert_eq!(sensor_orig.calibration_verified, sensor_rt.calibration_verified);
        assert_eq!(
            sensor_orig.derivation_equation_constants,
            sensor_rt.derivation_equation_constants
        );
        assert_eq!(sensor_orig.calibration_points, sensor_rt.calibration_points);
    }

    #[test]
    fn roundtrip_synchronous_sensor_compound_datasets_exact_values_in_order() {
        let rt = roundtrip(REFERENCE_SENSORS);
        let cb = rt.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
        let sensor = &cb.synchronous_sensors["Oxygen Sensor"];

        assert_eq!(
            sensor.derivation_equation_constants,
            vec![
                EquationConstant { name: "a".to_string(), value: 0.4375 },
                EquationConstant { name: "b".to_string(), value: -2.75 },
            ]
        );
        assert_eq!(
            sensor.calibration_points,
            vec![
                CalibrationPoint { input_value: 4.0, output_value: -1.0 },
                CalibrationPoint { input_value: 20.0, output_value: 6.0 },
            ]
        );
    }

    #[test]
    fn roundtrip_empty_synchronous_sensors_map_stays_empty_not_absent() {
        // reference_config.h5 has zero sensors already — the write path must
        // skip creating the Synchronous_Sensors group entirely (not write an
        // empty placeholder), and the read path must default back to an
        // empty map, not error.
        let rt = roundtrip(REFERENCE);
        let cb = rt.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
        assert!(cb.synchronous_sensors.is_empty());
    }

    #[test]
    fn roundtrip_synchronous_sensor_with_zero_row_compound_datasets() {
        // The distinct edge case from the empty-*map* test above: here the
        // sensor itself exists (its group is created), but both compound
        // datasets have zero rows — proving 0-length compound dataset
        // creation/read genuinely works, not assumed.
        let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let cb = config.optical_trains[0].optional_components.clearbox.as_mut().unwrap();
        cb.synchronous_sensors.insert(
            "Untested Sensor".to_string(),
            SynchronousSensor {
                enabled: Some(false),
                sensor_name: Some("Placeholder".to_string()),
                sensor_output_range_low: None,
                sensor_output_range_high: None,
                sensor_output_space: None,
                sensor_model: None,
                sensor_manufacturer: None,
                sensor_scope: None,
                units_derived_quantity: None,
                port_id: None,
                sensor_type: None,
                input_type: None,
                algorithm_type: None,
                algorithm_equation: None,
                calibration_source: None,
                calibration_verified: None,
                sample_period: None,
                metadata: None,
                derivation_equation_constants: vec![],
                calibration_points: vec![],
            },
        );

        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&config).write(tmp.path()).unwrap();
        let rt = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();
        let cb_rt = rt.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
        let sensor_rt = &cb_rt.synchronous_sensors["Untested Sensor"];
        assert!(sensor_rt.derivation_equation_constants.is_empty());
        assert!(sensor_rt.calibration_points.is_empty());
        assert_eq!(sensor_rt.sensor_name, Some("Placeholder".to_string()));
    }

    #[test]
    fn write_synchronous_sensor_constant_name_too_long_for_fixed64_errors() {
        // Derivation_Equation_Constants.name is a 64-byte FixedUnicode field
        // (see SYNCHRONOUS_SENSOR_PLAN.md's "Compound dataset string
        // convention"). A name whose UTF-8 encoding exceeds 64 bytes must be
        // rejected with a clear error at write time, not silently truncated.
        let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let cb = config.optical_trains[0].optional_components.clearbox.as_mut().unwrap();
        cb.synchronous_sensors.insert(
            "Oversized Name Sensor".to_string(),
            SynchronousSensor {
                enabled: None,
                sensor_name: None,
                sensor_output_range_low: None,
                sensor_output_range_high: None,
                sensor_output_space: None,
                sensor_model: None,
                sensor_manufacturer: None,
                sensor_scope: None,
                units_derived_quantity: None,
                port_id: None,
                sensor_type: None,
                input_type: None,
                algorithm_type: None,
                algorithm_equation: None,
                calibration_source: None,
                calibration_verified: None,
                sample_period: None,
                metadata: None,
                derivation_equation_constants: vec![EquationConstant {
                    name: "a".repeat(65),
                    value: 1.0,
                }],
                calibration_points: vec![],
            },
        );

        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        let err = Hdf5WriterV1_0::new(&config).write(tmp.path()).unwrap_err();
        assert!(
            matches!(err, crate::error::MachineConfigError::Parse(_)),
            "expected a Parse error for an oversized constant name, got: {err:?}"
        );
    }

    #[test]
    fn roundtrip_synchronous_sensor_arbitrary_differently_styled_key() {
        // The map key is a free-form label with no schema meaning — proves a
        // key unlike the fixture's own "Oxygen Sensor" (different style:
        // underscore-joined, all-caps) survives a write->read cycle verbatim.
        let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let cb = config.optical_trains[0].optional_components.clearbox.as_mut().unwrap();
        cb.synchronous_sensors.insert(
            "HUMIDITY_SENSOR_2".to_string(),
            SynchronousSensor {
                enabled: Some(true),
                sensor_name: None,
                sensor_output_range_low: None,
                sensor_output_range_high: None,
                sensor_output_space: None,
                sensor_model: None,
                sensor_manufacturer: None,
                sensor_scope: None,
                units_derived_quantity: None,
                port_id: Some(9),
                sensor_type: None,
                input_type: None,
                algorithm_type: None,
                algorithm_equation: None,
                calibration_source: None,
                calibration_verified: None,
                sample_period: None,
                metadata: None,
                derivation_equation_constants: vec![],
                calibration_points: vec![],
            },
        );

        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&config).write(tmp.path()).unwrap();
        let rt = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();
        let cb_rt = rt.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
        assert!(cb_rt.synchronous_sensors.contains_key("HUMIDITY_SENSOR_2"));
        assert_eq!(cb_rt.synchronous_sensors["HUMIDITY_SENSOR_2"].port_id, Some(9));
    }

    #[test]
    fn opcua_and_synchronous_sensor_coexist_through_writer() {
        // Proves the Writer side of the cross-feature guarantee (the Reader
        // side is proved directly against the pre-built combined fixture in
        // capabilities/v1_0/hdf5.rs's own test module) — parses a real
        // OPCUA-only fixture, adds a sensor purely in memory, writes, and
        // confirms both survive re-reading. See SYNCHRONOUS_SENSOR_PLAN.md's
        // Phase 0 fixture-layout decision for why both tests are kept.
        let mut config = MachineConfigReader::open(REFERENCE_OPCUA).unwrap().parse().unwrap();
        assert!(config.opcua.is_some(), "fixture must already have OPCUA before the test adds a sensor");

        let cb = config.optical_trains[0].optional_components.clearbox.as_mut().unwrap();
        cb.synchronous_sensors.insert(
            "Oxygen Sensor".to_string(),
            SynchronousSensor {
                enabled: Some(true),
                sensor_name: Some("ZR800 Oxygen Analyzer".to_string()),
                sensor_output_range_low: Some(-1.0),
                sensor_output_range_high: Some(6.0),
                sensor_output_space: Some("log10(ppm)".to_string()),
                sensor_model: Some("ZR810".to_string()),
                sensor_manufacturer: Some("Industrial Physics".to_string()),
                sensor_scope: Some("Global".to_string()),
                units_derived_quantity: Some("ppm".to_string()),
                port_id: Some(5),
                sensor_type: Some("Oxygen Sensor".to_string()),
                input_type: Some("4-20 mA".to_string()),
                algorithm_type: Some("Log-Linear".to_string()),
                algorithm_equation: Some("log(ppm) = a*mA + b".to_string()),
                calibration_source: Some("Datasheet".to_string()),
                calibration_verified: Some(false),
                sample_period: Some(5.0),
                metadata: None,
                derivation_equation_constants: vec![
                    EquationConstant { name: "a".to_string(), value: 0.4375 },
                    EquationConstant { name: "b".to_string(), value: -2.75 },
                ],
                calibration_points: vec![
                    CalibrationPoint { input_value: 4.0, output_value: -1.0 },
                    CalibrationPoint { input_value: 20.0, output_value: 6.0 },
                ],
            },
        );

        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&config).write(tmp.path()).unwrap();
        let rt = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();

        assert!(rt.opcua.is_some(), "OPCUA must survive alongside the newly-added sensor");
        let cb_rt = rt.optical_trains[0].optional_components.clearbox.as_ref().unwrap();
        let sensor_rt = &cb_rt.synchronous_sensors["Oxygen Sensor"];
        assert_eq!(sensor_rt.sensor_name, Some("ZR800 Oxygen Analyzer".to_string()));
        assert_eq!(sensor_rt.derivation_equation_constants.len(), 2);
        assert_eq!(sensor_rt.calibration_points.len(), 2);
    }

    #[test]
    fn roundtrip_sfcf_metadata() {
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        let sfcf_orig = orig.optical_trains[0].scan_field_correction_file.as_ref().unwrap();
        let sfcf_rt = rt.optical_trains[0].scan_field_correction_file.as_ref().unwrap();
        assert_eq!(sfcf_orig.document_name, sfcf_rt.document_name);
        assert_eq!(sfcf_orig.file_size, sfcf_rt.file_size);
        assert_eq!(sfcf_orig.document_id, sfcf_rt.document_id);
    }

    #[test]
    fn roundtrip_correction_data_sha256() {
        use crate::models::CorrectionData;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn cd_hash(cd: &CorrectionData) -> u64 {
            let mut h = DefaultHasher::new();
            for v in &cd.data {
                v.to_bits().hash(&mut h);
            }
            h.finish()
        }

        let reader = MachineConfigReader::open(REFERENCE).unwrap();
        let original_corr = reader.get_correction_data(0).unwrap();
        let original_inv = reader.get_inverse_correction_data(0).unwrap();

        let config = reader.parse_with_binary().unwrap();
        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&config).write(tmp.path()).unwrap();
        let rt_reader = MachineConfigReader::open(tmp.path()).unwrap();
        let rt_corr = rt_reader.get_correction_data(0).unwrap();
        let rt_inv = rt_reader.get_inverse_correction_data(0).unwrap();

        assert_eq!(
            cd_hash(&original_corr),
            cd_hash(&rt_corr),
            "Correction_Data hash changed across write roundtrip"
        );
        assert_eq!(
            cd_hash(&original_inv),
            cd_hash(&rt_inv),
            "Inverse_Correction_Data hash changed across write roundtrip"
        );
    }

    #[test]
    fn roundtrip_opcua_fields() {
        let orig = MachineConfigReader::open(REFERENCE_OPCUA).unwrap().parse().unwrap();
        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&orig).write(tmp.path()).unwrap();
        let rt = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();

        let orig_opcua = orig.opcua.as_ref().unwrap();
        let rt_opcua = rt.opcua.as_ref().unwrap();
        assert_eq!(orig_opcua.client.server_url, rt_opcua.client.server_url);
        assert_eq!(orig_opcua.client.bfs_max_depth, rt_opcua.client.bfs_max_depth);
        assert_eq!(orig_opcua.client.extra, rt_opcua.client.extra);
        assert_eq!(orig_opcua.pipe.buffer_size, rt_opcua.pipe.buffer_size);
        assert_eq!(orig_opcua.triggers_enabled, rt_opcua.triggers_enabled);
        assert_eq!(orig_opcua.triggers.len(), rt_opcua.triggers.len());

        // The 22 newly-promoted fields + the new trigger_stop_ceiling_layers field.
        assert_eq!(orig_opcua.client.keep_alive_count, rt_opcua.client.keep_alive_count);
        assert_eq!(orig_opcua.client.lifetime_count, rt_opcua.client.lifetime_count);
        assert_eq!(orig_opcua.client.machine_profile, rt_opcua.client.machine_profile);
        assert_eq!(orig_opcua.client.queue_policy, rt_opcua.client.queue_policy);
        assert_eq!(orig_opcua.client.queue_size_data_change, rt_opcua.client.queue_size_data_change);
        assert_eq!(orig_opcua.client.queue_size_events, rt_opcua.client.queue_size_events);
        assert_eq!(orig_opcua.client.reconnect_interval, rt_opcua.client.reconnect_interval);
        assert_eq!(orig_opcua.client.root_node, rt_opcua.client.root_node);
        assert_eq!(orig_opcua.client.sync_loop_interval_initial, rt_opcua.client.sync_loop_interval_initial);
        assert_eq!(orig_opcua.client.sync_loop_interval_settled, rt_opcua.client.sync_loop_interval_settled);
        assert_eq!(orig_opcua.pipe.configure_client, rt_opcua.pipe.configure_client);
        assert_eq!(orig_opcua.pipe.inbound_rate_limit, rt_opcua.pipe.inbound_rate_limit);
        assert_eq!(orig_opcua.pipe.max_inbound_message_size, rt_opcua.pipe.max_inbound_message_size);
        assert_eq!(orig_opcua.pipe.min_integrity_level, rt_opcua.pipe.min_integrity_level);
        assert_eq!(orig_opcua.pipe.pipe_name, rt_opcua.pipe.pipe_name);
        assert_eq!(orig_opcua.pipe.user_access_level, rt_opcua.pipe.user_access_level);
        assert_eq!(orig_opcua.trigger_stop_ceiling_layers, rt_opcua.trigger_stop_ceiling_layers);
        assert_eq!(orig_opcua.trigger_stop_ceiling_layers, Some(3));

        let orig_t = &orig_opcua.triggers["Laser Emission Interlock"];
        let rt_t = &rt_opcua.triggers["Laser Emission Interlock"];
        assert_eq!(orig_t.signal, rt_t.signal);
        assert_eq!(orig_t.rule_enabled, rt_t.rule_enabled);
        assert_eq!(orig_t.extra, rt_t.extra);
        assert_eq!(orig_t.case_sensitivity, rt_t.case_sensitivity);
        assert_eq!(orig_t.component, rt_t.component);
        assert_eq!(orig_t.cooldown_period, rt_t.cooldown_period);
        assert_eq!(orig_t.event, rt_t.event);
        assert_eq!(orig_t.max_fires_per_job, rt_t.max_fires_per_job);
        assert_eq!(orig_t.trigger_label, rt_t.trigger_label);
    }

    /// `trigger_stop_ceiling_layers` specifically, including the `None` case —
    /// the one field that lives directly on `OpcuaConfig` with no `extra`
    /// bucket to fall back on if the write/read pairing were wrong.
    #[test]
    fn roundtrip_trigger_stop_ceiling_layers_none_and_some() {
        let mut orig = MachineConfigReader::open(REFERENCE_OPCUA).unwrap().parse().unwrap();

        let tmp_some = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&orig).write(tmp_some.path()).unwrap();
        let rt_some = MachineConfigReader::open(tmp_some.path()).unwrap().parse().unwrap();
        assert_eq!(rt_some.opcua.unwrap().trigger_stop_ceiling_layers, Some(3));

        orig.opcua.as_mut().unwrap().trigger_stop_ceiling_layers = None;
        let tmp_none = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&orig).write(tmp_none.path()).unwrap();
        let rt_none = MachineConfigReader::open(tmp_none.path()).unwrap().parse().unwrap();
        assert_eq!(rt_none.opcua.unwrap().trigger_stop_ceiling_layers, None);
    }

    #[test]
    fn roundtrip_synthetic_2laser() {
        let orig = MachineConfigReader::open(SYNTHETIC).unwrap().parse().unwrap();
        let rt = roundtrip(SYNTHETIC);
        assert_eq!(orig.optical_trains.len(), rt.optical_trains.len());
        assert_eq!(orig.meta.machine_name, rt.meta.machine_name);
        assert_eq!(
            orig.optical_trains[0].scanner.working_distance,
            rt.optical_trains[0].scanner.working_distance,
        );
    }
}
