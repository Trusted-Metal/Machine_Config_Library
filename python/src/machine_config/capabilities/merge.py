"""Merge / Replace helpers for facade set* APIs."""
from __future__ import annotations

import copy
from typing import Any

from machine_config.capabilities.generated import SetMode


def snapshot(value: Any) -> Any:
    return copy.deepcopy(value)


def apply_set_mode(
    current: dict[str, Any],
    incoming: dict[str, Any],
    mode: SetMode = SetMode.MERGE,
) -> dict[str, Any]:
    if mode == SetMode.REPLACE:
        return snapshot(incoming)
    out = snapshot(current)
    for key, value in incoming.items():
        out[key] = value
    return out
