"""File_Version 1.0 keeps ClearBox per optical train under ``Optional_Components/ClearBox/``; core
exposes it as the opaque ``ClearBox`` extension tree, always in the File_Version 1.1 layout, so an
extension library never has to know which file version a tree came from::

    ClearBox/                      Output_Path, Software_Trigger_Delay   (one shared value each)
      <train_id>/                  per-train attributes, correction datasets, Synchronous_Sensors/
        Power_Characterization/    structured replacement for Volts_To_Watts_Algorithm/Params

Everything else in a v1.0 ClearBox group is carried verbatim, including the attributes v1.1 dropped
(``Selected_Camera``, ``Show_Console``, ...), so a v1.0 file round-trips without losing them. The two
shared values are taken from the first train that has a ClearBox.

Mirrors ``rust/src/clearbox_legacy.rs``.
"""
from __future__ import annotations

import copy
from typing import Optional

from machine_config.extension_contract import Attr, ExtensionTree, Table, TableField
from machine_config.models import CalibrationPoint, EquationConstant, PowerCharacterization
from machine_config.power_characterization import (
    backward_flat_fields_coefficients,
    forward_power_characterization_coefficients,
)

EXTENSION_NAME = "ClearBox"

_POWER_CHARACTERIZATION = "Power_Characterization"
_SHARED_ATTRS = ("Output_Path", "Software_Trigger_Delay")
_FLAT_ALGORITHM = "Volts_To_Watts_Algorithm"
_FLAT_PARAMS = "Volts_To_Watts_Params"
_V1_1_ONLY_ATTRS = ("Firmware_Version",)
_EQUATION_CONSTANT_NAME_BYTES = 64


def _text(tree: ExtensionTree, name: str) -> Optional[str]:
    attr = tree.attrs.get(name)
    if attr is None or not isinstance(attr.value, str):
        return None
    s = attr.value.strip()
    return s or None


def _str_attr(value: Optional[str]) -> Attr:
    return Attr("utf8", value if value is not None else "")


def _power_characterization_to_tree(pc: Optional[PowerCharacterization]) -> ExtensionTree:
    tree = ExtensionTree()
    tree.attrs["Algorithm_Type"] = _str_attr(pc.algorithm_type if pc else None)
    tree.attrs["Algorithm_Equation"] = _str_attr(pc.algorithm_equation if pc else None)
    tree.attrs["Input_Type"] = _str_attr(pc.input_type if pc else None)
    tree.attrs["Units_Derived_Quantity"] = _str_attr(pc.units_derived_quantity if pc else None)
    constants = pc.derivation_equation_constants if pc else []
    points = pc.characterization_points if pc else []
    tree.tables["Derivation_Equation_Constants"] = Table(
        fields=[TableField("name", "utf8", _EQUATION_CONSTANT_NAME_BYTES), TableField("value", "f64")],
        rows=len(constants),
        columns={"name": [c.name for c in constants], "value": [c.value for c in constants]},
    )
    tree.tables["Characterization_Points"] = Table(
        fields=[TableField("input_value", "f64"), TableField("output_value", "f64")],
        rows=len(points),
        columns={
            "input_value": [p.input_value for p in points],
            "output_value": [p.output_value for p in points],
        },
    )
    return tree


def _power_characterization_from_tree(tree: ExtensionTree) -> PowerCharacterization:
    constants: list = []
    t = tree.tables.get("Derivation_Equation_Constants")
    if t is not None and "name" in t.columns and "value" in t.columns:
        constants = [
            EquationConstant(name=n, value=float(v))
            for n, v in zip(t.columns["name"], t.columns["value"])
        ]
    points: list = []
    t = tree.tables.get("Characterization_Points")
    if t is not None and "input_value" in t.columns and "output_value" in t.columns:
        points = [
            CalibrationPoint(input_value=float(i), output_value=float(o))
            for i, o in zip(t.columns["input_value"], t.columns["output_value"])
        ]
    return PowerCharacterization(
        algorithm_type=_text(tree, "Algorithm_Type"),
        algorithm_equation=_text(tree, "Algorithm_Equation"),
        input_type=_text(tree, "Input_Type"),
        units_derived_quantity=_text(tree, "Units_Derived_Quantity"),
        derivation_equation_constants=constants,
        characterization_points=points,
    )


def from_v1_0_trains(trains: list) -> Optional[ExtensionTree]:
    """Build the ``ClearBox`` tree from each train's verbatim v1.0 group, in train order.

    ``trains`` is a list of ``(train_id, tree)``; returns ``None`` when no train has a ClearBox.
    """
    if not trains:
        return None
    root = ExtensionTree()
    first = trains[0][1]
    for name in _SHARED_ATTRS:
        if name in first.attrs:
            root.attrs[name] = first.attrs[name]
    for train_id, tree in trains:
        for name in _SHARED_ATTRS:
            tree.attrs.pop(name, None)
        algorithm = _text(tree, _FLAT_ALGORITHM)
        params = _text(tree, _FLAT_PARAMS)
        tree.attrs.pop(_FLAT_ALGORITHM, None)
        tree.attrs.pop(_FLAT_PARAMS, None)
        pc = forward_power_characterization_coefficients(algorithm, params)
        if _POWER_CHARACTERIZATION in tree.children:
            raise ValueError(
                f"{train_id}: a File_Version 1.0 ClearBox group cannot contain {_POWER_CHARACTERIZATION}"
            )
        tree.children[_POWER_CHARACTERIZATION] = _power_characterization_to_tree(pc)
        root.children[train_id] = tree
    return root


def to_v1_0_trains(tree: ExtensionTree) -> dict:
    """Split the ``ClearBox`` tree back into one verbatim v1.0 group per train id.

    Raises rather than dropping anything v1.0 has no place for: attributes or datasets directly on
    the extension root other than the two shared values.
    """

    def lossy(what: str) -> ValueError:
        return ValueError(f"ClearBox: {what} has no File_Version 1.0 location")

    for name in tree.attrs:
        if name not in _SHARED_ATTRS:
            raise lossy(f'attribute "{name}"')
    for name in [*tree.datasets, *tree.tables]:
        raise lossy(f'dataset "{name}"')

    out: dict = {}
    for train_id, child in tree.children.items():
        group = copy.deepcopy(child)
        for name in _SHARED_ATTRS:
            if name not in group.attrs and name in tree.attrs:
                group.attrs[name] = tree.attrs[name]
        for name in _V1_1_ONLY_ATTRS:
            group.attrs.pop(name, None)
        pc_tree = group.children.pop(_POWER_CHARACTERIZATION, None)
        pc = _power_characterization_from_tree(pc_tree) if pc_tree is not None else None
        algorithm, params = backward_flat_fields_coefficients(pc)
        group.attrs[_FLAT_ALGORITHM] = _str_attr(algorithm)
        group.attrs[_FLAT_PARAMS] = _str_attr(params)
        out[train_id] = group
    return out
