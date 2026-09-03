// Machine Config Library — C++ Full Workflow: Calibration Adjustment
//
// Run from the repo root after building:
//
//   cmake --build cpp/build --config Release
//   cpp/build/full_workflow          (Linux)
//   cpp\build\Release\full_workflow  (Windows)
//
// Load examples/dummy_2train.h5, apply new scanner offsets via setScanner(Merge).

#include "machine_config/machine_config.hpp"

#include <array>
#include <filesystem>
#include <iostream>
#include <optional>
#include <sstream>
#include <string>
#include <vector>

#ifndef EXAMPLES_DIR
#  error "EXAMPLES_DIR must be defined by CMakeLists.txt"
#endif

using machine_config::MachineConfigReader;
using machine_config::capabilities::MachineConfigFileV1_0;
using machine_config::capabilities::SetMode;

static std::string fmtOpt(std::optional<double> v) {
    if (!v) return "null";
    std::ostringstream ss;
    ss << *v;
    return ss.str();
}

int main() {
    const std::filesystem::path dummy{EXAMPLES_DIR "/dummy_2train.h5"};

    if (!std::filesystem::exists(dummy)) {
        std::cerr << "Dummy file not found: " << dummy
                  << "\nRun: python examples/generate_dummy.py\n";
        return 1;
    }

    try {
        auto opened = MachineConfigFileV1_0::open(dummy);
        if (!opened.ok()) {
            std::cerr << "open failed: " << opened.errorMessage() << "\n";
            return 1;
        }
        auto file = opened.value();

        std::cout << "=== Full Workflow: Calibration Adjustment ===\n\n";
        std::cout << "Machine : " << file->getMeta().machine_name << "\n";
        std::cout << "Trains  : " << file->opticalTrainCount() << "\n\n";
        std::cout << "Before calibration:\n";

        MachineConfigReader reader{dummy.string()};
        for (std::size_t i = 0; i < file->opticalTrainCount(); ++i) {
            auto s = file->getScanner(i);
            if (!s.ok()) {
                std::cerr << s.errorMessage() << "\n";
                return 1;
            }
            std::cout << "  Train " << (i + 1)
                      << "  offset x=" << fmtOpt(s.value().scan_head_offset_x)
                      << ", y=" << fmtOpt(s.value().scan_head_offset_y) << "\n";
            auto cd = reader.getCorrectionData(i);
            std::cout << "           correction grid " << cd.shape[0] << "x"
                      << cd.shape[1] << "x" << cd.shape[2] << "\n";
        }
        std::cout << "\n";

        const std::array<std::pair<double, double>, 2> newOffsets{{{-91.5, 24.0}, {91.5, -24.0}}};
        for (std::size_t i = 0; i < newOffsets.size(); ++i) {
            auto sc = file->getScanner(i);
            if (!sc.ok()) {
                std::cerr << sc.errorMessage() << "\n";
                return 1;
            }
            auto patched = sc.value();
            patched.scan_head_offset_x = newOffsets[i].first;
            patched.scan_head_offset_y = newOffsets[i].second;
            auto setR = file->setScanner(i, patched, SetMode::Merge);
            if (!setR.ok()) {
                std::cerr << setR.errorMessage() << "\n";
                return 1;
            }
        }

        auto tmp = std::filesystem::temp_directory_path() / "mc_full_workflow_tmp.h5";
        std::string out = tmp.string();
        auto saved = file->save(&out);
        if (!saved.ok()) {
            std::cerr << "save failed: " << saved.errorMessage() << "\n";
            return 1;
        }
        file->close();
        std::cout << "Written to : " << tmp.filename().string() << "\n\n";

        auto again = MachineConfigFileV1_0::open(tmp);
        if (!again.ok()) {
            std::cerr << again.errorMessage() << "\n";
            return 1;
        }
        auto updated = again.value();
        MachineConfigReader reader2{tmp.string()};
        std::vector<std::string> failures;

        for (std::size_t i = 0; i < newOffsets.size(); ++i) {
            double ex = newOffsets[i].first, ey = newOffsets[i].second;
            auto s = updated->getScanner(i);
            if (!s.ok()) {
                failures.push_back("  train" + std::to_string(i + 1) + ": " + s.errorMessage());
                continue;
            }
            auto got_x = s.value().scan_head_offset_x;
            auto got_y = s.value().scan_head_offset_y;
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
                if (cd.shape[0] != 257 || cd.shape[1] != 257 || cd.shape[2] != 2)
                    failures.push_back("  train" + std::to_string(i + 1) +
                                       " correction shape mismatch");
            } catch (const std::exception& e) {
                failures.push_back("  train" + std::to_string(i + 1) +
                                   " correction read error: " + e.what());
            }
        }

        std::cout << "After calibration:\n";
        for (std::size_t i = 0; i < updated->opticalTrainCount(); ++i) {
            auto s = updated->getScanner(i);
            if (!s.ok()) continue;
            std::cout << "  Train " << (i + 1)
                      << "  offset x=" << fmtOpt(s.value().scan_head_offset_x)
                      << ", y=" << fmtOpt(s.value().scan_head_offset_y) << "\n";
        }
        std::cout << "\n";

        updated->close();
        std::filesystem::remove(tmp);

        if (!failures.empty()) {
            std::cout << "FAIL\n";
            for (const auto& msg : failures) std::cout << msg << "\n";
            return 1;
        }
        std::cout << "PASS\n";
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << "\n";
        return 1;
    }
}
