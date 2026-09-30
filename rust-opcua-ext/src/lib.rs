pub mod engine;
pub mod raw_group;
pub mod reader;
pub mod types;
pub mod writer;

#[cfg(test)] mod tests;

pub use engine::{
    evaluate_condition, AtomicCondition, CachedValue, CaseSensitivity, Combinator, ComparisonOp,
    Condition, EvaluationMode, SignalKey, TriggerActionKind, TriggerRule, EPSILON,
};
pub use reader::read_opcua_config;
pub use types::{
    is_flag_enabled, AttributeValue, ClientConfig, FlagLike, PipeConfig, TriggerConditionConfig,
    TriggerConfig, TriggerRuleConfig,
};
pub use writer::write_opcua_config;
