"""File_Version 1.1 adapter tests — the real v1.1 (not the mock in
test_adapter_migration.py). Follows docs/contributing.md's 7-test template,
made concrete for docs/migrations/v1_0_to_v1_1.md's 5 changes, plus the extra
hard-error tests V1_1_IMPLEMENTATION_PLAN.md's Testing section calls for.

Phase 2 (V1_1_IMPLEMENTATION_PLAN.md) removed migrate_v1_to_v1_1/
migrate_v1_1_to_v1 — upgrade/downgrade is now just parse()/write(), using
MachineConfigWriter's target_version parameter. Every test below that used
to call a migrate function directly now goes through a real HDF5 write+read
instead — a genuine strengthening, since it now exercises the same code
path a real caller uses.

Tests
-----
  test_v1_1_read                                    natively-authored v1.1 fixture -> StableModel, all 5 changes asserted
  test_v1_1_roundtrip                                write -> read -> write -> read, cross-wiring guard
  test_v1_to_v1_1                                    real v1.0 fixture written as v1.1, values asserted
  test_v1_to_v1_1_output_path_disagreement_raises    Change 1 Consolidate hard error
  test_v1_to_v1_1_software_trigger_delay_disagreement_raises
  test_v1_to_v1_1_unrecognized_algorithm_type_best_effort   Change 3/4: never raises, best-effort
  test_v1_to_v1_1_unrecognized_algorithm_type_best_effort_for_light_source
  test_v1_1_to_v1                                    real v1.1 fixture written as v1.0, values asserted
  test_v1_1_to_v1_reorders_constants_by_name         Change 3 backward: row-order hazard
  test_v1_1_to_v1_missing_derivation_constants_writes_blank
  test_v1_1_to_v1_missing_characterization_points_writes_blank
  test_v1_unaffected                                 v1.0 path undisturbed
  test_roundtrip_v1_0_to_v1_1_to_v1_0                the acceptance criterion, made concrete
  test_roundtrip_v1_1_to_v1_0_to_v1_1                mirror direction
  test_writer_target_version_overrides_meta
  test_writer_defaults_to_meta_file_version
  test_dispatcher_v1_to_v1_1 / test_dispatcher_v1_1_to_v1   full public API, real registry (no monkeypatching)
  test_adapters_satisfy_protocol
"""
from __future__ import annotations

from dataclasses import replace as dc_replace
from pathlib import Path

import pytest

from machine_config import (
    CalibrationPoint,
    EquationConstant,
    MachineConfigReader,
    MachineConfigWriter,
    OptionalComponents,
)
from machine_config.reader import ReaderAdapter
from machine_config.writer import WriterAdapter
from machine_config.builder import MockConfigBuilder
from machine_config.capabilities.v1_1 import Hdf5AdapterV1_1, Hdf5WriterV1_1
from machine_config.power_characterization import (
    forward_power_characterization_coefficients,
    forward_power_characterization_points,
)

_REPO_ROOT = Path(__file__).parent.parent.parent
_REFERENCE_V1_0 = _REPO_ROOT / "fixtures" / "reference_config_opcua_synchronous_sensors.h5"
_REFERENCE_V1_1 = _REPO_ROOT / "fixtures" / "reference_config_v1_1.h5"


def _mock_v1_0_config(n_lasers: int = 2):
    return MockConfigBuilder(n_lasers=n_lasers).build()


def _mock_v1_1_config(n_lasers: int = 2):
    """In-memory v1.1-shaped MachineConfig for tests that need one without
    touching disk. `MockConfigBuilder` already builds `power_characterization`
    natively now (POWER_CHARACTERIZATION_UNIFICATION_PLAN.md) — there's no
    separate flat-field shape left to convert away from, so this is just the
    v1.0 mock with `meta.file_version` overridden.
    """
    cfg = _mock_v1_0_config(n_lasers)
    return dc_replace(cfg, meta=dc_replace(cfg.meta, file_version="1.1"))


