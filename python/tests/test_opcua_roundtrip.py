"""
Phase 1.8e — OPCUA round-trip tests
=====================================
Each test follows the pattern:

  parse → mutate (or build from scratch) → write → re-parse → assert

All fixtures are module-level functions (not class-instance methods) to
avoid the PytestRemovedIn10Warning about instance-method fixtures.
"""
from __future__ import annotations

import pytest

from machine_config import (
    MachineConfigReader,
    MachineConfigWriter,
    OpcuaClientConfig,
    OpcuaConfig,
    OpcuaPipeConfig,
    OpcuaTrigger,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_CLIENT = OpcuaClientConfig(
    server_url="opc.tcp://localhost:4840",
    auth_mode="Anonymous",
    security_mode="None",
    security_policy="None",
    bfs_max_depth=8,
    publish_interval=500,
    sampling_interval=500,
    session_timeout=30000,
)

_PIPE = OpcuaPipeConfig(pipe_enabled=True, buffer_size=32768)


# ---------------------------------------------------------------------------
# Test 1: opcua=None is preserved through HDF5 round-trip
# ---------------------------------------------------------------------------

def test_roundtrip_opcua_none(tmp_path, reference_reader):
    """Configs without OPCUA survive write→re-parse with opcua == None."""
    config = reference_reader.parse()
    assert config.opcua is None

    out = tmp_path / "no_opcua.h5"
    MachineConfigWriter(config).write(out)

    reparsed = MachineConfigReader(out).parse()
    assert reparsed.opcua is None


# ---------------------------------------------------------------------------
# Test 2: minimal OpcuaConfig round-trips intact
# ---------------------------------------------------------------------------

def test_roundtrip_minimal_opcua(tmp_path, reference_reader):
    """A minimal OpcuaConfig (no triggers) survives write→re-parse."""
    config = reference_reader.parse()
    config.opcua = OpcuaConfig(
        client=_CLIENT,
        pipe=_PIPE,
        triggers={},
        triggers_enabled=False,
    )

    out = tmp_path / "minimal_opcua.h5"
    MachineConfigWriter(config).write(out)

    rt = MachineConfigReader(out).parse()
    assert rt.opcua is not None
    assert rt.opcua.client.server_url == "opc.tcp://localhost:4840"
    assert rt.opcua.pipe.pipe_enabled is True
    assert rt.opcua.pipe.buffer_size == 32768
    assert rt.opcua.triggers == {}
    assert rt.opcua.triggers_enabled is False


# ---------------------------------------------------------------------------
# Test 3: client scalar fields are exact after round-trip
# ---------------------------------------------------------------------------

def test_roundtrip_client_fields(tmp_path, reference_reader):
    config = reference_reader.parse()
    config.opcua = OpcuaConfig(
        client=_CLIENT,
        pipe=_PIPE,
        triggers={},
    )

    out = tmp_path / "client_fields.h5"
    MachineConfigWriter(config).write(out)

    rt = MachineConfigReader(out).parse()
    c = rt.opcua.client
    assert c.auth_mode == "Anonymous"
    assert c.security_mode == "None"
    assert c.security_policy == "None"
    assert c.bfs_max_depth == 8
    assert c.publish_interval == 500
    assert c.sampling_interval == 500
    assert c.session_timeout == 30000


# ---------------------------------------------------------------------------
# Test 4: triggers_enabled=True round-trips as True
# ---------------------------------------------------------------------------

def test_roundtrip_triggers_enabled_true(tmp_path, reference_reader):
    config = reference_reader.parse()
    config.opcua = OpcuaConfig(
        client=_CLIENT,
        pipe=_PIPE,
        triggers={},
        triggers_enabled=True,
    )

    out = tmp_path / "triggers_enabled.h5"
    MachineConfigWriter(config).write(out)

    rt = MachineConfigReader(out).parse()
    assert rt.opcua.triggers_enabled is True


# ---------------------------------------------------------------------------
# Test 5: triggers_enabled=None omits the attr (reads back as None)
# ---------------------------------------------------------------------------

def test_roundtrip_triggers_enabled_none(tmp_path, reference_reader):
    config = reference_reader.parse()
    config.opcua = OpcuaConfig(
        client=_CLIENT,
        pipe=_PIPE,
        triggers={},
        triggers_enabled=None,
    )

    out = tmp_path / "triggers_enabled_none.h5"
    MachineConfigWriter(config).write(out)

    rt = MachineConfigReader(out).parse()
    assert rt.opcua.triggers_enabled is None


# ---------------------------------------------------------------------------
# Test 6: single trigger with all known fields round-trips
# ---------------------------------------------------------------------------

def test_roundtrip_single_trigger(tmp_path, reference_reader):
    trigger = OpcuaTrigger(
        id="t1",
        signal="sig",
        subsystem="Sub",
        rule_enabled=True,
        start_value="0",
        stop_value="100",
    )
    config = reference_reader.parse()
    config.opcua = OpcuaConfig(
        client=_CLIENT,
        pipe=_PIPE,
        triggers={"MyTrigger": trigger},
        triggers_enabled=True,
    )

    out = tmp_path / "single_trigger.h5"
    MachineConfigWriter(config).write(out)

    rt = MachineConfigReader(out).parse()
    assert "MyTrigger" in rt.opcua.triggers
    t = rt.opcua.triggers["MyTrigger"]
    assert t.id == "t1"
    assert t.signal == "sig"
    assert t.subsystem == "Sub"
    assert t.rule_enabled is True
    assert t.start_value == "0"
    assert t.stop_value == "100"


# ---------------------------------------------------------------------------
# Test 7: trigger extra fields survive round-trip
# ---------------------------------------------------------------------------

def test_roundtrip_trigger_extra_fields(tmp_path, reference_reader):
    trigger = OpcuaTrigger(
        id="t2",
        signal="sig2",
        subsystem="Sub2",
        rule_enabled=False,
        extra={"Custom_Label": "foo", "Priority": "high"},
    )
    config = reference_reader.parse()
    config.opcua = OpcuaConfig(
        client=_CLIENT,
        pipe=_PIPE,
        triggers={"ExtraTrigger": trigger},
    )

    out = tmp_path / "trigger_extra.h5"
    MachineConfigWriter(config).write(out)

    rt = MachineConfigReader(out).parse()
    t = rt.opcua.triggers["ExtraTrigger"]
    assert t.extra["Custom_Label"] == "foo"
    assert t.extra["Priority"] == "high"


# ---------------------------------------------------------------------------
# Test 8: multiple triggers all survive round-trip
# ---------------------------------------------------------------------------

def test_roundtrip_multiple_triggers(tmp_path, reference_reader):
    config = reference_reader.parse()
    config.opcua = OpcuaConfig(
        client=_CLIENT,
        pipe=_PIPE,
        triggers={
            "Trigger A": OpcuaTrigger(id="a", signal="sig_a"),
            "Trigger B": OpcuaTrigger(id="b", signal="sig_b"),
            "Trigger C": OpcuaTrigger(id="c", signal="sig_c"),
        },
        triggers_enabled=True,
    )

    out = tmp_path / "multi_trigger.h5"
    MachineConfigWriter(config).write(out)

    rt = MachineConfigReader(out).parse()
    triggers = rt.opcua.triggers
    assert set(triggers.keys()) == {"Trigger A", "Trigger B", "Trigger C"}
    assert triggers["Trigger A"].id == "a"
    assert triggers["Trigger B"].signal == "sig_b"
    assert triggers["Trigger C"].id == "c"


# ---------------------------------------------------------------------------
# Test 9: full reference_config_opcua.h5 fixture round-trips
# ---------------------------------------------------------------------------

def test_roundtrip_full_opcua_fixture(tmp_path, opcua_reader):
    config = opcua_reader.parse()
    assert config.opcua is not None

    out = tmp_path / "full_opcua.h5"
    MachineConfigWriter(config).write(out)

    rt = MachineConfigReader(out).parse()
    assert rt.opcua is not None
    assert rt.opcua.client.server_url == config.opcua.client.server_url
    assert rt.opcua.client.bfs_max_depth == config.opcua.client.bfs_max_depth
    assert rt.opcua.pipe.pipe_enabled == config.opcua.pipe.pipe_enabled
    assert rt.opcua.pipe.buffer_size == config.opcua.pipe.buffer_size
    assert rt.opcua.triggers_enabled == config.opcua.triggers_enabled
    assert set(rt.opcua.triggers.keys()) == set(config.opcua.triggers.keys())


# ---------------------------------------------------------------------------
# Test 10: JSON round-trip (to_json → config_from_dict) for OpcuaConfig
# ---------------------------------------------------------------------------

def test_roundtrip_json_opcua(tmp_path, reference_reader):
    """OpcuaConfig survives the to_json → config_from_dict cycle."""
    from machine_config import config_from_dict
    import json

    config = reference_reader.parse()
    config.opcua = OpcuaConfig(
        client=_CLIENT,
        pipe=_PIPE,
        triggers={
            "Interlock": OpcuaTrigger(
                id="il1",
                signal="light",
                subsystem="Chamber",
                rule_enabled=True,
                start_value="false",
                stop_value="true",
                extra={"Event": "SensorEvents"},
            )
        },
        triggers_enabled=True,
    )

    # Persist to HDF5, then use the reader's to_json() for the JSON round-trip
    out = tmp_path / "json_opcua.h5"
    MachineConfigWriter(config).write(out)

    json_str = MachineConfigReader(out).to_json()
    d = json.loads(json_str)
    rt = config_from_dict(d)

    assert rt.opcua is not None
    assert rt.opcua.client.server_url == "opc.tcp://localhost:4840"
    assert rt.opcua.pipe.buffer_size == 32768
    assert rt.opcua.triggers_enabled is True
    t = rt.opcua.triggers["Interlock"]
    assert t.id == "il1"
    assert t.rule_enabled is True
    assert t.extra["Event"] == "SensorEvents"
