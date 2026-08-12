//! Capability-API error taxonomy (schema/capabilities/errors.yaml).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    UnsupportedVersion(String),
    UnsupportedInVersion(String),
    NotPresent(String),
    ValidationError(String),
    IoError(String),
    InvalidIndex(String),
    Closed,
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(m)
            | Self::UnsupportedInVersion(m)
            | Self::NotPresent(m)
            | Self::ValidationError(m)
            | Self::IoError(m)
            | Self::InvalidIndex(m) => write!(f, "{m}"),
            Self::Closed => write!(f, "MachineConfigFile session is closed"),
        }
    }
}

impl std::error::Error for CapabilityError {}
