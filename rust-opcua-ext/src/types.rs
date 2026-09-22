//! Extension configuration types
//!
//! `OpcuaConfig` holds the only current extension's types, integration with the
//! MCL's LoadedConfig for it's own modeled types enables streamlined extension support

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Build configuration from /Configuration/build_configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Unique build identifier
    pub build_id: String,
    /// Operator/user name
    pub username: String,
    /// Total number of layers
    pub layer_count: u32,
    /// Number of parts being built
    pub num_of_parts: u32,
    /// Alloy/material name
    pub alloy: String,
    /// Material ID
    pub material_id: String,
    /// Build plate thickness (mm)
    pub build_plate_thick: f64,
    /// Build plate weight (kg)
    pub build_plate_weight: f64,
    /// Starting height (mm)
    pub starting_height: f64,
    /// Layer thickness (mm)
    pub layer_thick: f64,
    /// Estimated build time
    pub est_build_time: f64,
    /// Time unit for estimated build time
    pub est_build_time_unit: String,
    /// Whether to save binary data
    pub save_bin_data: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            build_id: String::new(),
            username: String::new(),
            layer_count: 0,
            num_of_parts: 0,
            alloy: String::new(),
            material_id: String::new(),
            build_plate_thick: 0.0,
            build_plate_weight: 0.0,
            starting_height: 0.0,
            layer_thick: 0.03, // 30 microns default
            est_build_time: 60.0,
            est_build_time_unit: "Minutes".to_string(),
            save_bin_data: false,
        }
    }
}

/// Sensor configuration from /Configuration/sensor_configurations/sensor_{id}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorConfig {
    /// Sensor ID
    pub sensor_id: String,
    /// Derived units (e.g., "Watts", "Celsius")
    pub sensor_derived_units: String,
    /// Port ID
    pub sensor_port_id: String,
    /// Sensor type
    pub sensor_type: String,
    /// Input type (e.g., "Analog", "Digital")
    pub sensor_input_type: String,
    /// Whether this sensor is enabled
    pub sensor_enabled: bool,
    /// Timing/sampling interval
    pub sensor_timing: f64,
    /// Associated laser number
    pub sensor_laser_num: u8,
    /// Sensor input values [min, max]
    pub sensor_input: [f64; 2],
    /// Derivation equation constants
    pub sensor_derivation_eqn_constants: Vec<f64>,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            sensor_id: "sensor_1".to_string(),
            sensor_derived_units: "Watts".to_string(),
            sensor_port_id: "1".to_string(),
            sensor_type: "Laser Power".to_string(),
            sensor_input_type: "Analog".to_string(),
            sensor_enabled: true,
            sensor_timing: 0.1,
            sensor_laser_num: 1,
            sensor_input: [0.0, 10.0],
            sensor_derivation_eqn_constants: vec![0.0, 1.0],
        }
    }
}

/// Active machine configuration — held in `AppState.machine_config`.
///
/// Fields are extracted from the library facade at load time via its public
/// getters and stored as the library's own model types. The app never reaches
/// past the facade's stable API surface, so it is version-agnostic: adding a
/// new adapter to the library requires no change here.
#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub source_file: PathBuf,
    // Extracted via the facade's public getters — library-owned types, Clone+Serialize.
    pub meta: machine_config::MachineConfigMeta,
    pub machine: machine_config::Machine,
    pub optical_trains: Vec<machine_config::OpticalTrain>,
    /// None when the file has no OPCUA group, or when validation fails.
    pub opcua: Option<machine_config::OpcuaConfig>,
    /// Correction data per optical train, parallel-indexed to `optical_trains`.
    /// Fetched via the facade's `get_correction_data(i)` at load time, while the
    /// file is still open — `None` for a train whose `ClearBox` has no
    /// correction grid, `Some(_)` with the library's own flat, row-major
    /// `CorrectionData` otherwise. Deliberately the facade's own type, not a
    /// re-flattened copy: the NaN-as-`None` grid conversion is the library's
    /// logic to own, not this app's to reimplement.
    pub correction_data: Vec<Option<machine_config::CorrectionData>>,
    /// Same shape as `correction_data`, for the inverse correction grid.
    pub inverse_correction_data: Vec<Option<machine_config::CorrectionData>>,
    // Legacy groups outside the MCF library's schema.
    pub build: BuildConfig,
    pub sensors: Vec<SensorConfig>,
    /// `recoater_blade_type` from the legacy `/Configuration/settings_configuration` group
    /// (Phase 3d-v). The only field of the old, pre-migration `SettingsConfig` struct that
    /// turned out to still be a genuine orphan after checking all 11 against both the MCF
    /// library's schema and the app's own live `Settings` — see
    /// docs/MCF_INTEGRATION_PLAN.md, Phase 3d-v. Falls back to `"Standard"` when the group
    /// or attribute is absent, matching the old `SettingsConfig::default()` exactly.
    pub recoater_blade_type: String,
}

