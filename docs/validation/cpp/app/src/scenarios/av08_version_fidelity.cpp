// AV-08: File_Version string survives write -> read unchanged
#include "av08_version_fidelity.hpp"
#include "machine_config/machine_config.hpp"

namespace av08 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto path = fixturesDir / "reference_config.h5";
    try {
        MachineConfig cfg = MachineConfigReader{path}.parse();
        std::string origVersion = cfg.meta.file_version;

        auto tmp = scenarios::makeTempPath(".h5");
        MachineConfigWriter{cfg}.write(tmp);
        MachineConfig rb = MachineConfigReader{tmp}.parse();
        std::string rbVersion = rb.meta.file_version;

        if (rbVersion != "1.0") {
            return {false, "file_version after roundtrip: expected '1.0', got '" + rbVersion + "'"};
        }
        if (rbVersion != origVersion) {
            return {false, "file_version changed: '" + origVersion + "' -> '" + rbVersion + "'"};
        }

        return {true, "File_Version survives roundtrip unchanged: '" + rbVersion + "'"};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace av08
