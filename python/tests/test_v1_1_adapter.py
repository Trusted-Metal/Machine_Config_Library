"""File_Version 1.1 adapter tests — the real v1.1 (not the mock in
test_adapter_migration.py). Mirrors the Rust/Node suites for
docs/migrations/v1_0_to_v1_1.md's 5 changes.

ClearBox is an opaque extension tree in core, always in the File_Version 1.1 layout
(``ClearBox/<train_id>/Power_Characterization/``, shared ``Output_Path`` and
``Software_Trigger_Delay`` on the extension root). A File_Version 1.0 file is shimmed into and out
of that layout on read and write, so upgrade/downgrade is just parse()/write() using
MachineConfigWriter's target_version parameter.
"""
from __future__ import annotations

import copy
from dataclasses import replace as dc_replace
from pathlib import Path

import pytest

from machine_config import MachineConfigReader, MachineConfigWriter
from machine_config.builder import MockConfigBuilder
from machine_config.capabilities.v1_0.hdf5 import Hdf5AdapterV1_0
from machine_config.capabilities.v1_1 import Hdf5AdapterV1_1, Hdf5WriterV1_1
from machine_config.extension_contract import Attr, canonical_digest
from machine_config.models import CalibrationPoint, PowerCharacterization
from machine_config.power_characterization import forward_power_characterization_points
from machine_config.reader import ReaderAdapter
from machine_config.writer import WriterAdapter

from clearbox_helpers import (
    attr,
    clearbox,
    clearbox_root,
    constants,
    integer,
    make_power_characterization,
    point_count,
    power_characterization,
    text,
)

_REPO_ROOT = Path(__file__).parent.parent.parent
_REFERENCE_V1_0 = _REPO_ROOT / "fixtures" / "reference_config_opcua_synchronous_sensors.h5"
_REFERENCE_V1_1 = _REPO_ROOT / "fixtures" / "reference_config_v1_1.h5"


def _mock_v1_0_config(n_lasers: int = 2):
    return MockConfigBuilder(n_lasers=n_lasers).build()


def _mock_v1_1_config(n_lasers: int = 2):
    """In-memory v1.1-shaped config: the v1.0 mock with ``meta.file_version`` overridden (its
    ClearBox tree is already in the v1.1 layout)."""
    cfg = _mock_v1_0_config(n_lasers)
    return dc_replace(cfg, meta=dc_replace(cfg.meta, file_version="1.1"))


def _via_file(cfg, version: str, tmp_path):
    """Write ``cfg`` as ``version`` and read it back through the public reader."""
    out = tmp_path / f"via_{version}_{id(cfg)}.h5"
    MachineConfigWriter(cfg, target_version=version).write(out)
    return MachineConfigReader(out).parse()


def _names(pc):
    return [n for n, _ in constants(pc)]


def _values(pc):
    return [v for _, v in constants(pc)]


def _flat_points(pc: PowerCharacterization):
    return [x for p in pc.characterization_points for x in (p.input_value, p.output_value)]


# ---------------------------------------------------------------------------
# read
# ---------------------------------------------------------------------------