# ---------------------------------------------------------------------------
# test_v1_1_read
# ---------------------------------------------------------------------------

def test_v1_1_read():
    """Natively-authored v1.1 fixture -> StableModel — all 5 changes asserted by value."""
    cfg = MachineConfigReader(_REFERENCE_V1_1).parse()
    assert cfg.meta.file_version == "1.1"

    for t in cfg.optical_trains:
        cb = t.optional_components.clearbox
        # Change 1: Consolidate — same shared value on every train
        assert cb.output_path == "/recordings/"
        assert cb.software_trigger_delay == 3000
        # Change 1: Addition
        assert cb.firmware_version == "2.4.1"
        # Change 1: Removal
        assert cb.selected_camera is None
        assert cb.custom_video_format is None
        assert cb.video_output is None
        assert cb.show_console is None
        assert cb.correction_grid_domain_shape is None
        assert cb.inverse_grid_domain_shape is None

        # Change 5: no on-disk source in v1.1, always None
        assert t.scanner.x_axis.tuning_parameters is None
        assert t.scanner.x_axis.tuning_type is None
        assert t.scanner.y_axis.tuning_parameters is None
        assert t.scanner.y_axis.tuning_type is None

    # Change 2: OPCUA relocated, contents unaffected
    assert cfg.opcua is not None
    assert cfg.opcua.client.machine_profile is not None

    # Change 3: ClearBox Power_Characterization — train 1 LINEAR, train 2 POLYNOMIAL
    cb0 = cfg.optical_trains[0].optional_components.clearbox
    pc0 = cb0.power_characterization
    assert pc0.algorithm_type == "LINEAR"
    assert pc0.algorithm_equation == "W = a*V + b"
    assert pc0.input_type == "0-10 V"
    assert pc0.units_derived_quantity == "Watts"
    assert [c.name for c in pc0.derivation_equation_constants] == ["b", "a"]
    assert len(pc0.characterization_points) == 3

    cb1 = cfg.optical_trains[1].optional_components.clearbox
    pc1 = cb1.power_characterization
    assert pc1.algorithm_type == "POLYNOMIAL"
    assert pc1.algorithm_equation == "W = c0 + c1*V + c2*V^2"
    assert [c.name for c in pc1.derivation_equation_constants] == ["c0", "c1", "c2"]

    # Change 4: LightSource Power_Characterization — inverse data availability
    lspc0 = cfg.optical_trains[0].light_source.power_characterization
    assert lspc0.algorithm_type == "LINEAR"
    assert lspc0.input_type == "Volts"
    assert len(lspc0.characterization_points) == 5
    assert lspc0.derivation_equation_constants  # populated in this review fixture


# ---------------------------------------------------------------------------
# test_v1_1_roundtrip
# ---------------------------------------------------------------------------

def test_v1_1_roundtrip(tmp_path):
    """write -> read -> write -> read, no drift. ClearBox and Light_Source
    power_characterization deliberately given different values in the same
    object — direct test for the cross-wiring risk (same struct, two HDF5
    paths per train; a swapped read/write assignment would only be caught
    if the two instances are distinguishable).
    """
    cfg = _mock_v1_1_config(n_lasers=1)
    train = cfg.optical_trains[0]
    cb = train.optional_components.clearbox
    cb = dc_replace(
        cb,
        power_characterization=dc_replace(
            cb.power_characterization,
            algorithm_type="LINEAR",
            derivation_equation_constants=[
                EquationConstant(name="b", value=1.0),
                EquationConstant(name="a", value=2.0),
            ],
            characterization_points=[],
        ),
    )
    ls = dc_replace(
        train.light_source,
        power_characterization=dc_replace(
            train.light_source.power_characterization,
            algorithm_type="POLYNOMIAL",
            derivation_equation_constants=[],
            characterization_points=[CalibrationPoint(input_value=9.0, output_value=99.0)],
        ),
    )
    train = dc_replace(
        train, light_source=ls, optional_components=OptionalComponents(clearbox=cb)
    )
    cfg = dc_replace(cfg, optical_trains=[train])

    p1 = tmp_path / "v1_1_a.h5"
    MachineConfigWriter(cfg).write(p1)
    mid = MachineConfigReader(p1).parse()
    p2 = tmp_path / "v1_1_b.h5"
    MachineConfigWriter(mid).write(p2)
    result = MachineConfigReader(p2).parse()

    cb_r = result.optical_trains[0].optional_components.clearbox
    ls_r = result.optical_trains[0].light_source
    assert cb_r.power_characterization.algorithm_type == "LINEAR"
    assert [c.name for c in cb_r.power_characterization.derivation_equation_constants] == ["b", "a"]
    assert cb_r.power_characterization.characterization_points == []
    assert ls_r.power_characterization.algorithm_type == "POLYNOMIAL"
    assert ls_r.power_characterization.derivation_equation_constants == []
    assert ls_r.power_characterization.characterization_points[0].input_value == 9.0
    # Cross-wiring guard: the two instances must not have swapped.
    assert cb_r.power_characterization.algorithm_type != ls_r.power_characterization.algorithm_type


