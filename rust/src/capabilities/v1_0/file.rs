//! File_Version 1.0 stable model facade.

use crate::builder::MockConfigBuilder;
use crate::capabilities::errors::CapabilityError;
use crate::capabilities::generated::SetMode;
use crate::capabilities::merge::apply_set_mode;
use crate::capabilities::result::Result;
use crate::models::{
    ClearBox, Collimator, LightSource, Machine, MachineConfig, MachineConfigMeta, OpcuaConfig,
    OpticalTrain, Scanner, ScannerCard,
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

    pub fn get_opcua(&self) -> Result<OpcuaConfig, CapabilityError> {
        self.assert_open()?;
        self.config
            .opcua
            .clone()
            .ok_or_else(|| CapabilityError::NotPresent("OPCUA group is not present".into()))
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
                CapabilityError::ValidationError(
                    "save() requires a path for create()-d files".into(),
                )
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
