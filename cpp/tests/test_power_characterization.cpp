// Unit tests for machine_config/power_characterization.hpp — the shared,
// version-agnostic shape-conversion functions Phase 2
// (V1_1_IMPLEMENTATION_PLAN.md) introduced to replace
// migrateV1ToV1_1/migrateV1_1ToV1. Pure functions, no HDF5 I/O — integration
// with the real writers is covered separately in test_v1_1_adapter.cpp.
// Mirrors python/tests/test_power_characterization.py and
// rust/tests/power_characterization_test.rs.
#include <catch2/catch_test_macros.hpp>

#include <optional>
#include <string>
#include <vector>

#include "machine_config/power_characterization.hpp"

using namespace machine_config;

static std::vector<std::string> names(const std::vector<EquationConstant>& constants) {
    std::vector<std::string> out;
    for (const auto& c : constants) out.push_back(c.name);
    return out;
}

// ---------------------------------------------------------------------------
// forwardPowerCharacterizationCoefficients (Change 3 / ClearBox shape)
// ---------------------------------------------------------------------------

TEST_CASE("forward coefficients: both nullopt returns nullopt") {
    REQUIRE_FALSE(forwardPowerCharacterizationCoefficients(std::nullopt, std::nullopt).has_value());
}

TEST_CASE("forward coefficients: LINEAR") {
    auto pc = forwardPowerCharacterizationCoefficients("LINEAR", "50.0,100.0");
    REQUIRE(pc.has_value());
    REQUIRE(pc->algorithm_type == std::optional<std::string>{"LINEAR"});
    REQUIRE(pc->algorithm_equation == std::optional<std::string>{"W = a*V + b"});
    REQUIRE(names(pc->derivation_equation_constants) == std::vector<std::string>{"b", "a"});
    REQUIRE(pc->derivation_equation_constants[0].value == 50.0);
    REQUIRE(pc->derivation_equation_constants[1].value == 100.0);
    REQUIRE(pc->characterization_points.empty());
}

TEST_CASE("forward coefficients: POLYNOMIAL") {
    auto pc = forwardPowerCharacterizationCoefficients("POLYNOMIAL", "1.0,2.0,3.0");
    REQUIRE(pc.has_value());
    REQUIRE(pc->algorithm_type == std::optional<std::string>{"POLYNOMIAL"});
    REQUIRE(pc->algorithm_equation == std::optional<std::string>{"W = c0 + c1*V + c2*V^2"});
    REQUIRE(names(pc->derivation_equation_constants) == std::vector<std::string>{"c0", "c1", "c2"});
}

TEST_CASE("forward coefficients: blank input_type and units_derived_quantity") {
    // Confirmed 2026-09-01: blank, not hardcoded — see header docs.
    auto pc = forwardPowerCharacterizationCoefficients("LINEAR", "50.0,100.0");
    REQUIRE_FALSE(pc->input_type.has_value());
    REQUIRE_FALSE(pc->units_derived_quantity.has_value());
}

TEST_CASE("forward coefficients: unrecognized algorithm never throws") {
    // Never a hard error — see header docs' "Never raises" section.
    auto pc = forwardPowerCharacterizationCoefficients("EXPONENTIAL", "1.5,2.5,3.5");
    REQUIRE(pc.has_value());
    REQUIRE(pc->algorithm_type == std::optional<std::string>{"EXPONENTIAL"});
    REQUIRE_FALSE(pc->algorithm_equation.has_value());
    REQUIRE(names(pc->derivation_equation_constants) == std::vector<std::string>{"0", "1", "2"});
    REQUIRE(pc->derivation_equation_constants[0].value == 1.5);
    REQUIRE(pc->derivation_equation_constants[1].value == 2.5);
    REQUIRE(pc->derivation_equation_constants[2].value == 3.5);
}

// ---------------------------------------------------------------------------
// forwardPowerCharacterizationPoints (Change 4 / Light_Source shape)
// ---------------------------------------------------------------------------

