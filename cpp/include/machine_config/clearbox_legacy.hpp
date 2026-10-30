#pragma once
// File_Version 1.0 keeps ClearBox per optical train under Optional_Components/ClearBox/; core
// exposes it as the opaque "ClearBox" extension tree, always in the File_Version 1.1 layout, so an
// extension library never has to know which file version a tree came from:
//
//   ClearBox/                      Output_Path, Software_Trigger_Delay   (one shared value each)
//     <train_id>/                  per-train attributes, correction datasets, Synchronous_Sensors/
//       Power_Characterization/    structured replacement for Volts_To_Watts_Algorithm/Params
//
// Everything else in a v1.0 ClearBox group is carried verbatim, including the attributes v1.1
// dropped (Selected_Camera, Show_Console, ...), so a v1.0 file round-trips without losing them.
// The two shared values are taken from the first train that has a ClearBox.
//
// Mirrors rust/src/clearbox_legacy.rs.

#include "machine_config/extension_contract.hpp"
#include "machine_config/models.hpp"
#include "machine_config/power_characterization.hpp"

#include <algorithm>
#include <map>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace machine_config::clearbox_legacy {

inline constexpr const char* EXTENSION_NAME = "ClearBox";

namespace detail {

inline constexpr const char* POWER_CHARACTERIZATION = "Power_Characterization";
inline constexpr const char* SHARED_ATTRS[] = {"Output_Path", "Software_Trigger_Delay"};
inline constexpr const char* FLAT_ALGORITHM = "Volts_To_Watts_Algorithm";
inline constexpr const char* FLAT_PARAMS = "Volts_To_Watts_Params";
inline constexpr const char* V1_1_ONLY_ATTRS[] = {"Firmware_Version"};
inline constexpr std::uint64_t EQUATION_CONSTANT_NAME_BYTES = 64;

inline std::optional<std::string> text(const extension::Tree& tree, const std::string& name) {
    auto it = tree.attrs.find(name);
    if (it == tree.attrs.end()) return std::nullopt;
    const auto* s = std::get_if<std::string>(&it->second.value);
    if (!s) return std::nullopt;
    auto first = s->find_first_not_of(" \t\r\n");
    if (first == std::string::npos) return std::nullopt;
    auto last = s->find_last_not_of(" \t\r\n");
    return s->substr(first, last - first + 1);
}

inline extension::Attr strAttr(const std::optional<std::string>& v) {
    return extension::Attr{"utf8", v.value_or(std::string())};
}

inline extension::Tree powerCharacterizationToTree(const std::optional<PowerCharacterization>& pc) {
    extension::Tree tree;
    tree.attrs["Algorithm_Type"] = strAttr(pc ? pc->algorithm_type : std::nullopt);
    tree.attrs["Algorithm_Equation"] = strAttr(pc ? pc->algorithm_equation : std::nullopt);
    tree.attrs["Input_Type"] = strAttr(pc ? pc->input_type : std::nullopt);
    tree.attrs["Units_Derived_Quantity"] = strAttr(pc ? pc->units_derived_quantity : std::nullopt);

    std::vector<std::string> names;
    std::vector<double> values;
    if (pc)
        for (const auto& c : pc->derivation_equation_constants) {
            names.push_back(c.name);
            values.push_back(c.value);
        }
    extension::Table constants;
    constants.fields = {{"name", "utf8", EQUATION_CONSTANT_NAME_BYTES}, {"value", "f64", 0}};
    constants.rows = names.size();
    constants.columns["name"] = names;
    constants.columns["value"] = values;
    tree.tables["Derivation_Equation_Constants"] = std::move(constants);

    std::vector<double> in, out;
    if (pc)
        for (const auto& p : pc->characterization_points) {
            in.push_back(p.input_value);
            out.push_back(p.output_value);
        }
    extension::Table points;
    points.fields = {{"input_value", "f64", 0}, {"output_value", "f64", 0}};
    points.rows = in.size();
    points.columns["input_value"] = in;
    points.columns["output_value"] = out;
    tree.tables["Characterization_Points"] = std::move(points);
    return tree;
}

}  // namespace detail

