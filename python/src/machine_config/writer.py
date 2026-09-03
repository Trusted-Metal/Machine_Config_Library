"""Public HDF5 writer: dispatch to a File_Version adapter.

On-disk group paths and HDF5 attribute names live in the matching adapter.
"""
from __future__ import annotations

import dataclasses
from pathlib import Path
from typing import Protocol, runtime_checkable

from machine_config.capabilities.file_version import UnsupportedFileVersion
from machine_config.capabilities.v1_0.writer import Hdf5WriterV1_0
from machine_config.capabilities.v1_1.writer import Hdf5WriterV1_1
from machine_config.models import MachineConfig


@runtime_checkable
class WriterAdapter(Protocol):
    """Structural contract every version-specific write adapter must satisfy."""
    def write(self, path: str | Path) -> None: ...


_ADAPTERS = {
    "1.0": Hdf5WriterV1_0,
    "1.1": Hdf5WriterV1_1,
}


class MachineConfigWriter:
    """Write a stable :class:`MachineConfig` using a File_Version adapter.

    The adapter is chosen by *target_version* if given, else by
    ``config.meta.file_version`` (today's default behavior, preserved).
    *target_version* lets a caller upgrade/downgrade without mutating the
    model just to express intent — e.g. reading a v1.0 file and writing it
    as v1.1 no longer requires ``config.meta.file_version = "1.1"`` first.
    Never mutates *config* itself; only the on-disk File_Version changes.
    """

    def __init__(self, config: MachineConfig, target_version: str | None = None) -> None:
        self.config = config
        version = (target_version or config.meta.file_version or "1.0").strip() or "1.0"
        adapter_cls = _ADAPTERS.get(version)
        if adapter_cls is None:
            raise UnsupportedFileVersion(version)
        # Each adapter stamps config.meta.file_version verbatim as the
        # on-disk File_Version attribute — if target_version overrides the
        # adapter choice, the config handed to the adapter must reflect that
        # too, or the file would claim the wrong version on disk. A copy,
        # not a mutation: self.config (and the caller's original object)
        # stay untouched either way.
        resolved_config = (
            config
            if version == (config.meta.file_version or "1.0").strip()
            else dataclasses.replace(
                config, meta=dataclasses.replace(config.meta, file_version=version)
            )
        )
        self._backend = adapter_cls(resolved_config)
        self.file_version = version

    def write(self, path: str | Path) -> None:
        self._backend.write(path)
