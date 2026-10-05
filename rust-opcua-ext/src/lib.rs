pub mod raw_group;
pub mod reader;
pub mod types;
pub mod writer;

#[cfg(test)]
mod tests;

pub use reader::read_opcua_config;
pub use types::{
    is_flag_enabled, AttributeValue, ClientConfig, FlagLike, PipeConfig, TriggerConditionConfig,
    TriggerConfig, TriggerRuleConfig,
};
pub use writer::write_opcua_config;
pub(crate) const OPCUA_ROOT: &str = "Extensions/TM_OPCUA";
