"""Capability-API error taxonomy (schema/capabilities/errors.yaml)."""
from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

CapabilityErrorCode = Literal[
    "UnsupportedVersion",
    "UnsupportedInVersion",
    "NotPresent",
    "ValidationError",
    "IoError",
    "InvalidIndex",
    "Closed",
]


@dataclass(frozen=True)
class CapabilityError:
    code: CapabilityErrorCode
    message: str


def capability_error(code: CapabilityErrorCode, message: str) -> CapabilityError:
    return CapabilityError(code=code, message=message)


class SessionClosedError(RuntimeError):
    """Programmer bug — session used after close()."""

    def __init__(self) -> None:
        super().__init__("MachineConfigFile session is closed")
