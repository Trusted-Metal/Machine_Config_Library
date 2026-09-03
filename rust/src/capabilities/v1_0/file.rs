//! File_Version 1.0 stable model facade.

use crate::builder::MockConfigBuilder;
use crate::capabilities::errors::CapabilityError;
use crate::capabilities::generated::SetMode;
use crate::capabilities::merge::apply_set_mode;
use crate::capabilities::result::Result;
use crate::models::{
    nested_to_array3, ClearBox, Collimator, CorrectionData, LightSource, Machine, MachineConfig,
    MachineConfigMeta, OpcuaConfig, OpticalTrain, Scanner, ScannerCard,
};
use super::hdf5::Hdf5AdapterV1_0;
use crate::writer::MachineConfigWriter;
use std::path::{Path, PathBuf};

pub struct MachineConfigFileV1_0 {
    config: MachineConfig,
    path: Option<PathBuf>,
    file_version: String,
    closed: bool,
}

impl MachineConfigFileV1_0 {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CapabilityError> {
        let path = path.as_ref().to_path_buf();
        let reader =
            Hdf5AdapterV1_0::open(&path).map_err(|e| CapabilityError::IoError(e.to_string()))?;
        let config = reader
            .parse_with_binary()
            .map_err(|e| CapabilityError::IoError(e.to_string()))?;
        let fv = {
            let v = config.meta.file_version.trim();
            if v.is_empty() {
                "1.0".to_string()
            } else {
                v.to_string()
            }
        };
        if fv != "1.0" {
            return Err(CapabilityError::UnsupportedVersion(format!(
                "No adapter for File_Version \"{fv}\" (v1.0 facade)"
            )));
        }
        Ok(Self {
            config,
            path: Some(path),
            file_version: fv,
            closed: false,
        })
    }

    pub fn create(version: &str) -> Result<Self, CapabilityError> {
        let fv = version.trim();
        let fv = if fv.is_empty() { "1.0" } else { fv };
        if fv != "1.0" {
            return Err(CapabilityError::UnsupportedVersion(format!(
                "create() unsupported for File_Version \"{fv}\""
            )));
        }
        let mut config = MockConfigBuilder::new(1).build();
        config.meta.file_version = "1.0".to_owned();
        Ok(Self {
            config,
            path: None,
            file_version: "1.0".to_owned(),
            closed: false,
        })
    }

    fn assert_open(&self) -> Result<(), CapabilityError> {
        if self.closed {
            Err(CapabilityError::Closed)
        } else {
            Ok(())
        }
    }

    pub fn file_version(&self) -> &str {
        &self.file_version
    }

    pub fn get_meta(&self) -> Result<MachineConfigMeta, CapabilityError> {
        self.assert_open()?;
        Ok(self.config.meta.clone())
    }

    pub fn set_meta(
        &mut self,
        model: MachineConfigMeta,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.assert_open()?;
        self.config.meta = apply_set_mode(&self.config.meta, &model, mode)?;
        Ok(())
    }

    pub fn get_machine(&self) -> Result<Machine, CapabilityError> {
        self.assert_open()?;
        Ok(self.config.machine.clone())
    }

    pub fn set_machine(&mut self, model: Machine, mode: SetMode) -> Result<(), CapabilityError> {
        self.assert_open()?;
        self.config.machine = apply_set_mode(&self.config.machine, &model, mode)?;
        Ok(())
    }

    pub fn optical_train_count(&self) -> Result<usize, CapabilityError> {
        self.assert_open()?;
        Ok(self.config.optical_trains.len())
    }

    pub fn get_train(&self, index: usize) -> Result<OpticalTrain, CapabilityError> {
        self.assert_open()?;
        self.config
            .optical_trains
            .get(index)
            .cloned()
            .ok_or_else(|| {
                CapabilityError::InvalidIndex(format!(
                    "optical train index {index} out of range [0, {})",
                    self.config.optical_trains.len()
                ))
            })
    }

    pub fn set_train(
        &mut self,
        index: usize,
        model: OpticalTrain,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.assert_open()?;
        let n = self.config.optical_trains.len();
        if index >= n {
            return Err(CapabilityError::InvalidIndex(format!(
                "optical train index {index} out of range [0, {n})"
            )));
        }
        self.config.optical_trains[index] =
            apply_set_mode(&self.config.optical_trains[index], &model, mode)?;
        Ok(())
    }

