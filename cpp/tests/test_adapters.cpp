#include <catch2/catch_test_macros.hpp>
#include <nlohmann/json.hpp>

#include "machine_config/adapters/test_v0_9_to_v1_0.hpp"
#include "machine_config/adapters/registry.hpp"

using namespace machine_config::adapters;
using json = nlohmann::json;

static json make_config(json train) {
    return json{{"optical_trains", json::array({std::move(train)})}};
}

static json first_train(json config) {
    return config["optical_trains"][0];
}

TEST_CASE("field_add inserts null") {
    V0_9_to_V1_0 adapter;
    auto t = first_train(adapter.adapt(make_config(json::object())));
    REQUIRE(t.contains("test_added_field"));
    REQUIRE(t["test_added_field"].is_null());
}

TEST_CASE("field_rename moves value") {
    V0_9_to_V1_0 adapter;
    auto t = first_train(adapter.adapt(make_config({{"test_old_name", "val"}})));
    REQUIRE(t["test_new_name"] == "val");
    REQUIRE_FALSE(t.contains("test_old_name"));
}

TEST_CASE("field_remove erases key") {
    V0_9_to_V1_0 adapter;
    auto t = first_train(adapter.adapt(make_config({{"test_removed_field", "val"}})));
    REQUIRE_FALSE(t.contains("test_removed_field"));
}

TEST_CASE("field_move into test_nested") {
    V0_9_to_V1_0 adapter;
    auto t = first_train(adapter.adapt(make_config({{"test_move_field", "val"}})));
    REQUIRE_FALSE(t.contains("test_move_field"));
    REQUIRE(t["test_nested"]["test_move_field"] == "val");
}

TEST_CASE("field_move_rename into test_nested") {
    V0_9_to_V1_0 adapter;
    auto t = first_train(adapter.adapt(make_config({{"test_move_rename_old", "val"}})));
    REQUIRE_FALSE(t.contains("test_move_rename_old"));
    REQUIRE(t["test_nested"]["test_move_rename_new"] == "val");
}

TEST_CASE("version strings") {
    V0_9_to_V1_0 adapter;
    REQUIRE(adapter.from_version() == "0.9");
    REQUIRE(adapter.to_version() == "1.0");
}

TEST_CASE("registry chain") {
    auto chain09 = AdapterRegistry::instance().get_chain("0.9", "1.0");
    REQUIRE(chain09.size() == 1);
    REQUIRE(AdapterRegistry::instance().get_chain("1.0", "1.0").empty());
}
