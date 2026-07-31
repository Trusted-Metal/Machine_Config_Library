"""
Tests for cross_check._diff() — the parity comparison function.

Two concerns are verified here:

1. int/float representation differences (e.g. 670 vs 670.0) are *ignored*,
   because JSON has no int/float distinction.  JavaScript's JSON.stringify
   always emits ``670`` for a whole-number float while Python emits ``670.0``;
   both are valid representations of the same value.

2. Real semantic differences — wrong value, wrong type (string vs number,
   null vs number), missing/extra fields — are *still caught*.  This is the
   safety net that ensures the ignore_numeric_type_changes fix does not mask
   genuine bugs.

These tests import _diff directly from tools/cross_check.py so they exercise
exactly the same comparison logic that cross_check runs in CI.
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

_TOOLS = Path(__file__).resolve().parents[2] / "tools"
if str(_TOOLS) not in sys.path:
    sys.path.insert(0, str(_TOOLS))

from cross_check import _DEEPDIFF, _diff  # type: ignore[import]  # noqa: E402


# ---------------------------------------------------------------------------
# int/float representation parity — the behaviour the fix introduces
# ---------------------------------------------------------------------------

def test_int_float_same_value_no_diff():
    """670 (int from JS) and 670.0 (float from Python) are the same JSON number."""
    assert _diff({"v": 670}, {"v": 670.0}) == []


def test_int_float_nested_no_diff():
    """Numeric type normalisation applies recursively inside nested objects."""
    a = {"scanner": {"working_distance": 670, "scan_field_x": 100}}
    b = {"scanner": {"working_distance": 670.0, "scan_field_x": 100.0}}
    assert _diff(a, b) == []


def test_int_float_in_list_no_diff():
    """Numeric type normalisation applies inside arrays."""
    assert _diff({"vals": [1, 2, 3]}, {"vals": [1.0, 2.0, 3.0]}) == []


def test_identical_objects_no_diff():
    """Identical dicts always return no differences."""
    data = {"name": "TM-LPBF-02", "wavelength": 1070.0, "trains": 2, "active": True}
    assert _diff(data, data) == []


def test_null_vs_null_no_diff():
    """Two null values compare equal."""
    assert _diff({"v": None}, {"v": None}) == []


def test_fractional_float_no_diff():
    """Non-whole floats that are identical (both sides) produce no diff."""
    assert _diff({"v": 670.5}, {"v": 670.5}) == []


# ---------------------------------------------------------------------------
# Real differences — must still be caught after the fix
# ---------------------------------------------------------------------------

def test_different_numeric_values_caught():
    """A genuine value difference (670 vs 671) must be reported."""
    result = _diff({"v": 670}, {"v": 671})
    assert result != [], "Value mismatch 670 vs 671 was not caught"


def test_fractional_value_difference_caught():
    """670.1 vs 670.2 differ by more than float noise — must be caught."""
    result = _diff({"v": 670.1}, {"v": 670.2})
    assert result != [], "Value mismatch 670.1 vs 670.2 was not caught"


def test_string_vs_number_caught():
    """'670' (string) vs 670 (number) is a real type error — must be caught.

    This is the primary safety net: ignore_numeric_type_changes must never
    mask a field that is accidentally serialised as a string instead of a
    number (or vice versa).
    """
    result = _diff({"v": "670"}, {"v": 670})
    assert result != [], "String '670' vs int 670 was not caught"


def test_string_vs_float_caught():
    """'670.0' (string) vs 670.0 (float) is a real type error — must be caught."""
    result = _diff({"v": "670.0"}, {"v": 670.0})
    assert result != [], "String '670.0' vs float 670.0 was not caught"


def test_number_vs_null_caught():
    """670.0 vs null is a real difference — must be caught."""
    result = _diff({"v": 670.0}, {"v": None})
    assert result != [], "Number vs null was not caught"


def test_null_vs_number_caught():
    """null vs 670.0 (reversed order) is a real difference — must be caught."""
    result = _diff({"v": None}, {"v": 670.0})
    assert result != [], "Null vs number was not caught"


def test_missing_field_caught():
    """A field present in the reference but absent in the comparison must be caught."""
    result = _diff({"a": 1.0, "b": 2.0}, {"a": 1.0})
    assert result != [], "Missing field 'b' was not caught"


def test_extra_field_caught():
    """An unexpected extra field in the comparison output must be caught."""
    result = _diff({"a": 1}, {"a": 1.0, "extra": "unexpected"})
    assert result != [], "Extra field 'extra' was not caught"


def test_wrong_string_value_caught():
    """A string field with the wrong value must be caught."""
    result = _diff({"name": "Machine A"}, {"name": "Machine B"})
    assert result != [], "Wrong string value was not caught"


def test_wrong_nested_string_caught():
    """A wrong string value buried inside a nested object must be caught."""
    a = {"meta": {"machine_name": "AconityMIDI", "schema_version": "v1"}}
    b = {"meta": {"machine_name": "AconityMIDI", "schema_version": "v2"}}
    result = _diff(a, b)
    assert result != [], "Wrong nested string value was not caught"