    pub fn get_scanner(&self, index: usize) -> Result<Scanner, CapabilityError> {
        Ok(self.get_train(index)?.scanner)
    }

    pub fn set_scanner(
        &mut self,
        index: usize,
        model: Scanner,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.assert_open()?;
        let n = self.config.optical_trains.len();
        if index >= n {
            return Err(CapabilityError::InvalidIndex(format!(
                "optical train index {index} out of range [0, {n})"
            )));
        }
        self.config.optical_trains[index].scanner =
            apply_set_mode(&self.config.optical_trains[index].scanner, &model, mode)?;
        Ok(())
    }

    pub fn get_light_source(&self, index: usize) -> Result<LightSource, CapabilityError> {
        Ok(self.get_train(index)?.light_source)
    }

    pub fn set_light_source(
        &mut self,
        index: usize,
        model: LightSource,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.assert_open()?;
        let n = self.config.optical_trains.len();
        if index >= n {
            return Err(CapabilityError::InvalidIndex(format!(
                "optical train index {index} out of range [0, {n})"
            )));
        }
        self.config.optical_trains[index].light_source = apply_set_mode(
            &self.config.optical_trains[index].light_source,
            &model,
            mode,
        )?;
        Ok(())
    }

    pub fn get_collimator(&self, index: usize) -> Result<Collimator, CapabilityError> {
        Ok(self.get_train(index)?.collimator)
    }

    pub fn set_collimator(
        &mut self,
        index: usize,
        model: Collimator,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.assert_open()?;
        let n = self.config.optical_trains.len();
        if index >= n {
            return Err(CapabilityError::InvalidIndex(format!(
                "optical train index {index} out of range [0, {n})"
            )));
        }
        self.config.optical_trains[index].collimator = apply_set_mode(
            &self.config.optical_trains[index].collimator,
            &model,
            mode,
        )?;
        Ok(())
    }

    pub fn get_scanner_card(&self, index: usize) -> Result<ScannerCard, CapabilityError> {
        Ok(self.get_train(index)?.scanner_card)
    }

    pub fn set_scanner_card(
        &mut self,
        index: usize,
        model: ScannerCard,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.assert_open()?;
        let n = self.config.optical_trains.len();
        if index >= n {
            return Err(CapabilityError::InvalidIndex(format!(
                "optical train index {index} out of range [0, {n})"
            )));
        }
        self.config.optical_trains[index].scanner_card = apply_set_mode(
            &self.config.optical_trains[index].scanner_card,
            &model,
            mode,
        )?;
        Ok(())
    }

    /// Returns Ok(None) when no ClearBox is installed on the train.
    pub fn get_clearbox(&self, index: usize) -> Result<Option<ClearBox>, CapabilityError> {
        Ok(self.get_train(index)?.optional_components.clearbox)
    }

    pub fn set_clearbox(
        &mut self,
        index: usize,
        model: ClearBox,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.assert_open()?;
        let n = self.config.optical_trains.len();
        if index >= n {
            return Err(CapabilityError::InvalidIndex(format!(
                "optical train index {index} out of range [0, {n})"
            )));
        }
        let current = self.config.optical_trains[index]
            .optional_components
            .clearbox
            .clone()
            .ok_or_else(|| CapabilityError::NotPresent("clearbox is not present".into()))?;
        self.config.optical_trains[index]
            .optional_components
            .clearbox = Some(apply_set_mode(&current, &model, mode)?);
        Ok(())
    }

    /// Returns the `(257, 257, 2)` float64 correction grid for optical train
    /// `index` as a version-agnostic [`CorrectionData`] — same type and shape
    /// as [`crate::reader::MachineConfigReader::get_correction_data`].
    /// Converts the already-loaded in-memory `ClearBox` model rather than
    /// re-reading the file, so this works for both `open()`- and
    /// `create()`-based instances alike. Errors with `NotPresent` if the
    /// train has no ClearBox installed.
    pub fn get_correction_data(&self, index: usize) -> Result<CorrectionData, CapabilityError> {
        let cb = self.get_clearbox(index)?.ok_or_else(|| {
            CapabilityError::NotPresent("clearbox is not present".into())
        })?;
        Ok(grid_to_correction_data(&cb.correction_data))
    }

