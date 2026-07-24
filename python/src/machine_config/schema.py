"""Loads and exposes the canonical machine_config_v1 JSON Schema.

Usage:
    from machine_config.schema import SCHEMA
    jsonschema.validate(output, SCHEMA)
"""
from __future__ import annotations

import json
from pathlib import Path

# schema/ lives four directories above this file:
#   python/src/machine_config/schema.py  →  ../../../..  →  repo root
_SCHEMA_PATH = (
    Path(__file__).parent.parent.parent.parent
    / "schema"
    / "machine_config_v1.schema.json"
)

SCHEMA: dict = json.loads(_SCHEMA_PATH.read_text(encoding="utf-8"))

# The schema version this library produces. Stamped into every JSON export.
# Bump this (and create a new schema file) only for breaking output changes.
SCHEMA_VERSION: str = "v1"
