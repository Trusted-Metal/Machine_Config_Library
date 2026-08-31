#pragma once
// Public HDF5 writer: dispatch to a File_Version adapter.
// On-disk group paths and HDF5 attribute names live in the matching adapter.

#include "machine_config/adapters.hpp"
#include "machine_config/models.hpp"
#include "machine_config/capabilities/v1_0/writer.hpp"

#include <filesystem>
#include <functional>
#include <memory>
#include <stdexcept>
#include <string>
#include <unordered_map>

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

    void write(std::filesystem::path path) const {
        std::string fv = cfg_.meta.file_version;
        auto first = fv.find_first_not_of(" \t\r\n");
        if (first == std::string::npos) {
            fv = "1.0";
        } else {
            auto last = fv.find_last_not_of(" \t\r\n");
            fv = fv.substr(first, last - first + 1);
            if (fv.empty()) fv = "1.0";
        }
        resolveWriter(fv, cfg_, productionWriterRegistry())->write(std::move(path));
    }

private:
    const MachineConfig& cfg_;
};

}  // namespace machine_config