# ---------------------------------------------------------------------------
# test_v1_to_v1_1 (v1.0 fixture written as v1.1)
# ---------------------------------------------------------------------------

def test_v1_to_v1_1(tmp_path):
    """Real v1.0 fixture, written as v1.1 via target_version — asserts the
    manifest's actual rules. No migrate function involved: Hdf5WriterV1_1's
    own fallback (machine_config.power_characterization) derives
    power_characterization from the v1.0-sourced flat fields.
    """
    source = MachineConfigReader(_REFERENCE_V1_0).parse()
    out = tmp_path / "v1_1_from_v1_0.h5"
    MachineConfigWriter(source, target_version="1.1").write(out)
    migrated = MachineConfigReader(out).parse()
    assert migrated.meta.file_version == "1.1"

    for i, t in enumerate(migrated.optical_trains):
        src_cb = source.optical_trains[i].optional_components.clearbox
        cb = t.optional_components.clearbox
        # Consolidate: same per-train value carried straight through (real
        # fixture already agrees, confirmed in docs/migrations/v1_0_to_v1_1.md).
        assert cb.output_path == src_cb.output_path
        assert cb.software_trigger_delay == src_cb.software_trigger_delay
        # Addition: no v1.0 source.
        assert cb.firmware_version is None
        # Removal.
        assert cb.selected_camera is None
        assert cb.custom_video_format is None
        # Change 3: derived ClearBox data has real constants, zero points.
        assert cb.power_characterization.characterization_points == []
        assert cb.power_characterization.derivation_equation_constants
        assert cb.power_characterization.algorithm_type == src_cb.power_characterization.algorithm_type

        ls = t.light_source
        # Change 4: derived Light_Source data is the inverse — zero
        # constants, real points.
        assert ls.power_characterization.derivation_equation_constants == []
        assert ls.power_characterization.characterization_points


def test_v1_to_v1_1_output_path_disagreement_raises(tmp_path):
    cfg = _mock_v1_0_config(n_lasers=2)
    t1 = cfg.optical_trains[1]
    cb1 = dc_replace(t1.optional_components.clearbox, output_path="/other/")
    cfg = dc_replace(
        cfg,
        optical_trains=[
            cfg.optical_trains[0],
            dc_replace(t1, optional_components=OptionalComponents(clearbox=cb1)),
        ],
    )
    with pytest.raises(ValueError, match=r"Consolidate conflict on 'Output_Path'"):
        MachineConfigWriter(cfg, target_version="1.1").write(tmp_path / "x.h5")


