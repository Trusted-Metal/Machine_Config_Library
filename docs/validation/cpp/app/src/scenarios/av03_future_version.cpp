// AV-03: v1.0 reader encountering a v1.1 file fails loudly
#include "av03_future_version.hpp"
#include "machine_config/machine_config.hpp"

namespace av03 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "v1_1_simulated.h5");
    try {
        MachineConfigReader{fixture}.parse();
        return {false, "no error raised for File_Version='1.1'"};
    } catch (const std::exception& e) {
        std::string msg = e.what();
        if (msg.find("1.1") != std::string::npos) {
            return {true, std::string("error raised naming version '1.1': ") + msg};
        }
        return {false, std::string("error raised but did not name version '1.1': ") + msg};
    }
}

} // namespace av03
