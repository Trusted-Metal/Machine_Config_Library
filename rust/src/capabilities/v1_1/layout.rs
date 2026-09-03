//! File_Version 1.1 on-disk HDF5 layout.
//!
//! Deliberately independent of `capabilities/v1_0/layout.rs` — nothing here
//! imports from or refers to v1.0, so v1.0 can change or be removed later
//! without affecting v1.1. See `docs/migrations/v1_0_to_v1_1.md` for the full
//! migration manifest this layout implements.

pub const FILE_VERSION: &str = "1.1";

pub const ROOT_MACHINE: &str = "Machine";
pub const ROOT_OPTICAL_TRAINS: &str = "Machine/Optical_Trains";
pub const ROOT_EXTENSIONS: &str = "Extensions";
pub const GROUP_CLEARBOX: &str = "Extensions/ClearBox";
pub const ROOT_OPCUA: &str = "Extensions/TM_OPCUA";
pub const OPCUA_CLIENT: &str = "Extensions/TM_OPCUA/Client";
pub const OPCUA_PIPE: &str = "Extensions/TM_OPCUA/Pipe";
pub const OPCUA_TRIGGERS: &str = "Extensions/TM_OPCUA/Triggers";

pub const TRAIN_ID_PREFIX: &str = "Optical_Train_";
pub const GROUP_SCANNER: &str = "Scanner";
pub const GROUP_LIGHT_SOURCE: &str = "Light_Source";
pub const GROUP_COLLIMATOR: &str = "Collimator";
pub const GROUP_SCANNER_CARD: &str = "Scanner_Card";
pub const GROUP_POWER_CHARACTERIZATION: &str = "Power_Characterization";
pub const DS_DERIVATION_EQUATION_CONSTANTS: &str = "Derivation_Equation_Constants";
pub const DS_CHARACTERIZATION_POINTS: &str = "Characterization_Points";
pub const DS_CORRECTION_DATA: &str = "Correction_Data";
pub const DS_INVERSE_CORRECTION_DATA: &str = "Inverse_Correction_Data";
pub const DS_SCAN_FIELD_CORRECTION_FILE: &str = "scan_field_correction_file";

pub fn train_id(index: usize) -> String {
    format!("{TRAIN_ID_PREFIX}{:02}", index + 1)
}

pub fn train_path_by_id(tid: &str) -> String {
    format!("{ROOT_OPTICAL_TRAINS}/{tid}")
}

pub fn scanner_path_by_id(tid: &str) -> String {
    format!("{}/{GROUP_SCANNER}", train_path_by_id(tid))
}

pub fn light_source_path_by_id(tid: &str) -> String {
    format!("{}/{GROUP_LIGHT_SOURCE}", train_path_by_id(tid))
}

pub fn collimator_path_by_id(tid: &str) -> String {
    format!("{}/{GROUP_COLLIMATOR}", train_path_by_id(tid))
}

pub fn scanner_card_path_by_id(tid: &str) -> String {
    format!("{}/{GROUP_SCANNER_CARD}", train_path_by_id(tid))
}

pub fn clearbox_path_by_id(tid: &str) -> String {
    format!("{GROUP_CLEARBOX}/{tid}")
}

pub fn correction_data_path_by_id(tid: &str) -> String {
    format!("{}/{DS_CORRECTION_DATA}", clearbox_path_by_id(tid))
}

pub fn inverse_correction_data_path_by_id(tid: &str) -> String {
    format!("{}/{DS_INVERSE_CORRECTION_DATA}", clearbox_path_by_id(tid))
}

pub fn scan_field_correction_file_path_by_id(tid: &str) -> String {
    format!("{}/{DS_SCAN_FIELD_CORRECTION_FILE}", train_path_by_id(tid))
}
