//! File_Version 1.1 HDF5 writer — on-disk layout, attribute names, and casting.
//!
//! Deliberately independent of `capabilities/v1_0/writer.rs` — see the
//! matching note in `v1_1/hdf5.rs`'s module docstring.
//!
//! Type conventions (mirror reader helpers in reverse):
//!   None  -> ""   (empty string — matches what the machine software writes)
//!   bool  -> 0 or 1  (HDF5 integer attribute)
//!   float -> f64
//!   int   -> i64
//!   str   -> String (h5py-equivalent VarLenUnicode)

use std::path::Path;

use hdf5::types::VarLenUnicode;
use hdf5::{Dataset, File as H5File, Group};
use ndarray::Array1;

use super::hdf5::{RawCalibrationPoint, RawEquationConstant, EQUATION_CONSTANT_NAME_MAX_BYTES};
use super::layout;
use crate::error::{MachineConfigError, Result};
use crate::models::*;
use crate::power_characterization::{
    forward_power_characterization_coefficients, forward_power_characterization_points,
};
use crate::writer::WriterAdapter;

// ---------------------------------------------------------------------------
// Write helpers — independent copies of the equivalent v1.0 helpers.
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

/// Writes an i64 attribute (`1`) only when `val` is `true`; writes nothing at
/// all when `false` — matches v1.0's `Invert_*` convention.
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

/// Opens `name` under `parent` if it already exists, otherwise creates it.
/// Used for `Extensions` (and `Extensions/ClearBox`), which may need to be
/// reached from two independent call sites (ClearBox and OPCUA) depending on
/// which features are present in a given config.
fn ensure_group(parent: &Group, name: &str) -> Result<Group> {
    match parent.group(name) {
        Ok(g) => Ok(g),
        Err(_) => Ok(parent.create_group(name)?),
    }
}

// ---------------------------------------------------------------------------
// Public writer
// ---------------------------------------------------------------------------

/// File_Version 1.1 HDF5 writer. Public [`crate::writer::MachineConfigWriter`] dispatches here.
pub struct Hdf5WriterV1_1<'a> {
    config: &'a MachineConfig,
}

impl<'a> Hdf5WriterV1_1<'a> {
    pub fn new(config: &'a MachineConfig) -> Self {
        Self { config }
    }

    /// Writes the config to `path`, creating or overwriting the file.
    ///
    /// Returns `Err(MachineConfigError::ConsolidateConflict)` if the model's
    /// ClearBox-bearing trains disagree on `Output_Path` or
    /// `Software_Trigger_Delay` — see `docs/migrations/v1_0_to_v1_1.md` Change 1's
    /// "Consolidate conflict rule".
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let f = H5File::create(path)?;
        let root = f.group("/")?;
        self.write_root_attrs(&root)?;

        let machine_grp = f.create_group("Machine")?;
        self.write_machine_attrs(&machine_grp)?;
        let trains_grp = machine_grp.create_group("Optical_Trains")?;

        self.write_optical_trains(&f, &trains_grp)?;

        if let Some(opcua) = &self.config.opcua {
            self.write_opcua(&f, opcua)?;
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Root attributes -> MachineConfigMeta
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
    // Machine group -> Machine
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
        ws(
            grp,
            "Build_Plate_Corner_Radius_unit",
            ma.build_plate_radius_unit.as_deref().unwrap_or("mm"),
        )?;
        ws(grp, "Gas_Flow_Direction", ma.gas_flow_direction.as_deref().unwrap_or(""))?;
        ws(grp, "Recoat_Direction", ma.recoat_direction.as_deref().unwrap_or(""))?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Optical trains
    // ------------------------------------------------------------------

    fn write_optical_trains(&self, f: &H5File, trains_grp: &Group) -> Result<()> {
        let trains = &self.config.optical_trains;
        let trains_with_clearbox: Vec<(String, &ClearBox)> = trains
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                t.optional_components.clearbox.as_ref().map(|cb| (layout::train_id(i), cb))
            })
            .collect();

