"""Capability Result type — errors are part of the return signature."""
from __future__ import annotations

from dataclasses import dataclass
from typing import Generic, TypeVar, Union

T = TypeVar("T")
E = TypeVar("E")


@dataclass(frozen=True)
class Ok(Generic[T]):
    value: T
    ok: bool = True


@dataclass(frozen=True)
class Err(Generic[E]):
    error: E
    ok: bool = False


Result = Union[Ok[T], Err[E]]


def ok(value: T) -> Ok[T]:
    return Ok(value)


def err(error: E) -> Err[E]:
    return Err(error)