    /// Returns the `(257, 257, 2)` float64 *inverse* correction grid for
    /// optical train `index`. See [`Self::get_correction_data`] for details.
    pub fn get_inverse_correction_data(
        &self,
        index: usize,
    ) -> Result<CorrectionData, CapabilityError> {
        let cb = self.get_clearbox(index)?.ok_or_else(|| {
            CapabilityError::NotPresent("clearbox is not present".into())
        })?;
        Ok(grid_to_correction_data(&cb.inverse_correction_data))
    }

    /// Returns the OPCUA config, or `Err(ValidationError)` if OPCUA is present
    /// but missing one or more required fields. Collects every missing field
    /// at once (in `details`) rather than failing on the first one — see
    /// OPCUA_FIELD_PROMOTION_PLAN.md's "Why facade-only enforcement". The
    /// low-level reader/writer stay fully permissive; this is the one place
    /// "required" is enforced.
    pub fn get_opcua(&self) -> Result<OpcuaConfig, CapabilityError> {
        self.assert_open()?;
        let opcua = self
            .config
            .opcua
            .clone()
            .ok_or_else(|| CapabilityError::NotPresent("OPCUA group is not present".into()))?;

        let mut missing = Vec::new();
        if opcua.client.machine_profile.is_none() {
            missing.push("Machine_Profile".to_string());
        }
        if opcua.client.root_node.is_none() {
            missing.push("Root_Node".to_string());
        }
        if opcua.pipe.configure_client.is_none() {
            missing.push("Configure_Client".to_string());
        }
        if opcua.pipe.pipe_name.is_none() {
            missing.push("Pipe_Name".to_string());
        }
        if opcua.triggers_enabled.is_none() {
            missing.push("Triggers_Enabled".to_string());
        }
        if opcua.trigger_stop_ceiling_layers.is_none() {
            missing.push("Trigger_Stop_Ceiling_Layers".to_string());
        }
        for (name, trigger) in &opcua.triggers {
            if trigger.event.is_none() {
                missing.push(format!("{name}.Event"));
            }
        }

        if !missing.is_empty() {
            return Err(CapabilityError::ValidationError {
                message: format!(
                    "OPCUA is present but missing required field(s): {}",
                    missing.join(", ")
                ),
                details: Some(missing),
            });
        }

        Ok(opcua)
    }

    pub fn set_opcua(&mut self, model: OpcuaConfig, mode: SetMode) -> Result<(), CapabilityError> {
        self.assert_open()?;
        let current = self
            .config
            .opcua
            .clone()
            .ok_or_else(|| CapabilityError::NotPresent("OPCUA group is not present".into()))?;
        self.config.opcua = Some(apply_set_mode(&current, &model, mode)?);
        Ok(())
    }

    pub fn save(&mut self, path: Option<&Path>) -> Result<(), CapabilityError> {
        self.assert_open()?;
        let out = path
            .map(|p| p.to_path_buf())
            .or_else(|| self.path.clone())
            .ok_or_else(|| {
                CapabilityError::validation_error("save() requires a path for create()-d files")
            })?;
        MachineConfigWriter::new(&self.config)
            .write(&out)
            .map_err(|e| CapabilityError::IoError(e.to_string()))?;
        self.path = Some(out);
        Ok(())
    }

    pub fn close(&mut self) {
        self.closed = true;
    }
}