        let clearbox_root: Option<Group> = if !trains_with_clearbox.is_empty() {
            let output_path = consolidate_field(
                &trains_with_clearbox,
                "Output_Path",
                |cb| cb.output_path.clone(),
                |v| v.clone().unwrap_or_default(),
            )?;
            let software_trigger_delay = consolidate_field(
                &trains_with_clearbox,
                "Software_Trigger_Delay",
                |cb| cb.software_trigger_delay,
                |v| v.map(|x| x.to_string()).unwrap_or_default(),
            )?;
            let extensions = ensure_group(&f.group("/")?, layout::ROOT_EXTENSIONS)?;
            let cb_root = ensure_group(&extensions, "ClearBox")?;
            ws(&cb_root, "Output_Path", &output_path.unwrap_or_default())?;
            wi(&cb_root, "Software_Trigger_Delay", software_trigger_delay)?;
            Some(cb_root)
        } else {
            None
        };

        for (i, train) in trains.iter().enumerate() {
            let tid = layout::train_id(i);
            let train_grp = trains_grp.create_group(&tid)?;
            self.write_train_attrs(&train_grp, train)?;

            self.write_scanner(&train_grp.create_group(layout::GROUP_SCANNER)?, &train.scanner)?;
            self.write_light_source(
                &train_grp.create_group(layout::GROUP_LIGHT_SOURCE)?,
                &train.light_source,
            )?;
            self.write_collimator(
                &train_grp.create_group(layout::GROUP_COLLIMATOR)?,
                &train.collimator,
            )?;
            self.write_scanner_card(
                &train_grp.create_group(layout::GROUP_SCANNER_CARD)?,
                &train.scanner_card,
            )?;

            if let Some(cb) = &train.optional_components.clearbox {
                let cb_root = clearbox_root
                    .as_ref()
                    .expect("clearbox_root must exist whenever a train has a ClearBox");
                self.write_clearbox(cb_root, &tid, cb)?;
            }

            if let Some(sfcf) = &train.scan_field_correction_file {
                self.write_sfcf(&train_grp, sfcf)?;
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // Train-level scalar attrs
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
        ws(
            grp,
            "Build_Plane_Offset_Major_unit",
            t.build_plane_offset_major_unit.as_deref().unwrap_or("mm"),
        )?;
        wf(grp, "Build_Plane_Offset_Minor", t.build_plane_offset_minor)?;
        ws(
            grp,
            "Build_Plane_Offset_Minor_unit",
            t.build_plane_offset_minor_unit.as_deref().unwrap_or("mm"),
        )?;
        wf(grp, "Collimator_Focal_Length", t.collimator_focal_length)?;
        ws(
            grp,
            "Collimator_Focal_Length_unit",
            t.collimator_focal_length_unit.as_deref().unwrap_or("mm"),
        )?;
        wf(grp, "M2_Major", t.m2_major)?;
        wf(grp, "M2_Minor", t.m2_minor)?;
        wf(grp, "Major_Axis_Angle", t.major_axis_angle)?;
        ws(grp, "Major_Axis_Angle_unit", t.major_axis_angle_unit.as_deref().unwrap_or("degrees"))?;
        wf(grp, "Rayleigh_Length_Major", t.rayleigh_length_major)?;
        ws(
            grp,
            "Rayleigh_Length_Major_unit",
            t.rayleigh_length_major_unit.as_deref().unwrap_or("mm"),
        )?;
        wf(grp, "Rayleigh_Length_Minor", t.rayleigh_length_minor)?;
        ws(
            grp,
            "Rayleigh_Length_Minor_unit",
            t.rayleigh_length_minor_unit.as_deref().unwrap_or("mm"),
        )?;
        ws(grp, "Scanner_Number", t.scanner_number.as_deref().unwrap_or(""))?;
        wb(grp, "Thermal_Lensing_Test_Passed", t.thermal_lensing_passed)?;
        wf(grp, "Thermal_Lensing_Focal_Plane_Shift", t.thermal_lensing_focal_plane_shift)?;
        ws(
            grp,
            "Thermal_Lensing_Focal_Plane_Shift_unit",
            t.thermal_lensing_focal_plane_shift_unit.as_deref().unwrap_or("mm"),
        )?;
        wf(grp, "Thermal_Lensing_Threshold", t.thermal_lensing_threshold)?;
        ws(
            grp,
            "Thermal_Lensing_Threshold_unit",
            t.thermal_lensing_threshold_unit.as_deref().unwrap_or("mm"),
        )?;
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

        self.write_axis(&grp.create_group("X_Axis")?, &s.x_axis)?;
        self.write_axis(&grp.create_group("Y_Axis")?, &s.y_axis)?;
        if let Some(z) = &s.z_axis {
            self.write_axis(&grp.create_group("Z_Axis")?, z)?;
        }
        if let Some(focus) = &s.focus {
            self.write_axis(&grp.create_group("Focus")?, focus)?;
        }
        Ok(())
    }

    /// Change 5: `Tuning_Parameters`/`Tuning_Type` are never written in
    /// v1.1, regardless of what's on the model.
    fn write_axis(&self, grp: &Group, ax: &AxisConfig) -> Result<()> {
        wi(grp, "Actual_Bit_Resolution", ax.actual_bit_resolution)?;
        ws(grp, "Actual_Bit_Resolution_unit", ax.actual_bit_resolution_unit.as_deref().unwrap_or(""))?;
        wi(grp, "Commanded_Bit_Resolution", ax.commanded_bit_resolution)?;
        ws(
            grp,
            "Commanded_Bit_Resolution_unit",
            ax.commanded_bit_resolution_unit.as_deref().unwrap_or(""),
        )?;
        ws(grp, "Control_Type", ax.control_type.as_deref().unwrap_or(""))?;
        wf(grp, "Range_Of_Motion", ax.range_of_motion)?;
        ws(grp, "Range_Of_Motion_unit", ax.range_of_motion_unit.as_deref().unwrap_or(""))?;
        ws(grp, "Smoothing_Kernel", ax.smoothing_kernel.as_deref().unwrap_or(""))?;
        wf(grp, "Smoothing_Parameters", ax.smoothing_parameters)?;
        Ok(())
    }

    /// Always writes a complete, present group — schema-complete over
    /// omitted, even when `pc` is `None` or its lists are empty. See
    /// `docs/migrations/v1_0_to_v1_1.md` Change 3's derivation rules.
    fn write_power_characterization(
        &self,
        grp: &Group,
        pc: Option<&PowerCharacterization>,
    ) -> Result<()> {
        ws(grp, "Algorithm_Type", pc.and_then(|p| p.algorithm_type.as_deref()).unwrap_or(""))?;
        ws(grp, "Algorithm_Equation", pc.and_then(|p| p.algorithm_equation.as_deref()).unwrap_or(""))?;
        ws(grp, "Input_Type", pc.and_then(|p| p.input_type.as_deref()).unwrap_or(""))?;
        ws(
            grp,
            "Units_Derived_Quantity",
            pc.and_then(|p| p.units_derived_quantity.as_deref()).unwrap_or(""),
        )?;

        // UTF-8 — same shared convention as SynchronousSensor's own
        // Derivation_Equation_Constants (see `RawEquationConstant`'s doc
        // comment in `v1_1/hdf5.rs`).
        let empty: Vec<EquationConstant> = Vec::new();
        let constants = pc.map(|p| &p.derivation_equation_constants).unwrap_or(&empty);
        let const_rows: Vec<RawEquationConstant> = constants
            .iter()
            .map(|c| -> Result<RawEquationConstant> {
                Ok(RawEquationConstant {
                    name: c.name.parse().map_err(|e| {
                        MachineConfigError::Parse(format!(
                            "Derivation_Equation_Constants name {:?} does not fit in a \
                             {EQUATION_CONSTANT_NAME_MAX_BYTES}-byte FixedUnicode field: {e:?}",
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
            .create(layout::DS_DERIVATION_EQUATION_CONSTANTS)?;

        let empty_points: Vec<CalibrationPoint> = Vec::new();
        let points = pc.map(|p| &p.characterization_points).unwrap_or(&empty_points);
        let point_rows: Vec<RawCalibrationPoint> = points
            .iter()
            .map(|p| RawCalibrationPoint { input_value: p.input_value, output_value: p.output_value })
            .collect();
        let point_arr = Array1::from(point_rows);
        grp.new_dataset_builder()
            .with_data(&point_arr)
            .create(layout::DS_CHARACTERIZATION_POINTS)?;

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
        // Power_Bit_Resolution is always stored as a string in real HDF5 files.
        let pbr_str = ls.power_bit_resolution.map(|v| v.to_string()).unwrap_or_default();
        ws(grp, "Power_Bit_Resolution", &pbr_str)?;
        ws(grp, "Power_Bit_Resolution_unit", ls.power_bit_resolution_unit.as_deref().unwrap_or("bits"))?;
        // Change 4: Watts_To_Volts_Algorithm/Params are not written in v1.1 —
        // superseded by Power_Characterization.
        //
        // Phase 2 (V1_1_IMPLEMENTATION_PLAN.md): write the native
        // power_characterization if present; only derive from the flat
        // fields as a fallback when there's nothing native to write (e.g. a
        // v1.0-sourced model). Never the other way around — a present
        // native value always wins.
        let derived_pc;
        let pc = match ls.power_characterization.as_ref() {
            Some(pc) => Some(pc),
            None => {
                derived_pc = forward_power_characterization_points(
                    ls.watts_to_volts_algorithm.as_deref(),
                    ls.watts_to_volts_params.as_deref(),
                )?;
                derived_pc.as_ref()
            }
        };
        self.write_power_characterization(
            &grp.create_group(layout::GROUP_POWER_CHARACTERIZATION)?,
            pc,
        )?;
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

    /// Writes only the per-train subset — `Output_Path`/`Software_Trigger_Delay`
    /// are written once, at `Extensions/ClearBox/` itself, by
    /// [`Self::write_optical_trains`] before this is called.
    fn write_clearbox(&self, cb_root: &Group, tid: &str, cb: &ClearBox) -> Result<()> {
        let grp = cb_root.create_group(tid)?;
        ws(&grp, "Ip_Address", &cb.ip_address)?;
        ws(&grp, "Serial_Number", cb.serial_number.as_deref().unwrap_or(""))?;
        wi(&grp, "Data_Port", cb.data_port)?;
        wi(&grp, "Server_Port", cb.server_port)?;
        wi(&grp, "Actual_Timing_Offset", cb.actual_timing_offset)?;
        wi(&grp, "Commanded_Timing_Offset", cb.commanded_timing_offset)?;
        ws(&grp, "Manufacturer", cb.manufacturer.as_deref().unwrap_or(""))?;
        ws(&grp, "Model", cb.model.as_deref().unwrap_or(""))?;
        ws(&grp, "Firmware_Version", cb.firmware_version.as_deref().unwrap_or(""))?;

        let cd_arr = nested_to_array3(&cb.correction_data);
        let shape = cd_arr.shape().to_owned();
        let cd_ds = grp.new_dataset_builder().with_data(&cd_arr).create(layout::DS_CORRECTION_DATA)?;
        ws_ds(&cd_ds, "dimensions", "H,W,D")?;
        ws_ds(&cd_ds, "dtype", "float64")?;
        ws_ds(&cd_ds, "shape", &format!("{}x{}x{}", shape[0], shape[1], shape[2]))?;

        let icd_arr = nested_to_array3(&cb.inverse_correction_data);
        let icd_shape = icd_arr.shape().to_owned();
        let icd_ds = grp
            .new_dataset_builder()
            .with_data(&icd_arr)
            .create(layout::DS_INVERSE_CORRECTION_DATA)?;
        ws_ds(&icd_ds, "dimensions", "H,W,D")?;
        ws_ds(&icd_ds, "dtype", "float64")?;
        ws_ds(&icd_ds, "shape", &format!("{}x{}x{}", icd_shape[0], icd_shape[1], icd_shape[2]))?;

        if !cb.synchronous_sensors.is_empty() {
            let sensors_grp = grp.create_group("Synchronous_Sensors")?;
            for (name, sensor) in &cb.synchronous_sensors {
                self.write_synchronous_sensor(&sensors_grp.create_group(name)?, sensor)?;
            }
        }

        // Phase 2 (V1_1_IMPLEMENTATION_PLAN.md): write the native
        // power_characterization if present; only derive from the flat
        // fields as a fallback when there's nothing native to write (e.g. a
        // v1.0-sourced model). Never the other way around — a present
        // native value always wins.
        let derived_pc;
        let pc = match cb.power_characterization.as_ref() {
            Some(pc) => Some(pc),
            None => {
                derived_pc = forward_power_characterization_coefficients(
                    cb.volts_to_watts_algorithm.as_deref(),
                    cb.volts_to_watts_params.as_deref(),
                )?;
                derived_pc.as_ref()
            }
        };
        self.write_power_characterization(
            &grp.create_group(layout::GROUP_POWER_CHARACTERIZATION)?,
            pc,
        )?;

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

        // UTF-8, not ASCII — matches v1.0's own established convention for
        // this group (see `RawEquationConstant`'s doc comment in
        // `v1_1/hdf5.rs`); distinct from Power_Characterization's ASCII
        // convention above.
        let const_rows: Vec<RawEquationConstant> = sensor
            .derivation_equation_constants
            .iter()
            .map(|c| -> Result<RawEquationConstant> {
                Ok(RawEquationConstant {
                    name: c.name.parse().map_err(|e| {
                        MachineConfigError::Parse(format!(
                            "Derivation_Equation_Constants name {:?} does not fit in a \
                             {EQUATION_CONSTANT_NAME_MAX_BYTES}-byte FixedUnicode field: {e:?}",
                            c.name
                        ))
                    })?,
                    value: c.value,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let const_arr = Array1::from(const_rows);
        grp.new_dataset_builder().with_data(&const_arr).create("Derivation_Equation_Constants")?;

        let point_rows: Vec<RawCalibrationPoint> = sensor
            .calibration_points
            .iter()
            .map(|p| RawCalibrationPoint { input_value: p.input_value, output_value: p.output_value })
            .collect();
        let point_arr = Array1::from(point_rows);
        grp.new_dataset_builder().with_data(&point_arr).create("Calibration_Points")?;

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
            .create(layout::DS_SCAN_FIELD_CORRECTION_FILE)?;
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
    // OPCUA (Change 2: relocated to Extensions/TM_OPCUA)
    // ------------------------------------------------------------------

    fn write_opcua(&self, f: &H5File, opcua: &OpcuaConfig) -> Result<()> {
        let extensions = ensure_group(&f.group("/")?, layout::ROOT_EXTENSIONS)?;
        let opcua_grp = extensions.create_group("TM_OPCUA")?;

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

        let triggers_grp = opcua_grp.create_group("Triggers")?;
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

/// Change 1 (Consolidate): collect `field` from every ClearBox-bearing train
/// (via `getter`); hard error (formatted via `fmt`) if they disagree. See
/// `docs/migrations/v1_0_to_v1_1.md` Change 1's "Consolidate conflict rule" for
/// the exact message format.
fn consolidate_field<T: Clone + PartialEq>(
    trains_with_clearbox: &[(String, &ClearBox)],
    field_name: &'static str,
    getter: impl Fn(&ClearBox) -> T,
    fmt: impl Fn(&T) -> String,
) -> Result<T> {
    let values: Vec<(String, T)> =
        trains_with_clearbox.iter().map(|(tid, cb)| (tid.clone(), getter(cb))).collect();
    let first = values[0].1.clone();
    if values.iter().all(|(_, v)| *v == first) {
        return Ok(first);
    }
    let detail =
        values.iter().map(|(tid, v)| format!("{tid}={}", fmt(v))).collect::<Vec<_>>().join(", ");
    Err(MachineConfigError::ConsolidateConflict { field: field_name, detail })
}

impl<'a> WriterAdapter for Hdf5WriterV1_1<'a> {
    fn write(&self, path: &Path) -> Result<()> {
        self.write(path)
    }
}
