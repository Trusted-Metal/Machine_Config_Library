// Machine Config Library — C++ Full Workflow: Calibration Adjustment
//
// Run from the repo root after building:
//
//   cmake --build cpp/build --config Release
//   cpp/build/full_workflow          (Linux)
//   cpp\build\Release\full_workflow  (Windows)
//
// Scenario: a field calibration measured new scanner-head positions for both
// optical trains.  Load the current machine config, apply the updated offsets,
// write the modified config to a new file, and verify the changes persisted
// alongside the binary correction data.

#include "machine_config/reader.hpp"
#include "machine_config/writer.hpp"

#include <array>
#include <filesystem>
#include <iostream>
#include <optional>
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
        // ---------------------------------------------------------------------
        // 1. Print pre-calibration summary
        // ---------------------------------------------------------------------
        MachineConfigReader reader{fixture};
        auto cfg = reader.parse();

        std::cout << "=== Full Workflow: Calibration Adjustment ===\n\n";
        std::cout << "Machine : " << cfg.meta.machine_name << "\n";
        std::cout << "Trains  : " << cfg.optical_trains.size() << "\n\n";
        std::cout << "Before calibration:\n";

        for (size_t i = 0; i < cfg.optical_trains.size(); ++i) {
            const auto& s = cfg.optical_trains[i].scanner;
            std::cout << "  Train " << (i + 1)
                      << "  offset x=" << fmtOpt(s.scan_head_offset_x)
                      << ", y=" << fmtOpt(s.scan_head_offset_y) << "\n";
            auto cd = reader.getCorrectionData(i);
            std::cout << "           correction grid "
                      << cd.shape[0] << "x"
                      << cd.shape[1] << "x"
                      << cd.shape[2] << "\n";
        }
        std::cout << "\n";

        // ---------------------------------------------------------------------
        // 2. Apply new scanner offsets (post-calibration values)
        // ---------------------------------------------------------------------
        const std::array<std::pair<double, double>, 2> newOffsets{{{-91.5, 24.0}, {91.5, -24.0}}};
        for (size_t i = 0; i < newOffsets.size(); ++i) {
            cfg.optical_trains[i].scanner.scan_head_offset_x = newOffsets[i].first;
            cfg.optical_trains[i].scanner.scan_head_offset_y = newOffsets[i].second;
        }

        // ---------------------------------------------------------------------
        // 3. Write updated config
        // ---------------------------------------------------------------------
        auto tmp = std::filesystem::temp_directory_path() / "mc_full_workflow_tmp.h5";
        MachineConfigWriter{cfg}.write(tmp);
        std::cout << "Written to : " << tmp.filename().string() << "\n\n";

        // ---------------------------------------------------------------------
        // 4. Read back and verify
        // ---------------------------------------------------------------------
        MachineConfigReader reader2{tmp};
        auto updated = reader2.parse();
        std::vector<std::string> failures;

        for (size_t i = 0; i < newOffsets.size(); ++i) {
            double ex = newOffsets[i].first, ey = newOffsets[i].second;
            auto got_x = updated.optical_trains[i].scanner.scan_head_offset_x;
            auto got_y = updated.optical_trains[i].scanner.scan_head_offset_y;
            if (!got_x || *got_x != ex)
                failures.push_back("  train" + std::to_string(i + 1) +
                                   " offset_x: expected " + std::to_string(ex) +
                                   ", got " + fmtOpt(got_x));
            if (!got_y || *got_y != ey)
                failures.push_back("  train" + std::to_string(i + 1) +
                                   " offset_y: expected " + std::to_string(ey) +
                                   ", got " + fmtOpt(got_y));
            try {
                auto cd = reader2.getCorrectionData(i);
                const std::array<size_t, 3> expected{257, 257, 2};
                if (cd.shape != expected)
                    failures.push_back("  train" + std::to_string(i + 1) +
                                       " correction shape: expected {257,257,2}, got {" +
                                       std::to_string(cd.shape[0]) + "," +
                                       std::to_string(cd.shape[1]) + "," +
                                       std::to_string(cd.shape[2]) + "}");
            } catch (const std::exception& e) {
                failures.push_back("  train" + std::to_string(i + 1) +
                                   " correction read error: " + e.what());
            }
        }

        std::filesystem::remove(tmp);

        std::cout << "After calibration:\n";
        for (size_t i = 0; i < updated.optical_trains.size(); ++i) {
            const auto& s = updated.optical_trains[i].scanner;
            std::cout << "  Train " << (i + 1)
                      << "  offset x=" << fmtOpt(s.scan_head_offset_x)
                      << ", y=" << fmtOpt(s.scan_head_offset_y) << "\n";
        }
        std::cout << "\n";

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
