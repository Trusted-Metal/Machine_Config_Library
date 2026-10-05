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
    adapter.set_raw_group(crate::OPCUA_ROOT, &opcua_attrs)?;
    adapter.set_raw_group(&format!("{}/Pipe", crate::OPCUA_ROOT), &to_attrs(pipe)?)?;
    adapter.set_raw_group(&format!("{}/Client", crate::OPCUA_ROOT), &to_attrs(client)?)?;
    adapter.set_raw_group(
        &format!("{}/Trigger", crate::OPCUA_ROOT),
        &to_attrs(triggers)?,
    )?;

    let mut seen = std::collections::HashSet::new();
    for rule in &triggers.rules {
        if rule.rule_id == 0 {
            return Err(CapabilityError::validation_error(
                "rule_id is required (0 is the unset sentinel)",
            ));
        }
        if rule.rule_label.trim().is_empty() {
            return Err(CapabilityError::validation_error(format!(
                "rule_id {} has an empty rule_label — refusing to write (would collide with the Trigger group itself)",
                rule.rule_id
            )));
        }
        if !seen.insert(rule.rule_id) {
            return Err(CapabilityError::validation_error(format!(
                "duplicate rule_id: {}",
                rule.rule_id
            )));
        }
        let rule_path = format!("{}/Trigger/{}", crate::OPCUA_ROOT, rule.rule_label);
        adapter.set_raw_group(&rule_path, &to_attrs(rule)?)?;

        let mut seen_conditions = std::collections::HashSet::new();
        for cond in &rule.conditions {
            if cond.condition_id == 0 {
                return Err(CapabilityError::validation_error(format!(
                    "condition_id is required in rule '{}'",
                    rule.rule_label
                )));
            }
            if cond.condition_label.trim().is_empty() {
                return Err(CapabilityError::validation_error(format!(
                    "condition_id {} in rule '{}' has an empty condition_label — refusing to write",
                    cond.condition_id, rule.rule_label
                )));
            }
            if !seen_conditions.insert(cond.condition_id) {
                return Err(CapabilityError::validation_error(format!(
                    "duplicate condition_id {} in rule '{}'",
                    cond.condition_id, rule.rule_label
                )));
            }
            let cond_path = format!("{rule_path}/{}", cond.condition_label);
            adapter.set_raw_group(&cond_path, &to_attrs(cond)?)?;
        }
    }
    Ok(())
}

pub(crate) fn to_attrs<T: serde::Serialize>(v: &T) -> Result<ExtraAttrs, CapabilityError> {
    let value = serde_json::to_value(v).map_err(|e| CapabilityError::IoError(e.to_string()))?;
    let serde_json::Value::Object(map) = value else {
        return Err(CapabilityError::IoError("expected a JSON object".into()));
    };
    let mut attrs = ExtraAttrs::new();
    for (k, v) in map {
        if !matches!(
            v,
            serde_json::Value::Array(_) | serde_json::Value::Object(_)
        ) {
            attrs.insert(k, v);
        }
    }
    Ok(attrs)
}
