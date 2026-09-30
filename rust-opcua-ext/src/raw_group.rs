use machine_config::capabilities::v1_0::hdf5::Hdf5AdapterV1_0;
use machine_config::capabilities::v1_1::hdf5::Hdf5AdapterV1_1;
use machine_config::{CapabilityError, ExtraAttrs};

pub trait RawGroupAccess {
    fn get_raw_group(&self, hdf5_path: &str) -> Result<ExtraAttrs, CapabilityError>;
    fn list_raw_group_children(&self, hdf5_path: &str) -> Result<Vec<String>, CapabilityError>;
    fn set_raw_group(&self, hdf5_path: &str, attrs: &ExtraAttrs) -> Result<(), CapabilityError>;
}

impl RawGroupAccess for Hdf5AdapterV1_0 {
    fn get_raw_group(&self, p: &str) -> Result<ExtraAttrs, CapabilityError> {
        Hdf5AdapterV1_0::get_raw_group(self, p).map_err(|e| CapabilityError::IoError(e.to_string()))
    }
    fn list_raw_group_children(&self, p: &str) -> Result<Vec<String>, CapabilityError> {
        Hdf5AdapterV1_0::list_raw_group_children(self, p).map_err(|e| CapabilityError::IoError(e.to_string()))
    }
    fn set_raw_group(&self, p: &str, attrs: &ExtraAttrs) -> Result<(), CapabilityError> {
        Hdf5AdapterV1_0::set_raw_group(self, p, attrs).map_err(|e| CapabilityError::IoError(e.to_string()))
    }
}

impl RawGroupAccess for Hdf5AdapterV1_1 {
    fn get_raw_group(&self, p: &str) -> Result<ExtraAttrs, CapabilityError> {
        Hdf5AdapterV1_1::get_raw_group(self, p).map_err(|e| CapabilityError::IoError(e.to_string()))
    }
    fn list_raw_group_children(&self, p: &str) -> Result<Vec<String>, CapabilityError> {
        Hdf5AdapterV1_1::list_raw_group_children(self, p).map_err(|e| CapabilityError::IoError(e.to_string()))
    }
    fn set_raw_group(&self, p: &str, attrs: &ExtraAttrs) -> Result<(), CapabilityError> {
        Hdf5AdapterV1_1::set_raw_group(self, p, attrs).map_err(|e| CapabilityError::IoError(e.to_string()))
    }
}
