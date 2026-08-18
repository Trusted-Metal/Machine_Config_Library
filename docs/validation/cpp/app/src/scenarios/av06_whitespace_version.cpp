// AV-06: Dispatcher normalizes whitespace in File_Version (" 1.0 ")
#include "av06_whitespace_version.hpp"
#include "machine_config/machine_config.hpp"

namespace av06 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "version_whitespace.h5");
    try {
        MachineConfigReader{fixture}.parse();
        return {true, "whitespace version ' 1.0 ' dispatched to v1.0 adapter, reads OK"};
    } catch (const std::exception& e) {
        return {false, e.what()};
    }
}

} // namespace av06
