"""Stable File_Version model facade API.

Peek root File_Version, then dispatch to a version adapter. Adapters own
on-disk layout and map to stable models.
"""
from __future__ import annotations

from pathlib import Path

from machine_config.capabilities.errors import (
    CapabilityError,
    SessionClosedError,
    capability_error,
)
from machine_config.capabilities.file_version import (
    MissingRequiredGroup,
    UnsupportedFileVersion,
    peek_file_version,
)
from machine_config.capabilities.generated import SetMode
from machine_config.capabilities.merge import apply_set_mode, snapshot
from machine_config.capabilities.result import Result, err, ok
from machine_config.capabilities.v1_0 import MachineConfigFileV1_0

__all__ = [
    "CapabilityError",
    "MissingRequiredGroup",
    "SessionClosedError",
    "UnsupportedFileVersion",
    "capability_error",
    "Result",
    "ok",
    "err",
    "SetMode",
    "MachineConfigFileV1_0",
    "open_machine_config",
    "create_machine_config",
    "supported_file_versions",
    "apply_set_mode",
    "snapshot",
    "peek_file_version",
]


_OPEN = {
    "1.0": MachineConfigFileV1_0.open,
}

_CREATE = {
    "1.0": MachineConfigFileV1_0.create,
}


def open_machine_config(path: str | Path) -> Result[MachineConfigFileV1_0, CapabilityError]:
    try:
        version = peek_file_version(path)
    except Exception as e:  # noqa: BLE001
        return err(capability_error("IoError", str(e)))
    opener = _OPEN.get(version)
    if opener is None:
        return err(
            capability_error(
                "UnsupportedVersion",
                f'No capability adapter registered for File_Version "{version}"',
            )
        )
    return opener(path)


def create_machine_config(version: str = "1.0") -> Result[MachineConfigFileV1_0, CapabilityError]:
    fv = version.strip() or "1.0"
    creator = _CREATE.get(fv)
    if creator is None:
        return err(
            capability_error(
                "UnsupportedVersion",
                f'create() unsupported for File_Version "{fv}"',
            )
        )
    return creator(fv)


def supported_file_versions() -> list[str]:
    return list(_OPEN.keys())
