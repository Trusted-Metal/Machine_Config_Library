"""Extension contract: the opaque, language-neutral tree core uses to carry data that belongs to an
extension (a group under ``Extensions/<name>/``), so a separate library can pack and unpack it
without core knowing its schema.

Dependency-free on purpose (no h5py, no numpy): extension libraries import only this module. Core
reads and writes every tree verbatim, so unknown or private extensions round-trip untouched.

Pre-stable (0.x) until a second extension has been ported against it. The Node, Rust, Go and C++
implementations produce the identical canonical digest and JSON form; see
``docs/EXTENSION_CONTRACT.md``.
"""
from __future__ import annotations

import hashlib
import math
import struct
from dataclasses import dataclass, field
from typing import Any, Optional, Union

EXTENSION_CONTRACT_VERSION = "0.1.1"

#: Fixed-width storage types; each maps 1:1 to an HDF5 type.
DTYPES = ("utf8", "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "f32", "f64")

_INT_RANGES = {
    "i8": (-(2**7), 2**7 - 1),
    "u8": (0, 2**8 - 1),
    "i16": (-(2**15), 2**15 - 1),
    "u16": (0, 2**16 - 1),
    "i32": (-(2**31), 2**31 - 1),
    "u32": (0, 2**32 - 1),
    "i64": (-(2**63), 2**63 - 1),
    "u64": (0, 2**64 - 1),
}
_FLOAT_DTYPES = ("f32", "f64")
_DTYPE_TAG = {
    "utf8": 1, "i8": 2, "u8": 3, "i16": 4, "u16": 5, "i32": 6, "u32": 7, "i64": 8, "u64": 9,
    "f32": 10, "f64": 11,
}

Scalar = Union[str, int, float]


class ExtensionContractError(ValueError):
    """``path`` is the slash-joined location of the offending node; attributes are prefixed ``@``."""

    def __init__(self, path: str, message: str) -> None:
        super().__init__(f"{path or '<root>'}: {message}")
        self.path = path
        self.message = message


@dataclass
class Attr:
    """A group or dataset attribute: a scalar or a 1-D list of one dtype."""

    dtype: str
    value: Union[Scalar, list]


@dataclass
class Dataset:
    """N-d numeric array stored row-major in ``data`` (a flat list); ``len(data)`` equals the
    product of ``shape`` (empty shape = scalar)."""

    dtype: str
    shape: list
    data: list
    attrs: dict = field(default_factory=dict)


@dataclass
class TableField:
    """One column of a :class:`Table`. ``size`` is the fixed byte length of a ``utf8`` column and
    ``None`` for numeric ones."""

    name: str
    dtype: str
    size: Optional[int] = None


@dataclass
class Table:
    """1-D compound dataset (HDF5 compound type): one row per record, one named column per field.
    ``fields`` order is the compound member order and is significant; ``columns[name]`` is a list of
    ``rows`` values of that field's dtype."""

    fields: list
    rows: int
    columns: dict
    attrs: dict = field(default_factory=dict)


@dataclass
class ExtensionTree:
    """Order of attrs/children/datasets/tables is not significant; the canonical form sorts by UTF-8
    bytes. ``tables`` is empty unless used, so a tree without compound data is encoded and hashed
    exactly as in contract 0.1.0."""

    attrs: dict = field(default_factory=dict)
    children: dict = field(default_factory=dict)
    datasets: dict = field(default_factory=dict)
    tables: dict = field(default_factory=dict)


def _join(path: str, name: str) -> str:
    return f"{path}/{name}" if path else name


def validate_node_name(name: str, path: str) -> None:
    """Names become HDF5 link names: non-empty, no ``/``, no NUL."""
    if not isinstance(name, str) or name == "":
        raise ExtensionContractError(path, "name must not be empty")
    if "/" in name:
        raise ExtensionContractError(path, f'name "{name}" must not contain "/"')
    if "\0" in name:
        raise ExtensionContractError(path, "name must not contain NUL")


def _check_scalar(dtype: str, v: Any, path: str) -> None:
    if dtype == "utf8":
        if not isinstance(v, str):
            raise ExtensionContractError(path, f"utf8 value must be a string, got {type(v).__name__}")
        if "\0" in v:
            raise ExtensionContractError(path, "utf8 value must not contain NUL")
        return
    if isinstance(v, bool) or isinstance(v, str) or not isinstance(v, (int, float)):
        raise ExtensionContractError(path, f"{dtype} value must be a number, got {type(v).__name__}")
    if dtype in _FLOAT_DTYPES:
        return
    if isinstance(v, float):
        if not v.is_integer():
            raise ExtensionContractError(path, f"{dtype} value must be an integer, got {v}")
        v = int(v)
    lo, hi = _INT_RANGES[dtype]
    if v < lo or v > hi:
        raise ExtensionContractError(path, f"{dtype} value {v} is out of range")


def _validate_attrs(attrs: dict, path: str) -> None:
    for name, attr in attrs.items():
        attr_path = _join(path, f"@{name}")
        validate_node_name(name, attr_path)
        if attr.dtype not in DTYPES:
            raise ExtensionContractError(attr_path, f'unknown dtype "{attr.dtype}"')
        if isinstance(attr.value, list):
            for i, v in enumerate(attr.value):
                _check_scalar(attr.dtype, v, f"{attr_path}[{i}]")
        else:
            _check_scalar(attr.dtype, attr.value, attr_path)


def validate_tree(tree: ExtensionTree, path: str = "") -> None:
    """Raise :class:`ExtensionContractError` naming the first invalid node."""
    _validate_attrs(tree.attrs, path)
    for name, child in tree.children.items():
        child_path = _join(path, name)
        validate_node_name(name, child_path)
        validate_tree(child, child_path)
    for name, ds in tree.datasets.items():
        ds_path = _join(path, name)
        validate_node_name(name, ds_path)
        if name in tree.children:
            raise ExtensionContractError(ds_path, "a dataset and a child group cannot share a name")
        if ds.dtype not in DTYPES or ds.dtype == "utf8":
            raise ExtensionContractError(ds_path, f'dataset dtype must be numeric, got "{ds.dtype}"')
        if not all(isinstance(n, int) and not isinstance(n, bool) and n >= 0 for n in ds.shape):
            raise ExtensionContractError(ds_path, f"invalid shape [{','.join(map(str, ds.shape))}]")
        expected = math.prod(ds.shape)
        if len(ds.data) != expected:
            raise ExtensionContractError(
                ds_path,
                f"data length {len(ds.data)} does not match shape [{','.join(map(str, ds.shape))}]",
            )
        for i, v in enumerate(ds.data):
            _check_scalar(ds.dtype, v, f"{ds_path}[{i}]")
        _validate_attrs(ds.attrs, ds_path)
    for name, table in tree.tables.items():
        table_path = _join(path, name)
        validate_node_name(name, table_path)
        if name in tree.children or name in tree.datasets:
            raise ExtensionContractError(
                table_path, "a table cannot share a name with a dataset or child group"
            )
        _validate_table(table, table_path)


def _validate_table(table: Table, path: str) -> None:
    if not table.fields:
        raise ExtensionContractError(path, "a table needs at least one field")
    if not isinstance(table.rows, int) or isinstance(table.rows, bool) or table.rows < 0:
        raise ExtensionContractError(path, f"invalid row count {table.rows}")
    names: set = set()
    for f in table.fields:
        field_path = _join(path, f.name)
        validate_node_name(f.name, field_path)
        if f.name in names:
            raise ExtensionContractError(field_path, "duplicate field name")
        names.add(f.name)
        if f.dtype not in DTYPES:
            raise ExtensionContractError(field_path, f'unknown dtype "{f.dtype}"')
        if f.name not in table.columns:
            raise ExtensionContractError(field_path, "missing column")
        column = table.columns[f.name]
        if len(column) != table.rows:
            raise ExtensionContractError(
                field_path, f"column length {len(column)} does not match {table.rows} rows"
            )
        if f.dtype == "utf8":
            if not isinstance(f.size, int) or isinstance(f.size, bool) or f.size < 1:
                raise ExtensionContractError(field_path, "a utf8 field needs a positive integer size")
        elif f.size is not None:
            raise ExtensionContractError(field_path, "size is only valid for utf8 fields")
        for i, v in enumerate(column):
            row_path = f"{field_path}[{i}]"
            _check_scalar(f.dtype, v, row_path)
            if f.dtype == "utf8" and len(v.encode("utf-8")) > f.size:
                raise ExtensionContractError(
                    row_path, f"{len(v.encode('utf-8'))} UTF-8 bytes do not fit the {f.size}-byte field"
                )
    for name in table.columns:
        if name not in names:
            raise ExtensionContractError(_join(path, name), "column has no matching field")
    _validate_attrs(table.attrs, path)


# --------------------------------------------------------------------------------------------
# Canonical digest
#
# SHA-256 over a canonical binary encoding, identical in every language (all little-endian):
#
#   tree    = u32 attrCount, attr*, u32 childCount, (name tree)*, u32 dsCount, (name dataset)*
#   name    = u32 byteLength, UTF-8 bytes
#   attr    = name, u8 dtypeTag, u8 kind (0 scalar, 1 array), [u32 count], value*
#   dataset = u8 dtypeTag, u32 rank, u64 dim*, u64 count, value*, attr map (as in tree)
#   value   = utf8: u32 byteLength + bytes; ints: fixed width; f32/f64: IEEE bits
#
# Appended after the datasets only when a tree has tables (so trees without them hash as in 0.1.0):
#
#   tables  = u32 0xFFFFFFFF, u32 tableCount, (name table)*
#   table   = u32 fieldCount, (name, u8 dtypeTag, u32 size)*, u64 rows, column values*, attr map
#
# Table fields keep their declared order (it is the compound member order); each column's values
# follow in that order, row by row within a column.
#
# Entries are sorted by the UTF-8 bytes of their names. NaN is the quiet NaN and -0 is +0.
# --------------------------------------------------------------------------------------------

_STRUCT = {
    "i8": "<b", "u8": "<B", "i16": "<h", "u16": "<H", "i32": "<i", "u32": "<I", "i64": "<q",
    "u64": "<Q",
}


def _sorted_names(names) -> list:
    return sorted(names, key=lambda n: n.encode("utf-8"))


def _put_name(parts: list, s: str) -> None:
    b = s.encode("utf-8")
    parts.append(struct.pack("<I", len(b)))
    parts.append(b)


def _put_value(parts: list, dtype: str, v: Any) -> None:
    if dtype == "utf8":
        _put_name(parts, v)
    elif dtype == "f32":
        n = float(v)
        if math.isnan(n):
            parts.append(struct.pack("<I", 0x7FC00000))
        else:
            parts.append(struct.pack("<f", 0.0 if n == 0 else n))
    elif dtype == "f64":
        n = float(v)
        if math.isnan(n):
            parts.append(struct.pack("<Q", 0x7FF8000000000000))
        else:
            parts.append(struct.pack("<d", 0.0 if n == 0 else n))
    else:
        parts.append(struct.pack(_STRUCT[dtype], int(v)))


def _put_attrs(parts: list, attrs: dict) -> None:
    names = _sorted_names(attrs)
    parts.append(struct.pack("<I", len(names)))
    for n in names:
        a = attrs[n]
        _put_name(parts, n)
        parts.append(bytes([_DTYPE_TAG[a.dtype]]))
        if isinstance(a.value, list):
            parts.append(b"\x01")
            parts.append(struct.pack("<I", len(a.value)))
            for v in a.value:
                _put_value(parts, a.dtype, v)
        else:
            parts.append(b"\x00")
            _put_value(parts, a.dtype, a.value)


def _put_tree(parts: list, t: ExtensionTree) -> None:
    _put_attrs(parts, t.attrs)
    children = _sorted_names(t.children)
    parts.append(struct.pack("<I", len(children)))
    for n in children:
        _put_name(parts, n)
        _put_tree(parts, t.children[n])
    datasets = _sorted_names(t.datasets)
    parts.append(struct.pack("<I", len(datasets)))
    for n in datasets:
        ds = t.datasets[n]
        _put_name(parts, n)
        parts.append(bytes([_DTYPE_TAG[ds.dtype]]))
        parts.append(struct.pack("<I", len(ds.shape)))
        for d in ds.shape:
            parts.append(struct.pack("<Q", d))
        parts.append(struct.pack("<Q", len(ds.data)))
        for v in ds.data:
            _put_value(parts, ds.dtype, v)
        _put_attrs(parts, ds.attrs)
    tables = _sorted_names(t.tables)
    if tables:
        parts.append(struct.pack("<II", 0xFFFFFFFF, len(tables)))
        for n in tables:
            table = t.tables[n]
            _put_name(parts, n)
            parts.append(struct.pack("<I", len(table.fields)))
            for f in table.fields:
                _put_name(parts, f.name)
                parts.append(bytes([_DTYPE_TAG[f.dtype]]))
                parts.append(struct.pack("<I", f.size or 0))
            parts.append(struct.pack("<Q", table.rows))
            for f in table.fields:
                for v in table.columns[f.name]:
                    _put_value(parts, f.dtype, v)
            _put_attrs(parts, table.attrs)


def canonical_digest(tree: ExtensionTree) -> str:
    """Hex SHA-256 of the canonical encoding. Raises if the tree violates the contract."""
    validate_tree(tree)
    parts: list = []
    _put_tree(parts, tree)
    return hashlib.sha256(b"".join(parts)).hexdigest()


# --------------------------------------------------------------------------------------------
# JSON form (used by fixtures and ``to_json``): i64/u64 as decimal strings, non-finite floats as
# "NaN" / "Infinity" / "-Infinity", so the JSON is lossless in every language.
# --------------------------------------------------------------------------------------------


def _scalar_to_json(dtype: str, v: Any) -> Any:
    if dtype in ("i64", "u64"):
        return str(int(v))
    if dtype in _FLOAT_DTYPES:
        f = float(v)
        if math.isnan(f):
            return "NaN"
        if math.isinf(f):
            return "Infinity" if f > 0 else "-Infinity"
        return f
    return v


def _attrs_to_json(attrs: dict) -> dict:
    out = {}
    for name, a in attrs.items():
        if isinstance(a.value, list):
            value = [_scalar_to_json(a.dtype, v) for v in a.value]
        else:
            value = _scalar_to_json(a.dtype, a.value)
        out[name] = {"dtype": a.dtype, "value": value}
    return out


def tree_to_json(tree: ExtensionTree) -> dict:
    out = {
        "attrs": _attrs_to_json(tree.attrs),
        "children": {n: tree_to_json(c) for n, c in tree.children.items()},
        "datasets": {
            n: {
                "dtype": ds.dtype,
                "shape": list(ds.shape),
                "data": [_scalar_to_json(ds.dtype, v) for v in ds.data],
                "attrs": _attrs_to_json(ds.attrs),
            }
            for n, ds in tree.datasets.items()
        },
    }
    if tree.tables:
        out["tables"] = {
            n: {
                "fields": [
                    {"name": f.name, "dtype": f.dtype, **({} if f.size is None else {"size": f.size})}
                    for f in t.fields
                ],
                "rows": t.rows,
                "columns": {
                    f.name: [_scalar_to_json(f.dtype, v) for v in t.columns[f.name]] for f in t.fields
                },
                "attrs": _attrs_to_json(t.attrs),
            }
            for n, t in tree.tables.items()
        }
    return out


def _scalar_from_json(dtype: str, v: Any, path: str) -> Scalar:
    if dtype == "utf8":
        if not isinstance(v, str):
            raise ExtensionContractError(path, "utf8 JSON value must be a string")
        return v
    if dtype in ("i64", "u64"):
        if not isinstance(v, str) or not v.lstrip("-").isdigit():
            raise ExtensionContractError(path, f"{dtype} JSON value must be a decimal string")
        return int(v)
    if isinstance(v, str):
        if dtype not in _FLOAT_DTYPES:
            raise ExtensionContractError(path, f"{dtype} JSON value must be a number")
        if v == "NaN":
            return math.nan
        if v == "Infinity":
            return math.inf
        if v == "-Infinity":
            return -math.inf
        raise ExtensionContractError(path, f'invalid float JSON value "{v}"')
    if isinstance(v, bool) or not isinstance(v, (int, float)):
        raise ExtensionContractError(path, f"{dtype} JSON value must be a number")
    return v


def _attrs_from_json(d: Any, path: str) -> dict:
    out = {}
    for name, a in (d or {}).items():
        attr_path = _join(path, f"@{name}")
        dtype = a.get("dtype")
        if dtype not in DTYPES:
            raise ExtensionContractError(attr_path, f'unknown dtype "{dtype}"')
        raw = a.get("value")
        if isinstance(raw, list):
            value = [_scalar_from_json(dtype, v, f"{attr_path}[{i}]") for i, v in enumerate(raw)]
        else:
            value = _scalar_from_json(dtype, raw, attr_path)
        out[name] = Attr(dtype, value)
    return out


def _tree_from_json(d: dict, path: str) -> ExtensionTree:
    tree = ExtensionTree(attrs=_attrs_from_json(d.get("attrs"), path))
    for name, child in (d.get("children") or {}).items():
        tree.children[name] = _tree_from_json(child, _join(path, name))
    for name, ds in (d.get("datasets") or {}).items():
        ds_path = _join(path, name)
        dtype = ds.get("dtype")
        if dtype not in DTYPES or dtype == "utf8":
            raise ExtensionContractError(ds_path, f'dataset dtype must be numeric, got "{dtype}"')
        tree.datasets[name] = Dataset(
            dtype=dtype,
            shape=list(ds.get("shape", [])),
            data=[_scalar_from_json(dtype, v, f"{ds_path}[{i}]") for i, v in enumerate(ds.get("data", []))],
            attrs=_attrs_from_json(ds.get("attrs"), ds_path),
        )
    for name, t in (d.get("tables") or {}).items():
        tree.tables[name] = _table_from_json(t, _join(path, name))
    return tree


def _table_from_json(d: dict, path: str) -> Table:
    fields = []
    columns = {}
    for f in d.get("fields") or []:
        name = f.get("name")
        field_path = _join(path, str(name))
        dtype = f.get("dtype")
        if dtype not in DTYPES:
            raise ExtensionContractError(field_path, f'unknown dtype "{dtype}"')
        raw = (d.get("columns") or {}).get(name)
        if not isinstance(raw, list):
            raise ExtensionContractError(field_path, "missing column")
        fields.append(TableField(name, dtype, f.get("size")))
        columns[name] = [_scalar_from_json(dtype, v, f"{field_path}[{i}]") for i, v in enumerate(raw)]
    return Table(fields, d.get("rows", 0), columns, _attrs_from_json(d.get("attrs"), path))


def tree_from_json(d: dict) -> ExtensionTree:
    """Inverse of :func:`tree_to_json`; the result is validated."""
    tree = _tree_from_json(d, "")
    validate_tree(tree)
    return tree