def test_v1_1_read():
    """Natively-authored v1.1 fixture -> StableModel — all 5 changes asserted by value."""
    cfg = MachineConfigReader(_REFERENCE_V1_1).parse()
    assert cfg.meta.file_version == "1.1"

    # Change 1: Consolidate — one shared value, on the extension root.
    root = clearbox_root(cfg)
    assert text(root, "Output_Path") == "/recordings/"
    assert integer(root, "Software_Trigger_Delay") == 3000

    for i, t in enumerate(cfg.optical_trains):
        cb = clearbox(cfg, i)
        # Change 1: Addition
        assert text(cb, "Firmware_Version") == "2.4.1"
        # Change 1: Removal
        for removed in (
            "Selected_Camera", "Custom_Video_Format", "Video_Output",
            "Show_Console", "Output_Path", "Software_Trigger_Delay",
        ):
            assert removed not in cb.attrs

        # Change 5: no on-disk source in v1.1, always None
        assert t.scanner.x_axis.tuning_parameters is None
        assert t.scanner.x_axis.tuning_type is None
        assert t.scanner.y_axis.tuning_parameters is None
        assert t.scanner.y_axis.tuning_type is None

    # Change 2: the legacy OPCUA group is carried as the opaque "TM_OPCUA" extension
    assert "TM_OPCUA" in cfg.extensions
    assert "Machine_Profile" in cfg.extensions["TM_OPCUA"].children["Client"].attrs

    # Change 3: ClearBox Power_Characterization — train 1 LINEAR, train 2 POLYNOMIAL
    pc0 = power_characterization(cfg, 0)
    assert text(pc0, "Algorithm_Type") == "LINEAR"
    assert text(pc0, "Algorithm_Equation") == "W = a*V + b"
    assert text(pc0, "Input_Type") == "0-10 V"
    assert text(pc0, "Units_Derived_Quantity") == "Watts"
    assert _names(pc0) == ["b", "a"]
    assert point_count(pc0) == 3

    pc1 = power_characterization(cfg, 1)
    assert text(pc1, "Algorithm_Type") == "POLYNOMIAL"
    assert text(pc1, "Algorithm_Equation") == "W = c0 + c1*V + c2*V^2"
    assert _names(pc1) == ["c0", "c1", "c2"]

    # Change 4: LightSource Power_Characterization — inverse data availability
    lspc0 = cfg.optical_trains[0].light_source.power_characterization
    assert lspc0.algorithm_type == "LINEAR"
    assert lspc0.input_type == "Volts"
    assert len(lspc0.characterization_points) == 5
    assert lspc0.derivation_equation_constants  # populated in this review fixture


# ---------------------------------------------------------------------------
# roundtrip
# ---------------------------------------------------------------------------

def test_v1_1_roundtrip(tmp_path):
    """write -> read -> write -> read, no drift. ClearBox and Light_Source
    power_characterization deliberately given different values in the same
    object — direct test for the cross-wiring risk."""
    cfg = _mock_v1_1_config(n_lasers=1)
    clearbox(cfg, 0).children["Power_Characterization"] = make_power_characterization(
        "LINEAR", [("b", 1.0), ("a", 2.0)]
    )
    train = cfg.optical_trains[0]
    ls = dc_replace(
        train.light_source,
        power_characterization=dc_replace(
            train.light_source.power_characterization,
            algorithm_type="POLYNOMIAL",
            derivation_equation_constants=[],
            characterization_points=[CalibrationPoint(input_value=9.0, output_value=99.0)],
        ),
    )
    cfg = dc_replace(cfg, optical_trains=[dc_replace(train, light_source=ls)])

    p1 = tmp_path / "v1_1_a.h5"
    MachineConfigWriter(cfg).write(p1)
    mid = MachineConfigReader(p1).parse()
    p2 = tmp_path / "v1_1_b.h5"
    MachineConfigWriter(mid).write(p2)
    result = MachineConfigReader(p2).parse()

    cb_pc = power_characterization(result, 0)
    ls_pc = result.optical_trains[0].light_source.power_characterization
    assert text(cb_pc, "Algorithm_Type") == "LINEAR"
    assert _names(cb_pc) == ["b", "a"]
    assert point_count(cb_pc) == 0
    assert ls_pc.algorithm_type == "POLYNOMIAL"
    assert ls_pc.derivation_equation_constants == []
    assert ls_pc.characterization_points[0].input_value == 9.0
    # Cross-wiring guard: the two instances must not have swapped.
    assert text(cb_pc, "Algorithm_Type") != ls_pc.algorithm_type
    # The whole ClearBox tree, grids included, is bit-stable across the double round trip.
    assert canonical_digest(result.extensions["ClearBox"]) == canonical_digest(cfg.extensions["ClearBox"])


# ---------------------------------------------------------------------------
# v1.0 fixture written as v1.1
# ---------------------------------------------------------------------------

