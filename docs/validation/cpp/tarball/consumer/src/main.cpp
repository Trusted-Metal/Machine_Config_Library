// Minimal external consumer of the installed MachineConfig package.
// No reference to the repo's source tree anywhere in this file or its
// CMakeLists.txt — proves the packaged headers + Config.cmake are actually
// self-sufficient, not secretly relying on the source tree sitting next
// to it (which is what the from-source app at ../Cpp/ already covers).
#include "machine_config/machine_config.hpp"

#include <iostream>

int main(int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "Usage: consumer_app <reference_config.h5>\n";
        return 1;
    }
    try {
        machine_config::MachineConfig cfg =
            machine_config::MachineConfigReader{argv[1]}.parse();
        std::cout << "[PASS] machine_name=\"" << cfg.meta.machine_name << "\""
                  << " file_version=\"" << cfg.meta.file_version << "\""
                  << " trains=" << cfg.optical_trains.size()
                  << " package_version=\"" << machine_config::kVersionString << "\"\n";
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "[FAIL] " << e.what() << "\n";
        return 1;
    }
}
