//! Capability-API error taxonomy (schema/capabilities/errors.yaml).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    UnsupportedVersion(String),
    UnsupportedInVersion(String),
    NotPresent(String),
    /// `details` names every individual violation at once (e.g. every missing
    /// required OPCUA field) rather than only the first one encountered.
    /// `None` for validation failures with nothing more specific to list.
    ValidationError {
        message: String,
        details: Option<Vec<String>>,
    },
    IoError(String),
    InvalidIndex(String),
    Closed,
}

impl CapabilityError {
    /// Convenience constructor for the common case of no `details`.
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::ValidationError { message: message.into(), details: None }
    }
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(m)
            | Self::UnsupportedInVersion(m)
            | Self::NotPresent(m)
            | Self::IoError(m)
            | Self::InvalidIndex(m) => write!(f, "{m}"),
            Self::ValidationError { message, .. } => write!(f, "{message}"),
            Self::Closed => write!(f, "MachineConfigFile session is closed"),
        }
    }
}

impl std::error::Error for CapabilityError {}
