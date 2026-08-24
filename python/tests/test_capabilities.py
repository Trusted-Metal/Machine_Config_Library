"""Capability facade tests for File_Version 1.0 (full-model get/set)."""
from __future__ import annotations

from pathlib import Path

import numpy as np

from machine_config.capabilities import (
    create_machine_config,
    open_machine_config,
    supported_file_versions,
    SetMode,
)
from machine_config.capabilities.result import Ok
from machine_config.reader import MachineConfigReader
from machine_config.writer import MachineConfigWriter

REPO = Path(__file__).resolve().parents[2]
FIXTURE = REPO / "fixtures" / "reference_config.h5"
FIXTURE_OPCUA = REPO / "fixtures" / "reference_config_opcua.h5"
FIXTURE_OPCUA_MISSING_REQUIRED = (
    REPO / "docs" / "validation" / "fixtures" / "opcua_missing_required.h5"
)


def test_supported_versions():
    assert "1.0" in supported_file_versions()


def test_open_get_scanner_matches_reader():
    result = open_machine_config(FIXTURE)
    assert isinstance(result, Ok)
    file = result.value
    assert file.file_version() == "1.0"

    json_cfg = MachineConfigReader(str(FIXTURE)).parse()
    train = file.optical_train(0)
    assert isinstance(train, Ok)
    scanner = train.value.get_scanner()
    assert scanner["working_distance"] == json_cfg.optical_trains[0].scanner.working_distance
    assert scanner["manufacturer"] == json_cfg.optical_trains[0].scanner.manufacturer
    file.close()


def test_merge_set_scanner_roundtrip(tmp_path):
    out = tmp_path / "merge.h5"
    opened = open_machine_config(FIXTURE)
    assert isinstance(opened, Ok)
    file = opened.value
    train = file.optical_train(0)
    assert isinstance(train, Ok)
    before = train.value.get_scanner()
    manufacturer = before["manufacturer"]
    patched = {**before, "working_distance": 123.5}
    assert isinstance(train.value.set_scanner(patched, SetMode.MERGE), Ok)
    assert isinstance(file.save(str(out)), Ok)
    file.close()

    again = open_machine_config(out)
    assert isinstance(again, Ok)
    train2 = again.value.optical_train(0)
    assert isinstance(train2, Ok)
    after = train2.value.get_scanner()
    assert after["working_distance"] == 123.5
    assert after["manufacturer"] == manufacturer
    again.value.close()


def test_replace_set_scanner_clears_extra():
    opened = open_machine_config(FIXTURE)
    assert isinstance(opened, Ok)
    train = opened.value.optical_train(0)
    assert isinstance(train, Ok)
    before = train.value.get_scanner()
    replacement = {
        "manufacturer": "ReplaceCo",
        "model": "ReplaceModel",
        "serial_number": "R-1",
        "working_distance": 1,
        "working_distance_unit": "mm",
        "extra": {},
    }
    assert isinstance(train.value.set_scanner(replacement, SetMode.REPLACE), Ok)
    after = train.value.get_scanner()
    assert after["manufacturer"] == "ReplaceCo"
    assert after["working_distance"] == 1
    assert after.get("extra") in ({}, None)
    assert after.get("model") != before.get("model") or before.get("model") == "ReplaceModel"
    opened.value.close()


def test_optical_trains_and_invalid_index():
    result = open_machine_config(FIXTURE)
    assert isinstance(result, Ok)
    assert len(result.value.optical_trains()) >= 1
    bad = result.value.optical_train(999)
    assert not bad.ok
    assert bad.error.code == "InvalidIndex"
    result.value.close()


def test_opcua_not_present_vs_present():
    no_opc = open_machine_config(FIXTURE)
    assert isinstance(no_opc, Ok)
    missing = no_opc.value.opcua()
    assert not missing.ok
    assert missing.error.code == "NotPresent"
    no_opc.value.close()

    with_opc = open_machine_config(FIXTURE_OPCUA)
    assert isinstance(with_opc, Ok)
    present = with_opc.value.opcua()
    assert isinstance(present, Ok)
    assert present.value.get_model()
    with_opc.value.close()


def test_opcua_required_fields_present_on_reference_fixture():
    file = open_machine_config(FIXTURE_OPCUA).value
    result = file.opcua()
    assert isinstance(result, Ok), f"expected Ok, got {result}"
    model = result.value.get_model()
    assert model["client"]["machine_profile"] is not None
    assert all(t["event"] is not None for t in model["triggers"].values())
    file.close()


def test_opcua_missing_required_fields_reports_all_seven_at_once():
    file = open_machine_config(FIXTURE_OPCUA_MISSING_REQUIRED).value
    result = file.opcua()
    assert not result.ok
    assert result.error.code == "ValidationError"

    expected = {
        "Machine_Profile",
        "Root_Node",
        "Configure_Client",
        "Pipe_Name",
        "Triggers_Enabled",
        "Trigger_Stop_Ceiling_Layers",
        "Laser Emission Interlock.Event",
    }
    assert set(result.error.details) == expected
    assert not any(d.startswith("Chamber Oxygen Level") for d in result.error.details)
    file.close()


