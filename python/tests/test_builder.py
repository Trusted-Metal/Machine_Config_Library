"""Phase 1.6 — Builder tests: MockConfigBuilder, YamlConfigBuilder, ConfigEditor."""
from __future__ import annotations

from pathlib import Path

import h5py
import numpy as np
import pytest
import yaml

from machine_config import MachineConfigReader
from machine_config.builder import MockConfigBuilder, YamlConfigBuilder, ConfigEditor


# ---------------------------------------------------------------------------
# MockConfigBuilder
# ---------------------------------------------------------------------------

def test_mock_builder_single_laser(tmp_path: Path) -> None:
    out = tmp_path / "single.h5"
    MockConfigBuilder(n_lasers=1).save(out)
    config = MachineConfigReader(out).parse()
    assert len(config.optical_trains) == 1


def test_mock_builder_two_lasers(tmp_path: Path) -> None:
    out = tmp_path / "double.h5"
    MockConfigBuilder(n_lasers=2).save(out)
    config = MachineConfigReader(out).parse()
    assert len(config.optical_trains) == 2


def test_mock_builder_build_plate_dimensions(tmp_path: Path) -> None:
    out = tmp_path / "plate.h5"
    MockConfigBuilder(build_plate_x=300.0, build_plate_y=280.0).save(out)
    config = MachineConfigReader(out).parse()
    assert config.machine.build_plate.x == pytest.approx(300.0)
    assert config.machine.build_plate.y == pytest.approx(280.0)


def test_mock_builder_correction_grid_shape(tmp_path: Path) -> None:
    out = tmp_path / "grid.h5"
    MockConfigBuilder(n_lasers=1, include_clearbox=True).save(out)
    with h5py.File(out, "r") as f:
        ds = f["Machine/Optical_Trains/Optical_Train_01/Optional_Components/ClearBox/Correction_Data"]
        assert ds.shape == (257, 257, 2)


def test_mock_builder_correction_grid_is_nonzero(tmp_path: Path) -> None:
    out = tmp_path / "nonzero.h5"
    MockConfigBuilder(n_lasers=1, include_clearbox=True).save(out)
    with h5py.File(out, "r") as f:
        data = f["Machine/Optical_Trains/Optical_Train_01/Optional_Components/ClearBox/Correction_Data"][:]
    assert np.any(data != 0.0), "Correction_Data should be non-zero (Gaussian warp)"


# ---------------------------------------------------------------------------
# YamlConfigBuilder
# ---------------------------------------------------------------------------

def test_yaml_builder_roundtrip(tmp_path: Path) -> None:
    spec = {
        "machine": {
            "name": "YAML-Test-01",
            "manufacturer": "TestCo",
            "model": "TestMIDI",
            "build_plate_x": 250,
            "build_plate_y": 280,
            "gas_flow_direction": "Y+",
            "recoat_direction": "X+",
            "recoater_blade_type": "Standard",
        },
        "optical_trains": [
            {
                "scanner": {
                    "working_distance": 670,
                    "scan_head_offset_x": -87.5,
                    "scan_head_offset_y": 10.0,
                },
                "light_source": {
                    "power_max_nominal": 1000,
                    "wavelength": 1070,
                },
            }
        ],
    }
    spec_path = tmp_path / "test.yaml"
    spec_path.write_text(yaml.dump(spec), encoding="utf-8")
    out = tmp_path / "yaml_out.h5"
    YamlConfigBuilder(spec_path).save(out)

    config = MachineConfigReader(out).parse()
    assert config.meta.machine_name == "YAML-Test-01"
    assert config.machine.build_plate.x == pytest.approx(250.0)
    assert config.machine.build_plate.y == pytest.approx(280.0)
    assert len(config.optical_trains) == 1
    assert config.optical_trains[0].scanner.working_distance == pytest.approx(670.0)
    assert config.optical_trains[0].scanner.scan_head_offset_x == pytest.approx(-87.5)
    assert config.optical_trains[0].light_source.power_max_nominal == pytest.approx(1000.0)
    assert config.machine.recoater_blade_type == "Standard"


# ---------------------------------------------------------------------------
# ConfigEditor
# ---------------------------------------------------------------------------

def test_config_editor_modifies_offset(tmp_path: Path) -> None:
    src = tmp_path / "source.h5"
    dst = tmp_path / "edited.h5"
    MockConfigBuilder(n_lasers=2).save(src)

    editor = ConfigEditor(src)
    editor.set_scanner_offset(train_index=0, x=-99.9, y=5.5)
    editor.save(dst)

    config = MachineConfigReader(dst).parse()
    assert config.optical_trains[0].scanner.scan_head_offset_x == pytest.approx(-99.9)
    assert config.optical_trains[0].scanner.scan_head_offset_y == pytest.approx(5.5)
    # Train 1 should be unchanged
    orig = MachineConfigReader(src).parse()
    assert config.optical_trains[1].scanner.scan_head_offset_x == pytest.approx(
        orig.optical_trains[1].scanner.scan_head_offset_x
    )
