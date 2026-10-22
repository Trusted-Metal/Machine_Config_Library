// Machine Config Library — Rust implementation
// Phase 3: see IMPLEMENTATION_PLAN.md §3.1 for package structure

pub mod builder;
pub mod capabilities;
pub mod error;
pub mod extensions;
pub mod hash;
pub mod models;
pub mod power_characterization;
pub mod reader;
pub mod writer;

// Crate-root re-exports: consumers must never need `machine_config::models::X` or
// any other sub-module path for a public type (see VALIDATION_PLAN.md §8 S-09).
pub use builder::MockConfigBuilder;
pub use error::MachineConfigError;
pub use models::*;
pub use reader::MachineConfigReader;
pub use writer::MachineConfigWriter;

// Extension contract: core carries extension data as opaque trees; extension libraries depend on
// `mcf-extension-contract` directly and never on core.
pub use mcf_extension_contract as extension_contract;
pub use mcf_extension_contract::{canonical_digest, ExtensionTree, EXTENSION_CONTRACT_VERSION};

// Stable model facade (open/create a MachineConfigFile session, get/set with
// SetMode). Curated, not `pub use capabilities::*` — a wildcard would also
// re-export `capabilities`'s child modules (`errors`, `generated`, `merge`,
// `result`, `v1_0`) as new crate-root paths, and `capabilities::result::Result`
// is a different, two-parameter type from this crate's own `error::Result`.
pub use capabilities::{
    create_machine_config, open_machine_config, supported_file_versions, CapabilityError,
    MachineConfigFile, MachineConfigFileV1_0, MachineConfigFileV1_1, SetMode,
};