TEST_CASE("forward points: both nullopt returns nullopt") {
    REQUIRE_FALSE(forwardPowerCharacterizationPoints(std::nullopt, std::nullopt).has_value());
}

TEST_CASE("forward points: LINEAR") {
    auto pc = forwardPowerCharacterizationPoints("LINEAR", "1,100,10,1000");
    REQUIRE(pc.has_value());
    REQUIRE(pc->algorithm_type == std::optional<std::string>{"LINEAR"});
    REQUIRE(pc->algorithm_equation == std::optional<std::string>{"W = a*V + b"});
    REQUIRE(pc->characterization_points.size() == 2);
    REQUIRE(pc->characterization_points[0].input_value == 1.0);
    REQUIRE(pc->characterization_points[0].output_value == 100.0);
    REQUIRE(pc->characterization_points[1].input_value == 10.0);
    REQUIRE(pc->characterization_points[1].output_value == 1000.0);
    REQUIRE(pc->derivation_equation_constants.empty());
}

TEST_CASE("forward points: POLYNOMIAL leaves equation blank") {
    // Point count doesn't reliably indicate polynomial degree — never
    // generated for POLYNOMIAL, unlike the coefficients shape.
    auto pc = forwardPowerCharacterizationPoints("POLYNOMIAL", "1,100,10,1000");
    REQUIRE_FALSE(pc->algorithm_equation.has_value());
}

TEST_CASE("forward points: unrecognized algorithm never throws") {
    auto pc = forwardPowerCharacterizationPoints("QUADRATIC", "1,100,10,1000");
    REQUIRE(pc.has_value());
    REQUIRE(pc->algorithm_type == std::optional<std::string>{"QUADRATIC"});
    REQUIRE_FALSE(pc->algorithm_equation.has_value());
    REQUIRE(pc->characterization_points.size() == 2);
}

TEST_CASE("forward points: strips brackets") {
    // Real Watts_To_Volts_Params fixtures wrap the CSV in brackets, unlike
    // Volts_To_Watts_Params's plain form.
    auto pc = forwardPowerCharacterizationPoints("LINEAR", "[1,100,10,1000]");
    REQUIRE(pc->characterization_points.size() == 2);
    REQUIRE(pc->characterization_points[0].input_value == 1.0);
    REQUIRE(pc->characterization_points[1].output_value == 1000.0);
}

TEST_CASE("forward points: blank input_type and units_derived_quantity") {
    auto pc = forwardPowerCharacterizationPoints("LINEAR", "1,100");
    REQUIRE_FALSE(pc->input_type.has_value());
    REQUIRE_FALSE(pc->units_derived_quantity.has_value());
}

// ---------------------------------------------------------------------------
// backwardFlatFieldsCoefficients (Change 3 backward)
// ---------------------------------------------------------------------------

TEST_CASE("backward coefficients: nullopt pc returns {nullopt, nullopt}") {
    auto [algorithm, params] = backwardFlatFieldsCoefficients(std::nullopt);
    REQUIRE_FALSE(algorithm.has_value());
    REQUIRE_FALSE(params.has_value());
}

TEST_CASE("backward coefficients: empty constants writes blank, not an error") {
    PowerCharacterization pc;
    pc.algorithm_type = "LINEAR";
    pc.algorithm_equation = "W = a*V + b";
    auto [algorithm, params] = backwardFlatFieldsCoefficients(pc);
    REQUIRE(algorithm == std::optional<std::string>{"LINEAR"});
    REQUIRE(params == std::optional<std::string>{""});
}

TEST_CASE("backward coefficients: reorders by name") {
    // Rows aren't positionally guaranteed on disk — must re-sort before
    // joining as CSV: b before a for LINEAR.
    PowerCharacterization pc;
    pc.algorithm_type = "LINEAR";
    pc.derivation_equation_constants = {EquationConstant{"a", 2.0}, EquationConstant{"b", 1.0}};
    auto [algorithm, params] = backwardFlatFieldsCoefficients(pc);
    REQUIRE(algorithm == std::optional<std::string>{"LINEAR"});
    REQUIRE(params == std::optional<std::string>{"1.0,2.0"});
}

