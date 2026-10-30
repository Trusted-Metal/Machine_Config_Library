// Phase 3.6 — HDF5 writer: serialises MachineConfig back to a machine-config .h5 file.
// Exact inverse of reader.rs. See §3.10 (HDF5 path reference) and §3.12 (writer rules).
// Python reference: python/src/machine_config/writer.py

use std::path::Path;

use hdf5::types::VarLenUnicode;
use hdf5::{Dataset, File as H5File, Group};
use ndarray::Array1;

use super::layout;
use crate::clearbox_legacy;
use crate::error::{MachineConfigError, Result};
use crate::extensions::{validate_extension_trees, write_extension_tree};
use crate::models::*;
use crate::power_characterization::backward_flat_fields_points;
use crate::writer::WriterAdapter;
use mcf_extension_contract::ExtensionTree;

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
        self.validate_extensions()?;
        let clearbox = self.clearbox_groups()?;
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

            if let Some(tree) = clearbox.get(&tid) {
                let opt_grp = train_grp.create_group(layout::GROUP_OPTIONAL_COMPONENTS)?;
                write_extension_tree(&opt_grp, layout::GROUP_CLEARBOX, tree)?;
            }

            if let Some(sfcf) = &train.scan_field_correction_file {
                self.write_sfcf(&train_grp, sfcf)?;
            }
        }

        // v1.0 stores the TM_OPCUA extension at the root as `OPCUA/`.
        if let Some(tree) = self.config.extensions.get(layout::LEGACY_OPCUA_EXTENSION_NAME) {
            write_extension_tree(&root, layout::ROOT_OPCUA, tree)?;
        }
        Ok(())
    }

    /// Runs before the output file is created, so a rejected config never leaves a partial file.
    fn validate_extensions(&self) -> Result<()> {
        validate_extension_trees(&self.config.extensions)?;
        for name in self.config.extensions.keys() {
            if name != layout::LEGACY_OPCUA_EXTENSION_NAME && name != clearbox_legacy::EXTENSION_NAME {
                return Err(MachineConfigError::InvalidExtension(format!(
                    "File_Version 1.0 has no Extensions group; cannot write extension \"{name}\""
                )));
            }
        }
        Ok(())
    }

    /// The ClearBox extension split into one v1.0 group per optical train, keyed by train id.
    fn clearbox_groups(&self) -> Result<std::collections::BTreeMap<String, ExtensionTree>> {
        let Some(tree) = self.config.extensions.get(clearbox_legacy::EXTENSION_NAME) else {
            return Ok(Default::default());
        };
        let groups = clearbox_legacy::to_v1_0_trains(tree)?;
        let known: Vec<String> = (0..self.config.optical_trains.len()).map(layout::train_id).collect();
        if let Some(id) = groups.keys().find(|id| !known.contains(id)) {
            return Err(MachineConfigError::InvalidExtension(format!(
                "ClearBox: no optical train \"{id}\" in this configuration"
            )));
        }
        Ok(groups)
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
        ws(grp, "Build_Plate_X_Dimension_unit", ma.build_plate_x_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Build_Plate_Y_Dimension", ma.build_plate_y)?;
        ws(grp, "Build_Plate_Y_Dimension_unit", ma.build_plate_y_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Build_Plate_Z_Dimension", ma.build_plate_z)?;
        ws(grp, "Build_Plate_Z_Dimension_unit", ma.build_plate_z_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Build_Plate_Corner_Radius", ma.build_plate_radius)?;
        ws(grp, "Build_Plate_Corner_Radius_unit", ma.build_plate_radius_unit.as_deref().unwrap_or(""))?;
        ws(grp, "Gas_Flow_Direction", ma.gas_flow_direction.as_deref().unwrap_or(""))?;
        ws(grp, "Recoat_Direction", ma.recoat_direction.as_deref().unwrap_or(""))?;
        ws(grp, "Recoater_Blade_Type", ma.recoater_blade_type.as_deref().unwrap_or(""))?;
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
        ws(grp, "Beam_Waist_Major_unit", t.beam_waist_major_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Beam_Waist_Minor", t.beam_waist_minor)?;
        ws(grp, "Beam_Waist_Minor_unit", t.beam_waist_minor_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Beam_Waist_Offset_Z", t.beam_waist_offset_z)?;
        ws(grp, "Beam_Waist_Offset_Z_unit", t.beam_waist_offset_z_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Build_Plane_Offset_Major", t.build_plane_offset_major)?;
        ws(grp, "Build_Plane_Offset_Major_unit", t.build_plane_offset_major_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Build_Plane_Offset_Minor", t.build_plane_offset_minor)?;
        ws(grp, "Build_Plane_Offset_Minor_unit", t.build_plane_offset_minor_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Collimator_Focal_Length", t.collimator_focal_length)?;
        ws(grp, "Collimator_Focal_Length_unit", t.collimator_focal_length_unit.as_deref().unwrap_or(""))?;
        wf(grp, "M2_Major", t.m2_major)?;
        wf(grp, "M2_Minor", t.m2_minor)?;
        wf(grp, "Major_Axis_Angle", t.major_axis_angle)?;
        ws(grp, "Major_Axis_Angle_unit", t.major_axis_angle_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Rayleigh_Length_Major", t.rayleigh_length_major)?;
        ws(grp, "Rayleigh_Length_Major_unit", t.rayleigh_length_major_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Rayleigh_Length_Minor", t.rayleigh_length_minor)?;
        ws(grp, "Rayleigh_Length_Minor_unit", t.rayleigh_length_minor_unit.as_deref().unwrap_or(""))?;
        ws(grp, "Scanner_Number", t.scanner_number.as_deref().unwrap_or(""))?;
        wb(grp, "Thermal_Lensing_Test_Passed", t.thermal_lensing_passed)?;
        wf(grp, "Thermal_Lensing_Focal_Plane_Shift", t.thermal_lensing_focal_plane_shift)?;
        ws(grp, "Thermal_Lensing_Focal_Plane_Shift_unit", t.thermal_lensing_focal_plane_shift_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Thermal_Lensing_Threshold", t.thermal_lensing_threshold)?;
        ws(grp, "Thermal_Lensing_Threshold_unit", t.thermal_lensing_threshold_unit.as_deref().unwrap_or(""))?;
        Ok(())
    }

    fn write_scanner(&self, grp: &Group, s: &Scanner) -> Result<()> {
        ws(grp, "Manufacturer", &s.manufacturer)?;
        ws(grp, "Model", &s.model)?;
        ws(grp, "Serial_Number", &s.serial_number)?;
        wf(grp, "Working_Distance", s.working_distance)?;
        ws(grp, "Working_Distance_unit", s.working_distance_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Scan_Field_Size_X", s.scan_field_x)?;
        ws(grp, "Scan_Field_Size_X_unit", s.scan_field_x_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Scan_Field_Size_Y", s.scan_field_y)?;
        ws(grp, "Scan_Field_Size_Y_unit", s.scan_field_y_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Scan_Field_Size_Z", s.scan_field_z)?;
        ws(grp, "Scan_Field_Size_Z_unit", s.scan_field_z_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Scan_Head_Offset_X", s.scan_head_offset_x)?;
        ws(grp, "Scan_Head_Offset_X_unit", s.scan_head_offset_x_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Scan_Head_Offset_Y", s.scan_head_offset_y)?;
        ws(grp, "Scan_Head_Offset_Y_unit", s.scan_head_offset_y_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Scan_Head_Offset_Z", s.scan_head_offset_z)?;
        ws(grp, "Scan_Head_Offset_Z_unit", s.scan_head_offset_z_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Scan_Head_Rotation", s.scan_head_rotation)?;
        ws(grp, "Scan_Head_Rotation_unit", s.scan_head_rotation_unit.as_deref().unwrap_or(""))?;
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
        ws(grp, "Light_Wavelength_unit", ls.wavelength_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Power_Max_Nominal", ls.power_max_nominal)?;
        ws(grp, "Power_Max_Nominal_unit", ls.power_max_nominal_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Power_Max_Actual", ls.power_max_actual)?;
        ws(grp, "Power_Max_Actual_unit", ls.power_max_actual_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Power_Min_Actual", ls.power_min_actual)?;
        ws(grp, "Power_Min_Actual_unit", ls.power_min_actual_unit.as_deref().unwrap_or(""))?;
        wf(grp, "Power_Min_Nominal", ls.power_min_nominal)?;
        ws(grp, "Power_Min_Nominal_unit", ls.power_min_nominal_unit.as_deref().unwrap_or(""))?;
        // Power_Bit_Resolution is always stored as a string in real HDF5 files — see §3.11.
        let pbr_str = ls.power_bit_resolution.map(|v| v.to_string()).unwrap_or_default();
        ws(grp, "Power_Bit_Resolution", &pbr_str)?;
        ws(grp, "Power_Bit_Resolution_unit", ls.power_bit_resolution_unit.as_deref().unwrap_or(""))?;
        // power_characterization is the only StableModel representation of
        // this concept now (POWER_CHARACTERIZATION_UNIFICATION_PLAN.md) —
        // always derive the flat v1.0 shape from it, unconditionally.
        let (algorithm, params) = backward_flat_fields_points(ls.power_characterization.as_ref());
        ws(grp, "Watts_To_Volts_Algorithm", algorithm.as_deref().unwrap_or(""))?;
        ws(grp, "Watts_To_Volts_Params", params.as_deref().unwrap_or(""))?;
        Ok(())
    }

    fn write_collimator(&self, grp: &Group, c: &Collimator) -> Result<()> {
        ws(grp, "Manufacturer", &c.manufacturer)?;
        ws(grp, "Model", &c.model)?;
        ws(grp, "Serial_Number", &c.serial_number)?;
        wf(grp, "Focal_Length", c.focal_length)?;
        ws(grp, "Focal_Length_unit", c.focal_length_unit.as_deref().unwrap_or(""))?;
        Ok(())
    }

    fn write_scanner_card(&self, grp: &Group, sc: &ScannerCard) -> Result<()> {
        ws(grp, "Manufacturer", &sc.manufacturer)?;
        ws(grp, "Model", &sc.model)?;
        ws(grp, "Serial_Number", &sc.serial_number)?;
        ws(grp, "Communication_Protocol", sc.communication_protocol.as_deref().unwrap_or(""))?;
        wf(grp, "Sample_Period", sc.sample_period)?;
        ws(grp, "Sample_Period_unit", sc.sample_period_unit.as_deref().unwrap_or(""))?;
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

}

// v1.0's implementation of the WriterAdapter contract — this impl is
// necessarily version-specific (it's what crate::writer::MachineConfigWriter
// dispatches TO for File_Version "1.0", per DISPATCH_REGISTRY_PLAN.md's
// registry in writer.rs). MachineConfigWriter itself stays version-agnostic:
// it only ever touches `Box<dyn WriterAdapter>`, never `Hdf5WriterV1_0` by
// name. A future v1.1 gets its own separate `impl WriterAdapter for
// Hdf5WriterV1_1`, registered alongside this one. Delegates to the inherent
// `write<P: AsRef<Path>>` above — Rust always prefers an inherent method over
// a trait method of the same name on the same type (here `path: &Path`
// satisfies `P: AsRef<Path>` via the reflexive impl), so this can't recurse.
impl<'a> WriterAdapter for Hdf5WriterV1_0<'a> {
    fn write(&self, path: &Path) -> Result<()> {
        self.write(path)
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
        assert_eq!(orig.machine.recoater_blade_type, rt.machine.recoater_blade_type);
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

    fn clearbox_train(config: &MachineConfig) -> &ExtensionTree {
        &config.extensions["ClearBox"].children["Optical_Train_01"]
    }

    fn sensor_tree(constants: Vec<(String, f64)>) -> ExtensionTree {
        use mcf_extension_contract::{ColumnData, Dtype, Table, TableField};
        let mut tree = ExtensionTree::default();
        tree.tables.insert(
            "Derivation_Equation_Constants".into(),
            Table {
                fields: vec![TableField::utf8("name", 64), TableField::numeric("value", Dtype::F64)],
                rows: constants.len(),
                columns: vec![
                    ColumnData::Utf8(constants.iter().map(|c| c.0.clone()).collect()),
                    ColumnData::F64(constants.iter().map(|c| c.1).collect()),
                ],
                attrs: Default::default(),
            },
        );
        tree
    }

    fn add_sensor(config: &mut MachineConfig, label: &str, sensor: ExtensionTree) {
        config
            .extensions
            .get_mut("ClearBox")
            .unwrap()
            .children
            .get_mut("Optical_Train_01")
            .unwrap()
            .children
            .entry("Synchronous_Sensors".into())
            .or_default()
            .children
            .insert(label.into(), sensor);
    }

    #[test]
    fn roundtrip_clearbox_extension_tree() {
        let orig = MachineConfigReader::open(REFERENCE_SENSORS).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE_SENSORS);
        assert_eq!(
            crate::canonical_digest(&orig.extensions["ClearBox"]).unwrap(),
            crate::canonical_digest(&rt.extensions["ClearBox"]).unwrap()
        );
        assert_eq!(
            clearbox_train(&orig).attrs["Show_Console"],
            clearbox_train(&rt).attrs["Show_Console"],
            "v1.0-only attributes ride along as extra per-train attributes"
        );
    }

    #[test]
    fn roundtrip_synchronous_sensor_compound_datasets_exact_values_in_order() {
        use mcf_extension_contract::ColumnData;
        let rt = roundtrip(REFERENCE_SENSORS);
        let sensor = &clearbox_train(&rt).children["Synchronous_Sensors"].children["Oxygen Sensor"];
        let constants = &sensor.tables["Derivation_Equation_Constants"];
        assert_eq!(constants.columns[0], ColumnData::Utf8(vec!["a".into(), "b".into()]));
        assert_eq!(constants.columns[1], ColumnData::F64(vec![0.4375, -2.75]));
        let points = &sensor.tables["Calibration_Points"];
        assert_eq!(points.columns[0], ColumnData::F64(vec![4.0, 20.0]));
        assert_eq!(points.columns[1], ColumnData::F64(vec![-1.0, 6.0]));
    }

    #[test]
    fn roundtrip_empty_synchronous_sensors_map_stays_absent() {
        let rt = roundtrip(REFERENCE);
        assert!(!clearbox_train(&rt).children.contains_key("Synchronous_Sensors"));
    }

    #[test]
    fn roundtrip_synchronous_sensor_with_zero_row_compound_datasets() {
        let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        add_sensor(&mut config, "Untested Sensor", sensor_tree(vec![]));

        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&config).write(tmp.path()).unwrap();
        let rt = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();
        let sensor = &clearbox_train(&rt).children["Synchronous_Sensors"].children["Untested Sensor"];
        assert_eq!(sensor.tables["Derivation_Equation_Constants"].rows, 0);
    }

    #[test]
    fn write_synchronous_sensor_constant_name_too_long_for_fixed64_errors() {
        // The 64-byte name field must reject an over-long name, not truncate it.
        let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        add_sensor(&mut config, "Oversized Name Sensor", sensor_tree(vec![("a".repeat(65), 1.0)]));

        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        let err = Hdf5WriterV1_0::new(&config).write(tmp.path()).unwrap_err();
        assert!(
            matches!(err, crate::error::MachineConfigError::InvalidExtension(_)),
            "expected an InvalidExtension error for an oversized constant name, got: {err:?}"
        );
    }

    #[test]
    fn roundtrip_synchronous_sensor_arbitrary_differently_styled_key() {
        let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let mut sensor = ExtensionTree::default();
        sensor.attrs.insert("Port_ID".into(), mcf_extension_contract::Attr::scalar(mcf_extension_contract::Scalar::I64(9)));
        add_sensor(&mut config, "HUMIDITY_SENSOR_2", sensor);

        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&config).write(tmp.path()).unwrap();
        let rt = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();
        let sensors = &clearbox_train(&rt).children["Synchronous_Sensors"];
        assert_eq!(sensors.children["HUMIDITY_SENSOR_2"].attrs["Port_ID"].as_i64(), Some(9));
    }

    #[test]
    fn opcua_and_synchronous_sensor_coexist_through_writer() {
        let mut config = MachineConfigReader::open(REFERENCE_OPCUA).unwrap().parse().unwrap();
        assert!(config.extensions.contains_key("TM_OPCUA"));
        add_sensor(&mut config, "Oxygen Sensor", sensor_tree(vec![("a".into(), 0.4375)]));

        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&config).write(tmp.path()).unwrap();
        let rt = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();

        assert!(rt.extensions.contains_key("TM_OPCUA"), "OPCUA must survive alongside ClearBox");
        let sensor = &clearbox_train(&rt).children["Synchronous_Sensors"].children["Oxygen Sensor"];
        assert_eq!(sensor.tables["Derivation_Equation_Constants"].rows, 1);
    }

    #[test]
    fn write_rejects_a_clearbox_train_with_no_matching_optical_train() {
        let mut config = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let tree = config.extensions.get_mut("ClearBox").unwrap();
        let extra = tree.children["Optical_Train_01"].clone();
        tree.children.insert("Optical_Train_09".into(), extra);

        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        let err = Hdf5WriterV1_0::new(&config).write(tmp.path()).unwrap_err();
        assert!(matches!(err, crate::error::MachineConfigError::InvalidExtension(_)));
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
    fn roundtrip_correction_grids_are_bit_identical() {
        use mcf_extension_contract::DatasetData;
        let orig = MachineConfigReader::open(REFERENCE).unwrap().parse().unwrap();
        let rt = roundtrip(REFERENCE);
        for name in ["Correction_Data", "Inverse_Correction_Data"] {
            let bits = |c: &MachineConfig| match &clearbox_train(c).datasets[name].data {
                DatasetData::F64(v) => v.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                _ => panic!("{name} is not float64"),
            };
            assert_eq!(bits(&orig), bits(&rt), "{name} changed across write roundtrip");
        }
    }

    #[test]
    fn roundtrip_opcua_extension_tree() {
        let orig = MachineConfigReader::open(REFERENCE_OPCUA).unwrap().parse().unwrap();
        let tmp = NamedTempFile::with_suffix(".h5").unwrap();
        Hdf5WriterV1_0::new(&orig).write(tmp.path()).unwrap();
        let rt = MachineConfigReader::open(tmp.path()).unwrap().parse().unwrap();

        let (a, b) = (&orig.extensions["TM_OPCUA"], &rt.extensions["TM_OPCUA"]);
        assert_eq!(crate::canonical_digest(a).unwrap(), crate::canonical_digest(b).unwrap());
        // Dtypes survive too: the flag is float64 on disk in this layout.
        assert_eq!(
            a.children["Triggers"].attrs["Triggers_Enabled"],
            b.children["Triggers"].attrs["Triggers_Enabled"]
        );
        assert_eq!(b.children["Triggers"].attrs["Trigger_Stop_Ceiling_Layers"].as_i64(), Some(3));
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
