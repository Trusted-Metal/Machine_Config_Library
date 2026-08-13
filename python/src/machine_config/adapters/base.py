from __future__ import annotations
from abc import ABC, abstractmethod


class BaseAdapter(ABC):
    from_version: str
    to_version: str

    @abstractmethod
    def adapt(self, config: dict) -> dict: ...