def test_v1_to_v1_1(tmp_path):
    source = MachineConfigReader(_REFERENCE_V1_0).parse()
    out = tmp_path / "v1_1_from_v1_0.h5"
    MachineConfigWriter(source, target_version="1.1").write(out)
    migrated = MachineConfigReader(out).parse()
    assert migrated.meta.file_version == "1.1"

    # The shimmed tree is already in the v1.1 layout, so it survives unchanged.
    assert canonical_digest(migrated.extensions["ClearBox"]) == canonical_digest(source.extensions["ClearBox"])
    root = clearbox_root(migrated)
    assert "Output_Path" in root.attrs
    assert "Software_Trigger_Delay" in root.attrs

    for i, t in enumerate(migrated.optical_trains):
        cb = clearbox(migrated, i)
        # Addition: no v1.0 source.
        assert "Firmware_Version" not in cb.attrs
        # v1.0-only attributes ride along as extra per-train attributes.
        assert "Selected_Camera" in cb.attrs
        # Change 3: derived ClearBox data has real constants, zero points.
        pc = power_characterization(migrated, i)
        assert point_count(pc) == 0
        assert constants(pc)
        assert text(pc, "Algorithm_Type") == text(power_characterization(source, i), "Algorithm_Type")

        # Change 4: derived Light_Source data is the inverse — zero constants, real points.
        ls_pc = t.light_source.power_characterization
        assert ls_pc.derivation_equation_constants == []
        assert ls_pc.characterization_points


def test_v1_0_first_train_wins_for_shared_values(tmp_path):
    """File_Version 1.0 stores Output_Path/Software_Trigger_Delay per train; the extension tree
    holds one value each, taken from the first train."""
    cfg = _mock_v1_0_config(n_lasers=2)
    clearbox(cfg, 1).attrs["Output_Path"] = Attr("utf8", "/other/")
    clearbox(cfg, 1).attrs["Software_Trigger_Delay"] = Attr("i64", 9999)
    back = _via_file(cfg, "1.0", tmp_path)
    root = clearbox_root(back)
    assert text(root, "Output_Path") == "/recordings/"
    assert integer(root, "Software_Trigger_Delay") == 3000
    assert "Output_Path" not in clearbox(back, 1).attrs


def test_v1_to_v1_1_unrecognized_algorithm_type_best_effort(tmp_path):
    """Never a hard error — best-effort, verbatim algorithm_type, positionally-named constants,
    blank algorithm_equation."""
    cfg = _mock_v1_1_config(n_lasers=1)
    clearbox(cfg, 0).children["Power_Characterization"] = make_power_characterization(
        "EXPONENTIAL", [("0", 1.5), ("1", 2.5), ("2", 3.5)]
    )
    v1_0 = _via_file(cfg, "1.0", tmp_path)
    pc = power_characterization(v1_0, 0)
    assert text(pc, "Algorithm_Type") == "EXPONENTIAL"
    assert text(pc, "Algorithm_Equation") is None
    assert _names(pc) == ["0", "1", "2"]
    assert _values(pc) == [1.5, 2.5, 3.5]

    back = _via_file(v1_0, "1.1", tmp_path)
    assert text(power_characterization(back, 0), "Algorithm_Type") == "EXPONENTIAL"
    assert _values(power_characterization(back, 0)) == [1.5, 2.5, 3.5]


def test_v1_to_v1_1_unrecognized_algorithm_type_best_effort_for_light_source(tmp_path):
    cfg = _mock_v1_0_config(n_lasers=1)
    t0 = cfg.optical_trains[0]
    ls0 = dc_replace(
        t0.light_source,
        power_characterization=forward_power_characterization_points("QUADRATIC", "1,2,3,4"),
    )
    cfg = dc_replace(cfg, optical_trains=[dc_replace(t0, light_source=ls0)])
    result = _via_file(cfg, "1.1", tmp_path)
    pc = result.optical_trains[0].light_source.power_characterization
    assert pc.algorithm_type == "QUADRATIC"
    assert pc.algorithm_equation is None
    assert len(pc.characterization_points) == 2


