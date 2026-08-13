#pragma once
// Public HDF5 reader: peek File_Version, then dispatch to a version adapter.
// On-disk group paths and HDF5 attribute names live in the matching adapter.

#include "machine_config/models.hpp"
#include "machine_config/capabilities/v1_0/hdf5.hpp"

#include <highfive/H5File.hpp>

#include <filesystem>
#include <stdexcept>
#include <string>
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

class MachineConfigReader {
public:
    explicit MachineConfigReader(std::filesystem::path path)
        : path_(std::move(path)) {}

    MachineConfig parse() const { return adapter().parse(); }

    MachineConfig parseWithBinary() const { return adapter().parseWithBinary(); }

    std::string toJson(int indent = 2, bool include_binary = false) const {
        return adapter().toJson(indent, include_binary);
    }

    nlohmann::json getRawGroup(const std::string& hdf5_path) const {
        return adapter().getRawGroup(hdf5_path);
    }

    CorrectionData getCorrectionData(size_t train_index) const {
        return adapter().getCorrectionData(train_index);
    }

    CorrectionData getInverseCorrectionData(size_t train_index) const {
        return adapter().getInverseCorrectionData(train_index);
    }

    std::vector<uint8_t> getScanFieldCorrectionBytes(size_t train_index) const {
        return adapter().getScanFieldCorrectionBytes(train_index);
    }

private:
    std::filesystem::path path_;

    capabilities::v1_0::Hdf5AdapterV1_0 adapter() const {
        std::string ver = peekFileVersion(path_);
        if (ver != "1.0") {
            throw std::runtime_error(
                "No adapter registered for File_Version \"" + ver + "\"");
        }
        return capabilities::v1_0::Hdf5AdapterV1_0(path_);
    }
};

}  // namespace machine_config
