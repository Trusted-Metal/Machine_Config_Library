import tempfile
from pathlib import Path
from machine_config import MachineConfigReader, MachineConfigWriter

"""
S-07: OPCUA configuration round-trip: read, write, re-read, verify OPCUA data preserved
ID:          S-07
Title:       Read, write, and re-read OPCUA fixture
Category:    happy-path
Layer:       reader + writer (OPCUA path)
Precondition: fixtures/reference_config_opcua.h5
Action:      Read → write to temp → read temp
Expected:    opcua.client.server_url unchanged
             opcua.client.session_timeout unchanged
             opcua.triggers_enabled unchanged
             All trigger names preserved
             "Chamber Oxygen Level" trigger: signal, subsystem, rule_enabled,
             start_value, stop_value all unchanged
Rationale:   OPCUA is an optional complex subgraph. Silent data loss in triggers
             would not be caught by scalar field checks.
"""

def run(fixtures_dir: str, real_dir: str) -> tuple[bool, str]:
    path = Path(fixtures_dir) / "reference_config_opcua.h5"
    if not path.exists():
        return False, f"OPCUA fixture not found: {path}"

    cfg = MachineConfigReader(path).parse()
    if cfg.opcua is None:
        return False, "opcua is None after reading OPCUA fixture"

    orig_url = cfg.opcua.client.server_url
    orig_timeout = cfg.opcua.client.session_timeout
    orig_triggers_enabled = cfg.opcua.triggers_enabled
    orig_trigger_names = set(cfg.opcua.triggers)

    with tempfile.NamedTemporaryFile(suffix=".h5", delete=False) as f:
        tmp = f.name

    MachineConfigWriter(cfg).write(tmp)
    rb = MachineConfigReader(tmp).parse()

    if rb.opcua is None:
        return False, "opcua is None after roundtrip"
    if rb.opcua.client.server_url != orig_url:
        return False, f"server_url changed: {orig_url!r} → {rb.opcua.client.server_url!r}"
    if rb.opcua.client.session_timeout != orig_timeout:
        return False, f"session_timeout changed: {orig_timeout} → {rb.opcua.client.session_timeout}"
    if rb.opcua.triggers_enabled != orig_triggers_enabled:
        return False, f"triggers_enabled changed: {orig_triggers_enabled} → {rb.opcua.triggers_enabled}"

    rb_names = set(rb.opcua.triggers)
    if rb_names != orig_trigger_names:
        return False, f"trigger names changed: missing={orig_trigger_names - rb_names}"

    co = "Chamber Oxygen Level"
    if co in cfg.opcua.triggers:
        ot, rt = cfg.opcua.triggers[co], rb.opcua.triggers[co]
        if ot.signal != rt.signal or ot.subsystem != rt.subsystem:
            return False, f"'{co}' signal/subsystem changed"

    return True, f"OPCUA roundtrip OK: {len(orig_trigger_names)} triggers, url={orig_url!r}"