TEST_CASE("backward coefficients: reorders POLYNOMIAL by numeric suffix") {
    PowerCharacterization pc;
    pc.algorithm_type = "POLYNOMIAL";
    pc.derivation_equation_constants = {
        EquationConstant{"c2", 3.0}, EquationConstant{"c0", 1.0}, EquationConstant{"c1", 2.0}};
    auto [algorithm, params] = backwardFlatFieldsCoefficients(pc);
    REQUIRE(params == std::optional<std::string>{"1.0,2.0,3.0"});
}

TEST_CASE("backward coefficients: reorders positional fallback numerically") {
    // Unrecognized-algorithm-type fallback names ('0','1',...) must sort
    // numerically, not lexically (else '10' would sort before '2').
    PowerCharacterization pc;
    pc.algorithm_type = "EXPONENTIAL";
    pc.derivation_equation_constants = {
        EquationConstant{"1", 2.5}, EquationConstant{"0", 1.5}, EquationConstant{"2", 3.5}};
    auto [algorithm, params] = backwardFlatFieldsCoefficients(pc);
    REQUIRE(algorithm == std::optional<std::string>{"EXPONENTIAL"});
    REQUIRE(params == std::optional<std::string>{"1.5,2.5,3.5"});
}

TEST_CASE("forward then backward coefficients round-trips an unrecognized type") {
    std::string originalAlgorithm = "EXPONENTIAL";
    std::string originalParams = "1.5,2.5,3.5";
    auto pc = forwardPowerCharacterizationCoefficients(originalAlgorithm, originalParams);
    auto [algorithm, params] = backwardFlatFieldsCoefficients(pc);
    REQUIRE(algorithm == std::optional<std::string>{originalAlgorithm});
    REQUIRE(params == std::optional<std::string>{originalParams});
}

// ---------------------------------------------------------------------------
// backwardFlatFieldsPoints (Change 4 backward)
// ---------------------------------------------------------------------------

TEST_CASE("backward points: nullopt pc returns {nullopt, nullopt}") {
    auto [algorithm, params] = backwardFlatFieldsPoints(std::nullopt);
    REQUIRE_FALSE(algorithm.has_value());
    REQUIRE_FALSE(params.has_value());
}

TEST_CASE("backward points: empty writes blank, not an error") {
    PowerCharacterization pc;
    pc.algorithm_type = "LINEAR";
    pc.algorithm_equation = "W = a*V + b";
    auto [algorithm, params] = backwardFlatFieldsPoints(pc);
    REQUIRE(algorithm == std::optional<std::string>{"LINEAR"});
    REQUIRE(params == std::optional<std::string>{""});
}

TEST_CASE("backward points: no resort needed") {
    // Characterization_Points rows aren't named — on-disk order is already
    // correct, unlike the coefficients shape.
    PowerCharacterization pc;
    pc.algorithm_type = "LINEAR";
    pc.characterization_points = {CalibrationPoint{1.0, 100.0}, CalibrationPoint{10.0, 1000.0}};
    auto [algorithm, params] = backwardFlatFieldsPoints(pc);
    REQUIRE(algorithm == std::optional<std::string>{"LINEAR"});
    REQUIRE(params == std::optional<std::string>{"1.0,100.0,10.0,1000.0"});
}

TEST_CASE("forward then backward points round-trips") {
    std::string originalAlgorithm = "LINEAR";
    std::string originalParams = "1.0,100.0,10.0,1000.0";
    auto pc = forwardPowerCharacterizationPoints(originalAlgorithm, originalParams);
    auto [algorithm, params] = backwardFlatFieldsPoints(pc);
    REQUIRE(algorithm == std::optional<std::string>{originalAlgorithm});
    REQUIRE(params == std::optional<std::string>{originalParams});
}
