"""Compound tables (extension contract 0.1.1): validation, digest, JSON form, HDF5 round trip and
fixtures written by the Node and Rust implementations."""
from __future__ import annotations

import json
from pathlib import Path

import h5py
import numpy as np
import pytest

from machine_config.capabilities.v1_1.hdf5 import Hdf5AdapterV1_1
from machine_config.capabilities.v1_1.writer import Hdf5WriterV1_1
from machine_config.extension_contract import (
    EXTENSION_CONTRACT_VERSION,
    Attr,
    ExtensionContractError,
    ExtensionTree,
    Table,
    TableField,
    canonical_digest,
    tree_from_json,
    tree_to_json,
    validate_tree,
)

REPO = Path(__file__).resolve().parents[2]
FIXTURES = REPO / "fixtures"
# Same tree and digest as nodejs/tests/extension_tables.test.ts and extension-contract's tests.
TABLE_DIGEST = "600bca07eb400eea4d1bb726ffb8f6f5e274d1b02c58f62e08bb874789366858"
LEGACY_DIGEST = "8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890"


def points() -> Table:
    return Table(
        [TableField("input_value", "f64"), TableField("output_value", "f64")],
        2,
        {"input_value": [4.0, 20.0], "output_value": [-1.0, 6.0]},
    )


def table_tree() -> ExtensionTree:
    return ExtensionTree(
        attrs={"Schema": Attr("utf8", "1.0")},
        children={"Train_1": ExtensionTree(tables={"Points": points()})},
        tables={
            "Constants": Table(
                [TableField("name", "utf8", 64), TableField("value", "f64")],
                2,
                {"name": ["a", "b"], "value": [0.4375, -2.75]},
                {"Units": Attr("utf8", "V")},
            ),
            "Mixed": Table(
                [
                    TableField("id", "u32"),
                    TableField("count", "i64"),
                    TableField("ratio", "f32"),
                    TableField("flag", "u8"),
                    TableField("label", "utf8", 8),
                ],
                3,
                {
                    "id": [1, 2, 4294967295],
                    "count": [-9007199254740993, 0, 7],
                    "ratio": [0.5, -1.5, 2.0],
                    "flag": [0, 1, 1],
                    "label": ["x", "\u00b5m", ""],
                },
            ),
            "Empty": Table(
                [TableField("input_value", "f64"), TableField("output_value", "f64")],
                0,
                {"input_value": [], "output_value": []},
            ),
        },
    )


def v1_1_config():
    return Hdf5AdapterV1_1(str(FIXTURES / "reference_config_v1_1.h5")).parse()


def test_contract_stays_on_the_0_1_line():
    assert EXTENSION_CONTRACT_VERSION.startswith("0.1.")


def test_digest_is_pinned_and_matches_the_other_languages():
    assert canonical_digest(table_tree()) == TABLE_DIGEST


def test_empty_tables_map_does_not_change_the_digest():
    tree = ExtensionTree(attrs={"A": Attr("u32", 1)})
    plain = canonical_digest(tree)
    tree.tables = {}
    assert canonical_digest(tree) == plain
    assert "tables" not in tree_to_json(tree)


def test_golden_json_fixtures_keep_their_digests():
    legacy = json.loads((FIXTURES / "extension_sample_tree.json").read_text(encoding="utf-8"))
    assert canonical_digest(tree_from_json(legacy)) == LEGACY_DIGEST
    tables = json.loads((FIXTURES / "extension_table_sample_tree.json").read_text(encoding="utf-8"))
    assert canonical_digest(tree_from_json(tables)) == TABLE_DIGEST


def test_json_round_trips_losslessly():
    j = json.loads(json.dumps(tree_to_json(table_tree())))
    assert j["tables"]["Mixed"]["columns"]["count"] == ["-9007199254740993", "0", "7"]
    assert j["tables"]["Constants"]["fields"][0] == {"name": "name", "dtype": "utf8", "size": 64}
    assert j["tables"]["Constants"]["fields"][1] == {"name": "value", "dtype": "f64"}
    assert canonical_digest(tree_from_json(j)) == TABLE_DIGEST


def test_digest_is_sensitive_to_field_order_size_values_and_rows():
    base = canonical_digest(table_tree())

    swapped = table_tree()
    swapped.tables["Constants"].fields.reverse()
    assert canonical_digest(swapped) != base

    wider = table_tree()
    wider.tables["Constants"].fields[0].size = 32
    assert canonical_digest(wider) != base

    edited = table_tree()
    edited.tables["Constants"].columns["value"][1] = -2.5
    assert canonical_digest(edited) != base

    shorter = table_tree()
    p = shorter.children["Train_1"].tables["Points"]
    p.rows = 1
    p.columns = {"input_value": [4.0], "output_value": [-1.0]}
    assert canonical_digest(shorter) != base

    reordered = table_tree()
    reordered.tables = dict(reversed(list(reordered.tables.items())))
    assert canonical_digest(reordered) == base


def _invalid(mutate) -> ExtensionContractError:
    t = points()
    mutate(t)
    with pytest.raises(ExtensionContractError) as e:
        validate_tree(ExtensionTree(tables={"T": t}), "Ext")
    return e.value


