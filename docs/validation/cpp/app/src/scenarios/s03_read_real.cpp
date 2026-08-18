// S-03: Read real AconityMIDI fixture
#include "s03_read_real.hpp"
#include "machine_config/machine_config.hpp"

#include <sstream>

namespace s03 {

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
        std::ostringstream oss;
        oss << "machine_name=\"" << cfg.meta.machine_name << "\""
            << " | file_version=\"" << cfg.meta.file_version << "\""
            << " | trains=" << cfg.optical_trains.size()
            << " | build_plate_x=" << (cfg.machine.build_plate_x ? std::to_string(*cfg.machine.build_plate_x) : "null")
            << " | build_plate_y=" << (cfg.machine.build_plate_y ? std::to_string(*cfg.machine.build_plate_y) : "null")
            << " | wd=" << (cfg.optical_trains[0].scanner.working_distance ? std::to_string(*cfg.optical_trains[0].scanner.working_distance) : "null")
            << " | rotation[0]=" << (cfg.optical_trains[0].scanner.scan_head_rotation ? std::to_string(*cfg.optical_trains[0].scanner.scan_head_rotation) : "null")
            << " | hash=" << cfg.meta.configuration_hash.substr(0, 16) << "...";
        return {true, oss.str()};
    } catch (const std::exception& e) {
        return {false, std::string("read failed: ") + e.what()};
    }
}

} // namespace s03