// v1.0's implementation of the MachineConfigFile contract — this impl is
// necessarily version-specific (it's what capabilities::open_machine_config/
// create_machine_config dispatch TO for File_Version "1.0", per
// DISPATCH_REGISTRY_PLAN.md's registry in capabilities/mod.rs). Those two
// functions stay version-agnostic: they only ever touch
// `Box<dyn MachineConfigFile>`, never `MachineConfigFileV1_0` by name. A
// future v1.1 gets its own separate `impl MachineConfigFile for
// MachineConfigFileV1_1`, registered alongside this one. Every method here
// delegates to the identically-named, identically-signatured inherent method
// above — Rust always prefers an inherent method over a trait method of the
// same name on the same type, so these delegating calls resolve to the
// inherent methods, not back into this trait impl. No method body's logic
// lives here; this block only exists so `MachineConfigFileV1_0` can be boxed
// as `dyn MachineConfigFile`.
impl crate::capabilities::generated::MachineConfigFile for MachineConfigFileV1_0 {
    fn file_version(&self) -> &str {
        self.file_version()
    }
    fn optical_train_count(&self) -> Result<usize, CapabilityError> {
        self.optical_train_count()
    }
    fn get_meta(&self) -> Result<MachineConfigMeta, CapabilityError> {
        self.get_meta()
    }
    fn set_meta(&mut self, model: MachineConfigMeta, mode: SetMode) -> Result<(), CapabilityError> {
        self.set_meta(model, mode)
    }
    fn get_machine(&self) -> Result<Machine, CapabilityError> {
        self.get_machine()
    }
    fn set_machine(&mut self, model: Machine, mode: SetMode) -> Result<(), CapabilityError> {
        self.set_machine(model, mode)
    }
    fn get_train(&self, index: usize) -> Result<OpticalTrain, CapabilityError> {
        self.get_train(index)
    }
    fn set_train(
        &mut self,
        index: usize,
        model: OpticalTrain,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.set_train(index, model, mode)
    }
    fn get_scanner(&self, index: usize) -> Result<Scanner, CapabilityError> {
        self.get_scanner(index)
    }
    fn set_scanner(
        &mut self,
        index: usize,
        model: Scanner,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.set_scanner(index, model, mode)
    }
    fn get_light_source(&self, index: usize) -> Result<LightSource, CapabilityError> {
        self.get_light_source(index)
    }
    fn set_light_source(
        &mut self,
        index: usize,
        model: LightSource,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.set_light_source(index, model, mode)
    }
    fn get_collimator(&self, index: usize) -> Result<Collimator, CapabilityError> {
        self.get_collimator(index)
    }
    fn set_collimator(
        &mut self,
        index: usize,
        model: Collimator,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.set_collimator(index, model, mode)
    }
    fn get_scanner_card(&self, index: usize) -> Result<ScannerCard, CapabilityError> {
        self.get_scanner_card(index)
    }
    fn set_scanner_card(
        &mut self,
        index: usize,
        model: ScannerCard,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.set_scanner_card(index, model, mode)
    }
    fn get_clearbox(&self, index: usize) -> Result<Option<ClearBox>, CapabilityError> {
        self.get_clearbox(index)
    }
    fn set_clearbox(
        &mut self,
        index: usize,
        model: ClearBox,
        mode: SetMode,
    ) -> Result<(), CapabilityError> {
        self.set_clearbox(index, model, mode)
    }
    fn get_correction_data(&self, index: usize) -> Result<CorrectionData, CapabilityError> {
        self.get_correction_data(index)
    }
    fn get_inverse_correction_data(&self, index: usize) -> Result<CorrectionData, CapabilityError> {
        self.get_inverse_correction_data(index)
    }
    fn get_opcua(&self) -> Result<OpcuaConfig, CapabilityError> {
        self.get_opcua()
    }
    fn set_opcua(&mut self, model: OpcuaConfig, mode: SetMode) -> Result<(), CapabilityError> {
        self.set_opcua(model, mode)
    }
    fn save(&mut self, path: Option<&Path>) -> Result<(), CapabilityError> {
        self.save(path)
    }
    fn close(&mut self) {
        self.close()
    }
}

/// Converts a `ClearBox` grid field to a flat [`CorrectionData`] buffer via
/// the model-layer `Array3` helper — the same conversion the HDF5 adapter's
/// `get_correction_data`/`get_inverse_correction_data` use, just starting
/// from the in-memory model instead of a freshly-read dataset.
fn grid_to_correction_data(data: &Option<Vec<Vec<Vec<Option<f64>>>>>) -> CorrectionData {
    let arr = nested_to_array3(data);
    let shape = arr.shape();
    let shape = [shape[0], shape[1], shape[2]];
    CorrectionData {
        data: arr.into_raw_vec_and_offset().0,
        shape,
    }
}
