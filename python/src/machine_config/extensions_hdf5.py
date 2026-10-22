"""Read and write :class:`~machine_config.extension_contract.ExtensionTree` as HDF5 groups.

Version-independent: the adapters decide *where* an extension lives (``Extensions/<name>/`` in 1.1,
the legacy ``OPCUA/`` group in 1.0); this module only maps the contract's tree to and from the HDF5
group at that location. Everything is read and written verbatim with dtypes preserved, so
extensions core has no model for survive a read-modify-write.
"""
from __future__ import annotations

import h5py
import numpy as np

from machine_config.extension_contract import (
    Attr,
    Dataset,
    ExtensionContractError,
    ExtensionTree,
    validate_tree,
)

_NP_DTYPE = {
    "i8": np.int8, "u8": np.uint8, "i16": np.int16, "u16": np.uint16, "i32": np.int32,
    "u32": np.uint32, "i64": np.int64, "u64": np.uint64, "f32": np.float32, "f64": np.float64,
}


def _join(path: str, name: str) -> str:
    return f"{path}/{name}" if path else name


def _dtype_name(dt: np.dtype, path: str) -> str:
    if h5py.check_string_dtype(dt) is not None:
        return "utf8"
    kind, size = dt.kind, dt.itemsize
    if kind == "i" and size in (1, 2, 4, 8):
        return f"i{size * 8}"
    if kind == "u" and size in (1, 2, 4, 8):
        return f"u{size * 8}"
    if kind == "f" and size in (4, 8):
        return f"f{size * 8}"
    raise ExtensionContractError(path, f"unsupported HDF5 type {dt}")


def _text(x) -> str:
    return x.decode("utf-8") if isinstance(x, (bytes, np.bytes_)) else str(x)


def _read_attr(obj, name: str, path: str) -> Attr:
    aid = obj.attrs.get_id(name)
    shape = aid.shape
    if len(shape) > 1:
        raise ExtensionContractError(path, f"attributes must be scalar or 1-D, got rank {len(shape)}")
    dtype = _dtype_name(aid.dtype, path)
    value = obj.attrs[name]
    if dtype == "utf8":
        if len(shape) == 0:
            return Attr(dtype, _text(value.item() if isinstance(value, np.ndarray) else value))
        return Attr(dtype, [_text(x) for x in value])
    if len(shape) == 0:
        return Attr(dtype, value.item() if hasattr(value, "item") else value)
    return Attr(dtype, np.asarray(value).tolist())


def _read_attrs(obj, path: str) -> dict:
    return {name: _read_attr(obj, name, _join(path, f"@{name}")) for name in obj.attrs.keys()}


def read_extension_tree(group: h5py.Group, path: str = "") -> ExtensionTree:
    """Read a group and everything below it verbatim; unsupported HDF5 types fail with their path."""
    tree = ExtensionTree(attrs=_read_attrs(group, path))
    for name in group.keys():
        child_path = _join(path, name)
        obj = group[name]
        if isinstance(obj, h5py.Group):
            tree.children[name] = read_extension_tree(obj, child_path)
        elif isinstance(obj, h5py.Dataset):
            dtype = _dtype_name(obj.dtype, child_path)
            if dtype == "utf8":
                raise ExtensionContractError(child_path, "string datasets are not supported")
            data = np.asarray(obj[()]).ravel().tolist()
            tree.datasets[name] = Dataset(
                dtype=dtype, shape=list(obj.shape), data=data, attrs=_read_attrs(obj, child_path)
            )
        else:
            raise ExtensionContractError(child_path, "only groups and datasets are supported")
    return tree


def _write_attrs(obj, attrs: dict) -> None:
    for name, attr in attrs.items():
        if attr.dtype == "utf8":
            sdt = h5py.string_dtype("utf-8")
            if isinstance(attr.value, list):
                obj.attrs.create(name, data=np.array(attr.value, dtype=object), dtype=sdt)
            else:
                obj.attrs.create(name, data=attr.value, dtype=sdt)
        else:
            npdt = _NP_DTYPE[attr.dtype]
            obj.attrs.create(name, data=np.array(attr.value, dtype=npdt))


def _write_group(group: h5py.Group, tree: ExtensionTree) -> None:
    _write_attrs(group, tree.attrs)
    for name, ds in tree.datasets.items():
        arr = np.array(ds.data, dtype=_NP_DTYPE[ds.dtype]).reshape(tuple(ds.shape))
        dset = group.create_dataset(name, data=arr)
        _write_attrs(dset, ds.attrs)
    for name, child in tree.children.items():
        _write_group(group.create_group(name), child)


def write_extension_tree(parent: h5py.Group, name: str, tree: ExtensionTree) -> None:
    """Write *tree* as a new group *name* under *parent*; the caller guarantees *name* is unused."""
    validate_tree(tree, name)
    _write_group(parent.create_group(name), tree)
