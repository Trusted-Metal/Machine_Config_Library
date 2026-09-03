"""Capability-API error taxonomy (schema/capabilities/errors.yaml)."""
from __future__ import annotations

from dataclasses import dataclass
from typing import Literal, Optional

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
    # Names every individual violation at once (e.g. every missing required
    # OPCUA field) rather than only the first one encountered. None for
    # validation failures with nothing more specific to list.
    details: Optional[list[str]] = None


def capability_error(
    code: CapabilityErrorCode, message: str, details: Optional[list[str]] = None
) -> CapabilityError:
    return CapabilityError(code=code, message=message, details=details)


class SessionClosedError(RuntimeError):
    """Programmer bug — session used after close()."""

    def __init__(self) -> None:
        super().__init__("MachineConfigFile session is closed")