def test_v1_to_v1_1_software_trigger_delay_disagreement_raises(tmp_path):
    cfg = _mock_v1_0_config(n_lasers=2)
    t1 = cfg.optical_trains[1]
    cb1 = dc_replace(t1.optional_components.clearbox, software_trigger_delay=9999)
    cfg = dc_replace(
        cfg,
        optical_trains=[
            cfg.optical_trains[0],
            dc_replace(t1, optional_components=OptionalComponents(clearbox=cb1)),
        ],
    )
    with pytest.raises(ValueError, match=r"Consolidate conflict on 'Software_Trigger_Delay'"):
        MachineConfigWriter(cfg, target_version="1.1").write(tmp_path / "x.h5")


def test_v1_to_v1_1_unrecognized_algorithm_type_best_effort(tmp_path):
    """Phase 2: never a hard error — best-effort, verbatim algorithm_type,
    positionally-named constants, blank algorithm_equation. Confirmed via a
    real write, not just a direct function call, since the derivation now
    lives in Hdf5WriterV1_1's fallback.
    """
    cfg = _mock_v1_0_config(n_lasers=1)
    t0 = cfg.optical_trains[0]
    cb0 = dc_replace(
        t0.optional_components.clearbox,
        power_characterization=forward_power_characterization_coefficients(
            "EXPONENTIAL", "1.5,2.5,3.5"
        ),
    )
    cfg = dc_replace(
        cfg, optical_trains=[dc_replace(t0, optional_components=OptionalComponents(clearbox=cb0))]
    )
    out = tmp_path / "v1_1_unrecognized.h5"
    MachineConfigWriter(cfg, target_version="1.1").write(out)
    result = MachineConfigReader(out).parse()
    pc = result.optical_trains[0].optional_components.clearbox.power_characterization
    assert pc.algorithm_type == "EXPONENTIAL"
    assert pc.algorithm_equation is None
    assert [c.name for c in pc.derivation_equation_constants] == ["0", "1", "2"]
    assert [c.value for c in pc.derivation_equation_constants] == [1.5, 2.5, 3.5]

    # Round trip: writing back to v1.0 and re-reading reproduces the same
    # structure — v1.0's reader now forward-derives power_characterization
    # from whatever backward_flat_fields_coefficients wrote, so there's no
    # more raw string field to compare directly.
    v1_0_out = tmp_path / "v1_0_roundtrip.h5"
    MachineConfigWriter(result, target_version="1.0").write(v1_0_out)
    back = MachineConfigReader(v1_0_out).parse()
    back_pc = back.optical_trains[0].optional_components.clearbox.power_characterization
    assert back_pc.algorithm_type == "EXPONENTIAL"
    assert [c.value for c in back_pc.derivation_equation_constants] == [1.5, 2.5, 3.5]


def test_v1_to_v1_1_unrecognized_algorithm_type_best_effort_for_light_source(tmp_path):
    cfg = _mock_v1_0_config(n_lasers=1)
    t0 = cfg.optical_trains[0]
    ls0 = dc_replace(
        t0.light_source,
        power_characterization=forward_power_characterization_points("QUADRATIC", "1,2,3,4"),
    )
    cfg = dc_replace(cfg, optical_trains=[dc_replace(t0, light_source=ls0)])
    out = tmp_path / "v1_1_unrecognized_ls.h5"
    MachineConfigWriter(cfg, target_version="1.1").write(out)
    result = MachineConfigReader(out).parse()
    pc = result.optical_trains[0].light_source.power_characterization
    assert pc.algorithm_type == "QUADRATIC"
    assert pc.algorithm_equation is None
    assert len(pc.characterization_points) == 2


# ---------------------------------------------------------------------------
# test_v1_1_to_v1 (v1.1 fixture written as v1.0)
# ---------------------------------------------------------------------------

