#pragma once
// Shared abstract interfaces for File_Version reader/writer adapters
// (DISPATCH_REGISTRY_PLAN.md). Deliberately its own header, not folded into
// reader.hpp/writer.hpp: concrete adapters under capabilities/v1_0/ need to
// inherit from these, while reader.hpp/writer.hpp need to include the
// concrete adapters to build their production registries — putting the
// interface in either of those would create a circular #include.

#include "machine_config/models.hpp"

#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

namespace machine_config {

// Mirrors Hdf5AdapterV1_0's current public surface exactly (capabilities/v1_0/hdf5.hpp).
class ReaderAdapter {
public:
    virtual ~ReaderAdapter() = default;
    virtual MachineConfig parse() const = 0;
    virtual MachineConfig parseWithBinary() const = 0;
    virtual std::string toJson(int indent, bool include_binary) const = 0;
    virtual nlohmann::json getRawGroup(const std::string& hdf5_path) const = 0;
    virtual CorrectionData getCorrectionData(size_t train_index) const = 0;
    virtual CorrectionData getInverseCorrectionData(size_t train_index) const = 0;
    virtual std::vector<uint8_t> getScanFieldCorrectionBytes(size_t train_index) const = 0;
};

// Mirrors Hdf5WriterV1_0's current public surface exactly (capabilities/v1_0/writer.hpp).
class WriterAdapter {
public:
    virtual ~WriterAdapter() = default;
    virtual void write(std::filesystem::path path) const = 0;
};

}  // namespace machine_config
