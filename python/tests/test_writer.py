"""Phase 1.6 — Writer roundtrip tests.

Strategy: parse the reference HDF5 file → write to a temp file → re-parse → compare.
"""
from __future__ import annotations

import json
from dataclasses import replace
from pathlib import Path

import pytest

from machine_config import MachineConfig, MachineConfigReader, MachineConfigWriter
from machine_config.builder import MockConfigBuilder
from machine_config.capabilities.file_version import UnsupportedFileVersion


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def roundtrip_config(
    reference_config: MachineConfig, tmp_path_factory: pytest.TempPathFactory
) -> MachineConfig:
    """Write the reference config to a temp file and re-parse it once for the module."""
    out = tmp_path_factory.mktemp("writer") / "roundtrip.h5"
    MachineConfigWriter(reference_config).write(out)
    return MachineConfigReader(out).parse()


# ---------------------------------------------------------------------------
# Roundtrip correctness tests
# ---------------------------------------------------------------------------

def test_writer_roundtrip_machine_name(roundtrip_config: MachineConfig, reference_config: MachineConfig) -> None:
    assert roundtrip_config.meta.machine_name == reference_config.meta.machine_name


def test_writer_roundtrip_configuration_hash(roundtrip_config: MachineConfig, reference_config: MachineConfig) -> None:
    assert roundtrip_config.meta.configuration_hash == reference_config.meta.configuration_hash


def test_writer_roundtrip_build_plate(roundtrip_config: MachineConfig, reference_config: MachineConfig) -> None:
    orig = reference_config.machine.build_plate
    rt   = roundtrip_config.machine.build_plate
    assert rt.x == pytest.approx(orig.x)
    assert rt.y == pytest.approx(orig.y)
    assert rt.x_unit == orig.x_unit


def test_writer_roundtrip_optical_train_count(roundtrip_config: MachineConfig, reference_config: MachineConfig) -> None:
    assert len(roundtrip_config.optical_trains) == len(reference_config.optical_trains)


def test_writer_roundtrip_scanner_offsets(roundtrip_config: MachineConfig, reference_config: MachineConfig) -> None:
    for i, (orig_train, rt_train) in enumerate(
        zip(reference_config.optical_trains, roundtrip_config.optical_trains)
    ):
        assert rt_train.scanner.scan_head_offset_x == pytest.approx(
            orig_train.scanner.scan_head_offset_x
        ), f"Train {i}: scan_head_offset_x mismatch"


def test_writer_roundtrip_thermal_lensing(roundtrip_config: MachineConfig, reference_config: MachineConfig) -> None:
    for i, (orig_train, rt_train) in enumerate(
        zip(reference_config.optical_trains, roundtrip_config.optical_trains)
    ):
        assert rt_train.thermal_lensing_passed == orig_train.thermal_lensing_passed, (
            f"Train {i}: thermal_lensing_passed mismatch"
        )


def test_writer_null_field_survives_roundtrip(tmp_path: Path) -> None:
    """A field that is None should survive write → read as None (not crash)."""
    config  = MockConfigBuilder(n_lasers=1, include_clearbox=False).build()
    out     = tmp_path / "null_field.h5"
    MachineConfigWriter(config).write(out)
    config2 = MachineConfigReader(out).parse()
    # power_min_nominal is None in the mock builder
    assert config2.optical_trains[0].light_source.power_min_nominal is None


def test_writer_rejects_unknown_file_version(reference_config: MachineConfig) -> None:
    """Dispatch must fail before any v1.0 layout is written."""
    bad = replace(
        reference_config,
        meta=replace(reference_config.meta, file_version="2.0"),
    )
    with pytest.raises(UnsupportedFileVersion, match="2.0"):
        MachineConfigWriter(bad)


def test_writer_produces_schema_valid_output(
    reference_config: MachineConfig, tmp_path: Path
) -> None:
    """The JSON export of a round-tripped config should validate against the schema."""
    import jsonschema
    from machine_config.schema import SCHEMA

    out = tmp_path / "schema_check.h5"
    MachineConfigWriter(reference_config).write(out)
    config2  = MachineConfigReader(out).parse()
    reader2  = MachineConfigReader(out)
    as_dict  = reader2._config_to_dict(config2)
    # Raises jsonschema.ValidationError on failure — no errors means valid
    jsonschema.validate(as_dict, SCHEMA)
