"""Shared, version-agnostic conversion between the two on-disk shapes this
concept has ever used: the flat shape (an opaque ``Algorithm_Type`` string
plus a CSV ``Params`` string — File_Version 1.0's ``ClearBox.Volts_To_Watts_*``
and ``LightSource.Watts_To_Volts_*``) and the structured shape
(:class:`~machine_config.models.PowerCharacterization` — File_Version 1.1's
``Power_Characterization`` group).

Deliberately independent of both ``capabilities/v1_0/`` and
``capabilities/v1_1/`` — this module only transforms StableModel types
(:class:`PowerCharacterization`, :class:`EquationConstant`,
:class:`CalibrationPoint`), has no HDF5 I/O, and has no knowledge of
"1.0"/"1.1" as version strings. It lives at the same neutral level as
``models.py`` so that *either* version's reader can import it without either
version becoming a runtime dependency of the other (see
V1_1_IMPLEMENTATION_PLAN.md's Phase 2 "Structural requirement" note) — and so
that a future version reusing either shape needs zero new code here, only a
new pair of functions if a third shape is ever introduced.

Both writers — never the readers — call the matching "forward"/"backward"
function for the *other* shape, and only as a fallback: each writer first
looks at its own native field, and only derives from the other shape when its
own is absent. Concretely, ``Hdf5WriterV1_0`` writes ``volts_to_watts_*``/
``watts_to_volts_*`` directly if present, else derives them from
``power_characterization``; ``Hdf5WriterV1_1`` writes
``power_characterization`` directly if present, else derives it from the flat
fields. Both readers stay exactly as simple as every other version's reader —
each reads only its own on-disk shape, nothing more.

This placement (writer-side fallback, not reader-side eager derivation) is a
deliberate choice, not the only possible one — an earlier draft of Phase 2 put
this in the readers instead. Rejected because it only works for models that
came from an actual ``parse()`` call — anything built without going through a
real read (a hand-built model, a test mock, ``create()``) would silently skip
the derivation. Putting it in the writer instead means the fallback applies
universally, regardless of how the :class:`~machine_config.models.MachineConfig`
was produced: real reads, hand-built mocks, builders, all of it — because
every path to disk goes through a writer, but not every StableModel object
was ever read from a file. This is the mechanism behind
V1_1_IMPLEMENTATION_PLAN.md Phase 2's acceptance criterion: asking for a file
of one version, when the other version's data was loaded (or otherwise
present on the model), produces that version's file with as much data as
there is and nothing more.

Two shapes exist today:

- **Coefficients** (Change 3 / ClearBox): ``Params`` is a positional,
  comma-separated coefficient list. Named to match the generated equation's
  own variable names: ``b``/``a`` for LINEAR, ``c0``/``c1``/... for
  POLYNOMIAL, or a plain numeric fallback (``"0"``, ``"1"``, ...) for any
  other value — never an error, see "Never raises" below.
- **Points** (Change 4 / Light_Source): ``Params`` is an alternating
  ``input_value, output_value`` list — direct, mechanical parse, no
  coefficient-ordering convention to assume.

**Never raises.** Earlier design (File_Version 1.1's now-removed
``migrate_v1_to_v1_1``) treated an unrecognized ``Algorithm_Type`` as a hard
error, which made sense when this conversion only ran inside an explicit,
opt-in migration call. Once it runs implicitly as part of an ordinary write
(this module's actual job now), that would mean an unrelated value elsewhere
on the model could fail a write the caller never expected to fail. This is an
*incompleteness* problem (less information than the richer shape can hold),
not a *conflict* (two present, disagreeing values) — Change 1's Consolidate
check is the latter and correctly keeps its hard error in ``Hdf5WriterV1_1``,
untouched by this module. ``algorithm_type`` is always copied verbatim (open
string, per docs/migrations/v1_0_to_v1_1.md's resolved enum-vs-string decision),
coefficients/points are always parsed mechanically, and only
``algorithm_equation`` is left blank when it can't be
generated — round trip stays lossless even for unrecognized types, since
writing back re-sorts by the numeric fallback name and reproduces the
original CSV order exactly.

``input_type``/``units_derived_quantity`` are always left blank (``None``)
on forward derivation, not hardcoded — confirmed 2026-09-01: a hardcoded
guess is a special case that would need re-litigating for every future
version; blank is honest about "not derivable from what's on disk."
"""
from __future__ import annotations

from typing import Optional

from machine_config.models import CalibrationPoint, EquationConstant, PowerCharacterization

_RECOGNIZED_ALGORITHM_TYPES = ("LINEAR", "POLYNOMIAL")


def _parse_csv_floats(s: Optional[str]) -> list[float]:
    """Parses a comma-separated float list. Tolerates the bracketed
    ``"[1,2,3]"`` form real Watts_To_Volts_Params fixtures use, as well as
    the plain ``"1,2,3"`` form real Volts_To_Watts_Params fixtures use.
    """
    if not s:
        return []
    stripped = s.strip().strip("[]")
    if not stripped:
        return []
    return [float(x) for x in stripped.split(",") if x.strip() != ""]


def _generate_power_equation(algorithm_type: str, n_constants: int) -> str:
    if algorithm_type == "LINEAR":
        return "W = a*V + b"
    terms = ["c0"]
    for i in range(1, n_constants):
        terms.append(f"c{i}*V" if i == 1 else f"c{i}*V^{i}")
    return "W = " + " + ".join(terms)