def test_validation_rejects_malformed_tables_with_a_path():
    assert "at least one field" in str(_invalid(lambda t: t.fields.clear()))
    assert "duplicate field" in str(_invalid(lambda t: setattr(t.fields[1], "name", "input_value")))
    assert "does not match 3 rows" in str(_invalid(lambda t: setattr(t, "rows", 3)))
    assert "missing column" in str(_invalid(lambda t: t.columns.pop("output_value")))
    assert "no matching field" in str(_invalid(lambda t: t.columns.update(extra=[0.0, 0.0])))
    assert "only valid for utf8" in str(_invalid(lambda t: setattr(t.fields[0], "size", 8)))

    def no_size(t):
        t.fields[0] = TableField("input_value", "utf8")
        t.columns["input_value"] = ["a", "b"]

    assert "positive integer size" in str(_invalid(no_size))

    def too_long(t):
        t.fields[0] = TableField("input_value", "utf8", 2)
        t.columns["input_value"] = ["abc", "b"]

    assert "do not fit the 2-byte field" in str(_invalid(too_long))
    assert _invalid(lambda t: t.columns.update(output_value=[0.0])).path == "Ext/T/output_value"

    clash = ExtensionTree(children={"T": ExtensionTree()}, tables={"T": points()})
    with pytest.raises(ExtensionContractError, match="cannot share a name"):
        validate_tree(clash, "Ext")


# --------------------------------------------------------------------------- HDF5


def test_round_trips_every_table_verbatim(tmp_path):
    config = v1_1_config()
    config.extensions["Private_Ext"] = table_tree()
    out = tmp_path / "tables.h5"
    Hdf5WriterV1_1(config).write(str(out))

    got = Hdf5AdapterV1_1(str(out)).parse().extensions["Private_Ext"]
    assert canonical_digest(got) == TABLE_DIGEST
    assert [(f.name, f.dtype, f.size) for f in got.tables["Mixed"].fields] == [
        ("id", "u32", None),
        ("count", "i64", None),
        ("ratio", "f32", None),
        ("flag", "u8", None),
        ("label", "utf8", 8),
    ]
    assert got.tables["Mixed"].columns["label"] == ["x", "\u00b5m", ""]
    assert got.tables["Empty"].rows == 0
    assert got.tables["Constants"].attrs["Units"].value == "V"


def test_a_tree_without_tables_has_none_after_a_round_trip(tmp_path):
    config = v1_1_config()
    config.extensions["Private_Ext"] = ExtensionTree(attrs={"A": Attr("u32", 1)})
    out = tmp_path / "plain.h5"
    Hdf5WriterV1_1(config).write(str(out))
    assert Hdf5AdapterV1_1(str(out)).parse().extensions["Private_Ext"].tables == {}


def test_writer_rejects_an_invalid_table_before_creating_the_file(tmp_path):
    bad = table_tree()
    bad.tables["Constants"].columns["name"] = ["only-one"]
    config = v1_1_config()
    config.extensions["Bad"] = bad
    out = tmp_path / "never.h5"
    with pytest.raises(ExtensionContractError):
        Hdf5WriterV1_1(config).write(str(out))
    assert not out.exists()


@pytest.mark.parametrize("name", ["node", "rust"])
def test_reads_fixtures_written_by_other_languages(name):
    config = Hdf5AdapterV1_1(str(FIXTURES / f"extension_table_sample_{name}_v1_1.h5")).parse()
    assert canonical_digest(config.extensions["Private_Ext"]) == TABLE_DIGEST


# --------------------------------------------------------------------------- unsupported shapes


def _file_with(tmp_path, add):
    config = v1_1_config()
    config.extensions["Private_Ext"] = ExtensionTree()
    out = tmp_path / "unsupported.h5"
    Hdf5WriterV1_1(config).write(str(out))
    with h5py.File(out, "r+") as f:
        add(f["Extensions/Private_Ext"])
    return out


def test_rejects_a_compound_dataset_that_is_not_1_d(tmp_path):
    out = _file_with(
        tmp_path, lambda g: g.create_dataset("Grid", data=np.zeros((2, 2), dtype=[("a", "<f8")]))
    )
    with pytest.raises(ExtensionContractError, match=r"Private_Ext/Grid.*1-D"):
        Hdf5AdapterV1_1(str(out)).parse()


def test_rejects_an_array_member(tmp_path):
    out = _file_with(
        tmp_path, lambda g: g.create_dataset("Arr", data=np.zeros(1, dtype=[("a", "<f8", (2,))]))
    )
    with pytest.raises(ExtensionContractError, match=r"Private_Ext/Arr/a"):
        Hdf5AdapterV1_1(str(out)).parse()


def test_rejects_a_variable_length_string_member(tmp_path):
    dt = np.dtype([("s", h5py.string_dtype("utf-8"))])
    out = _file_with(
        tmp_path, lambda g: g.create_dataset("Vl", data=np.array([("x",)], dtype=dt), dtype=dt)
    )
    with pytest.raises(ExtensionContractError, match=r"Private_Ext/Vl/s.*variable-length"):
        Hdf5AdapterV1_1(str(out)).parse()
