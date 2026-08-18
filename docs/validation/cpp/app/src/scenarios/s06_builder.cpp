// S-06: Build synthetic config with MockConfigBuilder and verify fields
#include "s06_builder.hpp"
#include "machine_config/machine_config.hpp"

#include <cmath>

namespace s06 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path&, const std::filesystem::path&) {
    try {
        MockConfigBuilder builder;
        builder.laser_count = 2;
        MachineConfig cfg = builder.build();

        if (cfg.optical_trains.size() != 2) {
            return {false, "optical_trains count: " + std::to_string(cfg.optical_trains.size())};
        }

        const auto& r0 = cfg.optical_trains[0].scanner.scan_head_rotation;
        const auto& r1 = cfg.optical_trains[1].scanner.scan_head_rotation;
        if (!r0 || std::abs(*r0) > 0.001) return {false, "train[0].scan_head_rotation mismatch"};
        if (!r1 || std::abs(*r1 - 180.0) > 0.001) return {false, "train[1].scan_head_rotation mismatch"};
        if (cfg.meta.machine_name.empty()) return {false, "machine_name is empty"};

        const auto& clearbox = cfg.optical_trains[0].optional_components.clearbox;
        if (!clearbox) return {false, "clearbox is absent"};
        if (!clearbox->correction_data) return {false, "correction_data is absent"};

        GridCell center = (*clearbox->correction_data)[128][128][0];
        if (!center || !std::isfinite(*center) || std::abs(*center - 2.0) > 0.01) {
            return {false, "correction_data center: expected ~2.0"};
        }

        auto tmp = scenarios::makeTempPath(".h5");
        MachineConfigWriter{cfg}.write(tmp);
        MachineConfig rb = MachineConfigReader{tmp}.parse();

        if (rb.optical_trains.size() != 2) {
            return {false, "readback trains: " + std::to_string(rb.optical_trains.size())};
        }
        if (rb.meta.machine_name != cfg.meta.machine_name) {
            return {false, "machine_name changed after roundtrip"};
        }

        return {true, "2-laser build OK, center~=2.0, roundtrip OK"};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace s06
