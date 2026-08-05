// Catch2 builder tests — §4.20 acceptance criteria.
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include <filesystem>
#include <string>

#include "machine_config/builder.hpp"
#include "machine_config/reader.hpp"

using namespace machine_config;

static std::filesystem::path tmpBuilderPath(const std::string& tag) {
    return std::filesystem::temp_directory_path() /
           ("mc_builder_test_" + tag + ".h5");
}

// ---------------------------------------------------------------------------

TEST_CASE("MockBuilder1LaserRoundtrip") {
    auto out = tmpBuilderPath("1laser");
    MockConfigBuilder b; b.laser_count = 1;
    b.save(out);
    auto cfg = MachineConfigReader{out}.parse();
    REQUIRE(cfg.optical_trains.size() == 1);
    REQUIRE(cfg.meta.machine_name == "MockMachine");
    std::filesystem::remove(out);
}

TEST_CASE("MockBuilder2LaserRoundtrip") {
    auto out = tmpBuilderPath("2laser");
    MockConfigBuilder{}.save(out);
    auto cfg = MachineConfigReader{out}.parse();
    REQUIRE(cfg.optical_trains.size() == 2);
    REQUIRE(cfg.meta.machine_name == "MockMachine");
    std::filesystem::remove(out);
}

TEST_CASE("MockBuilderPlateDimensions") {
    auto out = tmpBuilderPath("plate");
    MockConfigBuilder{}.save(out);
    auto cfg = MachineConfigReader{out}.parse();
    REQUIRE(cfg.machine.build_plate_x.has_value());
    REQUIRE(cfg.machine.build_plate_y.has_value());
    REQUIRE(cfg.machine.build_plate_z.has_value());
    REQUIRE_THAT(*cfg.machine.build_plate_x, Catch::Matchers::WithinRel(250.0));
    REQUIRE_THAT(*cfg.machine.build_plate_y, Catch::Matchers::WithinRel(250.0));
    REQUIRE_THAT(*cfg.machine.build_plate_z, Catch::Matchers::WithinRel(20.0));
    std::filesystem::remove(out);
}

TEST_CASE("MockBuilderCorrectionGridShape") {
    auto out = tmpBuilderPath("grid_shape");
    MockConfigBuilder{}.save(out);
    auto cd = MachineConfigReader{out}.getCorrectionData(0);
    REQUIRE((cd.shape == std::array<size_t, 3>{257, 257, 2}));
    std::filesystem::remove(out);
}

TEST_CASE("MockBuilderCorrectionGridNonzero") {
    auto out = tmpBuilderPath("grid_val");
    MockConfigBuilder{}.save(out);
    auto cd = MachineConfigReader{out}.getCorrectionData(0);
    // Centre cell [128,128,0]: Gaussian peak ≈ 2.0
    double centre = cd.data[(128 * 257 + 128) * 2 + 0];
    REQUIRE(centre > 1.9);
    REQUIRE(centre < 2.1);
    std::filesystem::remove(out);
}

TEST_CASE("MockBuilderNoClearbox") {
    auto out = tmpBuilderPath("no_cb");
    MockConfigBuilder b; b.include_clearbox = false;
    b.save(out);
    auto cfg = MachineConfigReader{out}.parse();
    REQUIRE_FALSE(cfg.optical_trains[0].optional_components.clearbox.has_value());
    std::filesystem::remove(out);
}
