#pragma once
// JSON Schema validation against schema/machine_config_v1.schema.json (§4.21).

#include <nlohmann/json.hpp>
#include <nlohmann/json-schema.hpp>

#include <fstream>
#include <string>
#include <vector>

#ifndef SCHEMA_DIR
#  error "SCHEMA_DIR must be defined by CMakeLists.txt"
#endif

namespace machine_config {

// Loads and caches the schema document on first call (Meyer's singleton).
inline const nlohmann::json& schemaDoc() {
    static const nlohmann::json s = [] {
        std::ifstream f(std::string(SCHEMA_DIR) + "/machine_config_v1.schema.json");
        return nlohmann::json::parse(f);
    }();
    return s;
}

// Returns an empty vector when doc is valid; one string per violation otherwise.
inline std::vector<std::string> validate(const nlohmann::json& doc) {
    // No-op callbacks: the schema uses "format":"date-time" and
    // "contentEncoding":"base64" but we only need structural validation.
    auto noopFormat  = [](const std::string& /*fmt*/, const std::string& /*val*/) {};
    auto noopContent = [](const std::string& /*enc*/, const std::string& /*media*/,
                          const nlohmann::json& /*inst*/) {};
    nlohmann::json_schema::json_validator validator{nullptr, noopFormat, noopContent};
    validator.set_root_schema(schemaDoc());

    std::vector<std::string> errors;
    struct Collector : nlohmann::json_schema::basic_error_handler {
        std::vector<std::string>& out;
        explicit Collector(std::vector<std::string>& o) : out(o) {}
        void error(const nlohmann::json::json_pointer& ptr,
                   const nlohmann::json& /*instance*/,
                   const std::string& msg) override {
            out.push_back(ptr.to_string() + ": " + msg);
        }
    } collector{errors};

    validator.validate(doc, collector);
    return errors;
}

} // namespace machine_config