def test_v1_1_to_v1(tmp_path):
    """Real, natively-authored v1.1 fixture, written as v1.0 via
    target_version — surviving fields preserved; re-derivation correct.
    Hdf5WriterV1_0's own fallback derives the flat fields from the
    genuinely-native power_characterization.
    """
    source = MachineConfigReader(_REFERENCE_V1_1).parse()
    out = tmp_path / "v1_0_from_v1_1.h5"
    MachineConfigWriter(source, target_version="1.0").write(out)
    back = MachineConfigReader(out).parse()
    assert back.meta.file_version == "1.0"

    src_cb0 = source.optical_trains[0].optional_components.clearbox
    cb0 = back.optical_trains[0].optional_components.clearbox
    assert cb0.power_characterization.algorithm_type == src_cb0.power_characterization.algorithm_type
    expected = sorted(c.value for c in src_cb0.power_characterization.derivation_equation_constants)
    actual = sorted(c.value for c in cb0.power_characterization.derivation_equation_constants)
    assert actual == expected
    assert cb0.output_path == src_cb0.output_path
    assert cb0.software_trigger_delay == src_cb0.software_trigger_delay

    src_ls0 = source.optical_trains[0].light_source
    ls0 = back.optical_trains[0].light_source
    assert ls0.power_characterization.algorithm_type == src_ls0.power_characterization.algorithm_type
    expected_points = [
        (p.input_value, p.output_value) for p in src_ls0.power_characterization.characterization_points
    ]
    actual_points = [
        (p.input_value, p.output_value) for p in ls0.power_characterization.characterization_points
    ]
    assert actual_points == expected_points

    # Change 1 Removal fields: lost forever, not restored.
    assert cb0.selected_camera is None


def test_v1_1_to_v1_reorders_constants_by_name(tmp_path):
    """Change 3 backward: Derivation_Equation_Constants rows aren't
    positionally guaranteed — must re-sort by name before joining as CSV.
    """
    cfg = _mock_v1_1_config(n_lasers=1)
    t0 = cfg.optical_trains[0]
    cb0 = t0.optional_components.clearbox
    scrambled = [EquationConstant(name="a", value=2.0), EquationConstant(name="b", value=1.0)]
    cb0 = dc_replace(
        cb0,
        power_characterization=dc_replace(
            cb0.power_characterization,
            algorithm_type="LINEAR",
            derivation_equation_constants=scrambled,
        ),
    )
    cfg = dc_replace(
        cfg, optical_trains=[dc_replace(t0, optional_components=OptionalComponents(clearbox=cb0))]
    )
    out = tmp_path / "v1_0_reordered.h5"
    MachineConfigWriter(cfg, target_version="1.0").write(out)
    back = MachineConfigReader(out).parse()
    back_pc = back.optical_trains[0].optional_components.clearbox.power_characterization
    # b before a for LINEAR confirms the writer re-sorted by name before
    # joining as CSV — checked via the reader's forward-derivation, since
    # the raw CSV string is no longer exposed on the StableModel directly.
    assert [c.name for c in back_pc.derivation_equation_constants] == ["b", "a"]
    assert [c.value for c in back_pc.derivation_equation_constants] == [1.0, 2.0]


def test_v1_1_to_v1_missing_derivation_constants_writes_blank(tmp_path):
    """Change 3 backward: empty Derivation_Equation_Constants -> blank, not an error."""
    cfg = _mock_v1_1_config(n_lasers=1)
    t0 = cfg.optical_trains[0]
    cb0 = t0.optional_components.clearbox
    cb0 = dc_replace(
        cb0,
        power_characterization=dc_replace(cb0.power_characterization, derivation_equation_constants=[]),
    )
    cfg = dc_replace(
        cfg, optical_trains=[dc_replace(t0, optional_components=OptionalComponents(clearbox=cb0))]
    )
    out = tmp_path / "v1_0_blank_constants.h5"
    MachineConfigWriter(cfg, target_version="1.0").write(out)
    back = MachineConfigReader(out).parse()
    back_pc = back.optical_trains[0].optional_components.clearbox.power_characterization
    # Volts_To_Watts_Params written blank ("", not an error) — forward-
    # derivation on read correctly comes back with zero constants, not a
    # crash. Checked via power_characterization since the raw string is no
    # longer exposed on the StableModel directly.
    assert back_pc.derivation_equation_constants == []


