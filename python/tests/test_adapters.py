# Phase C — synthetic fixture structure and golden JSON tests

import json
from pathlib import Path

import h5py
import pytest

from machine_config import MachineConfig
from machine_config.adapters import REGISTRY, get_chain
from machine_config.adapters.test_v0_9_to_v1_0 import V0_9_to_V1_0
from machine_config.reader import MachineConfigReader

_FIXTURES = (
    Path(__file__).resolve().parent.parent.parent / "fixtures" / "adapters" / "test"
)

FIXTURE = _FIXTURES / "reference_synthetic_v0_9.h5"
GOLDEN  = _FIXTURES / "expected_after_upgrade.json"


@pytest.fixture(scope="module")
def v09_file():
    with h5py.File(FIXTURE, "r") as f:
        yield f


def _optical_trains(f: h5py.File) -> list[h5py.Group]:
    parent = f["Machine/Optical_Trains"]
    return [parent[n] for n in parent if isinstance(parent[n], h5py.Group)]


class TestSyntheticV09Fixture:
    def test_fixture_exists(self):
        assert FIXTURE.exists()

    def test_file_version_is_0_9(self, v09_file):
        assert v09_file.attrs["File_Version"] == "0.9"

    def test_has_two_optical_trains(self, v09_file):
        assert len(_optical_trains(v09_file)) == 2

    @pytest.mark.parametrize("attr", [
        "test_old_name",
        "test_removed_field",
        "test_move_field",
        "test_move_rename_old",
    ])
    def test_synthetic_attr_present_on_all_trains(self, v09_file, attr):
        for grp in _optical_trains(v09_file):
            assert attr in grp.attrs, f"{grp.name} missing {attr!r}"

    def test_test_added_field_not_pre_seeded(self, v09_file):
        """The adapter must add this; it must not exist in the raw fixture."""
        for grp in _optical_trains(v09_file):
            assert "test_added_field" not in grp.attrs

    def test_test_nested_group_not_pre_created(self, v09_file):
        """The adapter creates test_nested; the fixture must not pre-create it."""
        ot = v09_file["Machine/Optical_Trains"]
        for name in ot:
            grp = ot[name]
            if isinstance(grp, h5py.Group):
                assert "test_nested" not in grp


# ---------------------------------------------------------------------------
# Phase C.2 — golden JSON ("expected after upgrade") tests
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def golden():
    return json.loads(GOLDEN.read_text(encoding="utf-8"))


class TestGoldenAfterUpgrade:
    def test_golden_exists(self):
        assert GOLDEN.exists()

    def test_file_version_upgraded(self, golden):
        assert golden["meta"]["File_Version"] == "1.0"

    def test_two_optical_trains(self, golden):
        assert len(golden["optical_trains"]) == 2

    @pytest.mark.parametrize("train_idx", [0, 1])
    def test_field_add_applied(self, golden, train_idx):
        train = golden["optical_trains"][train_idx]
        assert "test_added_field" in train
        assert train["test_added_field"] is None

    @pytest.mark.parametrize("train_idx", [0, 1])
    def test_field_rename_applied(self, golden, train_idx):
        train = golden["optical_trains"][train_idx]
        assert "test_new_name" in train
        assert "test_old_name" not in train

    @pytest.mark.parametrize("train_idx", [0, 1])
    def test_field_remove_applied(self, golden, train_idx):
        assert "test_removed_field" not in golden["optical_trains"][train_idx]

    @pytest.mark.parametrize("train_idx", [0, 1])
    def test_field_move_applied(self, golden, train_idx):
        train = golden["optical_trains"][train_idx]
        assert "test_move_field" not in train
        assert train.get("test_nested", {}).get("test_move_field") == "synthetic_move_value"

    @pytest.mark.parametrize("train_idx", [0, 1])
    def test_field_move_rename_applied(self, golden, train_idx):
        train = golden["optical_trains"][train_idx]
        assert "test_move_rename_old" not in train
        assert train.get("test_nested", {}).get("test_move_rename_new") == "synthetic_move_rename_value"


# ---------------------------------------------------------------------------
# Phase D.1 — reader dispatch tests
# ---------------------------------------------------------------------------

REAL_FIXTURE   = _FIXTURES.parent.parent / "reference_config.h5"


class TestDispatch:
    def test_v0_9_fixture_file_version_upgraded(self):
        cfg = MachineConfigReader(FIXTURE).parse()
        assert cfg.meta.file_version == "1.0"

    def test_real_file_file_version_unchanged(self):
        cfg = MachineConfigReader(REAL_FIXTURE).parse()
        assert cfg.meta.file_version == "1.0"

    def test_v0_9_fixture_returns_valid_machineconfig(self):
        cfg = MachineConfigReader(FIXTURE).parse()
        assert isinstance(cfg, MachineConfig)
        assert len(cfg.optical_trains) == 2

    def test_real_file_returns_valid_machineconfig(self):
        cfg = MachineConfigReader(REAL_FIXTURE).parse()
        assert isinstance(cfg, MachineConfig)
        assert len(cfg.optical_trains) == 2

    def test_v0_9_optical_train_data_intact(self):
        """Real beam data must survive the adapter pass unchanged."""
        cfg = MachineConfigReader(FIXTURE).parse()
        train = cfg.optical_trains[0]
        assert train.beam_waist_major is not None
        assert train.beam_waist_major > 0


# ---------------------------------------------------------------------------
# Phase E.1 — adapter unit tests (plain dicts, no HDF5)
# ---------------------------------------------------------------------------

def _config(*extra_keys: str) -> dict:
    train = {k: "val" for k in extra_keys}
    return {"optical_trains": [train]}


def test_field_add():
    cfg = _config()
    result = V0_9_to_V1_0().adapt(cfg)
    assert result["optical_trains"][0]["test_added_field"] is None


def test_field_rename():
    cfg = _config("test_old_name")
    result = V0_9_to_V1_0().adapt(cfg)
    train = result["optical_trains"][0]
    assert train["test_new_name"] == "val"
    assert "test_old_name" not in train


def test_field_remove():
    cfg = _config("test_removed_field")
    result = V0_9_to_V1_0().adapt(cfg)
    assert "test_removed_field" not in result["optical_trains"][0]


def test_field_move():
    cfg = _config("test_move_field")
    result = V0_9_to_V1_0().adapt(cfg)
    train = result["optical_trains"][0]
    assert "test_move_field" not in train
    assert train["test_nested"]["test_move_field"] == "val"


def test_field_move_rename():
    cfg = _config("test_move_rename_old")
    result = V0_9_to_V1_0().adapt(cfg)
    train = result["optical_trains"][0]
    assert "test_move_rename_old" not in train
    assert train["test_nested"]["test_move_rename_new"] == "val"


def test_adapter_metadata():
    assert V0_9_to_V1_0.from_version == "0.9"
    assert V0_9_to_V1_0.to_version == "1.0"


def test_adapter_in_registry():
    assert ("0.9", "1.0") in REGISTRY


def test_get_chain_returns_adapter():
    assert len(get_chain("0.9", "1.0")) > 0
    assert get_chain("1.0", "1.0") == []
