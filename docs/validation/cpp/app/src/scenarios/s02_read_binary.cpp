// S-02: Read reference fixture with binary data (correction grids)
#include "s02_read_binary.hpp"
#include "machine_config/machine_config.hpp"

#include <algorithm>
#include <cmath>

namespace s02 {

using namespace machine_config;

scenarios::Result run(const std::filesystem::path& fixturesDir, const std::filesystem::path&) {
    auto path = fixturesDir / "reference_config.h5";
    try {
        MachineConfigReader reader{path};
        CorrectionData cd = reader.getCorrectionData(0);
        CorrectionData icd = reader.getInverseCorrectionData(0);

        if (cd.shape != std::array<std::size_t, 3>{257, 257, 2}) {
            return {false, "correction_data shape mismatch"};
        }
        if (icd.shape != std::array<std::size_t, 3>{257, 257, 2}) {
            return {false, "inverse_correction_data shape mismatch"};
        }
        if (!std::any_of(cd.data.begin(), cd.data.end(), [](double v) { return std::isfinite(v); })) {
            return {false, "correction_data: no finite values"};
        }
        if (!std::any_of(icd.data.begin(), icd.data.end(), [](double v) { return std::isfinite(v); })) {
            return {false, "inverse_correction_data: no finite values"};
        }
        if (scenarios::bitwiseEqual(cd.data, icd.data)) {
            return {false, "correction_data and inverse_correction_data are identical"};
        }

        std::string hash = scenarios::sha256Hex(cd.data);
        return {true, "shapes OK, finite values OK, forward!=inverse, correction_data SHA-256=" + hash};
    } catch (const std::exception& e) {
        return {false, std::string("exception: ") + e.what()};
    }
}

} // namespace s02
