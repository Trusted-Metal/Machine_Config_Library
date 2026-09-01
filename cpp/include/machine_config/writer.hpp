#pragma once
// Public HDF5 writer: dispatch to a File_Version adapter.
// On-disk group paths and HDF5 attribute names live in the matching adapter.

#include "machine_config/adapters.hpp"
#include "machine_config/models.hpp"
#include "machine_config/capabilities/v1_0/writer.hpp"
#include "machine_config/capabilities/v1_1/writer.hpp"

#include <filesystem>
#include <functional>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <utility>

namespace machine_config {

// Version -> constructor for that version's WriterAdapter. A real registry
// (DISPATCH_REGISTRY_PLAN.md), not a hardcoded check.
using WriterRegistry =
    std::unordered_map<std::string,
                        std::function<std::unique_ptr<WriterAdapter>(const MachineConfig&)>>;

inline const WriterRegistry& productionWriterRegistry() {
    static const WriterRegistry registry = {
        {"1.0", [](const MachineConfig& cfg) -> std::unique_ptr<WriterAdapter> {
             return std::make_unique<capabilities::v1_0::Hdf5WriterV1_0>(cfg);
         }},
        {"1.1", [](const MachineConfig& cfg) -> std::unique_ptr<WriterAdapter> {
             return std::make_unique<capabilities::v1_1::Hdf5WriterV1_1>(cfg);
         }},
    };
    return registry;
}

// Registry-parameterized so tests can inject a fake entry without touching
// global state — see DISPATCH_REGISTRY_PLAN.md's shared testing pattern.
inline std::unique_ptr<WriterAdapter> resolveWriter(
    const std::string& version, const MachineConfig& cfg, const WriterRegistry& registry) {
    auto it = registry.find(version);
    if (it == registry.end()) {
        throw std::runtime_error(
            "No adapter registered for File_Version \"" + version + "\"");
    }
    return it->second(cfg);
}

class MachineConfigWriter {
public:
    explicit MachineConfigWriter(const MachineConfig& cfg) : cfg_(cfg) {}

    // Like the single-argument constructor, but writes as targetVersion
    // regardless of cfg.meta.file_version — lets a caller upgrade/downgrade
    // without mutating the model just to express intent (e.g. reading a
    // v1.0 file and writing it as v1.1 no longer requires setting
    // cfg.meta.file_version = "1.1" first). Never mutates cfg itself; only
    // the on-disk File_Version changes.
    MachineConfigWriter(const MachineConfig& cfg, std::string targetVersion)
        : cfg_(cfg), targetVersion_(std::move(targetVersion)) {}

    void write(std::filesystem::path path) const {
        std::string current = normalizeVersion(cfg_.meta.file_version);
        std::string fv = targetVersion_ ? *targetVersion_ : current;
        // Every adapter stamps cfg.meta.file_version verbatim as the on-disk
        // File_Version attribute — if targetVersion_ overrides the adapter
        // choice, the config handed to the adapter must reflect that too,
        // or the file would claim the wrong version on disk. A copy, not a
        // mutation of the caller's config, and only made when actually
        // needed (the common case — no override — never pays for it).
        if (fv == current) {
            resolveWriter(fv, cfg_, productionWriterRegistry())->write(std::move(path));
        } else {
            MachineConfig corrected = cfg_;
            corrected.meta.file_version = fv;
            resolveWriter(fv, corrected, productionWriterRegistry())->write(std::move(path));
        }
    }

private:
    static std::string normalizeVersion(const std::string& raw) {
        auto first = raw.find_first_not_of(" \t\r\n");
        if (first == std::string::npos) return "1.0";
        auto last = raw.find_last_not_of(" \t\r\n");
        std::string fv = raw.substr(first, last - first + 1);
        return fv.empty() ? "1.0" : fv;
    }

    const MachineConfig& cfg_;
    std::optional<std::string> targetVersion_;
};

}  // namespace machine_config
