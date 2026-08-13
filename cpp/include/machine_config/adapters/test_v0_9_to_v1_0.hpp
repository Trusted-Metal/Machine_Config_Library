// AUTO-GENERATED from schema/adapters/test_v0_9_to_v1_0.yaml
// DO NOT EDIT MANUALLY — regenerate with: python tools/generate_adapters.py
#pragma once
#include <nlohmann/json.hpp>
#include "machine_config/adapters/adapter.hpp"

namespace machine_config::adapters {

class V0_9_to_V1_0 final : public IAdapter {
public:
    std::string_view from_version() const noexcept override { return "0.9"; }
    std::string_view to_version()   const noexcept override { return "1.0"; }

    nlohmann::json adapt(nlohmann::json config) const override {
        if (config.contains("optical_trains") && config.at("optical_trains").is_array()) {
            for (auto& train : config["optical_trains"]) {
                // field_add: test_added_field (path: optical_trains[*])
                if (!train.contains("test_added_field")) {
                    train["test_added_field"] = nullptr;
                }
                // field_rename: test_old_name -> test_new_name (path: optical_trains[*])
                if (train.contains("test_old_name")) {
                    train["test_new_name"] = train["test_old_name"];
                    train.erase("test_old_name");
                }
                // field_remove: test_removed_field dropped (path: optical_trains[*])
                train.erase("test_removed_field");
                // field_move: test_move_field (optical_trains[*] -> optical_trains[*].test_nested)
                if (train.contains("test_move_field")) {
                    if (!train.contains("test_nested") || !train.at("test_nested").is_object()) {
                        train["test_nested"] = nlohmann::json::object();
                    }
                    train["test_nested"]["test_move_field"] = train["test_move_field"];
                    train.erase("test_move_field");
                }
                // field_move_rename: test_move_rename_old -> test_nested.test_move_rename_new (optical_trains[*] -> optical_trains[*].test_nested)
                if (train.contains("test_move_rename_old")) {
                    if (!train.contains("test_nested") || !train.at("test_nested").is_object()) {
                        train["test_nested"] = nlohmann::json::object();
                    }
                    train["test_nested"]["test_move_rename_new"] = train["test_move_rename_old"];
                    train.erase("test_move_rename_old");
                }
            }
        }
        return config;
    }
};

} // namespace machine_config::adapters