/// Reads a ClearBox Power_Characterization tree back into the model.
inline PowerCharacterization powerCharacterizationFromTree(const extension::Tree& tree) {
    PowerCharacterization pc;
    if (auto it = tree.tables.find("Derivation_Equation_Constants"); it != tree.tables.end()) {
        const auto& cols = it->second.columns;
        auto n = cols.find("name");
        auto v = cols.find("value");
        if (n != cols.end() && v != cols.end()) {
            const auto* names = std::get_if<std::vector<std::string>>(&n->second);
            const auto* values = std::get_if<std::vector<double>>(&v->second);
            if (names && values)
                for (std::size_t i = 0; i < names->size() && i < values->size(); ++i)
                    pc.derivation_equation_constants.push_back({(*names)[i], (*values)[i]});
        }
    }
    if (auto it = tree.tables.find("Characterization_Points"); it != tree.tables.end()) {
        const auto& cols = it->second.columns;
        auto a = cols.find("input_value");
        auto b = cols.find("output_value");
        if (a != cols.end() && b != cols.end()) {
            const auto* in = std::get_if<std::vector<double>>(&a->second);
            const auto* out = std::get_if<std::vector<double>>(&b->second);
            if (in && out)
                for (std::size_t i = 0; i < in->size() && i < out->size(); ++i)
                    pc.characterization_points.push_back({(*in)[i], (*out)[i]});
        }
    }
    pc.algorithm_type = detail::text(tree, "Algorithm_Type");
    pc.algorithm_equation = detail::text(tree, "Algorithm_Equation");
    pc.input_type = detail::text(tree, "Input_Type");
    pc.units_derived_quantity = detail::text(tree, "Units_Derived_Quantity");
    return pc;
}

/// One optical train's verbatim File_Version 1.0 ClearBox group.
struct TrainGroup {
    std::string train_id;
    extension::Tree tree;
};

/// Builds the ClearBox tree from each train's verbatim v1.0 group, in train order; nullopt when no
/// train has a ClearBox.
inline std::optional<extension::Tree> fromV10Trains(std::vector<TrainGroup> trains) {
    if (trains.empty()) return std::nullopt;
    extension::Tree root;
    const auto& first = trains.front().tree;
    for (const char* name : detail::SHARED_ATTRS)
        if (auto it = first.attrs.find(name); it != first.attrs.end()) root.attrs[name] = it->second;
    for (auto& t : trains) {
        for (const char* name : detail::SHARED_ATTRS) t.tree.attrs.erase(name);
        auto pc = forwardPowerCharacterizationCoefficients(detail::text(t.tree, detail::FLAT_ALGORITHM),
                                                           detail::text(t.tree, detail::FLAT_PARAMS));
        t.tree.attrs.erase(detail::FLAT_ALGORITHM);
        t.tree.attrs.erase(detail::FLAT_PARAMS);
        if (t.tree.children.count(detail::POWER_CHARACTERIZATION))
            throw std::runtime_error(t.train_id + ": a File_Version 1.0 ClearBox group cannot contain " +
                                     detail::POWER_CHARACTERIZATION);
        t.tree.children[detail::POWER_CHARACTERIZATION] = detail::powerCharacterizationToTree(pc);
        root.children[t.train_id] = std::move(t.tree);
    }
    return root;
}

/// Splits the ClearBox tree back into one verbatim v1.0 group per train id.
///
/// Throws rather than dropping anything v1.0 has no place for: attributes or datasets directly on
/// the extension root other than the two shared values.
inline std::map<std::string, extension::Tree> toV10Trains(const extension::Tree& tree) {
    auto lossy = [](const std::string& what) {
        return std::runtime_error("ClearBox: " + what + " has no File_Version 1.0 location");
    };
    for (const auto& [name, attr] : tree.attrs) {
        (void)attr;
        bool shared = false;
        for (const char* s : detail::SHARED_ATTRS)
            if (name == s) shared = true;
        if (!shared) throw lossy("attribute \"" + name + "\"");
    }
    for (const auto& [name, ds] : tree.datasets) {
        (void)ds;
        throw lossy("dataset \"" + name + "\"");
    }
    for (const auto& [name, table] : tree.tables) {
        (void)table;
        throw lossy("dataset \"" + name + "\"");
    }

    std::map<std::string, extension::Tree> out;
    for (const auto& [trainId, child] : tree.children) {
        extension::Tree group = child;
        for (const char* name : detail::SHARED_ATTRS)
            if (!group.attrs.count(name))
                if (auto it = tree.attrs.find(name); it != tree.attrs.end()) group.attrs[name] = it->second;
        for (const char* name : detail::V1_1_ONLY_ATTRS) group.attrs.erase(name);
        std::optional<PowerCharacterization> pc;
        if (auto it = group.children.find(detail::POWER_CHARACTERIZATION); it != group.children.end()) {
            pc = powerCharacterizationFromTree(it->second);
            group.children.erase(it);
        }
        auto [algorithm, params] = backwardFlatFieldsCoefficients(pc);
        group.attrs[detail::FLAT_ALGORITHM] = detail::strAttr(algorithm);
        group.attrs[detail::FLAT_PARAMS] = detail::strAttr(params);
        out[trainId] = std::move(group);
    }
    return out;
}

}  // namespace machine_config::clearbox_legacy
