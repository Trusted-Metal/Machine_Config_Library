"""Public HDF5 reader: peek File_Version, then dispatch to a version adapter.

On-disk group paths and HDF5 attribute names live in the matching adapter
(e.g. ``capabilities.v1_0``). This module does not assume a v1.0 layout.
"""
from __future__ import annotations

from pathlib import Path
from typing import Protocol, runtime_checkable

from machine_config.capabilities.file_version import (
    UnsupportedFileVersion,
    peek_file_version,
)
from machine_config.capabilities.v1_0.hdf5 import Hdf5AdapterV1_0, config_from_dict
from machine_config.capabilities.v1_1.hdf5 import Hdf5AdapterV1_1
from machine_config.models import MachineConfig


@runtime_checkable
class ReaderAdapter(Protocol):
    """Structural contract every version-specific read adapter must satisfy."""
    def parse(self) -> MachineConfig: ...

__all__ = ["MachineConfigReader", "config_from_dict"]


_ADAPTERS = {
    "1.0": Hdf5AdapterV1_0,
    "1.1": Hdf5AdapterV1_1,
}


class MachineConfigReader:
    """Read a machine-config HDF5 file into a stable :class:`MachineConfig`.

    Peeks root ``File_Version`` first and routes to that version's adapter.
    """

    def __init__(self, path: str | Path) -> None:
        self.path = Path(path)
        version = peek_file_version(self.path)
        adapter_cls = _ADAPTERS.get(version)
        if adapter_cls is None:
            raise UnsupportedFileVersion(version)
        self._backend = adapter_cls(self.path)
        self.file_version = version

    def __getattr__(self, name: str):
        return getattr(self._backend, name)
