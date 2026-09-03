// AUTO-GENERATED from schema/capabilities — DO NOT EDIT
// python tools/generate_capabilities.py

//! Generated SetMode and facade trait.

use crate::capabilities::errors::CapabilityError;
use crate::capabilities::result::Result;
use crate::models::{
    ClearBox, Collimator, CorrectionData, LightSource, Machine, MachineConfigMeta,
    OpcuaConfig, OpticalTrain, Scanner, ScannerCard,
};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SetMode {
    #[default]
    Merge,
    Replace,
}

/// Version-agnostic stable facade — mirrors MachineConfigFileV1_0's full
/// public surface so the version-agnostic dispatch path
/// (capabilities::open_machine_config/create_machine_config) never loses
/// capability relative to using the concrete type directly.
pub trait MachineConfigFile {
    fn file_version(&self) -> &str;
    fn optical_train_count(&self) -> Result<usize, CapabilityError>;

    fn get_meta(&self) -> Result<MachineConfigMeta, CapabilityError>;
    fn set_meta(&mut self, model: MachineConfigMeta, mode: SetMode) -> Result<(), CapabilityError>;

    fn get_machine(&self) -> Result<Machine, CapabilityError>;
    fn set_machine(&mut self, model: Machine, mode: SetMode) -> Result<(), CapabilityError>;

    fn get_train(&self, index: usize) -> Result<OpticalTrain, CapabilityError>;
    fn set_train(&mut self, index: usize, model: OpticalTrain, mode: SetMode) -> Result<(), CapabilityError>;

    fn get_scanner(&self, index: usize) -> Result<Scanner, CapabilityError>;
    fn set_scanner(&mut self, index: usize, model: Scanner, mode: SetMode) -> Result<(), CapabilityError>;

    fn get_light_source(&self, index: usize) -> Result<LightSource, CapabilityError>;
    fn set_light_source(&mut self, index: usize, model: LightSource, mode: SetMode) -> Result<(), CapabilityError>;

    fn get_collimator(&self, index: usize) -> Result<Collimator, CapabilityError>;
    fn set_collimator(&mut self, index: usize, model: Collimator, mode: SetMode) -> Result<(), CapabilityError>;

    fn get_scanner_card(&self, index: usize) -> Result<ScannerCard, CapabilityError>;
    fn set_scanner_card(&mut self, index: usize, model: ScannerCard, mode: SetMode) -> Result<(), CapabilityError>;

    fn get_clearbox(&self, index: usize) -> Result<Option<ClearBox>, CapabilityError>;
    fn set_clearbox(&mut self, index: usize, model: ClearBox, mode: SetMode) -> Result<(), CapabilityError>;

    fn get_correction_data(&self, index: usize) -> Result<CorrectionData, CapabilityError>;
    fn get_inverse_correction_data(&self, index: usize) -> Result<CorrectionData, CapabilityError>;

    fn get_opcua(&self) -> Result<OpcuaConfig, CapabilityError>;
    fn set_opcua(&mut self, model: OpcuaConfig, mode: SetMode) -> Result<(), CapabilityError>;

    fn save(&mut self, path: Option<&Path>) -> Result<(), CapabilityError>;
    fn close(&mut self);
}
