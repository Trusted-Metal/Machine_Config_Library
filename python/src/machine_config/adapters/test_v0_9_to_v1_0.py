# AUTO-GENERATED from schema/adapters/test_v0_9_to_v1_0.yaml
# DO NOT EDIT MANUALLY — regenerate with: python tools/generate_adapters.py
from __future__ import annotations
from .base import BaseAdapter
from . import register


@register
class V0_9_to_V1_0(BaseAdapter):
    from_version: str = "0.9"
    to_version:   str = "1.0"

    def adapt(self, config: dict) -> dict:
        # field_add: test_added_field (path: optical_trains[*])
        for train in config.get("optical_trains", []):
            train.setdefault("test_added_field", None)
        # field_rename: test_old_name -> test_new_name (path: optical_trains[*])
        for train in config.get("optical_trains", []):
            if "test_old_name" in train:
                train["test_new_name"] = train.pop("test_old_name")
        # field_remove: test_removed_field dropped (path: optical_trains[*])
        for train in config.get("optical_trains", []):
            train.pop("test_removed_field", None)
        # field_move: test_move_field (optical_trains[*] -> optical_trains[*].test_nested)
        for train in config.get("optical_trains", []):
            if "test_move_field" in train:
                train.setdefault("test_nested", {})["test_move_field"] = train.pop("test_move_field")
        # field_move_rename: test_move_rename_old -> test_nested.test_move_rename_new (optical_trains[*] -> optical_trains[*].test_nested)
        for train in config.get("optical_trains", []):
            if "test_move_rename_old" in train:
                train.setdefault("test_nested", {})["test_move_rename_new"] = train.pop("test_move_rename_old")
        return config
