// S-07: OPCUA config roundtrip
//
// ID:          S-07
// Precondition: fixtures/reference_config_opcua.h5
// Action:      Read → write to temp → read temp
// Expected:    server_url, session_timeout, triggers_enabled, trigger names,
//              and "Chamber Oxygen Level" signal/subsystem all unchanged.

use machine_config::{MachineConfigReader, MachineConfigWriter};
use std::collections::HashSet;
use std::path::Path;

pub fn run(fixtures_dir: &Path, _real_dir: &Path) -> (bool, String) {
    let path = fixtures_dir.join("reference_config_opcua.h5");
    if !path.exists() {
        return (false, format!("OPCUA fixture not found: {}", path.display()));
    }

    let cfg = match MachineConfigReader::open(&path).and_then(|r| r.parse()) {
        Ok(c) => c,
        Err(e) => return (false, format!("read failed: {e}")),
    };
    let opcua = match &cfg.opcua {
        Some(o) => o,
        None => return (false, "opcua is None after reading OPCUA fixture".to_string()),
    };

    let orig_url = opcua.client.server_url.clone();
    let orig_timeout = opcua.client.session_timeout;
    let orig_triggers_enabled = opcua.triggers_enabled;
    let orig_trigger_names: HashSet<String> = opcua.triggers.keys().cloned().collect();

    let tmp = match tempfile::Builder::new().suffix(".h5").tempfile() {
        Ok(t) => t,
        Err(e) => return (false, format!("tempfile creation failed: {e}")),
    };
    if let Err(e) = MachineConfigWriter::new(&cfg).write(tmp.path()) {
        return (false, format!("write failed: {e}"));
    }
    let rb = match MachineConfigReader::open(tmp.path()).and_then(|r| r.parse()) {
        Ok(c) => c,
        Err(e) => return (false, format!("readback failed: {e}")),
    };
    let rb_opcua = match &rb.opcua {
        Some(o) => o,
        None => return (false, "opcua is None after roundtrip".to_string()),
    };

    if rb_opcua.client.server_url != orig_url {
        return (false, format!("server_url changed: {orig_url:?} -> {:?}", rb_opcua.client.server_url));
    }
    if rb_opcua.client.session_timeout != orig_timeout {
        return (
            false,
            format!("session_timeout changed: {orig_timeout} -> {}", rb_opcua.client.session_timeout),
        );
    }
    if rb_opcua.triggers_enabled != orig_triggers_enabled {
        return (
            false,
            format!("triggers_enabled changed: {orig_triggers_enabled:?} -> {:?}", rb_opcua.triggers_enabled),
        );
    }

    let rb_names: HashSet<String> = rb_opcua.triggers.keys().cloned().collect();
    if rb_names != orig_trigger_names {
        let missing: Vec<_> = orig_trigger_names.difference(&rb_names).collect();
        return (false, format!("trigger names changed: missing={missing:?}"));
    }

    let co = "Chamber Oxygen Level";
    if let (Some(ot), Some(rt)) = (opcua.triggers.get(co), rb_opcua.triggers.get(co)) {
        if ot.signal != rt.signal || ot.subsystem != rt.subsystem {
            return (false, format!("'{co}' signal/subsystem changed"));
        }
    }

    (
        true,
        format!("OPCUA roundtrip OK: {} triggers, url={orig_url:?}", orig_trigger_names.len()),
    )
}