def test_clearbox_train_with_no_matching_optical_train_is_rejected_for_v1_0(tmp_path):
    cfg = _mock_v1_1_config(n_lasers=1)
    cfg.extensions["ClearBox"].children["Optical_Train_07"] = copy.deepcopy(clearbox(cfg, 0))
    with pytest.raises(ValueError, match="Optical_Train_07"):
        MachineConfigWriter(cfg, target_version="1.0").write(tmp_path / "no_matching_train.h5")


# ---------------------------------------------------------------------------
# v1.1 fixture written as v1.0
# ---------------------------------------------------------------------------

def test_v1_1_to_v1(tmp_path):
    source = MachineConfigReader(_REFERENCE_V1_1).parse()
    out = tmp_path / "v1_0_from_v1_1.h5"
    MachineConfigWriter(source, target_version="1.0").write(out)
    back = Hdf5AdapterV1_0(str(out)).parse()
    assert back.meta.file_version == "1.0"

    src_pc0 = power_characterization(source, 0)
    pc0 = power_characterization(back, 0)
    assert text(pc0, "Algorithm_Type") == text(src_pc0, "Algorithm_Type")
    assert sorted(_values(pc0)) == sorted(_values(src_pc0))
    src_root, root = clearbox_root(source), clearbox_root(back)
    assert root.attrs["Output_Path"] == src_root.attrs["Output_Path"]
    assert integer(root, "Software_Trigger_Delay") == integer(src_root, "Software_Trigger_Delay")

    src_ls = source.optical_trains[0].light_source.power_characterization
    ls = back.optical_trains[0].light_source.power_characterization
    assert ls.algorithm_type == src_ls.algorithm_type
    assert _flat_points(ls) == _flat_points(src_ls)

    # v1.1-only fields: lost on the way down, not restored.
    assert "Firmware_Version" not in clearbox(back, 0).attrs
    assert "Selected_Camera" not in clearbox(back, 0).attrs


def test_v1_1_to_v1_reorders_constants_by_name(tmp_path):
    """Derivation_Equation_Constants rows aren't positionally guaranteed — the writer must
    re-sort by name before joining as CSV."""
    cfg = _mock_v1_1_config(n_lasers=1)
    clearbox(cfg, 0).children["Power_Characterization"] = make_power_characterization(
        "LINEAR", [("a", 2.0), ("b", 1.0)]
    )
    back = _via_file(cfg, "1.0", tmp_path)
    assert constants(power_characterization(back, 0)) == [("b", 1.0), ("a", 2.0)]


def test_v1_1_to_v1_missing_derivation_constants_writes_blank(tmp_path):
    cfg = _mock_v1_1_config(n_lasers=1)
    clearbox(cfg, 0).children["Power_Characterization"] = make_power_characterization("LINEAR", [])
    back = _via_file(cfg, "1.0", tmp_path)
    assert constants(power_characterization(back, 0)) == []


def test_v1_1_to_v1_missing_characterization_points_writes_blank(tmp_path):
    cfg = _mock_v1_1_config(n_lasers=1)
    t0 = cfg.optical_trains[0]
    ls0 = dc_replace(
        t0.light_source,
        power_characterization=dc_replace(t0.light_source.power_characterization, characterization_points=[]),
    )
    cfg = dc_replace(cfg, optical_trains=[dc_replace(t0, light_source=ls0)])
    back = _via_file(cfg, "1.0", tmp_path)
    assert back.optical_trains[0].light_source.power_characterization.characterization_points == []


# ---------------------------------------------------------------------------
# v1.0 path unaffected
# ---------------------------------------------------------------------------

def test_v1_unaffected():
    """The v1.0 read path surfaces the flat Volts_To_Watts fields as Power_Characterization."""
    config = MachineConfigReader(_REFERENCE_V1_0).parse()
    assert config.meta.file_version == "1.0"
    assert len(config.optical_trains) > 0
    assert text(power_characterization(config, 0), "Algorithm_Type") == "LINEAR"
    assert "Volts_To_Watts_Algorithm" not in clearbox(config, 0).attrs


# ---------------------------------------------------------------------------
# Round-trip tests — the acceptance criterion stated 2026-09-01, made concrete
# ---------------------------------------------------------------------------