def test_opcua_optional_field_never_appears_in_missing_details(tmp_path):
    # opcua_missing_required.h5 only clears the 7 required fields — every
    # optional field is still present there, so absence of an optional field
    # from `details` would be trivially true. To make this a real check, also
    # clear an optional field (keep_alive_count) in memory, re-write to a temp
    # file, and confirm `details` still names exactly the same 7 items.
    config = MachineConfigReader(str(FIXTURE_OPCUA_MISSING_REQUIRED)).parse()
    config.opcua.client.keep_alive_count = None
    out = tmp_path / "missing_required_plus_optional.h5"
    MachineConfigWriter(config).write(out)

    file = open_machine_config(out).value
    result = file.opcua()
    assert not result.ok
    assert not any("Keep_Alive_Count" in d for d in result.error.details), result.error.details
    assert len(result.error.details) == 7
    file.close()


def test_optional_components_and_clearbox():
    opened = open_machine_config(FIXTURE)
    assert isinstance(opened, Ok)
    train = opened.value.optical_train(0)
    assert isinstance(train, Ok)
    oc = train.value.optional_components()
    assert oc is not None
    cb = oc.clearbox()
    assert isinstance(cb, Ok)
    opened.value.close()

    created = create_machine_config("1.0")
    assert isinstance(created, Ok)
    t0 = created.value.optical_train(0)
    assert isinstance(t0, Ok)
    model = t0.value.get_model()
    model["optional_components"] = {"clearbox": None}
    assert isinstance(t0.value.set_model(model, SetMode.REPLACE), Ok)
    assert t0.value.optional_components() is None
    created.value.close()


def test_get_correction_data_matches_reader():
    file = open_machine_config(FIXTURE).value
    reader_adapter = MachineConfigReader(str(FIXTURE))
    # MachineConfigReader.parse() doesn't expose get_correction_data directly;
    # the v1_0 adapter it dispatches to does — same method the facade should
    # match exactly.
    from machine_config.capabilities.v1_0.hdf5 import Hdf5AdapterV1_0
    adapter = Hdf5AdapterV1_0(str(FIXTURE))

    result = file.get_correction_data(0)
    assert isinstance(result, Ok)
    expected = adapter.get_correction_data(0)
    np.testing.assert_array_equal(result.value, expected)  # NaN-aware equality

    inv_result = file.get_inverse_correction_data(0)
    assert isinstance(inv_result, Ok)
    inv_expected = adapter.get_inverse_correction_data(0)
    np.testing.assert_array_equal(inv_result.value, inv_expected)
    file.close()


def test_get_correction_data_shape_and_nan_present_through_facade():
    file = open_machine_config(FIXTURE).value
    grid = file.get_correction_data(0).value
    assert grid.shape == (257, 257, 2)
    assert np.isnan(grid).any()
    file.close()


def test_get_correction_data_missing_clearbox_is_not_present(tmp_path):
    # Every stock fixture's trains have a ClearBox, so build one without: take
    # the reference config, strip train 0's ClearBox, re-write to a temp file.
    config = MachineConfigReader(str(FIXTURE)).parse()
    config.optical_trains[0].optional_components.clearbox = None
    out = tmp_path / "no_clearbox.h5"
    MachineConfigWriter(config).write(out)

    file = open_machine_config(out).value
    result = file.get_correction_data(0)
    assert not result.ok
    assert result.error.code == "NotPresent"
    inv_result = file.get_inverse_correction_data(0)
    assert not inv_result.ok
    assert inv_result.error.code == "NotPresent"
    file.close()


def test_get_correction_data_works_on_create_based_instance_without_touching_disk():
    # The specific case that rules out delegate-to-Reader-by-reopening: a
    # create()-d facade has no path at all, so this must convert the
    # already-loaded in-memory model, not re-read from anywhere.
    file = create_machine_config("1.0").value
    grid = file.get_correction_data(0)
    assert isinstance(grid, Ok)
    assert grid.value.shape == (257, 257, 2)
    inv_grid = file.get_inverse_correction_data(0)
    assert isinstance(inv_grid, Ok)
    assert inv_grid.value.shape == (257, 257, 2)
    file.close()


def test_create_set_machine_save_reopen(tmp_path):
    out = tmp_path / "created.h5"
    created = create_machine_config("1.0")
    assert isinstance(created, Ok)
    file = created.value
    assert file.file_version() == "1.0"
    meta = file.meta().get_model()
    meta = {**meta, "machine_name": "CreatedMachine"}
    assert isinstance(file.meta().set_model(meta, SetMode.MERGE), Ok)
    assert isinstance(file.save(str(out)), Ok)
    file.close()

    again = open_machine_config(out)
    assert isinstance(again, Ok)
    assert again.value.file_version() == "1.0"
    assert again.value.meta().get_model()["machine_name"] == "CreatedMachine"
    again.value.close()
