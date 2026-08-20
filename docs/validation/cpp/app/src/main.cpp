#include "scenarios/common.hpp"
#include "scenarios/scenarios.hpp"

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
        {"AV-12", av12::run}, {"AV-13", av13::run},
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
