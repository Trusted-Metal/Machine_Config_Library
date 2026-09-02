#pragma once
// Shared, version-agnostic conversion between the two on-disk shapes this
// concept has ever used: the flat shape (an opaque Algorithm_Type string
// plus a CSV Params string — File_Version 1.0's ClearBox.Volts_To_Watts_*
// and LightSource.Watts_To_Volts_*) and the structured shape
// (PowerCharacterization — File_Version 1.1's Power_Characterization group).
//
// Deliberately independent of both capabilities::v1_0 and capabilities::v1_1
// — this header only transforms StableModel types (PowerCharacterization,
// EquationConstant, CalibrationPoint), has no HDF5 I/O, and has no knowledge
// of "1.0"/"1.1" as version strings. It lives at the same neutral level as
// models.hpp so that either version's writer can include it without either
// version becoming a runtime dependency of the other (see
// V1_1_IMPLEMENTATION_PLAN.md's Phase 2 "Structural requirement" note) — and
// so a future version reusing either shape needs zero new code here, only a
// new pair of functions if a third shape is ever introduced.
//
// power_characterization is the only StableModel representation of this
// concept, for files of either version — there is no separate
// Volts_To_Watts_*/Watts_To_Volts_* field to fall back to. Hdf5ReaderV1_0
// calls the forward function unconditionally as part of ordinary parsing, to
// populate power_characterization from the on-disk flat attrs;
// Hdf5WriterV1_0 calls the backward function unconditionally to derive the
// flat attrs it writes from power_characterization. Hdf5AdapterV1_1 reads
// and writes power_characterization natively, with no conversion at all.
//
// (An earlier design — V1_1_IMPLEMENTATION_PLAN.md's Phase 2 — kept both
// representations on the StableModel side by side, with each writer
// deriving from the other shape only as a fallback when its own native
// field was absent, and kept this conversion out of the readers entirely
// because it only worked for models that came from an actual parse() call.
// That asymmetry — silently empty for any consumer that only ever read the
// legacy field — is exactly what motivated collapsing to a single field;
// see POWER_CHARACTERIZATION_UNIFICATION_PLAN.md's "Context" section.)
//
// Two shapes exist today:
// - Coefficients (Change 3 / ClearBox): Params is a positional,
//   comma-separated coefficient list. Named to match the generated
//   equation's own variable names: b/a for LINEAR, c0/c1/... for
//   POLYNOMIAL, or a plain numeric fallback ("0", "1", ...) for any other
//   value — never an error, see "Never raises" below.
// - Points (Change 4 / Light_Source): Params is an alternating
//   input_value, output_value list — direct, mechanical parse, no
//   coefficient-ordering convention to assume.
//
// Never raises on an unrecognized Algorithm_Type. Earlier design
// (File_Version 1.1's now-removed migrateV1ToV1_1) treated this as a hard
// error (std::runtime_error), which made sense when this conversion only
// ran inside an explicit, opt-in migration call. Once it runs implicitly as
// part of an ordinary write (this header's actual job now), that would mean
// an unrelated value elsewhere on the model could fail a write the caller
// never expected to fail. This is an *incompleteness* problem (less
// information than the richer shape can hold), not a *conflict* (two
// present, disagreeing values) — Change 1's Consolidate check is the latter
// and correctly keeps its hard error in Hdf5WriterV1_1, untouched by this
// header. algorithm_type is always copied verbatim (open string, per
// docs/migrations/v1_0_to_v1_1.md's resolved enum-vs-string decision),
// coefficients/points are always parsed mechanically, and only
// algorithm_equation is left blank when it can't be generated — round trip
// stays lossless even for unrecognized types, since writing back re-sorts
// by the numeric fallback name and reproduces the original CSV order
// exactly. (A genuinely malformed CSV float — not an unrecognized algorithm
// type, actual corrupt data — still throws via std::stod, same as before;
// that failure mode is orthogonal to this rule and unchanged.)
//
// input_type/units_derived_quantity are always left blank (nullopt) on
// forward derivation, not hardcoded — confirmed 2026-09-01: a hardcoded
// guess is a special case that would need re-litigating for every future
// version; blank is honest about "not derivable from what's on disk."

#include "machine_config/models.hpp"

#include <algorithm>
#include <cctype>
#include <cstdlib>
#include <iomanip>
#include <optional>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

