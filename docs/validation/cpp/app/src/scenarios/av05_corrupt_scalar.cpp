// AV-05: Reader handles corrupt required attribute (Build_Plate_X_Dimension) gracefully
#include "av05_corrupt_scalar.hpp"
#include "machine_config/machine_config.hpp"

namespace av05 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "corrupt_scalar.h5");
    try {
        MachineConfigReader{fixture}.parse();
        return {false, "no error raised for corrupt Build_Plate_X_Dimension"};
    } catch (const std::exception& e) {
        return {true, std::string("error raised for corrupt scalar: ") + e.what()};
    }
}

} // namespace av05
