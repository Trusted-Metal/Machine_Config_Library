// AV-01: Reader rejects unknown File_Version
//
// No typed exception hierarchy exists for the plain reader/writer (see
// VALIDATION_PLAN.md §9.5 "No typed exception hierarchy") — every error path
// throws a bare std::runtime_error. This inspects .what() for the version
// string rather than a typed error field.
#include "av01_unknown_version.hpp"
#include "machine_config/machine_config.hpp"

namespace av01 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto fixture = scenarios::avFixture(fixturesDir, "v2_0_unknown.h5");
    try {
        MachineConfigReader{fixture}.parse();
        return {false, "no error raised for File_Version='2.0'"};
    } catch (const std::exception& e) {
        std::string msg = e.what();
        if (msg.find("2.0") != std::string::npos) {
            return {true, std::string("error raised naming version '2.0': ") + msg};
        }
        return {false, std::string("error raised but did not name version '2.0': ") + msg};
    }
}

} // namespace av01
