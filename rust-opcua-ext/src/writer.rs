use crate::raw_group::RawGroupAccess;
use crate::types::{ClientConfig, PipeConfig, TriggerConfig};
use machine_config::{CapabilityError, ExtraAttrs};

pub fn write_opcua_config(
    adapter: &impl RawGroupAccess,
    pipe: &PipeConfig,
    client: &ClientConfig,
    triggers: &TriggerConfig,
) -> Result<(), CapabilityError> {
    let mut opcua_attrs = ExtraAttrs::new();
    opcua_attrs.insert("Opcua_Schema_Version".into(), "1.2".into());
    adapter.set_raw_group("OPCUA", &opcua_attrs)?;
    adapter.set_raw_group("OPCUA/Pipe", &to_attrs(pipe)?)?;
    adapter.set_raw_group("OPCUA/Client", &to_attrs(client)?)?;

    let mut seen = std::collections::HashSet::new();
    for rule in &triggers.rules {
        if !seen.insert(rule.rule_id) {
            return Err(CapabilityError::validation_error(format!("duplicate rule_id: {}", rule.rule_id)));
        }
        let rule_path = format!("OPCUA/Triggers/{}", rule.rule_label);
        adapter.set_raw_group(&rule_path, &to_attrs(rule)?)?;

        let mut seen_conditions = std::collections::HashSet::new();
        for cond in &rule.conditions {
            if !seen_conditions.insert(cond.condition_id) {
                return Err(CapabilityError::validation_error(format!(
                    "duplicate condition_id {} in rule '{}'", cond.condition_id, rule.rule_label
                )));
            }
            let cond_path = format!("{rule_path}/Conditions/{}", cond.condition_label);
            adapter.set_raw_group(&cond_path, &to_attrs(cond)?)?;
        }
    }
    Ok(())
}

fn to_attrs<T: serde::Serialize>(
    v: &T,
) -> Result<ExtraAttrs, CapabilityError> {
    let value = serde_json::to_value(v)
        .map_err(|e| CapabilityError::IoError(e.to_string()))?;
    serde_json::from_value(value)
        .map_err(|e| CapabilityError::IoError(e.to_string()))
}