def test_roundtrip_v1_0_to_v1_1_to_v1_0(tmp_path):
    source = MachineConfigReader(_REFERENCE_V1_0).parse()
    mid = _via_file(source, "1.1", tmp_path)
    back = _via_file(mid, "1.0", tmp_path)

    assert back.meta.file_version == "1.0"
    assert canonical_digest(back.extensions["ClearBox"]) == canonical_digest(source.extensions["ClearBox"])
    for i, t in enumerate(back.optical_trains):
        src_ls = source.optical_trains[i].light_source.power_characterization
        ls = t.light_source.power_characterization
        assert ls.algorithm_type == src_ls.algorithm_type
        assert _flat_points(ls) == _flat_points(src_ls)


def test_roundtrip_v1_1_to_v1_0_to_v1_1(tmp_path):
    """Fields v1.0 can represent survive; fields only v1.1 can hold come back blank — the
    expected, documented richer-to-simpler loss."""
    source = MachineConfigReader(_REFERENCE_V1_1).parse()
    mid = _via_file(source, "1.0", tmp_path)
    back = _via_file(mid, "1.1", tmp_path)

    assert back.meta.file_version == "1.1"
    for i in range(len(back.optical_trains)):
        src_pc = power_characterization(source, i)
        pc = power_characterization(back, i)
        assert text(pc, "Algorithm_Type") == text(src_pc, "Algorithm_Type")
        assert sorted(_values(pc)) == sorted(_values(src_pc))
        # Expected loss: v1.1-only fields have no v1.0 round-trip path.
        assert "Firmware_Version" not in clearbox(back, i).attrs
        assert text(pc, "Input_Type") is None
        assert text(pc, "Units_Derived_Quantity") is None
        assert point_count(pc) == 0


# ---------------------------------------------------------------------------
# Writer target_version parameter
# ---------------------------------------------------------------------------

def test_writer_target_version_overrides_meta(tmp_path):
    source = MachineConfigReader(_REFERENCE_V1_0).parse()
    assert source.meta.file_version == "1.0"
    result = _via_file(source, "1.1", tmp_path)
    assert result.meta.file_version == "1.1"
    # Writer must not mutate its input.
    assert source.meta.file_version == "1.0"

    v1_1_source = MachineConfigReader(_REFERENCE_V1_1).parse()
    assert v1_1_source.meta.file_version == "1.1"
    result2 = _via_file(v1_1_source, "1.0", tmp_path)
    assert result2.meta.file_version == "1.0"
    assert v1_1_source.meta.file_version == "1.1"


def test_writer_defaults_to_meta_file_version(tmp_path):
    source = MachineConfigReader(_REFERENCE_V1_0).parse()
    out = tmp_path / "default.h5"
    MachineConfigWriter(source).write(out)  # no target_version
    assert MachineConfigReader(out).parse().meta.file_version == "1.0"


# ---------------------------------------------------------------------------
# Dispatcher-level tests — full public API via the real "1.1" registry entry
# ---------------------------------------------------------------------------

def test_dispatcher_v1_to_v1_1(tmp_path):
    source = MachineConfigReader(_REFERENCE_V1_0).parse()
    result = _via_file(source, "1.1", tmp_path)
    assert result.meta.file_version == "1.1"
    assert text(clearbox_root(result), "Output_Path") == "/recordings/"
    assert power_characterization(result, 0) is not None


def test_dispatcher_v1_1_to_v1(tmp_path):
    v1_1_cfg = MachineConfigReader(_REFERENCE_V1_1).parse()
    result = _via_file(v1_1_cfg, "1.0", tmp_path)
    assert result.meta.file_version == "1.0"
    assert text(power_characterization(result, 0), "Algorithm_Type") == "LINEAR"


def test_adapters_satisfy_protocol(tmp_path):
    cfg = _mock_v1_1_config(n_lasers=1)
    p = tmp_path / "proto_check.h5"
    Hdf5WriterV1_1(cfg).write(p)
    assert isinstance(Hdf5AdapterV1_1(p), ReaderAdapter)
    assert isinstance(Hdf5WriterV1_1(cfg), WriterAdapter)
