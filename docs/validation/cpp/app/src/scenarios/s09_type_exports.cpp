// S-09: Public type export surface
//
// Every public type must resolve from the single umbrella header
// (no internal include paths) and be usable, not just nameable.
// See VALIDATION_PLAN.md §8 S-09 and §9.5.
#include "s09_type_exports.hpp"
#include "machine_config/machine_config.hpp"

namespace s09 {

using namespace machine_config;

// OpcuaConfig is never populated by MockConfigBuilder's default build (no
// OPCUA fixture involved here) — type-annotating a never-called parameter is
// exactly what VALIDATION_PLAN.md §8 S-09 allows: "Construct OR
// type-annotate a variable with each type."
static void typeCheckOpcua(const OpcuaConfig&) {}

scenarios::Result run(const std::filesystem::path&, const std::filesystem::path&) {
    (void)&typeCheckOpcua;

    MockConfigBuilder builder;
    builder.laser_count = 2;
    MachineConfig config = builder.build();

    const MachineConfigMeta& meta = config.meta;
    const Machine& machine = config.machine;

    if (config.optical_trains.empty()) {
        return {false, "builder produced zero optical trains"};
    }
    const OpticalTrain& train = config.optical_trains[0];
    const Scanner& scanner = train.scanner;
    const LightSource& lightSource = train.light_source;
    const Collimator& collimator = train.collimator;
    const ScannerCard& scannerCard = train.scanner_card;
    const OptionalComponents& optionalComponents = train.optional_components;

    if (!optionalComponents.clearbox.has_value()) {
        return {false, "expected builder to include a clearbox by default"};
    }
    const ClearBox& clearbox = *optionalComponents.clearbox;

    if (!train.scan_field_correction_file.has_value()) {
        return {false, "expected builder to include an SFCF by default"};
    }
    const ScanFieldCorrectionFile& sfcf = *train.scan_field_correction_file;

    MachineConfigReader reader{std::filesystem::path{"unused"}};
    MachineConfigWriter writer{config};
    (void)reader;
    (void)writer;

    if (meta.machine_name.empty() || machine.manufacturer.empty() ||
        scanner.manufacturer.empty() || lightSource.manufacturer.empty() ||
        collimator.manufacturer.empty() || scannerCard.manufacturer.empty() ||
        clearbox.ip_address.empty() || sfcf.document_name.empty()) {
        return {false, "one or more constructed fields were unexpectedly empty"};
    }

    return {true, "all public types resolve and are usable from the umbrella header"};
}

} // namespace s09
