#pragma once
// Public HDF5 reader: peek File_Version, then dispatch to a version adapter.
// On-disk group paths and HDF5 attribute names live in the matching adapter.

#include "machine_config/adapters.hpp"
#include "machine_config/models.hpp"
#include "machine_config/capabilities/v1_0/hdf5.hpp"
#include "machine_config/capabilities/v1_1/hdf5.hpp"

#include <highfive/H5File.hpp>

#include <filesystem>
#include <functional>
#include <memory>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

namespace machine_config {

/// Read only the root File_Version attribute. Does not walk groups.
inline std::string peekFileVersion(const std::filesystem::path& path) {
    HighFive::File f(path.string(), HighFive::File::ReadOnly);
    if (!f.hasAttribute("File_Version")) return "1.0";
    std::string ver = readRequiredStr(f, "File_Version");
    auto first = ver.find_first_not_of(" \t\r\n");
    if (first == std::string::npos) return "1.0";
    auto last = ver.find_last_not_of(" \t\r\n");
    ver = ver.substr(first, last - first + 1);
    return ver.empty() ? "1.0" : ver;
}

// Version -> constructor for that version's ReaderAdapter. A real registry
// (DISPATCH_REGISTRY_PLAN.md), not a hardcoded check: adding a version means
// adding an entry here, never editing MachineConfigReader itself.
using ReaderRegistry =
    std::unordered_map<std::string,
                        std::function<std::unique_ptr<ReaderAdapter>(const std::filesystem::path&)>>;

inline const ReaderRegistry& productionReaderRegistry() {
    static const ReaderRegistry registry = {
        {"1.0", [](const std::filesystem::path& path) -> std::unique_ptr<ReaderAdapter> {
             return std::make_unique<capabilities::v1_0::Hdf5AdapterV1_0>(path);
         }},
        {"1.1", [](const std::filesystem::path& path) -> std::unique_ptr<ReaderAdapter> {
             return std::make_unique<capabilities::v1_1::Hdf5AdapterV1_1>(path);
         }},
    };
    return registry;
}

// Registry-parameterized so tests can inject a fake entry without touching
// global state — see DISPATCH_REGISTRY_PLAN.md's shared testing pattern.
inline std::unique_ptr<ReaderAdapter> resolveReader(
    const std::string& version, const std::filesystem::path& path, const ReaderRegistry& registry) {
    auto it = registry.find(version);
    if (it == registry.end()) {
        throw std::runtime_error(
            "No adapter registered for File_Version \"" + version + "\"");
    }
    return it->second(path);
}

class MachineConfigReader {
public:
    explicit MachineConfigReader(std::filesystem::path path)
        : path_(std::move(path)) {}

    MachineConfig parse() const { return adapter()->parse(); }

    MachineConfig parseWithBinary() const { return adapter()->parseWithBinary(); }

    std::string toJson(int indent = 2, bool include_binary = false) const {
        return adapter()->toJson(indent, include_binary);
    }

    nlohmann::json getRawGroup(const std::string& hdf5_path) const {
        return adapter()->getRawGroup(hdf5_path);
    }

    CorrectionData getCorrectionData(size_t train_index) const {
        return adapter()->getCorrectionData(train_index);
    }

    CorrectionData getInverseCorrectionData(size_t train_index) const {
        return adapter()->getInverseCorrectionData(train_index);
    }

    std::vector<uint8_t> getScanFieldCorrectionBytes(size_t train_index) const {
        return adapter()->getScanFieldCorrectionBytes(train_index);
    }

private:
    std::filesystem::path path_;

    std::unique_ptr<ReaderAdapter> adapter() const {
        std::string ver = peekFileVersion(path_);
        return resolveReader(ver, path_, productionReaderRegistry());
    }
};

}  // namespace machine_config
