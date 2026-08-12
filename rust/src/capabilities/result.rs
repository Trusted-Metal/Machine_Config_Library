//! Capability Result alias and helpers.
pub type Result<T, E = crate::capabilities::errors::CapabilityError> = std::result::Result<T, E>;
