"""Configuration-hash tests.

See machine_config/hash.py's module docstring for the design rationale:
this hash is for MCF's own cross-language self-consistency, not for matching
any external producer's own hashing scheme. A file MCF's own writer produced
is expected to validate; a file authored by anything else is expected not to
— that is the correct, permanent result, not a bug to fix later.
"""
from __future__ import annotations

import dataclasses
from pathlib import Path

import h5py
import pytest

from machine_config import MachineConfig, MachineConfigReader, MachineConfigWriter
from machine_config.builder import MockConfigBuilder
from machine_config.capabilities.v1_0.hdf5 import Hdf5AdapterV1_0
from machine_config.capabilities.v1_1.hdf5 import Hdf5AdapterV1_1
from machine_config.hash import compute_configuration_hash

_REPO_ROOT = Path(__file__).parent.parent.parent
_REFERENCE_V1_1 = _REPO_ROOT / "fixtures" / "reference_config_v1_1.h5"


@pytest.fixture()
def built_config() -> MachineConfig:
    return MockConfigBuilder(n_lasers=1, machine_name="HashTestMachine").build()


def test_writer_produces_a_real_non_passthrough_hash(built_config: MachineConfig, tmp_path) -> None:
    # The input config's placeholder hash must never survive to disk verbatim.
    original = built_config.meta.configuration_hash
    out = tmp_path / "out.h5"
    MachineConfigWriter(built_config).write(out)
    written = MachineConfigReader(out).parse()
    assert len(written.meta.configuration_hash) == 64
    assert written.meta.configuration_hash != original


def test_hash_changes_when_a_scalar_field_changes(built_config: MachineConfig) -> None:
    h1 = compute_configuration_hash(built_config)
    changed = dataclasses.replace(
        built_config,
        machine=dataclasses.replace(built_config.machine, gas_flow_direction="X-"),
    )
    h2 = compute_configuration_hash(changed)
    assert h1 != h2


def test_hash_stable_across_pure_roundtrip(built_config: MachineConfig, tmp_path) -> None:
    out1 = tmp_path / "one.h5"
    MachineConfigWriter(built_config).write(out1)
    config1 = MachineConfigReader(out1).parse()

    out2 = tmp_path / "two.h5"
    MachineConfigWriter(config1).write(out2)
    config2 = MachineConfigReader(out2).parse()

    assert config1.meta.configuration_hash == config2.meta.configuration_hash


def test_reader_sets_is_valid_true_for_self_written_file(built_config: MachineConfig, tmp_path) -> None:
    out = tmp_path / "out.h5"
    MachineConfigWriter(built_config).write(out)
    config = MachineConfigReader(out).parse()
    assert config.meta.is_valid is True


def test_reader_sets_is_valid_false_for_hand_tampered_hash_without_erroring(
    built_config: MachineConfig, tmp_path
) -> None:
    out = tmp_path / "out.h5"
    MachineConfigWriter(built_config).write(out)

    with h5py.File(out, "a") as f:
        f.attrs["Configuration_Hash"] = "0" * 64

    # Must still be fully readable — a hash mismatch is never an error.
    config = MachineConfigReader(out).parse()
    assert config.meta.is_valid is False
    assert config.meta.machine_name == built_config.meta.machine_name


def test_reference_config_reads_back_invalid_forever(reference_config: MachineConfig) -> None:
    """Permanent regression guard: fixtures/reference_config.h5 was authored
    externally (not by MCF's own writer), so MCF's recomputed hash will never
    match its stored value. This is expected and correct — proof there's no
    false claim of parity with an external producer's own hashing scheme —
    not something to "fix" by regenerating the fixture.
    """
    assert reference_config.meta.is_valid is False


def test_unset_unit_fields_round_trip_as_none_not_a_default(
    built_config: MachineConfig, tmp_path
) -> None:
    """Prerequisite fix (CONFIGURATION_HASH_PLAN.md): the writer must not
    silently materialize a default value (e.g. "mm") for a *_unit field left
    unset. One representative field per affected struct — BuildPlate,
    OpticalTrain (top-level), Scanner, LightSource, Collimator, ScannerCard —
    left None, written, and read back; each must still be None, not a
    materialized default. This is what makes a hash computed before writing
    agree with one computed after (see machine_config/hash.py) without any
    write->re-read->patch workaround.
    """
    train = built_config.optical_trains[0]
    unset_train = dataclasses.replace(
        train,
        major_axis_angle_unit=None,
        scanner=dataclasses.replace(train.scanner, working_distance_unit=None),
        light_source=dataclasses.replace(train.light_source, wavelength_unit=None),
        collimator=dataclasses.replace(train.collimator, focal_length_unit=None),
        scanner_card=dataclasses.replace(train.scanner_card, sample_period_unit=None),
    )
    config = dataclasses.replace(
        built_config,
        machine=dataclasses.replace(built_config.machine, build_plate=dataclasses.replace(
            built_config.machine.build_plate, corner_radius_unit=None
        )),
        optical_trains=[unset_train],
    )

    out = tmp_path / "out.h5"
    MachineConfigWriter(config).write(out)
    result = MachineConfigReader(out).parse()

    rt_train = result.optical_trains[0]
    assert result.machine.build_plate.corner_radius_unit is None
    assert rt_train.major_axis_angle_unit is None
    assert rt_train.scanner.working_distance_unit is None
    assert rt_train.light_source.wavelength_unit is None
    assert rt_train.collimator.focal_length_unit is None
    assert rt_train.scanner_card.sample_period_unit is None


def test_correction_grid_patch_after_write_does_not_invalidate_hash(tmp_path) -> None:
    """MockConfigBuilder.save() writes scalars via MachineConfigWriter, then
    patches ClearBox correction datasets directly via h5py afterward. Binary
    correction data is deliberately excluded from the configuration hash
    (see machine_config/hash.py), so that post-hoc patch must not flip
    is_valid to False.
    """
    out = tmp_path / "out.h5"
    MockConfigBuilder(n_lasers=1, machine_name="HashTestMachine").save(out)
    config = MachineConfigReader(out).parse()
    assert config.meta.is_valid is True


def test_v1_0_and_v1_1_dict_builders_agree_on_firmware_version(tmp_path) -> None:
    """Regression guard (found via a real cross-language configuration-hash
    mismatch, tools/cross_check.py Phase 5): hash.py always builds its dict
    via Hdf5AdapterV1_0, regardless of the config's actual file_version, on
    the documented assumption that v1_0's and v1_1's _config_to_dict()
    already produce identical output for identical MachineConfig content.
    That assumption was false for ClearBox.firmware_version (a v1.1-only
    on-disk attribute, but a real StableModel field regardless of version) —
    v1_0's _clearbox_to_dict silently omitted it entirely. Verified against
    fixtures/reference_config_v1_1.h5, which has firmware_version set.
    """
    if not _REFERENCE_V1_1.exists():
        pytest.skip(f"fixture not present: {_REFERENCE_V1_1}")

    config = MachineConfigReader(_REFERENCE_V1_1).parse()
    assert any(
        t.optional_components.clearbox is not None
        and t.optional_components.clearbox.firmware_version is not None
        for t in config.optical_trains
    ), "fixture must actually exercise firmware_version for this test to be meaningful"

    v1_0_builder = Hdf5AdapterV1_0(tmp_path)
    v1_1_builder = Hdf5AdapterV1_1(tmp_path)
    d0 = v1_0_builder._config_to_dict(config, include_binary=False)
    d1 = v1_1_builder._config_to_dict(config, include_binary=False)
    assert d0 == d1
