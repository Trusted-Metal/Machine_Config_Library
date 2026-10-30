#pragma once
// Test helpers for reading ClearBox, which core exposes as an opaque extension tree in its
// File_Version 1.1 layout for both file versions (see machine_config/clearbox_legacy.hpp).

#include "machine_config/clearbox_legacy.hpp"
#include "machine_config/models.hpp"

#include <catch2/catch_test_macros.hpp>

#include <cstdint>
#include <cstdio>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace cbtest {

using machine_config::MachineConfig;
namespace ext = machine_config::extension;

inline std::string trainKey(std::size_t index) {
    char buf[32];
    std::snprintf(buf, sizeof buf, "Optical_Train_%02d", static_cast<int>(index) + 1);
    return buf;
}

inline const ext::Tree& clearboxRoot(const MachineConfig& cfg) {
    auto it = cfg.extensions.find("ClearBox");
    REQUIRE(it != cfg.extensions.end());
    return it->second;
}

inline ext::Tree& clearboxRoot(MachineConfig& cfg) {
    auto it = cfg.extensions.find("ClearBox");
    REQUIRE(it != cfg.extensions.end());
    return it->second;
}

inline const ext::Tree& clearbox(const MachineConfig& cfg, std::size_t index) {
    const auto& root = clearboxRoot(cfg);
    auto it = root.children.find(trainKey(index));
    REQUIRE(it != root.children.end());
    return it->second;
}

inline ext::Tree& clearbox(MachineConfig& cfg, std::size_t index) {
    auto& root = clearboxRoot(cfg);
    auto it = root.children.find(trainKey(index));
    REQUIRE(it != root.children.end());
    return it->second;
}

inline const ext::Tree& powerCharacterization(const MachineConfig& cfg, std::size_t index) {
    const auto& train = clearbox(cfg, index);
    auto it = train.children.find("Power_Characterization");
    REQUIRE(it != train.children.end());
    return it->second;
}

/// Non-empty string attribute; the contract stores "absent" as "".
inline std::string text(const ext::Tree& tree, const std::string& name) {
    auto it = tree.attrs.find(name);
    if (it == tree.attrs.end()) return {};
    const auto* s = std::get_if<std::string>(&it->second.value);
    return s ? *s : std::string();
}

inline bool hasAttr(const ext::Tree& tree, const std::string& name) { return tree.attrs.count(name) != 0; }

inline bool integer(const ext::Tree& tree, const std::string& name, std::int64_t& out) {
    auto it = tree.attrs.find(name);
    if (it == tree.attrs.end()) return false;
    if (const auto* i = std::get_if<std::int64_t>(&it->second.value)) { out = *i; return true; }
    if (const auto* u = std::get_if<std::uint64_t>(&it->second.value)) { out = static_cast<std::int64_t>(*u); return true; }
    return false;
}

inline bool real(const ext::Tree& tree, const std::string& name, double& out) {
    auto it = tree.attrs.find(name);
    if (it == tree.attrs.end()) return false;
    if (const auto* d = std::get_if<double>(&it->second.value)) { out = *d; return true; }
    return false;
}

/// One train's forward/inverse grid as a flat double vector plus its shape.
inline const std::vector<double>& grid(const MachineConfig& cfg, std::size_t index, const std::string& name,
                                       std::vector<std::uint64_t>* shape = nullptr) {
    const auto& train = clearbox(cfg, index);
    auto it = train.datasets.find(name);
    REQUIRE(it != train.datasets.end());
    if (shape) *shape = it->second.shape;
    const auto* data = std::get_if<std::vector<double>>(&it->second.data);
    REQUIRE(data != nullptr);
    return *data;
}

inline std::vector<std::pair<std::string, double>> constants(const ext::Tree& pc) {
    std::vector<std::pair<std::string, double>> out;
    auto it = pc.tables.find("Derivation_Equation_Constants");
    if (it == pc.tables.end()) return out;
    const auto& names = std::get<std::vector<std::string>>(it->second.columns.at("name"));
    const auto& values = std::get<std::vector<double>>(it->second.columns.at("value"));
    for (std::size_t i = 0; i < names.size(); ++i) out.emplace_back(names[i], values[i]);
    return out;
}

inline std::size_t pointCount(const ext::Tree& pc) {
    auto it = pc.tables.find("Characterization_Points");
    return it == pc.tables.end() ? 0 : static_cast<std::size_t>(it->second.rows);
}

/// A Power_Characterization tree with the given constants and no points.
inline ext::Tree makePowerCharacterization(const std::string& algorithm,
                                           const std::vector<std::pair<std::string, double>>& rows) {
    machine_config::PowerCharacterization pc;
    pc.algorithm_type = algorithm;
    for (const auto& [name, value] : rows) pc.derivation_equation_constants.push_back({name, value});
    return machine_config::clearbox_legacy::detail::powerCharacterizationToTree(pc);
}

/// A sensor group whose only content is a Derivation_Equation_Constants table.
inline ext::Tree sensorTree(const std::vector<std::pair<std::string, double>>& rows) {
    std::vector<std::string> names;
    std::vector<double> values;
    for (const auto& [name, value] : rows) {
        names.push_back(name);
        values.push_back(value);
    }
    ext::Tree tree;
    ext::Table t;
    t.fields = {{"name", "utf8", 64}, {"value", "f64", 0}};
    t.rows = names.size();
    t.columns["name"] = names;
    t.columns["value"] = values;
    tree.tables["Derivation_Equation_Constants"] = std::move(t);
    return tree;
}

/// Puts `sensor` under train 0's Synchronous_Sensors/<label> (mutates cfg).
inline void addSensor(MachineConfig& cfg, const std::string& label, ext::Tree sensor) {
    clearbox(cfg, 0).children["Synchronous_Sensors"].children[label] = std::move(sensor);
}

}  // namespace cbtest
