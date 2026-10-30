"""Extension contract: validation, canonical digest, JSON form, HDF5 round trip, cross-language fixtures."""
from __future__ import annotations

import copy
import json
from pathlib import Path

import h5py
import pytest

from machine_config import MachineConfigReader
from machine_config.capabilities.v1_0.hdf5 import Hdf5AdapterV1_0
from machine_config.capabilities.v1_0.writer import Hdf5WriterV1_0
from machine_config.capabilities.v1_1.hdf5 import Hdf5AdapterV1_1
from machine_config.capabilities.v1_1.writer import Hdf5WriterV1_1
from machine_config.extension_contract import (
    Attr,
    Dataset,
    ExtensionContractError,
    ExtensionTree,
    canonical_digest,
    tree_from_json,
    tree_to_json,
    validate_tree,
)
from machine_config.hash import compute_configuration_hash

REPO = Path(__file__).resolve().parents[2]
FIXTURES = REPO / "fixtures"
PINNED_DIGEST = "8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890"


def sample_tree() -> ExtensionTree:
    return ExtensionTree(
        attrs={
            "Schema": Attr("utf8", "1.2"),
            "Flag": Attr("u32", 1),
            "Float_Flag": Attr("f64", 1.0),
            "Small": Attr("i8", -5),
            "Wide": Attr("i64", -9007199254740993),
            "Huge": Attr("u64", 18446744073709551615),
            "Ratio": Attr("f32", 0.5),
            "Pair": Attr("f64", [1.5, -2.5]),
            "Ids": Attr("u32", [1, 2, 3]),
        },
        children={
            "Job Start": ExtensionTree(
                attrs={"Label": Attr("utf8", "Job Start")},
                children={"Condition 1": ExtensionTree(attrs={"Value": Attr("utf8", "running")})},
            )
        },
        datasets={
            "Grid": Dataset(
                "f64", [2, 3], [1.0, 2.0, 3.0, 4.0, 5.0, 6.5], {"Units": Attr("utf8", "mm")}
            )
        },
    )


def v1_1_config():
    return Hdf5AdapterV1_1(str(FIXTURES / "reference_config_v1_1.h5")).parse()


# --------------------------------------------------------------------------- contract


def test_pinned_conformance_digest():
    assert canonical_digest(sample_tree()) == PINNED_DIGEST


def test_digest_ignores_insertion_order():
    t = sample_tree()
    reordered = ExtensionTree(
        attrs=dict(reversed(list(t.attrs.items()))),
        children=t.children,
        datasets=t.datasets,
    )
    assert canonical_digest(reordered) == PINNED_DIGEST


def test_json_round_trip_is_lossless():
    j = json.loads(json.dumps(tree_to_json(sample_tree())))
    back = tree_from_json(j)
    assert back.attrs["Wide"].value == -9007199254740993
    assert back.attrs["Huge"].value == 18446744073709551615
    assert canonical_digest(back) == PINNED_DIGEST


@pytest.mark.parametrize(
    "tree, fragment",
    [
        (ExtensionTree(attrs={"A": Attr("u8", 300)}), "A"),
        (ExtensionTree(attrs={"A": Attr("i32", "x")}), "A"),
        (ExtensionTree(attrs={"A": Attr("bogus", 1)}), "A"),
        (ExtensionTree(attrs={"": Attr("u8", 1)}), ""),
        (ExtensionTree(children={"a/b": ExtensionTree()}), "a/b"),
        (
            ExtensionTree(
                children={"X": ExtensionTree()},
                datasets={"X": Dataset("f64", [1], [1.0])},
            ),
            "X",
        ),
        (ExtensionTree(datasets={"D": Dataset("f64", [2, 2], [1.0, 2.0, 3.0])}), "D"),
    ],
)
def test_validation_rejects_invalid_trees(tree, fragment):
    with pytest.raises(ExtensionContractError) as exc:
        validate_tree(tree, "Ext")
    assert exc.value.path.startswith("Ext")
    assert fragment in exc.value.path or fragment == ""


def test_validation_error_reports_the_nested_path():
    bad = ExtensionTree(children={"Outer": ExtensionTree(attrs={"A": Attr("u8", -1)})})
    with pytest.raises(ExtensionContractError) as exc:
        validate_tree(bad, "Ext")
    assert exc.value.path == "Ext/Outer/@A"


# --------------------------------------------------------------------------- HDF5 round trip


