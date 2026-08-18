// S-09: Public type export surface
//
// ID:          S-09
// Title:       Verify all public model types are importable from the library surface
// Category:    happy-path
// Layer:       public API / packaging
// Action:      Import every consumer-facing type directly from the crate root
//              (no `machine_config::models::...` or other sub-module path) and
//              prove each is usable, not just nameable.
// Rationale:   If a consumer must import from an internal path, the library's
//              public API surface is incomplete. See VALIDATION_PLAN.md §8.

use machine_config::{
    Machine, MachineConfig, MachineConfigMeta, MachineConfigReader, MachineConfigWriter,
    MockConfigBuilder, OpticalTrain, Scanner, LightSource, Collimator, ScannerCard,
    ClearBox, ScanFieldCorrectionFile, OpcuaConfig, OptionalComponents,
};
use std::path::Path;

// `MockConfigBuilder::new(n).build()` never sets OPCUA (that only comes from
// the OPCUA fixture used in S-07) — there's no real value to hold here, so
// type-annotating a never-called parameter is what VALIDATION_PLAN.md §8 S-09
// explicitly allows: "Construct OR type-annotate a variable with each type."
#[allow(dead_code)]
fn type_check_opcua(_opcua: &OpcuaConfig) {}

pub fn run(_fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let builder: MockConfigBuilder = MockConfigBuilder::new(2);
    let config: MachineConfig = builder.build();

    let meta: &MachineConfigMeta = &config.meta;
    let machine: &Machine = &config.machine;

    if config.optical_trains.is_empty() {
        return (false, "builder produced zero optical trains".to_string());
    }
    let train: &OpticalTrain = &config.optical_trains[0];
    let scanner: &Scanner = &train.scanner;
    let light_source: &LightSource = &train.light_source;
    let collimator: &Collimator = &train.collimator;
    let scanner_card: &ScannerCard = &train.scanner_card;
    let optional_components: &OptionalComponents = &train.optional_components;

    let clearbox: &ClearBox = match &optional_components.clearbox {
        Some(cb) => cb,
        None => return (false, "expected builder to include a clearbox by default".to_string()),
    };
    let sfcf: &ScanFieldCorrectionFile = match &train.scan_field_correction_file {
        Some(s) => s,
        None => return (false, "expected builder to include an SFCF by default".to_string()),
    };

    let writer: MachineConfigWriter = MachineConfigWriter::new(&config);
    let _reader_type_check: fn(&str) -> machine_config::error::Result<MachineConfigReader> =
        |p| MachineConfigReader::open(p);

    if meta.machine_name.is_empty()
        || machine.manufacturer.is_empty()
        || scanner.manufacturer.is_empty()
        || light_source.manufacturer.is_empty()
        || collimator.manufacturer.is_empty()
        || scanner_card.manufacturer.is_empty()
        || clearbox.ip_address.is_empty()
        || sfcf.document_name.is_empty()
    {
        return (false, "one or more constructed fields were unexpectedly empty".to_string());
    }
    let _ = writer;

    (
        true,
        "all public types resolve and are usable from the crate root".to_string(),
    )
}