def _sorted_constants_by_name(constants: list[EquationConstant]) -> list[EquationConstant]:
    """Named rows aren't positionally guaranteed on disk — re-sort by name
    before joining as CSV: ``b`` before ``a`` for LINEAR, ``c0 < c1 < c2 < ...``
    by numeric suffix for POLYNOMIAL, ``"0" < "1" < "2" < ...`` numerically
    for the unrecognized-algorithm-type fallback naming (see module
    docstring's "Never raises"), anything else falls back to alphabetical by
    name (natively-authored v1.1 data can use arbitrary names, since
    Algorithm_Type is an open string). See docs/migrations/v1_0_to_v1_1.md
    Change 3's "Backward migration" section.
    """
    def key(c: EquationConstant):
        if c.name == "b":
            return (0, 0, "")
        if c.name == "a":
            return (0, 1, "")
        if c.name.startswith("c") and c.name[1:].isdigit():
            return (1, int(c.name[1:]), "")
        if c.name.isdigit():
            return (2, int(c.name), "")
        return (3, 0, c.name)

    return sorted(constants, key=key)


def forward_power_characterization_coefficients(
    algorithm_type: Optional[str], params: Optional[str]
) -> Optional[PowerCharacterization]:
    """Coefficients-shape forward derivation (Change 3 / ClearBox):
    ``Volts_To_Watts_Algorithm``/``Volts_To_Watts_Params`` -> PowerCharacterization.

    ``None`` if both inputs are ``None`` (nothing to derive). Never raises —
    see module docstring.
    """
    if algorithm_type is None and params is None:
        return None
    values = _parse_csv_floats(params)
    if algorithm_type == "LINEAR":
        names = ["b", "a"]
    elif algorithm_type == "POLYNOMIAL":
        names = [f"c{i}" for i in range(len(values))]
    else:
        names = [str(i) for i in range(len(values))]
    constants = [EquationConstant(name=n, value=v) for n, v in zip(names, values)]
    equation = (
        _generate_power_equation(algorithm_type, len(values))
        if algorithm_type in _RECOGNIZED_ALGORITHM_TYPES
        else None
    )
    return PowerCharacterization(
        algorithm_type=algorithm_type,
        algorithm_equation=equation,
        input_type=None,
        units_derived_quantity=None,
        derivation_equation_constants=constants,
        characterization_points=[],
    )


def forward_power_characterization_points(
    algorithm_type: Optional[str], params: Optional[str]
) -> Optional[PowerCharacterization]:
    """Points-shape forward derivation (Change 4 / Light_Source):
    ``Watts_To_Volts_Algorithm``/``Watts_To_Volts_Params`` -> PowerCharacterization.

    Inverse data-availability from the coefficients shape — direct,
    mechanical parse, no coefficient-ordering convention to assume. ``None``
    if both inputs are ``None``. Never raises — see module docstring.
    """
    if algorithm_type is None and params is None:
        return None
    values = _parse_csv_floats(params)
    points = [
        CalibrationPoint(input_value=values[i], output_value=values[i + 1])
        for i in range(0, len(values) - 1, 2)
    ]
    # POLYNOMIAL (and the unrecognized-type fallback): left blank, not
    # generated — point count doesn't reliably indicate polynomial degree.
    equation = "W = a*V + b" if algorithm_type == "LINEAR" else None
    return PowerCharacterization(
        algorithm_type=algorithm_type,
        algorithm_equation=equation,
        input_type=None,
        units_derived_quantity=None,
        derivation_equation_constants=[],
        characterization_points=points,
    )


def backward_flat_fields_coefficients(
    pc: Optional[PowerCharacterization],
) -> tuple[Optional[str], Optional[str]]:
    """Coefficients-shape backward derivation (Change 3 / ClearBox):
    PowerCharacterization -> (Algorithm_Type, Volts_To_Watts_Params).

    ``(None, None)`` if ``pc`` is ``None``. Blank params (``""``, not an
    error) if ``derivation_equation_constants`` is empty — absence, not
    ambiguity, see docs/migrations/v1_0_to_v1_1.md Change 3's "Backward
    migration" section.
    """
    if pc is None:
        return None, None
    if not pc.derivation_equation_constants:
        return pc.algorithm_type, ""
    ordered = _sorted_constants_by_name(pc.derivation_equation_constants)
    return pc.algorithm_type, ",".join(str(c.value) for c in ordered)


def backward_flat_fields_points(
    pc: Optional[PowerCharacterization],
) -> tuple[Optional[str], Optional[str]]:
    """Points-shape backward derivation (Change 4 / Light_Source):
    PowerCharacterization -> (Algorithm_Type, Watts_To_Volts_Params).

    ``(None, None)`` if ``pc`` is ``None``. No re-sort needed —
    Characterization_Points rows aren't named, and on-disk order is already
    correct. Blank params if empty, same absence-not-ambiguity reasoning as
    the coefficients shape.
    """
    if pc is None:
        return None, None
    if not pc.characterization_points:
        return pc.algorithm_type, ""
    flat: list[str] = []
    for p in pc.characterization_points:
        flat.append(str(p.input_value))
        flat.append(str(p.output_value))
    return pc.algorithm_type, ",".join(flat)