namespace machine_config {

// Parses a comma-separated float list. Tolerates the bracketed "[1,2,3]"
// form real Watts_To_Volts_Params fixtures use, as well as the plain
// "1,2,3" form real Volts_To_Watts_Params fixtures use — brackets are
// stripped unconditionally (a no-op for the unbracketed case).
inline std::vector<double> pcParseCsvFloats(const std::optional<std::string>& s) {
    std::vector<double> out;
    if (!s) return out;
    std::string str = *s;
    auto first = str.find_first_not_of(" \t\r\n");
    if (first == std::string::npos) return out;
    auto last = str.find_last_not_of(" \t\r\n");
    str = str.substr(first, last - first + 1);
    while (!str.empty() && (str.front() == '[' || str.front() == ']')) str.erase(str.begin());
    while (!str.empty() && (str.back() == '[' || str.back() == ']')) str.pop_back();
    if (str.empty()) return out;
    std::stringstream ss(str);
    std::string token;
    while (std::getline(ss, token, ',')) {
        auto tf = token.find_first_not_of(" \t\r\n");
        if (tf == std::string::npos) continue;
        auto tl = token.find_last_not_of(" \t\r\n");
        token = token.substr(tf, tl - tf + 1);
        if (!token.empty()) out.push_back(std::stod(token));
    }
    return out;
}

inline std::string pcGeneratePowerEquation(const std::string& algorithmType, std::size_t nConstants) {
    if (algorithmType == "LINEAR") return "W = a*V + b";
    std::vector<std::string> terms = {"c0"};
    for (std::size_t i = 1; i < nConstants; ++i) {
        if (i == 1) terms.push_back("c1*V");
        else        terms.push_back("c" + std::to_string(i) + "*V^" + std::to_string(i));
    }
    std::string result = "W = ";
    for (std::size_t i = 0; i < terms.size(); ++i) {
        if (i) result += " + ";
        result += terms[i];
    }
    return result;
}

// Named rows aren't positionally guaranteed on disk — re-sort by name before
// joining as CSV: b before a for LINEAR, c0 < c1 < c2 < ... by numeric
// suffix for POLYNOMIAL, "0" < "1" < "2" < ... numerically for the
// unrecognized-algorithm-type fallback naming (see header docs' "Never
// raises"), anything else falls back to alphabetical by name
// (natively-authored v1.1 data can use arbitrary names, since Algorithm_Type
// is an open string).
inline std::vector<EquationConstant> pcSortedConstantsByName(std::vector<EquationConstant> constants) {
    struct Key { int category; double numeric; std::string name; };
    auto isAllDigits = [](const std::string& s) {
        return !s.empty() && std::all_of(s.begin(), s.end(),
            [](unsigned char ch) { return std::isdigit(ch) != 0; });
    };
    auto key = [&](const EquationConstant& c) -> Key {
        if (c.name == "b") return {0, 0.0, ""};
        if (c.name == "a") return {0, 1.0, ""};
        if (c.name.size() > 1 && c.name[0] == 'c' && isAllDigits(c.name.substr(1))) {
            return {1, std::stod(c.name.substr(1)), ""};
        }
        if (isAllDigits(c.name)) {
            return {2, std::stod(c.name), ""};
        }
        return {3, 0.0, c.name};
    };
    std::stable_sort(constants.begin(), constants.end(), [&](const EquationConstant& x, const EquationConstant& y) {
        Key kx = key(x), ky = key(y);
        if (kx.category != ky.category) return kx.category < ky.category;
        if (kx.category == 3) return kx.name < ky.name;
        return kx.numeric < ky.numeric;
    });
    return constants;
}

// Formats a double the way backward derivation needs to: always with a
// decimal point (e.g. "1.0", not "1"), 17 significant digits — the minimum
// that guarantees an exact round-trip for any IEEE-754 double, needed since
// this string may be re-parsed later (e.g. re-reading a downgraded file).
inline std::string pcFormatDoubleForCsv(double v) {
    std::ostringstream oss;
    oss << std::setprecision(17) << v;
    std::string s = oss.str();
    if (s.find('.') == std::string::npos &&
        s.find('e') == std::string::npos && s.find('E') == std::string::npos) {
        s += ".0";
    }
    return s;
}

// Coefficients-shape forward derivation (Change 3 / ClearBox):
// Volts_To_Watts_Algorithm/Volts_To_Watts_Params -> PowerCharacterization.
//
// nullopt if both inputs are nullopt (nothing to derive). Never throws for
// an unrecognized algorithm_type — see header docs. Can still throw
// (std::invalid_argument, via std::stod) for a genuinely malformed CSV
// float, an orthogonal, pre-existing failure mode.
inline std::optional<PowerCharacterization> forwardPowerCharacterizationCoefficients(
    const std::optional<std::string>& algorithmType, const std::optional<std::string>& params)
{
    if (!algorithmType && !params) return std::nullopt;
    std::string algo = algorithmType.value_or("");
    auto values = pcParseCsvFloats(params);
    std::vector<std::string> names;
    names.reserve(values.size());
    if (algo == "LINEAR") {
        names = {"b", "a"};
    } else if (algo == "POLYNOMIAL") {
        for (std::size_t i = 0; i < values.size(); ++i) names.push_back("c" + std::to_string(i));
    } else {
        for (std::size_t i = 0; i < values.size(); ++i) names.push_back(std::to_string(i));
    }
    std::size_t n = std::min(names.size(), values.size());
    std::vector<EquationConstant> constants;
    constants.reserve(n);
    for (std::size_t i = 0; i < n; ++i) constants.push_back({names[i], values[i]});

    PowerCharacterization pc;
    pc.algorithm_type = algo;
    if (algo == "LINEAR" || algo == "POLYNOMIAL") {
        pc.algorithm_equation = pcGeneratePowerEquation(algo, values.size());
    }
    pc.input_type = std::nullopt;
    pc.units_derived_quantity = std::nullopt;
    pc.derivation_equation_constants = std::move(constants);
    pc.characterization_points = {};
    return pc;
}

// Points-shape forward derivation (Change 4 / Light_Source):
// Watts_To_Volts_Algorithm/Watts_To_Volts_Params -> PowerCharacterization.
//
// Inverse data-availability from the coefficients shape — direct,
// mechanical parse, no coefficient-ordering convention to assume. nullopt
// if both inputs are nullopt. Never throws for an unrecognized
// algorithm_type — see header docs.
inline std::optional<PowerCharacterization> forwardPowerCharacterizationPoints(
    const std::optional<std::string>& algorithmType, const std::optional<std::string>& params)
{
    if (!algorithmType && !params) return std::nullopt;
    std::string algo = algorithmType.value_or("");
    auto values = pcParseCsvFloats(params);
    std::vector<CalibrationPoint> points;
    for (std::size_t i = 0; i + 1 < values.size(); i += 2)
        points.push_back({values[i], values[i + 1]});
    // POLYNOMIAL (and the unrecognized-type fallback): left blank, not
    // generated — point count doesn't reliably indicate polynomial degree.
    std::optional<std::string> equation;
    if (algo == "LINEAR") equation = "W = a*V + b";

    PowerCharacterization pc;
    pc.algorithm_type = algo;
    pc.algorithm_equation = equation;
    pc.input_type = std::nullopt;
    pc.units_derived_quantity = std::nullopt;
    pc.derivation_equation_constants = {};
    pc.characterization_points = std::move(points);
    return pc;
}

// Coefficients-shape backward derivation (Change 3 / ClearBox):
// PowerCharacterization -> (Algorithm_Type, Volts_To_Watts_Params).
//
// {nullopt, nullopt} if pc is nullopt. Blank params ("", not an error) if
// derivation_equation_constants is empty — absence, not ambiguity, see
// docs/migrations/v1_0_to_v1_1.md Change 3's "Backward migration" section.
inline std::pair<std::optional<std::string>, std::optional<std::string>>
backwardFlatFieldsCoefficients(const std::optional<PowerCharacterization>& pc) {
    if (!pc) return {std::nullopt, std::nullopt};
    if (pc->derivation_equation_constants.empty()) return {pc->algorithm_type, std::string()};
    auto ordered = pcSortedConstantsByName(pc->derivation_equation_constants);
    std::string result;
    for (std::size_t i = 0; i < ordered.size(); ++i) {
        if (i) result += ",";
        result += pcFormatDoubleForCsv(ordered[i].value);
    }
    return {pc->algorithm_type, result};
}

// Points-shape backward derivation (Change 4 / Light_Source):
// PowerCharacterization -> (Algorithm_Type, Watts_To_Volts_Params).
//
// {nullopt, nullopt} if pc is nullopt. No re-sort needed —
// Characterization_Points rows aren't named, and on-disk order is already
// correct. Blank params if empty, same absence-not-ambiguity reasoning as
// the coefficients shape.
inline std::pair<std::optional<std::string>, std::optional<std::string>>
backwardFlatFieldsPoints(const std::optional<PowerCharacterization>& pc) {
    if (!pc) return {std::nullopt, std::nullopt};
    if (pc->characterization_points.empty()) return {pc->algorithm_type, std::string()};
    std::string result;
    for (std::size_t i = 0; i < pc->characterization_points.size(); ++i) {
        if (i) result += ",";
        result += pcFormatDoubleForCsv(pc->characterization_points[i].input_value);
        result += ",";
        result += pcFormatDoubleForCsv(pc->characterization_points[i].output_value);
    }
    return {pc->algorithm_type, result};
}

}  // namespace machine_config
