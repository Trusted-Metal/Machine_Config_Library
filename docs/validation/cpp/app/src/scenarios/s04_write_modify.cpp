// S-04: Write modified config and verify field change survives roundtrip
#include "s04_write_modify.hpp"
#include "machine_config/machine_config.hpp"

namespace s04 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto path = fixturesDir / "reference_config.h5";
    try {
        MachineConfig cfg = MachineConfigReader{path}.parse();
        auto origX = cfg.machine.build_plate_x;

        MachineConfig modified = cfg;
        modified.meta.machine_name = "VALIDATION_TEST_MACHINE";
        modified.machine.machine_name = "VALIDATION_TEST_MACHINE";

        auto tmp = scenarios::makeTempPath(".h5");
        MachineConfigWriter{modified}.write(tmp);
        MachineConfig rb = MachineConfigReader{tmp}.parse();

        if (rb.meta.machine_name != "VALIDATION_TEST_MACHINE") {
            return {false, "machine_name not persisted: '" + rb.meta.machine_name + "'"};
        }
        if (rb.meta.file_version != "1.0") {
            return {false, "file_version changed: '" + rb.meta.file_version + "'"};
        }
        if (rb.machine.build_plate_x != origX) {
            return {false, "build_plate_x changed"};
        }

        return {true, "machine_name persisted, file_version and other fields unchanged"};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace s04
