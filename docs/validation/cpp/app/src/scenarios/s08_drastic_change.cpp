// S-08: Drastic field change to real file, verify adapter pipeline integrity
#include "s08_drastic_change.hpp"
#include "machine_config/machine_config.hpp"

#include <cmath>

namespace s08 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path&, const std::filesystem::path& realDir) {
    std::filesystem::path matched;
    for (const auto& entry : std::filesystem::directory_iterator(realDir)) {
        auto name = entry.path().filename().string();
        if (entry.path().extension() == ".h5" &&
            name.find("AconityMIDI") != std::string::npos &&
            name.find("OG_178") != std::string::npos) {
            matched = entry.path();
            break;
        }
    }
    if (matched.empty()) {
        return {false, "real AconityMIDI file not found in " + realDir.string()};
    }

    try {
        MachineConfig cfg = MachineConfigReader{matched}.parse();

        MachineConfig modified = cfg;
        modified.meta.machine_name = "MODIFIED_ACONITY_VALIDATION";
        modified.machine.machine_name = "MODIFIED_ACONITY_VALIDATION";
        modified.machine.build_plate_x = 350.0;

        OpticalTrain newTrain = cfg.optical_trains[1];
        newTrain.train_id = "Optical_Train_03";
        newTrain.scanner.scan_head_rotation = 90.0;
        newTrain.optional_components.clearbox = std::nullopt;
        modified.optical_trains.push_back(newTrain);

        auto tmp = scenarios::makeTempPath(".h5");
        MachineConfigWriter{modified}.write(tmp);
        MachineConfig rb = MachineConfigReader{tmp}.parse();

        if (rb.optical_trains.size() != 3) {
            return {false, "optical_trains: expected 3, got " + std::to_string(rb.optical_trains.size())};
        }
        if (!rb.machine.build_plate_x || std::abs(*rb.machine.build_plate_x - 350.0) > 0.001) {
            return {false, "build_plate_x mismatch"};
        }
        const auto& r2 = rb.optical_trains[2].scanner.scan_head_rotation;
        if (!r2 || std::abs(*r2 - 90.0) > 0.001) {
            return {false, "train[2].scan_head_rotation mismatch"};
        }
        if (rb.optical_trains[2].optional_components.clearbox.has_value()) {
            return {false, "train[2].clearbox should be absent"};
        }
        if (rb.meta.machine_name != "MODIFIED_ACONITY_VALIDATION") {
            return {false, "machine_name: got '" + rb.meta.machine_name + "'"};
        }
        if (rb.meta.file_version != "1.0") {
            return {false, "file_version changed: '" + rb.meta.file_version + "'"};
        }

        return {true, "3 trains, build_plate_x=350.0, rotation=90.0, clearbox cleared, machine_name OK"};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace s08