def test_v1_1_round_trip_preserves_digest(tmp_path):
    config = v1_1_config()
    config.extensions["Private_Ext"] = sample_tree()
    out = tmp_path / "ext.h5"
    Hdf5WriterV1_1(config).write(str(out))

    with h5py.File(out, "r") as f:
        assert "Private_Ext" in f["Extensions"]

    back = Hdf5AdapterV1_1(str(out)).parse()
    assert "Private_Ext" in back.extensions
    assert canonical_digest(back.extensions["Private_Ext"]) == PINNED_DIGEST


@pytest.mark.parametrize("name", ["node", "rust"])
def test_reads_fixtures_written_by_other_languages(name):
    config = Hdf5AdapterV1_1(str(FIXTURES / f"extension_sample_{name}_v1_1.h5")).parse()
    assert canonical_digest(config.extensions["Private_Ext"]) == PINNED_DIGEST


def test_clearbox_is_an_ordinary_extension_in_v1_1():
    config = Hdf5AdapterV1_1(str(FIXTURES / "reference_config_v1_1.h5")).parse()
    assert "ClearBox" in config.extensions


def test_writer_rejects_invalid_tree_before_creating_the_file(tmp_path):
    config = v1_1_config()
    config.extensions["Bad"] = ExtensionTree(attrs={"A": Attr("u8", 999)})
    out = tmp_path / "never.h5"
    with pytest.raises(ExtensionContractError):
        Hdf5WriterV1_1(config).write(str(out))
    assert not out.exists()


def test_writer_accepts_a_clearbox_extension_tree(tmp_path):
    config = v1_1_config()
    config.extensions["ClearBox"] = ExtensionTree(attrs={"Output_Path": Attr("utf8", "x")})
    out = tmp_path / "clearbox.h5"
    Hdf5WriterV1_1(config).write(str(out))
    back = Hdf5AdapterV1_1(str(out)).parse()
    assert back.extensions["ClearBox"].attrs["Output_Path"].value == "x"


# --------------------------------------------------------------------------- legacy 1.0


def test_v1_0_opcua_group_round_trips_as_tm_opcua(tmp_path):
    fixture = FIXTURES / "reference_config_opcua.h5"
    config = Hdf5AdapterV1_0(str(fixture)).parse()
    original = canonical_digest(config.extensions["TM_OPCUA"])

    out = tmp_path / "legacy.h5"
    Hdf5WriterV1_0(config).write(str(out))
    with h5py.File(out, "r") as f:
        assert "OPCUA" in f
    assert canonical_digest(Hdf5AdapterV1_0(str(out)).parse().extensions["TM_OPCUA"]) == original


def test_v1_0_cannot_hold_other_extensions(tmp_path):
    config = Hdf5AdapterV1_0(str(FIXTURES / "reference_config.h5")).parse()
    config.extensions["Private_Ext"] = sample_tree()
    with pytest.raises(ValueError):
        Hdf5WriterV1_0(config).write(str(tmp_path / "never.h5"))


def test_v1_0_opcua_group_migrates_to_v1_1_extension(tmp_path):
    config = Hdf5AdapterV1_0(str(FIXTURES / "reference_config_opcua.h5")).parse()
    original = canonical_digest(config.extensions["TM_OPCUA"])
    config.meta.file_version = "1.1"
    out = tmp_path / "migrated.h5"
    Hdf5WriterV1_1(config).write(str(out))
    with h5py.File(out, "r") as f:
        assert "TM_OPCUA" in f["Extensions"]
    assert canonical_digest(Hdf5AdapterV1_1(str(out)).parse().extensions["TM_OPCUA"]) == original


# --------------------------------------------------------------------------- hash


def test_hash_unchanged_without_extensions_and_changes_with_them():
    config = v1_1_config()
    base = compute_configuration_hash(config)

    with_ext = copy.deepcopy(config)
    with_ext.extensions["Private_Ext"] = sample_tree()
    assert compute_configuration_hash(with_ext) != base

    edited = copy.deepcopy(with_ext)
    edited.extensions["Private_Ext"].attrs["Flag"] = Attr("u32", 2)
    assert compute_configuration_hash(edited) != compute_configuration_hash(with_ext)

    cleared = copy.deepcopy(with_ext)
    del cleared.extensions["Private_Ext"]
    assert compute_configuration_hash(cleared) == base


def test_golden_json_fixture_has_the_pinned_digest():
    j = json.loads((FIXTURES / "extension_sample_tree.json").read_text(encoding="utf-8"))
    assert canonical_digest(tree_from_json(j)) == PINNED_DIGEST
