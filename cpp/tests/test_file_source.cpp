// FileSource: reading an MCF through a file-access property list hook instead of a plain path.
// This is how an MCF stored inside a Lossless Build Log (.lbl) container is read in place (the
// hook installs the LBL VFD); here an HDF5 in-memory file image stands in for the container so
// the test needs no extra library.
#include <catch2/catch_test_macros.hpp>
#include <nlohmann/json.hpp>

#include <hdf5.h>

#include <filesystem>
#include <fstream>
#include <iterator>
#include <string>
#include <vector>

#include "machine_config/capabilities.hpp"
#include "machine_config/reader.hpp"

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by tests/CMakeLists.txt"
#endif

using namespace machine_config;

namespace {

const std::string REF_V1_0 = std::string(FIXTURES_DIR) + "/reference_config.h5";
const std::string REF_V1_1 = std::string(FIXTURES_DIR) + "/reference_config_v1_1.h5";

std::vector<char> slurp(const std::string& path) {
    std::ifstream in(path, std::ios::binary);
    return {std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>()};
}

// Hook that serves `bytes` as the file: HDF5 ignores the path when a core-driver image is set.
// Each image gets its own name: HDF5 treats core files with equal names as the same open file.
FileSource imageSource(const std::vector<char>& bytes, int* calls = nullptr) {
    static int next = 0;
    return FileSource{"not-on-disk-" + std::to_string(next++) + ".h5", [&bytes, calls](hid_t fapl) {
                          if (calls) ++*calls;
                          REQUIRE(H5Pset_fapl_core(fapl, 1 << 20, 0) >= 0);
                          REQUIRE(H5Pset_file_image(fapl, const_cast<char*>(bytes.data()), bytes.size()) >= 0);
                      }};
}

}  // namespace

TEST_CASE("FileSourcePathIsImplicit") {
    FileSource a = REF_V1_0;
    FileSource b = std::filesystem::path(REF_V1_0);
    FileSource c = REF_V1_0.c_str();
    REQUIRE_FALSE(a.redirected());
    REQUIRE(b.path == c.path);
    const std::filesystem::path& p = a;
    REQUIRE(p == std::filesystem::path(REF_V1_0));
    REQUIRE(savePath(a) == a.path);
}

TEST_CASE("FileSourceHookReaderMatchesPath") {
    for (const auto& ref : {REF_V1_0, REF_V1_1}) {
        auto bytes = slurp(ref);
        REQUIRE(bytes.size() > 1000);
        int calls = 0;
        MachineConfigReader viaHook{imageSource(bytes, &calls)};
        auto cfg = viaHook.parse();
        nlohmann::json a = cfg;
        nlohmann::json b = MachineConfigReader{ref}.parse();
        REQUIRE(a == b);
        REQUIRE(calls >= 2);  // File_Version peek + parse
        if (!cfg.optical_trains.empty() && cfg.optical_trains[0].scan_field_correction_file)
            REQUIRE(viaHook.getScanFieldCorrectionBytes(0) == MachineConfigReader{ref}.getScanFieldCorrectionBytes(0));
    }
}

TEST_CASE("FileSourceHookCapabilityOpenIsReadOnly") {
    auto bytes = slurp(REF_V1_1);
    auto viaHook = capabilities::openMachineConfig(imageSource(bytes));
    REQUIRE(viaHook.ok());
    auto direct = capabilities::openMachineConfig(REF_V1_1);
    REQUIRE(direct.ok());
    REQUIRE(viaHook.value()->fileVersion() == "1.1");
    REQUIRE(nlohmann::json(viaHook.value()->getMeta()) == nlohmann::json(direct.value()->getMeta()));
    REQUIRE(viaHook.value()->opticalTrainCount() == direct.value()->opticalTrainCount());
    // Never write back through the hook's path (it may name the container that holds the MCF).
    auto saved = viaHook.value()->save();
    REQUIRE_FALSE(saved.ok());
    REQUIRE(saved.errorCode() == "ValidationError");
}

TEST_CASE("FileSourceHookErrorsSurface") {
    FileSource broken{"not-on-disk.h5", [](hid_t) { throw std::runtime_error("hook failed"); }};
    auto r = capabilities::openMachineConfig(broken);
    REQUIRE_FALSE(r.ok());
    REQUIRE(r.errorCode() == "IoError");
    REQUIRE_THROWS(MachineConfigReader{broken}.parse());
}
