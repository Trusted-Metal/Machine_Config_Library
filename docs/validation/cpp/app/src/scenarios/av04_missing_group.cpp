// AV-04: Reader returns an error when required group (Machine/) is absent
#include "av04_missing_group.hpp"
#include "machine_config/machine_config.hpp"

namespace av04 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "missing_machine_group.h5");
    try {
        MachineConfigReader{fixture}.parse();
        return {false, "no error raised for missing Machine/ group"};
    } catch (const std::exception& e) {
        return {true, std::string("error raised for missing Machine/ group: ") + e.what()};
    }
}

} // namespace av04
