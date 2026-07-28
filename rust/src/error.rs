// Phase 3.3 — Crate-wide error type and Result alias.
// All modules use `crate::error::Result<T>`.

use thiserror::Error;

/// All errors that the machine-config library can produce.
#[derive(Debug, Error)]
pub enum MachineConfigError {
    /// Propagated from the `hdf5` (hdf5-metno) crate.
    #[error("HDF5 error: {0}")]
    Hdf5(#[from] hdf5::Error),

    /// Propagated from `serde_json`.
    #[error("JSON serialisation error: {0}")]
    Json(#[from] serde_json::Error),

    /// Any parse-time error with a human-readable description.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Rule 8 (§0.5): a `_unit` attribute was present but did not match the
    /// locked expected value for the current schema version.
    #[error("Unit mismatch on attribute '{attr}': expected '{expected}', got '{actual}'")]
    UnitMismatch {
        attr: String,
        expected: String,
        actual: String,
    },

    /// The file's `File_Version` attribute is not `"1.0"`.
    /// A warning is emitted for unrecognised minor versions; this variant is
    /// reserved for incompatible future major versions.
    #[error("File version '{0}' is not supported by this reader (expected \"1.0\")")]
    UnsupportedVersion(String),

    /// A required HDF5 group or dataset was absent from the file.
    #[error("Required HDF5 path missing: {0}")]
    MissingGroup(String),
}

/// Shorthand `Result` used throughout the crate.
pub type Result<T> = std::result::Result<T, MachineConfigError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_message() {
        let e = MachineConfigError::Parse("bad value".into());
        assert_eq!(e.to_string(), "Parse error: bad value");
    }

    #[test]
    fn unit_mismatch_message_contains_all_fields() {
        let e = MachineConfigError::UnitMismatch {
            attr: "Working_Distance_unit".into(),
            expected: "mm".into(),
            actual: "cm".into(),
        };
        let msg = e.to_string();
        assert!(msg.contains("Working_Distance_unit"), "attr missing from message");
        assert!(msg.contains("mm"), "expected unit missing from message");
        assert!(msg.contains("cm"), "actual unit missing from message");
    }

    #[test]
    fn unsupported_version_message_contains_version() {
        let e = MachineConfigError::UnsupportedVersion("2.0".into());
        assert!(e.to_string().contains("2.0"));
    }

    #[test]
    fn missing_group_message_contains_path() {
        let e = MachineConfigError::MissingGroup("Machine/Optical_Trains".into());
        assert!(e.to_string().contains("Machine/Optical_Trains"));
    }

    #[test]
    fn from_serde_json_error_produces_json_variant() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let mc_err: MachineConfigError = json_err.into();
        assert!(matches!(mc_err, MachineConfigError::Json(_)));
    }

    #[test]
    fn result_alias_ok_and_err() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: Result<i32> = Err(MachineConfigError::Parse("test".into()));
        assert!(err.is_err());
    }
}
