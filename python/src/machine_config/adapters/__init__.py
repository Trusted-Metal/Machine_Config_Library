from __future__ import annotations
from .base import BaseAdapter

REGISTRY: dict[tuple[str, str], type[BaseAdapter]] = {}


def register(cls: type[BaseAdapter]) -> type[BaseAdapter]:
    REGISTRY[(cls.from_version, cls.to_version)] = cls
    return cls


# Import generated adapters after REGISTRY is defined so @register fires into a live dict.
# Add each new generated adapter here as it is created.
from .test_v0_9_to_v1_0 import V0_9_to_V1_0  # noqa: F401, E402


def get_chain(from_version: str, to_version: str) -> list[BaseAdapter]:
    # TODO: build a graph walk when multiple hops are needed
    if from_version == to_version:
        return []
    cls = REGISTRY.get((from_version, to_version))
    return [cls()] if cls is not None else []


__all__ = ["BaseAdapter", "REGISTRY", "register", "get_chain"]
