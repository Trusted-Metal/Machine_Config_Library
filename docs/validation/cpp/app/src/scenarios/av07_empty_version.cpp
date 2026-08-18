// AV-07: Dispatcher handles empty string File_Version
//
// Either outcome is acceptable per VALIDATION_PLAN.md — this records the
// actual behavior rather than asserting one, mirroring every other
// language's AV-07.
#include "av07_empty_version.hpp"
#include "machine_config/machine_config.hpp"

namespace av07 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "empty_version.h5");
    try {
        MachineConfig cfg = MachineConfigReader{fixture}.parse();
        return {true, "empty File_Version dispatches OK, file_version='" + cfg.meta.file_version + "'"};
    } catch (const std::exception& e) {
        return {true, std::string("empty File_Version raises: ") + e.what()};
    }
}

} // namespace av07
