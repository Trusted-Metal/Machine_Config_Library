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


def test_opcua_and_synchronous_sensor_coexist_through_writer(tmp_path, opcua_reader):
    """Proves the Writer side of the cross-feature guarantee (the Reader
    side is proved directly against the pre-built combined fixture in
    test_reader.py's TestSynchronousSensor) — parses a real OPCUA-only
    fixture, adds a sensor purely in memory, writes, and confirms both
    survive re-reading. See SYNCHRONOUS_SENSOR_PLAN.md's Phase 0
    fixture-layout decision for why both tests are kept.
    """
    from machine_config import CalibrationPoint, EquationConstant, SynchronousSensor

    config = opcua_reader.parse()
    assert config.opcua is not None, "fixture must already have OPCUA before the test adds a sensor"

    cb = config.optical_trains[0].optional_components.clearbox
    cb.synchronous_sensors["Oxygen Sensor"] = SynchronousSensor(
        enabled=True,
        sensor_name="ZR800 Oxygen Analyzer",
        sensor_output_range_low=-1.0,
        sensor_output_range_high=6.0,
        sensor_output_space="log10(ppm)",
        sensor_model="ZR810",
        sensor_manufacturer="Industrial Physics",
        sensor_scope="Global",
        units_derived_quantity="ppm",
        port_id=5,
        sensor_type="Oxygen Sensor",
        input_type="4-20 mA",
        algorithm_type="Log-Linear",
        algorithm_equation="log(ppm) = a*mA + b",
        calibration_source="Datasheet",
        calibration_verified=False,
        sample_period=5.0,
        metadata=None,
        derivation_equation_constants=[
            EquationConstant(name="a", value=0.4375),
            EquationConstant(name="b", value=-2.75),
        ],
        calibration_points=[
            CalibrationPoint(input_value=4.0, output_value=-1.0),
            CalibrationPoint(input_value=20.0, output_value=6.0),
        ],
    )

    out = tmp_path / "opcua_and_sensor.h5"
    MachineConfigWriter(config).write(out)
    rt = MachineConfigReader(out).parse()

    assert rt.opcua is not None, "OPCUA must survive alongside the newly-added sensor"
    rt_sensor = rt.optical_trains[0].optional_components.clearbox.synchronous_sensors["Oxygen Sensor"]
    assert rt_sensor.sensor_name == "ZR800 Oxygen Analyzer"
    assert len(rt_sensor.derivation_equation_constants) == 2
    assert len(rt_sensor.calibration_points) == 2


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
                event="SensorEvents",
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
    # Event is now a typed field, not swept into extra.
    assert t.event == "SensorEvents"
    assert "Event" not in t.extra


# ---------------------------------------------------------------------------
# Test 11: every one of the 22 promoted fields (plus trigger_stop_ceiling_layers)
# has its real value from reference_config_opcua.h5 — verified directly via
# h5py before writing this test — and none of them land in `extra` anymore.
# ---------------------------------------------------------------------------

def test_promoted_fields_have_real_values(opcua_reader):
    config = opcua_reader.parse()
    opcua = config.opcua

    c = opcua.client
    assert c.keep_alive_count == 240
    assert c.lifetime_count == 2400
    assert c.machine_profile == "Aconity"
    assert c.queue_policy == "DropOldest"
    assert c.queue_size_data_change == 100
    assert c.queue_size_events == 7200
    assert c.reconnect_interval == 10000
    assert c.root_node == "MachineFleet"
    assert c.sync_loop_interval_initial == 1000
    assert c.sync_loop_interval_settled == 30000
    assert c.extra == {}

    p = opcua.pipe
    assert p.configure_client is True
    assert p.inbound_rate_limit == -1
    assert p.max_inbound_message_size == 65536
    assert p.min_integrity_level == "0x2000"
    assert p.pipe_name == "\\\\.\\pipe\\opc_ua_client_pipe"
    assert p.user_access_level == "AnyLocalUser"
    assert p.extra == {}

    assert opcua.trigger_stop_ceiling_layers == 3

    laser = opcua.triggers["Laser Emission Interlock"]
    assert laser.case_sensitivity == "Exact"
    assert laser.component == "machine_state_indicator"
    assert laser.cooldown_period == 0
    assert laser.event == "SensorEvents"
    assert laser.max_fires_per_job == 0
    assert laser.trigger_label == "Laser Emission Interlock"
    assert laser.extra == {}

    oxygen = opcua.triggers["Chamber Oxygen Level"]
    assert oxygen.component == "process_chamber::gas_management::oxygen_sensor::1"
    assert oxygen.event == "SensorEvents"
    assert oxygen.trigger_label == "Chamber Oxygen Level"
    assert oxygen.extra == {}


# ---------------------------------------------------------------------------
# Test 12: opcua_missing_required.h5 (Phase 0) removes all seven Phase-2-
# required attributes. The reader must stay permissive (facade-only
# enforcement — see OPCUA_FIELD_PROMOTION_PLAN.md): parsing succeeds, the
# removed fields read back None, and the per-trigger Event asymmetry is
# exactly as the fixture intends.
# ---------------------------------------------------------------------------

def test_missing_required_fixture_parses_gracefully(opcua_missing_required_reader):
    config = opcua_missing_required_reader.parse()
    opcua = config.opcua
    assert opcua is not None

    assert opcua.client.machine_profile is None
    assert opcua.client.root_node is None
    assert opcua.pipe.configure_client is None
    assert opcua.pipe.pipe_name is None
    assert opcua.triggers_enabled is None
    assert opcua.trigger_stop_ceiling_layers is None

    laser = opcua.triggers["Laser Emission Interlock"]
    assert laser.event is None, "Event was deliberately removed from this trigger only"
    oxygen = opcua.triggers["Chamber Oxygen Level"]
    assert oxygen.event == "SensorEvents", "the other trigger must be unaffected"

    # Untouched fields elsewhere confirm the rest of the file is unaffected.
    assert opcua.client.server_url
    assert opcua.client.keep_alive_count == 240
    assert opcua.pipe.buffer_size == 65536
    assert laser.trigger_label == "Laser Emission Interlock"


# ---------------------------------------------------------------------------
# Test 13: trigger_stop_ceiling_layers round-trips through a temp file, both
# when populated and when None — the one field with no `extra` bucket to
# fall back on if the write/read pairing were mismatched.
# ---------------------------------------------------------------------------

def test_roundtrip_trigger_stop_ceiling_layers_none_and_some(tmp_path, opcua_reader):
    config = opcua_reader.parse()
    assert config.opcua.trigger_stop_ceiling_layers == 3

    out_some = tmp_path / "ceiling_some.h5"
    MachineConfigWriter(config).write(out_some)
    rt_some = MachineConfigReader(out_some).parse()
    assert rt_some.opcua.trigger_stop_ceiling_layers == 3

    config.opcua.trigger_stop_ceiling_layers = None
    out_none = tmp_path / "ceiling_none.h5"
    MachineConfigWriter(config).write(out_none)
    rt_none = MachineConfigReader(out_none).parse()
    assert rt_none.opcua.trigger_stop_ceiling_layers is None
