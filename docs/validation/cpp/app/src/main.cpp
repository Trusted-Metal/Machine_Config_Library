#include "scenarios/common.hpp"
#include "scenarios/s01_read_scalars.hpp"
#include "scenarios/s02_read_binary.hpp"
#include "scenarios/s03_read_real.hpp"
#include "scenarios/s04_write_modify.hpp"
#include "scenarios/s05_binary_roundtrip.hpp"
#include "scenarios/s06_builder.hpp"
#include "scenarios/s07_opcua.hpp"
#include "scenarios/s08_drastic_change.hpp"
#include "scenarios/s09_type_exports.hpp"
#include "scenarios/av01_unknown_version.hpp"
#include "scenarios/av02_missing_version.hpp"
#include "scenarios/av03_future_version.hpp"
#include "scenarios/av04_missing_group.hpp"
#include "scenarios/av05_corrupt_scalar.hpp"
#include "scenarios/av06_whitespace_version.hpp"
#include "scenarios/av07_empty_version.hpp"
#include "scenarios/av08_version_fidelity.hpp"

#include <iostream>
#include <vector>

using RunFn = scenarios::Result (*)(const std::filesystem::path&, const std::filesystem::path&);

int main(int argc, char** argv) {
    if (argc != 3) {
        std::cerr << "Usage: validation_app <fixtures_dir> <real_dir>\n";
        return 1;
    }
    std::filesystem::path fixturesDir = argv[1];
    std::filesystem::path realDir = argv[2];

    std::vector<std::pair<const char*, RunFn>> scenarioList = {
        {"S-01", s01::run}, {"S-02", s02::run}, {"S-03", s03::run}, {"S-04", s04::run},
        {"S-05", s05::run}, {"S-06", s06::run}, {"S-07", s07::run}, {"S-08", s08::run},
        {"S-09", s09::run},
        {"AV-01", av01::run}, {"AV-02", av02::run}, {"AV-03", av03::run}, {"AV-04", av04::run},
        {"AV-05", av05::run}, {"AV-06", av06::run}, {"AV-07", av07::run}, {"AV-08", av08::run},
    };

    int passed = 0, failed = 0;
    for (const auto& [id, run] : scenarioList) {
        scenarios::Result r;
        try {
            r = run(fixturesDir, realDir);
        } catch (const std::exception& e) {
            r = {false, std::string("EXCEPTION: ") + e.what()};
        }
        std::cout << (r.passed ? "[PASS] " : "[FAIL] ") << id << ": " << r.detail << "\n";
        r.passed ? ++passed : ++failed;
    }

    std::cout << "\n" << (passed + failed) << " scenarios: " << passed << " passed, " << failed << " failed\n";
    return failed == 0 ? 0 : 1;
}
