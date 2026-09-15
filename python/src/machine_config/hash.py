"""SHA-256 configuration-integrity hash for :class:`~machine_config.models.MachineConfig`.

Computed over the canonical, ``include_binary=False`` JSON shape of the whole
config (the same shape ``to_json()``/``export-json`` produce), minus
``meta.configuration_hash`` (can't hash itself) and ``meta.export_date``
(changes on every re-export even when nothing configuration-wise changed).
Object keys are recursively sorted so the result doesn't depend on this
language's particular field-declaration order.

This hash is computed for MCF's own self-consistency across MCF's own five
language implementations — it is NOT designed to match any other producer's
own hashing scheme (e.g. an external system that may hash data which never
survives to the on-disk file at all). A file MCF's own writer produced will
read back valid; a file authored by anything else will very likely read back
invalid — that's expected, not a bug: it answers "was this MCF-touched file
tampered with since MCF itself last touched it," not "does this match some
other system's proprietary algorithm."
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

from machine_config.capabilities.v1_0.hdf5 import Hdf5AdapterV1_0
from machine_config.models import MachineConfig

# _config_to_dict() is a pure data transformation (never touches self.path or
# performs I/O) — instantiate with a placeholder path to reuse ~300 lines of
# existing, tested dict-building logic rather than duplicating it. Always use
# the v1_0 adapter's implementation regardless of which version the config
# was parsed from: the StableModel's dict shape is version-agnostic by design
# (verified by tools/cross_check.py's read-parity phase), so v1_0's and
# v1_1's _config_to_dict() must already produce identical output for
# identical MachineConfig content.
_DICT_BUILDER = Hdf5AdapterV1_0(Path("."))


def compute_configuration_hash(config: MachineConfig) -> str:
    """Return the SHA-256 hex digest of *config*'s canonical, hash-relevant content.

    Excludes ``meta.configuration_hash`` and ``meta.export_date``; excludes
    binary correction-grid data (``correction_data``/``inverse_correction_data``/
    ``raw_bytes``) since a separate, dedicated mechanism (``correction-hash``)
    already exists for binary-grid integrity. Recursively sorts all object
    keys; arrays keep their existing element order.
    """
    d = _DICT_BUILDER._config_to_dict(config, include_binary=False)
    d["meta"].pop("configuration_hash", None)
    d["meta"].pop("export_date", None)
    canonical = json.dumps(
        d, sort_keys=True, ensure_ascii=False, default=str, separators=(",", ":")
    )
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()
