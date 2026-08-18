// S-01: Read reference fixture, all scalar fields
//
// Expected: machine_name = "TM-LPBF-02: AconityMIDI+_OG"
//           build_plate_x ~= 250.0, build_plate_y ~= 250.0
//           len(optical_trains) = 2
//           train[0].scanner.working_distance ~= 670.0
//           train[0].scanner.scan_head_rotation ~= 0.0
//           train[1].scanner.scan_head_rotation ~= 180.0
//           configuration_hash: 64 hex characters
//           file_version: "1.0"

#include "s01_read_scalars.hpp"
#include "machine_config/machine_config.hpp"

#include <algorithm>
#include <cctype>
#include <cmath>

namespace s01 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto path = fixturesDir / "reference_config.h5";
    MachineConfig cfg;
    try {
        cfg = MachineConfigReader{path}.parse();
    } catch (const std::exception& e) {
        return {false, std::string("read failed: ") + e.what()};
    }

    if (cfg.meta.machine_name != "TM-LPBF-02: AconityMIDI+_OG") {
        return {false, "machine_name: got '" + cfg.meta.machine_name + "'"};
    }
    if (!cfg.machine.build_plate_x || std::abs(*cfg.machine.build_plate_x - 250.0) > 0.001) {
        return {false, "build_plate_x mismatch"};
    }
    if (!cfg.machine.build_plate_y || std::abs(*cfg.machine.build_plate_y - 250.0) > 0.001) {
        return {false, "build_plate_y mismatch"};
    }
    if (cfg.optical_trains.size() != 2) {
        return {false, "optical_trains count: got " + std::to_string(cfg.optical_trains.size())};
    }

    const auto& wd = cfg.optical_trains[0].scanner.working_distance;
    if (!wd || std::abs(*wd - 670.0) > 0.1) {
        return {false, "train[0].working_distance mismatch"};
    }
    const auto& r0 = cfg.optical_trains[0].scanner.scan_head_rotation;
    if (!r0 || std::abs(*r0) > 0.001) {
        return {false, "train[0].scan_head_rotation mismatch"};
    }
    const auto& r1 = cfg.optical_trains[1].scanner.scan_head_rotation;
    if (!r1 || std::abs(*r1 - 180.0) > 0.001) {
        return {false, "train[1].scan_head_rotation mismatch"};
    }

    const auto& h = cfg.meta.configuration_hash;
    bool allHex = h.size() == 64 &&
        std::all_of(h.begin(), h.end(), [](unsigned char c) { return std::isxdigit(c); });
    if (!allHex) {
        return {false, "configuration_hash invalid: '" + h + "'"};
    }
    if (cfg.meta.file_version != "1.0") {
        return {false, "file_version: got '" + cfg.meta.file_version + "'"};
    }

    return {true, "all scalar fields match expected values"};
}

} // namespace s01
