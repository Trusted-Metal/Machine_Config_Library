// S-05: Full binary roundtrip with correction hash verification
//
// parseWithBinary(), not parse(): this library splits scalars-only vs
// scalars+binary reads (like Rust, unlike Python's single always-binary
// parse()) — writing a config read via plain parse() would zero-fill the
// correction grid by design (see capabilities/v1_0/writer.hpp's
// "Absent Grid3D becomes a zero-filled default" comment).
#include "s05_binary_roundtrip.hpp"
#include "machine_config/machine_config.hpp"

namespace s05 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto path = fixturesDir / "reference_config.h5";
    try {
        MachineConfigReader reader{path};
        MachineConfig cfg = reader.parseWithBinary();

        CorrectionData cdBefore = reader.getCorrectionData(0);
        CorrectionData icdBefore = reader.getInverseCorrectionData(0);

        auto tmp = scenarios::makeTempPath(".h5");
        MachineConfigWriter{cfg}.write(tmp);

        MachineConfigReader reader2{tmp};
        CorrectionData cdAfter = reader2.getCorrectionData(0);
        CorrectionData icdAfter = reader2.getInverseCorrectionData(0);

        if (!scenarios::bitwiseEqual(cdBefore.data, cdAfter.data)) {
            return {false, "correction_data mismatch after roundtrip: " +
                scenarios::sha256Hex(cdBefore.data).substr(0, 16) + "... -> " +
                scenarios::sha256Hex(cdAfter.data).substr(0, 16) + "..."};
        }
        if (!scenarios::bitwiseEqual(icdBefore.data, icdAfter.data)) {
            return {false, "inverse_correction_data mismatch after roundtrip"};
        }

        return {true, "correction_data preserved: SHA-256=" + scenarios::sha256Hex(cdBefore.data)};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace s05
