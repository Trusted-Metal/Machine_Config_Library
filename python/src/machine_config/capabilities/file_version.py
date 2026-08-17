"""Peek on-disk File_Version without assuming HDF5 group layout.

Root attribute ``File_Version`` is the adapter key. Everything else (groups,
attribute names, casting) lives in the matching File_Version adapter.
"""
from __future__ import annotations

from pathlib import Path

import h5py

FILE_VERSION_ATTR = "File_Version"


class UnsupportedFileVersion(ValueError):
    def __init__(self, version: str) -> None:
        self.version = version
        super().__init__(
            f'No adapter registered for File_Version "{version}". '
            "Each on-disk version needs its own adapter."
        )


class MissingRequiredGroup(KeyError):
    """A required HDF5 group is absent in an otherwise valid file."""
    def __init__(self, path: str) -> None:
        self.path = path
        super().__init__(f"Required HDF5 group missing: {path}")


def peek_file_version(path: str | Path) -> str:
    """Read only the root File_Version attribute. Does not walk groups."""
    with h5py.File(path, "r") as f:
        raw = f.attrs.get(FILE_VERSION_ATTR, "1.0")
        v = str(raw).strip()
        return v or "1.0"
