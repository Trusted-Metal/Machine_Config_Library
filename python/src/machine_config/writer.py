"""Public HDF5 writer: dispatch to a File_Version adapter.

On-disk group paths and HDF5 attribute names live in the matching adapter.
"""
from __future__ import annotations

from pathlib import Path

from machine_config.capabilities.file_version import UnsupportedFileVersion
from machine_config.capabilities.v1_0.writer import Hdf5WriterV1_0
from machine_config.models import MachineConfig


_ADAPTERS = {
    "1.0": Hdf5WriterV1_0,
}


class MachineConfigWriter:
    """Write a stable :class:`MachineConfig` using the model's File_Version adapter."""

    def __init__(self, config: MachineConfig) -> None:
        self.config = config
        version = (config.meta.file_version or "1.0").strip() or "1.0"
        adapter_cls = _ADAPTERS.get(version)
        if adapter_cls is None:
            raise UnsupportedFileVersion(version)
        self._backend = adapter_cls(config)
        self.file_version = version

    def write(self, path: str | Path) -> None:
        self._backend.write(path)
