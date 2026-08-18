// Machine Config Library — C++ Quickstart
//
// Run from the repo root after building:
//
//   cmake --build cpp/build --config Release
//   cpp/build/quickstart          (Linux)
//   cpp\build\Release\quickstart  (Windows)
//
// Opens examples/dummy_2train.h5 via the stable model facade.

#include "machine_config/machine_config.hpp"

#include <filesystem>
#include <iostream>
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

        std::cout << "=== Machine Config Quickstart ===\n\n";
        std::cout << "File version   : " << file->fileVersion() << "\n";
        auto meta = file->getMeta();
        std::cout << "Machine name   : " << meta.machine_name << "\n";
        std::cout << "Optical trains : " << file->opticalTrainCount() << "\n";

        for (std::size_t i = 0; i < file->opticalTrainCount(); ++i) {
            auto sc = file->getScanner(i);
            if (!sc.ok()) {
                std::cerr << sc.errorMessage() << "\n";
                return 1;
            }
            std::cout << "  Train " << i << "  wd=" << fmtOpt(sc.value().working_distance)
                      << " " << sc.value().working_distance_unit.value_or("")
                      << "  offset x=" << fmtOpt(sc.value().scan_head_offset_x)
                      << ", y=" << fmtOpt(sc.value().scan_head_offset_y) << "\n";
            if (file->hasOptionalComponents(i)) {
                auto cb = file->getClearbox(i);
                std::cout << "           clearbox: " << (cb.ok() ? "present" : cb.errorCode())
                          << "\n";
            } else {
                std::cout << "           optionalComponents: none\n";
            }
        }

        auto scanner = file->getScanner(0);
        if (!scanner.ok()) {
            std::cerr << scanner.errorMessage() << "\n";
            return 1;
        }

        MachineConfigReader reader{dummy.string()};
        auto cd = reader.getCorrectionData(0);
        std::cout << "Correction grid: [" << cd.shape[0] << ", " << cd.shape[1] << ", "
                  << cd.shape[2] << "]   (train 0)\n\n";

        auto tmp = std::filesystem::temp_directory_path() / "mc_quickstart_tmp.h5";
        auto setR = file->setScanner(0, scanner.value(), SetMode::Merge);
        if (!setR.ok()) {
            std::cerr << "setScanner failed: " << setR.errorMessage() << "\n";
            return 1;
        }
        std::string out = tmp.string();
        auto saved = file->save(&out);
        if (!saved.ok()) {
            std::cerr << "save failed: " << saved.errorMessage() << "\n";
            return 1;
        }
        std::cout << "Written to     : " << tmp.filename().string() << "\n\n";

        auto again = MachineConfigFileV1_0::open(tmp);
        if (!again.ok()) {
            std::cerr << again.errorMessage() << "\n";
            return 1;
        }
        std::vector<std::string> failures;
        if (again.value()->getMeta().machine_name != meta.machine_name)
            failures.push_back("  machine_name mismatch after round-trip");
        if (again.value()->opticalTrainCount() != file->opticalTrainCount())
            failures.push_back("  train_count mismatch after round-trip");
        auto wd2 = again.value()->getScanner(0);
        if (!wd2.ok() || wd2.value().working_distance != scanner.value().working_distance)
            failures.push_back("  working_distance mismatch after round-trip");

        file->close();
        again.value()->close();
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
