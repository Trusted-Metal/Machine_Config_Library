"""Unit tests for machine_config.power_characterization — the shared,
version-agnostic shape-conversion functions Phase 2 (V1_1_IMPLEMENTATION_PLAN.md)
introduced to replace migrate_v1_to_v1_1/migrate_v1_1_to_v1. Pure functions,
no HDF5 I/O — integration with the real writers is covered separately in
test_v1_1_adapter.py.
"""
from __future__ import annotations

from machine_config.models import CalibrationPoint, EquationConstant, PowerCharacterization
from machine_config.power_characterization import (
    backward_flat_fields_coefficients,
    backward_flat_fields_points,
    forward_power_characterization_coefficients,
    forward_power_characterization_points,
)


# ---------------------------------------------------------------------------
# forward_power_characterization_coefficients (Change 3 / ClearBox shape)
# ---------------------------------------------------------------------------

def test_forward_coefficients_both_none_returns_none():
    assert forward_power_characterization_coefficients(None, None) is None


def test_forward_coefficients_linear():
    pc = forward_power_characterization_coefficients("LINEAR", "50.0,100.0")
    assert pc.algorithm_type == "LINEAR"
    assert pc.algorithm_equation == "W = a*V + b"
    assert [c.name for c in pc.derivation_equation_constants] == ["b", "a"]
    assert [c.value for c in pc.derivation_equation_constants] == [50.0, 100.0]
    assert pc.characterization_points == []


def test_forward_coefficients_polynomial():
    pc = forward_power_characterization_coefficients("POLYNOMIAL", "1.0,2.0,3.0")
    assert pc.algorithm_type == "POLYNOMIAL"
    assert pc.algorithm_equation == "W = c0 + c1*V + c2*V^2"
    assert [c.name for c in pc.derivation_equation_constants] == ["c0", "c1", "c2"]


def test_forward_coefficients_blank_input_type_and_units():
    """Confirmed 2026-09-01: blank, not hardcoded — see module docstring."""
    pc = forward_power_characterization_coefficients("LINEAR", "50.0,100.0")
    assert pc.input_type is None
    assert pc.units_derived_quantity is None


def test_forward_coefficients_unrecognized_algorithm_never_raises():
    """Never a hard error — see module docstring's 'Never raises' section."""
    pc = forward_power_characterization_coefficients("EXPONENTIAL", "1.5,2.5,3.5")
    assert pc.algorithm_type == "EXPONENTIAL"
    assert pc.algorithm_equation is None
    assert [c.name for c in pc.derivation_equation_constants] == ["0", "1", "2"]
    assert [c.value for c in pc.derivation_equation_constants] == [1.5, 2.5, 3.5]


# ---------------------------------------------------------------------------
# forward_power_characterization_points (Change 4 / Light_Source shape)
# ---------------------------------------------------------------------------

def test_forward_points_both_none_returns_none():
    assert forward_power_characterization_points(None, None) is None


def test_forward_points_linear():
    pc = forward_power_characterization_points("LINEAR", "1,100,10,1000")
    assert pc.algorithm_type == "LINEAR"
    assert pc.algorithm_equation == "W = a*V + b"
    assert [(p.input_value, p.output_value) for p in pc.characterization_points] == [
        (1.0, 100.0),
        (10.0, 1000.0),
    ]
    assert pc.derivation_equation_constants == []


def test_forward_points_polynomial_leaves_equation_blank():
    """Point count doesn't reliably indicate polynomial degree — never
    generated for POLYNOMIAL, unlike the coefficients shape."""
    pc = forward_power_characterization_points("POLYNOMIAL", "1,100,10,1000")
    assert pc.algorithm_equation is None


def test_forward_points_unrecognized_algorithm_never_raises():
    pc = forward_power_characterization_points("QUADRATIC", "1,100,10,1000")
    assert pc.algorithm_type == "QUADRATIC"
    assert pc.algorithm_equation is None
    assert len(pc.characterization_points) == 2


def test_forward_points_strips_brackets():
    """Real Watts_To_Volts_Params fixtures wrap the CSV in brackets, unlike
    Volts_To_Watts_Params's plain form."""
    pc = forward_power_characterization_points("LINEAR", "[1,100,10,1000]")
    assert [(p.input_value, p.output_value) for p in pc.characterization_points] == [
        (1.0, 100.0),
        (10.0, 1000.0),
    ]


def test_forward_points_blank_input_type_and_units():
    pc = forward_power_characterization_points("LINEAR", "1,100")
    assert pc.input_type is None
    assert pc.units_derived_quantity is None


