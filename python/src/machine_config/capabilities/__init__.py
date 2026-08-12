"""Stable File_Version model facade API."""
from __future__ import annotations

from pathlib import Path

from machine_config.capabilities.errors import (
    CapabilityError,
    SessionClosedError,
    capability_error,
)
from machine_config.capabilities.generated import SetMode
from machine_config.capabilities.merge import apply_set_mode, snapshot
from machine_config.capabilities.result import Result, err, ok
from machine_config.capabilities.v1_0 import MachineConfigFileV10
from machine_config.reader import MachineConfigReader

__all__ = [
    "CapabilityError",
    "SessionClosedError",
    "capability_error",
    "Result",
    "ok",
    "err",
    "SetMode",
    "MachineConfigFileV10",
    "open_machine_config",
    "create_machine_config",
    "supported_file_versions",
    "apply_set_mode",
    "snapshot",
]


def open_machine_config(path: str | Path) -> Result[MachineConfigFileV10, CapabilityError]:
    try:
        reader = MachineConfigReader(str(path))
        config = reader.parse()
        version = (config.meta.file_version or "1.0").strip() or "1.0"
        if version == "1.0":
            return MachineConfigFileV10.open(path)
        return err(
            capability_error(
                "UnsupportedVersion",
                f'No capability adapter registered for File_Version "{version}"',
            )
        )
    except Exception as e:  # noqa: BLE001
        return err(capability_error("IoError", str(e)))


def create_machine_config(version: str = "1.0") -> Result[MachineConfigFileV10, CapabilityError]:
    return MachineConfigFileV10.create(version)


def supported_file_versions() -> list[str]:
    return ["1.0"]
