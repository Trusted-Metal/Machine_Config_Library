"""Capability facade tests for File_Version 1.0 (full-model get/set)."""
from __future__ import annotations

from pathlib import Path

from machine_config.capabilities import (
    create_machine_config,
    open_machine_config,
    supported_file_versions,
    SetMode,
)
from machine_config.capabilities.result import Ok
from machine_config.reader import MachineConfigReader

REPO = Path(__file__).resolve().parents[2]
FIXTURE = REPO / "fixtures" / "reference_config.h5"
FIXTURE_OPCUA = REPO / "fixtures" / "reference_config_opcua.h5"


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
