// Machine Config Library — C++ Quickstart
//
// Run from the repo root after building:
//
//   cmake --build cpp/build --config Release
//   cpp/build/quickstart          (Linux)
//   cpp\build\Release\quickstart  (Windows)
//
// Demonstrates the six essential operations:
//   1. Open an HDF5 machine config file
//   2. Read scalar fields (machine name, train count, working distance)
//   3. Inspect binary data shape (ClearBox correction grid)
//   4. Write the config to a temporary HDF5 file
//   5. Read the temporary file back
//   6. Assert round-trip fidelity and print PASS / FAIL

#include "machine_config/reader.hpp"
#include "machine_config/writer.hpp"

#include <filesystem>
#include <iostream>
#include <sstream>
#include <string>
#include <vector>

#ifndef FIXTURES_DIR
#  error "FIXTURES_DIR must be defined by CMakeLists.txt"
#endif

using namespace machine_config;

static std::string fmtOpt(std::optional<double> v) {
    if (!v) return "null";
    std::ostringstream ss;
    ss << *v;
    return ss.str();
}

int main() {
    const std::filesystem::path fixture{FIXTURES_DIR "/reference_config.h5"};

    if (!std::filesystem::exists(fixture)) {
        std::cerr << "Fixture not found: " << fixture
                  << "\nEnsure fixtures/ is present.\n";
        return 1;
    }

    try {
        // Step 1 & 2 — Open and read scalar fields
        MachineConfigReader reader{fixture};
        auto config = reader.parse();

        std::cout << "=== Machine Config Quickstart ===\n\n";
        std::cout << "Machine name   : " << config.meta.machine_name << "\n";
        std::cout << "Optical trains : " << config.optical_trains.size() << "\n";

        const auto& train0 = config.optical_trains[0];
        std::cout << "Working dist   : "
                  << fmtOpt(train0.scanner.working_distance) << " "
                  << train0.scanner.working_distance_unit.value_or("") << "   (train 0)\n";

        // Step 3 — Binary data shape
        auto cd = reader.getCorrectionData(0);
        std::cout << "Correction grid: ["
                  << cd.shape[0] << ", " << cd.shape[1] << ", " << cd.shape[2]
                  << "]   (train 0)\n\n";

        // Step 4 — Write to a temporary file
        auto tmp = std::filesystem::temp_directory_path() / "mc_quickstart_tmp.h5";
        MachineConfigWriter{config}.write(tmp);
        std::cout << "Written to     : " << tmp.filename().string() << "\n\n";

        // Step 5 — Read back
        auto config2 = MachineConfigReader{tmp}.parse();

        // Step 6 — Assert round-trip fidelity
        std::vector<std::string> failures;

        if (config2.meta.machine_name != config.meta.machine_name)
            failures.push_back("  machine_name: expected \"" + config.meta.machine_name +
                               "\", got \"" + config2.meta.machine_name + "\"");

        if (config2.optical_trains.size() != config.optical_trains.size())
            failures.push_back("  train_count: expected " +
                               std::to_string(config.optical_trains.size()) +
                               ", got " + std::to_string(config2.optical_trains.size()));

        auto wd2 = config2.optical_trains[0].scanner.working_distance;
        if (wd2 != train0.scanner.working_distance)
            failures.push_back("  working_distance: expected " +
                               fmtOpt(train0.scanner.working_distance) +
                               ", got " + fmtOpt(wd2));

        std::filesystem::remove(tmp);

        if (!failures.empty()) {
            std::cout << "FAIL\n";
            for (const auto& msg : failures)
                std::cout << msg << "\n";
            return 1;
        }
        std::cout << "PASS\n";
        return 0;

    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << "\n";
        return 1;
    }
}