def test_v1_1_to_v1_missing_characterization_points_writes_blank(tmp_path):
    """Change 4 backward: empty Characterization_Points -> blank, not an error."""
    cfg = _mock_v1_1_config(n_lasers=1)
    t0 = cfg.optical_trains[0]
    ls0 = dc_replace(
        t0.light_source,
        power_characterization=dc_replace(t0.light_source.power_characterization, characterization_points=[]),
    )
    cfg = dc_replace(cfg, optical_trains=[dc_replace(t0, light_source=ls0)])
    out = tmp_path / "v1_0_blank_points.h5"
    MachineConfigWriter(cfg, target_version="1.0").write(out)
    back = MachineConfigReader(out).parse()
    # Same reasoning as the Change 3 case above.
    assert back.optical_trains[0].light_source.power_characterization.characterization_points == []


# ---------------------------------------------------------------------------
# test_v1_unaffected
# ---------------------------------------------------------------------------

def test_v1_unaffected():
    """Existing v1.0 read path still reads the same underlying HDF5
    attributes — the unification plan didn't change what's on disk for
    v1.0 files, only how the StableModel represents it (always via
    power_characterization now, forward-derived; the flat fields it used
    to populate no longer exist on the StableModel at all)."""
    config = MachineConfigReader(_REFERENCE_V1_0).parse()
    assert config.meta.file_version == "1.0"
    assert len(config.optical_trains) > 0
    cb0 = config.optical_trains[0].optional_components.clearbox
    assert cb0.power_characterization is not None
    assert cb0.power_characterization.algorithm_type == "LINEAR"


# ---------------------------------------------------------------------------
# Round-trip tests — the acceptance criterion stated 2026-09-01, made concrete
# ---------------------------------------------------------------------------

def test_roundtrip_v1_0_to_v1_1_to_v1_0(tmp_path):
    """Ask the API for a V1.0 file when a V1.0 file's data was loaded — via
    a real v1.1 file in between — and get back exactly what was there,
    nothing more, nothing less. The literal acceptance criterion."""
    source = MachineConfigReader(_REFERENCE_V1_0).parse()
    v1_1_path = tmp_path / "up.h5"
    MachineConfigWriter(source, target_version="1.1").write(v1_1_path)
    mid = MachineConfigReader(v1_1_path).parse()
    v1_0_path = tmp_path / "down.h5"
    MachineConfigWriter(mid, target_version="1.0").write(v1_0_path)
    back = MachineConfigReader(v1_0_path).parse()

    assert back.meta.file_version == "1.0"
    for i, t in enumerate(back.optical_trains):
        src_cb = source.optical_trains[i].optional_components.clearbox
        cb = t.optional_components.clearbox
        assert cb.output_path == src_cb.output_path
        assert cb.software_trigger_delay == src_cb.software_trigger_delay
        assert cb.power_characterization.algorithm_type == src_cb.power_characterization.algorithm_type
        assert sorted(c.value for c in cb.power_characterization.derivation_equation_constants) == sorted(
            c.value for c in src_cb.power_characterization.derivation_equation_constants
        )

        src_ls = source.optical_trains[i].light_source
        ls = t.light_source
        assert ls.power_characterization.algorithm_type == src_ls.power_characterization.algorithm_type
        src_points = [
            (p.input_value, p.output_value) for p in src_ls.power_characterization.characterization_points
        ]
        back_points = [
            (p.input_value, p.output_value) for p in ls.power_characterization.characterization_points
        ]
        assert back_points == src_points