/// JSON boundary for `LoadedConfig` — `LoadedConfig` itself deliberately isn't
/// `Serialize` (Phase 2a), so this DTO defines exactly the shape any JSON consumer
/// receives. `opcua` is an *optional* key (matching the Phase 4 TS `LoadedConfig`
/// interface's `opcua?: McfOpcuaConfig`) because `machine_config::OpcuaConfig` has no
/// `skip_serializing_if` on any of its own fields — that omission has to happen at
/// this boundary, not inside the library type. Holds references, not clones, for every
/// field except `optical_trains` (see that field's own doc comment for why) — so it's
/// built and consumed via `serde_json::to_value` within the borrow's lifetime (see
/// docs/MCF_INTEGRATION_PLAN.md, Phase 3c-ii for why callers return `serde_json::Value`
/// rather than this struct itself).
///
/// Lives here, not in `commands.rs` (its original Phase 3c-ii location), because both
/// `commands.rs` (desktop) and `daemon/server.rs` (headless) need it and neither may
/// depend on the other's feature gate — see docs/MCF_INTEGRATION_PLAN.md, Phase 3l.
/// This module is never feature-gated, so it's the correct neutral home, the same way
/// `crate::machine_config::MachineConfig` was itself the neutral, ungated type both
/// sides independently imported before this integration.
#[derive(serde::Serialize)]
pub(crate) struct LoadedConfigJson<'a> {
    source_file: &'a std::path::Path,
    meta: &'a machine_config::MachineConfigMeta,
    machine: &'a machine_config::Machine,
    optical_trains: &'a [machine_config::OpticalTrain],
    #[serde(skip_serializing_if = "Option::is_none")]
    opcua: Option<&'a machine_config::OpcuaConfig>,
    build: &'a BuildConfig,
    sensors: &'a [SensorConfig],
    /// Phase 4a: exposed for display only (`SettingsPage.tsx`'s `MachineSettingsPanel`) — see
    /// docs/MCF_INTEGRATION_PLAN.md, Phase 4a's `recoater_blade_type` deferral note. No edit path;
    /// same value `LoadedConfig.recoater_blade_type` already holds (Phase 3d-v).
    recoater_blade_type: &'a str,
    /// Parallel to `optical_trains` (same shape/index convention as `LoadedConfig.correction_data`
    /// itself) — a cheap derived boolean, not the actual grid data. This is the *only* sanctioned
    /// way for the frontend's per-laser "Correction Data" badge to know whether a train has one.
    /// `optical_trains[i].optional_components.clearbox.correction_data`/`inverse_correction_data`
    /// and `scan_field_correction_file.raw_bytes` are stripped at load time (see
    /// `machine_config::reader::strip_heavy_binary_fields`), so they're never present to leak here
    /// in the first place — this boolean is the sole signal by construction, not by a second pass.
    has_correction_data: Vec<bool>,
}

impl<'a> From<&'a LoadedConfig> for LoadedConfigJson<'a> {
    fn from(config: &'a LoadedConfig) -> Self {
        Self {
            source_file: &config.source_file,
            meta: &config.meta,
            machine: &config.machine,
            optical_trains: &config.optical_trains,
            opcua: config.opcua.as_ref(),
            build: &config.build,
            sensors: &config.sensors,
            recoater_blade_type: &config.recoater_blade_type,
            has_correction_data: config.correction_data.iter().map(Option::is_some).collect(),
        }
    }
}

#[cfg(test)]
mod contract {
    use super::*;

    /// `LoadedConfig` must hold the library's own model types directly. If this stops
    /// compiling, someone reintroduced the mapping antipattern this integration removed —
    /// fix the field type, don't fix this test.
    #[test]
    fn loaded_config_holds_library_types_directly() {
        fn assert_types(c: &LoadedConfig) {
            let _: &machine_config::MachineConfigMeta = &c.meta;
            let _: &machine_config::Machine = &c.machine;
            let _: &Vec<machine_config::OpticalTrain> = &c.optical_trains;
            let _: &Option<machine_config::OpcuaConfig> = &c.opcua;
            let _: &Vec<Option<machine_config::CorrectionData>> = &c.correction_data;
            let _: &Vec<Option<machine_config::CorrectionData>> = &c.inverse_correction_data;
        }
        let _ = assert_types; // never called — the type-check at compile time is the test
    }
}