"""Generate synthetic HDF5 fixtures used by adapter migration tests.

Run from the repo root:
    python tools/generate_adapter_fixtures.py
"""

from __future__ import annotations

import json
import shutil
from pathlib import Path

import h5py

from machine_config.reader import _hdf5_to_raw_dict

REPO_ROOT = Path(__file__).resolve().parent.parent
SOURCE     = REPO_ROOT / "fixtures" / "reference_config.h5"
DEST_DIR   = REPO_ROOT / "fixtures" / "adapters" / "test"

# Synthetic field names that must be present in v0.9 before the adapter runs.
# field_add  ("test_added_field")    — adapter adds it; must NOT be pre-seeded.
# field_rename old side              — must be present so the rename has something to act on.
# field_remove                       — must be present so the remove has something to delete.
# field_move  source side            — must be present so the move has something to relocate.
# field_move_rename old/source side  — must be present so the move+rename has something to act on.
_SYNTHETIC_ATTRS: dict[str, str] = {
    "test_old_name":       "synthetic_rename_value",
    "test_removed_field":  "synthetic_remove_value",
    "test_move_field":     "synthetic_move_value",
    "test_move_rename_old": "synthetic_move_rename_value",
}


def _optical_train_groups(f: h5py.File) -> list[h5py.Group]:
    ot_parent = f.get("Machine/Optical_Trains")
    if ot_parent is None:
        return []
    return [ot_parent[name] for name in ot_parent if isinstance(ot_parent[name], h5py.Group)]


def generate_v0_9(dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(SOURCE, dest)

    with h5py.File(dest, "a") as f:
        f.attrs["File_Version"] = "0.9"
        for grp in _optical_train_groups(f):
            for attr, value in _SYNTHETIC_ATTRS.items():
                grp.attrs[attr] = value

    print(f"wrote {dest.relative_to(REPO_ROOT)}")


def generate_golden_json(src: Path, dest: Path) -> None:
    from machine_config.adapters.test_v0_9_to_v1_0 import V0_9_to_V1_0

    with h5py.File(src, "r") as f:
        raw = _hdf5_to_raw_dict(f)
    adapted = V0_9_to_V1_0().adapt(raw)
    # reflect the File_Version upgrade the dispatch layer applies in Phase D
    adapted["meta"]["File_Version"] = "1.0"

    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(json.dumps(adapted, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"wrote {dest.relative_to(REPO_ROOT)}")


if __name__ == "__main__":
    v0_9 = DEST_DIR / "reference_synthetic_v0_9.h5"
    generate_v0_9(v0_9)
    generate_golden_json(v0_9, DEST_DIR / "expected_after_upgrade.json")
