// AV-02: Reader handles absent File_Version attribute predictably
//
// Either outcome (defaults to v1.0 dispatch, or a typed error) is acceptable
// per VALIDATION_PLAN.md — this records the actual behavior rather than
// asserting one, mirroring every other language's AV-02.
#include "av02_missing_version.hpp"
#include "machine_config/machine_config.hpp"

namespace av02 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "missing_version.h5");
    try {
        MachineConfig cfg = MachineConfigReader{fixture}.parse();
        return {true, "missing File_Version dispatches OK, file_version='" + cfg.meta.file_version + "'"};
    } catch (const std::exception& e) {
        return {true, std::string("missing File_Version raises: ") + e.what()};
    }
}

} // namespace av02