def test_roundtrip_v1_1_to_v1_0_to_v1_1(tmp_path):
    """Mirror direction: starting from a natively-authored v1.1 file,
    downgrade then upgrade again. Fields v1.0 can represent survive; fields
    only v1.1 can hold come back blank — expected, documented
    richer-to-simpler loss, not a bug.
    """
    source = MachineConfigReader(_REFERENCE_V1_1).parse()
    v1_0_path = tmp_path / "down.h5"
    MachineConfigWriter(source, target_version="1.0").write(v1_0_path)
    mid = MachineConfigReader(v1_0_path).parse()
    v1_1_path = tmp_path / "up.h5"
    MachineConfigWriter(mid, target_version="1.1").write(v1_1_path)
    back = MachineConfigReader(v1_1_path).parse()

    assert back.meta.file_version == "1.1"
    for i, t in enumerate(back.optical_trains):
        src_cb = source.optical_trains[i].optional_components.clearbox
        cb = t.optional_components.clearbox
        assert cb.power_characterization.algorithm_type == src_cb.power_characterization.algorithm_type
        src_values = sorted(c.value for c in src_cb.power_characterization.derivation_equation_constants)
        back_values = sorted(c.value for c in cb.power_characterization.derivation_equation_constants)
        assert back_values == src_values
        # Expected loss: v1.1-only fields have no v1.0 round-trip path.
        assert cb.firmware_version is None
        assert cb.power_characterization.input_type is None
        assert cb.power_characterization.units_derived_quantity is None
        assert cb.power_characterization.characterization_points == []


# ---------------------------------------------------------------------------
# Writer target_version parameter
# ---------------------------------------------------------------------------

def test_writer_target_version_overrides_meta(tmp_path):
    source = MachineConfigReader(_REFERENCE_V1_0).parse()
    assert source.meta.file_version == "1.0"
    out = tmp_path / "overridden_up.h5"
    MachineConfigWriter(source, target_version="1.1").write(out)
    result = MachineConfigReader(out).parse()
    assert result.meta.file_version == "1.1"
    # Writer must not mutate its input.
    assert source.meta.file_version == "1.0"

    v1_1_source = MachineConfigReader(_REFERENCE_V1_1).parse()
    assert v1_1_source.meta.file_version == "1.1"
    out2 = tmp_path / "overridden_down.h5"
    MachineConfigWriter(v1_1_source, target_version="1.0").write(out2)
    result2 = MachineConfigReader(out2).parse()
    assert result2.meta.file_version == "1.0"
    assert v1_1_source.meta.file_version == "1.1"


def test_writer_defaults_to_meta_file_version(tmp_path):
    source = MachineConfigReader(_REFERENCE_V1_0).parse()
    out = tmp_path / "default.h5"
    MachineConfigWriter(source).write(out)  # no target_version — today's behavior
    result = MachineConfigReader(out).parse()
    assert result.meta.file_version == "1.0"


# ---------------------------------------------------------------------------
# Dispatcher-level tests — full public API via the real "1.1" registry entry
# (no monkeypatching needed, unlike the mock's temporary injection).
# ---------------------------------------------------------------------------

def test_dispatcher_v1_to_v1_1(tmp_path):
    source = MachineConfigReader(_REFERENCE_V1_0).parse()
    out = tmp_path / "dispatcher_v1_1.h5"
    MachineConfigWriter(source, target_version="1.1").write(out)
    result = MachineConfigReader(out).parse()
    assert result.meta.file_version == "1.1"
    assert result.optical_trains[0].optional_components.clearbox.output_path == "/recordings/"
    assert result.optical_trains[0].optional_components.clearbox.power_characterization is not None


def test_dispatcher_v1_1_to_v1(tmp_path):
    v1_1_cfg = MachineConfigReader(_REFERENCE_V1_1).parse()
    out = tmp_path / "dispatcher_migrated_v1.h5"
    MachineConfigWriter(v1_1_cfg, target_version="1.0").write(out)
    result = MachineConfigReader(out).parse()
    assert result.meta.file_version == "1.0"
    assert (
        result.optical_trains[0].optional_components.clearbox.power_characterization.algorithm_type
        == "LINEAR"
    )


def test_adapters_satisfy_protocol(tmp_path):
    cfg = _mock_v1_1_config(n_lasers=1)
    p = tmp_path / "proto_check.h5"
    Hdf5WriterV1_1(cfg).write(p)
    assert isinstance(Hdf5AdapterV1_1(p), ReaderAdapter)
    assert isinstance(Hdf5WriterV1_1(cfg), WriterAdapter)
