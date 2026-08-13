pub mod registry;
pub mod test_v0_9_to_v1_0;

/// Transforms a raw JSON map from one schema version to the next.
/// Methods used instead of associated constants to preserve dyn compatibility.
pub trait Adapter {
    fn from_version(&self) -> &'static str;
    fn to_version(&self) -> &'static str;

    fn adapt(
        &self,
        config: serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Map<String, serde_json::Value>;
}