# ---------------------------------------------------------------------------
# backward_flat_fields_coefficients (Change 3 backward)
# ---------------------------------------------------------------------------

def test_backward_coefficients_none_pc_returns_none_none():
    assert backward_flat_fields_coefficients(None) == (None, None)


def test_backward_coefficients_empty_constants_writes_blank_not_error():
    pc = PowerCharacterization(
        algorithm_type="LINEAR",
        algorithm_equation="W = a*V + b",
        input_type=None,
        units_derived_quantity=None,
        derivation_equation_constants=[],
        characterization_points=[],
    )
    assert backward_flat_fields_coefficients(pc) == ("LINEAR", "")


def test_backward_coefficients_reorders_by_name():
    """Rows aren't positionally guaranteed on disk — must re-sort before
    joining as CSV: b before a for LINEAR."""
    pc = PowerCharacterization(
        algorithm_type="LINEAR",
        algorithm_equation=None,
        input_type=None,
        units_derived_quantity=None,
        derivation_equation_constants=[
            EquationConstant(name="a", value=2.0),
            EquationConstant(name="b", value=1.0),
        ],
        characterization_points=[],
    )
    algorithm, params = backward_flat_fields_coefficients(pc)
    assert algorithm == "LINEAR"
    assert params == "1.0,2.0"


def test_backward_coefficients_reorders_polynomial_by_numeric_suffix():
    pc = PowerCharacterization(
        algorithm_type="POLYNOMIAL",
        algorithm_equation=None,
        input_type=None,
        units_derived_quantity=None,
        derivation_equation_constants=[
            EquationConstant(name="c2", value=3.0),
            EquationConstant(name="c0", value=1.0),
            EquationConstant(name="c1", value=2.0),
        ],
        characterization_points=[],
    )
    _, params = backward_flat_fields_coefficients(pc)
    assert params == "1.0,2.0,3.0"


def test_backward_coefficients_reorders_positional_fallback_numerically():
    """Unrecognized-algorithm-type fallback names ('0','1',...) must sort
    numerically, not lexically (else '10' would sort before '2')."""
    pc = PowerCharacterization(
        algorithm_type="EXPONENTIAL",
        algorithm_equation=None,
        input_type=None,
        units_derived_quantity=None,
        derivation_equation_constants=[
            EquationConstant(name="1", value=2.5),
            EquationConstant(name="0", value=1.5),
            EquationConstant(name="2", value=3.5),
        ],
        characterization_points=[],
    )
    algorithm, params = backward_flat_fields_coefficients(pc)
    assert algorithm == "EXPONENTIAL"
    assert params == "1.5,2.5,3.5"


def test_forward_then_backward_coefficients_round_trips_unrecognized_type():
    original_algorithm, original_params = "EXPONENTIAL", "1.5,2.5,3.5"
    pc = forward_power_characterization_coefficients(original_algorithm, original_params)
    algorithm, params = backward_flat_fields_coefficients(pc)
    assert algorithm == original_algorithm
    assert params == original_params


# ---------------------------------------------------------------------------
# backward_flat_fields_points (Change 4 backward)
# ---------------------------------------------------------------------------

def test_backward_points_none_pc_returns_none_none():
    assert backward_flat_fields_points(None) == (None, None)


def test_backward_points_empty_writes_blank_not_error():
    pc = PowerCharacterization(
        algorithm_type="LINEAR",
        algorithm_equation="W = a*V + b",
        input_type=None,
        units_derived_quantity=None,
        derivation_equation_constants=[],
        characterization_points=[],
    )
    assert backward_flat_fields_points(pc) == ("LINEAR", "")


def test_backward_points_no_resort_needed():
    """Characterization_Points rows aren't named — on-disk order is already
    correct, unlike the coefficients shape."""
    pc = PowerCharacterization(
        algorithm_type="LINEAR",
        algorithm_equation=None,
        input_type=None,
        units_derived_quantity=None,
        derivation_equation_constants=[],
        characterization_points=[
            CalibrationPoint(input_value=1.0, output_value=100.0),
            CalibrationPoint(input_value=10.0, output_value=1000.0),
        ],
    )
    algorithm, params = backward_flat_fields_points(pc)
    assert algorithm == "LINEAR"
    assert params == "1.0,100.0,10.0,1000.0"


def test_forward_then_backward_points_round_trips():
    original_algorithm, original_params = "LINEAR", "1.0,100.0,10.0,1000.0"
    pc = forward_power_characterization_points(original_algorithm, original_params)
    algorithm, params = backward_flat_fields_points(pc)
    assert algorithm == original_algorithm
    assert params == original_params
