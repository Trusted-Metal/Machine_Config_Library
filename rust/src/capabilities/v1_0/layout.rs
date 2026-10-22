//! File_Version 1.0 on-disk HDF5 layout.

pub const FILE_VERSION: &str = "1.0";
pub const ROOT_MACHINE: &str = "Machine";
pub const ROOT_OPTICAL_TRAINS: &str = "Machine/Optical_Trains";
/// File_Version 1.0 has no Extensions group: the one extension it can hold lives at the root as `OPCUA/`.
pub const ROOT_OPCUA: &str = "OPCUA";
/// The extension name core exposes the legacy `OPCUA/` group under.
pub const LEGACY_OPCUA_EXTENSION_NAME: &str = "TM_OPCUA";
pub const TRAIN_ID_PREFIX: &str = "Optical_Train_";
pub const GROUP_SCANNER: &str = "Scanner";
pub const GROUP_LIGHT_SOURCE: &str = "Light_Source";
pub const GROUP_COLLIMATOR: &str = "Collimator";
pub const GROUP_SCANNER_CARD: &str = "Scanner_Card";
pub const GROUP_OPTIONAL_COMPONENTS: &str = "Optional_Components";
pub const GROUP_CLEARBOX: &str = "ClearBox";
pub const DS_CORRECTION_DATA: &str = "Correction_Data";
pub const DS_INVERSE_CORRECTION_DATA: &str = "Inverse_Correction_Data";
pub const DS_SCAN_FIELD_CORRECTION_FILE: &str = "scan_field_correction_file";

pub fn train_id(index: usize) -> String {
    format!("{TRAIN_ID_PREFIX}{:02}", index + 1)
}

pub fn train_path(index: usize) -> String {
    format!("{ROOT_OPTICAL_TRAINS}/{}", train_id(index))
}

pub fn train_path_by_id(tid: &str) -> String {
    format!("{ROOT_OPTICAL_TRAINS}/{tid}")
}

pub fn clearbox_path(train_index: usize) -> String {
    format!(
        "{}/{GROUP_OPTIONAL_COMPONENTS}/{GROUP_CLEARBOX}",
        train_path(train_index)
    )
}

pub fn correction_data_path(train_index: usize) -> String {
    format!("{}/{DS_CORRECTION_DATA}", clearbox_path(train_index))
}

pub fn inverse_correction_data_path(train_index: usize) -> String {
    format!("{}/{DS_INVERSE_CORRECTION_DATA}", clearbox_path(train_index))
}

pub fn scan_field_correction_file_path(train_index: usize) -> String {
    format!("{}/{DS_SCAN_FIELD_CORRECTION_FILE}", train_path(train_index))
}
