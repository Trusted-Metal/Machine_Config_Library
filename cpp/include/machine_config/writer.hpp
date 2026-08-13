#pragma once
// Public HDF5 writer: dispatch to a File_Version adapter.
// On-disk group paths and HDF5 attribute names live in the matching adapter.

#include "machine_config/models.hpp"
#include "machine_config/capabilities/v1_0/writer.hpp"

#include <filesystem>
#include <stdexcept>
#include <string>

namespace machine_config {

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
        if (fv == "1.0") {
            capabilities::v1_0::Hdf5WriterV1_0{cfg_}.write(std::move(path));
            return;
        }
        throw std::runtime_error(
            "No adapter registered for File_Version \"" + fv + "\"");
    }

private:
    const MachineConfig& cfg_;
};

}  // namespace machine_config
