"""Test helpers for reading ClearBox, which core exposes as an opaque extension tree in its
File_Version 1.1 layout for both file versions (see ``machine_config.clearbox_legacy``)."""
from __future__ import annotations

import numpy as np

from machine_config.extension_contract import Attr, ExtensionTree, Table, TableField
from machine_config.models import MachineConfig


def train_key(index: int) -> str:
    return f"Optical_Train_{index + 1:02d}"


def clearbox_root(cfg: MachineConfig) -> ExtensionTree:
    tree = cfg.extensions.get("ClearBox")
    if tree is None:
        raise AssertionError("config has no ClearBox extension")
    return tree


def clearbox(cfg: MachineConfig, index: int) -> ExtensionTree:
    return clearbox_root(cfg).children[train_key(index)]


def power_characterization(cfg: MachineConfig, index: int) -> ExtensionTree:
    return clearbox(cfg, index).children["Power_Characterization"]


def attr(tree: ExtensionTree, name: str):
    """The attribute's value, or ``None`` when absent."""
    a = tree.attrs.get(name)
    return None if a is None else a.value


def text(tree: ExtensionTree, name: str):
    """Non-empty string attribute; the contract stores "absent" as ``""``."""
    v = attr(tree, name)
    return v if isinstance(v, str) and v != "" else None


def integer(tree: ExtensionTree, name: str):
    v = attr(tree, name)
    return v if isinstance(v, int) and not isinstance(v, bool) else None


def constants(pc: ExtensionTree) -> list:
    t = pc.tables["Derivation_Equation_Constants"]
    return list(zip(t.columns["name"], t.columns["value"]))


def point_count(pc: ExtensionTree) -> int:
    return pc.tables["Characterization_Points"].rows


def make_power_characterization(algorithm: str, rows: list) -> ExtensionTree:
    tree = ExtensionTree()
    for name, value in (
        ("Algorithm_Type", algorithm),
        ("Algorithm_Equation", ""),
        ("Input_Type", ""),
        ("Units_Derived_Quantity", ""),
    ):
        tree.attrs[name] = Attr("utf8", value)
    tree.tables["Derivation_Equation_Constants"] = Table(
        fields=[TableField("name", "utf8", 64), TableField("value", "f64")],
        rows=len(rows),
        columns={"name": [r[0] for r in rows], "value": [r[1] for r in rows]},
    )
    tree.tables["Characterization_Points"] = Table(
        fields=[TableField("input_value", "f64"), TableField("output_value", "f64")],
        rows=0,
        columns={"input_value": [], "output_value": []},
    )
    return tree


def sensor_tree(rows: list, attrs: dict | None = None) -> ExtensionTree:
    """A sensor group whose only content is a ``Derivation_Equation_Constants`` table."""
    tree = ExtensionTree()
    tree.attrs = dict(attrs or {})
    tree.tables["Derivation_Equation_Constants"] = Table(
        fields=[TableField("name", "utf8", 64), TableField("value", "f64")],
        rows=len(rows),
        columns={"name": [r[0] for r in rows], "value": [r[1] for r in rows]},
    )
    return tree


def add_sensor(cfg: MachineConfig, label: str, sensor: ExtensionTree) -> None:
    """Adds ``sensor`` under train 0's ``Synchronous_Sensors/<label>`` (mutates ``cfg``)."""
    train = clearbox(cfg, 0)
    sensors = train.children.setdefault("Synchronous_Sensors", ExtensionTree())
    sensors.children[label] = sensor


def grid(cfg: MachineConfig, index: int, name: str) -> np.ndarray:
    """A forward/inverse grid of train ``index`` as an array of its declared shape."""
    ds = clearbox(cfg, index).datasets[name]
    return np.array(ds.data, dtype="<f8").reshape(ds.shape)
