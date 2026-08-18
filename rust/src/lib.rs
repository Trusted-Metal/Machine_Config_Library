// Machine Config Library — Rust implementation
// Phase 3: see IMPLEMENTATION_PLAN.md §3.1 for package structure

pub mod builder;
pub mod capabilities;
pub mod error;
pub mod models;
pub mod reader;
pub mod writer;

// Crate-root re-exports: consumers must never need `machine_config::models::X` or
// any other sub-module path for a public type (see VALIDATION_PLAN.md §8 S-09).
pub use builder::MockConfigBuilder;
pub use error::MachineConfigError;
pub use models::*;
pub use reader::MachineConfigReader;
pub use writer::MachineConfigWriter;
